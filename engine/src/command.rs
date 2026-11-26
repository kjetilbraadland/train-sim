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
    /// Siding: a siding already branches from `node` (in any direction).
    SidingAlreadyPlaced(u32),
    /// Siding: a train is currently in the siding at `node` (can't divert
    /// another in while one is parked).
    SidingOccupied(u32),
    /// Siding: no siding is placed at `node`.
    NoSidingAtNode(u32),
    /// Siding: the train is not stopped at `node` (must be idle on the
    /// main line to divert).
    TrainNotAtNode { train: u32, at: u32, want: u32 },
    /// Siding: the train is not in a siding.
    NotInSiding(u32),
    /// Route: empty route.
    EmptyRoute,
    /// Route: route[0] is not the node the train is at.
    RouteWrongStart { node: u32, train_at: u32 },
    /// Route: no edge connects two consecutive route nodes.
    RouteDisconnected { from: u32, to: u32 },
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
            CmdError::SidingAlreadyPlaced(n) => {
                write!(f, "a siding already branches from node {n}")
            }
            CmdError::SidingOccupied(n) => {
                write!(f, "a train is currently in the siding at node {n}")
            }
            CmdError::NoSidingAtNode(n) => {
                write!(f, "no siding is placed at node {n}")
            }
            CmdError::TrainNotAtNode { train, at, want } => {
                write!(f, "train {train} is at node {at}, not stopped at node {want}")
            }
            CmdError::NotInSiding(n) => {
                write!(f, "train {n} is not in a siding")
            }
            CmdError::EmptyRoute => write!(f, "route must not be empty"),
            CmdError::RouteWrongStart { node, train_at } => {
                write!(f, "route starts at node {node} but the train is at node {train_at}")
            }
            CmdError::RouteDisconnected { from, to } => {
                write!(f, "no track between nodes {from} and {to}")
            }
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
                in_siding: None,
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
                is_siding: false,
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
            // `update_signals` recomputes it every tick). A parked train
            // (in a siding) does not occupy the main edge, but does occupy
            // its main-line node (ADR 0016).
            let edge_occupied = state
                .trains
                .iter()
                .any(|t| t.edge == Some(*edge) && t.in_siding.is_none());
            let dest_occupied = state.trains.iter().any(|t| {
                (t.edge.is_none() && t.node == to && t.in_siding.is_none())
                    || t.in_siding == Some(to)
            });
            state.signals.push(Signal {
                id,
                edge: *edge,
                from: *from,
                to,
                green: !edge_occupied && !dest_occupied,
            });
        }
        Command::BuildSiding { node, dir } => {
            // Validate before mutating (ADR 0011).
            let step = crate::map::grid_dir_step(*dir)
                .ok_or(CmdError::InvalidDir(*dir))?;
            let anchor = state
                .map
                .node(*node)
                .ok_or(CmdError::UnknownNode(*node))?;
            // One siding per main-line node (in any direction).
            if state.sidings.iter().any(|s| s.node == *node) {
                return Err(CmdError::SidingAlreadyPlaced(*node));
            }
            if state.money < crate::economy::SIDING_COST {
                return Err(CmdError::InsufficientFunds {
                    cost: crate::economy::SIDING_COST,
                    have: state.money,
                });
            }
            // Target cell: snap to an existing node if close, else create one.
            let target_pos = [
                anchor.pos[0] + step[0],
                anchor.pos[1] + step[1],
            ];
            let end_node = match crate::map::node_near(&state.map, target_pos, NODE_MATCH_EPS) {
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
            // The siding branch must not overlap an existing (main or siding)
            // edge between the anchor and the end node.
            let overlap = state.map.edges.iter().any(|e| {
                (e.a == *node && e.b == end_node) || (e.b == *node && e.a == end_node)
            });
            if overlap {
                return Err(CmdError::DuplicateEdge(*node, end_node));
            }
            state.money -= crate::economy::SIDING_COST;
            let length = state
                .map
                .node_distance(*node, end_node)
                .unwrap_or(crate::map::GRID_STEP);
            let edge_id = state.map.edges.iter().map(|e| e.id).max().unwrap_or(0) + 1;
            state.map.edges.push(Edge {
                id: edge_id,
                a: *node,
                b: end_node,
                length,
                has_signal: false,
                is_siding: true,
            });
            let siding_id = state.sidings.iter().map(|s| s.id).max().unwrap_or(0) + 1;
            state.sidings.push(crate::map::Siding {
                id: siding_id,
                edge: edge_id,
                node: *node,
                end_node,
            });
        }
        Command::EnterSiding { train_id, node } => {
            // Validate before mutating (ADR 0011).
            let t = state
                .trains
                .iter()
                .find(|t| t.id == *train_id)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
            if t.node != *node || t.edge.is_some() {
                return Err(CmdError::TrainNotAtNode {
                    train: *train_id,
                    at: t.node,
                    want: *node,
                });
            }
            if t.in_siding.is_some() {
                return Err(CmdError::NotInSiding(*train_id)); // already parked
            }
            // The siding at `node` must exist and be free.
            let siding = state.sidings.iter().find(|s| s.node == *node);
            let Some(siding) = siding else {
                return Err(CmdError::NoSidingAtNode(*node));
            };
            if state.trains.iter().any(|x| x.in_siding == Some(*node)) {
                return Err(CmdError::SidingOccupied(*node));
            }
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .expect("train validated above");
            t.in_siding = Some(*node);
            t.edge = Some(siding.edge);
            t.progress = 1.0; // parked at the siding's dead-end
        }
        Command::ExitSiding { train_id, node } => {
            // Validate before mutating (ADR 0011).
            let t = state
                .trains
                .iter()
                .find(|t| t.id == *train_id)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
            if t.in_siding != Some(*node) {
                return Err(CmdError::NotInSiding(*train_id));
            }
            // The main node must be free of other trains before the parked
            // train re-enters it, or it would block the line again. The
            // normal block-section signal check (in `move_trains`) handles
            // holding the train if a downstream edge is red.
            let main_occupied = state.trains.iter().any(|x| {
                x.id != *train_id
                    && x.edge.is_none()
                    && x.node == *node
                    && x.in_siding.is_none()
            });
            if main_occupied {
                return Err(CmdError::SidingOccupied(*node));
            }
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .expect("train validated above");
            t.in_siding = None;
            t.edge = None;
            t.progress = 0.0;
            t.node = *node;
            t.held_by_signal = false;
            // `traded_at` is deliberately left untouched: if the train already
            // traded at this station before parking, it does not re-trade on
            // re-entry (no park-and-re-enter exploit). If it had not traded
            // here, it trades once now, as a normal arrival.
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sim;
    use crate::state::{Goods, Signal};

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
    fn build_siding_creates_branch_and_costs() {
        let mut s = SimState::new_game();
        assert_eq!(s.money, 500);
        // Node 2 (750,1150) north 300 m -> (750,1450): no node there yet.
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        assert_eq!(s.money, 420, "siding costs 80");
        assert_eq!(s.sidings.len(), 1);
        assert_eq!(s.sidings[0].node, 2);
        // A new node + a new (siding) edge were created.
        assert_eq!(s.map.nodes.len(), 15);
        assert_eq!(s.map.edges.len(), 15);
        let e = s.map.edge(s.sidings[0].edge).expect("siding edge exists");
        assert!(e.is_siding, "the branch edge is flagged as a siding");
    }

    #[test]
    fn build_siding_duplicate_node_rejected() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        // A second siding off the same node (different direction) is rejected.
        assert_eq!(
            apply(&mut s, &Command::BuildSiding { node: 2, dir: 0 }),
            Err(CmdError::SidingAlreadyPlaced(2))
        );
        assert_eq!(s.sidings.len(), 1, "no second siding on a rejected build");
    }

    #[test]
    fn build_siding_rejected_when_broke() {
        let mut s = SimState::new_game();
        s.money = 50;
        assert_eq!(
            apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }),
            Err(CmdError::InsufficientFunds { cost: 80, have: 50 })
        );
        assert_eq!(s.sidings.len(), 0);
    }

    #[test]
    fn build_siding_unknown_node_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildSiding { node: 999, dir: 0 }),
            Err(CmdError::UnknownNode(999))
        );
    }

    #[test]
    fn build_siding_invalid_dir_rejected() {
        let mut s = SimState::new_game();
        assert_eq!(
            apply(&mut s, &Command::BuildSiding { node: 2, dir: 9 }),
            Err(CmdError::InvalidDir(9))
        );
    }

    #[test]
    fn enter_and_exit_siding() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        // Divert the (stopped) train onto the siding.
        apply(&mut s, &Command::EnterSiding { train_id: 1, node: 2 }).unwrap();
        assert_eq!(s.trains[0].in_siding, Some(2));
        // A parked train is off the main line: it does not block the node.
        assert!(!Sim::node_occupied(&s, 2));
        // Exit back onto the main line.
        apply(&mut s, &Command::ExitSiding { train_id: 1, node: 2 }).unwrap();
        assert_eq!(s.trains[0].in_siding, None);
        assert_eq!(s.trains[0].node, 2);
        assert!(s.trains[0].edge.is_none());
    }

    #[test]
    fn enter_siding_rejected_when_occupied() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        apply(&mut s, &Command::EnterSiding { train_id: 1, node: 2 }).unwrap();
        // A second train cannot enter the same (occupied) siding.
        assert_eq!(
            apply(&mut s, &Command::EnterSiding { train_id: 2, node: 2 }),
            Err(CmdError::SidingOccupied(2))
        );
    }

    #[test]
    fn enter_siding_rejected_no_siding_at_node() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        assert_eq!(
            apply(&mut s, &Command::EnterSiding { train_id: 1, node: 2 }),
            Err(CmdError::NoSidingAtNode(2))
        );
    }

    #[test]
    fn enter_siding_rejected_when_train_not_stopped_there() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        // The train is at node 0, not node 2.
        assert_eq!(
            apply(&mut s, &Command::EnterSiding { train_id: 1, node: 2 }),
            Err(CmdError::TrainNotAtNode { train: 1, at: 0, want: 2 })
        );
    }

    #[test]
    fn exit_siding_rejected_when_not_in_siding() {
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        assert_eq!(
            apply(&mut s, &Command::ExitSiding { train_id: 1, node: 2 }),
            Err(CmdError::NotInSiding(1))
        );
    }

    #[test]
    fn parked_train_does_not_block_passer() {
        // The core throughput property: a train parked in a siding must not
        // block another train passing the junction node.
        let mut s = SimState::new_game();
        // Siding off node 2 (the north line's middle).
        apply(&mut s, &Command::BuildSiding { node: 2, dir: 2 }).unwrap();
        // Train 1: parked in the siding at node 2.
        apply(&mut s, &Command::BuyTrain { node: 2 }).unwrap();
        apply(&mut s, &Command::EnterSiding { train_id: 1, node: 2 }).unwrap();
        // Train 2: comes in from node 0 toward node 2.
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        apply(&mut s, &Command::SetRoute { train_id: 2, route: vec![0, 1, 2] }).unwrap();
        // A signal on edge 1 (1 -> 2) would be red if the parked train
        // occupied node 2; it must be green so train 2 can proceed.
        s.signals.push(Signal {
            id: 99, edge: 1, from: 1, to: 2, green: true,
        });
        for _ in 0..700 {
            crate::Sim::tick(&mut s);
        }
        // The parked train must still be parked (it does not move on its own).
        assert_eq!(s.trains[0].in_siding, Some(2), "parked train stays parked");
        // The passing train must have reached node 2 (not blocked by the parker).
        assert_eq!(
            s.trains[1].node, 2,
            "passing train should have reached node 2 despite the parked train"
        );
        assert!(s.trains[1].edge.is_none());
    }

    #[test]
    fn parked_train_does_not_trade_while_parked() {
        // A parked train sits on the spur; it must not re-trade while parked.
        let mut s = SimState::new_game();
        apply(&mut s, &Command::BuyTrain { node: 0 }).unwrap();
        // City A (node 0) supplies Coal; load some, then park it.
        apply(&mut s, &Command::BuildSiding { node: 0, dir: 2 }).unwrap();
        apply(&mut s, &Command::EnterSiding { train_id: 1, node: 0 }).unwrap();
        // Parked: edge is the siding branch, so auto_trade's "stopped at a
        // station" eligibility (edge.is_none()) is false -> no re-trade.
        let before = s.score;
        for _ in 0..50 {
            crate::Sim::tick(&mut s);
        }
        assert_eq!(s.score, before, "a parked train must not trade while parked");
    }
}
