use yew::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{SinkExt, StreamExt};
use wasm_bindgen_futures::spawn_local;
use common::{Bug, BugMap, WsMessage};
use web_sys::{Element, MouseEvent};
use shared_frontend::components::app_shell::AppShell;
use shared_frontend::components::header::HeaderProps;
use shared_frontend::components::footer::FooterProps;
use shared_core::i18n::Language;

#[function_component(CanvasApp)]
fn canvas_app() -> Html {
    let bugs_state = use_state(|| BugMap::new());
    let ws_tx = use_mut_ref(|| None::<futures::channel::mpsc::UnboundedSender<WsMessage>>);

    {
        let bugs_state = bugs_state.clone();
        let ws_tx_clone = ws_tx.clone();
        use_effect_with((), move |_| {
            if let Ok(ws) = WebSocket::open("ws://127.0.0.1:3000/ws") {
                let (mut write, mut read) = ws.split();
                let (tx, mut rx) = futures::channel::mpsc::unbounded::<WsMessage>();
                *ws_tx_clone.borrow_mut() = Some(tx);

                spawn_local(async move {
                    while let Some(msg) = rx.next().await {
                        if let Ok(s) = serde_json::to_string(&msg) {
                            let _ = write.send(Message::Text(s)).await;
                        }
                    }
                });

                spawn_local(async move {
                    while let Some(Ok(msg)) = read.next().await {
                        if let Message::Text(text) = msg {
                            if let Ok(parsed) = serde_json::from_str::<WsMessage>(&text) {
                                match parsed {
                                    WsMessage::Sync(map) => {
                                        bugs_state.set(map);
                                    }
                                    WsMessage::Update(update) => {
                                        let mut current = (*bugs_state).clone();
                                        current.merge(&update);
                                        bugs_state.set(current);
                                    }
                                }
                            }
                        }
                    }
                });
            }
            || ()
        });
    }

    let on_canvas_click = {
        let bugs_state = bugs_state.clone();
        let ws_tx = ws_tx.clone();
        Callback::from(move |e: MouseEvent| {
            let target = match e.target_dyn_into::<Element>() {
                Some(t) => t,
                None => return,
            };
            if target.class_name() != "canvas-container" {
                return;
            }
            
            let x = e.offset_x() as f64;
            let y = e.offset_y() as f64;

            let new_bug = Bug {
                id: uuid::Uuid::new_v4().to_string(),
                x,
                y,
                bounty: 10,
                resolved: false,
            };

            let mut update_map = BugMap::new();
            let ts = js_sys::Date::now() as i64;
            update_map.insert(new_bug.clone(), ts);

            let mut current = (*bugs_state).clone();
            current.merge(&update_map);
            bugs_state.set(current);

            if let Some(tx) = ws_tx.borrow().as_ref() {
                let _ = tx.unbounded_send(WsMessage::Update(update_map));
            }
        })
    };

    let on_bug_click = {
        let bugs_state = bugs_state.clone();
        let ws_tx = ws_tx.clone();
        Callback::from(move |(id, resolve): (String, bool)| {
            let mut current = (*bugs_state).clone();
            if let Some(entry) = current.bugs.get(&id) {
                let mut updated_bug = entry.bug.clone();
                if resolve {
                    updated_bug.resolved = true;
                } else {
                    updated_bug.bounty += 10;
                }

                let mut update_map = BugMap::new();
                let ts = js_sys::Date::now() as i64;
                update_map.insert(updated_bug, ts);

                current.merge(&update_map);
                bugs_state.set(current);

                if let Some(tx) = ws_tx.borrow().as_ref() {
                    let _ = tx.unbounded_send(WsMessage::Update(update_map));
                }
            }
        })
    };

    let header = HeaderProps {
        site_title: "Canvas Bugs".into(),
        theme: "crateria".into(),
        language: Language::English,
        toggle_theme: Callback::noop(),
        on_language_change: Callback::noop(),
        is_authenticated: false,
        pin_required: false,
        on_logout: Callback::noop(),
        logout_tooltip: "Logout".into(),
        theme_toggle_tooltip: "Toggle Theme".into(),
        print_tooltip: "Print".into(),
        on_print: None,
        enable_translation: false,
        enable_themes: false,
        enable_print: false,
        print_disabled: true,
        site_url: Some("/".into()),
        repo: Some("https://github.com/studio2201/canvas".into()),
        version: Some("0.1.0".into()),
        version_url: Some("https://github.com/studio2201/canvas/releases/tag/v0.1.0".into()),
    };

    let footer = FooterProps {
        show_version: true,
        version: "0.1.0".into(),
        show_github: true,
        github_url: Some("https://github.com/studio2201/canvas".into()),
        version_url: Some("https://github.com/studio2201/canvas/releases/tag/v0.1.0".into()),
        repo: Some("studio2201/canvas".into()),
        show_coffee: false,
        coffee_url: None,
        children: html! {},
    };

    html! {
        <AppShell
            header={header}
            footer={footer}
            use_container={false}
        >
            <div 
                class="canvas-container" 
                style="position: relative; width: 100%; height: 80vh; background-color: #f0f0f0; overflow: hidden; cursor: crosshair;"
                onclick={on_canvas_click}
            >
                {
                    for bugs_state.bugs.values().map(|entry| {
                        let bug = &entry.bug;
                        let id = bug.id.clone();
                        let id2 = bug.id.clone();
                        
                        let on_click_bounty = {
                            let cb = on_bug_click.clone();
                            let bid = id.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.stop_propagation();
                                cb.emit((bid.clone(), false));
                            })
                        };
                        
                        let on_click_resolve = {
                            let cb = on_bug_click.clone();
                            let bid = id2.clone();
                            Callback::from(move |e: MouseEvent| {
                                e.stop_propagation();
                                cb.emit((bid.clone(), true));
                            })
                        };

                        let bg_color = if bug.resolved { "#a0e0a0" } else { "#ffb0b0" };
                        
                        html! {
                            <div
                                style={format!("position: absolute; left: {}px; top: {}px; transform: translate(-50%, -50%); background-color: {}; padding: 8px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.2); cursor: default;", bug.x, bug.y, bg_color)}
                            >
                                <div style="font-weight: bold; margin-bottom: 4px;">{ format!("Bounty: ${}", bug.bounty) }</div>
                                if !bug.resolved {
                                    <button onclick={on_click_bounty} style="margin-right: 4px; cursor: pointer;">{ "+ Bounty" }</button>
                                    <button onclick={on_click_resolve} style="cursor: pointer;">{ "Resolve" }</button>
                                } else {
                                    <span style="color: #444;">{ "Resolved" }</span>
                                }
                            </div>
                        }
                    })
                }
            </div>
        </AppShell>
    }
}

fn main() {
    yew::Renderer::<CanvasApp>::new().render();
}
