use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn map(locations: &[Location]) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-yellow-400" { "🗺️ Treasure Map" }

        div class="bg-slate-800 rounded-lg p-4 mb-8 border border-slate-700" {
            p class="text-slate-300" {
                "Explore locations around you. Green markers have more sats available, red markers are nearly empty."
            }
        }

        // Map container
        div id="map" class="w-full h-96 rounded-lg border border-slate-700 mb-8" {}

        // Locations list
        div class="bg-slate-800 rounded-lg p-6 border border-slate-700" {
            h2 class="text-2xl font-bold mb-4 text-yellow-400" { "All Locations" }
            div class="grid gap-4" {
                @for location in locations {
                    (location_card(location))
                }
                @if locations.is_empty() {
                    p class="text-slate-400 text-center py-8" {
                        "No locations yet. Be the first to "
                        a href="/locations/new" class="text-yellow-400 hover:text-yellow-300" {
                            "add one"
                        }
                        "!"
                    }
                }
            }
        }

        // Map initialization script
        (PreEscaped(format!(r#"
        <script>
            // Initialize map
            const map = L.map('map').setView([37.7749, -122.4194], 12);

            L.tileLayer('https://{{s}}.tile.openstreetmap.org/{{z}}/{{x}}/{{y}}.png', {{
                attribution: '© OpenStreetMap contributors',
                className: 'map-tiles'
            }}).addTo(map);

            // Add custom CSS for dark theme
            const style = document.createElement('style');
            style.textContent = `
                .map-tiles {{
                    filter: invert(100%) hue-rotate(180deg) brightness(95%) contrast(90%);
                }}
            `;
            document.head.appendChild(style);

            // Add locations as markers
            const locations = {};
            let bounds = [];

            locations.forEach(loc => {{
                const satsPercent = (loc.current_sats / loc.max_sats) * 100;
                const color = satsPercent > 50 ? '#22c55e' : satsPercent > 20 ? '#eab308' : '#ef4444';

                const marker = L.circleMarker([loc.latitude, loc.longitude], {{
                    radius: 8,
                    fillColor: color,
                    color: '#fff',
                    weight: 2,
                    opacity: 1,
                    fillOpacity: 0.8
                }}).addTo(map);

                marker.bindPopup(`
                    <div style="color: #0f172a;">
                        <h3 style="font-weight: bold; margin-bottom: 4px;">${{loc.name}}</h3>
                        <p style="margin: 4px 0;">⚡ ${{loc.current_sats}} / ${{loc.max_sats}} sats</p>
                        <a href="/locations/${{loc.id}}" style="color: #3b82f6; text-decoration: underline;">View details</a>
                    </div>
                `);

                bounds.push([loc.latitude, loc.longitude]);
            }});

            if (bounds.length > 0) {{
                map.fitBounds(bounds, {{ padding: [50, 50] }});
            }}
        </script>
        "#, locations = serde_json::to_string(locations).unwrap_or_else(|_| "[]".to_string()))))
    }
}

fn location_card(location: &Location) -> Markup {
    let sats_percent = if location.max_sats > 0 {
        (location.current_sats as f64 / location.max_sats as f64 * 100.0) as i32
    } else {
        0
    };

    let color_class = if sats_percent > 50 {
        "text-green-400"
    } else if sats_percent > 20 {
        "text-yellow-400"
    } else {
        "text-red-400"
    };

    html! {
        a href={"/locations/" (location.id)}
            class="block p-4 bg-slate-700 hover:bg-slate-600 rounded-lg transition border border-slate-600" {
            div class="flex justify-between items-start" {
                div {
                    h3 class="text-xl font-semibold text-yellow-400 mb-2" { (location.name) }
                    @if let Some(desc) = &location.description {
                        p class="text-slate-300 text-sm mb-2" { (desc) }
                    }
                    p class="text-slate-400 text-sm" {
                        "📍 " (format!("{:.4}, {:.4}", location.latitude, location.longitude))
                    }
                }
                div class="text-right" {
                    div class=(format!("text-2xl font-bold {}", color_class)) {
                        (location.current_sats) " ⚡"
                    }
                    div class="text-slate-400 text-sm" {
                        "/ " (location.max_sats) " sats"
                    }
                }
            }
        }
    }
}
