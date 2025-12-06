use maud::{html, Markup};

pub fn register(error: Option<&str>) -> Markup {
    html! {
        div class="max-w-md mx-auto" {
            h1 class="text-4xl font-bold mb-8 text-highlight" { "Register" }

            form action="/register" method="post"
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
                        placeholder="Choose a username";
                }

                // Email field (optional)
                div {
                    label for="email" class="block mb-2 text-sm font-medium text-primary" {
                        "Email (optional)"
                    }
                    input type="email" id="email" name="email"
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="your@email.com";
                }

                // Password field
                div {
                    label for="password" class="block mb-2 text-sm font-medium text-primary" {
                        "Password"
                    }
                    input type="password" id="password" name="password" required
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Choose a strong password";
                }

                // Confirm password field
                div {
                    label for="confirm_password" class="block mb-2 text-sm font-medium text-primary" {
                        "Confirm Password"
                    }
                    input type="password" id="confirm_password" name="confirm_password" required
                        class="bg-tertiary border border-accent-muted text-primary text-sm rounded-lg focus:ring-accent focus:border-accent block w-full p-2.5"
                        placeholder="Confirm your password";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full btn-primary" {
                        "Register"
                    }
                }

                // Login link
                div class="text-center" {
                    p class="text-sm text-muted" {
                        "Already have an account? "
                        a href="/login" class="text-highlight hover:bg-accent-hover" {
                            "Login here"
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
                    alert('Passwords do not match!');
                    return false;
                }
            });
            "#))
        }
    }
}
