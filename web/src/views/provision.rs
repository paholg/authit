use super::components::UserForm;
use dioxus::prelude::*;
use types::ResetLink;

#[component]
pub fn Provision(token: String) -> Element {
    let username = use_signal(String::new);
    let display_name = use_signal(String::new);
    let email = use_signal(String::new);
    let mut submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut reset_link = use_signal(|| None::<ResetLink>);

    // Verify token on mount
    let token_for_verify = token.clone();
    let token_valid = use_resource(move || {
        let token = token_for_verify.clone();
        async move { api::verify_provision(token).await }
    });

    let can_submit = !username.read().is_empty() && !display_name.read().is_empty();

    // If we have a reset link, redirect to it
    if let Some(link) = reset_link.read().as_ref() {
        let url = link.url.clone();
        return rsx! {
            div { class: "provision-page",
                div { class: "provision-card",
                    div { class: "provision-header",
                        h1 { class: "provision-title", "Account Created!" }
                    }
                    div { class: "provision-body",
                        p { "Your account has been created. Click the button below to set up your credentials." }
                        a {
                            href: "{url}",
                            class: "btn btn-primary btn-lg",
                            "Set Up Credentials"
                        }
                    }
                }
            }
        };
    }

    match &*token_valid.read() {
        Some(Ok(_)) => {
            rsx! {
                div { class: "provision-page",
                    div { class: "provision-card",
                        div { class: "provision-header",
                            h1 { class: "provision-title", "Create Your Account" }
                            p { class: "provision-subtitle", "Enter your information to create your account." }
                        }
                        div { class: "provision-body",
                            if let Some(err) = error.read().as_ref() {
                                div { class: "alert alert-error", "{err}" }
                            }

                            UserForm { username, display_name, email }
                        }
                        div { class: "provision-footer",
                            button {
                                class: "btn btn-primary btn-lg",
                                disabled: !can_submit || *submitting.read(),
                                onclick: {
                                    let token = token.clone();
                                    move |_| {
                                        let token = token.clone();
                                        let name = username.read().clone();
                                        let dname = display_name.read().clone();
                                        let email_address = email.read().clone();
                                        spawn(async move {
                                            submitting.set(true);
                                            error.set(None);
                                            match api::complete_provision(token, name, dname, email_address).await {
                                                Ok(link) => reset_link.set(Some(link)),
                                                Err(e) => error.set(Some(e.to_string())),
                                            }
                                            submitting.set(false);
                                        });
                                    }
                                },
                                if *submitting.read() { "Creating Account..." } else { "Create Account" }
                            }
                        }
                    }
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                div { class: "provision-page",
                    div { class: "provision-card",
                        div { class: "provision-header",
                            h1 { class: "provision-title", "Invalid Link" }
                        }
                        div { class: "provision-body",
                            div { class: "alert alert-error", "{e}" }
                            p { "This provision link is invalid or has expired. Please contact your administrator for a new link." }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "provision-page",
                    div { class: "provision-card",
                        div { class: "provision-body",
                            div { class: "loading", "Verifying link..." }
                        }
                    }
                }
            }
        }
    }
}
