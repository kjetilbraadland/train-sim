# 0004 — Grid-based (Factorio-style) placement

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
The player builds the entire rail network from an empty map. Free-form
node linking is more "train-sim" but is harder to reason about, validate,
and render cleanly as the network grows. The owner wants the Factorio
feel: snap-to-grid construction.

## Decision
Placement is **grid-based**. Track segments, signals, and sidings snap to
grid cells; a track segment occupies the edge between two adjacent (or
diagonal) cells. The map is a grid of cells; the engine's node/edge model
is generated from grid placements. The client renders the grid and snaps
placements to it.

## Consequences
- Constructions are easy to validate (no overlapping/invalid geometry), easy
  to reason about, and render cleanly at any scale.
- The engine's underlying node/edge graph is an implementation detail
  derived from the grid; the *canonical* representation of a build is the
  set of placed grid objects.
- Cost: less organic/curved track than free-form; grid cell size is a
  tuning parameter (open question in DESIGN.md §8).
