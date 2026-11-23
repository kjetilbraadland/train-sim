# 0014 — No train cap (train count limited only by money)

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
Open question (DESIGN.md §8): is there a maximum number of trains? A hard
cap would be an arbitrary constraint; the natural limit is the economy.

## Decision
**Owner decision (2026-11-23):** there is **no train cap**. The number of
trains is unbounded — limited only by money (each train costs 200).
`BuyTrain` assigns a deterministic id (one past the highest existing id)
and is gated only by funds and a valid node.

## Consequences
- No arbitrary cap; the economy is the sole pacing mechanism.
- Confirms the M1 assumption (no cap in v1) — no rework.
- Cost: a player could in theory buy many trains; the money sink (200 each)
  is the only brake. If performance or balance ever needs a soft cap, it
  is a new ADR.
