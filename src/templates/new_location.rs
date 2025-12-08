use maud::{html, Markup, PreEscaped};

pub fn new_location() -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-highlight" {
            i class="fa-solid fa-plus mr-2" {}
            "Add New Location"
        }

        form id="locationForm" action="/api/locations" method="post"
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

            // Submit button
            div {
                button type="submit"
                    class="w-full btn-primary" {
                    "Create Location"
                }
            }
        }

        // JavaScript for map and GPS
        (PreEscaped(r#"
        <script>
            let map, marker;

            // Initialize preview map
            function initMap() {
                map = new maplibregl.Map({
                    container: 'previewMap',
                    style: 'https://tiles.openfreemap.org/styles/positron',
                    center: [-122.4194, 37.7749],
                    zoom: 13
                });

                map.addControl(new maplibregl.NavigationControl());

                marker = new maplibregl.Marker({draggable: true})
                    .setLngLat([-122.4194, 37.7749])
                    .addTo(map);

                marker.on('dragend', function() {
                    const lngLat = marker.getLngLat();
                    document.getElementById('latitude').value = lngLat.lat.toFixed(6);
                    document.getElementById('longitude').value = lngLat.lng.toFixed(6);
                });
            }

            // Update map when coordinates change
            function updateMapPosition() {
                const lat = parseFloat(document.getElementById('latitude').value);
                const lng = parseFloat(document.getElementById('longitude').value);

                if (!isNaN(lat) && !isNaN(lng)) {
                    marker.setLngLat([lng, lat]);
                    map.jumpTo({center: [lng, lat], zoom: 15});
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

                        marker.setLngLat([lng, lat]);
                        map.jumpTo({center: [lng, lat], zoom: 15});
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

                const formData = {
                    name: document.getElementById('name').value,
                    description: document.getElementById('description').value,
                    latitude: parseFloat(document.getElementById('latitude').value),
                    longitude: parseFloat(document.getElementById('longitude').value)
                };

                try {
                    const response = await fetch('/api/locations', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json'
                        },
                        body: JSON.stringify(formData)
                    });

                    if (response.ok) {
                        const result = await response.json();
                        // Redirect to the profile page where user can program NFC
                        window.location.href = '/profile';
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
