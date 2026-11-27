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
    /// Throughput: units delivered in the trailing 10 s window (100 ticks),
    /// per second (M2: the metric the player optimizes).
    pub throughput: f64,
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
            throughput: Self::throughput(self),
        }
    }

    /// Units delivered in the trailing 10 s window, per second.
    /// `score_window[i]` is the score at the start of tick `i`; the window
    /// holds the last 100 ticks (10 s).
    fn throughput(s: &SimState) -> f64 {
        if s.score_window.len() < 100 {
            return 0.0;
        }
        let delivered = s.score - s.score_window[0];
        delivered as f64 / 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::default_map;

    #[test]
    fn score_window_bounded_to_100() {
        let mut s = SimState::new_game();
        for _ in 0..300 {
            crate::Sim::tick(&mut s);
        }
        assert_eq!(s.score_window.len(), 100, "window must not grow unbounded");
    }

    #[test]
    fn throughput_reflects_window() {
        let mut s = SimState::new_game();
        // A train delivering 4 units over the run: after 700 ticks (70 s)
        // the trailing 10 s window contains the delivery, so throughput
        // must equal 4 units / 10 s = 0.4 u/s.
        use crate::state::Train;
        s.trains.push(Train {
            id: 0, node: 0, edge: None, progress: 0.0, speed: 15.0,
            route: vec![0, 1, 2, 3], route_idx: 1, cargo: vec![],
            capacity: 8, held_by_signal: false, just_arrived: false,
            traded_at: None,
        });
        for _ in 0..700 {
            crate::Sim::tick(&mut s);
        }
        let st = s.status();
        assert_eq!(st.score, 4, "City B takes all 4 Coal units");
        assert!((st.throughput - 0.4).abs() < 1e-9, "got {}", st.throughput);
    }

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
            score_window: vec![],
        };
        let st = s.status();
        assert_eq!(st.tick, 7);
        assert_eq!(st.score, 5);
        assert!(st.paused);
        assert_eq!(st.speed, 2.0);
    }
}
