use crate::models::{Location, User};
use maud::{html, Markup};

pub fn profile(_user: &User, locations: &[Location], max_sats_per_location: i64) -> Markup {
    html! {
        div class="max-w-6xl mx-auto" {
            // Locations section
            div class="mb-8" {
                div class="flex justify-between items-center mb-6" {
                    h1 class="text-4xl font-bold text-highlight" {
                        "My Locations "
                        span class="text-secondary" { "[" (locations.len()) "]" }
                    }
                    a href="/locations/new" class="btn-primary" {
                        i class="fa-solid fa-plus mr-2" {}
                        "Add New Location"
                    }
                }

                @if locations.is_empty() {
                    div class="bg-secondary rounded-lg p-12 border border-accent-muted text-center" {
                        div class="text-6xl mb-4 opacity-50" {
                            i class="fa-solid fa-location-dot" {}
                        }
                        h3 class="text-2xl font-bold text-primary mb-2" { "No locations yet" }
                        p class="text-secondary mb-6" {
                            "Create your first treasure location and start sharing sats with the world!"
                        }
                        a href="/locations/new" class="btn-primary" {
                            "Create Your First Location"
                        }
                    }
                } @else {
                    div class="grid gap-4" {
                        @for location in locations {
                            (location_card(location, max_sats_per_location))
                        }
                    }
                }
            }
        }
    }
}

fn location_card(location: &Location, max_sats_per_location: i64) -> Markup {
    let sats_percent = if max_sats_per_location > 0 {
        (location.current_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    let color_class = if sats_percent > 50 {
        "text-success"
    } else if sats_percent > 20 {
        "text-warning"
    } else {
        "text-error"
    };

    html! {
        a href={"/locations/" (location.id)}
            class="card-interactive" {
            div class="flex flex-col md:flex-row md:justify-between md:items-center gap-4" {
                // Left: Location info
                div class="flex-1" {
                    h3 class="text-xl font-semibold text-highlight mb-2" { (location.name) }
                    @if let Some(desc) = &location.description {
                        p class="text-secondary text-sm mb-2 line-clamp-2" { (desc) }
                    }
                    div class="flex items-center gap-4 text-sm text-muted" {
                        span {
                            i class="fa-solid fa-location-dot mr-1" {}
                            (format!("{:.4}, {:.4}", location.latitude, location.longitude))
                        }
                        span {
                            i class="fa-solid fa-calendar mr-1" {}
                            (location.created_at.format("%b %d, %Y").to_string())
                        }
                    }
                }

                // Right: Stats
                div class="flex md:flex-col gap-4 md:gap-2 md:items-end" {
                    div class="text-right" {
                        div class=(format!("text-2xl font-bold {}", color_class)) {
                            (location.current_sats) " "
                            i class="fa-solid fa-bolt" {}
                        }
                        div class="text-muted text-sm" {
                            "/ " (max_sats_per_location) " sats"
                        }
                    }

                    // Progress bar
                    div class="w-32" {
                        div class={
                            "progress "
                            @if sats_percent > 50 { "progress-success" }
                            @else if sats_percent > 20 { "progress-warning" }
                            @else { "progress-error" }
                        } {
                            div class="progress-bar" style=(format!("width: {}%", sats_percent)) {}
                        }
                    }
                }
            }
        }
    }
}
