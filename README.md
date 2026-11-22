# Train Logistics Puzzle

A train logistics puzzle: optimize loading/unloading of goods, trade routes
between cities, and rail signals to keep tracks flowing.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ engine (Rust crate, pure)                                  │
│  - SimState: complete game state, serde-serializable       │
│  - Sim::tick: the only place rules live                   │
│  - No I/O, no network, no wall clock, no global RNG       │
│  - Deterministic: same state + same commands = same result │
├─────────────────────────────────────────────────────────────┤
│ server (Rust, axum)                                       │
│  - 10 Hz tick loop, broadcasts JSON snapshots             │
│  - WebSocket /ws: commands in, snapshots out              │
│  - Serves the three.js client (static/)                   │
│  - Owns NO game rules                                    │
├─────────────────────────────────────────────────────────────┤
│ client (three.js, static/)                                │
│  - Renders snapshots, sends commands                      │
│  - Owns NO game rules                                    │
└─────────────────────────────────────────────────────────────┘
```

The split means the engine is testable and scalable without graphics:
headless, replayable, and the same code runs in unit tests, the server,
and (potentially) a headless AI benchmark.

## Build & run

```sh
cargo build --release
# server (PORT env, default 8771)
PORT=8771 ./target/release/train-sim-server
# open http://<host>:8771
```

## Tests

```sh
cargo test
```

Engine tests cover: map well-formedness, train movement, signal blocking,
auto-trade scoring, and determinism (identical inputs → identical state).

## Documentation

- **`DESIGN.md`** — the source of truth for game design (owner: Dampkjetil).
- **`GLOSSARY.md`** — shared vocabulary; use these terms in code and reports.
- **`docs/adr/`** — Architecture Decision Records (the *why* behind decisions).
- **`RUNBOOK.md`** — the daily cron's step-by-step process.

## Commands (WebSocket JSON)

```json
{"type":"NewGame","mapId":null}
{"type":"SetPaused","paused":true}
{"type":"SetSpeed","multiplier":2.0}
{"type":"SetRoute","trainId":0,"route":[0,1,2,3]}
{"type":"Load","trainId":0,"good":"Coal","qty":4}
{"type":"Unload","trainId":0,"good":"Coal","qty":4}
```

## Roadmap (phases — see DESIGN.md)

- [x] Deterministic engine core (map, trains, signals, auto-trade)
- [x] axum server + WebSocket snapshots
- [x] three.js client (map, trains, signals, HUD)
- [ ] **M1 — Build system:** place track/signals/sidings/trains; costs; money
- [ ] **M2 — Sandbox:** open-ended build loop is playable and fun to optimize
- [ ] **M3 — Puzzle level:** timer + deliver-100 target + win/lose
- [ ] Demand generation from seeded RNG
- [ ] Score/leaderboard, win conditions
