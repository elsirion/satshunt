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

    html! {
        // Use orange border for inactive locations to draw attention
        @if location.is_active() {
            div class="card-brutal" {
                (location_card_content(location, max_sats_per_location, withdrawable_sats, sats_percent))
            }
        } @else {
            div class="card-brutal" style="border-color: var(--highlight);" {
                (location_card_content(location, max_sats_per_location, withdrawable_sats, sats_percent))
            }
        }
    }
}

fn location_card_content(
    location: &Location,
    max_sats_per_location: i64,
    withdrawable_sats: i64,
    sats_percent: i32,
) -> Markup {
    html! {
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
                    span class="badge-brutal filled" { "ACTIVE" }
                } @else if location.is_deactivated() {
                    span class="badge-brutal grey" {
                        i class="fa-solid fa-pause mr-1" {}
                        "DEACTIVATED"
                    }
                } @else if location.is_admin_deactivated() {
                    span class="badge-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                        i class="fa-solid fa-ban mr-1" {}
                        "ADMIN DEACTIVATED"
                    }
                } @else if location.is_programmed() {
                    span class="badge-brutal orange" {
                        i class="fa-solid fa-hourglass-half mr-1" {}
                        "SCAN TO ACTIVATE"
                    }
                } @else {
                    span class="badge-brutal orange" {
                        i class="fa-solid fa-wrench mr-1" {}
                        "NEEDS SETUP"
                    }
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

            // Stats (show for active and deactivated locations)
            @if location.is_active() || location.is_deactivated() || location.is_admin_deactivated() {
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

            // Action button
            div class="pt-4" style="border-top: 3px solid var(--accent-muted);" {
                @if location.is_active() {
                    div class="flex gap-2" {
                        a href={"/locations/" (location.id)}
                            class="btn-brutal text-center flex-1" {
                            i class="fa-solid fa-info-circle mr-2" {}
                            "INFO"
                        }
                        button
                            onclick={
                                "if(confirm('DEACTIVATE THIS LOCATION? Users will no longer be able to collect sats.')) { "
                                "fetch('/api/locations/" (location.id) "/deactivate', { method: 'POST' }) "
                                ".then(r => r.ok ? location.reload() : alert('FAILED TO DEACTIVATE')) "
                                "}"
                            }
                            class="btn-brutal" style="border-color: var(--accent-muted); color: var(--text-muted);" {
                            i class="fa-solid fa-pause" {}
                        }
                    }
                } @else if location.is_deactivated() {
                    div class="flex gap-2" {
                        a href={"/locations/" (location.id)}
                            class="btn-brutal text-center flex-1" {
                            i class="fa-solid fa-info-circle mr-2" {}
                            "INFO"
                        }
                        button
                            onclick={
                                "fetch('/api/locations/" (location.id) "/reactivate', { method: 'POST' }) "
                                ".then(r => r.ok ? location.reload() : alert('FAILED TO REACTIVATE')) "
                            }
                            class="btn-brutal-fill" {
                            i class="fa-solid fa-play mr-2" {}
                            "REACTIVATE"
                        }
                    }
                } @else if location.is_admin_deactivated() {
                    div class="flex gap-2" {
                        a href={"/locations/" (location.id)}
                            class="btn-brutal text-center flex-1" {
                            i class="fa-solid fa-info-circle mr-2" {}
                            "INFO"
                        }
                        span class="btn-brutal text-center" style="border-color: var(--accent-muted); color: var(--text-muted); cursor: not-allowed;" {
                            i class="fa-solid fa-lock mr-2" {}
                            "CONTACT ADMIN"
                        }
                    }
                } @else {
                    a href={"/locations/" (location.id)}
                        class="btn-brutal-orange text-center w-full block" {
                        i class="fa-solid fa-arrow-right mr-2" {}
                        "CONTINUE SETUP"
                    }
                }
            }
        }
    }
}
