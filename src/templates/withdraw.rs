use crate::models::Location;
use maud::{html, Markup, PreEscaped};

/// Render the withdrawal page with multiple withdrawal options.
///
/// The page has three tabs: LN Address, WebLN, and Paste Invoice.
/// The SUN parameters (picc_data and cmac) are passed to each API call
/// for counter verification.
pub fn withdraw(
    location: &Location,
    withdrawable_sats: i64,
    picc_data: &str,
    cmac: &str,
    error: Option<&str>,
) -> Markup {
    html! {
        div class="max-w-2xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-highlight orange font-bold mb-6 hover:text-primary transition" {
                "< BACK TO MAP"
            }

            // Error message
            @if let Some(error_msg) = error {
                div class="alert-brutal orange mb-6" {
                    p class="font-bold" { " " (error_msg) }
                }
            }

            // Location header card
            div class="card-brutal mb-6" {
                h1 class="text-2xl font-black text-primary mb-2" {
                    "WITHDRAW FROM"
                }
                h2 class="text-3xl font-black text-highlight orange mb-6" {
                    (location.name)
                }

                // Available sats display
                div class="card-brutal-inset p-6 text-center" {
                    div class="label-brutal text-xs mb-2" { "AVAILABLE TO WITHDRAW" }
                    div class="text-5xl font-black text-highlight orange" {
                        (withdrawable_sats)
                        " "
                        i class="fa-solid fa-bolt" {}
                    }
                    div class="text-sm text-muted mt-2 font-bold" { "SATS" }
                }
            }

            // No sats available warning
            @if withdrawable_sats <= 0 {
                div class="card-brutal-inset p-6 text-center mb-6" {
                    p class="text-xl font-bold text-muted" {
                        "No sats available at this location."
                    }
                    p class="text-sm text-muted mt-2" {
                        "Check back later - locations refill automatically!"
                    }
                }
            } @else {
                // Withdrawal options card
                div class="card-brutal" {
                    h2 class="heading-breaker" {
                        i class="fa-solid fa-wallet mr-2" {}
                        "CHOOSE WITHDRAWAL METHOD"
                    }

                    // Tab navigation
                    div class="flex gap-2 mt-6 mb-6" {
                        button id="tab-ln-address" onclick="switchTab('ln-address')"
                            class="btn-brutal-fill flex-1" style="background: var(--highlight); border-color: var(--highlight);" {
                            i class="fa-solid fa-at mr-2" {}
                            "LN ADDRESS"
                        }
                        button id="tab-webln" onclick="switchTab('webln')"
                            class="btn-brutal flex-1" {
                            i class="fa-solid fa-bolt mr-2" {}
                            "WEBLN"
                        }
                        button id="tab-invoice" onclick="switchTab('invoice')"
                            class="btn-brutal flex-1" {
                            i class="fa-solid fa-paste mr-2" {}
                            "INVOICE"
                        }
                    }

                    // Tab content: LN Address
                    div id="content-ln-address" class="tab-content" {
                        div class="p-4" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                            p class="text-secondary font-bold mb-4" {
                                "Enter your Lightning Address to receive " (withdrawable_sats) " sats."
                            }
                            div class="space-y-4" {
                                div {
                                    label class="label-brutal" for="lnAddress" { "LIGHTNING ADDRESS" }
                                    input type="text" id="lnAddress" placeholder="satoshi@wallet.com"
                                        class="input-brutal-box w-full"
                                        autocomplete="off"
                                        autocapitalize="off";
                                    div class="text-xs text-muted mt-1 font-bold" {
                                        "Example: you@getalby.com, user@walletofsatoshi.com"
                                    }
                                }
                                button type="button" onclick="withdrawLnAddress()"
                                    id="btn-ln-address"
                                    class="btn-brutal-fill w-full" style="background: var(--highlight); border-color: var(--highlight);" {
                                    i class="fa-solid fa-paper-plane mr-2" {}
                                    "WITHDRAW " (withdrawable_sats) " SATS"
                                }
                            }
                        }
                    }

                    // Tab content: WebLN
                    div id="content-webln" class="tab-content hidden" {
                        div class="p-4" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                            div id="webln-available" {
                                p class="text-secondary font-bold mb-4" {
                                    "Connect your browser wallet to receive " (withdrawable_sats) " sats."
                                }
                                button type="button" onclick="withdrawWebLN()"
                                    id="btn-webln"
                                    class="btn-brutal-fill w-full" style="background: var(--highlight); border-color: var(--highlight);" {
                                    i class="fa-solid fa-bolt mr-2" {}
                                    "CONNECT WALLET & WITHDRAW"
                                }
                                div class="text-xs text-muted mt-3 font-bold" {
                                    "Requires a WebLN-compatible wallet like "
                                    a href="https://getalby.com" target="_blank" class="text-highlight" style="border-bottom: 1px solid var(--highlight);" { "Alby" }
                                    " browser extension."
                                }
                            }
                            div id="webln-unavailable" class="hidden text-center" {
                                p class="text-muted font-bold mb-4" {
                                    i class="fa-solid fa-exclamation-triangle mr-2" {}
                                    "WebLN not detected in your browser."
                                }
                                p class="text-sm text-muted" {
                                    "Install a WebLN wallet extension like "
                                    a href="https://getalby.com" target="_blank" class="text-highlight" style="border-bottom: 1px solid var(--highlight);" { "Alby" }
                                    " to use this feature."
                                }
                                div class="mt-4" {
                                    button type="button" onclick="switchTab('ln-address')"
                                        class="btn-brutal" {
                                        "USE LN ADDRESS INSTEAD"
                                    }
                                }
                            }
                        }
                    }

                    // Tab content: Paste Invoice
                    div id="content-invoice" class="tab-content hidden" {
                        div class="p-4" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                            p class="text-secondary font-bold mb-4" {
                                "Paste a Lightning invoice for exactly " (withdrawable_sats) " sats."
                            }
                            div class="space-y-4" {
                                div {
                                    label class="label-brutal" for="invoice" { "LIGHTNING INVOICE" }
                                    textarea id="invoice" rows="4" placeholder="lnbc..."
                                        class="input-brutal-box w-full font-mono text-sm resize-none" {}
                                    div class="text-xs text-muted mt-1 font-bold" {
                                        "Must be a valid BOLT11 invoice for the exact amount."
                                    }
                                }
                                button type="button" onclick="withdrawInvoice()"
                                    id="btn-invoice"
                                    class="btn-brutal-fill w-full" style="background: var(--highlight); border-color: var(--highlight);" {
                                    i class="fa-solid fa-paper-plane mr-2" {}
                                    "WITHDRAW " (withdrawable_sats) " SATS"
                                }
                            }
                        }
                    }

                    // Loading/processing state (hidden by default)
                    div id="processing-state" class="hidden p-6 text-center" {
                        i class="fa-solid fa-spinner fa-spin text-4xl text-highlight mb-4" {}
                        p class="text-xl font-bold text-primary" { "Processing withdrawal..." }
                        p class="text-sm text-muted mt-2" { "Please wait, do not close this page." }
                    }

                    // Error display
                    div id="withdraw-error" class="hidden mt-4" {
                        div class="alert-brutal orange" {
                            p id="withdraw-error-message" class="font-bold" {}
                        }
                    }
                }
            }

            // Location map
            div class="card-brutal-inset mt-6" {
                h2 class="heading-breaker" {
                    i class="fa-solid fa-map-marker-alt mr-2" {}
                    "LOCATION"
                }
                div id="map" class="w-full h-48 mt-6" style="border: 3px solid var(--accent-border);" {}
            }
        }

        // Map initialization
        (PreEscaped(format!(r#"
        <script>
            const map = new maplibregl.Map({{
                container: 'map',
                style: 'https://tiles.openfreemap.org/styles/positron',
                center: [{}, {}],
                zoom: 14
            }});

            new maplibregl.Marker()
                .setLngLat([{}, {}])
                .addTo(map);
        </script>
        "#, location.longitude, location.latitude, location.longitude, location.latitude)))

        // Withdrawal JavaScript
        (PreEscaped(format!(r#"
        <script>
            const locationId = "{}";
            const piccData = "{}";
            const cmac = "{}";
            const withdrawableSats = {};

            // Tab switching
            function switchTab(tabName) {{
                // Hide all content
                document.querySelectorAll('.tab-content').forEach(el => el.classList.add('hidden'));
                // Show selected content
                document.getElementById('content-' + tabName).classList.remove('hidden');

                // Update tab button styles
                ['ln-address', 'webln', 'invoice'].forEach(name => {{
                    const btn = document.getElementById('tab-' + name);
                    if (name === tabName) {{
                        btn.className = 'btn-brutal-fill flex-1';
                        btn.style.background = 'var(--highlight)';
                        btn.style.borderColor = 'var(--highlight)';
                    }} else {{
                        btn.className = 'btn-brutal flex-1';
                        btn.style.background = '';
                        btn.style.borderColor = '';
                    }}
                }});
            }}

            // Check WebLN availability
            if (typeof window.webln === 'undefined') {{
                document.getElementById('webln-available').classList.add('hidden');
                document.getElementById('webln-unavailable').classList.remove('hidden');
            }}

            function showProcessing() {{
                document.querySelectorAll('.tab-content').forEach(el => el.classList.add('hidden'));
                document.getElementById('processing-state').classList.remove('hidden');
                document.getElementById('withdraw-error').classList.add('hidden');
            }}

            function hideProcessing() {{
                document.getElementById('processing-state').classList.add('hidden');
            }}

            function showError(message) {{
                hideProcessing();
                document.getElementById('withdraw-error-message').textContent = message;
                document.getElementById('withdraw-error').classList.remove('hidden');
                // Show the active tab again
                const activeTab = document.querySelector('.btn-brutal-fill');
                if (activeTab) {{
                    const tabName = activeTab.id.replace('tab-', '');
                    document.getElementById('content-' + tabName).classList.remove('hidden');
                }}
            }}

            async function withdrawLnAddress() {{
                const address = document.getElementById('lnAddress').value.trim();
                if (!address) {{
                    showError('Please enter a Lightning address.');
                    return;
                }}

                if (!address.includes('@') || !address.includes('.')) {{
                    showError('Invalid Lightning address format. Use user@domain.com');
                    return;
                }}

                showProcessing();

                try {{
                    const response = await fetch(`/api/withdraw/${{locationId}}/ln-address?picc_data=${{encodeURIComponent(piccData)}}&cmac=${{encodeURIComponent(cmac)}}`, {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ ln_address: address }})
                    }});

                    const result = await response.json();

                    if (result.success) {{
                        window.location.href = result.redirect_url;
                    }} else {{
                        showError(result.error || 'Withdrawal failed. Please try again.');
                    }}
                }} catch (err) {{
                    showError('Request failed. Please check your connection and try again.');
                }}
            }}

            async function withdrawWebLN() {{
                if (typeof window.webln === 'undefined') {{
                    showError('WebLN not available. Please install Alby or similar.');
                    return;
                }}

                showProcessing();

                try {{
                    // Enable WebLN
                    await window.webln.enable();

                    // Request invoice from wallet
                    const invoiceRequest = await window.webln.makeInvoice({{
                        amount: withdrawableSats,
                        defaultMemo: 'SatsHunt withdrawal from {}'
                    }});

                    const invoice = invoiceRequest.paymentRequest;

                    // Submit invoice to our API
                    const response = await fetch(`/api/withdraw/${{locationId}}/invoice?picc_data=${{encodeURIComponent(piccData)}}&cmac=${{encodeURIComponent(cmac)}}`, {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ invoice: invoice }})
                    }});

                    const result = await response.json();

                    if (result.success) {{
                        window.location.href = result.redirect_url;
                    }} else {{
                        showError(result.error || 'Withdrawal failed. Please try again.');
                    }}
                }} catch (err) {{
                    if (err.message && err.message.includes('User rejected')) {{
                        showError('Wallet connection was rejected.');
                    }} else {{
                        showError('WebLN error: ' + (err.message || 'Unknown error'));
                    }}
                }}
            }}

            async function withdrawInvoice() {{
                const invoice = document.getElementById('invoice').value.trim();
                if (!invoice) {{
                    showError('Please paste a Lightning invoice.');
                    return;
                }}

                if (!invoice.toLowerCase().startsWith('lnbc')) {{
                    showError('Invalid invoice format. Must start with lnbc...');
                    return;
                }}

                showProcessing();

                try {{
                    const response = await fetch(`/api/withdraw/${{locationId}}/invoice?picc_data=${{encodeURIComponent(piccData)}}&cmac=${{encodeURIComponent(cmac)}}`, {{
                        method: 'POST',
                        headers: {{ 'Content-Type': 'application/json' }},
                        body: JSON.stringify({{ invoice: invoice }})
                    }});

                    const result = await response.json();

                    if (result.success) {{
                        window.location.href = result.redirect_url;
                    }} else {{
                        showError(result.error || 'Withdrawal failed. Please try again.');
                    }}
                }} catch (err) {{
                    showError('Request failed. Please check your connection and try again.');
                }}
            }}

            // Handle Enter key in inputs
            document.getElementById('lnAddress').addEventListener('keypress', function(e) {{
                if (e.key === 'Enter') withdrawLnAddress();
            }});
        </script>
        "#,
            location.id,
            picc_data,
            cmac,
            withdrawable_sats,
            location.name.replace("'", "\\'")
        )))
    }
}
