# Train Logistics Puzzle — DESIGN

**Owner:** Dampkjetil. **Status:** v1 draft (numbers approved 2026-11-22).
The cron implements *strictly against this file*. When the design is unclear,
the cron flags it in the report instead of guessing. Design calls are the
owner's; the engineer does not make them.

## 1. Vision

A Factorio-style, open-ended train logistics game. The player is a **system
designer**: they build a rail network from an empty map — track, signals,
sidings, trains — and the trains run automatically. The player optimizes
throughput. No win state in the sandbox; a separate **puzzle level** mode
gives a concrete target.

- Real-time: trains run continuously; the player can pause / speed up and
  build while running.
- Grid-based placement (Factorio-style): track / signals / sidings snap to
  grid cells.
- The player builds **everything**: track segments, signals, sidings, trains.
  No starter network.

## 2. Core loop

1. Earn money by delivering goods between cities.
2. Spend money on track / signals / sidings / trains.
3. Re-route and re-signal to raise throughput.
4. (Puzzle mode) hit the target before the timer.

## 3. Economy (v1 — approved)

| Item          | Cost |
|---------------|------|
| Start funds   | 500  |
| Delivery      | +10 per unit |
| Train         | 200  |
| Track segment | 20   |
| Signal        | 50   |
| Siding        | 80   |

A first train + minimal track ≈ 240, leaving room to build out. Numbers are
tunable; this is the v1 baseline.

## 4. Mechanics

- **Trains** move along track at a constant speed, following a route the
  player assigns. They auto-load from a city's supply and auto-unload into
  its demand (Factorio-style: machines run themselves).
- **Signals** are automatic block sections: red when the section or the
  destination node is occupied, green otherwise. The player places them and
  chooses which section/direction they protect.
- **Sidings** let a train pull off the main line so another can pass — the
  player's main throughput tool.
- **Cities** have fixed supply (produces) and demand (wants) per good.
  Goods: Coal, Steel, Food, Timber.

## 5. Puzzle level (M3)

- Deliver **100 units** of goods before the timer runs out.
- Fixed map, target score, timer, win/lose.
- The sandbox is the engine; the puzzle is a constrained instance of it.

## 6. Architecture (unchanged)

- **engine** (Rust crate, pure): `SimState` + `Sim::tick`. No I/O, no wall
  clock, seeded RNG. Deterministic: same state + same commands = same result.
- **server** (axum): 10 Hz tick loop, WebSocket snapshots + commands, static
  client. Owns no rules.
- **client** (three.js): renders snapshots, sends commands. Owns no rules.

## 7. Phase plan (each phase = one or a few reviewable branches)

- **M1 — Build system:** place track / signals / sidings / trains; costs;
  money; a train bought and routed actually moves and trades.
- **M2 — Sandbox mode:** the open-ended build loop is playable and fun to
  optimize; economy depth.
- **M3 — Puzzle level:** timer + 100-unit target + win/lose, on a fixed map.

## 8. Open questions (flag, don't guess)

- [ ] Grid cell size / world scale (meters per cell).
- [ ] Do trains have a fixed speed, or a speed the player can raise (cost)?
  *Flagged 2026-11-23 (M1 BuyTrain): bought trains use the fixed base speed
  15 m/s (same as the pre-M1 starter train) — minimal assumption, not a
  design call.*
- [ ] Siding length: how many carriages / how long must it be?
- [ ] Can the player delete placed objects (and get a refund)?
- [ ] City supply/demand: fixed per city, or do they vary over time?
- [ ] Max trains / max track (any cap), or unbounded?
  *Flagged 2026-11-23 (M1 BuyTrain): no cap — trains are unbounded, limited
  only by money — minimal assumption, not a design call.*

> When an open question blocks work, the cron notes it here and in the daily
> report, and proceeds with the minimal reasonable assumption *explicitly
> labeled* — it never silently invents a design.
