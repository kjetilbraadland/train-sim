# GLOSSARY

Shared vocabulary for train-sim. When a word here and the code disagree,
the code wins and this file gets fixed — but flag it. When a new term is
needed, add it here (and, if it's a decision, an ADR) before using it in
code or reports.

## Simulation core

- **Tick** — one discrete step of the simulation (100 ms of sim time at 1x
  speed). The engine advances by ticks; there is no wall clock.
- **SimState** — the complete, serializable game state. The single source of
  truth; the server broadcasts it each tick.
- **Sim::tick** — the only place game rules live. A pure function over
  `SimState`.
- **Deterministic** — same initial state + same command sequence ⇒ same
  result. No wall clock, no unseeded RNG.
- **Command** — a player intent the client sends to the server (tagged enum:
  `NewGame`, `SetPaused`, `SetSpeed`, `SetRoute`, `Load`, `Unload`,
  `BuyTrain` + build commands in M1). Applied by `command::apply` in the
  engine — the server owns no rules.
- **CmdError** — the error a rejected command returns (e.g.
  `InsufficientFunds`, `UnknownNode`, `UnknownTrain`, `UnsupportedMap`). A
  rejected command leaves the state unchanged (ADR 0011).
- **New game** — `SimState::new_game()`: the default map, starting funds
  (500), no trains; the player buys the first train (ADR 0010).
- **Snapshot** — the full JSON `SimState` broadcast server→client each tick.

## Map & network

- **Grid cell** — one cell of the placement grid. All build objects snap to
  cells (ADR 0004).
- **Node** — a point in the track graph (a junction, station, or signal
  position).
- **Edge** — a track segment between two nodes, with a length in metres.
- **Section** — the portion of track a signal protects. In the current model
  a section is one edge; the term is used so we can later model multi-edge
  sections without renaming.
- **Track segment** — a placed piece of track (costs 20). In the grid model
  it is the edge between two adjacent cells.
- **Siding** — a short branch track a train can pull onto so another can
  pass (costs 80). The player's main throughput tool.
- **City** — a station hub with a fixed **supply** (goods it produces) and
  **demand** (goods it wants).

## Signals

- **Signal** — a placed object (costs 50) protecting one direction of one
  section.
- **Block section** — a section that may hold at most one train. Signals
  enforce this (ADR 0005).
- **Green / Red** — signal aspects. Green: section free and destination
  node unoccupied. Red: otherwise. A train already on the section is never
  stopped by its own signal.

## Trains & goods

- **Train** — a moving agent with a carriage capacity, a **route**, and
  **cargo**. Costs 200.
- **Route** — the ordered list of nodes a train travels. `route_idx` points
  at the *next* node; `route[0]` is the starting node.
- **Cargo** — the goods in a train's carriages, bounded by capacity.
- **Goods** — the tradeable items: Coal, Steel, Food, Timber.
- **Supply** — goods a city produces (per-tick capacity).
- **Demand** — goods a city wants (per-tick capacity).
- **Auto-trade** — on arrival at a station, a train unloads cargo matching
  the city's demand and loads from its supply up to capacity (ADR 0006).
  Happens once per stop, not every tick (gated by `traded_at`,
  ADR 0012); a train bought at a station trades on its first tick.

## Economy

- **Money** — the single currency. Earned from deliveries (+10/unit), spent
  on track/signals/sidings/trains (ADR 0007). No train cap: train count
  is limited only by money (ADR 0014); train speed is fixed (ADR 0013).
- **Throughput** — units delivered per unit time; the metric the player
  optimizes.

## Game modes & phases

- **Sandbox mode** — the open-ended build loop: build a network, optimize
  throughput, no win state.
- **Puzzle level** — a constrained instance of the sandbox: fixed map, a
  target (deliver 100 units), a timer, win/lose.
- **M1 — Build system** — place track/signals/sidings/trains; costs; money;
  a bought + routed train moves and trades.
- **M2 — Sandbox** — the open-ended build loop is playable and fun to
  optimize; economy depth.
- **M3 — Puzzle level** — timer + 100-unit target + win/lose on a fixed map.

## People & process

- **Owner** — Dampkjetil. The design authority and merge gate.
- **Cron** — the daily improvement job. An engineer, not a game designer
  (ADR 0008).
- **DESIGN.md** — the source of truth for game design. Owned by the owner.
- **RUNBOOK.md** — the cron's step-by-step process.
- **ADR** — Architecture Decision Record (see `docs/adr/`).
