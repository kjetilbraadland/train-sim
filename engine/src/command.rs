//! Command application: the single place commands mutate state.
//!
//! Pure and deterministic: no I/O, no wall clock. The server decodes JSON
//! into [`Command`] and calls [`apply`]; it owns no rules of its own.

use crate::state::{SimState, Train};
use crate::Command;

/// Error from a rejected command (insufficient funds, unknown id, ...).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdError {
    InsufficientFunds { cost: u32, have: u32 },
    UnknownNode(u32),
    UnknownTrain(u32),
    UnsupportedMap(String),
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
            let t = state
                .trains
                .iter_mut()
                .find(|t| t.id == *train_id)
                .ok_or(CmdError::UnknownTrain(*train_id))?;
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
}
