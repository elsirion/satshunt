use maud::{html, Markup};

pub fn login(error: Option<&str>) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-highlight" { "Login" }

        div class="max-w-md mx-auto" {
            form action="/login" method="post"
                class="bg-secondary rounded-lg p-8 border border-accent-muted space-y-6" {

                @if let Some(error_msg) = error {
                    div class="bg-error border border-error text-primary px-4 py-3 rounded-lg" {
                        (error_msg)
                    }
                }

                // Username field
                div {
                    label for="username" class="block mb-2 text-sm font-medium text-primary" {
                        "Username"
                    }
                    input type="text" id="username" name="username" required autofocus
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Enter your username";
                }

                // Password field
                div {
                    label for="password" class="block mb-2 text-sm font-medium text-primary" {
                        "Password"
                    }
                    input type="password" id="password" name="password" required
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Enter your password";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full btn-primary" {
                        "Login"
                    }
                }

                // Register link
                div class="text-center" {
                    p class="text-sm text-muted" {
                        "Don't have an account? "
                        a href="/register" class="text-highlight hover:bg-accent-hover" {
                            "Register here"
                        }
                    }
                }
            }
        }
    }
}
