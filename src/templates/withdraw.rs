use crate::models::Location;
use maud::{html, Markup, PreEscaped};

/// Render the withdrawal page with multiple withdrawal options.
///
/// The page has three tabs: LN Address, LNURL, and Paste Invoice.
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
                        button id="tab-lnurl" onclick="switchTab('lnurl')"
                            class="btn-brutal-fill flex-1" style="background: var(--highlight); border-color: var(--highlight);" {
                            i class="fa-solid fa-link mr-2" {}
                            "LNURL"
                        }
                        button id="tab-ln-address" onclick="switchTab('ln-address')"
                            class="btn-brutal flex-1" {
                            i class="fa-solid fa-at mr-2" {}
                            "LN ADDRESS"
                        }
                        button id="tab-invoice" onclick="switchTab('invoice')"
                            class="btn-brutal flex-1" {
                            i class="fa-solid fa-paste mr-2" {}
                            "INVOICE"
                        }
                    }

                    // Tab content: LNURL (default)
                    div id="content-lnurl" class="tab-content" {
                        div class="p-4" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                            p class="text-secondary font-bold mb-4" {
                                "Use LNURL to withdraw " (withdrawable_sats) " sats with any Lightning wallet."
                            }
                            div class="space-y-3" {
                                a id="lnurl-link" href="#"
                                    class="btn-brutal-fill w-full block text-center" style="background: var(--highlight); border-color: var(--highlight); text-decoration: none;" {
                                    i class="fa-solid fa-external-link-alt mr-2" {}
                                    "OPEN IN WALLET"
                                }
                                div class="text-xs text-muted mt-2 font-bold text-center" {
                                    "Works with any LNURL-compatible wallet (Zeus, Phoenix, Breez, etc.)"
                                }
                            }
                        }
                    }

                    // Tab content: LN Address
                    div id="content-ln-address" class="tab-content hidden" {
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
                ['lnurl', 'ln-address', 'invoice'].forEach(name => {{
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

            // Bech32 encoding for LNURL
            const CHARSET = 'qpzry9x8gf2tvdw0s3jn54khce6mua7l';

            function bech32Polymod(values) {{
                const GEN = [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3];
                let chk = 1;
                for (const v of values) {{
                    const b = chk >> 25;
                    chk = ((chk & 0x1ffffff) << 5) ^ v;
                    for (let i = 0; i < 5; i++) {{
                        if ((b >> i) & 1) {{
                            chk ^= GEN[i];
                        }}
                    }}
                }}
                return chk;
            }}

            function bech32HrpExpand(hrp) {{
                const ret = [];
                for (const c of hrp) {{
                    ret.push(c.charCodeAt(0) >> 5);
                }}
                ret.push(0);
                for (const c of hrp) {{
                    ret.push(c.charCodeAt(0) & 31);
                }}
                return ret;
            }}

            function bech32CreateChecksum(hrp, data) {{
                const values = bech32HrpExpand(hrp).concat(data).concat([0, 0, 0, 0, 0, 0]);
                const polymod = bech32Polymod(values) ^ 1;
                const ret = [];
                for (let i = 0; i < 6; i++) {{
                    ret.push((polymod >> (5 * (5 - i))) & 31);
                }}
                return ret;
            }}

            function bech32Encode(hrp, data) {{
                const combined = data.concat(bech32CreateChecksum(hrp, data));
                let ret = hrp + '1';
                for (const d of combined) {{
                    ret += CHARSET[d];
                }}
                return ret;
            }}

            function convertBits(data, fromBits, toBits, pad) {{
                let acc = 0;
                let bits = 0;
                const ret = [];
                const maxv = (1 << toBits) - 1;
                for (const value of data) {{
                    acc = (acc << fromBits) | value;
                    bits += fromBits;
                    while (bits >= toBits) {{
                        bits -= toBits;
                        ret.push((acc >> bits) & maxv);
                    }}
                }}
                if (pad) {{
                    if (bits > 0) {{
                        ret.push((acc << (toBits - bits)) & maxv);
                    }}
                }}
                return ret;
            }}

            function urlToLnurl(url) {{
                const data = new TextEncoder().encode(url);
                const converted = convertBits(Array.from(data), 8, 5, true);
                return bech32Encode('lnurl', converted).toUpperCase();
            }}

            // Generate LNURL on page load
            (function() {{
                const lnurlApiUrl = `/api/lnurlw/${{locationId}}?p=${{encodeURIComponent(piccData)}}&c=${{encodeURIComponent(cmac)}}`;
                const fullUrl = window.location.origin + lnurlApiUrl;
                const lnurlString = urlToLnurl(fullUrl);
                document.getElementById('lnurl-link').href = 'lightning:' + lnurlString;
            }})()

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
            withdrawable_sats
        )))
    }
}
