use crate::models::{User, UserTransaction};
use maud::{html, Markup, PreEscaped};

/// Render the wallet page showing user's balance and transaction history.
pub fn wallet(
    balance_sats: i64,
    transactions: &[UserTransaction],
    user: Option<&User>,
    success: Option<&str>,
    amount: Option<i64>,
    location_name: Option<&str>,
) -> Markup {
    html! {
        div class="max-w-2xl mx-auto" {
            // Success message for collection
            @if let (Some("collected"), Some(amt)) = (success, amount) {
                div class="alert-brutal mb-6" style="background: var(--color-success); border-color: var(--color-success);" {
                    p class="font-bold text-white" {
                        i class="fa-solid fa-check-circle mr-2" {}
                        "Collected " (amt) " sats"
                        @if let Some(name) = location_name {
                            " from " (name)
                        }
                        "!"
                    }
                }
            }

            // Balance card
            div class="card-brutal mb-6" {
                h1 class="heading-breaker" {
                    i class="fa-solid fa-wallet mr-2" {}
                    @if let Some(u) = user.filter(|u| !u.is_anonymous()) {
                        (u.display_name()) "'S WALLET"
                    } @else {
                        "MY WALLET"
                    }
                }

                div class="p-8 text-center" {
                    @if let Some(u) = user.filter(|u| !u.is_anonymous()) {
                        p class="text-sm text-muted mb-4 font-bold" {
                            i class="fa-solid fa-user mr-1" {}
                            "Logged in as " (u.display_name())
                        }
                    }
                    div class="label-brutal text-xs mb-2" { "CURRENT BALANCE" }
                    div class="text-6xl font-black text-highlight orange" {
                        (balance_sats)
                        " "
                        i class="fa-solid fa-bolt" {}
                    }
                    div class="text-sm text-muted mt-2 font-bold" { "SATS" }
                }

                // Withdraw button (future feature)
                div class="p-6" style="border-top: 3px solid var(--accent-muted);" {
                    button disabled class="btn-brutal w-full opacity-50 cursor-not-allowed" {
                        i class="fa-solid fa-arrow-right-from-bracket mr-2" {}
                        "WITHDRAW (COMING SOON)"
                    }
                    p class="text-xs text-muted mt-2 text-center font-bold" {
                        "Lightning withdrawals will be available soon!"
                    }
                }
            }

            // Backup reminder for anonymous users
            @if user.map(|u| u.is_anonymous()).unwrap_or(true) {
                div class="card-brutal mb-6" style="border-color: var(--color-warning);" {
                    div class="p-4" {
                        h3 class="font-bold text-primary mb-2" {
                            i class="fa-solid fa-exclamation-triangle mr-2 text-highlight orange" {}
                            "BACKUP YOUR WALLET"
                        }
                        p class="text-sm text-secondary" {
                            "Your wallet is stored in this browser. Bookmark this page or save your wallet ID:"
                        }
                        @if let Some(u) = user {
                            div class="mt-3 p-2 font-mono text-xs break-all" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                                (u.id)
                            }
                        } @else {
                            div class="mt-3 p-2 text-sm text-muted" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                                "Collect some sats to create your wallet!"
                            }
                        }
                    }
                }
            }

            // Find more locations CTA
            div class="card-brutal mb-6" {
                div class="p-6 text-center" {
                    h3 class="font-bold text-primary mb-2" {
                        i class="fa-solid fa-map-location-dot mr-2 text-highlight orange" {}
                        "FIND MORE SATS"
                    }
                    p class="text-sm text-secondary mb-4" {
                        "Explore the map to find NFC stickers and collect more sats!"
                    }
                    a href="/map" class="btn-brutal-fill inline-block" style="background: var(--highlight); border-color: var(--highlight);" {
                        i class="fa-solid fa-map mr-2" {}
                        "VIEW MAP"
                    }
                }
            }

            // Transaction history
            div class="card-brutal" {
                h2 class="heading-breaker" {
                    i class="fa-solid fa-clock-rotate-left mr-2" {}
                    "TRANSACTION HISTORY"
                }

                @if transactions.is_empty() {
                    div class="p-6 text-center" {
                        p class="text-muted font-bold" { "No transactions yet." }
                        p class="text-sm text-muted mt-2" { "Go find some NFC stickers to collect sats!" }
                    }
                } @else {
                    div class="divide-y" style="border-color: var(--accent-muted);" {
                        @for tx in transactions {
                            div class="p-4 flex items-center justify-between" {
                                div {
                                    @if tx.is_collect() {
                                        span class="font-bold" style="color: var(--color-success);" {
                                            i class="fa-solid fa-arrow-down mr-2" {}
                                            "Collected"
                                        }
                                    } @else {
                                        span class="font-bold" style="color: var(--color-error);" {
                                            i class="fa-solid fa-arrow-up mr-2" {}
                                            "Withdrew"
                                        }
                                    }
                                    div class="text-xs text-muted mt-1 font-bold" {
                                        (tx.created_at.format("%Y-%m-%d %H:%M UTC"))
                                    }
                                }
                                div class="text-right" {
                                    @if tx.is_collect() {
                                        span class="font-bold text-lg" style="color: var(--color-success);" {
                                            "+" (tx.sats()) " sats"
                                        }
                                    } @else {
                                        span class="font-bold text-lg" style="color: var(--color-error);" {
                                            "-" (tx.sats()) " sats"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Store user ID in localStorage as backup
        @if let Some(u) = user {
            (PreEscaped(format!(r#"
            <script>
                // Backup user ID to localStorage
                localStorage.setItem('satshunt_uid', '{}');
            </script>
            "#, u.id)))
        }
    }
}
