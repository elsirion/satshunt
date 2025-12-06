use crate::models::{Location, Photo};
use maud::{html, Markup, PreEscaped};

pub fn location_detail(location: &Location, photos: &[Photo], base_url: &str) -> Markup {
    let sats_percent = if location.max_sats > 0 {
        (location.current_sats as f64 / location.max_sats as f64 * 100.0) as i32
    } else {
        0
    };

    html! {
        div class="max-w-4xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-yellow-400 hover:text-yellow-300 mb-6" {
                "← Back to map"
            }

            // Location header
            div class="bg-slate-800 rounded-lg p-8 mb-8 border border-slate-700" {
                h1 class="text-4xl font-bold mb-4 text-yellow-400" { (location.name) }

                @if let Some(desc) = &location.description {
                    p class="text-slate-300 mb-6" { (desc) }
                }

                // Stats grid
                div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6" {
                    div class="bg-slate-700 rounded p-4" {
                        div class="text-sm text-slate-400 mb-1" { "Available Sats" }
                        div class="text-2xl font-bold text-yellow-400" {
                            (location.current_sats) " ⚡"
                        }
                    }
                    div class="bg-slate-700 rounded p-4" {
                        div class="text-sm text-slate-400 mb-1" { "Max Capacity" }
                        div class="text-2xl font-bold text-slate-300" {
                            (location.max_sats) " ⚡"
                        }
                    }
                    div class="bg-slate-700 rounded p-4" {
                        div class="text-sm text-slate-400 mb-1" { "Fill Level" }
                        div class="text-2xl font-bold text-green-400" {
                            (sats_percent) "%"
                        }
                    }
                    div class="bg-slate-700 rounded p-4" {
                        div class="text-sm text-slate-400 mb-1" { "Coordinates" }
                        div class="text-sm font-mono text-slate-300" {
                            (format!("{:.4}", location.latitude)) br;
                            (format!("{:.4}", location.longitude))
                        }
                    }
                }

                // Progress bar
                div class="w-full bg-slate-700 rounded-full h-4" {
                    div class="bg-yellow-500 h-4 rounded-full transition-all"
                        style=(format!("width: {}%", sats_percent)) {}
                }
            }

            // Photos
            @if !photos.is_empty() {
                div class="bg-slate-800 rounded-lg p-8 mb-8 border border-slate-700" {
                    h2 class="text-2xl font-bold mb-4 text-yellow-400" { "📸 Photos" }
                    div class="grid grid-cols-1 md:grid-cols-3 gap-4" {
                        @for photo in photos {
                            img src={"/uploads/" (photo.file_path)}
                                alt="Location photo"
                                class="w-full h-48 object-cover rounded-lg border border-slate-600";
                        }
                    }
                }
            }

            // Map
            div class="bg-slate-800 rounded-lg p-8 mb-8 border border-slate-700" {
                h2 class="text-2xl font-bold mb-4 text-yellow-400" { "🗺️ Location" }
                div id="map" class="w-full h-64 rounded-lg border border-slate-700" {}
            }

            // Testing section with LNURL-withdraw QR
            div class="bg-slate-800 rounded-lg p-8 border border-slate-700 border-dashed" {
                h2 class="text-2xl font-bold mb-4 text-yellow-400" { "🧪 Testing - LNURL Withdraw" }
                p class="text-slate-300 mb-4" {
                    "Scan this QR code with your Lightning wallet to test withdrawing sats from this location. "
                    "In production, this would be written to an NFC tag."
                }

                @if location.current_sats == 0 {
                    div class="bg-yellow-900 border border-yellow-700 text-yellow-200 px-4 py-3 rounded-lg mb-6" {
                        p { "⚠️ No sats available in this location. Wait for it to refill from the donation pool." }
                    }
                }

                div class="flex flex-col md:flex-row gap-6 items-center" {
                    // QR Code
                    div class="bg-white p-4 rounded-lg" {
                        div id="lnurlQR" class="w-48 h-48" {}
                    }

                    // Details
                    div class="flex-1" {
                        div class="bg-slate-700 rounded-lg p-4 mb-4" {
                            p class="text-sm text-slate-400 mb-1" { "Available to Withdraw" }
                            p class="text-3xl font-bold text-yellow-400" {
                                (location.current_sats) " sats"
                            }
                        }
                        details class="bg-slate-700 rounded-lg p-4" {
                            summary class="cursor-pointer text-slate-300 hover:text-slate-200 font-semibold" {
                                "Show LNURL"
                            }
                            div class="mt-2 p-3 bg-slate-800 rounded text-xs font-mono break-all text-slate-300" {
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
