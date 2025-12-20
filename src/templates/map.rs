use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn map(locations: &[Location], max_sats_per_location: i64) -> Markup {
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
                @for location in locations {
                    (location_card(location, max_sats_per_location))
                }
                @if locations.is_empty() {
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

        // Map initialization script
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
            const locations = {locations};
            const maxSatsPerLocation = {max_sats_per_location};
            const bounds = new maplibregl.LngLatBounds();

            locations.forEach(loc => {{
                // Calculate withdrawable amount (accounting for 2 sat fee + 0.5% routing fee)
                const routingFeeMsats = Math.ceil(loc.current_msats * 0.005);
                const fixedFeeMsats = 2000;
                const withdrawableMsats = Math.max(0, loc.current_msats - routingFeeMsats - fixedFeeMsats);
                const withdrawableSats = Math.floor(withdrawableMsats / 1000);

                const satsPercent = (withdrawableSats / maxSatsPerLocation) * 100;
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
                                <p style="margin: 4px 0;"><i class="fa-solid fa-bolt"></i> ${{withdrawableSats}} / ${{maxSatsPerLocation}} sats</p>
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
        locations = serde_json::to_string(locations).unwrap_or_else(|_| "[]".to_string()),
        max_sats_per_location = max_sats_per_location
        )))
    }
}

fn location_card(location: &Location, max_sats_per_location: i64) -> Markup {
    let withdrawable_sats = location.withdrawable_sats();
    let sats_percent = if max_sats_per_location > 0 {
        (withdrawable_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
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
                    @if sats_percent > 50 {
                        div class="text-2xl font-black text-primary" {
                            (withdrawable_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    } @else {
                        div class="text-2xl font-black text-highlight orange" {
                            (withdrawable_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                    }
                    div class="text-muted text-sm mono" {
                        "/ " (max_sats_per_location) " SATS"
                    }
                }
            }
        }
    }
}
