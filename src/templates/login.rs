use maud::{html, Markup};

pub fn login(error: Option<&str>) -> Markup {
    html! {
        h1 class="text-4xl font-bold mb-8 text-yellow-400" { "Login" }

        div class="max-w-md mx-auto" {
            form action="/login" method="post"
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
                        placeholder="Enter your username";
                }

                // Password field
                div {
                    label for="password" class="block mb-2 text-sm font-medium text-slate-200" {
                        "Password"
                    }
                    input type="password" id="password" name="password" required
                        class="bg-slate-700 border border-slate-600 text-slate-200 text-sm rounded-lg focus:ring-yellow-500 focus:border-yellow-500 block w-full p-2.5"
                        placeholder="Enter your password";
                }

                // Submit button
                div {
                    button type="submit"
                        class="w-full px-6 py-3 bg-yellow-500 hover:bg-yellow-600 text-slate-900 font-semibold rounded-lg transition" {
                        "Login"
                    }
                }

                // Register link
                div class="text-center" {
                    p class="text-sm text-slate-400" {
                        "Don't have an account? "
                        a href="/register" class="text-yellow-400 hover:text-yellow-300" {
                            "Register here"
                        }
                    }
                }
            }
        }
    }
}
