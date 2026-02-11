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

    let on_refresh_tree = move |_| {
        tree.set(storage::scan_directory());
    };

    let on_select_file = move |path: PathBuf| {
        if let Ok(req) = storage::load_request(&path) {
            current_request.set(req);
            current_path.set(Some(path));
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
            let _ = storage::save_request(path, &current_request.read());
        } else {
            // New file logic could be added here
        }
    };

    rsx! {
        style { {include_str!("style.css")} }
        div { id: "main",
            div { class: "sidebar",
                h3 { "Requests" }
                button { onclick: on_refresh_tree, "Refresh" }
                Sidebar { node: tree.read().clone(), on_select: on_select_file, current_path: current_path.read().clone() }
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
                            textarea {
                                class: "body-editor",
                                value: "{current_request.read().body}",
                                oninput: move |evt| {
                                    current_request.write().body = evt.value();
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
fn Sidebar(node: FileNode, on_select: EventHandler<PathBuf>, current_path: Option<PathBuf>) -> Element {
    match node {
        FileNode::Folder { name, children, .. } => {
            rsx! {
                div { class: "tree-node",
                    div { class: "folder-node", "📁 {name}" }
                    div { style: "margin-left: 10px",
                        for child in children {
                            Sidebar { 
                                node: child.clone(), 
                                on_select: move |p| on_select.call(p),
                                current_path: current_path.clone()
                            }
                        }
                    }
                }
            }
        }
        FileNode::File { name, path } => {
            let is_selected = current_path.map_or(false, |p| p == path);
            rsx! {
                div { 
                    class: if is_selected { "file-node selected" } else { "file-node" },
                    onclick: move |_| on_select.call(path.clone()),
                    "📄 {name}" 
                }
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
                // Clone Rc for the closures in this iteration
                let headers_for_key = headers_rc.clone();
                let headers_for_val = headers_rc.clone();
                let headers_for_del = headers_rc.clone();

                div { class: "header-row", key: "{i}",
                    input {
                        r#type: "text",
                        placeholder: "Key",
                        value: "{k}",
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
