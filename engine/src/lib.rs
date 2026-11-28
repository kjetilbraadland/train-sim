//! train-sim-engine
//!
//! A pure, deterministic train-logistics simulation. No I/O, no network, no
//! graphics, no wall clock. The whole game state is a value in [`SimState`];
//! advancing the game is `state.tick()`. This makes the engine trivially
//! testable and reproducible: the same initial state + same input sequence
//! always yields the same state (a property we assert in tests).
//!
//! Architecture boundary:
//!   - `engine` (this crate): pure state + rules. No dependencies beyond serde.
//!   - `server`: axum + WebSocket. Reads commands, applies them, broadcasts
//!     state snapshots. Knows nothing about rendering.
//!   - `client`: three.js. Subscribes to snapshots, renders. Knows nothing
//!     about rules.
//!
//! Determinism rules:
//!   - Time is discrete ticks (`SIM_DT` seconds per tick).
//!   - No `SystemTime`, no `rand::thread_rng`. Randomness (demand generation)
//!     comes from a seeded `Rng` stored in state, so replays are exact.

pub mod command;
pub mod economy;
pub mod map;
pub mod scoring;
pub mod state;
pub mod sim;

pub use state::{Goods, SimState, Train};
pub use map::{City, Map, Node, Edge};
pub use sim::Sim;

/// Seconds simulated per tick. Fixed, so movement is deterministic.
pub const SIM_DT: f64 = 0.1;

/// A command the player issues to the simulation. The server decodes JSON
/// into these and applies them to the state via [`command::apply`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    /// Start a new game with the default map (or a provided one).
    NewGame { map_id: Option<String> },
    /// Pause / resume the simulation.
    SetPaused { paused: bool },
    /// Change the simulation speed multiplier (1.0 = realtime).
    SetSpeed { multiplier: f64 },
    /// Assign a route to a train: ordered list of node ids.
    SetRoute { train_id: u32, route: Vec<u32> },
    /// At a station: load `qty` of `good` (from city supply into carriages).
    Load { train_id: u32, good: Goods, qty: u32 },
    /// At a station: unload `qty` of `good` (from carriages into city demand).
    Unload { train_id: u32, good: Goods, qty: u32 },
    /// Buy a train at a node (costs `economy::TRAIN_COST`). M1.
    BuyTrain { node: u32 },
    /// Build a track segment (costs `economy::TRACK_COST`) from `from`, one
    /// grid cell (300 m) in direction `dir`:
    /// 0=E 1=W 2=N 3=S 4=NE 5=NW 6=SE 7=SW (ADR 0004, ADR 0015).
    /// If no node exists in the target cell, a node is created there.
    /// M1.
    BuildTrack { from: u32, dir: u8 },
    /// Place a signal (costs `economy::SIGNAL_COST`) on `edge`, protecting
    /// the direction from `from` (must be an endpoint of the edge) toward
    /// the other endpoint. Automatic block-section signal (ADR 0005). M1.
    BuildSignal { edge: u32, from: u32 },
    /// Remove a track segment and refund `economy::TRACK_COST`. Rejected
    /// while a train occupies the edge or an endpoint (a train cannot be
    /// left on a vanished track). M2.
    DeleteTrack { edge: u32 },
    /// Remove a signal and refund `economy::SIGNAL_COST`. M2.
    DeleteSignal { signal: u32 },
    /// Remove a train and refund `economy::TRAIN_COST`. Rejected while the
    /// train is moving (a moving train cannot be deleted mid-section). M2.
    DeleteTrain { train: u32 },
}
