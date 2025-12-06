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
                title { (title) " - SatShunt" }

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

                // Custom styles
                style {
                    "
                    body {
                        background-color: #0f172a;
                        color: #e2e8f0;
                    }
                    "
                }
            }
            body class="bg-slate-900 text-slate-200" {
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
        nav class="bg-slate-800 border-b border-slate-700" {
            div class="max-w-screen-xl mx-auto p-4" {
                div class="flex items-center justify-between" {
                    // Left: Logo
                    a href="/" class="flex items-center space-x-3" {
                        span class="text-2xl font-semibold whitespace-nowrap text-yellow-400" {
                            "⚡ SatShunt"
                        }
                    }

                    // Center: Menu items (desktop)
                    div class="hidden md:flex md:items-center md:justify-center md:flex-1" {
                        ul class="flex space-x-8" {
                            li {
                                a href="/" class="text-slate-200 hover:text-yellow-400 transition" {
                                    "Home"
                                }
                            }
                            li {
                                a href="/map" class="text-slate-200 hover:text-yellow-400 transition" {
                                    "Map"
                                }
                            }
                            li {
                                a href="/locations/new" class="text-slate-200 hover:text-yellow-400 transition" {
                                    "Add Location"
                                }
                            }
                            li {
                                a href="/donate" class="text-yellow-400 hover:text-yellow-300 transition" {
                                    "💰 Donate"
                                }
                            }
                        }
                    }

                    // Right: Login status (desktop)
                    div class="hidden md:flex md:items-center md:space-x-4" {
                        @if let Some(user) = username {
                            span class="text-slate-300 text-sm" {
                                "👤 " (user)
                            }
                            form action="/logout" method="post" class="inline" {
                                button type="submit"
                                    class="text-slate-400 hover:text-yellow-400 text-sm transition" {
                                    "Logout"
                                }
                            }
                        } @else {
                            a href="/login"
                                class="text-slate-200 hover:text-yellow-400 text-sm transition" {
                                "Login"
                            }
                            a href="/register"
                                class="px-4 py-2 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg text-sm transition" {
                                "Register"
                            }
                        }
                    }

                    // Mobile menu button
                    button data-collapse-toggle="navbar-mobile" type="button"
                        class="inline-flex items-center p-2 w-10 h-10 justify-center text-slate-400 rounded-lg md:hidden hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-slate-600"
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
                            a href="/" class="block py-2 px-3 text-slate-200 rounded hover:bg-slate-700" {
                                "Home"
                            }
                        }
                        li {
                            a href="/map" class="block py-2 px-3 text-slate-200 rounded hover:bg-slate-700" {
                                "Map"
                            }
                        }
                        li {
                            a href="/locations/new" class="block py-2 px-3 text-slate-200 rounded hover:bg-slate-700" {
                                "Add Location"
                            }
                        }
                        li {
                            a href="/donate" class="block py-2 px-3 text-yellow-400 rounded hover:bg-slate-700" {
                                "💰 Donate"
                            }
                        }
                    }

                    // Login status (mobile)
                    div class="mt-4 pt-4 border-t border-slate-700" {
                        @if let Some(user) = username {
                            div class="flex items-center justify-between px-3 py-2" {
                                span class="text-slate-300 text-sm" {
                                    "👤 " (user)
                                }
                                form action="/logout" method="post" class="inline" {
                                    button type="submit"
                                        class="text-slate-400 hover:text-yellow-400 text-sm transition" {
                                        "Logout"
                                    }
                                }
                            }
                        } @else {
                            div class="flex flex-col space-y-2 px-3 py-2" {
                                a href="/login"
                                    class="block py-2 px-3 text-slate-200 rounded hover:bg-slate-700 text-center" {
                                    "Login"
                                }
                                a href="/register"
                                    class="block py-2 px-3 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg text-center transition" {
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
        footer class="bg-slate-800 border-t border-slate-700 mt-16" {
            div class="max-w-screen-xl mx-auto p-4 md:p-8" {
                div class="sm:flex sm:items-center sm:justify-between" {
                    span class="text-sm text-slate-400 sm:text-center" {
                        "© 2024 SatShunt. A Lightning treasure hunt game."
                    }
                    div class="flex mt-4 space-x-5 sm:justify-center sm:mt-0" {
                        a href="https://github.com" class="text-slate-400 hover:text-yellow-400" {
                            span class="sr-only" { "GitHub" }
                            "GitHub"
                        }
                    }
                }
            }
        }
    }
}
