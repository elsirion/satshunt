use crate::models::Donation;
use maud::{html, Markup, PreEscaped};

/// pool_balance_sats: total pool balance across all locations in sats
/// num_locations: number of active locations
/// received_donations: list of received donations for display
pub fn donate(
    pool_balance_sats: i64,
    num_locations: usize,
    received_donations: &[Donation],
) -> Markup {
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
                // Amount selection
                div id="amountSelection" {
                    label class="label-brutal mb-4 block" {
                        "Choose donation amount"
                    }
                    div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4" {
                        (amount_button("1000", "1K sats"))
                        (amount_button("5000", "5K sats"))
                        (amount_button("10000", "10K sats"))
                        (amount_button("50000", "50K sats"))
                    }
                    div class="grid grid-cols-2 md:grid-cols-4 gap-4" {
                        (amount_button("100000", "100K sats"))
                        (amount_button("500000", "500K sats"))
                        (amount_button("1000000", "1M sats"))
                        (amount_button("custom", "Custom"))
                    }

                    // Custom amount
                    div id="customAmountDiv" class="hidden mt-4" {
                        label for="customAmount" class="label-brutal mb-2 block" {
                            "Custom amount (sats)"
                        }
                        div class="flex gap-2" {
                            input type="number" id="customAmount" min="1" step="1"
                                class="flex-1 input-brutal-box"
                                placeholder="Enter amount in sats";
                            button type="button" id="customSubmit"
                                class="btn-brutal-orange" {
                                "Create Invoice"
                            }
                        }
                    }
                }

                // Invoice display area (will be populated by HTMX)
                div id="invoiceArea" class="hidden mt-6" {}

                // Payment status area (will be populated by HTMX when payment received)
                div id="paymentStatus" {}
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

        // JavaScript for amount selection
        (PreEscaped(r#"
        <script>
            let selectedAmount = 0;

            // Amount button click handlers
            document.querySelectorAll('.amount-btn').forEach(button => {
                button.addEventListener('click', async function() {
                    const amount = this.dataset.amount;

                    if (amount === 'custom') {
                        // Show custom input
                        document.getElementById('customAmountDiv').classList.remove('hidden');
                        selectedAmount = 0;
                    } else {
                        // Generate invoice immediately
                        selectedAmount = parseInt(amount);
                        await generateInvoice(selectedAmount);
                    }
                });
            });

            // Custom amount submit
            document.getElementById('customSubmit').addEventListener('click', async function() {
                const customAmount = parseInt(document.getElementById('customAmount').value);
                if (customAmount > 0) {
                    selectedAmount = customAmount;
                    await generateInvoice(selectedAmount);
                } else {
                    alert('Please enter a valid amount');
                }
            });

            async function generateInvoice(amount) {
                try {
                    // Hide amount selection
                    document.getElementById('amountSelection').classList.add('hidden');

                    // Show loading
                    document.getElementById('invoiceArea').innerHTML = `
                        <div class="text-center py-8">
                            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-yellow-400 mx-auto mb-4"></div>
                            <p class="text-slate-300">Generating invoice...</p>
                        </div>
                    `;
                    document.getElementById('invoiceArea').classList.remove('hidden');

                    // Generate invoice
                    const response = await fetch('/api/donate/invoice', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json'
                        },
                        body: JSON.stringify({ amount: amount })
                    });

                    if (!response.ok) {
                        throw new Error('Failed to generate invoice');
                    }

                    const data = await response.json();

                    // Display invoice and QR code
                    document.getElementById('invoiceArea').innerHTML = `
                        <div class="bg-tertiary rounded-lg p-6">
                            <div class="text-center mb-4">
                                <p class="text-2xl font-bold text-highlight mb-2">${amount.toLocaleString()} sats</p>
                                <p class="text-sm text-muted">Scan with your Lightning wallet</p>
                            </div>
                            <div class="bg-white p-4 rounded-lg inline-block mx-auto block">
                                <img src="${data.qr_code}" alt="Invoice QR Code" class="w-64 h-64 mx-auto">
                            </div>
                            <details class="mt-4">
                                <summary class="cursor-pointer text-muted hover:text-secondary text-sm">
                                    Show invoice string
                                </summary>
                                <div class="mt-2 p-3 bg-secondary rounded text-xs font-mono break-all text-secondary">
                                    ${data.invoice}
                                </div>
                            </details>
                            <div class="mt-6 bg-info border border-info text-primary px-4 py-3 rounded-lg">
                                <p class="text-sm flex items-center">
                                    <i class="fa-solid fa-hourglass-half animate-pulse mr-2"></i>
                                    Waiting for payment...
                                </p>
                            </div>
                        </div>
                    `;

                    // Start waiting for payment with HTMX
                    const paymentStatusDiv = document.getElementById('paymentStatus');
                    paymentStatusDiv.setAttribute('hx-get', `/api/donate/wait/${data.invoice}:${amount}`);
                    paymentStatusDiv.setAttribute('hx-trigger', 'load');
                    paymentStatusDiv.setAttribute('hx-swap', 'innerHTML');
                    htmx.process(paymentStatusDiv);

                } catch (error) {
                    console.error('Error:', error);
                    document.getElementById('invoiceArea').innerHTML = `
                        <div class="bg-error border border-error text-primary px-4 py-3 rounded-lg">
                            <p class="font-semibold">Error</p>
                            <p class="text-sm">${error.message}</p>
                        </div>
                    `;
                    // Show amount selection again
                    document.getElementById('amountSelection').classList.remove('hidden');
                }
            }
        </script>
        "#))
    }
}

fn amount_button(amount: &str, label: &str) -> Markup {
    html! {
        button type="button" data-amount=(amount)
            class="amount-btn btn-brutal font-black" {
            (label)
        }
    }
}
