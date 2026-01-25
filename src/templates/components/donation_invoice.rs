use maud::{html, Markup, PreEscaped};

/// Configuration for the donation invoice component
pub struct DonationInvoiceConfig<'a> {
    /// Prefix for element IDs to avoid collisions (e.g., "location" -> "locationInvoiceArea")
    pub id_prefix: &'a str,
    /// Optional location ID for location-specific donations
    pub location_id: Option<&'a str>,
    /// Available amount buttons as (value, label) pairs
    pub amounts: &'a [(&'a str, &'a str)],
    /// Optional label shown above the amount buttons
    pub label: Option<&'a str>,
}

impl Default for DonationInvoiceConfig<'_> {
    fn default() -> Self {
        Self {
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
            label: None,
        }
    }
}

/// Renders the donation amount selection buttons and invoice display area
pub fn donation_invoice_markup(config: &DonationInvoiceConfig) -> Markup {
    let prefix = config.id_prefix;
    let btn_class = if prefix.is_empty() {
        "amount-btn"
    } else {
        "location-amount-btn"
    };

    html! {
        // Amount selection
        div id={(prefix) "AmountSelection"} {
            @if let Some(label_text) = config.label {
                div class="label-brutal mb-4" { (label_text) }
            }
            div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-4" {
                @for (value, label) in config.amounts {
                    @if let Some(loc_id) = config.location_id {
                        button type="button" data-amount=(value) data-location-id=(loc_id)
                            class={(btn_class) " btn-brutal font-black"} {
                            (label)
                        }
                    } @else {
                        button type="button" data-amount=(value)
                            class={(btn_class) " btn-brutal font-black"} {
                            (label)
                        }
                    }
                }
            }

            // Custom amount input
            div id={(prefix) "CustomAmountDiv"} class="hidden mt-4" {
                div class="flex gap-2" {
                    input type="number" id={(prefix) "CustomAmount"} min="1" step="1"
                        class="flex-1 input-brutal-box"
                        placeholder="Enter amount in sats";
                    @if let Some(loc_id) = config.location_id {
                        button type="button" id={(prefix) "CustomSubmit"}
                            class="btn-brutal-orange"
                            data-location-id=(loc_id) {
                            "Create Invoice"
                        }
                    } @else {
                        button type="button" id={(prefix) "CustomSubmit"}
                            class="btn-brutal-orange" {
                            "Create Invoice"
                        }
                    }
                }
            }
        }

        // Invoice display area (also used for payment confirmation)
        div id={(prefix) "InvoiceArea"} class="hidden mt-6" {}
    }
}

