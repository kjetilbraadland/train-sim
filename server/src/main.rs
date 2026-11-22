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

/// Build a fresh game state with the default map and one starter train.
fn new_game_state() -> SimState {
    use train_sim_engine::map::default_map;
    use train_sim_engine::state::Train;
    SimState {
        map: default_map(),
        trains: vec![Train {
            id: 0,
            node: 0,
            edge: None,
            progress: 0.0,
            speed: 15.0,
            route: vec![0, 1, 2, 3],
            route_idx: 1,
            cargo: vec![],
            capacity: 8,
            held_by_signal: false,
            just_arrived: false,
        }],
        signals: default_signals(),
        tick: 0,
        paused: false,
        speed: 1.0,
        score: 0,
        money: 0,
        rng_state: 0x5eed,
    }
}

fn default_signals() -> Vec<train_sim_engine::state::Signal> {
    use train_sim_engine::state::Signal;
    // Signals on the edges flagged has_signal in the default map.
    vec![
        Signal { id: 0, edge: 1, from: 1, to: 2, green: true },
        Signal { id: 1, edge: 4, from: 4, to: 5, green: true },
        Signal { id: 2, edge: 6, from: 7, to: 8, green: true },
        Signal { id: 3, edge: 9, from: 10, to: 11, green: true },
        Signal { id: 4, edge: 10, from: 2, to: 12, green: true },
        Signal { id: 5, edge: 13, from: 13, to: 3, green: true },
    ]
}

async fn status_handler(State(app): State<Arc<AppState>>) -> impl IntoResponse {
    let state = app.state.read().await;
    (axum::http::StatusCode::OK, serde_json::to_string(&state.status()).unwrap())
}

async fn root_handler() -> impl IntoResponse {
    "train-sim server"
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

    // Inbound: decode commands, apply to state.
    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };
        let text = text.to_string();
        match serde_json::from_str::<Command>(&text) {
            Ok(cmd) => {
                let mut state = app.state.write().await;
                apply_command(&mut state, cmd);
            }
            Err(e) => tracing::warn!("bad command: {e}"),
        }
    }
    out.abort();
}

fn apply_command(state: &mut SimState, cmd: Command) {
    match cmd {
        Command::NewGame { map_id: _ } => {
            *state = new_game_state();
        }
        Command::SetPaused { paused } => state.paused = paused,
        Command::SetSpeed { multiplier } => state.speed = multiplier.clamp(0.25, 8.0),
        Command::SetRoute { train_id, route } => {
            if let Some(t) = state.trains.iter_mut().find(|t| t.id == train_id) {
                t.route = route;
                t.route_idx = 1; // route[0] is the starting node
            }
        }
        Command::Load { train_id, good, qty } => {
            if let Some(t) = state.trains.iter_mut().find(|t| t.id == train_id) {
                let space = t.capacity - t.cargo_total();
                let q = qty.min(space);
                t.set_cargo(good, t.cargo_qty(good) + q);
            }
        }
        Command::Unload { train_id, good, qty } => {
            if let Some(t) = state.trains.iter_mut().find(|t| t.id == train_id) {
                let have = t.cargo_qty(good);
                let q = qty.min(have);
                t.set_cargo(good, have - q);
            }
        }
    }
}
