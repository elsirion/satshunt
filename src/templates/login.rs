use maud::{html, Markup};

pub fn login(error: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto" {
            h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" { "LOGIN" }

            form action="/login" method="post"
                class="card-brutal-inset space-y-6" {

                @if let Some(error_msg) = error {
                    div class="alert-brutal orange" {
                        (error_msg)
                    }
                }

                // Username field
                div {
                    label for="username" class="label-brutal" {
                        "USERNAME"
                    }
                    input type="text" id="username" name="username" required autofocus
                        class="input-brutal-box w-full"
                        placeholder="ENTER USERNAME";
                }

                // Password field
                div {
                    label for="password" class="label-brutal" {
                        "PASSWORD"
                    }
                    input type="password" id="password" name="password" required
                        class="input-brutal-box w-full"
                        placeholder="ENTER PASSWORD";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full btn-brutal-fill" {
                        "LOGIN"
                    }
                }

                // Register link
                div class="text-center" {
                    p class="text-sm text-muted font-bold" {
                        "DON'T HAVE AN ACCOUNT? "
                        a href="/register" class="text-highlight orange" {
                            "REGISTER HERE"
                        }
                    }
                }
            }
        }
    }
}
