use crate::db::Database;
use crate::lightning::Lightning;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Message to notify the DonationService about new pending donations
pub struct NewDonation {
    pub invoice: String,
    pub amount_msats: i64,
    /// Location ID for direct location donations (None = global donation)
    pub location_id: Option<String>,
}

/// Background service that tracks pending donations and credits the pool when payments arrive.
/// Resilient against server restarts (loads pending donations from DB on startup) and
/// client disconnects (runs independently of HTTP connections).
pub struct DonationService {
    db: Arc<Database>,
    lightning: Arc<dyn Lightning>,
    /// Sender for new donation notifications
    sender: mpsc::UnboundedSender<NewDonation>,
    /// Receiver for new donation notifications (wrapped in Option for take())
    receiver: Mutex<Option<mpsc::UnboundedReceiver<NewDonation>>>,
    /// Set of invoices currently being awaited (to prevent duplicate tasks)
    active_invoices: Mutex<HashSet<String>>,
}

impl DonationService {
    pub fn new(db: Arc<Database>, lightning: Arc<dyn Lightning>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            db,
            lightning,
            sender,
            receiver: Mutex::new(Some(receiver)),
            active_invoices: Mutex::new(HashSet::new()),
        }
    }

    /// Get a sender clone to notify about new donations
    pub fn get_sender(&self) -> mpsc::UnboundedSender<NewDonation> {
        self.sender.clone()
    }

    /// Start the donation service - loads pending donations and listens for new ones
    pub async fn start(self: Arc<Self>) {
        // Load existing pending donations from database
        match self.db.list_pending_donations().await {
            Ok(pending) => {
                tracing::info!("Loaded {} pending donations from database", pending.len());
                for donation in pending {
                    self.clone()
                        .spawn_await_task(
                            donation.invoice,
                            donation.amount_msats,
                            donation.location_id,
                        )
                        .await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to load pending donations: {}", e);
            }
        }

        // Take the receiver (can only be done once)
        let receiver = {
            let mut guard = self.receiver.lock().await;
            guard.take()
        };

        let Some(mut receiver) = receiver else {
            tracing::error!("DonationService receiver already taken");
            return;
        };

        // Listen for new donations
        while let Some(donation) = receiver.recv().await {
            self.clone()
                .spawn_await_task(
                    donation.invoice,
                    donation.amount_msats,
                    donation.location_id,
                )
                .await;
        }
    }

    /// Spawn a task to await payment for a specific invoice
    /// When payment is received, the donation is marked as 'received' in the database.
    /// Pool balances are calculated from received donations, so no separate pool update is needed.
    async fn spawn_await_task(
        self: Arc<Self>,
        invoice: String,
        amount_msats: i64,
        location_id: Option<String>,
    ) {
        // Check if already tracking this invoice
        {
            let mut active = self.active_invoices.lock().await;
            if active.contains(&invoice) {
                tracing::debug!(
                    "Already tracking invoice, skipping: {}",
                    &invoice[..20.min(invoice.len())]
                );
                return;
            }
            active.insert(invoice.clone());
        }

        let service = self.clone();
        let invoice_clone = invoice.clone();

        tokio::spawn(async move {
            tracing::info!("Awaiting payment for {} sats invoice", amount_msats / 1000);

            match service.lightning.await_payment(&invoice_clone).await {
                Ok(()) => {
                    tracing::info!(
                        "Payment received! Processing {} sats donation",
                        amount_msats / 1000
                    );

                    // Mark donation as received in database
                    // This automatically updates the pool balance (calculated from received donations)
                    match service.db.mark_donation_received(&invoice_clone).await {
                        Ok(donation) => {
                            if let Some(loc_id) = &location_id {
                                // Get updated location pool balance
                                match service.db.get_location_donation_pool_balance(loc_id).await {
                                    Ok(balance_msats) => {
                                        tracing::info!(
                                            "Location donation received (id: {}). Pool balance: {} sats",
                                            donation.id,
                                            balance_msats / 1000
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Donation received but failed to get balance: {}",
                                            e
                                        );
                                    }
                                }
                            } else {
                                // Global donation was split among all locations
                                match service.db.list_active_locations().await {
                                    Ok(locations) => {
                                        let per_location = if locations.is_empty() {
                                            0
                                        } else {
                                            donation.amount_msats / locations.len() as i64 / 1000
                                        };
                                        tracing::info!(
                                            "Global donation received (id: {}). Split {} sats each to {} locations",
                                            donation.id,
                                            per_location,
                                            locations.len()
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Donation received but failed to list locations: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Failed to mark donation as received: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to await payment: {}", e);
                }
            }

            // Remove from active set
            let mut active = service.active_invoices.lock().await;
            active.remove(&invoice_clone);
        });
    }
}
