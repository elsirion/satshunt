use crate::models::Stats;
use maud::{html, Markup};

pub fn home(stats: &Stats) -> Markup {
    html! {
        // Hero section
        div class="text-center mb-16" {
            h1 class="text-5xl font-bold mb-4 text-highlight" {
                "Welcome to SatsHunt"
            }
            p class="text-xl text-secondary mb-8" {
                "A real-world treasure hunt powered by Bitcoin Lightning"
            }
            div class="flex gap-4 justify-center" {
                a href="/map"
                    class="px-6 py-3 btn-primary" {
                    i class="fa-solid fa-map mr-2" {}
                    "View Treasure Map"
                }
                a href="/locations/new"
                    class="px-6 py-3 btn-secondary" {
                    i class="fa-solid fa-plus mr-2" {}
                    "Add Location"
                }
            }
        }

        // Stats section
        div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-16" {
            (stat_card(
                html! { i class="fa-solid fa-location-dot" {} },
                "Locations",
                &stats.total_locations.to_string()
            ))
            (stat_card(
                html! { i class="fa-solid fa-bolt" {} },
                "Sats Available",
                &format!("{}", stats.total_sats_available)
            ))
            (stat_card(
                html! {
                    svg style="height: 1em; width: 1em; display: inline-block;" viewBox="0 0 24 24" {
                        path fill="currentColor" d="M7.24 2C5.6 2 3.96 2 3.55 2.04C2.67 2.09 2.08 2.73 2.04 3.56C2 4.37 2 19.59 2.04 20.41C2.09 21.23 2.71 21.86 3.55 21.91C4.46 21.96 7.44 21.97 8.29 21.97C6.76 20.91 6.55 18.92 6.41 15.23C6.33 13.04 6.4 5.36 6.41 5.04L6.45 2.94L14.5 11V13.5L8.09 7.11C8.08 8.38 8.06 10.03 8.06 11.54C8.06 13 8.08 14.34 8.12 15.05C8.36 19.07 8.74 20.96 10.83 21.7C11.5 21.93 12.07 22 13.07 22C13.89 22 19.63 22 20.45 21.96C21.33 21.91 21.93 21.27 21.97 20.44C22 19.63 22 4.45 21.97 3.62C21.91 2.8 21.29 2.18 20.45 2.13C19.54 2.08 16.57 2.03 15.71 2.03C17.24 3.09 17.44 5.08 17.59 8.78C17.67 10.97 17.6 18.64 17.59 18.97L17.55 21.06L9.53 13V10.5L15.91 16.89C15.92 15.62 15.94 13.97 15.94 12.46C15.94 11 15.92 9.66 15.88 8.96C15.64 4.93 15.26 3.04 13.17 2.3C12.53 2.07 11.93 2 10.93 2H7.24Z" {}
                    }
                },
                "Total Scans",
                &stats.total_scans.to_string()
            ))
            (stat_card(
                html! { i class="fa-solid fa-coins" {} },
                "Donation Pool",
                &format!("{} sats", stats.donation_pool_sats)
            ))
        }

        // How it works section
        div class="bg-secondary rounded-lg p-8 mb-16 border border-accent-muted" {
            h2 class="text-3xl font-bold mb-6 text-highlight" { "How It Works" }

            div class="grid md:grid-cols-3 gap-8" {
                (step("1", "Find Locations", "Browse the map to find treasure locations near you. Each location has NFC stickers with sats waiting to be claimed."))
                (step("2", "Scan NFC Tag", "When you reach a location, use your Lightning wallet to scan the NFC sticker. It will offer you the available sats via LNURL-withdraw."))
                (step("3", "Claim Sats", "Accept the withdrawal in your wallet and the sats are yours! Locations refill over time from the donation pool."))
            }
        }

        // About section
        div class="bg-secondary rounded-lg p-8 border border-accent-muted" {
            h2 class="text-3xl font-bold mb-6 text-highlight" { "About SatsHunt" }
            div class="space-y-4 text-secondary" {
                p {
                    "SatsHunt is a real-world treasure hunting game powered by Bitcoin's Lightning Network. "
                    "Hide sats in interesting locations around the world using NFC stickers, and let others discover and claim them."
                }
                p {
                    "Each location contains an NFC tag with an LNURL-withdraw link. When scanned with a Lightning wallet, "
                    "it allows the finder to instantly claim the available satoshis. Locations automatically refill from a "
                    "community donation pool, keeping the game going."
                }
                p class="font-semibold text-highlight" {
                    "Get outside, explore new places, and stack sats!"
                }
            }
        }
    }
}

fn stat_card(icon: Markup, label: &str, value: &str) -> Markup {
    html! {
        div class="bg-secondary rounded-lg p-6 border border-accent-muted text-center" {
            div class="text-4xl mb-2 h-10 flex items-center justify-center" {
                (icon)
            }
            div class="text-3xl font-bold text-highlight mb-1" { (value) }
            div class="text-muted" { (label) }
        }
    }
}

fn step(number: &str, title: &str, description: &str) -> Markup {
    html! {
        div class="text-center" {
            div class="w-12 h-12 bg-highlight text-inverse rounded-full flex items-center justify-center text-xl font-bold mx-auto mb-4" {
                (number)
            }
            h3 class="text-xl font-semibold mb-2 text-highlight" { (title) }
            p class="text-secondary" { (description) }
        }
    }
}
