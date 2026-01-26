use crate::models::{User, UserRole};
use maud::{html, Markup};

pub fn admin_users(users: &[User]) -> Markup {
    let registered_users: Vec<_> = users.iter().filter(|u| !u.is_anonymous()).collect();
    let anon_users: Vec<_> = users.iter().filter(|u| u.is_anonymous()).collect();
    let registered_count = registered_users.len();
    let anon_count = anon_users.len();
    let total_count = users.len();

    html! {
        div class="mb-8" {
            div class="flex justify-between items-center mb-8" {
                h1 class="text-4xl font-black text-primary" style="letter-spacing: -0.02em;" {
                    "USER MANAGEMENT"
                }
            }

            // Filter buttons
            div class="flex flex-wrap gap-2 mb-6" {
                button type="button"
                    class="btn-brutal-fill"
                    id="filter-registered"
                    onclick="filterUsers('registered')" {
                    "REGISTERED "
                    span class="mono" { "[" (registered_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-anon"
                    onclick="filterUsers('anon')" {
                    "ANONYMOUS "
                    span class="mono" { "[" (anon_count) "]" }
                }
                button type="button"
                    class="btn-brutal"
                    id="filter-all"
                    onclick="filterUsers('all')" {
                    "ALL "
                    span class="mono" { "[" (total_count) "]" }
                }
            }

            @if users.is_empty() {
                div class="card-brutal-inset text-center" style="padding: 3rem;" {
                    div class="text-6xl mb-6 text-muted" {
                        i class="fa-solid fa-users" {}
                    }
                    h3 class="text-2xl font-black text-primary mb-3" { "NO USERS" }
                    p class="text-secondary mb-8 font-bold" {
                        "NO USERS FOUND IN THE SYSTEM."
                    }
                }
            } @else {
                div class="space-y-4" id="users-list" {
                    @for user in users {
                        (user_card(user))
                    }
                }
            }

            // Filter script
            script {
                (maud::PreEscaped(r#"
                function filterUsers(filter) {
                    const cards = document.querySelectorAll('[data-user-type]');
                    cards.forEach(card => {
                        const type = card.getAttribute('data-user-type');
                        if (filter === 'all' || type === filter) {
                            card.style.display = '';
                        } else {
                            card.style.display = 'none';
                        }
                    });

                    // Update button styles
                    const buttons = ['filter-registered', 'filter-anon', 'filter-all'];
                    buttons.forEach(id => {
                        const btn = document.getElementById(id);
                        if (id === 'filter-' + filter) {
                            btn.className = 'btn-brutal-fill';
                        } else {
                            btn.className = 'btn-brutal';
                        }
                    });
                }

                // Initialize with registered filter
                document.addEventListener('DOMContentLoaded', function() {
                    filterUsers('registered');
                });
                "#))
            }
        }
    }
}

fn user_card(user: &User) -> Markup {
    let role_badge_class = match user.role {
        UserRole::Admin => "badge-brutal orange",
        UserRole::Creator => "badge-brutal filled",
        UserRole::User => "badge-brutal",
    };
    let user_type = if user.is_anonymous() {
        "anon"
    } else {
        "registered"
    };

    html! {
        div class="card-brutal" data-user-type=(user_type) {
            div class="flex flex-col gap-4" {
                // Header with name and role
                div class="flex justify-between items-start gap-4" {
                    div class="flex-1" {
                        h3 class="text-xl font-black text-primary mb-2" {
                            @if let Some(username) = &user.username {
                                (username)
                            } @else {
                                span class="text-muted" { "anon_" (&user.id[..8]) }
                            }
                        }
                        div class="flex items-center gap-4 text-sm text-muted font-bold mono" {
                            span {
                                i class="fa-solid fa-fingerprint mr-1" {}
                                (&user.id[..8]) "..."
                            }
                            @if let Some(email) = &user.email {
                                span {
                                    i class="fa-solid fa-envelope mr-1" {}
                                    (email)
                                }
                            }
                            span {
                                i class="fa-solid fa-calendar mr-1" {}
                                (user.created_at.format("%Y-%m-%d").to_string())
                            }
                        }
                    }
                    span class=(role_badge_class) { (user.role.as_str().to_uppercase()) }
                }

                // Role selection
                div class="pt-4" style="border-top: 3px solid var(--accent-muted);" {
                    form class="flex items-center gap-4"
                        hx-post={"/api/admin/users/" (&user.id) "/role"}
                        hx-swap="none"
                        hx-on--after-request="if(event.detail.successful) window.location.reload()" {
                        label class="label-brutal" for={"role-" (&user.id)} { "ROLE" }
                        select name="role" id={"role-" (&user.id)}
                            class="flex-1 px-3 py-2 bg-tertiary text-primary font-bold mono"
                            style="border: 3px solid var(--accent-muted);" {
                            option value="user" selected[user.role == UserRole::User] { "User" }
                            option value="creator" selected[user.role == UserRole::Creator] { "Creator" }
                            option value="admin" selected[user.role == UserRole::Admin] { "Admin" }
                        }
                        button type="submit" class="btn-brutal" {
                            i class="fa-solid fa-save mr-2" {}
                            "SAVE"
                        }
                    }
                }
            }
        }
    }
}
