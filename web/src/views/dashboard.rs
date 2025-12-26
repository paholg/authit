use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Dashboard" }
                p { class: "page-subtitle", "Welcome to Authit - your Kanidm administration interface." }
            }
            div { class: "dashboard-grid",
                Link {
                    to: Route::users(),
                    class: "dashboard-card",
                    h3 { class: "dashboard-card-title", "Manage Users" }
                    p { class: "dashboard-card-desc",
                        "View users, manage group memberships, and generate credential reset links."
                    }
                }
            }
        }
    }
}
