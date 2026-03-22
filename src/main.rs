use dioxus::prelude::*;
mod storage;
mod client;

use storage::{FileNode, HttpRequest as RequestData};
use client::{HttpResponse, execute_request};
use std::path::PathBuf;

fn main() {
    dioxus::launch(app);
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Headers,
    Body,
}

fn app() -> Element {
    let mut tree = use_signal(|| storage::scan_directory());
    let mut current_request = use_signal(RequestData::new);
    let mut current_path = use_signal(|| None::<PathBuf>);
    let mut response = use_signal(|| None::<Result<HttpResponse, String>>);
    let mut active_tab = use_signal(|| Tab::Headers);
    let mut loading = use_signal(|| false);
    let mut save_status = use_signal(|| None::<Result<(), String>>);
    let mut creating_new = use_signal(|| None::<PathBuf>);
    let mut error_message = use_signal(|| None::<String>);
    let mut content_type = use_signal(|| String::new());

    let on_refresh_tree = move |_| {
        tree.set(storage::scan_directory());
    };

    let on_select_file = move |path: PathBuf| {
        println!("Loading request from: {:?}", path);
        match storage::load_request(&path) {
            Ok(req) => {
                println!("Successfully loaded request from {:?}", path);
                // Extract Content-Type from headers if present
                let ct = req.headers.iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                content_type.set(ct);
                current_request.set(req);
                current_path.set(Some(path));
                save_status.set(None);
            },
            Err(e) => {
                eprintln!("Failed to load request from {:?}: {}", path, e);
            }
        }
    };

    let on_send = move |_| {
        spawn(async move {
            loading.set(true);
            let req = current_request.read().clone();
            let res = execute_request(&req).await;
            response.set(Some(res));
            loading.set(false);
        });
    };

    let on_save = move |_| {
        if let Some(path) = current_path.read().as_ref() {
            println!("Saving request to: {:?}", path);
            match storage::save_request(path, &current_request.read()) {
                Ok(_) => {
                    println!("Successfully saved request to {:?}", path);
                    save_status.set(Some(Ok(())));
                },
                Err(e) => {
                    eprintln!("Failed to save request: {}", e);
                    save_status.set(Some(Err(e.to_string())));
                },
            }
        } else {
            eprintln!("Cannot save: no file selected");
            save_status.set(Some(Err("No file selected".to_string())));
        }
    };

    let on_new_request = move |parent_path: Option<PathBuf>| {
        let target_path = parent_path.unwrap_or_else(|| storage::get_base_dir());
        println!("Creating new request in: {:?}", target_path);
        creating_new.set(Some(target_path));
    };

    let on_create_file = move |(parent_path, filename): (PathBuf, String)| {
        let file_path = parent_path.join(format!("{}.req", filename));
        println!("Attempting to create file: {:?}", file_path);

        if file_path.exists() {
            eprintln!("File already exists: {:?}", file_path);
            error_message.set(Some(format!("File '{}' already exists", filename)));
            return;
        }

        match storage::save_request(&file_path, &RequestData::new()) {
            Ok(_) => {
                println!("Successfully created new request: {:?}", file_path);
                creating_new.set(None);
                error_message.set(None);
                tree.set(storage::scan_directory());
                current_request.set(RequestData::new());
                current_path.set(Some(file_path));
                save_status.set(None);
            },
            Err(e) => {
                eprintln!("Failed to create file: {}", e);
                error_message.set(Some(format!("Failed to create file: {}", e)));
            }
        }
    };

    let on_cancel_new = move |_| {
        creating_new.set(None);
    };

    let mut on_content_type_change = move |new_type: String| {
        content_type.set(new_type.clone());

        // Update Content-Type header
        let mut headers = current_request.read().headers.clone();

        // Remove existing Content-Type header
        headers.retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));

        // Add new Content-Type if not empty
        if !new_type.is_empty() {
            headers.push(("Content-Type".to_string(), new_type));
        }

        current_request.write().headers = headers;
    };

    rsx! {
        style { {include_str!("style.css")} }
        div { id: "main",
            div { class: "sidebar",
                h3 { "Requests" }
                button { onclick: on_refresh_tree, "Refresh" }
                Sidebar {
                    node: tree.read().clone(),
                    on_select: on_select_file,
                    current_path: current_path.read().clone(),
                    on_new_request: on_new_request,
                    creating_new: creating_new.read().clone(),
                    on_create_file: on_create_file,
                    on_cancel_new: on_cancel_new
                }
                if let Some(err) = error_message.read().as_ref() {
                    div {
                        class: "error-dialog",
                        style: "position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: #1e1e1e; border: 2px solid #f44747; padding: 20px; border-radius: 5px; z-index: 1000;",
                        div { style: "color: #f44747; margin-bottom: 10px;", "{err}" }
                        button {
                            onclick: move |_| error_message.set(None),
                            "OK"
                        }
                    }
                }
                if error_message.read().is_some() {
                    div {
                        class: "overlay",
                        style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); z-index: 999;",
                        onclick: move |_| error_message.set(None)
                    }
                }
            }
            div { class: "content",
                div { class: "address-bar",
                    select {
                        value: "{current_request.read().method}",
                        onchange: move |evt| {
                            current_request.write().method = evt.value();
                        },
                        option { value: "GET", "GET" }
                        option { value: "POST", "POST" }
                        option { value: "PUT", "PUT" }
                        option { value: "DELETE", "DELETE" }
                        option { value: "PATCH", "PATCH" }
                    }
                    input {
                        r#type: "text",
                        placeholder: "https://api.example.com",
                        value: "{current_request.read().url}",
                        oninput: move |evt| {
                            current_request.write().url = evt.value();
                        }
                    }
                    button {
                        disabled: loading(),
                        onclick: on_send,
                        if loading() { "Sending..." } else { "Send" }
                    }
                    button { onclick: on_save, "Save" }
                    if let Some(status) = save_status.read().as_ref() {
                        match status {
                            Ok(_) => rsx! { span { style: "color: #4ec9b0; margin-left: 10px;", "✓ Saved" } },
                            Err(e) => rsx! { span { style: "color: #f44747; margin-left: 10px;", "✗ {e}" } },
                        }
                    }
                }

                div { class: "tabs",
                    div { 
                        class: if active_tab() == Tab::Headers { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set(Tab::Headers),
                        "Headers"
                    }
                    div { 
                        class: if active_tab() == Tab::Body { "tab active" } else { "tab" },
                        onclick: move |_| active_tab.set(Tab::Body),
                        "Body"
                    }
                }

                div { class: "tab-content",
                    match active_tab() {
                        Tab::Headers => rsx! {
                            HeadersEditor { 
                                headers: current_request.read().headers.clone(),
                                on_change: move |new_headers| {
                                    current_request.write().headers = new_headers;
                                }
                            }
                        },
                        Tab::Body => rsx! {
                            div { style: "display: flex; flex-direction: column; height: 100%;",
                                div { style: "margin-bottom: 10px; display: flex; align-items: center;",
                                    label { style: "margin-right: 10px; color: #cccccc;", "Content-Type:" }
                                    select {
                                        value: "{content_type()}",
                                        onchange: move |evt| {
                                            on_content_type_change(evt.value());
                                        },
                                        style: "padding: 5px; background: #3c3c3c; color: #cccccc; border: 1px solid #3e3e3e; border-radius: 3px;",
                                        option { value: "", "None" }
                                        option { value: "application/json", "application/json" }
                                        option { value: "application/xml", "application/xml" }
                                        option { value: "application/x-www-form-urlencoded", "application/x-www-form-urlencoded" }
                                        option { value: "text/plain", "text/plain" }
                                        option { value: "text/html", "text/html" }
                                        option { value: "multipart/form-data", "multipart/form-data" }
                                    }
                                }
                                textarea {
                                    class: "body-editor",
                                    style: "flex: 1;",
                                    value: "{current_request.read().body}",
                                    oninput: move |evt| {
                                        current_request.write().body = evt.value();
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "result-area",
                    match response.read().as_ref() {
                        Some(Ok(res)) => rsx! {
                            div { class: "result-header", "Status: {res.status} {res.status_text}" }
                            pre { class: "result-body", "{res.body}" }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "result-header", style: "color: #f44747", "Error" }
                            pre { class: "result-body", "{e}" }
                        },
                        None => rsx! {
                            div { class: "result-header", "No response yet" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Sidebar(
    node: FileNode,
    on_select: EventHandler<PathBuf>,
    current_path: Option<PathBuf>,
    on_new_request: EventHandler<Option<PathBuf>>,
    creating_new: Option<PathBuf>,
    on_create_file: EventHandler<(PathBuf, String)>,
    on_cancel_new: EventHandler<()>
) -> Element {
    let mut show_context_menu = use_signal(|| false);
    let mut context_menu_pos = use_signal(|| (0.0, 0.0));

    match node {
        FileNode::Folder { name, children, path } => {
            let path_for_menu = path.clone();
            let path_for_new = path.clone();
            let is_creating_here = creating_new.as_ref().map_or(false, |p| p == &path);

            rsx! {
                div { class: "tree-node",
                    div {
                        class: "folder-node",
                        oncontextmenu: move |evt| {
                            evt.prevent_default();
                            context_menu_pos.set((evt.client_coordinates().x, evt.client_coordinates().y));
                            show_context_menu.set(true);
                        },
                        "📁 {name}"
                    }
                    div { style: "margin-left: 10px",
                        if is_creating_here {
                            NewFileEditor {
                                parent_path: path_for_new.clone(),
                                on_create: on_create_file,
                                on_cancel: on_cancel_new
                            }
                        }
                        for child in children {
                            Sidebar {
                                node: child.clone(),
                                on_select: move |p| on_select.call(p),
                                current_path: current_path.clone(),
                                on_new_request: move |p| on_new_request.call(p),
                                creating_new: creating_new.clone(),
                                on_create_file: move |(p, n)| on_create_file.call((p, n)),
                                on_cancel_new: move |()| on_cancel_new.call(())
                            }
                        }
                    }
                    if show_context_menu() {
                        div {
                            class: "context-menu",
                            style: "position: fixed; left: {context_menu_pos().0}px; top: {context_menu_pos().1}px; background: #2d2d2d; border: 1px solid #3e3e3e; border-radius: 3px; padding: 5px 0; z-index: 1000; box-shadow: 0 2px 8px rgba(0,0,0,0.3);",
                            onmouseleave: move |_| show_context_menu.set(false),
                            div {
                                class: "context-menu-item",
                                style: "padding: 8px 20px; cursor: pointer; color: #cccccc;",
                                onclick: move |_| {
                                    show_context_menu.set(false);
                                    on_new_request.call(Some(path_for_menu.clone()));
                                },
                                "New Request"
                            }
                        }
                        div {
                            class: "overlay",
                            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 999;",
                            onclick: move |_| show_context_menu.set(false)
                        }
                    }
                }
            }
        }
        FileNode::File { name, path } => {
            let is_selected = current_path.map_or(false, |p| p == path);
            let display_name = name.strip_suffix(".req").unwrap_or(&name);
            rsx! {
                div {
                    class: if is_selected { "file-node selected" } else { "file-node" },
                    onclick: move |_| on_select.call(path.clone()),
                    "📄 {display_name}"
                }
            }
        }
    }
}

#[component]
fn NewFileEditor(
    parent_path: PathBuf,
    on_create: EventHandler<(PathBuf, String)>,
    on_cancel: EventHandler<()>
) -> Element {
    let mut filename = use_signal(|| String::new());
    let parent_path_rc = std::rc::Rc::new(parent_path);

    rsx! {
        div {
            class: "file-node editing",
            style: "display: flex; align-items: center; padding: 2px 0;",
            "📄 "
            input {
                r#type: "text",
                placeholder: "filename",
                value: "{filename()}",
                style: "flex: 1; background: #3c3c3c; color: #cccccc; border: 1px solid #007acc; padding: 2px 5px; font-size: 13px;",
                autofocus: true,
                oninput: move |evt| filename.set(evt.value()),
                onkeydown: {
                    let parent_path_for_key = parent_path_rc.clone();
                    move |evt| {
                        if evt.key() == Key::Enter {
                            let name = filename().trim().to_string();
                            if !name.is_empty() {
                                on_create.call(((*parent_path_for_key).clone(), name));
                            }
                        } else if evt.key() == Key::Escape {
                            on_cancel.call(());
                        }
                    }
                }
            }
            button {
                style: "margin-left: 5px; padding: 2px 8px; background: #0e639c; border: none; color: white; cursor: pointer; font-size: 11px;",
                onclick: {
                    let parent_path_for_btn = parent_path_rc.clone();
                    move |_| {
                        let name = filename().trim().to_string();
                        if !name.is_empty() {
                            on_create.call(((*parent_path_for_btn).clone(), name));
                        }
                    }
                },
                "✓"
            }
            button {
                style: "margin-left: 2px; padding: 2px 8px; background: #5a5a5a; border: none; color: white; cursor: pointer; font-size: 11px;",
                onclick: move |_| on_cancel.call(()),
                "✕"
            }
        }
    }
}

#[component]
fn HeadersEditor(headers: Vec<(String, String)>, on_change: EventHandler<Vec<(String, String)>>) -> Element {
    // Use Rc to share the read-only props with closures
    let headers_rc = std::rc::Rc::new(headers);

    let mut display_headers = headers_rc.as_ref().clone();
    if display_headers.is_empty() || !display_headers.last().unwrap().0.is_empty() {
        display_headers.push(("".to_string(), "".to_string()));
    }

    rsx! {
        div {
            for (i, (k, v)) in display_headers.into_iter().enumerate() {
                {
                    let headers_for_key = headers_rc.clone();
                    let headers_for_val = headers_rc.clone();
                    let headers_for_del = headers_rc.clone();
                    let is_content_type = k.eq_ignore_ascii_case("content-type");

                    rsx! {
                        div { class: "header-row", key: "{i}",
                            input {
                                r#type: "text",
                                placeholder: "Key",
                                value: "{k}",
                                readonly: is_content_type,
                                style: if is_content_type { "background: #2d2d2d; color: #858585;" } else { "" },
                                oninput: move |evt| {
                                    let mut new_headers = headers_for_key.as_ref().clone();
                                    if i < new_headers.len() {
                                        new_headers[i].0 = evt.value();
                                    } else {
                                        new_headers.push((evt.value(), "".to_string()));
                                    }
                                    on_change.call(new_headers);
                                }
                            }
                            input {
                                r#type: "text",
                                placeholder: "Value",
                                value: "{v}",
                                readonly: is_content_type,
                                style: if is_content_type { "background: #2d2d2d; color: #858585;" } else { "" },
                                oninput: move |evt| {
                                    let mut new_headers = headers_for_val.as_ref().clone();
                                    if i < new_headers.len() {
                                        new_headers[i].1 = evt.value();
                                    } else {
                                        new_headers.push(("".to_string(), evt.value()));
                                    }
                                    on_change.call(new_headers);
                                }
                            }
                            button {
                                disabled: is_content_type,
                                style: if is_content_type { "opacity: 0.3; cursor: not-allowed;" } else { "" },
                                onclick: move |_| {
                                    let mut new_headers = headers_for_del.as_ref().clone();
                                    if i < new_headers.len() {
                                        new_headers.remove(i);
                                        on_change.call(new_headers);
                                    }
                                },
                                "✕"
                            }
                        }
                    }
                }
            }
        }
    }
}
