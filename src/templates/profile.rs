use crate::models::{Location, User};
use maud::{html, Markup};

pub fn profile(_user: &User, locations: &[Location], max_sats_per_location: i64) -> Markup {
    html! {
        // Locations section
        div class="mb-8" {
                div class="flex justify-between items-center mb-8" {
                    h1 class="text-4xl font-black text-primary" style="letter-spacing: -0.02em;" {
                        "MY LOCATIONS "
                        span class="text-muted mono" { "[" (locations.len()) "]" }
                    }
                    a href="/locations/new" class="btn-brutal-orange" {
                        i class="fa-solid fa-plus mr-2" {}
                        "ADD LOCATION"
                    }
                }

                @if locations.is_empty() {
                    div class="card-brutal-inset text-center" style="padding: 3rem;" {
                        div class="text-6xl mb-6 text-muted" {
                            i class="fa-solid fa-location-dot" {}
                        }
                        h3 class="text-2xl font-black text-primary mb-3" { "NO LOCATIONS YET" }
                        p class="text-secondary mb-8 font-bold" {
                            "CREATE YOUR FIRST TREASURE LOCATION AND START SHARING SATS WITH THE WORLD!"
                        }
                        a href="/locations/new" class="btn-brutal-fill" {
                            "CREATE FIRST LOCATION"
                        }
                    }
                } @else {
                    div class="space-y-4" {
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

    // Determine status text
    let status_text = match location.status.as_str() {
        "created" => "CREATED",
        "programmed" => "PROGRAMMED",
        "active" => "ACTIVE",
        _ => "UNKNOWN",
    };

    html! {
        div class="card-brutal" {
            div class="flex flex-col gap-4" {
                // Header with name and status
                div class="flex justify-between items-start gap-4" {
                    div class="flex-1" {
                        h3 class="text-xl font-black text-primary mb-2" { (location.name) }
                        @if let Some(desc) = &location.description {
                            p class="text-secondary text-sm mb-2 font-bold" { (desc) }
                        }
                    }
                    @if location.is_active() {
                        span class="badge-brutal filled" { (status_text) }
                    } @else if location.is_programmed() {
                        span class="badge-brutal grey" { (status_text) }
                    } @else {
                        span class="badge-brutal white" { (status_text) }
                    }
                }

                // Location info
                div class="flex items-center gap-4 text-sm text-muted font-bold mono" {
                    span {
                        i class="fa-solid fa-location-dot mr-1" {}
                        (format!("{:.4}, {:.4}", location.latitude, location.longitude))
                    }
                    span {
                        i class="fa-solid fa-calendar mr-1" {}
                        (location.created_at.format("%Y-%m-%d").to_string())
                    }
                }

                // Stats (only show for active locations)
                @if location.is_active() {
                    div class="pt-4" style="border-top: 3px solid var(--accent-muted);" {
                        div class="flex justify-between items-center mb-3" {
                            div class="label-brutal" { "BALANCE" }
                            div class="text-muted text-xs mono" {
                                (withdrawable_sats) " / " (max_sats_per_location) " SATS"
                            }
                        }
                        div class="progress-brutal" {
                            @if sats_percent > 50 {
                                div class="progress-brutal-bar" style=(format!("width: {}%", sats_percent)) {
                                    div class="progress-brutal-value" { (sats_percent) "%" }
                                }
                            } @else {
                                div class="progress-brutal-bar orange" style=(format!("width: {}%", sats_percent)) {
                                    div class="progress-brutal-value" { (sats_percent) "%" }
                                }
                            }
                        }
                    }
                }

                // Action buttons based on status
                div class="flex gap-2 pt-4" style="border-top: 3px solid var(--accent-muted);" {
                    @if location.is_created() || location.is_programmed() {
                        // Location needs to be programmed or can retry programming
                        @if let Some(token) = &location.write_token {
                            a href={"/setup/" (token)}
                                class="btn-brutal-orange flex-1 text-center" {
                                i class="fa-solid fa-microchip mr-2" {}
                                @if location.is_created() {
                                    "PROGRAM NFC"
                                } @else {
                                    "RE-PROGRAM NFC"
                                }
                            }
                        }

                        @if location.is_programmed() {
                            // Show waiting message in addition to re-program button
                            div class="alert-brutal" style="font-size: 0.75rem; padding: 0.5rem 0.75rem;" {
                                "WAITING FOR FIRST SCAN TO ACTIVATE"
                            }
                        }
                    }

                    // View location button (always available)
                    a href={"/locations/" (location.id)}
                        class={
                            "btn-brutal text-center "
                            @if !location.is_created() && !location.is_programmed() { "flex-1" }
                        } {
                        i class="fa-solid fa-eye mr-2" {}
                        "VIEW DETAILS"
                    }

                    // Delete button (only for non-active locations)
                    @if !location.is_active() {
                        button
                            onclick={
                                "if(confirm('DELETE THIS LOCATION? THIS CANNOT BE UNDONE.')) { "
                                "fetch('/api/locations/" (location.id) "', { method: 'DELETE' }) "
                                ".then(r => r.ok ? window.location.reload() : alert('FAILED TO DELETE')) "
                                "}"
                            }
                            class="btn-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                            i class="fa-solid fa-trash mr-2" {}
                            "DELETE"
                        }
                    }
                }
            }
        }
    }
}
