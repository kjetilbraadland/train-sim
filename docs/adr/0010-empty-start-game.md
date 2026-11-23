# 0010 — New game starts empty; the player buys the first train

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
Pre-M1, `new_game` seeded a starter train on the default map, so the player
could not do the core loop (earn → buy → build). The design (ADR 0006,
DESIGN.md) is that the player builds *everything* from an empty map.

## Decision
`SimState::new_game()` starts with the default map, **starting funds
(500)**, **no trains**, and one signal per signal-flagged edge. The player
buys the first train (`BuyTrain`) and builds out. There is no starter
train and no starter network.

## Consequences
- The real progression loop (earn → buy → build → optimize) is playable from
  the first tick.
- Matches DESIGN.md "the player builds everything."
- Cost: a fresh game is empty until the first `BuyTrain`; the client must
  surface "buy a train" clearly. `traded_at` (ADR 0012) ensures a freshly
  bought train at a station trades once, not every tick.
