use maud::{html, Markup, PreEscaped};

pub fn new_location() -> Markup {
    html! {
        div class="max-w-2xl mx-auto" {
            h1 class="text-4xl font-bold mb-8 text-highlight" {
                i class="fa-solid fa-plus mr-2" {}
                "Add New Location"
            }

            form id="locationForm" action="/api/locations" method="post" enctype="multipart/form-data"
                class="bg-secondary rounded-lg p-8 border border-accent-muted space-y-6" {

                // Name field
                div {
                    label for="name" class="block mb-2 text-sm font-medium text-primary" {
                        "Location Name"
                    }
                    input type="text" id="name" name="name" required
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Treasure Rock in Central Park";
                }

                // Description
                div {
                    label for="description" class="block mb-2 text-sm font-medium text-primary" {
                        "Description (optional)"
                    }
                    textarea id="description" name="description" rows="3"
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Behind the large oak tree near the fountain..." {}
                }

                // Coordinates
                div class="grid md:grid-cols-2 gap-4" {
                    div {
                        label for="latitude" class="block mb-2 text-sm font-medium text-primary" {
                            "Latitude"
                        }
                        input type="number" id="latitude" name="latitude" required step="any" value="37.7749"
                            class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                            placeholder="37.7749";
                    }
                    div {
                        label for="longitude" class="block mb-2 text-sm font-medium text-primary" {
                            "Longitude"
                        }
                        input type="number" id="longitude" name="longitude" required step="any" value="-122.4194"
                            class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                            placeholder="-122.4194";
                    }
                }

                // GPS button
                div {
                    button type="button" id="useGps"
                        class="w-full px-4 py-2 btn-secondary" {
                        i class="fa-solid fa-location-crosshairs mr-2" {}
                        "Use My Current Location"
                    }
                }

                // Map preview
                div {
                    label class="block mb-2 text-sm font-medium text-primary" {
                        "Location Preview"
                    }
                    div id="previewMap" class="w-full h-64 rounded-lg border border-accent-muted" {}
                }

                // Photo upload
                div {
                    label for="photos" class="block mb-2 text-sm font-medium text-primary" {
                        "Photos"
                    }
                    input type="file" id="photos" name="photos" accept="image/*" multiple
                        class="block w-full text-sm text-muted file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-highlight file:text-inverse hover:file:brightness-110";
                    p class="mt-1 text-sm text-muted" {
                        "Upload photos to help others find the location"
                    }
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full btn-primary" {
                        "Create Location"
                    }
                }
            }
        }

        // JavaScript for map and GPS
        (PreEscaped(r#"
        <script>
            let map, marker;

            // Initialize preview map
            function initMap() {
                map = L.map('previewMap').setView([37.7749, -122.4194], 13);

                L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
                    attribution: '© OpenStreetMap contributors',
                    className: 'map-tiles'
                }).addTo(map);

                // Add dark theme
                const style = document.createElement('style');
                style.textContent = `
                    .map-tiles {
                        filter: invert(100%) hue-rotate(180deg) brightness(95%) contrast(90%);
                    }
                `;
                document.head.appendChild(style);

                marker = L.marker([37.7749, -122.4194], {draggable: true}).addTo(map);

                marker.on('dragend', function(e) {
                    const pos = marker.getLatLng();
                    document.getElementById('latitude').value = pos.lat.toFixed(6);
                    document.getElementById('longitude').value = pos.lng.toFixed(6);
                });
            }

            // Update map when coordinates change
            function updateMapPosition() {
                const lat = parseFloat(document.getElementById('latitude').value);
                const lng = parseFloat(document.getElementById('longitude').value);

                if (!isNaN(lat) && !isNaN(lng)) {
                    marker.setLatLng([lat, lng]);
                    map.setView([lat, lng], 15);
                }
            }

            document.getElementById('latitude').addEventListener('change', updateMapPosition);
            document.getElementById('longitude').addEventListener('change', updateMapPosition);

            // GPS button
            document.getElementById('useGps').addEventListener('click', function() {
                if ('geolocation' in navigator) {
                    navigator.geolocation.getCurrentPosition(function(position) {
                        const lat = position.coords.latitude;
                        const lng = position.coords.longitude;

                        document.getElementById('latitude').value = lat.toFixed(6);
                        document.getElementById('longitude').value = lng.toFixed(6);

                        marker.setLatLng([lat, lng]);
                        map.setView([lat, lng], 15);
                    }, function(error) {
                        alert('Unable to get location: ' + error.message);
                    });
                } else {
                    alert('Geolocation is not supported by your browser');
                }
            });

            // Form submission
            document.getElementById('locationForm').addEventListener('submit', async function(e) {
                e.preventDefault();

                const formData = new FormData(this);

                try {
                    const response = await fetch('/api/locations', {
                        method: 'POST',
                        body: formData
                    });

                    if (response.ok) {
                        const result = await response.json();
                        // Redirect to the NFC setup page using the write_token
                        window.location.href = '/setup/' + result.write_token;
                    } else {
                        const error = await response.text();
                        alert('Error creating location: ' + error);
                    }
                } catch (err) {
                    alert('Error: ' + err.message);
                }
            });

            // Initialize map when page loads
            window.addEventListener('load', initMap);
        </script>
        "#))
    }
}
