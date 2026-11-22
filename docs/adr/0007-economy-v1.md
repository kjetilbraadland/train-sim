# 0007 — Economy v1 numbers

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
The progression loop is: deliver goods → earn money → buy track/signals/
sidings/trains → raise throughput. We need a concrete, tunable baseline so
the build loop is playable and money *matters* (not trivially abundant,
not impossibly scarce).

## Decision
**Economy v1** (owner-approved baseline; numbers are tunable, this is the
starting point):

| Item            | Cost |
|-----------------|------|
| Start funds     | 500  |
| Delivery        | +10 per unit |
| Train           | 200  |
| Track segment   | 20   |
| Signal          | 50   |
| Siding          | 80   |

A first train + minimal track ≈ 240, leaving room to build out.

## Consequences
- Clear, simple money sink/source; easy to reason about in tests.
- Numbers are a *baseline*, not a law: tuning is a design change (owner),
  not an engineering decision. If balance is off, the owner adjusts these
  and notes it here.
- Cost: no resource diversity in v1 (single currency); adding materials
  (e.g. steel to build track) is a future ADR.
