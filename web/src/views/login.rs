use dioxus::prelude::*;

#[component]
pub fn Login() -> Element {
    rsx! {
        div { class: "login-page",
            div { class: "login-card",
                div { class: "login-header",
                    h1 { class: "login-title", "Authit" }
                    p { class: "login-subtitle", "Kanidm Administration" }
                }
                form {
                    action: "/auth/login",
                    method: "get",
                    button {
                        r#type: "submit",
                        class: "btn btn-primary login-btn",
                        "Sign in with Kanidm"
                    }
                }
            }
        }
    }
}
