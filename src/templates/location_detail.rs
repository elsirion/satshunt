use crate::models::{Location, Photo, Scan};
use maud::{html, Markup, PreEscaped};

pub fn location_detail(location: &Location, photos: &[Photo], scans: &[Scan], max_sats_per_location: i64, current_user_id: Option<&str>, error: Option<&str>) -> Markup {
    let sats_percent = if max_sats_per_location > 0 {
        (location.current_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    let is_owner = current_user_id.map(|id| id == location.user_id).unwrap_or(false);
    let can_manage_photos = is_owner && !location.is_active();

    html! {
        div class="max-w-4xl mx-auto" {
            // Back button
            a href="/map" class="inline-flex items-center text-highlight hover:bg-accent-hover mb-6" {
                "← Back to map"
            }

            // Error message
            @if let Some(error_msg) = error {
                div class="bg-red-900 border border-red-700 text-red-200 px-4 py-3 rounded-lg mb-6" {
                    p class="font-semibold" { "⚠ " (error_msg) }
                }
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
                                @if photos.is_empty() {
                                    p class="text-yellow-300 mb-3 font-semibold" {
                                        i class="fa-solid fa-info-circle mr-1" {}
                                        "Please add at least one photo before programming the NFC sticker."
                                    }
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            @if photos.is_empty() {
                                                button disabled
                                                    class="inline-flex items-center px-4 py-2 bg-gray-600 text-gray-400 rounded-lg font-semibold cursor-not-allowed" {
                                                    i class="fa-solid fa-microchip mr-2" {}
                                                    "Program NFC Sticker"
                                                }
                                            } @else {
                                                a href={"/setup/" (token)}
                                                    class="inline-flex items-center px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg font-semibold transition-colors" {
                                                    i class="fa-solid fa-microchip mr-2" {}
                                                    "Program NFC Sticker"
                                                }
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
                                @if photos.is_empty() {
                                    p class="text-blue-300 mb-3 font-semibold" {
                                        i class="fa-solid fa-info-circle mr-1" {}
                                        "Note: You need at least one photo. Add photos below before the location goes live."
                                    }
                                }
                                div class="flex gap-3" {
                                    @if let Some(token) = &location.write_token {
                                        @if !location.write_token_used {
                                            @if photos.is_empty() {
                                                button disabled
                                                    class="inline-flex items-center px-4 py-2 bg-gray-600 text-gray-400 rounded-lg font-semibold cursor-not-allowed" {
                                                    i class="fa-solid fa-redo mr-2" {}
                                                    "Re-program NFC Sticker"
                                                }
                                            } @else {
                                                a href={"/setup/" (token)}
                                                    class="inline-flex items-center px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-semibold transition-colors" {
                                                    i class="fa-solid fa-redo mr-2" {}
                                                    "Re-program NFC Sticker"
                                                }
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
            div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
                h2 class="text-2xl font-bold mb-4 text-highlight" {
                    i class="fa-solid fa-camera mr-2" {}
                    "Photos"
                }

                @if !photos.is_empty() {
                    div id="photosGrid" class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6" {
                        @for photo in photos {
                            div class="relative group" {
                                img src={"/uploads/" (photo.file_path)}
                                    alt="Location photo"
                                    class="w-full h-48 object-cover rounded-lg border border-accent-muted cursor-pointer hover:opacity-90 transition-opacity"
                                    onclick={"openPhotoViewer('/uploads/" (photo.file_path) "')"};
                                @if can_manage_photos {
                                    button
                                        onclick={
                                            "event.stopPropagation(); \
                                            if(confirm('Are you sure you want to delete this photo?')) { \
                                            fetch('/api/photos/" (photo.id) "', { method: 'DELETE' }) \
                                            .then(r => r.ok ? location.reload() : alert('Failed to delete photo')) \
                                            }"
                                        }
                                        class="absolute top-2 right-2 bg-red-600 hover:bg-red-700 text-white p-2 rounded-lg opacity-0 group-hover:opacity-100 transition-opacity" {
                                        i class="fa-solid fa-trash" {}
                                    }
                                }
                            }
                        }
                    }
                } @else {
                    p class="text-muted mb-6" { "No photos yet." }
                }

                @if can_manage_photos {
                    div class="border-t border-accent-muted pt-6" {
                        form id="photoUploadForm" enctype="multipart/form-data" {
                            label for="photoInput" class="block mb-2 text-sm font-medium text-primary" {
                                "Add Photo"
                            }
                            div class="flex gap-3" {
                                input type="file" id="photoInput" name="photo" accept="image/*"
                                    class="flex-1 block text-sm text-muted file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-highlight file:text-inverse hover:file:brightness-110";
                                button type="submit"
                                    class="px-6 py-2 btn-primary" {
                                    i class="fa-solid fa-upload mr-2" {}
                                    "Upload"
                                }
                            }
                        }
                    }
                }
            }

            // Payout History
            @if !scans.is_empty() {
                div class="bg-secondary rounded-lg p-8 mb-8 border border-accent-muted" {
                    h2 class="text-2xl font-bold mb-4 text-highlight" {
                        i class="fa-solid fa-history mr-2" {}
                        "Payout History"
                    }

                    div class="overflow-x-auto" {
                        table class="w-full" {
                            thead {
                                tr class="border-b border-accent-muted" {
                                    th class="text-left py-3 px-4 text-secondary font-semibold" { "Date" }
                                    th class="text-right py-3 px-4 text-secondary font-semibold" { "Amount" }
                                }
                            }
                            tbody {
                                @for scan in scans {
                                    tr class="border-b border-accent-muted hover:bg-tertiary transition-colors" {
                                        td class="py-3 px-4 text-secondary" {
                                            (scan.scanned_at.format("%Y-%m-%d %H:%M:%S UTC"))
                                        }
                                        td class="py-3 px-4 text-right font-mono" {
                                            span class="text-highlight font-semibold" {
                                                (scan.sats_withdrawn)
                                            }
                                            " "
                                            i class="fa-solid fa-bolt text-highlight" {}
                                        }
                                    }
                                }
                            }
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
            location.name, location.current_sats
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
