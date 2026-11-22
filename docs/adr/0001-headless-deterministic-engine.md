# 0001 — Headless, deterministic engine split from graphics

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
The game must be testable and scalable without graphics. We want to verify
game rules (movement, signals, trade, scoring) in fast unit tests, run
replays/benchmarks headlessly, and keep the visual layer swappable. A single
monolith that mixes rules, I/O, and rendering would make none of that
reliable.

## Decision
The engine is a pure Rust crate (`train-sim-engine`) with **no I/O, no
network, no graphics, no wall clock, and no unseeded RNG**. The entire game
state is a value (`SimState`); advancing the game is `Sim::tick`. The server
(axum) owns the tick loop and I/O; the three.js client owns rendering. The
engine is **deterministic**: the same initial state + the same command
sequence always yields the same state. Randomness (e.g. demand generation)
comes from a seeded RNG stored in state, so replays are exact.

## Consequences
- Rules are unit-testable and replayable; we assert determinism in a test.
- The engine cannot do I/O, so all side effects live in the server.
- A headless AI/benchmark can drive the engine directly.
- Cost: the engine is more constrained (no `std::time`, no `rand::thread_rng`);
  any new mechanic must be expressible as a pure state transition.
