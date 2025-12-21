use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn nfc_setup(location: &Location, write_token: &str, base_url: &str) -> Markup {
    let _lnurlw_url = format!("{}/api/lnurlw/{}", base_url, location.id);
    let _setup_url = format!("{}/setup/{}", base_url, write_token);

    // Generate Boltcard deep links
    let keys_request_url = format!(
        "{}/api/boltcard/{}?onExisting=UpdateVersion",
        base_url, write_token
    );
    let keys_request_url_encoded = urlencoding::encode(&keys_request_url);
    let boltcard_program_link = format!("boltcard://program?url={}", keys_request_url_encoded);
    let boltcard_reset_link = format!("boltcard://reset?url={}", keys_request_url_encoded);

    html! {
        div class="max-w-2xl mx-auto" {
            div class="card-brutal" {
                h1 class="text-4xl font-black mb-6 text-primary" style="letter-spacing: -0.02em;" {
                    i class="fa-solid fa-tag mr-2" {}
                    "NFC STICKER SETUP"
                }

                div class="alert-brutal mb-6" {
                    p class="font-bold" {
                        i class="fa-solid fa-check mr-2" {}
                        "LOCATION CREATED SUCCESSFULLY!"
                    }
                }

                p class="text-secondary mb-6 font-bold" {
                    "YOUR LOCATION \"" (location.name) "\" HAS BEEN CREATED. NOW YOU NEED TO WRITE THE LNURL-WITHDRAW LINK TO AN NFC STICKER."
                }

                // Instructions
                div class="card-brutal-inset mb-6" {
                    h2 class="heading-breaker" { "SETUP INSTRUCTIONS" }

                    div class="mb-4 mt-8" {
                        h3 class="text-base font-black mb-3 text-primary" {
                            i class="fa-solid fa-mobile-screen mr-2" {}
                            "METHOD 1: BOLTCARD NFC PROGRAMMER (RECOMMENDED)"
                        }
                        ol class="list-decimal list-inside space-y-3 text-secondary ml-4 font-bold text-sm" {
                            li { "INSTALL THE BOLTCARD NFC PROGRAMMER APP ON YOUR PHONE" }
                            li { "CLICK THE \"SETUP BOLTCARD\" BUTTON BELOW" }
                            li { "TAP YOUR NFC STICKER TO YOUR PHONE WHEN PROMPTED" }
                            li { "PLACE THE NFC STICKER AT THE LOCATION: " span class="mono" { (location.name) } }
                        }
                    }

                    div {
                        h3 class="text-base font-black mb-3 text-primary" {
                            i class="fa-solid fa-qrcode mr-2" {}
                            "METHOD 2: MANUAL LNURL-W SETUP"
                        }
                        ol class="list-decimal list-inside space-y-3 text-secondary ml-4 font-bold text-sm" {
                            li { "SCAN THE QR CODE BELOW WITH YOUR NFC WRITING APP (LIKE LNBITS NFC)" }
                            li { "FOLLOW THE APP'S INSTRUCTIONS TO WRITE THE LNURL TO YOUR NFC STICKER" }
                            li { "PLACE THE NFC STICKER AT THE LOCATION: " span class="mono" { (location.name) } }
                        }
                        p class="text-xs text-muted mt-2 font-bold mono" {
                            i class="fa-solid fa-info-circle mr-1" {}
                            "NOTE: THIS METHOD IS SIMPLER BUT DOESN'T SUPPORT ADVANCED FEATURES LIKE COUNTER-BASED SECURITY"
                        }
                    }
                }

                // Boltcard Deep Links section
                div class="card-brutal mb-6" style="background-color: #ffffff; color: #000000;" {
                    h3 class="font-black mb-4 text-center text-lg" style="color: #000000;" {
                        i class="fa-solid fa-mobile-screen mr-2" {}
                        "BOLTCARD NFC PROGRAMMER"
                    }

                    div class="flex gap-4 mb-4 justify-center" {
                        a href=(boltcard_program_link)
                            class="flex-1 max-w-xs btn-brutal-fill text-center" {
                            i class="fa-solid fa-plus mr-2" {}
                            "SETUP BOLTCARD"
                        }
                        a href=(boltcard_reset_link)
                            class="flex-1 max-w-xs btn-brutal-orange text-center" {
                            i class="fa-solid fa-rotate-right mr-2" {}
                            "RESET BOLTCARD"
                        }
                    }

                    p class="text-xs text-center font-bold mono" style="color: #000000;" {
                        "TAP A BUTTON TO OPEN THE BOLTCARD NFC PROGRAMMER APP"
                    }
                }

                // QR Code section (fallback method)
                details class="card-brutal mb-6" id="qr-details" style="background-color: #ffffff; color: #000000;" {
                    summary class="cursor-pointer hover:opacity-70 font-black text-center" style="color: #000000;" {
                        i class="fa-solid fa-qrcode mr-2" {}
                        "SHOW QR CODE (MANUAL METHOD)"
                    }
                    div class="mt-4 text-center" {
                        div id="qrcode" class="mx-auto mb-4 flex justify-center" {}
                        p class="text-xs mb-2 font-bold mono" style="color: #000000;" { "SCAN WITH BOLTCARD NFC PROGRAMMER APP" }

                        div class="mt-4" {
                            p class="text-xs font-black mb-2" style="color: #000000;" { "KEYS REQUEST URL (FOR MANUAL ENTRY):" }
                            div class="p-3 text-xs mono break-all" style="background-color: #f0f0f0; color: #000000; border: 2px solid #000000;" {
                                (keys_request_url)
                            }
                        }
                    }
                }

                // Warning
                div class="alert-brutal orange mb-6" {
                    p class="font-bold mb-2" {
                        i class="fa-solid fa-triangle-exclamation mr-2" {}
                        "IMPORTANT"
                    }
                    ul class="list-disc list-inside text-xs space-y-1 font-bold" {
                        li { "YOU CAN RETRY PROGRAMMING IF THE NFC WRITE FAILS" }
                        li { "THE SAME KEYS WILL BE USED FOR RETRIES UNTIL THE LOCATION IS ACTIVATED" }
                        li { "THIS LINK BECOMES INVALID AFTER THE FIRST SUCCESSFUL SCAN OF THE NFC STICKER" }
                        li { "MAKE SURE TO TEST THE NFC STICKER BEFORE LEAVING THE LOCATION" }
                    }
                }

                // Actions
                div class="flex gap-4" {
                    a href={"/locations/" (location.id)}
                        class="flex-1 btn-brutal text-center" {
                        "VIEW LOCATION"
                    }
                    a href="/map"
                        class="flex-1 btn-brutal-fill text-center" {
                        "BACK TO MAP"
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
                        correctLevel: QRCode.CorrectLevel.M
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
        "#, boltcard_program_link)))
    }
}
