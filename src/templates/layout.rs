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

                // Custom color palette
                link rel="stylesheet" href="/static/css/colors.css";

                // Tailwind CSS CDN
                script src="https://cdn.tailwindcss.com" {}

                // Flowbite CSS & JS
                link href="https://cdn.jsdelivr.net/npm/flowbite@2.5.1/dist/flowbite.min.css" rel="stylesheet";
                script src="https://cdn.jsdelivr.net/npm/flowbite@2.5.1/dist/flowbite.min.js" defer {}

                // HTMX
                script src="https://unpkg.com/htmx.org@1.9.10" {}

                // Leaflet for maps
                link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css"
                    integrity="sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY="
                    crossorigin="";
                script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"
                    integrity="sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo="
                    crossorigin="" {}
            }
            body {
                (navbar(username))
                main class="container mx-auto px-4 py-8" {
                    (content)
                }
                (footer())
            }
        }
    }
}

fn navbar(username: Option<&str>) -> Markup {
    html! {
        nav class="bg-secondary border-b border-accent-muted" {
            div class="max-w-screen-xl mx-auto p-4" {
                div class="flex items-center justify-between" {
                    // Left: Logo
                    a href="/" class="flex items-center space-x-2" {
                        img src="/static/images/satshunt_logo_small.png" alt="SatsHunt Logo" class="h-10 w-10";
                        span class="text-2xl font-semibold whitespace-nowrap text-highlight" {
                            "SatsHunt"
                        }
                    }

                    // Center: Menu items (desktop)
                    div class="hidden md:flex md:items-center md:justify-center md:flex-1" {
                        ul class="flex space-x-8" {
                            li {
                                a href="/" class="text-primary transition hover:text-accent" {
                                    "Home"
                                }
                            }
                            li {
                                a href="/map" class="text-primary transition hover:text-accent" {
                                    "Map"
                                }
                            }
                            li {
                                a href="/locations/new" class="text-primary transition hover:text-accent" {
                                    "Add Location"
                                }
                            }
                            li {
                                a href="/donate" class="text-highlight transition hover:brightness-110" {
                                    "💰 Donate"
                                }
                            }
                        }
                    }

                    // Right: Login status (desktop)
                    div class="hidden md:flex md:items-center md:space-x-4" {
                        @if let Some(user) = username {
                            a href="/profile" class="text-primary hover:text-accent text-sm transition flex items-center gap-1" {
                                "👤 " (user)
                            }
                            form action="/logout" method="post" class="inline" {
                                button type="submit"
                                    class="text-secondary hover:text-accent text-sm transition" {
                                    "Logout"
                                }
                            }
                        } @else {
                            a href="/login"
                                class="text-primary hover:text-accent text-sm transition" {
                                "Login"
                            }
                            a href="/register"
                                class="btn-primary text-sm" {
                                "Register"
                            }
                        }
                    }

                    // Mobile menu button
                    button data-collapse-toggle="navbar-mobile" type="button"
                        class="inline-flex items-center p-2 w-10 h-10 justify-center text-secondary rounded-lg md:hidden hover:bg-tertiary focus:outline-none focus:ring-2 focus:ring-accent-muted"
                        aria-controls="navbar-mobile" aria-expanded="false" {
                        span class="sr-only" { "Open main menu" }
                        svg class="w-5 h-5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 17 14" {
                            path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M1 1h15M1 7h15M1 13h15";
                        }
                    }
                }

                // Mobile menu (collapsed by default)
                div class="hidden w-full md:hidden" id="navbar-mobile" {
                    // Menu items
                    ul class="flex flex-col mt-4 space-y-2" {
                        li {
                            a href="/" class="block py-2 px-3 text-primary rounded hover:bg-tertiary" {
                                "Home"
                            }
                        }
                        li {
                            a href="/map" class="block py-2 px-3 text-primary rounded hover:bg-tertiary" {
                                "Map"
                            }
                        }
                        li {
                            a href="/locations/new" class="block py-2 px-3 text-primary rounded hover:bg-tertiary" {
                                "Add Location"
                            }
                        }
                        li {
                            a href="/donate" class="block py-2 px-3 text-highlight rounded hover:bg-tertiary" {
                                "💰 Donate"
                            }
                        }
                    }

                    // Login status (mobile)
                    div class="mt-4 pt-4 border-t border-accent-muted" {
                        @if let Some(user) = username {
                            div class="px-3 py-2 space-y-2" {
                                a href="/profile" class="block py-2 px-3 text-primary rounded hover:bg-tertiary text-center" {
                                    "👤 " (user) " - View Profile"
                                }
                                form action="/logout" method="post" {
                                    button type="submit"
                                        class="w-full py-2 px-3 text-secondary hover:text-accent text-sm transition rounded hover:bg-tertiary" {
                                        "Logout"
                                    }
                                }
                            }
                        } @else {
                            div class="flex flex-col space-y-2 px-3 py-2" {
                                a href="/login"
                                    class="block py-2 px-3 text-primary rounded hover:bg-tertiary text-center" {
                                    "Login"
                                }
                                a href="/register"
                                    class="btn-primary text-center" {
                                    "Register"
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
        footer class="bg-secondary border-t border-accent-muted mt-16" {
            div class="max-w-screen-xl mx-auto p-4 md:p-8" {
                div class="sm:flex sm:items-center sm:justify-between" {
                    span class="text-sm text-secondary sm:text-center" {
                        "© 2024 SatsHunt. A Lightning treasure hunt game."
                    }
                    div class="flex mt-4 space-x-5 sm:justify-center sm:mt-0" {
                        a href="https://github.com" class="text-secondary hover:text-accent" {
                            span class="sr-only" { "GitHub" }
                            "GitHub"
                        }
                    }
                }
            }
        }
    }
}
