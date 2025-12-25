use crate::models::{Location, Photo, Refill, Scan};
use maud::{html, Markup, PreEscaped};

#[allow(clippy::too_many_arguments)] // All parameters are needed for the template
pub fn location_detail(
    location: &Location,
    photos: &[Photo],
    scans: &[Scan],
    refills: &[Refill],
    max_sats_per_location: i64,
    current_user_id: Option<&str>,
    error: Option<&str>,
    base_url: &str,
) -> Markup {
    let withdrawable_sats = location.withdrawable_sats();
    let sats_percent = if max_sats_per_location > 0 {
        (withdrawable_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    let is_owner = current_user_id
        .map(|id| id == location.user_id)
        .unwrap_or(false);
    let can_manage_photos = is_owner && !location.is_active();

    // Generate Boltcard deep link for NFC programming
    let boltcard_deep_link = location.write_token.as_ref().map(|token| {
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
                    p class="font-bold" { "⚠ " (error_msg) }
                }
            }

            // Next step banner for non-active locations (owner only)
            @if is_owner && !location.is_active() {
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
                            @if let Some(ref deep_link) = boltcard_deep_link {
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

                    // Status badge
                    @if location.is_active() {
                        span class="badge-brutal filled" { "ACTIVE" }
                    } @else if location.is_programmed() {
                        span class="badge-brutal grey" { "PROGRAMMED" }
                    } @else {
                        span class="badge-brutal white" { "CREATED" }
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
                            (withdrawable_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="card-brutal-inset p-4" {
                        div class="label-brutal text-xs mb-2" { "MAX CAPACITY" }
                        div class="text-2xl font-black text-secondary" {
                            (max_sats_per_location) " "
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

            // Payout History
            @if !scans.is_empty() {
                div class="card-brutal-inset mb-8" {
                    h2 class="heading-breaker" {
                        i class="fa-solid fa-history mr-2" {}
                        "PAYOUT HISTORY"
                    }

                    div class="overflow-x-auto mt-8" {
                        table class="w-full" {
                            thead {
                                tr style="border-bottom: 2px solid var(--accent-muted);" {
                                    th class="text-left py-3 px-4 text-secondary font-black" { "DATE" }
                                    th class="text-right py-3 px-4 text-secondary font-black" { "AMOUNT" }
                                }
                            }
                            tbody {
                                @for scan in scans {
                                    tr style="border-bottom: 2px solid var(--accent-muted);" class="hover:bg-tertiary transition-colors" {
                                        td class="py-3 px-4 text-secondary font-bold mono text-sm" {
                                            (scan.scanned_at.format("%Y-%m-%d %H:%M:%S UTC"))
                                        }
                                        td class="py-3 px-4 text-right mono" {
                                            span class="text-highlight orange font-black" {
                                                (scan.sats_withdrawn())
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

            // Refill History
            @if !refills.is_empty() {
                div class="card-brutal-inset mb-8" {
                    details {
                        summary class="text-2xl font-black text-primary cursor-pointer select-none hover:text-highlight transition-colors" {
                            i class="fa-solid fa-fill-drip mr-2" {}
                            "REFILL HISTORY "
                            span class="text-base text-muted mono" { "[" (refills.len()) " REFILLS]" }
                        }

                        div class="overflow-x-auto mt-4" {
                            table id="refillsTable" class="w-full" {
                                thead {
                                    tr style="border-bottom: 2px solid var(--accent-muted);" {
                                        th class="text-left py-3 px-4 text-secondary font-black text-xs" { "DATE" }
                                        th class="text-right py-3 px-4 text-secondary font-black text-xs" { "AMOUNT ADDED" }
                                        th class="text-right py-3 px-4 text-secondary font-black text-xs" { "BALANCE BEFORE" }
                                        th class="text-right py-3 px-4 text-secondary font-black text-xs" { "BALANCE AFTER" }
                                        th class="text-right py-3 px-4 text-secondary font-black text-xs" { "BASE RATE" }
                                        th class="text-right py-3 px-4 text-secondary font-black text-xs" { "SLOWDOWN" }
                                    }
                                }
                                tbody id="refillsTableBody" {
                                    @for (index, refill) in refills.iter().enumerate() {
                                        tr style="border-bottom: 2px solid var(--accent-muted);" class="hover:bg-tertiary transition-colors refill-row" data-index=(index) {
                                            td class="py-3 px-4 text-secondary font-bold mono text-xs" {
                                                (refill.refilled_at.format("%Y-%m-%d %H:%M:%S UTC"))
                                            }
                                            td class="py-3 px-4 text-right mono text-sm" {
                                                span class="text-highlight orange font-black" {
                                                    "+" (format!("{:.3}", refill.sats_added()))
                                                }
                                                " "
                                                i class="fa-solid fa-bolt text-highlight orange" {}
                                            }
                                            td class="py-3 px-4 text-right mono text-muted font-bold text-sm" {
                                                (format!("{:.3}", refill.balance_before_sats()))
                                            }
                                            td class="py-3 px-4 text-right mono text-primary font-bold text-sm" {
                                                (format!("{:.3}", refill.balance_after_sats()))
                                            }
                                            td class="py-3 px-4 text-right mono text-secondary font-bold text-xs" {
                                                (format!("{:.3}", refill.base_rate_sats_per_min())) " SATS/MIN"
                                            }
                                            td class="py-3 px-4 text-right mono text-secondary font-bold text-xs" {
                                                (format!("{:.3}x", refill.slowdown_factor))
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Pagination controls
                        @if refills.len() > 20 {
                            div id="refillsPagination" class="flex items-center justify-center gap-2 mt-6" {
                                button id="refillsPrevBtn"
                                    class="btn-brutal disabled:opacity-50 disabled:cursor-not-allowed"
                                    onclick="changeRefillsPage(-1)" {
                                    i class="fa-solid fa-chevron-left mr-2" {}
                                    "PREVIOUS"
                                }
                                div id="refillsPageInfo" class="px-4 py-2 text-secondary font-bold mono" {}
                                button id="refillsNextBtn"
                                    class="btn-brutal disabled:opacity-50 disabled:cursor-not-allowed"
                                    onclick="changeRefillsPage(1)" {
                                    "NEXT"
                                    i class="fa-solid fa-chevron-right ml-2" {}
                                }
                            }
                        }
                    }
                }

                // Refills pagination script
                @if refills.len() > 20 {
                    (PreEscaped(format!(r#"
                    <script>
                        let refillsCurrentPage = 1;
                        const refillsPerPage = 20;
                        const refillsTotalItems = {};

                        function updateRefillsTable() {{
                            const rows = document.querySelectorAll('.refill-row');
                            const startIndex = (refillsCurrentPage - 1) * refillsPerPage;
                            const endIndex = startIndex + refillsPerPage;

                            rows.forEach((row, index) => {{
                                if (index >= startIndex && index < endIndex) {{
                                    row.style.display = '';
                                }} else {{
                                    row.style.display = 'none';
                                }}
                            }});

                            // Update pagination controls
                            const totalPages = Math.ceil(refillsTotalItems / refillsPerPage);
                            document.getElementById('refillsPageInfo').textContent = `Page ${{refillsCurrentPage}} of ${{totalPages}}`;
                            document.getElementById('refillsPrevBtn').disabled = refillsCurrentPage === 1;
                            document.getElementById('refillsNextBtn').disabled = refillsCurrentPage === totalPages;
                        }}

                        function changeRefillsPage(delta) {{
                            const totalPages = Math.ceil(refillsTotalItems / refillsPerPage);
                            const newPage = refillsCurrentPage + delta;

                            if (newPage >= 1 && newPage <= totalPages) {{
                                refillsCurrentPage = newPage;
                                updateRefillsTable();
                            }}
                        }}

                        // Initialize on page load
                        updateRefillsTable();
                    </script>
                    "#, refills.len())))
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
            location.name, withdrawable_sats
        )))

        // Photo upload script - auto-upload on file selection
        @if can_manage_photos {
            (PreEscaped(format!(r#"
            <script>
                document.getElementById('photoInput').addEventListener('change', async function() {{
                    if (!this.files || this.files.length === 0) return;

                    // Show loading state
                    document.getElementById('addPhotoBtn').classList.add('hidden');
                    document.getElementById('uploadingState').classList.remove('hidden');

                    const formData = new FormData();
                    formData.append('photo', this.files[0]);

                    try {{
                        const response = await fetch('/api/locations/{}/photos', {{
                            method: 'POST',
                            body: formData
                        }});

                        if (response.ok) {{
                            location.reload();
                        }} else {{
                            alert('Failed to upload photo');
                            // Reset state
                            document.getElementById('addPhotoBtn').classList.remove('hidden');
                            document.getElementById('uploadingState').classList.add('hidden');
                        }}
                    }} catch (err) {{
                        alert('Error uploading photo: ' + err.message);
                        // Reset state
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
