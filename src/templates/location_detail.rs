use super::format_sats_si;
use crate::models::{Donation, Location, NfcCard, Photo, ScanWithUser, UserRole};
use crate::templates::components::{
    donation_invoice_markup, donation_invoice_script, DonationInvoiceConfig,
};
use maud::{html, Markup, PreEscaped};

#[allow(clippy::too_many_arguments)] // All parameters are needed for the template
pub fn location_detail(
    location: &Location,
    photos: &[Photo],
    scans: &[ScanWithUser],
    available_sats: i64,
    pool_sats: i64,
    current_user_id: Option<&str>,
    current_user_role: UserRole,
    error: Option<&str>,
    success: Option<&str>,
    withdrawn_amount: Option<i64>,
    base_url: &str,
    donations: &[Donation],
    nfc_card: Option<&NfcCard>,
) -> Markup {
    // Max fill = 10% of pool, fill percentage based on available vs max fill
    let max_fill_sats = (pool_sats as f64 * 0.1) as i64;
    let sats_percent = if max_fill_sats > 0 {
        ((available_sats as f64 / max_fill_sats as f64) * 100.0).min(100.0) as i32
    } else {
        0
    };

    let is_owner = current_user_id
        .map(|id| id == location.user_id)
        .unwrap_or(false);
    let is_admin = current_user_role == UserRole::Admin;
    let can_manage_photos = is_owner && !location.is_active();

    // Generate Boltcard deep links for NFC programming and reset
    let boltcard_program_deep_link = location.write_token.as_ref().map(|token| {
        let keys_request_url = format!(
            "{}/api/boltcard/{}?onExisting=UpdateVersion",
            base_url, token
        );
        let keys_request_url_encoded = urlencoding::encode(&keys_request_url);
        format!("boltcard://program?url={}", keys_request_url_encoded)
    });

    html! {
        div class="max-w-4xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-highlight orange font-bold mb-6 hover:text-primary transition" {
                "← BACK TO MAP"
            }

            // Error message
            @if let Some(error_msg) = error {
                div class="alert-brutal orange mb-6" {
                    p class="font-bold" { " " (error_msg) }
                }
            }

            // Success message (e.g., after withdrawal)
            @if success == Some("withdrawn") {
                div class="mb-6 p-4" style="background: var(--highlight-glow); border: 3px solid var(--highlight);" {
                    div class="flex items-center gap-3" {
                        i class="fa-solid fa-check-circle text-2xl text-highlight" {}
                        div {
                            p class="font-black text-highlight text-lg" {
                                "SUCCESS!"
                            }
                            p class="text-primary font-bold" {
                                @if let Some(amount) = withdrawn_amount {
                                    (amount) " sats sent to your wallet."
                                } @else {
                                    "Sats sent to your wallet."
                                }
                            }
                        }
                    }
                }
            }

            // Next step banner for non-active locations (owner only, not for deactivated)
            @if is_owner && !location.is_active() && !location.is_deactivated() && !location.is_admin_deactivated() {
                div class="mb-6 p-6" style="background: var(--highlight-glow); border: 3px solid var(--highlight);" {
                    // Step 1: Upload photo
                    @if photos.is_empty() {
                        div class="flex flex-col md:flex-row md:items-center justify-between gap-4" {
                            div class="flex items-center gap-4" {
                                div class="w-12 h-12 flex items-center justify-center text-2xl font-black mono" style="background: var(--highlight); color: var(--text-inverse);" { "1" }
                                div {
                                    div class="text-xs text-highlight font-black mb-1" style="letter-spacing: 0.1em;" { "NEXT STEP" }
                                    div class="text-xl font-black text-primary" { "UPLOAD A PHOTO" }
                                    div class="text-sm text-secondary font-bold mt-1" { "Add at least one photo so people can find this location" }
                                }
                            }
                            div class="flex gap-2" {
                                button type="button" onclick="document.getElementById('photoInput').click()" class="btn-brutal-fill" style="background: var(--highlight); border-color: var(--highlight);" {
                                    i class="fa-solid fa-camera mr-2" {}
                                    "ADD PHOTO"
                                }
                                (delete_button(&location.id))
                            }
                        }
                    }
                    // Step 2: Program NFC - inline UI
                    @else if location.is_created() {
                        div class="space-y-4" {
                            div class="flex flex-col md:flex-row md:items-center justify-between gap-4" {
                                div class="flex items-center gap-4" {
                                    div class="w-12 h-12 flex items-center justify-center text-2xl font-black mono" style="background: var(--highlight); color: var(--text-inverse);" { "2" }
                                    div {
                                        div class="text-xs text-highlight font-black mb-1" style="letter-spacing: 0.1em;" { "NEXT STEP" }
                                        div class="text-xl font-black text-primary" { "PROGRAM NFC STICKER" }
                                    }
                                }
                                (delete_button(&location.id))
                            }

                            // NFC Programming UI
                            @if let Some(ref deep_link) = boltcard_program_deep_link {
                                div class="p-4" style="background: var(--bg-secondary); border: 2px solid var(--accent-muted);" {
                                    p class="text-primary font-bold mb-3" {
                                        "Tap the button below with the "
                                        span class="text-highlight" { "Boltcard NFC Programmer" }
                                        " app, then hold your NFC sticker to your phone."
                                    }
                                    div class="flex flex-wrap gap-2 mb-3" {
                                        a href=(deep_link) class="btn-brutal-fill text-center" style="background: var(--highlight); border-color: var(--highlight);" {
                                            i class="fa-solid fa-microchip mr-2" {}
                                            "PROGRAM NFC"
                                        }
                                    }
                                    div class="text-xs text-muted font-bold" {
                                        "Don't have the app? "
                                        a href="https://apps.apple.com/app/boltcard-nfc-programmer/id6450968873" target="_blank" class="text-highlight" style="border-bottom: 1px solid var(--highlight);" { "iOS" }
                                        " · "
                                        a href="https://play.google.com/store/apps/details?id=com.lightningnfcapp" target="_blank" class="text-highlight" style="border-bottom: 1px solid var(--highlight);" { "Android" }
                                    }
                                }
                            }
                        }
                    }
                    // Step 3: Waiting for activation scan
                    @else {
                        div class="flex flex-col md:flex-row md:items-center justify-between gap-4" {
                            div class="flex items-center gap-4" {
                                div class="w-12 h-12 flex items-center justify-center text-2xl font-black mono" style="background: var(--highlight); color: var(--text-inverse);" { "3" }
                                div {
                                    div class="text-xs text-highlight font-black mb-1" style="letter-spacing: 0.1em;" { "NEXT STEP" }
                                    div class="text-xl font-black text-primary" { "SCAN NFC TO ACTIVATE" }
                                    div class="text-sm text-secondary font-bold mt-1" { "Scan the NFC sticker with a Lightning wallet to go live" }
                                }
                            }
                            div class="flex gap-2" {
                                div class="flex items-center gap-2 px-4 py-2 font-black" style="border: 3px solid var(--highlight); color: var(--highlight);" {
                                    i class="fa-solid fa-hourglass-half" {}
                                    "WAITING FOR SCAN"
                                }
                                (delete_button(&location.id))
                            }
                        }
                    }
                }
            }

            // Location header
            div class="card-brutal mb-8" {
                div class="flex justify-between items-start mb-4" {
                    h1 class="text-4xl font-black text-primary" { (location.name) }

                    // Status badge and deactivate/reactivate controls
                    div class="flex items-center gap-2" {
                        @if is_owner || is_admin {
                            @if location.is_active() {
                                button
                                    onclick={
                                        "if(confirm('DEACTIVATE THIS LOCATION? Users will no longer be able to collect sats.')) { "
                                        "fetch('/api/locations/" (location.id) "/deactivate', { method: 'POST' }) "
                                        ".then(r => r.ok ? location.reload() : alert('FAILED TO DEACTIVATE')) "
                                        "}"
                                    }
                                    class="btn-brutal" style="border-color: var(--accent-muted); color: var(--text-muted); padding: 0.25rem 0.5rem;"
                                    title=(if is_admin && !is_owner { "Admin deactivate" } else { "Deactivate" }) {
                                    i class="fa-solid fa-pause" {}
                                }
                            } @else if location.is_deactivated() {
                                button
                                    onclick={
                                        "fetch('/api/locations/" (location.id) "/reactivate', { method: 'POST' }) "
                                        ".then(r => r.ok ? location.reload() : alert('FAILED TO REACTIVATE')) "
                                    }
                                    class="btn-brutal-fill" style="padding: 0.25rem 0.5rem;"
                                    title="Reactivate" {
                                    i class="fa-solid fa-play" {}
                                }
                            } @else if location.is_admin_deactivated() {
                                @if is_admin {
                                    button
                                        onclick={
                                            "fetch('/api/locations/" (location.id) "/reactivate', { method: 'POST' }) "
                                            ".then(r => r.ok ? location.reload() : alert('FAILED TO REACTIVATE')) "
                                        }
                                        class="btn-brutal-fill" style="padding: 0.25rem 0.5rem;"
                                        title="Admin reactivate" {
                                        i class="fa-solid fa-play" {}
                                    }
                                } @else {
                                    span class="btn-brutal" style="border-color: var(--accent-muted); color: var(--text-muted); padding: 0.25rem 0.5rem; cursor: not-allowed;"
                                        title="Contact admin to reactivate" {
                                        i class="fa-solid fa-lock" {}
                                    }
                                }
                            }
                        }

                        // Status badge
                        @if location.is_active() {
                            span class="badge-brutal filled" { "ACTIVE" }
                        } @else if location.is_deactivated() {
                            span class="badge-brutal grey" {
                                i class="fa-solid fa-pause mr-1" {}
                                "DEACTIVATED"
                            }
                        } @else if location.is_admin_deactivated() {
                            span class="badge-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                                i class="fa-solid fa-ban mr-1" {}
                                "ADMIN DEACTIVATED"
                            }
                        } @else if location.is_programmed() {
                            span class="badge-brutal grey" { "PROGRAMMED" }
                        } @else {
                            span class="badge-brutal white" { "CREATED" }
                        }
                    }
                }

                @if let Some(desc) = &location.description {
                    p class="text-secondary mb-6 font-bold" { (desc) }
                }

                // Stats grid
                div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6" {
                    div class="card-brutal-inset p-4" {
                        div class="label-brutal text-xs mb-2" { "AVAILABLE SATS" }
                        div class="text-2xl font-black text-highlight orange" {
                            (format_sats_si(available_sats)) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="card-brutal-inset p-4" {
                        div class="label-brutal text-xs mb-2" { "POOL BALANCE" }
                        div class="text-2xl font-black text-secondary" {
                            (format_sats_si(pool_sats)) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="card-brutal-inset p-4" {
                        div class="label-brutal text-xs mb-2" { "FILL LEVEL" }
                        div class="text-2xl font-black text-primary" {
                            (sats_percent) "%"
                        }
                    }
                    div class="card-brutal-inset p-4" {
                        div class="label-brutal text-xs mb-2" { "COORDINATES" }
                        div class="text-sm mono text-secondary font-bold" {
                            (format!("{:.4}", location.latitude)) br;
                            (format!("{:.4}", location.longitude))
                        }
                    }
                }

                // Progress bar
                div class="progress-brutal" {
                    @if sats_percent > 50 {
                        div class="progress-brutal-bar" style=(format!("width: {}%", sats_percent)) {
                            div class="progress-brutal-value" { (sats_percent) "%" }
                        }
                    } @else {
                        div class="progress-brutal-bar orange" style=(format!("width: {}%", sats_percent)) {
                            div class="progress-brutal-value" { (sats_percent) "%" }
                        }
                    }
                }

            }

            // Map
            div class="card-brutal-inset mb-8" {
                h2 class="heading-breaker" {
                    i class="fa-solid fa-map mr-2" {}
                    "LOCATION"
                }
                div id="map" class="w-full h-64 mt-8" style="border: 3px solid var(--accent-border);" {}
            }

            // Photos
            div class="card-brutal-inset mb-8" {
                h2 class="heading-breaker" {
                    i class="fa-solid fa-camera mr-2" {}
                    "PHOTOS"
                }

                @if !photos.is_empty() {
                    div id="photosGrid" class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6 mt-8" {
                        @for photo in photos {
                            div class="relative group" {
                                img src={"/uploads/" (photo.file_path)}
                                    alt="Location photo"
                                    class="w-full h-48 object-cover cursor-pointer hover:opacity-90 transition-opacity"
                                    style="border: 3px solid var(--accent-muted);"
                                    onclick={"openPhotoViewer('/uploads/" (photo.file_path) "')"};
                                @if can_manage_photos {
                                    button
                                        onclick={
                                            "event.stopPropagation(); \
                                            if(confirm('DELETE THIS PHOTO? THIS CANNOT BE UNDONE.')) { \
                                            fetch('/api/photos/" (photo.id) "', { method: 'DELETE' }) \
                                            .then(r => r.ok ? location.reload() : alert('FAILED TO DELETE PHOTO')) \
                                            }"
                                        }
                                        class="absolute top-2 right-2 btn-brutal opacity-0 group-hover:opacity-100 transition-opacity" style="border-color: var(--highlight); color: var(--highlight); background: var(--bg-primary);" {
                                        i class="fa-solid fa-trash" {}
                                    }
                                }
                            }
                        }
                    }
                } @else {
                    p class="text-muted mb-6 mt-8 font-bold" { "NO PHOTOS YET." }
                }

                @if can_manage_photos {
                    div class="pt-6 mt-6" style="border-top: 3px solid var(--accent-muted);" {
                        // Hidden file input
                        input type="file" id="photoInput" name="photo" accept="image/*" class="hidden";
                        // Upload button that triggers file input
                        button type="button" id="addPhotoBtn" onclick="document.getElementById('photoInput').click()"
                            class="btn-brutal-orange" {
                            i class="fa-solid fa-camera mr-2" {}
                            "ADD PHOTO"
                        }
                        // Loading state (hidden by default)
                        div id="uploadingState" class="hidden flex items-center gap-2 text-highlight font-bold" {
                            i class="fa-solid fa-spinner fa-spin mr-2" {}
                            "UPLOADING..."
                        }
                    }
                }
            }

            // Donation Pool Section
            div class="card-brutal-inset mb-8" {
                h2 class="heading-breaker orange" {
                    i class="fa-solid fa-coins mr-2" {}
                    "DONATION POOL"
                }

                div class="mt-8" {
                    // Current pool balance
                    div class="flex items-center justify-between mb-6 p-4" style="background: var(--bg-tertiary); border: 2px solid var(--accent-muted);" {
                        div {
                            div class="label-brutal text-xs mb-1" { "POOL BALANCE" }
                            div class="text-3xl font-black text-highlight orange" {
                                (format_sats_si(pool_sats)) " "
                                i class="fa-solid fa-bolt" {}
                            }
                        }
                        div class="text-right" {
                            div class="text-sm text-muted font-bold" { "DEDICATED TO THIS LOCATION" }
                            div class="text-xs text-secondary font-bold mt-1" { "Fills location over time" }
                        }
                    }

                    // Donation form
                    div id="donationContainer" {
                        (donation_invoice_markup(&DonationInvoiceConfig {
                            id_prefix: "location",
                            location_id: Some(&location.id),
                            amounts: &[
                                ("1000", "1K"),
                                ("5000", "5K"),
                                ("10000", "10K"),
                                ("custom", "Custom"),
                            ],
                            label: Some("Donate to this location"),
                        }))
                    }
                }
            }

            // Recent Donations History (default collapsed)
            @if !donations.is_empty() {
                div class="card-brutal-inset mb-8" {
                    details {
                        summary class="text-2xl font-black text-primary cursor-pointer select-none hover:text-highlight transition-colors" {
                            i class="fa-solid fa-gift mr-2 text-highlight orange" {}
                            "DONATION HISTORY "
                            span class="text-base text-muted mono" { "[" (donations.len()) " DONATIONS]" }
                        }

                        div class="overflow-x-auto mt-4" {
                            table class="w-full" {
                                thead {
                                    tr style="border-bottom: 2px solid var(--accent-muted);" {
                                        th class="text-left py-3 px-4 text-secondary font-black" { "DATE" }
                                        th class="text-left py-3 px-4 text-secondary font-black" { "SOURCE" }
                                        th class="text-right py-3 px-4 text-secondary font-black" { "AMOUNT" }
                                    }
                                }
                                tbody {
                                    @for donation in donations {
                                        @let is_global = donation.invoice.contains("-split-");
                                        tr style="border-bottom: 2px solid var(--accent-muted);" class="hover:bg-tertiary transition-colors" {
                                            td class="py-3 px-4 text-secondary font-bold mono text-sm" {
                                                @if let Some(received_at) = donation.received_at {
                                                    (received_at.format("%Y-%m-%d %H:%M UTC"))
                                                }
                                            }
                                            td class="py-3 px-4 text-sm font-bold" {
                                                @if is_global {
                                                    span class="text-muted" {
                                                        i class="fa-solid fa-globe mr-1" {}
                                                        "Global"
                                                    }
                                                } @else {
                                                    span class="text-highlight orange" {
                                                        i class="fa-solid fa-location-dot mr-1" {}
                                                        "Direct"
                                                    }
                                                }
                                            }
                                            td class="py-3 px-4 text-right mono" {
                                                span class="text-highlight orange font-black" {
                                                    (format_sats_si(donation.amount_sats()))
                                                }
                                                " "
                                                i class="fa-solid fa-bolt text-highlight orange" {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Scan History
            @if !scans.is_empty() {
                div class="card-brutal-inset mb-8" {
                    h2 class="heading-breaker" {
                        i class="fa-solid fa-history mr-2" {}
                        "SCAN HISTORY"
                    }

                    div class="overflow-x-auto mt-8" {
                        table class="w-full" {
                            thead {
                                tr style="border-bottom: 2px solid var(--accent-muted);" {
                                    th class="text-left py-3 px-4 text-secondary font-black" { "DATE" }
                                    th class="text-left py-3 px-4 text-secondary font-black" { "SCANNER" }
                                    th class="text-right py-3 px-4 text-secondary font-black" { "STATUS" }
                                }
                            }
                            tbody {
                                @for scan in scans {
                                    tr style="border-bottom: 2px solid var(--accent-muted);" class="hover:bg-tertiary transition-colors" {
                                        td class="py-3 px-4 text-secondary font-bold mono text-sm" {
                                            (scan.scanned_at.format("%Y-%m-%d %H:%M"))
                                        }
                                        td class="py-3 px-4 text-secondary font-bold" {
                                            i class="fa-solid fa-user mr-2 text-muted" {}
                                            (scan.scanner_display_name())
                                        }
                                        td class="py-3 px-4 text-right" {
                                            @if scan.is_claimed() {
                                                span class="text-highlight orange font-black" {
                                                    (format_sats_si(scan.sats_claimed()))
                                                    " "
                                                    i class="fa-solid fa-bolt" {}
                                                }
                                            } @else if scan.is_claimable() {
                                                span class="text-secondary font-bold" {
                                                    i class="fa-solid fa-clock mr-1" {}
                                                    "PENDING"
                                                }
                                            } @else {
                                                span class="text-muted font-bold" {
                                                    i class="fa-solid fa-times mr-1" {}
                                                    "EXPIRED"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // NFC Card Management (for owner/admin)
            @if (is_owner || is_admin) && nfc_card.is_some() {
                @let card = nfc_card.unwrap();
                div class="card-brutal-inset mb-8" {
                    h2 class="heading-breaker" {
                        i class="fa-solid fa-microchip mr-2" {}
                        "NFC CARD MANAGEMENT"
                    }

                    div class="mt-8 space-y-6" {
                        // Wipe QR Code
                        div class="p-4" style="background: var(--bg-secondary); border: 2px solid var(--accent-muted);" {
                            div class="label-brutal text-xs mb-3" { "WIPE NFC CARD" }
                            p class="text-sm text-muted font-bold mb-4" {
                                "Scan this QR code with the Boltcard NFC Programmer app to wipe the NFC card."
                            }
                            div class="flex flex-col items-center gap-4" {
                                div class="p-2" style="background: white; border: 3px solid var(--accent-muted);" {
                                    canvas id="wipeQrCode" {}
                                }
                                button id="copyWipeJsonBtn" class="btn-brutal text-center" style="border-color: var(--accent-muted); color: var(--text-secondary);" {
                                    i class="fa-solid fa-copy mr-2" {}
                                    "COPY JSON"
                                }
                            }
                        }

                        // Reprogram button
                        @if let Some(ref deep_link) = boltcard_program_deep_link {
                            div class="p-4" style="background: var(--bg-secondary); border: 2px solid var(--accent-muted);" {
                                div class="label-brutal text-xs mb-3" { "REPROGRAM NFC" }
                                p class="text-sm text-muted font-bold mb-4" {
                                    "Tap the button below with the Boltcard NFC Programmer app to reprogram the NFC card with new keys."
                                }
                                a href=(deep_link) class="btn-brutal-fill text-center inline-block" style="background: var(--highlight); border-color: var(--highlight);" {
                                    i class="fa-solid fa-microchip mr-2" {}
                                    "REPROGRAM NFC"
                                }
                            }
                        }

                        // Card info
                        div class="p-4" style="background: var(--bg-secondary); border: 2px solid var(--accent-muted);" {
                            div class="label-brutal text-xs mb-3" { "CARD INFO" }
                            div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm mono" {
                                div {
                                    span class="text-muted font-bold" { "UID: " }
                                    span class="text-secondary font-bold" { (card.uid.as_deref().unwrap_or("Not set")) }
                                }
                                div {
                                    span class="text-muted font-bold" { "Counter: " }
                                    span class="text-secondary font-bold" { (card.counter) }
                                }
                            }
                        }
                    }
                }

                // Wipe QR Code script
                @if let Some(ref uid) = card.uid {
                    (PreEscaped(format!(r#"
                    <script src="https://cdn.jsdelivr.net/npm/qrious@4.0.2/dist/qrious.min.js"></script>
                    <script>
                        const wipeJson = JSON.stringify({{
                            "action": "wipe",
                            "k0": "{}",
                            "k1": "{}",
                            "k2": "{}",
                            "k3": "{}",
                            "k4": "{}",
                            "uid": "{}",
                            "version": 1
                        }});

                        new QRious({{
                            element: document.getElementById('wipeQrCode'),
                            value: wipeJson,
                            size: 200,
                            background: '#ffffff',
                            foreground: '#000000'
                        }});

                        document.getElementById('copyWipeJsonBtn').addEventListener('click', async function() {{
                            try {{
                                await navigator.clipboard.writeText(wipeJson);
                                const btn = this;
                                const originalHtml = btn.innerHTML;
                                btn.innerHTML = '<i class="fa-solid fa-check mr-2"></i>COPIED!';
                                setTimeout(() => btn.innerHTML = originalHtml, 2000);
                            }} catch (err) {{
                                alert('Failed to copy to clipboard');
                            }}
                        }});
                    </script>
                    "#, card.k0_auth_key, card.k1_decrypt_key, card.k2_cmac_key, card.k3, card.k4, uid)))
                }
            }

        }

        // Map script
        (PreEscaped(format!(r#"
        <script>
            // Initialize map with MapLibre
            const map = new maplibregl.Map({{
                container: 'map',
                style: 'https://tiles.openfreemap.org/styles/positron',
                center: [{}, {}],
                zoom: 15
            }});

            map.addControl(new maplibregl.NavigationControl());

            // Add marker
            new maplibregl.Marker()
                .setLngLat([{}, {}])
                .setPopup(new maplibregl.Popup({{ offset: 25 }})
                    .setHTML('<div style="color: #0f172a; padding: 8px;"><b>{}</b><br>{} sats available</div>'))
                .addTo(map)
                .togglePopup();
        </script>
        "#,
            location.longitude, location.latitude,
            location.longitude, location.latitude,
            location.name, format_sats_si(available_sats)
        )))

        // Photo upload script - auto-upload on file selection
        @if can_manage_photos {
            (PreEscaped(format!(r#"
            <script>
                document.getElementById('photoInput').addEventListener('change', async function() {{
                    if (!this.files || this.files.length === 0) return;

                    const file = this.files[0];
                    const maxSize = 20 * 1024 * 1024; // 20MB

                    if (file.size > maxSize) {{
                        alert('Photo is too large. Maximum size is 20MB.');
                        return;
                    }}

                    // Show loading state
                    document.getElementById('addPhotoBtn').classList.add('hidden');
                    document.getElementById('uploadingState').classList.remove('hidden');

                    const formData = new FormData();
                    formData.append('photo', file);

                    try {{
                        const response = await fetch('/api/locations/{}/photos', {{
                            method: 'POST',
                            body: formData
                        }});

                        if (response.ok) {{
                            location.reload();
                        }} else {{
                            let msg = 'Failed to upload photo';
                            if (response.status === 413) {{
                                msg = 'Photo is too large. Maximum size is 20MB.';
                            }} else if (response.status === 400) {{
                                msg = 'Invalid image file. Please try a different photo.';
                            }} else if (response.status === 403) {{
                                msg = 'You do not have permission to upload photos here.';
                            }}
                            alert(msg);
                            document.getElementById('addPhotoBtn').classList.remove('hidden');
                            document.getElementById('uploadingState').classList.add('hidden');
                        }}
                    }} catch (err) {{
                        alert('Upload failed. Check your connection and try again.');
                        document.getElementById('addPhotoBtn').classList.remove('hidden');
                        document.getElementById('uploadingState').classList.add('hidden');
                    }}
                }});
            </script>
            "#, location.id)))
        }

        // Photo viewer lightbox
        div id="photoViewer" class="hidden fixed inset-0 bg-black bg-opacity-60 z-[9999] flex items-center justify-center"
            onclick="closePhotoViewer()" {
            // Close button
            button class="absolute top-4 right-4 text-white hover:text-gray-300 transition-colors z-10"
                onclick="closePhotoViewer()"
                aria-label="Close" {
                i class="fa-solid fa-xmark text-4xl" {}
            }
            // Image
            img id="photoViewerImage" src="" alt="Full size photo" class="max-w-full max-h-full object-contain cursor-default p-4";
        }

        // Photo viewer script
        (PreEscaped(r#"
        <script>
            function openPhotoViewer(photoUrl) {
                const viewer = document.getElementById('photoViewer');
                const img = document.getElementById('photoViewerImage');
                img.src = photoUrl;
                viewer.classList.remove('hidden');
                document.body.style.overflow = 'hidden';
            }

            function closePhotoViewer() {
                const viewer = document.getElementById('photoViewer');
                viewer.classList.add('hidden');
                document.body.style.overflow = '';
            }

            // Close on Escape key
            document.addEventListener('keydown', function(e) {
                if (e.key === 'Escape') {
                    closePhotoViewer();
                }
            });
        </script>
        "#))

        // Location donation script
        (donation_invoice_script(&DonationInvoiceConfig {
            id_prefix: "location",
            location_id: Some(&location.id),
            amounts: &[
                ("1000", "1K"),
                ("5000", "5K"),
                ("10000", "10K"),
                ("custom", "Custom"),
            ],
            label: Some("Donate to this location"),
        }))

    }
}

fn delete_button(location_id: &str) -> Markup {
    html! {
        button
            onclick={
                "if(confirm('DELETE THIS LOCATION?')) { "
                "fetch('/api/locations/" (location_id) "', { method: 'DELETE' }) "
                ".then(r => r.ok ? window.location.href='/profile' : alert('FAILED')) "
                "}"
            }
            class="btn-brutal" style="border-color: var(--accent-muted); color: var(--text-muted);" {
            i class="fa-solid fa-trash" {}
        }
    }
}
