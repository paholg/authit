use dioxus::prelude::*;

mod views;

#[cfg(feature = "server")]
mod auth_routes;

use views::{Dashboard, Login, Users};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    Login {},
    #[layout(AuthenticatedLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/users")]
        UserList {},
        #[route("/users/:user_id")]
        UserDetail { user_id: String },
}

impl Route {
    pub fn users() -> Self {
        Route::UserList {}
    }

    pub fn user_detail(user_id: String) -> Self {
        Route::UserDetail { user_id }
    }
}

#[component]
fn UserList() -> Element {
    rsx! { Users { user_id: None } }
}

#[component]
fn UserDetail(user_id: String) -> Element {
    rsx! { Users { user_id: Some(user_id) } }
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "server")]
    {
        use auth_routes::{auth_router, AuthState};
        use server::Config;

        dioxus::serve(|| async move {
            let config = Config::from_env().expect("Failed to load configuration");
            let auth_state =
                AuthState::new(config).expect("Failed to create auth state");

            let auth_routes = auth_router(auth_state);

            Ok(dioxus::server::router(App).merge(auth_routes))
        });
    }

    #[cfg(all(feature = "web", not(feature = "server")))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// Global error state - use `use_error()` to access
#[derive(Clone, Copy)]
pub struct ErrorState(Signal<Option<String>>);

impl ErrorState {
    pub fn set(&mut self, error: impl Into<String>) {
        self.0.set(Some(error.into()));
    }

    pub fn clear(&mut self) {
        self.0.set(None);
    }
}

/// Get the global error state for setting/clearing errors
pub fn use_error() -> ErrorState {
    use_context::<ErrorState>()
}

#[component]
fn ErrorBanner() -> Element {
    let mut error_state = use_context::<ErrorState>();
    let error = error_state.0.read();

    if let Some(err) = error.as_ref() {
        rsx! {
            div { class: "error-banner",
                span { "{err}" }
                button {
                    class: "error-banner-close",
                    onclick: move |_| error_state.clear(),
                    "×"
                }
            }
        }
    } else {
        rsx! {}
    }
}

#[component]
fn AuthenticatedLayout() -> Element {
    let user = use_server_future(api::get_current_user)?;

    match user() {
        Some(Ok(Some(session))) => {
            let session_clone = session.clone();
            use_context_provider(|| session);
            use_context_provider(|| ErrorState(Signal::new(None)));
            let initial = session_clone
                .display_name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();

            rsx! {
                div { class: "app-layout",
                    // Sidebar
                    aside { class: "sidebar",
                        div { class: "sidebar-header",
                            span { class: "sidebar-logo", "Authit" }
                        }
                        nav { class: "sidebar-nav",
                            Link { to: Route::Dashboard {}, "Dashboard" }
                            Link { to: Route::users(), "Users" }
                        }
                        div { class: "sidebar-footer",
                            div { class: "sidebar-user",
                                div { class: "sidebar-avatar", "{initial}" }
                                div { class: "sidebar-user-info",
                                    div { class: "sidebar-user-name", "{session_clone.display_name}" }
                                    div { class: "sidebar-user-role", "{session_clone.username}" }
                                }
                            }
                            a { href: "/auth/logout", rel: "external", class: "sidebar-logout", "Sign out" }
                        }
                    }
                    // Main content
                    main { class: "main-content",
                        ErrorBanner {}
                        Outlet::<Route> {}
                    }
                }
            }
        }
        Some(Ok(None)) | Some(Err(_)) => {
            let nav = navigator();
            nav.push(Route::Login {});
            rsx! {
                div { class: "loading", "Redirecting to login..." }
            }
        }
        None => {
            rsx! {
                div { class: "loading", "Loading..." }
            }
        }
    }
}
