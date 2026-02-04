use super::format_sats_si;
use crate::models::Location;
use maud::{html, Markup, PreEscaped};

/// Display the map with locations and their computed balances
/// location_balances is a slice of (location, available_sats, pool_sats)
pub fn map(location_balances: &[(&Location, i64, i64)]) -> Markup {
    html! {
        h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" {
            i class="fa-solid fa-map mr-2" {}
            "TREASURE MAP"
        }

        div class="alert-brutal mb-8" {
            p class="text-sm font-bold" {
                "EXPLORE LOCATIONS AROUND YOU. TAP MARKERS TO SEE AVAILABLE SATS."
            }
        }

        // Map container
        div id="map" class="w-full h-96 mb-8" style="border: 3px solid var(--accent-border);" {}

        // Locations list
        div class="card-brutal-inset" {
            h2 class="heading-breaker" { "ALL LOCATIONS" }
            div class="grid gap-4" {
                @for (location, available_sats, pool_sats) in location_balances {
                    (location_card(location, *available_sats, *pool_sats))
                }
                @if location_balances.is_empty() {
                    div class="text-center py-8" {
                        p class="text-muted font-bold mb-4" {
                            "NO LOCATIONS YET. BE THE FIRST TO "
                            a href="/locations/new" class="text-highlight orange" {
                                "ADD ONE"
                            }
                            "!"
                        }
                    }
                }
            }
        }

        // Map initialization script - build GeoJSON for clustering
        (PreEscaped(format!(r#"
        <script>
            // Initialize map with MapLibre
            const map = new maplibregl.Map({{
                container: 'map',
                style: 'https://tiles.openfreemap.org/styles/positron',
                center: [-122.4194, 37.7749],
                zoom: 12
            }});

            map.addControl(new maplibregl.NavigationControl());

            // Location data as GeoJSON
            const locations = {locations_json};
            const geojson = {{
                type: 'FeatureCollection',
                features: locations.map(loc => ({{
                    type: 'Feature',
                    geometry: {{ type: 'Point', coordinates: [loc.longitude, loc.latitude] }},
                    properties: loc
                }}))
            }};

            const bounds = new maplibregl.LngLatBounds();
            locations.forEach(loc => bounds.extend([loc.longitude, loc.latitude]));

            map.on('load', () => {{
                // Add clustered source
                map.addSource('locations', {{
                    type: 'geojson',
                    data: geojson,
                    cluster: true,
                    clusterMaxZoom: 14,
                    clusterRadius: 50
                }});

                // Cluster circles
                map.addLayer({{
                    id: 'clusters',
                    type: 'circle',
                    source: 'locations',
                    filter: ['has', 'point_count'],
                    paint: {{
                        'circle-color': '#F7931A',
                        'circle-radius': ['step', ['get', 'point_count'], 18, 10, 24, 50, 32],
                        'circle-stroke-width': 2,
                        'circle-stroke-color': '#fff'
                    }}
                }});

                // Cluster count labels
                map.addLayer({{
                    id: 'cluster-count',
                    type: 'symbol',
                    source: 'locations',
                    filter: ['has', 'point_count'],
                    layout: {{
                        'text-field': '{{point_count_abbreviated}}',
                        'text-font': ['Noto Sans Bold'],
                        'text-size': 12
                    }},
                    paint: {{
                        'text-color': '#fff'
                    }}
                }});

                // Individual location points
                map.addLayer({{
                    id: 'unclustered-point',
                    type: 'circle',
                    source: 'locations',
                    filter: ['!', ['has', 'point_count']],
                    paint: {{
                        'circle-color': '#F7931A',
                        'circle-radius': 8,
                        'circle-stroke-width': 2,
                        'circle-stroke-color': '#fff'
                    }}
                }});

                // Click on cluster to zoom in
                map.on('click', 'clusters', async (e) => {{
                    const features = map.queryRenderedFeatures(e.point, {{ layers: ['clusters'] }});
                    const clusterId = features[0].properties.cluster_id;
                    const source = map.getSource('locations');
                    try {{
                        const zoom = await source.getClusterExpansionZoom(clusterId);
                        map.easeTo({{
                            center: features[0].geometry.coordinates,
                            zoom: zoom
                        }});
                    }} catch (err) {{
                        console.error('Error expanding cluster:', err);
                    }}
                }});

                // Click on individual point to show popup
                map.on('click', 'unclustered-point', (e) => {{
                    const loc = e.features[0].properties;
                    new maplibregl.Popup({{ offset: 15 }})
                        .setLngLat(e.features[0].geometry.coordinates)
                        .setHTML(`
                            <div style="color: #0f172a; padding: 8px;">
                                <h3 style="font-weight: bold; margin-bottom: 4px;">${{loc.name}}</h3>
                                <p style="margin: 4px 0;"><i class="fa-solid fa-bolt"></i> ${{loc.available_sats_fmt}} sats available</p>
                                <p style="margin: 4px 0; font-size: 0.9em; color: #666;">Pool: ${{loc.pool_sats_fmt}} sats</p>
                                <a href="/locations/${{loc.id}}" style="color: #3b82f6; text-decoration: underline;">View details</a>
                            </div>
                        `)
                        .addTo(map);
                }});

                // Change cursor on hover
                map.on('mouseenter', 'clusters', () => {{ map.getCanvas().style.cursor = 'pointer'; }});
                map.on('mouseleave', 'clusters', () => {{ map.getCanvas().style.cursor = ''; }});
                map.on('mouseenter', 'unclustered-point', () => {{ map.getCanvas().style.cursor = 'pointer'; }});
                map.on('mouseleave', 'unclustered-point', () => {{ map.getCanvas().style.cursor = ''; }});

                // Fit bounds after layers are added
                if (locations.length > 0) {{
                    map.fitBounds(bounds, {{ padding: 50, animate: false }});
                }}
            }});
        </script>
        "#,
        locations_json = build_locations_json(location_balances)
        )))
    }
}

