use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn nfc_setup(location: &Location, write_token: &str, base_url: &str) -> Markup {
    let lnurlw_url = format!("{}/api/lnurlw/{}", base_url, location.id);
    let setup_url = format!("{}/setup/{}", base_url, write_token);

    html! {
        div class="max-w-2xl mx-auto" {
            div class="bg-slate-800 rounded-lg p-8 border border-slate-700" {
                h1 class="text-4xl font-bold mb-6 text-yellow-400" { "🏷️ NFC Sticker Setup" }

                div class="bg-green-900 border border-green-700 text-green-200 px-4 py-3 rounded-lg mb-6" {
                    p class="font-semibold" { "✓ Location created successfully!" }
                }

                p class="text-slate-300 mb-6" {
                    "Your location \"" (location.name) "\" has been created. Now you need to write the LNURL-withdraw link to an NFC sticker."
                }

                // Instructions
                div class="bg-slate-700 rounded-lg p-6 mb-6" {
                    h2 class="text-xl font-bold mb-4 text-yellow-400" { "Setup Instructions" }
                    ol class="list-decimal list-inside space-y-3 text-slate-300" {
                        li { "Scan the QR code below with your NFC writing app (like Boltcard or LNbits NFC)" }
                        li { "Follow the app's instructions to write the LNURL to your NFC sticker" }
                        li { "Place the NFC sticker at the location: " em { (location.name) } }
                        li { "The location will start with 0 sats and refill from the donation pool" }
                    }
                }

                // QR Code section
                div class="bg-white rounded-lg p-8 mb-6 text-center" {
                    h3 class="text-slate-900 font-bold mb-4" { "One-Time Setup Link" }
                    img id="qrcode" src="" alt="Setup QR Code" class="mx-auto mb-4";
                    p class="text-slate-600 text-sm mb-2" { "Scan with your NFC writing app" }

                    details class="mt-4" {
                        summary class="cursor-pointer text-slate-700 hover:text-slate-900" {
                            "Show LNURL (for manual entry)"
                        }
                        div class="mt-2 p-3 bg-slate-100 rounded text-xs font-mono break-all text-slate-900" {
                            (lnurlw_url)
                        }
                    }
                }

                // Warning
                div class="bg-yellow-900 border border-yellow-700 text-yellow-200 px-4 py-3 rounded-lg mb-6" {
                    p class="font-semibold mb-2" { "⚠️ Important" }
                    ul class="list-disc list-inside text-sm space-y-1" {
                        li { "This setup link can only be used once" }
                        li { "After writing the NFC sticker, this page will no longer be accessible" }
                        li { "Make sure to test the NFC sticker before leaving the location" }
                    }
                }

                // Actions
                div class="flex gap-4" {
                    a href={"/locations/" (location.id)}
                        class="flex-1 px-6 py-3 bg-slate-700 hover:bg-slate-600 text-center text-slate-200 font-semibold rounded-lg transition" {
                        "View Location"
                    }
                    a href="/map"
                        class="flex-1 px-6 py-3 bg-yellow-500 hover:bg-yellow-600 text-center text-slate-900 font-semibold rounded-lg transition" {
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
