//! Economy v1 numbers (ADR 0007 — owner-approved baseline).
//!
//! Tuning these is a design change (owner), not an engineering decision.

/// Starting funds for a new game.
pub const START_FUNDS: u32 = 500;
/// Cost of a train.
pub const TRAIN_COST: u32 = 200;
/// Cost of a track segment.
pub const TRACK_COST: u32 = 20;
/// Cost of a signal.
pub const SIGNAL_COST: u32 = 50;
/// Cost of a siding.
pub const SIDING_COST: u32 = 80;
/// Money earned per unit of goods delivered.
pub const DELIVERY_REWARD: u32 = 10;
