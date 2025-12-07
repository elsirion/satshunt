use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn nfc_setup(location: &Location, write_token: &str, base_url: &str) -> Markup {
    let lnurlw_url = format!("{}/api/lnurlw/{}", base_url, location.id);
    let _setup_url = format!("{}/setup/{}", base_url, write_token);

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
                    ol class="list-decimal list-inside space-y-3 text-secondary" {
                        li { "Scan the QR code below with your NFC writing app (like Boltcard or LNbits NFC)" }
                        li { "Follow the app's instructions to write the LNURL to your NFC sticker" }
                        li { "Place the NFC sticker at the location: " em { (location.name) } }
                        li { "The location will start with 0 sats and refill from the donation pool" }
                    }
                }

                // QR Code section
                div class="bg-white rounded-lg p-8 mb-6 text-center" {
                    h3 class="text-inverse font-bold mb-4" { "One-Time Setup Link" }
                    img id="qrcode" src="" alt="Setup QR Code" class="mx-auto mb-4";
                    p class="text-muted text-sm mb-2" { "Scan with your NFC writing app" }

                    details class="mt-4" {
                        summary class="cursor-pointer text-tertiary hover:text-secondary" {
                            "Show LNURL (for manual entry)"
                        }
                        div class="mt-2 p-3 bg-elevated rounded text-xs font-mono break-all text-inverse" {
                            (lnurlw_url)
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
            // Generate QR code for the LNURL
            const canvas = document.createElement('canvas');
            const qr = new QRCode(canvas, {{
                text: '{}',
                width: 256,
                height: 256,
                colorDark: '#000000',
                colorLight: '#ffffff',
                correctLevel: QRCode.CorrectLevel.H
            }});

            // Convert canvas to image
            setTimeout(() => {{
                const img = document.getElementById('qrcode');
                img.src = canvas.toDataURL();
            }}, 100);
        </script>
        "#, lnurlw_url)))
    }
}
