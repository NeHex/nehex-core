use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use axum::{
    Router,
    extract::{Query, State, ws::Message, ws::WebSocket, ws::WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{RwLock, mpsc};

use crate::state::AppState;

#[derive(Default)]
pub struct OnlinePresenceHub {
    next_id: AtomicU64,
    pages: RwLock<HashMap<String, HashMap<u64, mpsc::Sender<Value>>>>,
}

impl OnlinePresenceHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(&self, page_key: String, tx: mpsc::Sender<Value>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        {
            let mut guard = self.pages.write().await;
            let page_connections = guard.entry(page_key.clone()).or_default();
            page_connections.insert(id, tx);
        }

        self.broadcast_online_count(&page_key).await;
        id
    }

    pub async fn disconnect(&self, page_key: &str, connection_id: u64) {
        let mut removed_page = false;
        {
            let mut guard = self.pages.write().await;
            if let Some(page_connections) = guard.get_mut(page_key) {
                page_connections.remove(&connection_id);
                if page_connections.is_empty() {
                    guard.remove(page_key);
                    removed_page = true;
                }
            }
        }

        if !removed_page {
            self.broadcast_online_count(page_key).await;
        }
    }

    async fn broadcast_online_count(&self, page_key: &str) {
        let snapshot = {
            let guard = self.pages.read().await;
            guard.get(page_key).cloned()
        };

        let Some(page_connections) = snapshot else {
            return;
        };

        let count = page_connections.len();
        let payload = json!({ "online": count });
        let mut stale_ids = Vec::new();

        for (connection_id, tx) in page_connections {
            match tx.try_send(payload.clone()) {
                Ok(_) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    stale_ids.push(connection_id);
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Backpressure: skip this push for slow clients.
                }
            }
        }

        if stale_ids.is_empty() {
            return;
        }

        let mut guard = self.pages.write().await;
        if let Some(page_connections) = guard.get_mut(page_key) {
            for connection_id in stale_ids {
                page_connections.remove(&connection_id);
            }
            if page_connections.is_empty() {
                guard.remove(page_key);
            }
        }
    }
}

#[derive(Deserialize)]
struct OnlinePresenceQuery {
    page_path: Option<String>,
    page_full: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/ws/online", get(online_presence_websocket))
}

async fn online_presence_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<OnlinePresenceQuery>,
) -> impl IntoResponse {
    let page_key =
        normalize_online_page_key(query.page_path.as_deref(), query.page_full.as_deref());
    let hub = state.online_presence_hub.clone();

    ws.on_upgrade(move |socket| async move {
        handle_socket(socket, hub, page_key).await;
    })
}

async fn handle_socket(socket: WebSocket, hub: Arc<OnlinePresenceHub>, page_key: String) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Value>(64);

    let connection_id = hub.connect(page_key.clone(), tx).await;

    let writer = tokio::spawn(async move {
        while let Some(payload) = rx.recv().await {
            let text = payload.to_string();
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(message) = stream.next().await {
        match message {
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }

    writer.abort();
    hub.disconnect(&page_key, connection_id).await;
}

pub fn normalize_online_page_key(page_path: Option<&str>, page_full: Option<&str>) -> String {
    let mut path = page_path.unwrap_or_default().trim().to_string();
    let full = page_full.unwrap_or_default().trim();

    if path.is_empty() && !full.is_empty() {
        path = extract_path(full);
    }

    if path.is_empty() {
        return "/".to_string();
    }

    path = extract_path(&path);

    if !path.starts_with('/') {
        path.insert(0, '/');
    }

    if path.len() > 512 {
        path.truncate(512);
    }

    if path.is_empty() {
        "/".to_string()
    } else {
        path
    }
}

fn extract_path(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return "/".to_string();
    }

    let path_like = if let Some(proto_index) = raw.find("://") {
        let remain = &raw[(proto_index + 3)..];
        if let Some(path_index) = remain.find('/') {
            &remain[path_index..]
        } else {
            "/"
        }
    } else {
        raw
    };

    let mut normalized = path_like.to_string();
    if let Some(index) = normalized.find('?') {
        normalized.truncate(index);
    }
    if let Some(index) = normalized.find('#') {
        normalized.truncate(index);
    }

    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}
