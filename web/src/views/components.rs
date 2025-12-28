use std::collections::HashSet;

use dioxus::prelude::*;
use types::kanidm::Group;
use uuid::Uuid;

/// A reusable component that renders a list of groups with checkboxes.
#[component]
pub fn GroupCheckboxList(
    groups: Vec<Group>,
    selected: HashSet<Uuid>,
    on_toggle: EventHandler<Uuid>,
    #[props(default)] updating: Option<Uuid>,
) -> Element {
    rsx! {
        ul { class: "group-checklist",
            for group in groups {
                {
                    let is_checked = selected.contains(&group.uuid);
                    let group_id = group.uuid;
                    let is_updating = updating == Some(group_id);

                    rsx! {
                        li { class: "group-checklist-item",
                            label { class: "checkbox-label",
                                input {
                                    r#type: "checkbox",
                                    checked: is_checked,
                                    disabled: is_updating,
                                    onchange: move |_| on_toggle.call(group_id),
                                }
                                span { "{group.name}" }
                                if is_updating {
                                    span { class: "checkbox-updating", "(updating...)" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

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
