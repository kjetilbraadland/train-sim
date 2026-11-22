//! Sim: the tick loop. Pure functions over SimState.
//!
//! `Sim::tick` advances the state by one discrete step. It is the only place
//! where the rules live. No I/O, no wall clock, no global RNG.

use crate::state::SimState;

/// One simulation step.
pub struct Sim;

impl Sim {
    /// Advance the state by one tick.
    pub fn tick(state: &mut SimState) {
        if state.paused {
            return;
        }
        state.tick += 1;

        Self::update_signals(state);
        Self::move_trains(state);
        Self::auto_trade(state);
    }

    /// Signal logic (automatic block sections): a signal is green iff the
    /// section it protects is clear of trains AND the destination node is
    /// not occupied by a train. (A train stopped at the destination keeps the
    /// signal red for other trains, which is correct block-section behaviour.)
    fn update_signals(state: &mut SimState) {
        for s in state.signals.iter_mut() {
            let edge_occupied = state.trains.iter().any(|t| t.edge == Some(s.edge));
            let dest_occupied = state.trains.iter().any(|t| t.edge.is_none() && t.node == s.to);
            s.green = !edge_occupied && !dest_occupied;
        }
    }

    /// Move each train along its route, respecting signals.
    ///
    /// `route_idx` points at the *next* node to travel to. A train starts at
    /// `route[0]` with `route_idx == 1`. When it arrives at a node it advances
    /// `route_idx` and sets `just_arrived` (so trade happens once per stop).
    fn move_trains(state: &mut SimState) {
        let dt = crate::SIM_DT * state.speed;
        for t in state.trains.iter_mut() {
            t.just_arrived = false;
            if t.route.is_empty() {
                t.edge = None;
                t.progress = 0.0;
                t.held_by_signal = false;
                continue;
            }
            // Target node: the next node in the route.
            let target = match t.route.get(t.route_idx) {
                Some(&n) => n,
                None => {
                    // Reached the end of the route: stop at the last node.
                    t.edge = None;
                    t.progress = 0.0;
                    t.held_by_signal = false;
                    continue;
                }
            };
            // Find the edge leading from the current node to the target.
            let edge = state
                .map
                .edges_at(t.node)
                .into_iter()
                .find(|e| (e.a == t.node && e.b == target) || (e.b == t.node && e.a == target));
            match edge {
                None => {
                    // No direct edge: the route is invalid. Stop.
                    t.edge = None;
                    t.progress = 0.0;
                    t.held_by_signal = false;
                }
                Some(e) => {
                    // Determine direction.
                    let forward = e.a == t.node;
                    let from = if forward { e.a } else { e.b };
                    let to = if forward { e.b } else { e.a };

                    // If the train is already on this edge, it has passed the
                    // signal; let it finish the section (a signal must not stop
                    // a train mid-section, or it livelocks).
                    let already_on_edge = t.edge == Some(e.id);

                    if !already_on_edge {
                        // Signal check: block entry if the section or the
                        // destination node is occupied.
                        let sig = state.signals.iter().find(|s| s.edge == e.id && s.from == from && s.to == to);
                        let blocked = matches!(sig, Some(s) if !s.green);
                        if blocked {
                            t.edge = None;
                            t.progress = 0.0;
                            t.held_by_signal = true;
                            continue;
                        }
                    }
                    t.held_by_signal = false;

                    if t.edge != Some(e.id) {
                        // Start a new edge.
                        t.edge = Some(e.id);
                        t.progress = 0.0;
                    }

                    // Advance along the edge.
                    let step = t.speed * dt / e.length;
                    t.progress += step;
                    if t.progress >= 1.0 {
                        // Arrived at the far node.
                        t.node = to;
                        t.edge = None;
                        t.progress = 0.0;
                        t.route_idx += 1;
                        t.just_arrived = true;
                    }
                }
            }
        }
    }

