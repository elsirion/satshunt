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
                "EXPLORE LOCATIONS AROUND YOU. GREEN MARKERS HAVE MORE SATS AVAILABLE, RED MARKERS ARE NEARLY EMPTY."
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

        // Map initialization script - build JSON manually with computed balances
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

            // Add locations as markers
            const locations = {locations_json};
            const bounds = new maplibregl.LngLatBounds();

            locations.forEach(loc => {{
                const satsPercent = loc.pool_sats > 0 ? (loc.available_sats / loc.pool_sats * 10) * 100 : 0;
                const color = satsPercent > 50 ? '#22c55e' : satsPercent > 20 ? '#eab308' : '#ef4444';

                // Create custom marker element
                const el = document.createElement('div');
                el.style.width = '20px';
                el.style.height = '20px';
                el.style.borderRadius = '50%';
                el.style.backgroundColor = color;
                el.style.border = '2px solid #fff';
                el.style.cursor = 'pointer';
                el.style.boxShadow = '0 2px 4px rgba(0,0,0,0.3)';

                const marker = new maplibregl.Marker({{element: el}})
                    .setLngLat([loc.longitude, loc.latitude])
                    .setPopup(new maplibregl.Popup({{ offset: 25 }})
                        .setHTML(`
                            <div style="color: #0f172a; padding: 8px;">
                                <h3 style="font-weight: bold; margin-bottom: 4px;">${{loc.name}}</h3>
                                <p style="margin: 4px 0;"><i class="fa-solid fa-bolt"></i> ${{loc.available_sats}} sats available</p>
                                <p style="margin: 4px 0; font-size: 0.9em; color: #666;">Pool: ${{loc.pool_sats}} sats</p>
                                <a href="/locations/${{loc.id}}" style="color: #3b82f6; text-decoration: underline;">View details</a>
                            </div>
                        `))
                    .addTo(map);

                bounds.extend([loc.longitude, loc.latitude]);
            }});

            if (locations.length > 0) {{
                map.fitBounds(bounds, {{ padding: 50, animate: false }});
            }}
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
                r#"{{"id":"{}","name":"{}","latitude":{},"longitude":{},"available_sats":{},"pool_sats":{}}}"#,
                loc.id,
                loc.name.replace('"', r#"\""#),
                loc.latitude,
                loc.longitude,
                available_sats,
                pool_sats
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
                            (available_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    } @else {
                        div class="text-2xl font-black text-highlight orange" {
                            (available_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="text-muted text-sm mono" {
                        "POOL: " (pool_sats) " SATS"
                    }
                }
            }
        }
    }
}
