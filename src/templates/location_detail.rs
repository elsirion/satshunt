use crate::models::{Location, Photo};
use maud::{html, Markup, PreEscaped};

pub fn location_detail(location: &Location, photos: &[Photo], base_url: &str, max_sats_per_location: i64) -> Markup {
    let sats_percent = if max_sats_per_location > 0 {
        (location.current_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    html! {
        div class="max-w-4xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-highlight hover:bg-accent-hover mb-6" {
                "← Back to map"
            }

            // Status banner for non-active locations
            @if !location.is_active() {
                div class={
                    "bg-secondary rounded-lg p-6 mb-6 border-2 "
                    @if location.is_created() { "border-yellow-600" } @else { "border-blue-600" }
                } {
                    div class="flex items-start gap-4" {
                        div class="flex-shrink-0" {
                            @if location.is_created() {
                                i class="fa-solid fa-clock text-4xl text-yellow-500" {}
                            } @else {
                                i class="fa-solid fa-hourglass-half text-4xl text-blue-500" {}
                            }
                        }
                        div class="flex-1" {
                            h3 class={
                                "text-xl font-bold mb-2 "
                                @if location.is_created() { "text-yellow-400" } @else { "text-blue-400" }
                            } {
                                @if location.is_created() {
                                    "Location Not Yet Programmed"
                                } @else {
                                    "Location Waiting for Activation"
                                }
                            }
                            @if location.is_created() {
                                p class="text-secondary mb-3" {
                                    "This location has been created but the NFC sticker has not been programmed yet. "
                                    "It will not appear on the public map until it's programmed and activated."
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            a href={"/setup/" (token)}
                                                class="inline-flex items-center px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg font-semibold transition-colors" {
                                                i class="fa-solid fa-microchip mr-2" {}
                                                "Program NFC Sticker"
                                            }
                                        }
                                    }
                                    button
                                        onclick={
                                            "if(confirm('Are you sure you want to delete this location? This cannot be undone.')) { "
                                            "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                            ".then(r => r.ok ? window.location.href='/profile' : alert('Failed to delete location')) "
                                            "}"
                                        }
                                        class="inline-flex items-center px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-semibold transition-colors" {
                                        i class="fa-solid fa-trash mr-2" {}
                                        "Delete"
                                    }
                                }
                            } @else {
                                p class="text-secondary mb-3" {
                                    "The NFC sticker has been programmed. This location will become active and appear on the public map "
                                    "after the first successful scan and withdrawal."
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            a href={"/setup/" (token)}
                                                class="inline-flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-colors" {
                                                i class="fa-solid fa-redo mr-2" {}
                                                "Re-program NFC Sticker"
                                            }
                                        }
                                    }
                                    button
                                        onclick={
                                            "if(confirm('Are you sure you want to delete this location? This cannot be undone.')) { "
                                            "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                            ".then(r => r.ok ? window.location.href='/profile' : alert('Failed to delete location')) "
                                            "}"
                                        }
                                        class="inline-flex items-center px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-semibold transition-colors" {
                                        i class="fa-solid fa-trash mr-2" {}
                                        "Delete"
                                    }
                                }
                                p class="text-secondary text-sm mt-2 italic" {
                                    "If the NFC write failed, you can try again with the same keys"
                                }
                            }
                        }
                    }
                }
            }

            // Location header
            div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
                div class="flex justify-between items-start mb-4" {
                    h1 class="text-4xl font-bold text-highlight" { (location.name) }

                    // Status badge
                    @if location.is_active() {
                        div class="px-3 py-1 rounded-full text-white text-sm font-semibold bg-green-600" {
                            i class="fa-solid fa-check mr-1" {}
                            "Active"
                        }
                    } @else if location.is_programmed() {
                        div class="px-3 py-1 rounded-full text-white text-sm font-semibold bg-blue-600" {
                            i class="fa-solid fa-microchip mr-1" {}
                            "Programmed"
                        }
                    } @else {
                        div class="px-3 py-1 rounded-full text-white text-sm font-semibold bg-yellow-600" {
                            i class="fa-solid fa-clock mr-1" {}
                            "Created"
                        }
                    }
                }

                @if let Some(desc) = &location.description {
                    p class="text-secondary mb-6" { (desc) }
                }

                // Stats grid
                div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6" {
                    div class="bg-tertiary rounded p-4" {
                        div class="text-sm text-muted mb-1" { "Available Sats" }
                        div class="text-2xl font-bold text-highlight" {
                            (location.current_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="bg-tertiary rounded p-4" {
                        div class="text-sm text-muted mb-1" { "Max Capacity" }
                        div class="text-2xl font-bold text-secondary" {
                            (max_sats_per_location) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="bg-tertiary rounded p-4" {
                        div class="text-sm text-muted mb-1" { "Fill Level" }
                        div class="text-2xl font-bold text-success" {
                            (sats_percent) "%"
                        }
                    }
                    div class="bg-tertiary rounded p-4" {
                        div class="text-sm text-muted mb-1" { "Coordinates" }
                        div class="text-sm font-mono text-secondary" {
                            (format!("{:.4}", location.latitude)) br;
                            (format!("{:.4}", location.longitude))
                        }
                    }
                }

                // Progress bar
                div class="w-full bg-tertiary rounded-full h-4" {
                    div class="bg-highlight h-4 rounded-full transition-all"
                        style=(format!("width: {}%", sats_percent)) {}
                }
            }

            // Photos
            @if !photos.is_empty() {
                div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
                    h2 class="text-2xl font-bold mb-4 text-highlight" {
                        i class="fa-solid fa-camera mr-2" {}
                        "Photos"
                    }
                    div class="grid grid-cols-1 md:grid-cols-3 gap-4" {
                        @for photo in photos {
                            img src={"/uploads/" (photo.file_path)}
                                alt="Location photo"
                                class="w-full h-48 object-cover rounded-lg border border-accent-muted";
                        }
                    }
                }
            }

            // Map
            div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
                h2 class="text-2xl font-bold mb-4 text-highlight" {
                    i class="fa-solid fa-map mr-2" {}
                    "Location"
                }
                div id="map" class="w-full h-64 rounded-lg border border-accent-muted" {}
            }

            // Testing section with LNURL-withdraw QR
            div class="bg-secondary rounded-lg p-8 border border-accent-muted border-dashed" {
                h2 class="text-2xl font-bold mb-4 text-highlight" {
                    i class="fa-solid fa-flask mr-2" {}
                    "Testing - LNURL Withdraw"
                }

                @if location.is_active() {
                    p class="text-secondary mb-4" {
                        "Scan this QR code with your Lightning wallet to test withdrawing sats from this location. "
                        "This is the same LNURL that's programmed on the NFC tag."
                    }
                } @else {
                    p class="text-secondary mb-4" {
                        "This is a preview of the LNURL-withdraw that will be available once the location is active. "
                        @if location.is_created() {
                            "You need to program the NFC sticker first."
                        } @else {
                            "The NFC sticker needs to be scanned once to activate this location."
                        }
                    }
                }

                @if location.current_sats == 0 {
                    div class="bg-warning border border-warning text-primary px-4 py-3 rounded-lg mb-6" {
                        p {
                            i class="fa-solid fa-triangle-exclamation mr-2" {}
                            "No sats available in this location. Wait for it to refill from the donation pool."
                        }
                    }
                }

                div class="flex flex-col md:flex-row gap-6 items-center" {
                    // QR Code
                    div class="bg-white p-4 rounded-lg" {
                        div id="lnurlQR" class="w-48 h-48" {}
                    }

                    // Details
                    div class="flex-1" {
                        div class="bg-tertiary rounded-lg p-4 mb-4" {
                            p class="text-sm text-muted mb-1" { "Available to Withdraw" }
                            p class="text-3xl font-bold text-highlight" {
                                (location.current_sats) " sats"
                            }
                        }
                        details class="bg-tertiary rounded-lg p-4" {
                            summary class="cursor-pointer text-secondary hover:text-primary font-semibold" {
                                "Show LNURL"
                            }
                            div class="mt-2 p-3 bg-secondary rounded text-xs font-mono break-all text-secondary" {
                                (format!("{}/api/lnurlw/{}", base_url, location.id))
                            }
                        }
                    }
                }
            }
        }

        // Map script
        (PreEscaped(format!(r#"
        <script src="https://cdn.jsdelivr.net/npm/qrcodejs@1.0.0/qrcode.min.js"></script>
        <script>
            // Initialize map
            const map = L.map('map').setView([{}, {}], 15);

            L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
                attribution: '© OpenStreetMap contributors',
                className: 'map-tiles'
            }}).addTo(map);

            const style = document.createElement('style');
            style.textContent = `
                .map-tiles {{
                    filter: invert(100%) hue-rotate(180deg) brightness(95%) contrast(90%);
                }}
            `;
            document.head.appendChild(style);

            L.marker([{}, {}]).addTo(map)
                .bindPopup('<b>{}</b><br>{} sats available')
                .openPopup();

            // Generate LNURL-withdraw QR code
            const lnurlQRDiv = document.getElementById('lnurlQR');
            if (lnurlQRDiv) {{
                const lnurlUrl = '{}/api/lnurlw/{}';
                new QRCode(lnurlQRDiv, {{
                    text: lnurlUrl,
                    width: 192,
                    height: 192,
                    colorDark: '#000000',
                    colorLight: '#ffffff',
                    correctLevel: QRCode.CorrectLevel.M
                }});
            }}
        </script>
        "#,
            location.latitude, location.longitude,
            location.latitude, location.longitude,
            location.name, location.current_sats,
            base_url, location.id
        )))
    }
}
