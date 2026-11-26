# 0016 — Sidings: a short dead-end branch to let a train pass

- Status: Accepted
- Date: 2026-11-26
- Supersedes: —

## Context

DESIGN.md §4 names **sidings** as "the player's main throughput tool," and
`economy.rs` already reserves the cost (80, ADR 0007) — but the mechanic was
unimplemented. Without it, the core loop's "re-route and re-signal to raise
throughput" is impossible: two trains on the same section in opposite
directions deadlock, because a block-section signal turns red whenever the
section or the destination node is occupied, and there is no way to get a
train *off* the main line.

## Decision

A **siding** is a short dead-end branch track off one main-line node.

- `BuildSiding { node, dir }` (cost 80, ADR 0007) creates a siding branch
  edge from `node` one grid cell (300 m) in direction `dir` (0=E 1=W 2=N 3=S
  4=NE 5=NW 6=SE 7=SW). If no node exists in the target cell, one is created
  (same snap rule as `BuildTrack`, ADR 0015). The branch edge is flagged
  `is_siding = true`. One siding per main-line node (a second is rejected,
  `SidingAlreadyPlaced`). The branch must not overlap an existing edge.
- `EnterSiding { train_id, node }` diverts a *stopped* train at `node` onto
  the siding there: the train parks at the branch's dead-end
  (`in_siding = Some(node)`, `edge = the branch`, `progress = 1.0`). One train
  per siding (`SidingOccupied`). The train must be idle on the main line
  (`TrainNotAtNode` otherwise) and a siding must exist at the node
  (`NoSidingAtNode`).
- `ExitSiding { train_id, node }` sends a parked train back onto the main
  line at `node`, provided no *other* train is stopped there. The normal
  block-section signal check in `move_trains` then holds it if a downstream
  edge is red.
- **A parked train is off the main line.** It does not move on its own
  (`move_trains` skips it), it does not re-trade while parked, and — the key
  throughput property — it does **not** occupy its main-line node for signal
  purposes, so another train can pass the junction. `edge_between` ignores
  siding branch edges so routing resolves to the main edge, not the spur.

## Consequences

- The player can now stage a train off the line and let another pass, which
  is the throughput lever DESIGN.md §4/§7 (M2) calls for.
- Siding edges are a first-class, flagged part of the map; the client renders
  them as a distinct spur with a marker (orange when free, green when a train
  is parked).
- Open question, flagged not decided (DESIGN.md §8): **siding length / how
  many carriages it must hold** — a siding is currently one grid cell (300 m)
  and holds one train (capacity 8); whether it should be longer or multi-train
  is the owner's call.
