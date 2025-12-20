use crate::models::{Location, Photo, Refill, Scan};
use maud::{html, Markup, PreEscaped};

pub fn location_detail(location: &Location, photos: &[Photo], scans: &[Scan], refills: &[Refill], max_sats_per_location: i64, current_user_id: Option<&str>, error: Option<&str>) -> Markup {
    let withdrawable_sats = location.withdrawable_sats();
    let sats_percent = if max_sats_per_location > 0 {
        (withdrawable_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    let is_owner = current_user_id.map(|id| id == location.user_id).unwrap_or(false);
    let can_manage_photos = is_owner && !location.is_active();

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

            // Status banner for non-active locations
            @if !location.is_active() {
                div class="alert-brutal mb-6" {
                    div class="flex items-start gap-4" {
                        div class="flex-shrink-0" {
                            @if location.is_created() {
                                i class="fa-solid fa-clock text-3xl text-muted" {}
                            } @else {
                                i class="fa-solid fa-hourglass-half text-3xl text-muted" {}
                            }
                        }
                        div class="flex-1" {
                            h3 class="text-lg font-black mb-2 text-primary" {
                                @if location.is_created() {
                                    "LOCATION NOT YET PROGRAMMED"
                                } @else {
                                    "LOCATION WAITING FOR ACTIVATION"
                                }
                            }
                            @if location.is_created() {
                                p class="text-secondary mb-3 font-bold text-sm" {
                                    "THIS LOCATION HAS BEEN CREATED BUT THE NFC STICKER HAS NOT BEEN PROGRAMMED YET. "
                                    "IT WILL NOT APPEAR ON THE PUBLIC MAP UNTIL IT'S PROGRAMMED AND ACTIVATED."
                                }
                                @if photos.is_empty() {
                                    p class="text-highlight orange mb-3 font-bold text-sm" {
                                        i class="fa-solid fa-info-circle mr-1" {}
                                        "PLEASE ADD AT LEAST ONE PHOTO BEFORE PROGRAMMING THE NFC STICKER."
                                    }
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            @if photos.is_empty() {
                                                button disabled
                                                    class="inline-flex items-center btn-brutal opacity-50 cursor-not-allowed" {
                                                    i class="fa-solid fa-microchip mr-2" {}
                                                    "PROGRAM NFC STICKER"
                                                }
                                            } @else {
                                                a href={"/setup/" (token)}
                                                    class="inline-flex items-center btn-brutal-orange" {
                                                    i class="fa-solid fa-microchip mr-2" {}
                                                    "PROGRAM NFC STICKER"
                                                }
                                            }
                                        }
                                    }
                                    button
                                        onclick={
                                            "if(confirm('DELETE THIS LOCATION? THIS CANNOT BE UNDONE.')) { "
                                            "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                            ".then(r => r.ok ? window.location.href='/profile' : alert('FAILED TO DELETE LOCATION')) "
                                            "}"
                                        }
                                        class="inline-flex items-center btn-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                                        i class="fa-solid fa-trash mr-2" {}
                                        "DELETE"
                                    }
                                }
                            } @else {
                                p class="text-secondary mb-3 font-bold text-sm" {
                                    "THE NFC STICKER HAS BEEN PROGRAMMED. THIS LOCATION WILL BECOME ACTIVE AND APPEAR ON THE PUBLIC MAP "
                                    "AFTER THE FIRST SUCCESSFUL SCAN AND WITHDRAWAL."
                                }
                                @if photos.is_empty() {
                                    p class="text-highlight orange mb-3 font-bold text-sm" {
                                        i class="fa-solid fa-info-circle mr-1" {}
                                        "NOTE: YOU NEED AT LEAST ONE PHOTO. ADD PHOTOS BELOW BEFORE THE LOCATION GOES LIVE."
                                    }
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            @if photos.is_empty() {
                                                button disabled
                                                    class="inline-flex items-center btn-brutal opacity-50 cursor-not-allowed" {
                                                    i class="fa-solid fa-redo mr-2" {}
                                                    "RE-PROGRAM NFC STICKER"
                                                }
                                            } @else {
                                                a href={"/setup/" (token)}
                                                    class="inline-flex items-center btn-brutal-orange" {
                                                    i class="fa-solid fa-redo mr-2" {}
                                                    "RE-PROGRAM NFC STICKER"
                                                }
                                            }
                                        }
                                    }
                                    button
                                        onclick={
                                            "if(confirm('DELETE THIS LOCATION? THIS CANNOT BE UNDONE.')) { "
                                            "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                            ".then(r => r.ok ? window.location.href='/profile' : alert('FAILED TO DELETE LOCATION')) "
                                            "}"
                                        }
                                        class="inline-flex items-center btn-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                                        i class="fa-solid fa-trash mr-2" {}
                                        "DELETE"
                                    }
                                }
                                p class="text-muted text-xs mt-2 font-bold mono" {
                                    "IF THE NFC WRITE FAILED, YOU CAN TRY AGAIN WITH THE SAME KEYS"
                                }
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
                        form id="photoUploadForm" enctype="multipart/form-data" {
                            label for="photoInput" class="label-brutal mb-2 block" {
                                "ADD PHOTO"
                            }
                            div class="flex gap-3" {
                                input type="file" id="photoInput" name="photo" accept="image/*"
                                    class="flex-1 block text-sm text-muted font-bold file:mr-4 file:py-2 file:px-4 file:border-0 file:text-sm file:font-bold file:bg-highlight file:text-primary hover:file:brightness-110" style="border: 2px solid var(--accent-muted);";
                                button type="submit"
                                    class="btn-brutal-orange" {
                                    i class="fa-solid fa-upload mr-2" {}
                                    "UPLOAD"
                                }
                            }
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

        // Photo upload script
        @if can_manage_photos {
            (PreEscaped(format!(r#"
            <script>
                document.getElementById('photoUploadForm').addEventListener('submit', async function(e) {{
                    e.preventDefault();

                    const fileInput = document.getElementById('photoInput');
                    if (!fileInput.files || fileInput.files.length === 0) {{
                        alert('Please select a photo to upload');
                        return;
                    }}

                    const formData = new FormData();
                    formData.append('photo', fileInput.files[0]);

                    try {{
                        const response = await fetch('/api/locations/{}/photos', {{
                            method: 'POST',
                            body: formData
                        }});

                        if (response.ok) {{
                            location.reload();
                        }} else {{
                            alert('Failed to upload photo');
                        }}
                    }} catch (err) {{
                        alert('Error uploading photo: ' + err.message);
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
