use crate::models::{Location, User};
use maud::{html, Markup, PreEscaped};

/// Parameters for the collection page template.
pub struct CollectParams<'a> {
    pub location: &'a Location,
    pub available_sats: i64,
    pub current_balance_sats: i64,
    /// The scan ID for claiming (None if scan failed/expired)
    pub scan_id: Option<&'a str>,
    pub error: Option<&'a str>,
    pub is_new_user: bool,
    pub user: Option<&'a User>,
}

/// Render the collection page for the custodial wallet system.
///
/// Users collect sats into their wallet balance instead of immediate Lightning withdrawal.
pub fn collect(params: CollectParams<'_>) -> Markup {
    let CollectParams {
        location,
        available_sats,
        current_balance_sats,
        scan_id,
        error,
        is_new_user,
        user,
    } = params;

    // Can only claim if we have a valid scan_id
    let can_claim = scan_id.is_some() && available_sats > 0;

    html! {
        div class="max-w-2xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-highlight orange font-bold mb-6 hover:text-primary transition" {
                "< BACK TO MAP"
            }

            // Error message
            @if let Some(error_msg) = error {
                div class="alert-brutal orange mb-6" {
                    p class="font-bold" { (error_msg) }
                }
            }

            // Location header card
            div class="card-brutal mb-6" {
                h1 class="text-2xl font-black text-primary mb-2" {
                    "COLLECT SATS"
                }
                h2 class="text-3xl font-black text-highlight orange mb-6" {
                    (location.name)
                }

                // Available sats display
                div class="card-brutal-inset p-6 text-center" {
                    div class="label-brutal text-xs mb-2" { "AVAILABLE TO COLLECT" }
                    div class="text-5xl font-black text-highlight orange" {
                        (available_sats)
                        " "
                        i class="fa-solid fa-bolt" {}
                    }
                    div class="text-sm text-muted mt-2 font-bold" { "SATS" }
                }
            }

            // No sats available warning
            @if available_sats <= 0 {
                div class="card-brutal-inset p-6 text-center mb-6" {
                    p class="text-xl font-bold text-muted" {
                        "No sats available at this location."
                    }
                    p class="text-sm text-muted mt-2" {
                        "Check back later - locations refill automatically!"
                    }
                }
            } @else if !can_claim {
                // Has sats but no valid scan
                div class="card-brutal-inset p-6 text-center mb-6" {
                    p class="text-xl font-bold text-muted" {
                        "Scan the NFC sticker to collect sats."
                    }
                    p class="text-sm text-muted mt-2" {
                        "Hold your phone near the sticker to scan."
                    }
                }
            } @else {
                // Collection card
                div class="card-brutal mb-6" {
                    h2 class="heading-breaker" {
                        i class="fa-solid fa-hand-holding-dollar mr-2" {}
                        "COLLECT TO WALLET"
                    }

                    div class="p-4" style="background: var(--bg-tertiary); border-top: none;" {
                        p class="text-secondary font-bold mb-4 text-center" {
                            "Tap the button to add " (available_sats) " sats to your SatsHunt wallet."
                        }

                        // Collect button
                        button id="collect-btn" onclick="claimSats()"
                            class="btn-brutal-fill w-full text-xl py-4" style="background: var(--highlight); border-color: var(--highlight);" {
                            i class="fa-solid fa-bolt mr-3" {}
                            "COLLECT " (available_sats) " SATS"
                        }

                        // Processing state (hidden by default)
                        div id="processing-state" class="hidden text-center py-6" {
                            i class="fa-solid fa-spinner fa-spin text-4xl text-highlight mb-4" {}
                            p class="text-xl font-bold text-primary" { "Collecting sats..." }
                            p class="text-sm text-muted mt-2" { "Please wait, do not close this page." }
                        }

                        // Error display
                        div id="collect-error" class="hidden mt-4" {
                            div class="alert-brutal orange" {
                                p id="collect-error-message" class="font-bold" {}
                            }
                        }
                    }
                }
            }

            // Current balance card
            div class="card-brutal mb-6" {
                h2 class="heading-breaker" {
                    i class="fa-solid fa-wallet mr-2" {}
                    @if let Some(u) = user.filter(|u| !u.is_anonymous()) {
                        (u.display_name()) "'S WALLET"
                    } @else {
                        "YOUR WALLET"
                    }
                }

                div class="p-6 text-center" {
                    @if let Some(u) = user.filter(|u| !u.is_anonymous()) {
                        p class="text-sm text-muted mb-2 font-bold" {
                            i class="fa-solid fa-user mr-1" {}
                            "Logged in as " (u.display_name())
                        }
                    }
                    div class="text-4xl font-black text-primary" {
                        (current_balance_sats) " "
                        i class="fa-solid fa-bolt" {}
                    }
                    p class="text-sm text-muted mt-2 font-bold" { "CURRENT BALANCE" }

                    @if is_new_user {
                        div class="mt-4 p-4" style="background: var(--bg-elevated); border: 2px solid var(--accent-muted);" {
                            p class="text-sm text-secondary font-bold" {
                                i class="fa-solid fa-info-circle mr-2 text-highlight" {}
                                "Your wallet is saved in this browser. Visit your wallet page to track your sats!"
                            }
                        }
                    }

                    a href="/wallet" class="btn-brutal mt-4 inline-block" {
                        i class="fa-solid fa-wallet mr-2" {}
                        "VIEW WALLET"
                    }
                }
            }

            // Location map
            div class="card-brutal-inset" {
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

        // Claim JavaScript (only if we have a scan_id)
        @if let Some(sid) = scan_id {
            (PreEscaped(format!(r#"
            <script>
                const scanId = "{}";

                function showProcessing() {{
                    document.getElementById('collect-btn').classList.add('hidden');
                    document.getElementById('processing-state').classList.remove('hidden');
                    document.getElementById('collect-error').classList.add('hidden');
                }}

                function hideProcessing() {{
                    document.getElementById('processing-state').classList.add('hidden');
                    document.getElementById('collect-btn').classList.remove('hidden');
                }}

                function showError(message) {{
                    hideProcessing();
                    document.getElementById('collect-error-message').textContent = message;
                    document.getElementById('collect-error').classList.remove('hidden');
                }}

                async function claimSats() {{
                    showProcessing();

                    try {{
                        const response = await fetch(
                            `/api/claim/${{scanId}}`,
                            {{ method: 'POST' }}
                        );

                        const result = await response.json();

                        if (result.success) {{
                            // Store user_id in localStorage as backup
                            if (result.user_id) {{
                                localStorage.setItem('satshunt_uid', result.user_id);
                            }}
                            // Redirect to wallet with success message
                            window.location.href = `/wallet?success=collected&amount=${{result.collected_sats}}&location=${{encodeURIComponent(result.location_name || 'this location')}}`;
                        }} else {{
                            showError(result.error || 'Collection failed. Please try again.');
                        }}
                    }} catch (err) {{
                        console.error('Claim error:', err);
                        showError('Request failed. Please check your connection and try again.');
                    }}
                }}
            </script>
            "#, sid)))
        }
    }
}
