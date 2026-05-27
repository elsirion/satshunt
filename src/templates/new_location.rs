use crate::balance::BalanceConfig;
use crate::models::Location;
use maud::{html, Markup, PreEscaped};

const SECS_PER_MINUTE: i64 = 60;
const SECS_PER_HOUR: i64 = 60 * 60;
const SECS_PER_DAY: i64 = 24 * 60 * 60;

pub fn new_location() -> Markup {
    location_form(None, None)
}

pub fn edit_location(location: &Location, defaults: &BalanceConfig) -> Markup {
    location_form(Some(location), Some(defaults))
}

/// Pick the largest whole unit that exactly represents the given seconds.
/// Falls back to minutes if no whole unit fits cleanly.
fn split_secs(secs: i64) -> (i64, &'static str) {
    if secs > 0 && secs % SECS_PER_DAY == 0 {
        (secs / SECS_PER_DAY, "days")
    } else if secs > 0 && secs % SECS_PER_HOUR == 0 {
        (secs / SECS_PER_HOUR, "hours")
    } else {
        ((secs / SECS_PER_MINUTE).max(1), "minutes")
    }
}

fn location_form(existing: Option<&Location>, defaults: Option<&BalanceConfig>) -> Markup {
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

        // Payout schedule overrides (edit only)
        @if let (Some(location), Some(defaults)) = (existing, defaults) {
            (payout_overrides_form(location, defaults))
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

/// Render the payout-overrides editor. Only shown on the edit page so the setup
/// flow stays simple — overrides are an advanced, after-the-fact tweak.
///
/// A single toggle controls both fields at once: either this location overrides
/// the payout schedule entirely, or it inherits the global defaults entirely.
fn payout_overrides_form(location: &Location, defaults: &BalanceConfig) -> Markup {
    let is_overridden =
        location.time_to_full_secs.is_some() || location.max_fill_percentage.is_some();

    let (time_value, time_unit) = location
        .time_to_full_secs
        .map(split_secs)
        .unwrap_or_else(|| split_secs(defaults.time_to_full_secs));

    // Percentage stored 0.0–1.0; UI uses 0–100.
    let pct_value = location
        .max_fill_percentage
        .unwrap_or(defaults.max_fill_percentage)
        * 100.0;

    let (default_time_value, default_time_unit) = split_secs(defaults.time_to_full_secs);
    let default_pct = defaults.max_fill_percentage * 100.0;
    let default_summary = format!(
        "Defaults to {} {} to full and {}% of the donation pool.",
        default_time_value, default_time_unit, default_pct
    );

    html! {
        div class="card-brutal-inset mt-8" {
            h2 class="heading-breaker" {
                i class="fa-solid fa-sliders mr-2" {}
                "PAYOUT SCHEDULE"
            }

            p class="text-secondary text-sm font-bold mt-6 mb-4" {
                "Override how fast this location fills back up and what share of its donation pool a single claim can take. "
                (default_summary)
            }

            form id="payoutForm" class="space-y-6 mt-4" {

                // Single override toggle for both fields
                div {
                    label class="flex items-center gap-2 font-bold text-primary mb-2 cursor-pointer" {
                        input type="checkbox" id="payoutOverrideEnabled"
                            checked[is_overridden];
                        span { "OVERRIDE PAYOUT SCHEDULE FOR THIS LOCATION" }
                    }
                }

                // Time to full
                div {
                    label class="label-brutal mb-2 block" for="timeValue" { "TIME TO FULL" }
                    div class="grid grid-cols-3 gap-2" {
                        input type="number" id="timeValue" min="1" step="1"
                            class="input-brutal-box col-span-2"
                            value=(time_value)
                            disabled[!is_overridden];
                        select id="timeUnit"
                            class="input-brutal-box"
                            disabled[!is_overridden] {
                            option value="minutes" selected[time_unit == "minutes"] { "Minutes" }
                            option value="hours" selected[time_unit == "hours"] { "Hours" }
                            option value="days" selected[time_unit == "days"] { "Days" }
                        }
                    }
                }

                // Max fill percentage
                div {
                    label class="label-brutal mb-2 block" for="pctValue" { "MAX % OF POOL PER FILL" }
                    div class="flex items-center gap-2" {
                        input type="number" id="pctValue" min="0.01" max="100" step="0.01"
                            class="input-brutal-box flex-1"
                            value=(format!("{}", pct_value))
                            disabled[!is_overridden];
                        span class="font-black text-primary" { "%" }
                    }
                }

                div id="payoutMessage" class="text-sm font-bold hidden" {}

                button type="submit" class="w-full btn-brutal-fill" {
                    "SAVE PAYOUT SETTINGS"
                }
            }
        }

        (PreEscaped(format!(r#"
        <script>
            (function() {{
                const SECS_PER_MINUTE = 60;
                const SECS_PER_HOUR = 60 * 60;
                const SECS_PER_DAY = 24 * 60 * 60;

                const form = document.getElementById('payoutForm');
                const overrideEnabled = document.getElementById('payoutOverrideEnabled');
                const timeValue = document.getElementById('timeValue');
                const timeUnit = document.getElementById('timeUnit');
                const pctValue = document.getElementById('pctValue');
                const message = document.getElementById('payoutMessage');

                function syncDisabled() {{
                    const enabled = overrideEnabled.checked;
                    timeValue.disabled = !enabled;
                    timeUnit.disabled = !enabled;
                    pctValue.disabled = !enabled;
                }}

                overrideEnabled.addEventListener('change', syncDisabled);

                function toSeconds(value, unit) {{
                    switch (unit) {{
                        case 'minutes': return Math.round(value * SECS_PER_MINUTE);
                        case 'hours': return Math.round(value * SECS_PER_HOUR);
                        case 'days': return Math.round(value * SECS_PER_DAY);
                        default: return Math.round(value);
                    }}
                }}

                function showMessage(text, ok) {{
                    message.textContent = text;
                    message.classList.remove('hidden');
                    message.style.color = ok ? 'var(--highlight)' : 'var(--text-primary)';
                }}

                form.addEventListener('submit', async function(e) {{
                    e.preventDefault();

                    let timeSecs = null;
                    let maxPct = null;

                    if (overrideEnabled.checked) {{
                        const rawTime = parseFloat(timeValue.value);
                        if (!isFinite(rawTime) || rawTime <= 0) {{
                            showMessage('Enter a positive time-to-full value.', false);
                            return;
                        }}
                        timeSecs = toSeconds(rawTime, timeUnit.value);

                        const rawPct = parseFloat(pctValue.value);
                        if (!isFinite(rawPct) || rawPct <= 0 || rawPct > 100) {{
                            showMessage('Enter a percentage between 0 and 100.', false);
                            return;
                        }}
                        maxPct = rawPct / 100.0;
                    }}

                    try {{
                        const response = await fetch('/api/locations/{location_id}/payout', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify({{
                                time_to_full_secs: timeSecs,
                                max_fill_percentage: maxPct,
                            }}),
                        }});

                        if (response.ok) {{
                            showMessage('Payout settings saved.', true);
                        }} else {{
                            const text = await response.text();
                            showMessage('Failed to save: ' + (text || response.status), false);
                        }}
                    }} catch (err) {{
                        showMessage('Failed to save: ' + err.message, false);
                    }}
                }});

                syncDisabled();
            }})();
        </script>
        "#, location_id = location.id)))
    }
}
