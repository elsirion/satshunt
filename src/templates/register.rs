use maud::{html, Markup};

pub fn register(error: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto" {
            h1 class="text-4xl font-black mb-8 text-primary" style="letter-spacing: -0.02em;" { "REGISTER" }

            form action="/register" method="post"
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
                        placeholder="CHOOSE USERNAME";
                }

                // Email field (optional)
                div {
                    label for="email" class="label-brutal" {
                        "EMAIL (OPTIONAL)"
                    }
                    input type="email" id="email" name="email"
                        class="input-brutal-box w-full"
                        placeholder="YOUR@EMAIL.COM";
                }

                // Password field
                div {
                    label for="password" class="label-brutal" {
                        "PASSWORD"
                    }
                    input type="password" id="password" name="password" required
                        class="input-brutal-box w-full"
                        placeholder="CHOOSE STRONG PASSWORD";
                }

                // Confirm password field
                div {
                    label for="confirm_password" class="label-brutal" {
                        "CONFIRM PASSWORD"
                    }
                    input type="password" id="confirm_password" name="confirm_password" required
                        class="input-brutal-box w-full"
                        placeholder="CONFIRM PASSWORD";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full btn-brutal-fill" {
                        "REGISTER"
                    }
                }

                // Login link
                div class="text-center" {
                    p class="text-sm text-muted font-bold" {
                        "ALREADY HAVE AN ACCOUNT? "
                        a href="/login" class="text-highlight orange" {
                            "LOGIN HERE"
                        }
                    }
                }
            }
        }

        script {
            (maud::PreEscaped(r#"
            document.querySelector('form').addEventListener('submit', function(e) {
                const password = document.getElementById('password').value;
                const confirm = document.getElementById('confirm_password').value;

                if (password !== confirm) {
                    e.preventDefault();
                    alert('PASSWORDS DO NOT MATCH!');
                    return false;
                }
            });
            "#))
        }
    }
}
