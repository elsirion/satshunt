use maud::{html, Markup, PreEscaped};

pub fn new_location() -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-yellow-400" { "➕ Add New Location" }

        div class="max-w-2xl mx-auto" {
            form id="locationForm" action="/api/locations" method="post" enctype="multipart/form-data"
                class="bg-slate-800 rounded-lg p-8 border border-slate-700 space-y-6" {

                // Name field
                div {
                    label for="name" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Location Name"
                    }
                    input type="text" id="name" name="name" required
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Treasure Rock in Central Park";
                }

                // Description
                div {
                    label for="description" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Description (optional)"
                    }
                    textarea id="description" name="description" rows="3"
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Behind the large oak tree near the fountain..." {}
                }

                // Max sats
                div {
                    label for="max_sats" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Maximum Sats"
                    }
                    input type="number" id="max_sats" name="max_sats" required min="100" value="1000"
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5";
                    p class="mt-1 text-sm text-slate-400" {
                        "The maximum number of sats this location can hold"
                    }
                }

                // Coordinates
                div class="grid md:grid-cols-2 gap-4" {
                    div {
                        label for="latitude" class="block mb-2 text-sm font-medium text-slate-200" {
                            "Latitude"
                        }
                        input type="number" id="latitude" name="latitude" required step="any" value="37.7749"
                            class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                            placeholder="37.7749";
                    }
                    div {
                        label for="longitude" class="block mb-2 text-sm font-medium text-slate-200" {
                            "Longitude"
                        }
                        input type="number" id="longitude" name="longitude" required step="any" value="-122.4194"
                            class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                            placeholder="-122.4194";
                    }
                }

                // GPS button
                div {
                    button type="button" id="useGps"
                        class="w-full px-4 py-2 bg-slate-700 hover:bg-slate-600 text-slate-200 rounded-lg border border-slate-600 transition" {
                        "📍 Use My Current Location"
                    }
                }

                // Map preview
                div {
                    label class="block mb-2 text-sm font-medium text-slate-200" {
                        "Location Preview"
                    }
                    div id="previewMap" class="w-full h-64 rounded-lg border border-slate-700" {}
                }

                // Photo upload
                div {
                    label for="photos" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Photos"
                    }
                    input type="file" id="photos" name="photos" accept="image/*" multiple
                        class="block w-full text-sm text-slate-400 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-semibold file:bg-yellow-500 file:text-slate-900 hover:file:bg-yellow-600";
                    p class="mt-1 text-sm text-slate-400" {
                        "Upload photos to help others find the location"
                    }
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full px-6 py-3 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg transition" {
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
