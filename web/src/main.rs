use dioxus::prelude::*;

mod views;

use uuid::Uuid;
use views::{Dashboard, Login, Provision, Users};

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login?:error")]
    Login { error: Option<String> },
    #[route("/provision/:token")]
    Provision { token: String },
    #[layout(AuthenticatedLayout)]
        #[route("/")]
        Dashboard {},
        #[route("/users")]
        UserList {},
        #[route("/users/:user_id")]
        UserDetail { user_id: Uuid },
}

impl Route {
    pub fn users() -> Self {
        Route::UserList {}
    }

    pub fn user_detail(user_id: Uuid) -> Self {
        Route::UserDetail { user_id }
    }
}

#[component]
fn UserList() -> Element {
    rsx! { Users { user_id: None } }
}

#[component]
fn UserDetail(user_id: Uuid) -> Element {
    rsx! { Users { user_id: Some(user_id) } }
}

fn main() {
    #[cfg(feature = "server")]
    {
        server::init_tracing();
        dioxus::serve(|| async move {
            let routes = server::init().await?;

            Ok(dioxus::server::router(App).merge(routes))
        });
    }

    #[cfg(all(feature = "web", not(feature = "server")))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Title { "AuthIt!" }
        document::Link { rel: "icon", href: asset!("/assets/favicon.svg") }
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }

        Router::<Route> {}
    }
}

#[component]
fn NavLink(to: Route, children: Element) -> Element {
    let current_route: Route = use_route();
    let is_active = matches!(
        (&current_route, &to),
        (Route::Dashboard {}, Route::Dashboard {})
            | (Route::UserList {}, Route::UserList {})
            | (Route::UserDetail { .. }, Route::UserList {})
    );

    rsx! {
        Link {
            to,
            class: if is_active { "active" },
            {children}
        }
    }
}

/// Structured error information for display
#[derive(Clone, Debug, Default)]
pub struct ErrorInfo {
    pub message: String,
    pub chain: Vec<String>,
    pub backtrace: Option<String>,
}

impl ErrorInfo {
    /// Parse a ServerFnError to extract structured error info
    pub fn from_server_error(err: &ServerFnError) -> Self {
        match err {
            ServerFnError::ServerError {
                message, details, ..
            } => {
                if let Some(details) = details {
                    let chain = details
                        .get("chain")
                        .and_then(|c| c.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(|| vec![message.clone()]);
                    let backtrace = details
                        .get("backtrace")
                        .and_then(|b| b.as_str())
                        .map(String::from);
                    Self {
                        message: message.clone(),
                        chain,
                        backtrace,
                    }
                } else {
                    Self {
                        message: message.clone(),
                        chain: vec![message.clone()],
                        backtrace: None,
                    }
                }
            }
            other => Self {
                message: other.to_string(),
                chain: vec![other.to_string()],
                backtrace: None,
            },
        }
    }
}

/// Global error state - use `use_error()` to access
#[derive(Clone, Copy)]
pub struct ErrorState(Signal<Option<ErrorInfo>>);

impl ErrorState {
    pub fn set_info(&mut self, error: ErrorInfo) {
        self.0.set(Some(error));
    }

    pub fn set(&mut self, error: impl Into<String>) {
        let msg = error.into();
        self.0.set(Some(ErrorInfo {
            message: msg.clone(),
            chain: vec![msg],
            backtrace: None,
        }));
    }

    pub fn set_server_error(&mut self, err: &ServerFnError) {
        // Check for 401 (session expired) and redirect to login
        if let ServerFnError::ServerError { code: 401, message, .. } = err {
            let nav = navigator();
            nav.push(Route::Login {
                error: Some(message.clone()),
            });
            return;
        }
        self.0.set(Some(ErrorInfo::from_server_error(err)));
    }

    pub fn clear(&mut self) {
        self.0.set(None);
    }
}

/// Get the global error state for setting/clearing errors
pub fn use_error() -> ErrorState {
    use_context::<ErrorState>()
}

/// Filter backtrace to only show lines from this codebase
fn filter_backtrace(backtrace: &str) -> String {
    backtrace
        .lines()
        .filter(|line| {
            // Keep lines that reference our codebase
            line.contains("/authit/") || line.contains("authit::")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[component]
fn ErrorBanner() -> Element {
    let mut error_state = use_context::<ErrorState>();
    let error = error_state.0.read();

    if let Some(err) = error.as_ref() {
        let has_chain = err.chain.len() > 1;
        let filtered_backtrace = err.backtrace.as_ref().map(|bt| filter_backtrace(bt));
        let has_backtrace = filtered_backtrace
            .as_ref()
            .map(|bt| !bt.is_empty())
            .unwrap_or(false);

        rsx! {
            div { class: "error-banner",
                div { class: "error-banner-content",
                    div { class: "error-banner-header",
                        span { class: "error-banner-message", "{err.message}" }
                        div { class: "error-banner-actions",
                            button {
                                class: "error-banner-close",
                                onclick: move |_| error_state.clear(),
                                "×"
                            }
                        }
                    }
                    if has_chain || has_backtrace {
                        div { class: "error-details",
                            if has_chain {
                                div { class: "error-chain",
                                    h4 { class: "error-section-title", "Error Chain" }
                                    ol { class: "error-chain-list",
                                        for (i, msg) in err.chain.iter().enumerate() {
                                            li {
                                                key: "{i}",
                                                class: "error-chain-item",
                                                "{msg}"
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(backtrace) = &filtered_backtrace {
                                if has_backtrace {
                                    div { class: "error-backtrace",
                                        h4 { class: "error-section-title", "Backtrace" }
                                        pre { class: "error-backtrace-content", "{backtrace}" }
                                    }
                                }
                            }
                        }
                    }
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

    match &*user.read() {
        Some(Ok(Some(person))) => {
            let person = person.clone();
            use_context_provider(|| ErrorState(Signal::new(None)));
            let initial = person
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
                            span { class: "sidebar-logo", "AuthIt!" }
                        }
                        nav { class: "sidebar-nav",
                            NavLink { to: Route::Dashboard {}, "Dashboard" }
                            NavLink { to: Route::users(), "Users" }
                        }
                        div { class: "sidebar-footer",
                            div { class: "sidebar-user",
                                div { class: "sidebar-avatar", "{initial}" }
                                div { class: "sidebar-user-info",
                                    div { class: "sidebar-user-name", "{person.display_name}" }
                                    div { class: "sidebar-user-role", "{person.name}" }
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
            nav.push(Route::Login { error: None });
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
