use maud::{html, Markup, DOCTYPE};

pub fn base(title: &str, content: Markup) -> Markup {
    base_with_user(title, content, None)
}

pub fn base_with_user(title: &str, content: Markup, username: Option<&str>) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" class="dark" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                meta name="theme-color" content="#F7931A";
                title { (title) " - SatsHunt" }

                // Favicons
                link rel="icon" type="image/x-icon" href="/static/favicon.ico";
                link rel="icon" type="image/png" sizes="16x16" href="/static/images/favicon-16x16.png";
                link rel="icon" type="image/png" sizes="32x32" href="/static/images/favicon-32x32.png";
                link rel="apple-touch-icon" sizes="180x180" href="/static/images/apple-touch-icon.png";
                link rel="manifest" href="/static/site.webmanifest";

                // Tailwind CSS CDN
                script src="https://cdn.tailwindcss.com" {}

                // Brutalist design system (loaded after Tailwind to override)
                link rel="stylesheet" href="/static/css/brutalist.css";

                // Font Awesome
                link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.1/css/all.min.css"
                    integrity="sha512-DTOQO9RWCH3ppGqcWaEA1BIZOC6xxalwEsw9c2QQeAIftl+Vegovlnee1c9QX4TctnWMn13TZye+giMm8e2LwA=="
                    crossorigin="anonymous" referrerpolicy="no-referrer";

                // HTMX
                script src="https://unpkg.com/htmx.org@1.9.10" {}

                // MapLibre GL JS for maps
                link rel="stylesheet" href="https://unpkg.com/maplibre-gl@4.7.1/dist/maplibre-gl.css";
                script src="https://unpkg.com/maplibre-gl@4.7.1/dist/maplibre-gl.js" {}
            }
            body {
                (navbar(username))
                main class="content-container py-8" {
                    (content)
                }
                (footer())

                // Mobile menu toggle script
                script {
                    (maud::PreEscaped(r#"
                    document.addEventListener('DOMContentLoaded', function() {
                        const toggleBtn = document.querySelector('[data-collapse-toggle]');
                        if (toggleBtn) {
                            toggleBtn.addEventListener('click', function() {
                                const targetId = this.getAttribute('data-collapse-toggle');
                                const target = document.getElementById(targetId);
                                if (target) {
                                    target.classList.toggle('hidden');
                                    const expanded = !target.classList.contains('hidden');
                                    this.setAttribute('aria-expanded', expanded);
                                }
                            });
                        }
                    });
                    "#))
                }
            }
        }
    }
}

