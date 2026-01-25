use crate::models::Donation;
use crate::templates::components::{
    donation_invoice_markup, donation_invoice_script, DonationInvoiceConfig,
};
use maud::{html, Markup};

/// pool_balance_sats: total pool balance across all locations in sats
/// num_locations: number of active locations
/// received_donations: list of received donations for display
pub fn donate(
    pool_balance_sats: i64,
    num_locations: usize,
    received_donations: &[Donation],
) -> Markup {
    let config = DonationInvoiceConfig {
        id_prefix: "",
        location_id: None,
        amounts: &[
            ("1000", "1K sats"),
            ("5000", "5K sats"),
            ("10000", "10K sats"),
            ("50000", "50K sats"),
            ("100000", "100K sats"),
            ("500000", "500K sats"),
            ("1000000", "1M sats"),
            ("custom", "Custom"),
        ],
        label: Some("Choose donation amount"),
    };

    html! {
        h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" {
            i class="fa-solid fa-coins mr-2" {}
            "Donate to All Locations"
        }

        // Current pool stats
        div class="card-brutal-inset mb-8" {
            h2 class="heading-breaker orange" { "Donation Pool" }
            div class="text-center mt-8" {
                div class="stat-brutal" {
                    div class="stat-value orange" {
                        (pool_balance_sats) " "
                        i class="fa-solid fa-bolt" {}
                    }
                    div class="stat-label" { "sats split across " (num_locations) " locations" }
                }
            }
            div class="text-center mt-4 text-secondary font-bold" {
                "Your donation is divided equally among all active treasure locations"
            }
        }

        // Donation form
        div class="card-brutal-inset" {
            h2 class="heading-breaker orange" { "Make a Donation" }

            div id="donationContainer" class="mt-8" {
                (donation_invoice_markup(&config))
            }
        }

        // How it works
        div class="card-bar mt-8" {
            h2 class="text-2xl font-black mb-6" { "How It Works" }
            div class="space-y-3 text-secondary" {
                p class="font-bold" {
                    "Global donations are split equally into the donation pools of all locations, locations are automatically refilled from their local donation pools. "
                    "When someone claims sats, that location resets and starts refilling again. "
                    "You can also donate directly to a specific location."
                }
                p class="text-highlight orange font-black text-lg" {
                    "Keep the treasure hunt alive!"
                }
            }
        }

        // Recent donations list (split entries filtered at DB level)
        @if !received_donations.is_empty() {
            div class="card-brutal-inset mt-8" {
                h2 class="heading-breaker orange" { "Recent Donations" }
                div class="mt-6 overflow-x-auto" {
                    table class="w-full" {
                        thead {
                            tr class="border-b-2 border-tertiary" {
                                th class="text-left py-2 px-3 font-black text-muted" { "Time" }
                                th class="text-right py-2 px-3 font-black text-muted" { "Amount" }
                            }
                        }
                        tbody {
                            @for donation in received_donations {
                                tr class="border-b border-tertiary hover:bg-tertiary" {
                                    td class="py-2 px-3 text-secondary" {
                                        @if let Some(received_at) = donation.received_at {
                                            (received_at.format("%Y-%m-%d %H:%M UTC"))
                                        }
                                    }
                                    td class="py-2 px-3 text-right font-bold text-highlight orange" {
                                        (donation.amount_sats()) " "
                                        i class="fa-solid fa-bolt" {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        (donation_invoice_script(&config))
    }
}
