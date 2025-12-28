use dioxus::prelude::*;

#[component]
pub fn UserForm(
    username: Signal<String>,
    display_name: Signal<String>,
    email: Signal<String>,
) -> Element {
    rsx! {
        div { class: "form-group",
            label { class: "form-label", r#for: "username", "Username" }
            input {
                id: "username",
                class: "form-input",
                r#type: "text",
                placeholder: "e.g. jsmith",
                value: "{username}",
                oninput: move |e| username.set(e.value()),
            }
        }
        div { class: "form-group",
            label { class: "form-label", r#for: "display_name", "Display Name" }
            input {
                id: "display_name",
                class: "form-input",
                r#type: "text",
                placeholder: "e.g. John Smith",
                value: "{display_name}",
                oninput: move |e| display_name.set(e.value()),
            }
        }
        div { class: "form-group",
            label { class: "form-label", r#for: "email", "Email" }
            input {
                id: "email",
                class: "form-input",
                r#type: "email",
                placeholder: "e.g. jsmith@example.com",
                value: "{email}",
                oninput: move |e| email.set(e.value()),
            }
        }
    }
}