fn navbar(username: Option<&str>) -> Markup {
    html! {
        nav class="bg-secondary" style="border-bottom: 3px solid var(--accent-border);" {
            div class="content-container py-4" {
                div class="flex items-center justify-between" {
                    // Left: Logo
                    a href="/" class="flex items-center space-x-2" style="border-bottom: none;" {
                        img src="/static/images/satshunt_logo_small.png" alt="SatsHunt Logo" class="h-10 w-10";
                        span class="text-2xl font-black whitespace-nowrap text-highlight" style="letter-spacing: -0.02em;" {
                            "SATSHUNT"
                        }
                    }

                    // Center: Menu items (desktop)
                    div class="hidden md:flex md:items-center md:justify-center md:flex-1" {
                        ul class="flex space-x-8" {
                            li {
                                a href="/" class="text-primary transition hover:text-highlight font-bold" {
                                    "HOME"
                                }
                            }
                            li {
                                a href="/map" class="text-primary transition hover:text-highlight font-bold" {
                                    "MAP"
                                }
                            }
                            li {
                                a href="/locations/new" class="text-primary transition hover:text-highlight font-bold" {
                                    "ADD LOCATION"
                                }
                            }
                            li {
                                a href="/donate" class="text-highlight transition hover:text-primary font-bold orange" {
                                    i class="fa-solid fa-coins mr-2" {}
                                    "DONATE"
                                }
                            }
                        }
                    }

                    // Right: Login status (desktop)
                    div class="hidden md:flex md:items-center md:gap-3" {
                        @if let Some(user) = username {
                            a href="/profile" class="flex items-center gap-2 px-3 py-2 bg-tertiary text-primary text-sm font-bold mono" style="border: 2px solid var(--accent-muted);" {
                                i class="fa-solid fa-user" {}
                                (user)
                            }
                            form action="/logout" method="post" {
                                button type="submit"
                                    class="px-3 py-2 text-muted hover:text-primary text-sm font-bold" {
                                    i class="fa-solid fa-right-from-bracket mr-1" {}
                                    "LOGOUT"
                                }
                            }
                        } @else {
                            a href="/login"
                                class="px-3 py-2 text-primary text-sm font-bold" {
                                "LOGIN"
                            }
                            a href="/register"
                                class="btn-brutal-orange text-sm" style="padding: 0.5rem 1rem;" {
                                "REGISTER"
                            }
                        }
                    }

                    // Mobile menu button
                    button data-collapse-toggle="navbar-mobile" type="button"
                        class="inline-flex items-center p-2 w-10 h-10 justify-center text-primary md:hidden hover:bg-tertiary focus:outline-none"
                        style="border: 3px solid var(--accent-muted);"
                        aria-controls="navbar-mobile" aria-expanded="false" {
                        span class="sr-only" { "Open main menu" }
                        svg class="w-5 h-5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 17 14" {
                            path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M1 1h15M1 7h15M1 13h15";
                        }
                    }
                }

                // Mobile menu (collapsed by default)
                div class="hidden w-full md:hidden" id="navbar-mobile" style="border-top: 3px solid var(--accent-muted); margin-top: 1rem;" {
                    // Menu items
                    ul class="flex flex-col py-4" {
                        li {
                            a href="/" class="block py-3 text-primary font-bold hover:text-highlight" style="border-bottom: none;" {
                                "HOME"
                            }
                        }
                        li {
                            a href="/map" class="block py-3 text-primary font-bold hover:text-highlight" style="border-bottom: none;" {
                                "MAP"
                            }
                        }
                        li {
                            a href="/locations/new" class="block py-3 text-primary font-bold hover:text-highlight" style="border-bottom: none;" {
                                "ADD LOCATION"
                            }
                        }
                        li {
                            a href="/donate" class="block py-3 text-highlight font-bold hover:text-primary orange" style="border-bottom: none;" {
                                i class="fa-solid fa-coins mr-2" {}
                                "DONATE"
                            }
                        }
                    }

                    // Login status (mobile)
                    div class="py-4" style="border-top: 3px solid var(--accent-muted);" {
                        @if let Some(user) = username {
                            div class="space-y-3" {
                                a href="/profile" class="flex items-center gap-2 py-2 px-3 bg-tertiary text-primary font-bold mono" style="border: 3px solid var(--accent-muted); border-bottom: 3px solid var(--accent-muted);" {
                                    i class="fa-solid fa-user" {}
                                    (user)
                                }
                                form action="/logout" method="post" {
                                    button type="submit"
                                        class="w-full py-2 px-3 text-muted hover:text-primary font-bold text-left" style="border: none; background: none;" {
                                        i class="fa-solid fa-right-from-bracket mr-2" {}
                                        "LOGOUT"
                                    }
                                }
                            }
                        } @else {
                            div class="space-y-3" {
                                a href="/login"
                                    class="block py-2 px-3 text-primary font-bold" style="border-bottom: none;" {
                                    "LOGIN"
                                }
                                a href="/register"
                                    class="btn-brutal-orange block text-center" {
                                    "REGISTER"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn footer() -> Markup {
    html! {
        footer class="bg-secondary mt-16" style="border-top: 3px solid var(--accent-border);" {
            div class="content-container py-4 md:py-8" {
                div class="sm:flex sm:items-center sm:justify-between" {
                    span class="text-sm text-secondary sm:text-center font-bold mono" {
                        "© 2024 SATSHUNT · LIGHTNING TREASURE HUNT"
                    }
                    div class="flex mt-4 space-x-5 sm:justify-center sm:mt-0" {
                        a href="https://github.com" class="text-secondary hover:text-primary font-bold" {
                            span class="sr-only" { "GitHub" }
                            "GitHub"
                        }
                    }
                }
            }
        }
    }
}
