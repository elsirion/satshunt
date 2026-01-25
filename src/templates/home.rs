use crate::models::Stats;
use maud::{html, Markup};

pub fn home(stats: &Stats) -> Markup {
    html! {
        // Hero section
        div class="text-center mb-12" {
            h1 class="text-5xl font-black mb-3 text-highlight" style="letter-spacing: -0.02em;" {
                "SATSHUNT"
            }
            p class="text-lg text-secondary mb-8 font-bold" {
                "REAL-WORLD BITCOIN TREASURE HUNT"
            }
            div class="flex gap-4 justify-center" {
                a href="/map"
                    class="btn-brutal-orange" {
                    i class="fa-solid fa-map mr-2" {}
                    "VIEW MAP"
                }
                a href="/locations/new"
                    class="btn-brutal" {
                    i class="fa-solid fa-plus mr-2" {}
                    "ADD LOCATION"
                }
            }
        }

        // Stats section
        div class="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12" {
            (stat_card(
                "Locations",
                &stats.total_locations.to_string(),
                false,
                "fa-location-dot"
            ))
            (stat_card(
                "Sats Available",
                &format!("{}", stats.total_sats_available),
                true,
                "fa-bolt"
            ))
            (stat_card(
                "Total Scans",
                &stats.total_scans.to_string(),
                false,
                "nfc-svg"
            ))
        }

        // How it works section
        div class="card-brutal-inset mb-12" {
            h3 class="heading-breaker orange" { "HOW IT WORKS" }

            div class="grid md:grid-cols-3 gap-6 mt-8" {
                (step("1", "FIND LOCATIONS", "Browse the map to find treasure locations near you. Each location has NFC stickers with sats waiting to be claimed."))
                (step("2", "SCAN NFC TAG", "When you reach a location, scan the NFC sticker with your phone to claim the available sats."))
                (step("3", "CLAIM SATS", "The sats are added to your wallet instantly! Locations refill over time from the donation pool."))
            }
        }

        // About section
        div class="card-bar" {
            h2 class="text-2xl font-black mb-6" { "ABOUT SATSHUNT" }
            div class="space-y-4 text-secondary" {
                p class="font-bold" {
                    "SatsHunt is a real-world treasure hunting game powered by Bitcoin's Lightning Network. "
                    "Hide sats in interesting locations around the world using NFC stickers, and let others discover and claim them."
                }
                p class="font-bold" {
                    "Each location contains an NFC tag that lets finders instantly claim the available satoshis. "
                    "Locations automatically refill from a community donation pool, keeping the game going."
                }
                p class="font-black text-highlight text-lg" {
                    "Get outside, explore new places, and stack sats!"
                }
            }
        }
    }
}

fn stat_card(label: &str, value: &str, is_orange: bool, icon: &str) -> Markup {
    html! {
        div class="stat-brutal" style="min-height: 200px; display: flex; flex-direction: column; justify-content: center;" {
            // Icon above the value
            div class="mb-4 text-5xl" style="display: flex; justify-content: center;" {
                @if is_orange {
                    @if icon == "nfc-svg" {
                        svg class="text-highlight" style="height: 1.2em; width: 1.2em;" viewBox="0 0 24 24" {
                            path fill="currentColor" d="M7.24 2C5.6 2 3.96 2 3.55 2.04C2.67 2.09 2.08 2.73 2.04 3.56C2 4.37 2 19.59 2.04 20.41C2.09 21.23 2.71 21.86 3.55 21.91C4.46 21.96 7.44 21.97 8.29 21.97C6.76 20.91 6.55 18.92 6.41 15.23C6.33 13.04 6.4 5.36 6.41 5.04L6.45 2.94L14.5 11V13.5L8.09 7.11C8.08 8.38 8.06 10.03 8.06 11.54C8.06 13 8.08 14.34 8.12 15.05C8.36 19.07 8.74 20.96 10.83 21.7C11.5 21.93 12.07 22 13.07 22C13.89 22 19.63 22 20.45 21.96C21.33 21.91 21.93 21.27 21.97 20.44C22 19.63 22 4.45 21.97 3.62C21.91 2.8 21.29 2.18 20.45 2.13C19.54 2.08 16.57 2.03 15.71 2.03C17.24 3.09 17.44 5.08 17.59 8.78C17.67 10.97 17.6 18.64 17.59 18.97L17.55 21.06L9.53 13V10.5L15.91 16.89C15.92 15.62 15.94 13.97 15.94 12.46C15.94 11 15.92 9.66 15.88 8.96C15.64 4.93 15.26 3.04 13.17 2.3C12.53 2.07 11.93 2 10.93 2H7.24Z" {}
                        }
                    } @else {
                        i class={"fa-solid " (icon) " text-highlight"} {}
                    }
                } @else {
                    @if icon == "nfc-svg" {
                        svg class="text-primary" style="height: 1.2em; width: 1.2em;" viewBox="0 0 24 24" {
                            path fill="currentColor" d="M7.24 2C5.6 2 3.96 2 3.55 2.04C2.67 2.09 2.08 2.73 2.04 3.56C2 4.37 2 19.59 2.04 20.41C2.09 21.23 2.71 21.86 3.55 21.91C4.46 21.96 7.44 21.97 8.29 21.97C6.76 20.91 6.55 18.92 6.41 15.23C6.33 13.04 6.4 5.36 6.41 5.04L6.45 2.94L14.5 11V13.5L8.09 7.11C8.08 8.38 8.06 10.03 8.06 11.54C8.06 13 8.08 14.34 8.12 15.05C8.36 19.07 8.74 20.96 10.83 21.7C11.5 21.93 12.07 22 13.07 22C13.89 22 19.63 22 20.45 21.96C21.33 21.91 21.93 21.27 21.97 20.44C22 19.63 22 4.45 21.97 3.62C21.91 2.8 21.29 2.18 20.45 2.13C19.54 2.08 16.57 2.03 15.71 2.03C17.24 3.09 17.44 5.08 17.59 8.78C17.67 10.97 17.6 18.64 17.59 18.97L17.55 21.06L9.53 13V10.5L15.91 16.89C15.92 15.62 15.94 13.97 15.94 12.46C15.94 11 15.92 9.66 15.88 8.96C15.64 4.93 15.26 3.04 13.17 2.3C12.53 2.07 11.93 2 10.93 2H7.24Z" {}
                        }
                    } @else {
                        i class={"fa-solid " (icon) " text-primary"} {}
                    }
                }
            }
            // Value
            @if is_orange {
                div class="stat-value orange" {
                    (value)
                }
            } @else {
                div class="stat-value" {
                    (value)
                }
            }
            // Label
            div class="stat-label" { (label) }
        }
    }
}

fn step(number: &str, title: &str, description: &str) -> Markup {
    html! {
        div {
            div class="badge-brutal filled orange mb-4" style="font-size: 1rem; padding: 0.5rem 0.75rem;" {
                (number)
            }
            h3 class="text-lg font-black mb-2" { (title) }
            p class="text-secondary text-sm font-bold" { (description) }
        }
    }
}