/// Build JSON array for map markers with computed balances
fn build_locations_json(location_balances: &[(&Location, i64, i64)]) -> String {
    let items: Vec<String> = location_balances
        .iter()
        .map(|(loc, available_sats, pool_sats)| {
            format!(
                r#"{{"id":"{}","name":"{}","latitude":{},"longitude":{},"available_sats":{},"pool_sats":{},"available_sats_fmt":"{}","pool_sats_fmt":"{}"}}"#,
                loc.id,
                loc.name.replace('"', r#"\""#),
                loc.latitude,
                loc.longitude,
                available_sats,
                pool_sats,
                format_sats_si(*available_sats),
                format_sats_si(*pool_sats)
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

fn location_card(location: &Location, available_sats: i64, pool_sats: i64) -> Markup {
    // Color based on how full the location is relative to its pool
    let fill_percent = if pool_sats > 0 {
        ((available_sats as f64 / (pool_sats as f64 * 0.1)) * 100.0) as i32
    } else {
        0
    };

    html! {
        a href={"/locations/" (location.id)}
            class="block card-brutal transition hover:bg-elevated" {
            div class="flex justify-between items-start gap-4" {
                div class="flex-1" {
                    h3 class="text-xl font-black text-primary mb-2" { (location.name) }
                    @if let Some(desc) = &location.description {
                        p class="text-secondary text-sm mb-2 font-bold" { (desc) }
                    }
                    p class="text-muted text-sm mono" {
                        i class="fa-solid fa-location-dot mr-1" {}
                        (format!("{:.4}, {:.4}", location.latitude, location.longitude))
                    }
                }
                div class="text-right" {
                    @if fill_percent > 50 {
                        div class="text-2xl font-black text-primary" {
                            (format_sats_si(available_sats)) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    } @else {
                        div class="text-2xl font-black text-highlight orange" {
                            (format_sats_si(available_sats)) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="text-muted text-sm mono" {
                        "POOL: " (format_sats_si(pool_sats)) " SATS"
                    }
                }
            }
        }
    }
}
