use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::{State, ws::Message, ws::WebSocket, ws::WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{RwLock, mpsc};
use tracing::warn;

use crate::state::AppState;

const CONTENT_UPDATES_REDIS_CHANNEL: &str = "nehex:content-updates";
const REDIS_SUBSCRIBER_RETRY_SECONDS: u64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentUpdateEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub seq: i64,
    pub resource: String,
    pub action: String,
    pub ids: Vec<i64>,
    pub updated_at: String,
}

#[derive(Default)]
pub struct ContentUpdatesHub {
    next_id: AtomicU64,
    subscribers: RwLock<HashMap<u64, mpsc::Sender<Value>>>,
}

impl ContentUpdatesHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn connect(&self, tx: mpsc::Sender<Value>) -> u64 {
        let connection_id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let mut guard = self.subscribers.write().await;
        guard.insert(connection_id, tx);
        connection_id
    }

    pub async fn disconnect(&self, connection_id: u64) {
        let mut guard = self.subscribers.write().await;
        guard.remove(&connection_id);
    }

    pub async fn broadcast_event(&self, event: &ContentUpdateEvent) {
        let payload = match serde_json::to_value(event) {
            Ok(value) => value,
            Err(error) => {
                warn!("[content-updates] failed to encode websocket event: {error}");
                return;
            }
        };

        let snapshot = {
            let guard = self.subscribers.read().await;
            guard.clone()
        };

        let mut stale_ids = Vec::new();
        for (connection_id, tx) in snapshot {
            match tx.try_send(payload.clone()) {
                Ok(_) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    stale_ids.push(connection_id);
                }
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                    // Slow client: skip this event, next events will still be attempted.
                }
            }
        }

        if stale_ids.is_empty() {
            return;
        }

        let mut guard = self.subscribers.write().await;
        for connection_id in stale_ids {
            guard.remove(&connection_id);
        }
    }
}

pub fn router() -> Router<AppState> {
    Router::new().route("/ws/content-updates", get(content_updates_websocket))
}

async fn content_updates_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let hub = state.content_updates_hub.clone();
    ws.on_upgrade(move |socket| async move {
        handle_socket(socket, hub).await;
    })
}

async fn handle_socket(socket: WebSocket, hub: Arc<ContentUpdatesHub>) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Value>(128);

    let connection_id = hub.connect(tx).await;

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
    hub.disconnect(connection_id).await;
}

pub async fn emit_content_update_event(state: &AppState, event: ContentUpdateEvent) {
    if !state.settings.redis_enabled {
        state.content_updates_hub.broadcast_event(&event).await;
        return;
    }

    if let Err(error) = publish_event_to_redis(&state.settings.redis_url, &event).await {
        warn!("[content-updates] redis publish failed, fallback to local broadcast: {error}");
        state.content_updates_hub.broadcast_event(&event).await;
    }
}

pub fn spawn_redis_subscriber(state: AppState) {
    if !state.settings.redis_enabled {
        return;
    }

    let redis_url = state.settings.redis_url.clone();
    let hub = state.content_updates_hub.clone();

    tokio::spawn(async move {
        loop {
            match subscribe_and_forward_events(&redis_url, hub.clone()).await {
                Ok(()) => {
                    warn!("[content-updates] redis subscriber stream ended unexpectedly");
                }
                Err(error) => {
                    warn!("[content-updates] redis subscriber error: {error}");
                }
            }

            tokio::time::sleep(Duration::from_secs(REDIS_SUBSCRIBER_RETRY_SECONDS)).await;
        }
    });
}

async fn subscribe_and_forward_events(
    redis_url: &str,
    hub: Arc<ContentUpdatesHub>,
) -> Result<(), String> {
    let client = redis::Client::open(redis_url)
        .map_err(|error| format!("failed to open redis client: {error}"))?;
    let mut pubsub = client
        .get_async_pubsub()
        .await
        .map_err(|error| format!("failed to create redis pubsub: {error}"))?;

    pubsub
        .subscribe(CONTENT_UPDATES_REDIS_CHANNEL)
        .await
        .map_err(|error| format!("failed to subscribe redis channel: {error}"))?;

    let mut stream = pubsub.on_message();

    while let Some(message) = stream.next().await {
        let payload = match message.get_payload::<String>() {
            Ok(value) => value,
            Err(error) => {
                warn!("[content-updates] failed to read redis payload: {error}");
                continue;
            }
        };

        let event = match serde_json::from_str::<ContentUpdateEvent>(&payload) {
            Ok(value) => value,
            Err(error) => {
                warn!("[content-updates] invalid redis event payload: {error}");
                continue;
            }
        };

        hub.broadcast_event(&event).await;
    }

    Ok(())
}

async fn publish_event_to_redis(redis_url: &str, event: &ContentUpdateEvent) -> Result<(), String> {
    let payload = serde_json::to_string(event)
        .map_err(|error| format!("failed to encode redis payload: {error}"))?;
    let client = redis::Client::open(redis_url)
        .map_err(|error| format!("failed to open redis client: {error}"))?;
    let mut connection = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| format!("failed to connect redis: {error}"))?;

    redis::cmd("PUBLISH")
        .arg(CONTENT_UPDATES_REDIS_CHANNEL)
        .arg(payload)
        .query_async::<i64>(&mut connection)
        .await
        .map_err(|error| format!("failed to publish redis message: {error}"))?;

    Ok(())
}
