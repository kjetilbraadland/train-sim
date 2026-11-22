# 0006 — Trains auto-load/unload; the player is a system designer

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
Two candidate player loops: (a) an *active operator* who manually loads,
unloads, and routes each train every turn, or (b) a *system designer* who
configures the network once and lets the machines run. The owner chose the
Factorio-style system designer: the player builds and configures, the
trains run themselves.

## Decision
Trains **auto-load** from a city's supply and **auto-unload** into its
demand when they arrive at a station (gated on arrival, so it happens once
per stop, not every tick). The player's skill is **network design and
optimization** — where to build track, where to place signals and sidings,
how to route trains — not micromanaging every load. The player is a system
designer, not an active operator.

## Consequences
- The core loop is *build → run → observe → re-optimize*, matching Factorio.
- The engine's trade logic is automatic and testable; the player never
  issues per-unit load commands in the normal loop.
- Cost: less direct control; if we later want an optional "manual mode",
  that is a new ADR. The starter-train "departs empty" edge case (no load
  at the starting node) is a known bug to fix in M1.
