//! SimState: the complete, serializable game state.
//!
//! Everything the client needs to render and everything the rules need to
//! tick lives here. It is a plain value: Clone + Serialize + Deserialize.

use serde::{Deserialize, Serialize};

/// The goods that can be traded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Goods {
    Coal,
    Steel,
    Food,
    Timber,
}

/// A train: a moving agent with carriages carrying goods.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Train {
    pub id: u32,
    /// The node the train is currently at (or between, see `progress`).
    pub node: u32,
    /// The edge the train is travelling along (None = stopped at a node).
    pub edge: Option<u32>,
    /// 0.0..=1.0 progress along `edge` (from `edge.a` to `edge.b`).
    pub progress: f64,
    /// Speed in m/s (base speed; multiplied by speed multiplier by the sim).
    pub speed: f64,
    /// The planned route (node ids). The train follows it when not paused.
    pub route: Vec<u32>,
    /// Route index the train is working toward.
    pub route_idx: usize,
    /// Carriage contents: good -> quantity.
    pub cargo: Vec<(Goods, u32)>,
    /// Carriage capacity (total units).
    pub capacity: u32,
    /// Whether the train is currently held by a signal.
    pub held_by_signal: bool,
    /// True on the tick the train arrives at a node (used so auto-trade
    /// happens once per arrival, not every tick while stopped).
    pub just_arrived: bool,
    /// The node where the last auto-trade happened (a train stopped at a
    /// station — e.g. one just bought there — trades once, not every tick).
    pub traded_at: Option<u32>,
}

impl Train {
    pub fn cargo_qty(&self, g: Goods) -> u32 {
        self.cargo.iter().find(|(x, _)| *x == g).map(|(_, q)| *q).unwrap_or(0)
    }
    pub fn set_cargo(&mut self, g: Goods, qty: u32) {
        if qty == 0 {
            self.cargo.retain(|(x, _)| *x != g);
            return;
        }
        match self.cargo.iter_mut().find(|(x, _)| *x == g) {
            Some((_, q)) => *q = qty,
            None => self.cargo.push((g, qty)),
        }
    }
    pub fn cargo_total(&self) -> u32 {
        self.cargo.iter().map(|(_, q)| *q).sum()
    }
}

/// A signal: protects one direction of one edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: u32,
    /// The edge this signal gates.
    pub edge: u32,
    /// The direction it protects: from `from` toward `to`.
    pub from: u32,
    pub to: u32,
    /// Current aspect.
    pub green: bool,
}

/// The complete simulation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimState {
    pub map: crate::map::Map,
    pub trains: Vec<Train>,
    pub signals: Vec<Signal>,
    /// Discrete tick counter.
    pub tick: u64,
    /// Paused flag.
    pub paused: bool,
    /// Speed multiplier (1.0 = realtime).
    pub speed: f64,
    /// Score: total goods delivered so far.
    pub score: u32,
    /// Money: earned from deliveries, spent on builds (M1).
    pub money: u32,
    /// Deterministic RNG state for demand generation (xorshift64).
    pub rng_state: u64,
}

impl SimState {
    /// A fresh game: the default map, starting funds, no trains (M1: the
    /// player builds everything; the player buys the first train).
    pub fn new_game() -> Self {
        Self {
            map: crate::map::default_map(),
            trains: vec![],
            signals: crate::map::default_signals(),
            tick: 0,
            paused: false,
            speed: 1.0,
            score: 0,
            money: crate::economy::START_FUNDS,
            rng_state: 0x5eed,
        }
    }

    /// Deterministic xorshift64 step. No external RNG crate, so replays are
    /// exact and the state is fully serializable.
    pub fn rng_next(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    /// Deterministic float in [0,1).
    pub fn rng_f64(&mut self) -> f64 {
        (self.rng_next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut a = 0x1234u64;
        let mut b = 0x1234u64;
        for _ in 0..100 {
            a ^= a << 13; a ^= a >> 7; a ^= a << 17;
            b ^= b << 13; b ^= b >> 7; b ^= b << 17;
        }
        assert_eq!(a, b);
    }

    #[test]
    fn cargo_set_and_get() {
        let mut t = Train {
            id: 0, node: 0, edge: None, progress: 0.0, speed: 10.0,
            route: vec![], route_idx: 0, cargo: vec![], capacity: 10,
            held_by_signal: false, just_arrived: false, traded_at: None,
        };
        t.set_cargo(Goods::Coal, 3);
        assert_eq!(t.cargo_qty(Goods::Coal), 3);
        assert_eq!(t.cargo_total(), 3);
        t.set_cargo(Goods::Coal, 0);
        assert_eq!(t.cargo_qty(Goods::Coal), 0);
        assert_eq!(t.cargo_total(), 0);
    }
}
