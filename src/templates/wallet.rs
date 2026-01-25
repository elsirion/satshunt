use crate::models::{User, UserTransaction};
use maud::{html, Markup, PreEscaped};

/// Calculate the withdrawable amount after fees (2 sat fixed + 0.5% routing)
fn withdrawable_after_fees(balance_sats: i64) -> i64 {
    let balance_msats = balance_sats * 1000;
    let routing_fee_msats = ((balance_msats as f64) * 0.005).ceil() as i64;
    let fixed_fee_msats = 2000; // 2 sats
    let total_fee_msats = routing_fee_msats + fixed_fee_msats;
    ((balance_msats - total_fee_msats).max(0)) / 1000
}

/// Render the wallet page showing user's balance and transaction history.
pub fn wallet(
    balance_sats: i64,
    transactions: &[UserTransaction],
    user: Option<&User>,
    success: Option<&str>,
    amount: Option<i64>,
    location_name: Option<&str>,
    lnurlw_string: Option<&str>,
) -> Markup {
    let withdrawable_sats = withdrawable_after_fees(balance_sats);
    let fee_sats = balance_sats - withdrawable_sats;
    html! {
        div class="max-w-2xl mx-auto" {
            // Success message for collection
            @if let (Some("collected"), Some(amt)) = (success, amount) {
                div class="alert-brutal green success mb-6" {
                    "Collected " (amt) " sats"
                    @if let Some(name) = location_name {
                        " from " (name)
                    }
                    "!"
                }
            }

            // Success message for withdrawal
            @if let (Some("withdrawn"), Some(amt)) = (success, amount) {
                div class="alert-brutal green success mb-6" {
                    "Withdrew " (amt) " sats!"
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
                    div id="balance-display" class="text-6xl font-black text-highlight orange" {
                        (balance_sats)
                        " "
                        i class="fa-solid fa-bolt" {}
                    }
                    div class="text-sm text-muted mt-2 font-bold" { "SATS" }
                }

                // Withdraw section
                div class="p-6" style="border-top: 3px solid var(--accent-muted);" {
                    // Error message container (hidden by default)
                    div id="withdraw-error" class="alert-brutal mb-4 hidden" style="background: var(--color-error); border-color: var(--color-error);" {
                        p class="font-bold text-white" {
                            i class="fa-solid fa-exclamation-circle mr-2" {}
                            span id="withdraw-error-text" {}
                        }
                    }

                    // Success message container (hidden by default)
                    div id="withdraw-success" class="alert-brutal mb-4 hidden" style="background: var(--color-success); border-color: var(--color-success);" {
                        p class="font-bold text-white" {
                            i class="fa-solid fa-check-circle mr-2" {}
                            span id="withdraw-success-text" {}
                        }
                    }

                    @if balance_sats > 0 {
                        // Withdraw method tabs
                        div class="mb-4" {
                            div class="flex border-b-3" style="border-color: var(--accent-muted);" {
                                button
                                    id="tab-lnurl"
                                    class="withdraw-tab px-4 py-2 font-bold text-sm active"
                                    data-tab="lnurl"
                                    style="border-bottom: 3px solid var(--highlight); margin-bottom: -3px; color: var(--highlight);" {
                                    i class="fa-solid fa-bolt mr-2" {}
                                    "WALLET"
                                }
                                button
                                    id="tab-address"
                                    class="withdraw-tab px-4 py-2 font-bold text-sm"
                                    data-tab="address"
                                    style="border-bottom: 3px solid transparent; margin-bottom: -3px; color: var(--text-muted);" {
                                    i class="fa-solid fa-at mr-2" {}
                                    "LN ADDRESS"
                                }
                                button
                                    id="tab-invoice"
                                    class="withdraw-tab px-4 py-2 font-bold text-sm"
                                    data-tab="invoice"
                                    style="border-bottom: 3px solid transparent; margin-bottom: -3px; color: var(--text-muted);" {
                                    i class="fa-solid fa-paste mr-2" {}
                                    "INVOICE"
                                }
                            }
                        }

                        // Tab content: LNURL-withdraw link
                        div id="content-lnurl" class="withdraw-content" {
                            div class="text-center" {
                                @if withdrawable_sats > 0 {
                                    p class="text-sm text-secondary mb-2 font-bold" {
                                        "Open with your Lightning wallet to withdraw"
                                    }
                                    div class="mb-4 p-3" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                                        div class="text-2xl font-black text-highlight orange" {
                                            (withdrawable_sats) " sats"
                                        }
                                        div class="text-xs text-muted mt-1" {
                                            "(" (fee_sats) " sats fee: 2 sats + 0.5% routing)"
                                        }
                                    }
                                    @if let Some(lnurl) = lnurlw_string {
                                        a
                                            href={"lightning:" (lnurl)}
                                            class="btn-brutal-fill inline-block"
                                            style="background: var(--highlight); border-color: var(--highlight);" {
                                            i class="fa-solid fa-bolt mr-2" {}
                                            "OPEN IN WALLET"
                                        }
                                    } @else {
                                        p class="text-muted" { "LNURL not available" }
                                    }
                                } @else {
                                    p class="text-muted font-bold" {
                                        "Balance too low to withdraw (minimum ~3 sats to cover fees)"
                                    }
                                }
                            }
                        }

                        // Tab content: Lightning Address
                        div id="content-address" class="withdraw-content hidden" {
                            @if withdrawable_sats > 0 {
                                form id="withdraw-form-address" class="space-y-4" {
                                    div {
                                        label class="label-brutal text-xs mb-2 block" for="ln_address" {
                                            "LIGHTNING ADDRESS"
                                        }
                                        input
                                            type="text"
                                            id="ln_address"
                                            name="ln_address"
                                            placeholder="you@wallet.com"
                                            required
                                            class="input-brutal w-full"
                                            style="background: var(--bg-tertiary); border: 3px solid var(--accent-muted); padding: 12px; font-size: 16px;";
                                    }
                                    div class="p-3" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                                        div class="flex justify-between items-center" {
                                            span class="text-sm text-secondary font-bold" { "You'll receive:" }
                                            span class="text-lg font-black text-highlight orange" { (withdrawable_sats) " sats" }
                                        }
                                        div class="text-xs text-muted mt-1" {
                                            "(" (fee_sats) " sats fee: 2 sats + 0.5% routing)"
                                        }
                                    }
                                    button
                                        type="submit"
                                        id="withdraw-btn-address"
                                        class="btn-brutal-fill w-full"
                                        style="background: var(--highlight); border-color: var(--highlight);" {
                                        i class="fa-solid fa-arrow-right-from-bracket mr-2" {}
                                        "WITHDRAW " (withdrawable_sats) " SATS"
                                    }
                                }
                            } @else {
                                p class="text-muted font-bold text-center" {
                                    "Balance too low to withdraw (minimum ~3 sats to cover fees)"
                                }
                            }
                        }

                        // Tab content: Paste Invoice
                        div id="content-invoice" class="withdraw-content hidden" {
                            @if withdrawable_sats > 0 {
                                form id="withdraw-form-invoice" class="space-y-4" {
                                    div {
                                        label class="label-brutal text-xs mb-2 block" for="invoice" {
                                            "LIGHTNING INVOICE"
                                        }
                                        textarea
                                            id="invoice"
                                            name="invoice"
                                            placeholder="lnbc..."
                                            required
                                            rows="4"
                                            class="input-brutal w-full font-mono text-sm"
                                            style="background: var(--bg-tertiary); border: 3px solid var(--accent-muted); padding: 12px; resize: vertical;" {}
                                    }
                                    div class="p-3" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                                        p class="text-sm text-secondary font-bold mb-2" {
                                            "Create an invoice in your wallet and paste it here."
                                        }
                                        div class="flex justify-between items-center" {
                                            span class="text-sm text-secondary font-bold" { "Max withdrawal:" }
                                            span class="text-lg font-black text-highlight orange" { (withdrawable_sats) " sats" }
                                        }
                                        div class="text-xs text-muted mt-1" {
                                            "(" (fee_sats) " sats fee: 2 sats + 0.5% routing)"
                                        }
                                    }
                                    button
                                        type="submit"
                                        id="withdraw-btn-invoice"
                                        class="btn-brutal-fill w-full"
                                        style="background: var(--highlight); border-color: var(--highlight);" {
                                        i class="fa-solid fa-arrow-right-from-bracket mr-2" {}
                                        "PAY INVOICE"
                                    }
                                }
                            } @else {
                                p class="text-muted font-bold text-center" {
                                    "Balance too low to withdraw (minimum ~3 sats to cover fees)"
                                }
                            }
                        }
                    } @else {
                        div class="text-center" {
                            p class="text-muted font-bold" {
                                i class="fa-solid fa-coins mr-2" {}
                                "No balance to withdraw"
                            }
                            p class="text-xs text-muted mt-2" {
                                "Collect some sats from NFC stickers to start!"
                            }
                        }
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

        // Wallet scripts
        (PreEscaped(r#"
        <script>
            document.addEventListener('DOMContentLoaded', function() {
                // Tab switching
                const tabs = document.querySelectorAll('.withdraw-tab');
                const contents = document.querySelectorAll('.withdraw-content');

                tabs.forEach(tab => {
                    tab.addEventListener('click', function() {
                        const targetTab = this.dataset.tab;

                        // Update tab styles
                        tabs.forEach(t => {
                            t.style.borderBottomColor = 'transparent';
                            t.style.color = 'var(--text-muted)';
                        });
                        this.style.borderBottomColor = 'var(--highlight)';
                        this.style.color = 'var(--highlight)';

                        // Show/hide content
                        contents.forEach(c => c.classList.add('hidden'));
                        document.getElementById('content-' + targetTab).classList.remove('hidden');
                    });
                });

                // Helper function to handle withdrawal submission
                async function handleWithdraw(endpoint, body, btn) {
                    const errorDiv = document.getElementById('withdraw-error');
                    const errorText = document.getElementById('withdraw-error-text');
                    const successDiv = document.getElementById('withdraw-success');
                    const successText = document.getElementById('withdraw-success-text');

                    // Hide any previous messages
                    errorDiv.classList.add('hidden');
                    successDiv.classList.add('hidden');

                    // Disable button and show loading state
                    const originalText = btn.innerHTML;
                    btn.disabled = true;
                    btn.innerHTML = '<i class="fa-solid fa-spinner fa-spin mr-2"></i>WITHDRAWING...';

                    try {
                        const response = await fetch(endpoint, {
                            method: 'POST',
                            headers: {
                                'Content-Type': 'application/json',
                            },
                            body: JSON.stringify(body),
                        });

                        const data = await response.json();

                        if (data.success) {
                            // Show success message
                            successText.textContent = 'Withdrew ' + data.withdrawn_sats + ' sats!';
                            successDiv.classList.remove('hidden');

                            // Update balance display
                            const balanceDisplay = document.getElementById('balance-display');
                            if (balanceDisplay) {
                                balanceDisplay.innerHTML = data.new_balance_sats + ' <i class="fa-solid fa-bolt"></i>';
                            }

                            // Redirect to wallet page with success message after short delay
                            setTimeout(function() {
                                window.location.href = '/wallet?success=withdrawn&amount=' + data.withdrawn_sats;
                            }, 1500);
                        } else {
                            // Show error message
                            errorText.textContent = data.error || 'Withdrawal failed. Please try again.';
                            errorDiv.classList.remove('hidden');
                            btn.disabled = false;
                            btn.innerHTML = originalText;
                        }
                    } catch (err) {
                        errorText.textContent = 'Network error. Please try again.';
                        errorDiv.classList.remove('hidden');
                        btn.disabled = false;
                        btn.innerHTML = originalText;
                    }
                }

                // Lightning Address form submission
                const addressForm = document.getElementById('withdraw-form-address');
                if (addressForm) {
                    addressForm.addEventListener('submit', async function(e) {
                        e.preventDefault();
                        const lnAddress = document.getElementById('ln_address').value.trim();
                        const btn = document.getElementById('withdraw-btn-address');
                        await handleWithdraw('/api/wallet/withdraw', { ln_address: lnAddress }, btn);
                    });
                }

                // Invoice form submission
                const invoiceForm = document.getElementById('withdraw-form-invoice');
                if (invoiceForm) {
                    invoiceForm.addEventListener('submit', async function(e) {
                        e.preventDefault();
                        const invoice = document.getElementById('invoice').value.trim();
                        const btn = document.getElementById('withdraw-btn-invoice');
                        await handleWithdraw('/api/wallet/withdraw/invoice', { invoice: invoice }, btn);
                    });
                }
            });
        </script>
        "#))
    }
}
