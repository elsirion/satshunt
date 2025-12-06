use maud::{html, Markup};

pub fn register(error: Option<&str>) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-yellow-400" { "Register" }

        div class="max-w-md mx-auto" {
            form action="/register" method="post"
                class="bg-slate-800 rounded-lg p-8 border border-slate-700 space-y-6" {

                @if let Some(error_msg) = error {
                    div class="bg-red-900 border border-red-700 text-red-200 px-4 py-3 rounded-lg" {
                        (error_msg)
                    }
                }

                // Username field
                div {
                    label for="username" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Username"
                    }
                    input type="text" id="username" name="username" required autofocus
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Choose a username";
                }

                // Email field (optional)
                div {
                    label for="email" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Email (optional)"
                    }
                    input type="email" id="email" name="email"
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="your@email.com";
                }

                // Password field
                div {
                    label for="password" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Password"
                    }
                    input type="password" id="password" name="password" required
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Choose a strong password";
                }

                // Confirm password field
                div {
                    label for="confirm_password" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Confirm Password"
                    }
                    input type="password" id="confirm_password" name="confirm_password" required
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Confirm your password";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full px-6 py-3 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg transition" {
                        "Register"
                    }
                }

                // Login link
                div class="text-center" {
                    p class="text-sm text-slate-400" {
                        "Already have an account? "
                        a href="/login" class="text-yellow-400 hover:text-yellow-300" {
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
