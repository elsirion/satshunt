use crate::models::Location;
use maud::{html, Markup, PreEscaped};

pub fn admin_locations(locations: &[Location], max_sats_per_location: i64) -> Markup {
    let active: Vec<_> = locations.iter().filter(|l| l.is_active()).collect();
    let deactivated: Vec<_> = locations.iter().filter(|l| l.is_deactivated()).collect();
    let admin_deactivated: Vec<_> = locations
        .iter()
        .filter(|l| l.is_admin_deactivated())
        .collect();
    let programmed: Vec<_> = locations.iter().filter(|l| l.is_programmed()).collect();
    let created: Vec<_> = locations.iter().filter(|l| l.is_created()).collect();

    let active_count = active.len();
    let deactivated_count = deactivated.len();
    let admin_deactivated_count = admin_deactivated.len();
    let programmed_count = programmed.len();
    let created_count = created.len();
    let total_count = locations.len();

    html! {
        div class="mb-8" {
            div class="flex justify-between items-center mb-8" {
                h1 class="text-4xl font-black text-primary" style="letter-spacing: -0.02em;" {
                    "LOCATION MANAGEMENT"
                }
            }

            // Filter buttons
            div class="flex flex-wrap gap-2 mb-6" {
                button type="button"
                    class="btn-brutal-fill"
                    id="filter-all"
                    onclick="filterLocations('all')" {
                    "ALL "
                    span class="mono" { "[" (total_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-active"
                    onclick="filterLocations('active')" {
                    "ACTIVE "
                    span class="mono" { "[" (active_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-deactivated"
                    onclick="filterLocations('deactivated')" {
                    "DEACTIVATED "
                    span class="mono" { "[" (deactivated_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-admin_deactivated"
                    onclick="filterLocations('admin_deactivated')" {
                    "ADMIN DEACTIVATED "
                    span class="mono" { "[" (admin_deactivated_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-programmed"
                    onclick="filterLocations('programmed')" {
                    "PROGRAMMED "
                    span class="mono" { "[" (programmed_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-created"
                    onclick="filterLocations('created')" {
                    "CREATED "
                    span class="mono" { "[" (created_count) "]" }
                }
            }

            @if locations.is_empty() {
                div class="card-brutal-inset text-center" style="padding: 3rem;" {
                    div class="text-6xl mb-6 text-muted" {
                        i class="fa-solid fa-location-dot" {}
                    }
                    h3 class="text-2xl font-black text-primary mb-3" { "NO LOCATIONS" }
                    p class="text-secondary mb-8 font-bold" {
                        "NO LOCATIONS FOUND IN THE SYSTEM."
                    }
                }
            } @else {
                div class="space-y-4" id="locations-list" {
                    @for location in locations {
                        (location_card(location, max_sats_per_location))
                    }
                }
            }

            // Filter script
            script {
                (PreEscaped(r#"
                function filterLocations(filter) {
                    const cards = document.querySelectorAll('[data-location-status]');
                    cards.forEach(card => {
                        const status = card.getAttribute('data-location-status');
                        if (filter === 'all' || status === filter) {
                            card.style.display = '';
                        } else {
                            card.style.display = 'none';
                        }
                    });

                    // Update button styles
                    const filters = ['all', 'active', 'deactivated', 'admin_deactivated', 'programmed', 'created'];
                    filters.forEach(f => {
                        const btn = document.getElementById('filter-' + f);
                        if (f === filter) {
                            btn.className = 'btn-brutal-fill';
                        } else {
                            btn.className = 'btn-brutal';
                        }
                    });
                }

                // Initialize with all filter
                document.addEventListener('DOMContentLoaded', function() {
                    filterLocations('all');
                });
                "#))
            }
        }
    }
}

fn location_card(location: &Location, max_sats_per_location: i64) -> Markup {
    let status = if location.is_active() {
        "active"
    } else if location.is_deactivated() {
        "deactivated"
    } else if location.is_admin_deactivated() {
        "admin_deactivated"
    } else if location.is_programmed() {
        "programmed"
    } else {
        "created"
    };

    let status_badge = match status {
        "active" => html! { span class="badge-brutal filled" { "ACTIVE" } },
        "deactivated" => html! {
            span class="badge-brutal grey" {
                i class="fa-solid fa-pause mr-1" {}
                "DEACTIVATED"
            }
        },
        "admin_deactivated" => html! {
            span class="badge-brutal" style="border-color: var(--highlight); color: var(--highlight);" {
                i class="fa-solid fa-ban mr-1" {}
                "ADMIN DEACTIVATED"
            }
        },
        "programmed" => html! { span class="badge-brutal grey" { "PROGRAMMED" } },
        _ => html! { span class="badge-brutal white" { "CREATED" } },
    };

    let withdrawable_sats = location.withdrawable_sats();
    let sats_percent = if max_sats_per_location > 0 {
        (withdrawable_sats as f64 / max_sats_per_location as f64 * 100.0) as i32
    } else {
        0
    };

    html! {
        div class="card-brutal" data-location-status=(status) {
            div class="flex flex-col gap-4" {
                // Header with name and status
                div class="flex justify-between items-start gap-4" {
                    div class="flex-1" {
                        h3 class="text-xl font-black text-primary mb-2" {
                            a href={"/locations/" (location.id)} class="hover:text-highlight transition-colors" {
                                (location.name)
                            }
                        }
                        @if let Some(desc) = &location.description {
                            p class="text-secondary text-sm mb-2 font-bold" { (desc) }
                        }
                    }

                    div class="flex items-center gap-2" {
                        // Deactivate/reactivate button
                        @if location.is_active() {
                            button
                                onclick={
                                    "if(confirm('ADMIN DEACTIVATE THIS LOCATION?')) { "
                                    "fetch('/api/locations/" (location.id) "/deactivate', { method: 'POST' }) "
                                    ".then(r => r.ok ? location.reload() : alert('FAILED')) "
                                    "}"
                                }
                                class="btn-brutal" style="border-color: var(--accent-muted); color: var(--text-muted); padding: 0.25rem 0.5rem;"
                                title="Admin deactivate" {
                                i class="fa-solid fa-pause" {}
                            }
                        } @else if location.is_deactivated() || location.is_admin_deactivated() {
                            button
                                onclick={
                                    "fetch('/api/locations/" (location.id) "/reactivate', { method: 'POST' }) "
                                    ".then(r => r.ok ? location.reload() : alert('FAILED')) "
                                }
                                class="btn-brutal-fill" style="padding: 0.25rem 0.5rem;"
                                title="Reactivate" {
                                i class="fa-solid fa-play" {}
                            }
                        }

                        (status_badge)
                    }
                }

                // Location info
                div class="flex flex-wrap items-center gap-4 text-sm text-muted font-bold mono" {
                    span {
                        i class="fa-solid fa-location-dot mr-1" {}
                        (format!("{:.4}, {:.4}", location.latitude, location.longitude))
                    }
                    span {
                        i class="fa-solid fa-calendar mr-1" {}
                        (location.created_at.format("%Y-%m-%d").to_string())
                    }
                    span {
                        i class="fa-solid fa-user mr-1" {}
                        (&location.user_id[..8]) "..."
                    }
                }

                // Balance (for active/deactivated locations)
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
            }
        }
    }
}
