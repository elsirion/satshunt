use crate::models::Stats;
use maud::{html, Markup};

pub fn home(stats: &Stats) -> Markup {
    html! {
        // Hero section
        div class="text-center mb-16" {
            h1 class="text-5xl font-bold mb-4 bg-gradient-to-r from-yellow-400 to-orange-500 bg-clip-text text-transparent" {
                "Welcome to SatShunt"
            }
            p class="text-xl text-slate-300 mb-8" {
                "A real-world treasure hunt powered by Bitcoin Lightning"
            }
            div class="flex gap-4 justify-center" {
                a href="/map"
                    class="px-6 py-3 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg transition" {
                    "🗺️ View Treasure Map"
                }
                a href="/locations/new"
                    class="px-6 py-3 bg-slate-700 hover:bg-slate-600 text-slate-200 font-semibold rounded-lg transition" {
                    "➕ Add Location"
                }
            }
        }

        // Stats section
        div class="grid grid-cols-1 md:grid-cols-4 gap-6 mb-16" {
            (stat_card("🎯", "Locations", &stats.total_locations.to_string()))
            (stat_card("⚡", "Sats Available", &format!("{}", stats.total_sats_available)))
            (stat_card("📱", "Total Scans", &stats.total_scans.to_string()))
            (stat_card("💰", "Donation Pool", &format!("{} sats", stats.donation_pool_sats)))
        }

        // How it works section
        div class="bg-slate-800 rounded-lg p-8 mb-16 border border-slate-700" {
            h2 class="text-3xl font-bold mb-6 text-yellow-400" { "How It Works" }

            div class="grid md:grid-cols-3 gap-8" {
                (step("1", "Find Locations", "Browse the map to find treasure locations near you. Each location has NFC stickers with sats waiting to be claimed."))
                (step("2", "Scan NFC Tag", "When you reach a location, use your Lightning wallet to scan the NFC sticker. It will offer you the available sats via LNURL-withdraw."))
                (step("3", "Claim Sats", "Accept the withdrawal in your wallet and the sats are yours! Locations refill over time from the donation pool."))
            }
        }

        // About section
        div class="bg-slate-800 rounded-lg p-8 border border-slate-700" {
            h2 class="text-3xl font-bold mb-6 text-yellow-400" { "About SatShunt" }
            div class="space-y-4 text-slate-300" {
                p {
                    "SatShunt is a real-world treasure hunting game powered by Bitcoin's Lightning Network. "
                    "Hide sats in interesting locations around the world using NFC stickers, and let others discover and claim them."
                }
                p {
                    "Each location contains an NFC tag with an LNURL-withdraw link. When scanned with a Lightning wallet, "
                    "it allows the finder to instantly claim the available satoshis. Locations automatically refill from a "
                    "community donation pool, keeping the game going."
                }
                p class="font-semibold text-yellow-400" {
                    "Get outside, explore new places, and stack sats!"
                }
            }
        }
    }
}

fn stat_card(icon: &str, label: &str, value: &str) -> Markup {
    html! {
        div class="bg-slate-800 rounded-lg p-6 border border-slate-700 text-center" {
            div class="text-4xl mb-2" { (icon) }
            div class="text-3xl font-bold text-yellow-400 mb-1" { (value) }
            div class="text-slate-400" { (label) }
        }
    }
}

fn step(number: &str, title: &str, description: &str) -> Markup {
    html! {
        div class="text-center" {
            div class="w-12 h-12 bg-yellow-500 text-slate-900 rounded-full flex items-center justify-center text-xl font-bold mx-auto mb-4" {
                (number)
            }
            h3 class="text-xl font-semibold mb-2 text-yellow-400" { (title) }
            p class="text-slate-300" { (description) }
        }
    }
}
