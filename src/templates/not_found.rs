use maud::{html, Markup};

pub fn not_found(message: Option<&str>) -> Markup {
    let message = message.unwrap_or("This page doesn't exist or has been removed.");
    html! {
        div class="max-w-2xl mx-auto text-center py-12" {
            div class="mono text-9xl font-black text-highlight orange mb-4" style="letter-spacing: -0.05em;" {
                "404"
            }
            h1 class="text-4xl font-black mb-6 text-primary" style="letter-spacing: -0.02em;" {
                "NOT FOUND"
            }
            p class="text-secondary font-bold mb-8" {
                (message)
            }
            div class="flex flex-wrap items-center justify-center gap-3" {
                a href="/" class="btn-brutal-fill" {
                    i class="fa-solid fa-house mr-2" {}
                    "BACK TO HOME"
                }
                a href="/map" class="btn-brutal" {
                    i class="fa-solid fa-map mr-2" {}
                    "OPEN MAP"
                }
            }
        }
    }
}
