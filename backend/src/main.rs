use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request, State,
    },
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use common::{BugMap, WsMessage};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

type AppState = Arc<SharedState>;

struct SharedState {
    bugs: RwLock<BugMap>,
    tx: broadcast::Sender<WsMessage>,
    admin_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let admin_token = std::env::var("ADMIN_TOKEN").expect("ADMIN_TOKEN must be set");

    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(SharedState {
        bugs: RwLock::new(BugMap::new()),
        tx,
        admin_token,
    });

    let admin_routes = Router::new()
        .route("/status", get(|| async { "OK" }))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest("/api/admin", admin_routes)
        .with_state(state)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on {:?}", listener.local_addr());
    
    match axum::serve(listener, app).await {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!("Server error: {}", e);
            Err(e.into())
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    let initial_state = {
        let bugs = state.bugs.read().await;
        bugs.clone()
    };
    
    let msg = match serde_json::to_string(&WsMessage::Sync(initial_state)) {
        Ok(s) => s,
        Err(_) => return,
    };
    
    if sender.send(Message::Text(msg.into())).await.is_err() {
        return;
    }

    let mut rx = state.tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let s = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(_) => break,
            };
            if sender.send(Message::Text(s.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if let Ok(WsMessage::Update(update)) = serde_json::from_str::<WsMessage>(&text) {
                    let mut bugs = state_clone.bugs.write().await;
                    bugs.merge(&update);
                    let _ = state_clone.tx.send(WsMessage::Update(update));
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}

async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
    
    match auth_header {
        Some(auth) if auth == format!("Bearer {}", state.admin_token) => {
            Ok(next.run(request).await)
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