/// Returns the JavaScript for the donation invoice component
pub fn donation_invoice_script(config: &DonationInvoiceConfig) -> Markup {
    let prefix = config.id_prefix;
    let btn_selector = if prefix.is_empty() {
        ".amount-btn"
    } else {
        ".location-amount-btn"
    };
    let fn_suffix = if prefix.is_empty() { "" } else { "Location" };
    let location_id_js = config
        .location_id
        .map(|id| format!("'{}'", id))
        .unwrap_or_else(|| "null".to_string());

    PreEscaped(format!(
        r#"
<script>
    (function() {{
        const prefix = '{prefix}';
        const fnSuffix = '{fn_suffix}';
        const locationId = {location_id_js};

        // Copy invoice to clipboard
        window['copy' + fnSuffix + 'Invoice'] = function() {{
            const invoiceText = document.getElementById(prefix + 'InvoiceText');
            invoiceText.select();
            navigator.clipboard.writeText(invoiceText.value).then(() => {{
                const btn = event.target.closest('button');
                const icon = btn.querySelector('i');
                icon.classList.remove('fa-copy');
                icon.classList.add('fa-check');
                setTimeout(() => {{
                    icon.classList.remove('fa-check');
                    icon.classList.add('fa-copy');
                }}, 1500);
            }});
        }};

        // Generate invoice function
        window['generate' + fnSuffix + 'Invoice'] = async function(amount) {{
            try {{
                // Hide amount selection
                document.getElementById(prefix + 'AmountSelection').classList.add('hidden');
                document.getElementById(prefix + 'CustomAmountDiv').classList.add('hidden');

                // Show loading
                const invoiceArea = document.getElementById(prefix + 'InvoiceArea');
                invoiceArea.innerHTML = `
                    <div class="text-center py-8">
                        <div class="animate-spin rounded-full h-12 w-12 border-b-2 mx-auto mb-4" style="border-color: var(--highlight);"></div>
                        <p class="text-secondary font-bold">Generating invoice...</p>
                    </div>
                `;
                invoiceArea.classList.remove('hidden');

                // Generate invoice
                const body = locationId ? {{ amount, location_id: locationId }} : {{ amount }};
                const response = await fetch('/api/donate/invoice', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify(body)
                }});

                if (!response.ok) {{
                    throw new Error('Failed to generate invoice');
                }}

                const data = await response.json();

                // Display invoice and QR code
                invoiceArea.innerHTML = `
                    <div class="p-6" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);">
                        <div class="text-center mb-4">
                            <p class="text-2xl font-black text-highlight orange">${{amount.toLocaleString()}} sats</p>
                            <p class="text-sm text-muted font-bold">Scan with your Lightning wallet</p>
                        </div>
                        <div class="flex justify-center">
                            <a href="lightning:${{data.invoice}}" class="bg-white p-4 inline-block hover:opacity-90 transition-opacity" style="border: 3px solid var(--accent-border);">
                                <img src="${{data.qr_code}}" alt="Invoice QR Code" class="w-48 h-48">
                            </a>
                        </div>
                        <div class="mt-4">
                            <label class="text-muted text-sm font-bold block mb-2">
                                Invoice string
                            </label>
                            <div class="flex gap-2">
                                <input type="text" readonly value="${{data.invoice}}"
                                    class="flex-1 p-3 text-xs font-mono text-secondary"
                                    style="background: var(--bg-secondary); border: 2px solid var(--accent-muted);"
                                    id="${{prefix}}InvoiceText">
                                <button type="button" onclick="copy${{fnSuffix}}Invoice()" class="btn-brutal-orange px-4">
                                    <i class="fa-solid fa-copy"></i>
                                </button>
                            </div>
                        </div>
                        <div class="mt-6 p-3 flex items-center gap-2" style="background: var(--highlight-glow); border: 2px solid var(--highlight);">
                            <i class="fa-solid fa-hourglass-half animate-pulse text-highlight"></i>
                            <span class="text-sm font-bold text-primary">Waiting for payment...</span>
                        </div>
                        ${{locationId ? '<button type="button" onclick="reset' + fnSuffix + 'Donation()" class="btn-brutal mt-4 w-full">Cancel</button>' : ''}}
                    </div>
                `;

                // Store pending invoice for visibility change handler
                window[prefix + 'PendingInvoice'] = {{
                    invoice: data.invoice,
                    amount: amount
                }};

                // Start waiting for payment - target the invoice area so confirmation replaces it
                const invoiceAreaForHtmx = document.getElementById(prefix + 'InvoiceArea');
                invoiceAreaForHtmx.setAttribute('hx-get', `/api/donate/wait/${{data.invoice}}:${{amount}}:${{prefix}}`);
                invoiceAreaForHtmx.setAttribute('hx-trigger', 'load');
                invoiceAreaForHtmx.setAttribute('hx-swap', 'innerHTML');
                htmx.process(invoiceAreaForHtmx);

            }} catch (error) {{
                console.error('Error:', error);
                document.getElementById(prefix + 'InvoiceArea').innerHTML = `
                    <div class="p-4" style="background: var(--highlight-glow); border: 2px solid var(--highlight);">
                        <p class="font-bold text-highlight">Error</p>
                        <p class="text-sm text-secondary">${{error.message}}</p>
                    </div>
                `;
                document.getElementById(prefix + 'AmountSelection').classList.remove('hidden');
            }}
        }};

        // Reset function (for location donations with cancel button)
        window['reset' + fnSuffix + 'Donation'] = function() {{
            document.getElementById(prefix + 'InvoiceArea').classList.add('hidden');
            document.getElementById(prefix + 'InvoiceArea').innerHTML = '';
            document.getElementById(prefix + 'AmountSelection').classList.remove('hidden');
            document.getElementById(prefix + 'CustomAmountDiv').classList.add('hidden');
            // Clear pending invoice
            delete window[prefix + 'PendingInvoice'];
        }};

        // Amount button click handlers
        document.querySelectorAll('{btn_selector}').forEach(button => {{
            button.addEventListener('click', async function() {{
                const amount = this.dataset.amount;
                if (amount === 'custom') {{
                    document.getElementById(prefix + 'CustomAmountDiv').classList.remove('hidden');
                }} else {{
                    await window['generate' + fnSuffix + 'Invoice'](parseInt(amount));
                }}
            }});
        }});

        // Custom amount submit
        document.getElementById(prefix + 'CustomSubmit').addEventListener('click', async function() {{
            const customAmount = parseInt(document.getElementById(prefix + 'CustomAmount').value);
            if (customAmount > 0) {{
                await window['generate' + fnSuffix + 'Invoice'](customAmount);
            }} else {{
                alert('Please enter a valid amount');
            }}
        }});

        // Re-trigger polling when page becomes visible again (mobile browser backgrounding)
        document.addEventListener('visibilitychange', function() {{
            if (document.visibilityState === 'visible') {{
                const pending = window[prefix + 'PendingInvoice'];
                if (pending) {{
                    const invoiceArea = document.getElementById(prefix + 'InvoiceArea');
                    if (invoiceArea && !invoiceArea.classList.contains('hidden')) {{
                        // Directly fetch and update instead of relying on HTMX re-trigger
                        fetch(`/api/donate/wait/${{pending.invoice}}:${{pending.amount}}:${{prefix}}`)
                            .then(response => response.text())
                            .then(html => {{
                                invoiceArea.innerHTML = html;
                                // Check if payment was received (no more pending invoice needed)
                                if (html.includes('Payment received')) {{
                                    delete window[prefix + 'PendingInvoice'];
                                }}
                            }})
                            .catch(err => console.error('Failed to check payment status:', err));
                    }}
                }}
            }}
        }});
    }})();
</script>
"#,
        prefix = prefix,
        fn_suffix = fn_suffix,
        location_id_js = location_id_js,
        btn_selector = btn_selector,
    ))
}
