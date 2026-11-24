//! train-sim-server
//!
//! Thin I/O shell around the pure engine:
//!   - Runs the tick loop (one tick per 100 ms of wall time at 1x speed).
//!   - Serves the three.js client (static files) at `/`.
//!   - Accepts commands over WebSocket (`/ws`), applies them to the state,
//!     and broadcasts a JSON snapshot to all connected clients.
//!
//! The server owns NO game rules — everything is in `train-sim-engine`.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tower_http::services::ServeDir;

use train_sim_engine::command::{self, CmdError};
use train_sim_engine::{Command, Sim, SimState};

/// One tick per 100 ms of wall time (at 1x speed).
const TICK_MS: u64 = 100;

struct AppState {
    state: RwLock<SimState>,
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8771);

    let (tx, _rx) = broadcast::channel(1024);
    let app = Arc::new(AppState {
        state: RwLock::new(new_game_state()),
        tx,
    });

    // Tick loop: advance the sim, broadcast a snapshot each tick.
    {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
                let mut state = app.state.write().await;
                Sim::tick(&mut state);
                let snapshot = serde_json::to_string(&*state).unwrap();
                let _ = app.tx.send(snapshot);
            }
        });
    }

    let router = Router::new()
        .route("/", get(root_handler))
        .route("/ws", get(ws_handler))
        .route("/status", get(status_handler))
        .fallback_service(ServeDir::new("static"))
        .with_state(app);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("train-sim server on http://{}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

/// Build a fresh game state (engine owns the rules: default map, starting
/// funds, no trains — the player buys the first train, M1).
fn new_game_state() -> SimState {
    SimState::new_game()
}

async fn status_handler(State(app): State<Arc<AppState>>) -> impl IntoResponse {
    let state = app.state.read().await;
    (axum::http::StatusCode::OK, serde_json::to_string(&state.status()).unwrap())
}

async fn root_handler() -> impl IntoResponse {
    axum::response::Html(include_str!("../../static/index.html"))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, app))
}

async fn handle_ws(socket: WebSocket, app: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Outbound: forward broadcast snapshots to this client.
    let mut rx = app.tx.subscribe();
    let out = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Inbound: decode commands, apply to state (rules live in the engine).
    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };
        let text = text.to_string();
        match serde_json::from_str::<Command>(&text) {
            Ok(cmd) => {
                let mut state = app.state.write().await;
                if let Err(e) = command::apply(&mut state, &cmd) {
                    tracing::warn!("rejected command: {e}");
                }
            }
            Err(e) => tracing::warn!("bad command: {e}"),
        }
    }
    out.abort();
}
