use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn nfc_setup(location: &Location, write_token: &str, base_url: &str) -> Markup {
    let lnurlw_url = format!("{}/api/lnurlw/{}", base_url, location.id);
    let _setup_url = format!("{}/setup/{}", base_url, write_token);

    // Generate Boltcard deep links
    let keys_request_url = format!("{}/api/boltcard/{}?onExisting=UpdateVersion", base_url, write_token);
    let keys_request_url_encoded = urlencoding::encode(&keys_request_url);
    let boltcard_program_link = format!("boltcard://program?url={}", keys_request_url_encoded);
    let boltcard_reset_link = format!("boltcard://reset?url={}", keys_request_url_encoded);

    html! {
        div class="max-w-2xl mx-auto" {
            div class="bg-secondary rounded-lg p-8 border border-accent-muted" {
                h1 class="text-4xl font-bold mb-6 text-highlight" {
                    i class="fa-solid fa-tag mr-2" {}
                    "NFC Sticker Setup"
                }

                div class="bg-success border border-success text-primary px-4 py-3 rounded-lg mb-6" {
                    p class="font-semibold" {
                        i class="fa-solid fa-check mr-2" {}
                        "Location created successfully!"
                    }
                }

                p class="text-secondary mb-6" {
                    "Your location \"" (location.name) "\" has been created. Now you need to write the LNURL-withdraw link to an NFC sticker."
                }

                // Instructions
                div class="bg-tertiary rounded-lg p-6 mb-6" {
                    h2 class="text-xl font-bold mb-4 text-highlight" { "Setup Instructions" }

                    div class="mb-4" {
                        h3 class="text-lg font-semibold mb-2 text-highlight" {
                            i class="fa-solid fa-mobile-screen mr-2" {}
                            "Method 1: Boltcard NFC Programmer (Recommended)"
                        }
                        ol class="list-decimal list-inside space-y-3 text-secondary ml-4" {
                            li { "Install the Boltcard NFC Programmer app on your phone" }
                            li { "Click the \"Setup Boltcard\" button below" }
                            li { "Tap your NFC sticker to your phone when prompted" }
                            li { "Place the NFC sticker at the location: " em { (location.name) } }
                        }
                    }

                    div {
                        h3 class="text-lg font-semibold mb-2 text-highlight" {
                            i class="fa-solid fa-qrcode mr-2" {}
                            "Method 2: Manual LNURL-w Setup"
                        }
                        ol class="list-decimal list-inside space-y-3 text-secondary ml-4" {
                            li { "Scan the QR code below with your NFC writing app (like LNbits NFC)" }
                            li { "Follow the app's instructions to write the LNURL to your NFC sticker" }
                            li { "Place the NFC sticker at the location: " em { (location.name) } }
                        }
                        p class="text-sm text-muted mt-2" {
                            i class="fa-solid fa-info-circle mr-1" {}
                            "Note: This method is simpler but doesn't support advanced features like counter-based security"
                        }
                    }
                }

                // Boltcard Deep Links section
                div class="bg-white rounded-lg p-8 mb-6" {
                    h3 class="text-inverse font-bold mb-4 text-center" {
                        i class="fa-solid fa-mobile-screen mr-2" {}
                        "Boltcard NFC Programmer"
                    }

                    div class="flex gap-4 mb-4 justify-center" {
                        a href=(boltcard_program_link)
                            class="flex-1 max-w-xs px-6 py-4 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-center font-semibold transition-colors" {
                            i class="fa-solid fa-plus mr-2" {}
                            "Setup Boltcard"
                        }
                        a href=(boltcard_reset_link)
                            class="flex-1 max-w-xs px-6 py-4 bg-orange-600 hover:bg-orange-700 text-white rounded-lg text-center font-semibold transition-colors" {
                            i class="fa-solid fa-rotate-right mr-2" {}
                            "Reset Boltcard"
                        }
                    }

                    p class="text-sm text-center text-gray-600" {
                        "Tap a button to open the Boltcard NFC Programmer app"
                    }
                }

                // QR Code section (fallback method)
                details class="bg-white rounded-lg p-8 mb-6" id="qr-details" {
                    summary class="cursor-pointer text-inverse hover:text-gray-700 font-bold text-center" {
                        i class="fa-solid fa-qrcode mr-2" {}
                        "Show QR Code (Manual Method)"
                    }
                    div class="mt-4 text-center" {
                        div id="qrcode" class="mx-auto mb-4 flex justify-center" {}
                        p class="text-gray-600 text-sm mb-2" { "Scan with your NFC writing app" }

                        div class="mt-4" {
                            p class="text-sm font-semibold text-gray-700 mb-2" { "LNURL (for manual entry):" }
                            div class="p-3 bg-gray-100 rounded text-xs font-mono break-all text-gray-800" {
                                (lnurlw_url)
                            }
                        }
                    }
                }

                // Warning
                div class="bg-warning border border-warning text-primary px-4 py-3 rounded-lg mb-6" {
                    p class="font-semibold mb-2" {
                        i class="fa-solid fa-triangle-exclamation mr-2" {}
                        "Important"
                    }
                    ul class="list-disc list-inside text-sm space-y-1" {
                        li { "This setup link can only be used once" }
                        li { "After writing the NFC sticker, this page will no longer be accessible" }
                        li { "Make sure to test the NFC sticker before leaving the location" }
                    }
                }

                // Actions
                div class="flex gap-4" {
                    a href={"/locations/" (location.id)}
                        class="flex-1 px-6 py-3 btn-secondary text-center" {
                        "View Location"
                    }
                    a href="/map"
                        class="flex-1 px-6 py-3 btn-primary text-center" {
                        "Back to Map"
                    }
                }
            }
        }

        // QR code generation
        (PreEscaped(format!(r#"
        <script src="https://cdn.jsdelivr.net/npm/qrcodejs@1.0.0/qrcode.min.js"></script>
        <script>
            // Generate QR code when details are opened
            let qrGenerated = false;

            function generateQR() {{
                if (qrGenerated) return;

                const qrContainer = document.getElementById('qrcode');
                if (qrContainer) {{
                    new QRCode(qrContainer, {{
                        text: '{}',
                        width: 256,
                        height: 256,
                        colorDark: '#000000',
                        colorLight: '#ffffff',
                        correctLevel: QRCode.CorrectLevel.H
                    }});
                    qrGenerated = true;
                }}
            }}

            // Generate on details toggle
            const details = document.getElementById('qr-details');
            if (details) {{
                details.addEventListener('toggle', function() {{
                    if (this.open) {{
                        setTimeout(generateQR, 100);
                    }}
                }});
            }}

            // Also generate immediately in case details are opened by default
            setTimeout(generateQR, 500);
        </script>
        "#, lnurlw_url)))
    }
}
