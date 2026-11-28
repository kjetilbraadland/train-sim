//! Command application: the single place commands mutate state.
//!
//! Pure and deterministic: no I/O, no wall clock. The server decodes JSON
//! into [`Command`] and calls [`apply`]; it owns no rules of its own.

use crate::state::{Signal, SimState, Train};
use crate::map::{Edge, Node};
use crate::Command;

/// How close (m) a grid-step target cell must be to an existing node to
/// connect to it instead of creating a duplicate node.
const NODE_MATCH_EPS: f64 = 25.0;

/// Error from a rejected command (insufficient funds, unknown id, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdError {
    InsufficientFunds { cost: u32, have: u32 },
    UnknownNode(u32),
    UnknownTrain(u32),
    UnsupportedMap(String),
    /// Build: grid direction value out of range (0..=7).
    InvalidDir(u8),
    /// Build: no such edge.
    UnknownEdge(u32),
    /// Build: an edge already connects the two nodes.
    DuplicateEdge(u32, u32),
    /// Build: a signal already protects that edge in that direction.
    SignalAlreadyPlaced(u32),
    /// Build: `node` is not an endpoint of `edge`.
    NotEdgeEndpoint { edge: u32, node: u32 },
    /// Route: empty route.
    EmptyRoute,
    /// Route: route[0] is not the node the train is at.
    RouteWrongStart { node: u32, train_at: u32 },
    /// Route: no edge connects two consecutive route nodes.
    RouteDisconnected { from: u32, to: u32 },
    /// Delete track: a train occupies the edge or one of its endpoints, so
    /// removing it would strand the train.
    EdgeOccupied(u32),
    /// Delete train: the train is moving (on an edge), not stopped at a node.
    TrainMoving(u32),
    /// Delete signal: no such signal.
    UnknownSignal(u32),
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::InsufficientFunds { cost, have } => {
                write!(f, "insufficient funds: need {cost}, have {have}")
            }
            CmdError::UnknownNode(n) => write!(f, "unknown node {n}"),
            CmdError::UnknownTrain(n) => write!(f, "unknown train {n}"),
            CmdError::UnsupportedMap(m) => write!(f, "unsupported map '{m}'"),
            CmdError::InvalidDir(d) => write!(f, "invalid grid direction {d} (0..=7)"),
            CmdError::UnknownEdge(e) => write!(f, "unknown edge {e}"),
            CmdError::DuplicateEdge(a, b) => {
                write!(f, "edge already exists between nodes {a} and {b}")
            }
            CmdError::SignalAlreadyPlaced(e) => {
                write!(f, "a signal already protects edge {e} in that direction")
            }
            CmdError::NotEdgeEndpoint { edge, node } => {
                write!(f, "node {node} is not an endpoint of edge {edge}")
            }
            CmdError::EmptyRoute => write!(f, "route must not be empty"),
            CmdError::RouteWrongStart { node, train_at } => {
                write!(f, "route starts at node {node} but the train is at node {train_at}")
            }
            CmdError::RouteDisconnected { from, to } => {
                write!(f, "no track between nodes {from} and {to}")
            }
            CmdError::EdgeOccupied(e) => {
                write!(f, "track {e} is occupied by a train; it cannot be removed")
            }
            CmdError::TrainMoving(t) => {
                write!(f, "train {t} is moving and cannot be deleted")
            }
            CmdError::UnknownSignal(s) => write!(f, "unknown signal {s}"),
        }
    }
}

