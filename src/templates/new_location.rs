use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn new_location() -> Markup {
    location_form(None)
}

pub fn edit_location(location: &Location) -> Markup {
    location_form(Some(location))
}

fn location_form(existing: Option<&Location>) -> Markup {
    let is_edit = existing.is_some();
    let heading_icon = if is_edit { "fa-pen" } else { "fa-plus" };
    let heading = if is_edit {
        "EDIT LOCATION"
    } else {
        "ADD NEW LOCATION"
    };
    let submit_label = if is_edit {
        "SAVE CHANGES"
    } else {
        "CREATE LOCATION"
    };

    let name_value = existing.map(|l| l.name.clone()).unwrap_or_default();
    let description_value = existing
        .and_then(|l| l.description.clone())
        .unwrap_or_default();
    let lat_value = existing.map(|l| l.latitude).unwrap_or(37.7749);
    let lng_value = existing.map(|l| l.longitude).unwrap_or(-122.4194);
    let initial_zoom = if is_edit { 15 } else { 13 };

    let endpoint = if let Some(loc) = existing {
        format!("/api/locations/{}", loc.id)
    } else {
        "/api/locations".to_string()
    };
    let method = if is_edit { "PUT" } else { "POST" };
    let redirect_js = if let Some(loc) = existing {
        format!("window.location.href = '/locations/{}';", loc.id)
    } else {
        "window.location.href = '/locations/' + result.location_id;".to_string()
    };

    html! {
        h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" {
            i class=(format!("fa-solid {} mr-2", heading_icon)) {}
            (heading)
        }

        @if let Some(loc) = existing {
            a href=(format!("/locations/{}", loc.id))
                class="inline-flex items-center text-highlight orange font-bold mb-6 hover:text-primary transition" {
                "← BACK TO LOCATION"
            }
        }

        form id="locationForm"
            class="card-brutal-inset space-y-6" {

            // Name field
            div {
                label for="name" class="label-brutal" {
                    "LOCATION NAME"
                }
                input type="text" id="name" name="name" required
                    class="input-brutal-box w-full"
                    value=(name_value)
                    placeholder="TREASURE ROCK IN CENTRAL PARK";
            }

            // Description
            div {
                label for="description" class="label-brutal" {
                    "DESCRIPTION (OPTIONAL)"
                }
                textarea id="description" name="description" rows="3"
                    class="input-brutal-box w-full"
                    placeholder="BEHIND THE LARGE OAK TREE NEAR THE FOUNTAIN..." {
                    (description_value)
                }
            }

            // Coordinates
            div class="grid md:grid-cols-2 gap-4" {
                div {
                    label for="latitude" class="label-brutal" {
                        "LATITUDE"
                    }
                    input type="number" id="latitude" name="latitude" required step="any"
                        value=(format!("{}", lat_value))
                        class="input-brutal-box w-full"
                        placeholder="37.7749";
                }
                div {
                    label for="longitude" class="label-brutal" {
                        "LONGITUDE"
                    }
                    input type="number" id="longitude" name="longitude" required step="any"
                        value=(format!("{}", lng_value))
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
                    (submit_label)
                }
            }
        }

        // JavaScript for map and GPS
        (PreEscaped(format!(r#"
        <script>
            let map, marker;
            const initialLat = {lat};
            const initialLng = {lng};
            const initialZoom = {zoom};

            // Initialize preview map
            function initMap() {{
                map = new maplibregl.Map({{
                    container: 'previewMap',
                    style: 'https://tiles.openfreemap.org/styles/positron',
                    center: [initialLng, initialLat],
                    zoom: initialZoom
                }});

                map.addControl(new maplibregl.NavigationControl());

                marker = new maplibregl.Marker({{draggable: true}})
                    .setLngLat([initialLng, initialLat])
                    .addTo(map);

                marker.on('dragend', function() {{
                    const lngLat = marker.getLngLat();
                    document.getElementById('latitude').value = lngLat.lat.toFixed(6);
                    document.getElementById('longitude').value = lngLat.lng.toFixed(6);
                }});
            }}

            // Update map when coordinates change
            function updateMapPosition() {{
                const lat = parseFloat(document.getElementById('latitude').value);
                const lng = parseFloat(document.getElementById('longitude').value);

                if (!isNaN(lat) && !isNaN(lng)) {{
                    marker.setLngLat([lng, lat]);
                    map.jumpTo({{center: [lng, lat], zoom: 15}});
                }}
            }}

            document.getElementById('latitude').addEventListener('change', updateMapPosition);
            document.getElementById('longitude').addEventListener('change', updateMapPosition);

            // GPS button
            document.getElementById('useGps').addEventListener('click', function() {{
                if ('geolocation' in navigator) {{
                    navigator.geolocation.getCurrentPosition(function(position) {{
                        const lat = position.coords.latitude;
                        const lng = position.coords.longitude;

                        document.getElementById('latitude').value = lat.toFixed(6);
                        document.getElementById('longitude').value = lng.toFixed(6);

                        marker.setLngLat([lng, lat]);
                        map.jumpTo({{center: [lng, lat], zoom: 15}});
                    }}, function(error) {{
                        alert('Unable to get location: ' + error.message);
                    }});
                }} else {{
                    alert('Geolocation is not supported by your browser');
                }}
            }});

            // Form submission
            document.getElementById('locationForm').addEventListener('submit', async function(e) {{
                e.preventDefault();

                const formData = {{
                    name: document.getElementById('name').value,
                    description: document.getElementById('description').value,
                    latitude: parseFloat(document.getElementById('latitude').value),
                    longitude: parseFloat(document.getElementById('longitude').value)
                }};

                try {{
                    const response = await fetch('{endpoint}', {{
                        method: '{method}',
                        headers: {{
                            'Content-Type': 'application/json'
                        }},
                        body: JSON.stringify(formData)
                    }});

                    if (response.ok) {{
                        const result = response.status === 204 ? null : await response.json();
                        {redirect}
                    }} else {{
                        const error = await response.text();
                        alert('Error saving location: ' + error);
                    }}
                }} catch (err) {{
                    alert('Error: ' + err.message);
                }}
            }});

            // Initialize map when page loads
            window.addEventListener('load', initMap);
        </script>
        "#,
            lat = lat_value,
            lng = lng_value,
            zoom = initial_zoom,
            endpoint = endpoint,
            method = method,
            redirect = redirect_js,
        )))
    }
}
