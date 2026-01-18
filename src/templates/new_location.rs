use maud::{html, Markup, PreEscaped};

pub fn new_location() -> Markup {
    html! {
        h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" {
            i class="fa-solid fa-plus mr-2" {}
            "ADD NEW LOCATION"
        }

        form id="locationForm" action="/api/locations" method="post"
            class="card-brutal-inset space-y-6" {

            // Name field
            div {
                label for="name" class="label-brutal" {
                    "LOCATION NAME"
                }
                input type="text" id="name" name="name" required
                    class="input-brutal-box w-full"
                    placeholder="TREASURE ROCK IN CENTRAL PARK";
            }

            // Description
            div {
                label for="description" class="label-brutal" {
                    "DESCRIPTION (OPTIONAL)"
                }
                textarea id="description" name="description" rows="3"
                    class="input-brutal-box w-full"
                    placeholder="BEHIND THE LARGE OAK TREE NEAR THE FOUNTAIN..." {}
            }

            // Coordinates
            div class="grid md:grid-cols-2 gap-4" {
                div {
                    label for="latitude" class="label-brutal" {
                        "LATITUDE"
                    }
                    input type="number" id="latitude" name="latitude" required step="any" value="37.7749"
                        class="input-brutal-box w-full"
                        placeholder="37.7749";
                }
                div {
                    label for="longitude" class="label-brutal" {
                        "LONGITUDE"
                    }
                    input type="number" id="longitude" name="longitude" required step="any" value="-122.4194"
                        class="input-brutal-box w-full"
                        placeholder="-122.4194";
                }
            }

            // GPS button
            div {
                button type="button" id="useGps"
                    class="w-full btn-brutal" {
                    i class="fa-solid fa-location-crosshairs mr-2" {}
                    "USE MY CURRENT LOCATION"
                }
            }

            // Map preview
            div {
                label class="label-brutal mb-2 block" {
                    "LOCATION PREVIEW"
                }
                div id="previewMap" class="w-full h-64" style="border: 3px solid var(--accent-border);" {}
            }

            // Submit button
            div {
                button type="submit"
                    class="w-full btn-brutal-fill" {
                    "CREATE LOCATION"
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
                        // Redirect to the location details page to show next steps
                        window.location.href = '/locations/' + result.location_id;
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
