//! Scoring and stats helpers (pure, no state mutation).

use crate::state::SimState;

/// A summary of the current game status, sent to the client with each
/// snapshot (or on demand).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    pub tick: u64,
    pub score: u32,
    pub money: u32,
    pub paused: bool,
    pub speed: f64,
    pub trains: usize,
    pub signals_green: usize,
    pub signals_red: usize,
}

impl SimState {
    pub fn status(&self) -> Status {
        let (g, r) = self.signals.iter().fold((0usize, 0usize), |(g, r), s| {
            if s.green { (g + 1, r) } else { (g, r + 1) }
        });
        Status {
            tick: self.tick,
            score: self.score,
            money: self.money,
            paused: self.paused,
            speed: self.speed,
            trains: self.trains.len(),
            signals_green: g,
            signals_red: r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::default_map;

    #[test]
    fn status_reflects_state() {
        let s = SimState {
            map: default_map(),
            trains: vec![],
            signals: vec![],
            tick: 7,
            paused: true,
            speed: 2.0,
            score: 5,
            money: 50,
            rng_state: 1,
        };
        let st = s.status();
        assert_eq!(st.tick, 7);
        assert_eq!(st.score, 5);
        assert!(st.paused);
        assert_eq!(st.speed, 2.0);
    }
}