    /// Auto-trade: when a train *arrives* at a station, unload cargo that
    /// matches the city's demand (earning money + score), then load from the
    /// city's supply up to capacity. Gated on `just_arrived` so it happens
    /// once per stop, not every tick while the train is idle.
    fn auto_trade(state: &mut SimState) {
        for t in state.trains.iter_mut() {
            if !t.just_arrived {
                continue;
            }
            let city = state.map.cities.iter().find(|c| c.station == t.node);
            let Some(city) = city else { continue };

            // Unload: match cargo against demand.
            for (g, cap) in &city.demand {
                let have = t.cargo_qty(*g);
                if have == 0 {
                    continue;
                }
                let qty = have.min(*cap);
                t.set_cargo(*g, have - qty);
                state.score += qty;
                state.money += qty * 10; // 10 per unit delivered
            }

            // Load: fill capacity from supply.
            for (g, cap) in &city.supply {
                let space = t.capacity - t.cargo_total();
                if space == 0 {
                    break;
                }
                let qty = space.min(*cap);
                let cur = t.cargo_qty(*g);
                t.set_cargo(*g, cur + qty);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::default_map;
    use crate::state::{Goods, Signal, SimState, Train};

    fn fresh() -> SimState {
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
            signals: vec![],
            tick: 0,
            paused: false,
            speed: 1.0,
            score: 0,
            money: 0,
            rng_state: 0xdeadbeef,
        }
    }

    #[test]
    fn tick_advances_time() {
        let mut s = fresh();
        Sim::tick(&mut s);
        assert_eq!(s.tick, 1);
    }

    #[test]
    fn paused_state_does_not_advance() {
        let mut s = fresh();
        s.paused = true;
        Sim::tick(&mut s);
        assert_eq!(s.tick, 0);
    }

    #[test]
    fn train_moves_along_route() {
        let mut s = fresh();
        // 15 m/s * 0.1 s = 1.5 m per tick; edge is 300 m -> 200 ticks to cross.
        for _ in 0..100 {
            Sim::tick(&mut s);
        }
        // After 100 ticks the train should be mid-edge (progress ~0.5).
        let t = &s.trains[0];
        assert!(t.edge.is_some(), "train should be on an edge after 100 ticks");
        assert!((t.progress - 0.5).abs() < 0.01, "progress ~0.5, got {}", t.progress);
    }

    #[test]
    fn train_reaches_end_of_route() {
        let mut s = fresh();
        for _ in 0..700 {
            Sim::tick(&mut s);
        }
        let t = &s.trains[0];
        assert_eq!(t.node, 3, "train should have reached node 3");
        assert!(t.edge.is_none());
        // route_idx advanced past the end (route [0,1,2,3] -> idx 4).
        assert_eq!(t.route_idx, 4);
    }

    #[test]
    fn signal_blocks_train() {
        let mut s = fresh();
        // A signal protecting edge 1 (node 1 -> 2). Signals are automatic
        // block-section signals: they turn red when the section/destination
        // is occupied. So we create a real block by parking a second train
        // on the destination node 2.
        s.signals.push(Signal {
            id: 0,
            edge: 1,
            from: 1,
            to: 2,
            green: true,
        });
        // Second train occupying node 2 (the destination of the signal).
        s.trains.push(Train {
            id: 1,
            node: 2,
            edge: None,
            progress: 0.0,
            speed: 15.0,
            route: vec![],
            route_idx: 0,
            cargo: vec![],
            capacity: 8,
            held_by_signal: false,
            just_arrived: false,
        });
        // Move the first train to node 1 (edge 0 is 300 m at 15 m/s = 200 ticks).
        for _ in 0..200 {
            Sim::tick(&mut s);
        }
        assert_eq!(s.trains[0].node, 1, "train at node 1");
        // The signal must now be red (destination occupied) and the train held.
        assert!(!s.signals[0].green, "signal should be red (destination occupied)");
        let mut held_ticks = 0;
        for _ in 0..50 {
            Sim::tick(&mut s);
            if s.trains[0].held_by_signal {
                held_ticks += 1;
            }
        }
        assert!(held_ticks > 0, "train should be held by the red signal");
        // Remove the blocking train; the signal turns green; the train proceeds.
        s.trains.retain(|t| t.id != 1);
        for _ in 0..400 {
            Sim::tick(&mut s);
        }
        assert_eq!(s.trains[0].node, 3, "train should have proceeded after the block cleared");
    }

    #[test]
    fn auto_trade_earns_score_and_money() {
        let mut s = fresh();
        // Pre-load the train with Coal (City A supplies Coal, City B demands Coal).
        s.trains[0].set_cargo(Goods::Coal, 4);
        for _ in 0..700 {
            Sim::tick(&mut s);
        }
        // The train should have unloaded Coal at City B (demand Coal=3).
        assert!(s.score >= 3, "score should be >= 3, got {}", s.score);
        assert!(s.money >= 30, "money should be >= 30, got {}", s.money);
    }

    #[test]
    fn determinism_same_inputs_same_state() {
        let mut a = fresh();
        let mut b = fresh();
        for _ in 0..500 {
            Sim::tick(&mut a);
            Sim::tick(&mut b);
        }
        let ja = serde_json::to_string(&a).unwrap();
        let jb = serde_json::to_string(&b).unwrap();
        assert_eq!(ja, jb, "identical inputs must produce identical states");
    }
}
