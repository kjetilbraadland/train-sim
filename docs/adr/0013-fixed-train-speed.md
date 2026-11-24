# 0013 — Train speed is fixed (no speed upgrade)

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
Open question (DESIGN.md §8): do trains have a fixed speed, or can the
player raise speed at a cost? A speed upgrade would add a second
progression axis (money → speed) and complicate the economy.

## Decision
**Owner decision (2026-11-23):** train speed is **fixed** (base 15 m/s).
There is no speed upgrade. Throughput is raised by network design (track,
signals, sidings, more trains), not by making trains faster.

## Consequences
- One progression axis (money → network), simpler economy and balance.
- Confirms the M1 assumption (bought trains use 15 m/s) — no rework.
- Cost: no "turbo train" item; if wanted later, it is a new ADR + economy
  tuning (owner).
