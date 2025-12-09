use crate::models::{Location, User};
use maud::{html, Markup};

pub fn profile(_user: &User, locations: &[Location], max_sats_per_location: i64) -> Markup {
    html! {
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

fn location_card(location: &Location, max_sats_per_location: i64) -> Markup {
    // Calculate percentage based on withdrawable amount (after fees)
    let withdrawable_sats = location.withdrawable_sats();
    let sats_percent = if max_sats_per_location > 0 {
        (withdrawable_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
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

    // Determine status badge and color
    let (status_text, status_color, status_icon) = match location.status.as_str() {
        "created" => ("Created", "bg-yellow-600", "fa-solid fa-clock"),
        "programmed" => ("Programmed", "bg-blue-600", "fa-solid fa-microchip"),
        "active" => ("Active", "bg-green-600", "fa-solid fa-check"),
        _ => ("Unknown", "bg-gray-600", "fa-solid fa-question"),
    };

    html! {
        div class="bg-secondary rounded-lg p-6 border border-accent-muted hover:border-accent transition-colors" {
            div class="flex flex-col gap-4" {
                // Header with name and status
                div class="flex justify-between items-start gap-4" {
                    div class="flex-1" {
                        h3 class="text-xl font-semibold text-highlight mb-2" { (location.name) }
                        @if let Some(desc) = &location.description {
                            p class="text-secondary text-sm mb-2 line-clamp-2" { (desc) }
                        }
                    }
                    div class=(format!("px-3 py-1 rounded-full text-white text-sm font-semibold {}", status_color)) {
                        i class=(format!("{} mr-1", status_icon)) {}
                        (status_text)
                    }
                }

                // Location info
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

                // Stats (only show for active locations)
                @if location.is_active() {
                    div class="flex justify-between items-center pt-4 border-t border-accent-muted" {
                        div class="text-right" {
                            div class=(format!("text-2xl font-bold {}", color_class)) {
                                (withdrawable_sats) " "
                                i class="fa-solid fa-bolt" {}
                            }
                            div class="text-muted text-sm" {
                                "/ " (max_sats_per_location) " sats"
                            }
                        }

                        // Progress bar
                        div class="flex-1 max-w-xs ml-4" {
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

                // Action buttons based on status
                div class="flex gap-2 pt-4 border-t border-accent-muted" {
                    @if location.is_created() || location.is_programmed() {
                        // Location needs to be programmed or can retry programming
                        @if let Some(token) = &location.write_token {
                            a href={"/setup/" (token)}
                                class="btn-primary flex-1 text-center" {
                                i class="fa-solid fa-microchip mr-2" {}
                                @if location.is_created() {
                                    "Program NFC Sticker"
                                } @else {
                                    "Re-program NFC Sticker"
                                }
                            }
                        }

                        @if location.is_programmed() {
                            // Show waiting message in addition to re-program button
                            div class="text-center px-4 py-2 bg-blue-900 border border-blue-700 text-blue-200 rounded-lg text-sm" {
                                i class="fa-solid fa-info-circle mr-1" {}
                                "Waiting for first scan to activate"
                            }
                        }
                    }

                    // View location button (always available)
                    a href={"/locations/" (location.id)}
                        class={
                            "btn-secondary text-center "
                            @if !location.is_created() && !location.is_programmed() { "flex-1" }
                        } {
                        i class="fa-solid fa-eye mr-2" {}
                        "View Details"
                    }

                    // Delete button (only for non-active locations)
                    @if !location.is_active() {
                        button
                            onclick={
                                "if(confirm('Are you sure you want to delete this location? This cannot be undone.')) { "
                                "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                ".then(r => r.ok ? window.location.reload() : alert('Failed to delete location')) "
                                "}"
                            }
                            class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-semibold transition-colors" {
                            i class="fa-solid fa-trash mr-2" {}
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
