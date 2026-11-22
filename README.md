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

## Commands (WebSocket JSON)

```json
{"type":"newGame","mapId":null}
{"type":"setPaused","paused":true}
{"type":"setSpeed","multiplier":2.0}
{"type":"setRoute","trainId":0,"route":[0,1,2,3]}
{"type":"load","trainId":0,"good":"coal","qty":4}
{"type":"unload","trainId":0,"good":"coal","qty":4}
```

## Roadmap

- [x] Deterministic engine core (map, trains, signals, auto-trade)
- [x] axum server + WebSocket snapshots
- [x] three.js client (map, trains, signals, HUD)
- [ ] Manual load/unload UI (currently auto-trade)
- [ ] Route planning UI (click nodes to build a route)
- [ ] Multiple trains + train purchase
- [ ] Signal timing as a player tool (block sections)
- [ ] Demand generation from seeded RNG
- [ ] Score/leaderboard, win conditions
