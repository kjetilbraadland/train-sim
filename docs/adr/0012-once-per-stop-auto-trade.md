# 0012 — Auto-trade is once-per-stop, gated by `traded_at`

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
Auto-trade (ADR 0006) must happen once per stop, not every tick while a
train is idle — otherwise a train sitting at a station would re-unload /
re-load (and re-earn) every tick. A further bug: a train **bought at a
station** (or the pre-M1 starter train) never "arrived," so it departed
empty and scoring never started.

## Decision
A train trades at a station when it either **arrives** there
(`just_arrived`) **or** is stopped there and has not yet traded at that
node. `Train.traded_at: Option<u32>` records the last node traded at;
arrival resets it, so the next stop trades again. A bought train at a
station therefore loads on its first tick, and an idle train trades at
most once per stop.

## Consequences
- Scoring starts as soon as a bought train is routed — no "departs empty"
  bug.
- No per-tick re-earning while idle (deterministic, bounded scoring).
- Cost: one extra field on `Train`; the trade gate is slightly more
  complex than "arrived only."