/// Apply a command to the state. Pure and deterministic: the same state +
/// the same command always yields the same result.
pub fn apply(state: &mut SimState, cmd: &Command) -> Result<(), CmdError> {
    match cmd {
        Command::NewGame { map_id } => match map_id.as_deref() {
            None | Some("default") => {
                *state = SimState::new_game();
            }
            Some(other) => return Err(CmdError::UnsupportedMap(other.into())),
        },
        Command::SetPaused { paused } => state.paused = *paused,
        Command::SetSpeed { multiplier } => state.speed = multiplier.clamp(0.25, 8.0),
        Command::SetRoute { train_id, route } => {
            // Validate before mutating (ADR 0011): a rejected route must not
            // change the train.
            let at = state
                .trains
                .iter()
                .find(|t| t.id == *train_id)
                .map(|t| t.node)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
            if route.is_empty() {
                return Err(CmdError::EmptyRoute);
            }
            if route[0] != at {
                return Err(CmdError::RouteWrongStart {
                    node: route[0],
                    train_at: at,
                });
            }
            for w in route.windows(2) {
                if state.map.edge_between(w[0], w[1]).is_none() {
                    return Err(CmdError::RouteDisconnected {
                        from: w[0],
                        to: w[1],
                    });
                }
            }
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .expect("train validated above");
            t.route = route.clone();
            t.route_idx = 1; // route[0] is the starting node
        }
        Command::Load { train_id, good, qty } => {
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
            let space = t.capacity - t.cargo_total();
            let q = (*qty).min(space);
            t.set_cargo(*good, t.cargo_qty(*good) + q);
        }
        Command::Unload { train_id, good, qty } => {
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
            let have = t.cargo_qty(*good);
            let q = (*qty).min(have);
            t.set_cargo(*good, have - q);
        }
        Command::BuyTrain { node } => {
            if state.map.node(*node).is_none() {
                return Err(CmdError::UnknownNode(*node));
            }
            if state.money < crate::economy::TRAIN_COST {
                return Err(CmdError::InsufficientFunds {
                    cost: crate::economy::TRAIN_COST,
                    have: state.money,
                });
            }
            state.money -= crate::economy::TRAIN_COST;
            // Deterministic id: one past the highest existing train id.
            let id = state.trains.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            state.trains.push(Train {
                id,
                node: *node,
                edge: None,
                progress: 0.0,
                // Fixed base speed 15 m/s (matches the pre-M1 starter train).
                // Open question in DESIGN.md §8: fixed vs. upgradeable speed.
                speed: 15.0,
                route: vec![],
                route_idx: 0,
                cargo: vec![],
                capacity: 8,
                held_by_signal: false,
                just_arrived: false,
                traded_at: None,
            });
        }
        Command::BuildTrack { from, dir } => {
            // Validate before mutating (ADR 0011).
            let step = crate::map::grid_dir_step(*dir)
                .ok_or(CmdError::InvalidDir(*dir))?;
            let anchor = state
                .map
                .node(*from)
                .ok_or(CmdError::UnknownNode(*from))?;
            if state.money < crate::economy::TRACK_COST {
                return Err(CmdError::InsufficientFunds {
                    cost: crate::economy::TRACK_COST,
                    have: state.money,
                });
            }
            // Target cell: snap to an existing node if one is close enough,
            // otherwise create a node there.
            let target_pos = [
                anchor.pos[0] + step[0],
                anchor.pos[1] + step[1],
            ];
            let target = match crate::map::node_near(&state.map, target_pos, NODE_MATCH_EPS) {
                Some(id) => id,
                None => {
                    let id = state.map.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
                    state.map.nodes.push(Node {
                        id,
                        pos: target_pos,
                        city: None,
                    });
                    id
                }
            };
            if state.map.edge_between(*from, target).is_some() {
                return Err(CmdError::DuplicateEdge(*from, target));
            }
            state.money -= crate::economy::TRACK_COST;
            let length = state
                .map
                .node_distance(*from, target)
                .unwrap_or(crate::map::GRID_STEP);
            let id = state.map.edges.iter().map(|e| e.id).max().unwrap_or(0) + 1;
            state.map.edges.push(Edge {
                id,
                a: *from,
                b: target,
                length,
                has_signal: false,
            });
        }
        Command::BuildSignal { edge, from } => {
            // Validate before mutating (ADR 0011).
            let e = state.map.edge(*edge).ok_or(CmdError::UnknownEdge(*edge))?;
            let to = if e.a == *from {
                e.b
            } else if e.b == *from {
                e.a
            } else {
                return Err(CmdError::NotEdgeEndpoint {
                    edge: *edge,
                    node: *from,
                });
            };
            if state
                .signals
                .iter()
                .any(|s| s.edge == *edge && s.from == *from && s.to == to)
            {
                return Err(CmdError::SignalAlreadyPlaced(*edge));
            }
            if state.money < crate::economy::SIGNAL_COST {
                return Err(CmdError::InsufficientFunds {
                    cost: crate::economy::SIGNAL_COST,
                    have: state.money,
                });
            }
            state.money -= crate::economy::SIGNAL_COST;
            let id = state.signals.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            // Initial aspect follows the block-section rule (sim.rs
            // `update_signals` recomputes it every tick).
            let edge_occupied = state.trains.iter().any(|t| t.edge == Some(*edge));
            let dest_occupied =
                state.trains.iter().any(|t| t.edge.is_none() && t.node == to);
            state.signals.push(Signal {
                id,
                edge: *edge,
                from: *from,
                to,
                green: !edge_occupied && !dest_occupied,
            });
        }
        Command::DeleteTrack { edge } => {
            // Validate before mutating (ADR 0011).
            let e = state.map.edge(*edge).ok_or(CmdError::UnknownEdge(*edge))?;
            // A train on the edge or at either endpoint would be stranded by
            // removing the track, so reject.
            let occupied = state.trains.iter().any(
                |t| t.edge == Some(*edge) || t.node == e.a || t.node == e.b,
            );
            if occupied {
                return Err(CmdError::EdgeOccupied(*edge));
            }
            state.map.edges.retain(|x| x.id != *edge);
            // Full refund (labeled assumption, DESIGN.md §8).
            state.money = state.money.saturating_add(crate::economy::TRACK_COST);
        }
        Command::DeleteSignal { signal } => {
            // Validate before mutating (ADR 0011).
            if state.signals.iter().all(|s| s.id != *signal) {
                return Err(CmdError::UnknownSignal(*signal));
            }
            state.signals.retain(|s| s.id != *signal);
            // Full refund (labeled assumption, DESIGN.md §8).
            state.money = state.money.saturating_add(crate::economy::SIGNAL_COST);
        }
        Command::DeleteTrain { train } => {
            // Validate before mutating (ADR 0011).
            let t = state
                .trains
                .iter()
                .find(|t| t.id == *train)
                .ok_or(CmdError::UnknownTrain(*train))?;
            if t.edge.is_some() {
                return Err(CmdError::TrainMoving(*train));
            }
            state.trains.retain(|t| t.id != *train);
            // Full refund (labeled assumption, DESIGN.md §8).
            state.money = state.money.saturating_add(crate::economy::TRAIN_COST);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Goods;

    #[test]
    fn new_game_starts_with_start_funds_and_no_trains() {
        let s = SimState::new_game();
        assert_eq!(s.money, 500);
        assert!(s.trains.is_empty());
        // The default map flags 6 edges with signals; new_game derives one
        // signal per flagged edge.
        assert_eq!(s.signals.len(), 6);
    }

    #[test]
    fn buy_train_deducts_money_and_spawns() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        assert_eq!(s.money, 300);
        assert_eq!(s.trains.len(), 1);
        assert_eq!(s.trains[0].node, 0);
        assert_eq!(s.trains[0].id, 1);
        // A second train gets the next id.
        apply(&mut s, &Command::BuyTrain { node: 3 }).unwrap();
        assert_eq!(s.trains[1].id, 2);
        assert_eq!(s.money, 100);
    }

    #[test]
    fn buy_train_rejected_when_broke() {
        let mut s = SimState::new_game();
        s.money = 100;
        assert_eq!(
            apply(&mut s, &Command::BuyTrain { node: 0 }),
            Err(CmdError::InsufficientFunds { cost: 200, have: 100 })
        );
        assert!(s.trains.is_empty(), "no train on a rejected buy");
    }

    #[test]
    fn buy_train_rejected_on_unknown_node() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuyTrain { node: 999 }),
            Err(CmdError::UnknownNode(999))
        );
    }

    #[test]
    fn bought_train_moves_and_trades() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        // City A (node 0) supplies Coal; City B (node 3) demands Coal.
        apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![0, 1, 2, 3] })
            .unwrap();
        for _ in 0..700 {
            crate::Sim::tick(&mut s);
        }
        assert!(
            s.score >= 3,
            "bought train should have delivered goods, score={}",
            s.score
        );
        assert!(
            s.money >= 130,
            "money should have grown from deliveries, money={}",
            s.money
        );
    }

    #[test]
    fn set_route_unknown_train_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::SetRoute { train_id: 7, route: vec![0, 1] }),
            Err(CmdError::UnknownTrain(7))
        );
    }

    #[test]
    fn load_clamped_to_capacity() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::Load { train_id: 1, good: Goods::Coal, qty: 99 })
            .unwrap();
        assert_eq!(s.trains[0].cargo_total(), 8, "capacity is 8");
    }

    #[test]
    fn new_game_unknown_map_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::NewGame { map_id: Some("nope".into()) }),
            Err(CmdError::UnsupportedMap("nope".into()))
        );
        assert_eq!(s.money, 500, "rejected command must not change state");
    }

    #[test]
    fn build_track_creates_node_and_edge() {
        let mut s = SimState::new_game();
        assert_eq!(s.money, 500);
        // Node 5 (1650,1150) east 300 m -> (1950,1150): no node there yet.
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 0 }).unwrap();
        assert_eq!(s.money, 480, "track costs 20");
        assert_eq!(s.map.nodes.len(), 15, "a new node is created");
        assert_eq!(s.map.edges.len(), 15, "a new edge is created");
        let e = s.map.edge_between(5, 14).expect("edge 5-14 exists");
        assert!((e.length - 300.0).abs() < 1e-6);
        assert!(!e.has_signal, "built track has no signal yet");
    }

    #[test]
    fn build_track_reuses_existing_node() {
        let mut s = SimState::new_game();
        // Build an L: 5 -E-> 14, 14 -N-> 15, 5 -N-> 16, 16 -E-> 15 (reuse).
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 0 }).unwrap();
        apply(&mut s, &Command::BuildTrack { from: 14, dir: 2 }).unwrap();
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 2 }).unwrap();
        // (1950,1450) is now the existing node 15: connect, don't duplicate.
        apply(&mut s, &Command::BuildTrack { from: 16, dir: 0 }).unwrap();
        assert_eq!(s.map.nodes.len(), 17, "only two new nodes for four segments");
        assert_eq!(s.map.edges.len(), 18);
        assert!(s.map.edge_between(15, 16).is_some(), "15-16 connects");
        assert_eq!(s.money, 500 - 80);
    }

    #[test]
    fn build_track_duplicate_edge_rejected() {
        let mut s = SimState::new_game();
        // Node 5 (1650,1150) west 300 m -> node 4; edge 4-5 already exists.
        assert_eq!(
            apply(&mut s, &Command::BuildTrack { from: 5, dir: 1 }),
            Err(CmdError::DuplicateEdge(5, 4))
        );
        assert_eq!(s.money, 500, "rejected build must not change state");
        assert_eq!(s.map.nodes.len(), 14);
    }

    #[test]
    fn build_track_rejected_when_broke() {
        let mut s = SimState::new_game();
        s.money = 10;
        assert_eq!(
            apply(&mut s, &Command::BuildTrack { from: 5, dir: 0 }),
            Err(CmdError::InsufficientFunds { cost: 20, have: 10 })
        );
        assert_eq!(s.map.edges.len(), 14, "no edge on a rejected build");
    }

    #[test]
    fn build_track_invalid_dir_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildTrack { from: 5, dir: 8 }),
            Err(CmdError::InvalidDir(8))
        );
        assert_eq!(s.map.nodes.len(), 14, "no node on a rejected build");
        assert_eq!(s.map.edges.len(), 14);
    }

    #[test]
    fn build_track_unknown_node_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildTrack { from: 999, dir: 0 }),
            Err(CmdError::UnknownNode(999))
        );
    }

    #[test]
    fn build_signal_places_and_costs() {
        let mut s = SimState::new_game();
        assert_eq!(s.money, 500);
        apply(&mut s, &Command::BuildSignal { edge: 0, from: 0 }).unwrap();
        assert_eq!(s.money, 450, "signal costs 50");
        assert_eq!(s.signals.len(), 7);
        let sig = &s.signals[6];
        assert_eq!(sig.edge, 0);
        assert_eq!(sig.from, 0);
        assert_eq!(sig.to, 1);
        assert!(sig.green, "empty edge + empty dest = green");
    }

    #[test]
    fn build_signal_duplicate_rejected_other_direction_allowed() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuildSignal { edge: 0, from: 0 }).unwrap();
        assert_eq!(
            apply(&mut s, &Command::BuildSignal { edge: 0, from: 0 }),
            Err(CmdError::SignalAlreadyPlaced(0))
        );
        // Same edge, opposite direction: allowed (one signal per direction).
        apply(&mut s, &Command::BuildSignal { edge: 0, from: 1 }).unwrap();
        assert_eq!(s.signals.len(), 8);
    }

    #[test]
    fn build_signal_non_endpoint_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildSignal { edge: 0, from: 2 }),
            Err(CmdError::NotEdgeEndpoint { edge: 0, node: 2 })
        );
    }

    #[test]
    fn build_signal_unknown_edge_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildSignal { edge: 99, from: 0 }),
            Err(CmdError::UnknownEdge(99))
        );
    }

    #[test]
    fn build_signal_rejected_when_broke() {
        let mut s = SimState::new_game();
        s.money = 40;
        assert_eq!(
            apply(&mut s, &Command::BuildSignal { edge: 0, from: 0 }),
            Err(CmdError::InsufficientFunds { cost: 50, have: 40 })
        );
        assert_eq!(s.signals.len(), 6, "no signal on a rejected build");
    }

    #[test]
    fn set_route_rejects_empty() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        assert_eq!(
            apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![] }),
            Err(CmdError::EmptyRoute)
        );
    }

    #[test]
    fn set_route_rejects_wrong_start() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        assert_eq!(
            apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![1, 2] }),
            Err(CmdError::RouteWrongStart { node: 1, train_at: 0 })
        );
        assert!(s.trains[0].route.is_empty(), "rejected route must not change the train");
    }

    #[test]
    fn set_route_rejects_disconnected() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        // No edge connects node 0 to node 12.
        assert_eq!(
            apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![0, 12] }),
            Err(CmdError::RouteDisconnected { from: 0, to: 12 })
        );
    }

    #[test]
    fn set_route_accepts_connected_route() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![0, 1, 2, 3] })
            .unwrap();
        assert_eq!(s.trains[0].route, vec![0, 1, 2, 3]);
        assert_eq!(s.trains[0].route_idx, 1);
    }

    #[test]
    fn bought_train_runs_over_built_track() {
        // Integration: a route that only exists via player-built track.
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        // Build a little 2x1 block east and north of node 5.
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 0 }).unwrap(); // 5-14
        apply(&mut s, &Command::BuildTrack { from: 14, dir: 2 }).unwrap(); // 14-15
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 2 }).unwrap(); // 5-16
        apply(&mut s, &Command::BuildTrack { from: 16, dir: 0 }).unwrap(); // 16-15
        apply(
            &mut s,
            &Command::SetRoute {
                train_id: 1,
                route: vec![0, 1, 2, 3, 4, 5, 14, 15],
            },
        )
        .unwrap();
        for _ in 0..2500 {
            crate::Sim::tick(&mut s);
        }
        // City B (node 3) demands Coal 3 + Timber 1; A supplies both.
        assert!(
            s.score >= 3,
            "train should have delivered over built track, score={}",
            s.score
        );
        assert!(
            s.money > 220,
            "money should have grown from deliveries, money={}",
            s.money
        );
    }

    #[test]
    fn delete_track_removes_and_refunds() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuildTrack { from: 5, dir: 0 }).unwrap(); // 5-14
        assert_eq!(s.money, 480);
        assert_eq!(s.map.edges.len(), 15);
        apply(&mut s, &Command::DeleteTrack { edge: 14 }).unwrap();
        assert_eq!(s.money, 500, "full refund of the 20 track cost");
        assert_eq!(s.map.edges.len(), 14, "edge removed");
        assert!(s.map.edge_between(5, 14).is_none(), "edge gone");
    }

    #[test]
    fn delete_track_unknown_edge_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::DeleteTrack { edge: 999 }),
            Err(CmdError::UnknownEdge(999))
        );
        assert_eq!(s.money, 500, "rejected delete must not change state");
    }

    #[test]
    fn delete_track_rejected_when_occupied() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![0, 1, 2, 3] })
            .unwrap();
        for _ in 0..50 {
            crate::Sim::tick(&mut s);
        }
        // The train is now mid-edge 0 (node 0 -> 1).
        assert!(s.trains[0].edge == Some(0), "train on edge 0");
        assert_eq!(
            apply(&mut s, &Command::DeleteTrack { edge: 0 }),
            Err(CmdError::EdgeOccupied(0))
        );
        assert_eq!(s.map.edges.len(), 14, "edge not removed on rejection");
    }

    #[test]
    fn delete_signal_removes_and_refunds() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuildSignal { edge: 0, from: 0 }).unwrap();
        assert_eq!(s.money, 450);
        assert_eq!(s.signals.len(), 7);
        apply(&mut s, &Command::DeleteSignal { signal: 6 }).unwrap();
        assert_eq!(s.money, 500, "full refund of the 50 signal cost");
        assert_eq!(s.signals.len(), 6, "signal removed");
    }

    #[test]
    fn delete_signal_unknown_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::DeleteSignal { signal: 999 }),
            Err(CmdError::UnknownSignal(999))
        );
        assert_eq!(s.money, 500, "rejected delete must not change state");
    }

    #[test]
    fn delete_train_removes_and_refunds() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        assert_eq!(s.money, 300);
        assert_eq!(s.trains.len(), 1);
        // The train is stopped at node 0 (no route yet) -> deletable.
        apply(&mut s, &Command::DeleteTrain { train: 1 }).unwrap();
        assert_eq!(s.money, 500, "full refund of the 200 train cost");
        assert_eq!(s.trains.len(), 0, "train removed");
    }

    #[test]
    fn delete_train_rejected_when_moving() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::SetRoute { train_id: 1, route: vec![0, 1, 2, 3] })
            .unwrap();
        for _ in 0..50 {
            crate::Sim::tick(&mut s);
        }
        assert!(s.trains[0].edge.is_some(), "train is moving");
        assert_eq!(
            apply(&mut s, &Command::DeleteTrain { train: 1 }),
            Err(CmdError::TrainMoving(1))
        );
        assert_eq!(s.trains.len(), 1, "train not removed on rejection");
    }

    #[test]
    fn delete_train_unknown_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::DeleteTrain { train: 999 }),
            Err(CmdError::UnknownTrain(999))
        );
        assert_eq!(s.money, 500, "rejected delete must not change state");
    }
}
