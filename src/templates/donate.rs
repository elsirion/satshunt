use crate::models::DonationPool;
use maud::{html, Markup, PreEscaped};

pub fn donate(pool: &DonationPool) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-highlight" {
            i class="fa-solid fa-coins mr-2" {}
            "Donate to the Pool"
        }

        // Current pool stats
        div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
            h2 class="text-2xl font-bold mb-4 text-highlight" { "Current Donation Pool" }
            div class="text-center" {
                div class="text-6xl font-bold text-highlight mb-2" {
                    (pool.total_sats()) " "
                    i class="fa-solid fa-bolt" {}
                }
                p class="text-muted" { "Total sats available for refills" }
            }
        }

        // Why donate section
        div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
            h2 class="text-2xl font-bold mb-4 text-highlight" { "Why Donate?" }
            ul class="space-y-3 text-secondary" {
                li class="flex items-start" {
                    span class="text-highlight mr-2" {
                        i class="fa-solid fa-bolt" {}
                    }
                    "Keeps treasure locations refilling automatically"
                }
                li class="flex items-start" {
                    span class="text-highlight mr-2" {
                        i class="fa-solid fa-bolt" {}
                    }
                    "Enables new treasure hunters to find sats"
                }
                li class="flex items-start" {
                    span class="text-highlight mr-2" {
                        i class="fa-solid fa-bolt" {}
                    }
                    "Supports the community treasure hunt game"
                }
                li class="flex items-start" {
                    span class="text-highlight mr-2" {
                        i class="fa-solid fa-bolt" {}
                    }
                    "Locations refill at 1 sat per minute from this pool"
                }
            }
        }

        // Donation form
        div class="bg-secondary rounded-lg p-8 border border-accent-muted" {
            h2 class="text-2xl font-bold mb-6 text-highlight" { "Make a Donation" }

            div id="donationContainer" {
                // Amount selection
                div id="amountSelection" {
                    label class="block mb-4 text-sm font-medium text-primary" {
                        "Choose donation amount:"
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
                        label for="customAmount" class="block mb-2 text-sm font-medium text-primary" {
                            "Custom Amount (sats)"
                        }
                        div class="flex gap-2" {
                            input type="number" id="customAmount" min="1" step="1"
                                class="flex-1 bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent p-2.5"
                                placeholder="Enter amount in satoshis";
                            button type="button" id="customSubmit"
                                class="px-4 py-2 btn-primary" {
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
        div class="bg-secondary rounded-lg p-8 mt-8 border border-accent-muted" {
            h2 class="text-2xl font-bold mb-4 text-highlight" { "How Donations Work" }
            div class="space-y-4 text-secondary" {
                p {
                    "All donations go into a shared pool that automatically refills treasure locations. "
                    "Each location refills at a rate dependent on the current donation pool balance and its fill status and the maximum sats per location. "
                    "The formula will change over time to optimize for engagement and runway."
                }
                p {
                    "When treasure hunters scan an NFC tag and claim the sats, the location's balance is reset to zero. "
                    "It will start refilling again after a short delay."
                }
                p class="text-highlight font-semibold" {
                    "Your donation keeps the treasure hunt alive for everyone!"
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
            class="amount-btn px-4 py-3 bg-tertiary hover:bg-elevated text-primary font-semibold rounded-lg border border-accent-muted transition" {
            (label)
        }
    }
}
