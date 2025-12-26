use api::{Group, Person};
use crate::{use_error, Route};
use dioxus::prelude::*;

#[component]
pub fn Users(user_id: ReadOnlySignal<Option<String>>) -> Element {
    let mut users = use_signal(Vec::<Person>::new);
    let mut groups = use_signal(Vec::<Group>::new);
    let mut loading = use_signal(|| true);
    let mut error_state = use_error();

    // Fetch users and groups on mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);

            let users_result = api::list_users().await;
            let groups_result = api::list_groups().await;

            match (users_result, groups_result) {
                (Ok(mut u), Ok(mut g)) => {
                    u.sort_by(|a, b| {
                        let a_email = a.mail.as_deref().unwrap_or("");
                        let b_email = b.mail.as_deref().unwrap_or("");
                        a_email.to_lowercase().cmp(&b_email.to_lowercase())
                    });
                    g.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    users.set(u);
                    groups.set(g);
                }
                (Err(e), _) | (_, Err(e)) => {
                    error_state.set(e.to_string());
                }
            }
            loading.set(false);
        });
    });

    let refresh_user = move || {
        spawn(async move {
            if let Ok(mut u) = api::list_users().await {
                u.sort_by(|a, b| {
                    let a_email = a.mail.as_deref().unwrap_or("");
                    let b_email = b.mail.as_deref().unwrap_or("");
                    a_email.to_lowercase().cmp(&b_email.to_lowercase())
                });
                users.set(u);
            }
        });
    };

    let selected_user = use_memo(move || {
        user_id().and_then(|id| users.read().iter().find(|u| u.uuid == id).cloned())
    });

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "User Management" }
                p { class: "page-subtitle", "View and manage Kanidm users and their group memberships." }
            }

            if *loading.read() {
                div { class: "loading", "Loading users..." }
            } else {
                div { class: "grid grid-cols-3",
                    div { class: "card",
                        div { class: "card-header",
                            h2 { class: "card-title", "Users" }
                        }
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Username" }
                                        th { "Email" }
                                    }
                                }
                                tbody {
                                    for user in users.read().iter() {
                                        {
                                            let user_id = user.uuid.clone();
                                            let is_selected = selected_user().as_ref().map(|u| u.uuid == user_id).unwrap_or(false);
                                            rsx! {
                                                tr {
                                                    class: if is_selected { "selected" },
                                                    onclick: move |_| {
                                                        navigator().replace(Route::UserDetail { user_id: user_id.clone() });
                                                    },
                                                    td { "{user.display_name}" }
                                                    td { "{user.name}" }
                                                    td { {user.mail.as_deref().unwrap_or("-")} }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(u) = selected_user() {
                        UserDetailsCard {
                            user: u.clone(),
                            groups: groups.read().clone(),
                            on_updated: move |_| refresh_user(),
                        }
                    }
                }
            }
        }
    }
}

fn is_builtin_group(name: &str) -> bool {
    name.starts_with("idm_")
        || name.starts_with("system_")
        || name.starts_with("builtin_")
}

/// Check if user is member of group
fn is_member_of(user: &Person, group: &Group) -> bool {
    // user.groups contains entries like "groupname@domain"
    // group.name is just "groupname"
    let prefix = format!("{}@", group.name);
    user.groups.iter().any(|g| g.starts_with(&prefix))
}

#[component]
fn UserDetailsCard(user: Person, groups: Vec<Group>, on_updated: EventHandler<()>) -> Element {
    let mut error_state = use_error();
    let mut generating_reset = use_signal(|| false);
    let mut reset_link = use_signal(|| None::<String>);
    let mut updating_group = use_signal(|| None::<String>);

    let user_id = user.uuid.clone();

    // Separate groups into custom and built-in (already sorted from parent)
    let custom_groups: Vec<_> = groups
        .iter()
        .filter(|g| !is_builtin_group(&g.name))
        .collect();
    let builtin_groups: Vec<_> = groups
        .iter()
        .filter(|g| is_builtin_group(&g.name))
        .collect();

    rsx! {
        div { class: "card",
            div { class: "card-header",
                h2 { class: "card-title", "User Details" }
            }
            div { class: "card-body",
                div { class: "form-group",
                    span { class: "form-label", "Username" }
                    div { class: "form-value", "{user.name}" }
                }
                if let Some(mail) = &user.mail {
                    div { class: "form-group",
                        span { class: "form-label", "Email" }
                        div { class: "form-value", "{mail}" }
                    }
                }
                div { class: "form-group",
                    span { class: "form-label", "UUID" }
                    div { class: "form-value form-value-mono", "{user.uuid}" }
                }

                div { class: "divider" }

                h3 { class: "section-header", "Custom Groups" }
                ul { class: "group-checklist",
                    for group in &custom_groups {
                        {
                            let is_member = is_member_of(&user, group);
                            let group_name = group.name.clone();
                            let group_id = group.name.clone();
                            let user_id = user_id.clone();
                            let is_updating = updating_group.read().as_ref() == Some(&group_id);

                            rsx! {
                                li { class: "group-checklist-item",
                                    label { class: "checkbox-label",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_member,
                                            disabled: is_updating,
                                            onchange: move |_| {
                                                let group_id = group_id.clone();
                                                let user_id = user_id.clone();
                                                let add = !is_member;
                                                spawn(async move {
                                                    updating_group.set(Some(group_id.clone()));
                                                    match api::update_user_group(user_id, group_id, add).await {
                                                        Ok(()) => on_updated.call(()),
                                                        Err(e) => error_state.set(format!("Failed to update group: {}", e)),
                                                    }
                                                    updating_group.set(None);
                                                });
                                            }
                                        }
                                        span { "{group_name}" }
                                        if is_updating {
                                            span { class: "checkbox-updating", "(updating...)" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if custom_groups.is_empty() {
                    p { class: "text-muted", "No custom groups" }
                }

                div { class: "divider" }

                h3 { class: "section-header", "Built-in Groups" }
                ul { class: "group-checklist",
                    for group in &builtin_groups {
                        {
                            let is_member = is_member_of(&user, group);
                            let group_name = group.name.clone();
                            let group_id = group.name.clone();
                            let user_id = user_id.clone();
                            let is_updating = updating_group.read().as_ref() == Some(&group_id);

                            rsx! {
                                li { class: "group-checklist-item",
                                    label { class: "checkbox-label",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_member,
                                            disabled: is_updating,
                                            onchange: move |_| {
                                                let group_id = group_id.clone();
                                                let user_id = user_id.clone();
                                                let add = !is_member;
                                                spawn(async move {
                                                    updating_group.set(Some(group_id.clone()));
                                                    match api::update_user_group(user_id, group_id, add).await {
                                                        Ok(()) => on_updated.call(()),
                                                        Err(e) => error_state.set(format!("Failed to update group: {}", e)),
                                                    }
                                                    updating_group.set(None);
                                                });
                                            }
                                        }
                                        span { "{group_name}" }
                                        if is_updating {
                                            span { class: "checkbox-updating", "(updating...)" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "divider" }

                h3 { class: "section-header", "Credential Reset" }
                if let Some(link) = reset_link.read().as_ref() {
                    div {
                        div { class: "code-block", "{link}" }
                        button {
                            onclick: move |_| reset_link.set(None),
                            class: "btn btn-link",
                            style: "margin-top: 0.5rem;",
                            "Clear"
                        }
                    }
                } else {
                    button {
                        onclick: {
                            let user_id = user_id.clone();
                            move |_| {
                                let user_id = user_id.clone();
                                spawn(async move {
                                    generating_reset.set(true);
                                    match api::generate_reset_link(user_id).await {
                                        Ok(link) => reset_link.set(Some(link)),
                                        Err(e) => error_state.set(format!("Failed to generate reset link: {}", e)),
                                    }
                                    generating_reset.set(false);
                                });
                            }
                        },
                        disabled: *generating_reset.read(),
                        class: "btn btn-primary",
                        if *generating_reset.read() {
                            "Generating..."
                        } else {
                            "Generate Reset Link"
                        }
                    }
                }
            }
        }
    }
}
