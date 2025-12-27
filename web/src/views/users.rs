use crate::{Route, use_error};
use dioxus::document::eval;
use dioxus::prelude::*;
use jiff::Timestamp;
use types::{
    ResetLink,
    kanidm::{Group, Person},
};
use uuid::Uuid;

#[component]
pub fn Users(user_id: ReadSignal<Option<Uuid>>) -> Element {
    let mut users = use_signal(Vec::<Person>::new);
    let mut groups = use_signal(Vec::<Group>::new);
    let mut loading = use_signal(|| true);
    let mut error_state = use_error();
    let mut show_create_form = use_signal(|| false);
    let mut show_provision_modal = use_signal(|| false);

    // Fetch users and groups on mount
    use_effect(move || {
        spawn(async move {
            loading.set(true);

            let users_result = api::list_users().await;
            let groups_result = api::list_groups().await;

            match (users_result, groups_result) {
                (Ok(mut u), Ok(mut g)) => {
                    u.sort_unstable();
                    g.sort_unstable();
                    users.set(u);
                    groups.set(g);
                }
                (Err(e), _) | (_, Err(e)) => {
                    error_state.set_server_error(&e);
                }
            }
            loading.set(false);
        });
    });

    let selected_user = use_memo(move || {
        user_id().and_then(|id| users.read().iter().find(|u| u.uuid == id).cloned())
    });

    let refresh_users = move || {
        spawn(async move {
            if let Ok(mut u) = api::list_users().await {
                u.sort_unstable();
                users.set(u);
            }
        });
    };

    rsx! {
        div {
            div { class: "page-header",
                div { class: "page-header-content",
                    h1 { class: "page-title", "User Management" }
                    p { class: "page-subtitle", "View and manage Kanidm users and their group memberships." }
                }
                div { class: "page-header-actions",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| show_provision_modal.set(true),
                        "Generate Provision Link"
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_create_form.set(true),
                        "Create User"
                    }
                }
            }

            if *show_create_form.read() {
                CreateUserModal {
                    on_close: move |_| show_create_form.set(false),
                    on_created: move |_| {
                        show_create_form.set(false);
                        refresh_users();
                    },
                }
            }

            if *show_provision_modal.read() {
                ProvisionLinkModal {
                    on_close: move |_| show_provision_modal.set(false),
                }
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
                                            let user_id = user.uuid;
                                            let is_selected = selected_user().as_ref().map(|u| u.uuid == user_id).unwrap_or(false);
                                            rsx! {
                                                tr {
                                                    class: if is_selected { "selected" },
                                                    onclick: move |_| {
                                                        navigator().replace(Route::UserDetail { user_id });
                                                    },
                                                    td { "{user.display_name}" }
                                                    td { "{user.name}" }
                                                    td { {user.email_addresses.join(", ")} }
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
                            on_updated: move |_| refresh_users(),
                            on_deleted: move |_| {
                                refresh_users();
                                navigator().replace(Route::UserList {});
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ExpiryTime(expires_at: Timestamp) -> Element {
    let formatted = jiff::tz::TimeZone::get("America/Los_Angeles")
        .ok()
        .map(|tz| expires_at.to_zoned(tz))
        .map(|zdt| zdt.strftime("%b %d, %Y at %I:%M %p %Z").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    rsx! {
        span { class: "text-muted", "Expires: {formatted}" }
    }
}

fn is_builtin_group(name: &str) -> bool {
    name.starts_with("idm_") || name.starts_with("system_") || name.starts_with("builtin_")
}

/// Check if user is member of group
fn is_member_of(user: &Person, group: &Group) -> bool {
    // user.groups contains entries like "groupname@domain"
    // group.name is just "groupname"
    let prefix = format!("{}@", group.name);
    user.groups.iter().any(|g| g.starts_with(&prefix))
}

#[component]
fn UserDetailsCard(
    user: Person,
    groups: Vec<Group>,
    on_updated: EventHandler<()>,
    on_deleted: EventHandler<()>,
) -> Element {
    let mut error_state = use_error();
    let mut generating_reset = use_signal(|| false);
    let mut reset_link = use_signal(|| None::<ResetLink>);
    let mut updating_group = use_signal(|| None::<Uuid>);
    let mut copied = use_signal(|| false);
    let mut prev_user_id = use_signal(|| user.uuid);
    let mut show_delete_confirm = use_signal(|| false);
    let mut deleting = use_signal(|| false);

    let user_id = user.uuid;

    // Clear reset link when user changes
    if *prev_user_id.read() != user_id {
        prev_user_id.set(user_id);
        reset_link.set(None);
        copied.set(false);
        show_delete_confirm.set(false);
    }

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
                div { class: "form-group",
                    span { class: "form-label", "Email" }
                    div { class: "form-value", "{user.email_addresses.join(\", \")}" }
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
                            let group_id = group.uuid;
                            let is_updating = updating_group.read().as_ref() == Some(&group_id);

                            rsx! {
                                li { class: "group-checklist-item",
                                    label { class: "checkbox-label",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_member,
                                            disabled: is_updating,
                                            onchange: move |_| {
                                                let group_id = group_id;
                                                let user_id = user_id;
                                                let add = !is_member;
                                                spawn(async move {
                                                    updating_group.set(Some(group_id));
                                                    match api::update_user_group(user_id, group_id, add).await {
                                                        Ok(()) => on_updated.call(()),
                                                        Err(e) => error_state.set_server_error(&e),
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
                            let group_id = group.uuid;
                            let is_updating = updating_group.read().as_ref() == Some(&group_id);

                            rsx! {
                                li { class: "group-checklist-item",
                                    label { class: "checkbox-label",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_member,
                                            disabled: is_updating,
                                            onchange: move |_| {
                                                let group_id = group_id;
                                                let user_id = user_id;
                                                let add = !is_member;
                                                spawn(async move {
                                                    updating_group.set(Some(group_id));
                                                    match api::update_user_group(user_id, group_id, add).await {
                                                        Ok(()) => on_updated.call(()),
                                                        Err(e) => error_state.set_server_error(&e),
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
                    {
                        let url = link.url.clone();
                        let expires_at = link.expires_at;
                        rsx! {
                            div { class: "reset-link-container",
                                div { class: "code-block-wrapper",
                                    div { class: "code-block", "{url}" }
                                    button {
                                        class: if *copied.read() { "copy-btn copied" } else { "copy-btn" },
                                        title: if *copied.read() { "Copied!" } else { "Copy to clipboard" },
                                        onclick: {
                                            let url = url.clone();
                                            move |_| {
                                                let url = url.clone();
                                                spawn(async move {
                                                    let js = format!(
                                                        r#"navigator.clipboard.writeText("{}")"#,
                                                        url
                                                    );
                                                    if eval(&js).recv::<()>().await.is_ok() {
                                                        copied.set(true);
                                                    }
                                                });
                                            }
                                        },
                                        if *copied.read() {
                                            // Checkmark icon
                                            svg {
                                                width: "16",
                                                height: "16",
                                                view_box: "0 0 24 24",
                                                fill: "none",
                                                stroke: "currentColor",
                                                stroke_width: "2",
                                                stroke_linecap: "round",
                                                stroke_linejoin: "round",
                                                polyline { points: "20 6 9 17 4 12" }
                                            }
                                        } else {
                                            // Clipboard icon
                                            svg {
                                                width: "16",
                                                height: "16",
                                                view_box: "0 0 24 24",
                                                fill: "none",
                                                stroke: "currentColor",
                                                stroke_width: "2",
                                                stroke_linecap: "round",
                                                stroke_linejoin: "round",
                                                rect { x: "9", y: "9", width: "13", height: "13", rx: "2", ry: "2" }
                                                path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                                            }
                                        }
                                    }
                                }
                                div { class: "reset-link-expiry",
                                    ExpiryTime { expires_at }
                                }
                                button {
                                    onclick: move |_| {
                                        reset_link.set(None);
                                        copied.set(false);
                                    },
                                    class: "btn btn-link",
                                    "Clear"
                                }
                            }
                        }
                    }
                } else {
                    button {
                        onclick: {
                            move |_| {
                                spawn(async move {
                                    generating_reset.set(true);
                                    match api::generate_reset_link(user_id).await {
                                        Ok(link) => reset_link.set(Some(link)),
                                        Err(e) => error_state.set_server_error(&e),
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

                div { class: "divider" }

                h3 { class: "section-header section-header-danger", "Danger Zone" }
                button {
                    class: "btn btn-danger",
                    onclick: move |_| show_delete_confirm.set(true),
                    "Delete User"
                }
            }
        }

        if *show_delete_confirm.read() {
            DeleteConfirmModal {
                user_name: user.display_name.clone(),
                deleting: *deleting.read(),
                on_close: move |_| show_delete_confirm.set(false),
                on_confirm: {
                    move |_| {
                        let user_id = user_id;
                        spawn(async move {
                            deleting.set(true);
                            match api::delete_user(user_id).await {
                                Ok(()) => on_deleted.call(()),
                                Err(e) => error_state.set_server_error(&e),
                            }
                            deleting.set(false);
                            show_delete_confirm.set(false);
                        });
                    }
                },
            }
        }
    }
}

#[component]
fn DeleteConfirmModal(
    user_name: String,
    deleting: bool,
    on_close: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "modal-overlay",
            onclick: move |_| if !deleting { on_close.call(()) },
            div { class: "modal modal-sm",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Delete User" }
                    if !deleting {
                        button {
                            class: "modal-close",
                            onclick: move |_| on_close.call(()),
                            "×"
                        }
                    }
                }
                div { class: "modal-body",
                    p { "Are you sure you want to delete " strong { "{user_name}" } "?" }
                    p { class: "text-muted", "This action cannot be undone." }
                }
                div { class: "modal-footer",
                    button {
                        class: "btn btn-secondary",
                        disabled: deleting,
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-danger",
                        disabled: deleting,
                        onclick: move |_| on_confirm.call(()),
                        if deleting { "Deleting..." } else { "Delete" }
                    }
                }
            }
        }
    }
}

#[component]
fn CreateUserModal(on_close: EventHandler<()>, on_created: EventHandler<()>) -> Element {
    let mut error_state = use_error();
    let mut username = use_signal(String::new);
    let mut display_name = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut creating = use_signal(|| false);

    let can_submit = !username.read().is_empty() && !display_name.read().is_empty();

    rsx! {
        div { class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div { class: "modal",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Create User" }
                    button {
                        class: "modal-close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "modal-body",
                    div { class: "form-group",
                        label { class: "form-label", r#for: "username", "Username *" }
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
                        label { class: "form-label", r#for: "display_name", "Display Name *" }
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
                div { class: "modal-footer",
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: !can_submit || *creating.read(),
                        onclick: move |_| {
                            let name = username.read().clone();
                            let dname = display_name.read().clone();
                            let mail = email.read().clone();
                            spawn(async move {
                                creating.set(true);
                                match api::create_user(name, dname, mail).await {
                                    Ok(()) => on_created.call(()),
                                    Err(e) => error_state.set_server_error(&e),
                                }
                                creating.set(false);
                            });
                        },
                        if *creating.read() { "Creating..." } else { "Create" }
                    }
                }
            }
        }
    }
}

#[component]
fn ProvisionLinkModal(on_close: EventHandler<()>) -> Element {
    let mut error_state = use_error();
    let mut duration_hours = use_signal(|| 24u32);
    let mut max_uses = use_signal(|| Some(1u32));
    let mut generating = use_signal(|| false);
    let mut provision_url = use_signal(|| None::<String>);
    let mut copied = use_signal(|| false);

    rsx! {
        div { class: "modal-overlay",
            onclick: move |_| on_close.call(()),
            div { class: "modal",
                onclick: move |e| e.stop_propagation(),
                div { class: "modal-header",
                    h2 { class: "modal-title", "Generate Provision Link" }
                    button {
                        class: "modal-close",
                        onclick: move |_| on_close.call(()),
                        "×"
                    }
                }
                div { class: "modal-body",
                    if let Some(url) = provision_url.read().as_ref() {
                        {
                            let url = url.clone();
                            rsx! {
                                p { "Share this link with the user to let them create their own account:" }
                                div { class: "code-block-wrapper",
                                    div { class: "code-block", "{url}" }
                                    button {
                                        class: if *copied.read() { "copy-btn copied" } else { "copy-btn" },
                                        title: if *copied.read() { "Copied!" } else { "Copy to clipboard" },
                                        onclick: {
                                            let url = url.clone();
                                            move |_| {
                                                let url = url.clone();
                                                spawn(async move {
                                                    let js = format!(
                                                        r#"navigator.clipboard.writeText("{}")"#,
                                                        url.replace("\"", "\\\"")
                                                    );
                                                    if eval(&js).recv::<()>().await.is_ok() {
                                                        copied.set(true);
                                                    }
                                                });
                                            }
                                        },
                                        if *copied.read() {
                                            svg {
                                                width: "16",
                                                height: "16",
                                                view_box: "0 0 24 24",
                                                fill: "none",
                                                stroke: "currentColor",
                                                stroke_width: "2",
                                                stroke_linecap: "round",
                                                stroke_linejoin: "round",
                                                polyline { points: "20 6 9 17 4 12" }
                                            }
                                        } else {
                                            svg {
                                                width: "16",
                                                height: "16",
                                                view_box: "0 0 24 24",
                                                fill: "none",
                                                stroke: "currentColor",
                                                stroke_width: "2",
                                                stroke_linecap: "round",
                                                stroke_linejoin: "round",
                                                rect { x: "9", y: "9", width: "13", height: "13", rx: "2", ry: "2" }
                                                path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                                            }
                                        }
                                    }
                                }
                                p { class: "text-muted text-sm", "This link will expire based on the duration you selected." }
                            }
                        }
                    } else {
                        p { class: "text-muted", "Generate a link that allows someone to create their own account." }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "duration", "Link expires in" }
                            select {
                                id: "duration",
                                class: "form-input",
                                value: "{duration_hours}",
                                onchange: move |e| {
                                    if let Ok(v) = e.value().parse() {
                                        duration_hours.set(v);
                                    }
                                },
                                option { value: "1", "1 hour" }
                                option { value: "4", "4 hours" }
                                option { value: "24", "24 hours" }
                                option { value: "72", "3 days" }
                                option { value: "168", "7 days" }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "max_uses", "Maximum uses" }
                            select {
                                id: "max_uses",
                                class: "form-input",
                                value: "{max_uses().map(|n| n.to_string()).unwrap_or_default()}",
                                onchange: move |e| {
                                    let value = e.value();
                                    if value.is_empty() {
                                        max_uses.set(None);
                                    } else if let Ok(v) = value.parse() {
                                        max_uses.set(Some(v));
                                    }
                                },
                                option { value: "1", "1 use (single user)" }
                                option { value: "5", "5 uses" }
                                option { value: "10", "10 uses" }
                                option { value: "", "Unlimited" }
                            }
                        }
                    }
                }
                div { class: "modal-footer",
                    if provision_url.read().is_some() {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| on_close.call(()),
                            "Done"
                        }
                    } else {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary",
                            disabled: *generating.read(),
                            onclick: move |_| {
                                let hours = *duration_hours.read();
                                let uses = *max_uses.read();
                                spawn(async move {
                                    generating.set(true);
                                    match api::generate_provision_url(hours, uses).await {
                                        Ok(url) => provision_url.set(Some(url)),
                                        Err(e) => error_state.set_server_error(&e),
                                    }
                                    generating.set(false);
                                });
                            },
                            if *generating.read() { "Generating..." } else { "Generate Link" }
                        }
                    }
                }
            }
        }
    }
}
