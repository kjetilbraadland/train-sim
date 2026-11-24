# 0015 — Grid build commands: BuildTrack / BuildSignal

- Status: Accepted
- Date: 2026-11-24
- Supersedes: —

## Context

M1 (DESIGN.md) requires the player to build track and place signals before
trains can be routed. ADR 0004 chose grid-based (Factorio-style) placement,
but the concrete command interface was left open. The engine must decide
how a grid placement is encoded, how builds snap to the existing map
(the default map is not perfectly grid-aligned: the cross-links are
350 m, not 300 m), and how signals attach to edges.

## Decision

The player builds **from an existing node into one of 8 grid directions**
(0=E 1=W 2=N 3=S 4=NE 5=NW 6=SE 7=SW) with a 300 m cell size — a labeled
assumption matching the default map's dominant segment spacing (flagged in
DESIGN.md §8). `BuildTrack { from, dir }` computes the target cell as
`from.pos + dir_step`; if a node exists within 25 m of that cell it
connects to it, otherwise a new node is created there, and an edge is
added between the two (length = actual distance, `has_signal = false`).
Building onto an edge that already exists is rejected (`DuplicateEdge`).
`BuildSignal { edge, from }` places one signal per edge direction: `from`
must be an endpoint of the edge, `to` is the other endpoint, and a second
signal in the same direction on the same edge is rejected
(`SignalAlreadyPlaced`). The initial aspect follows the block-section rule
(green iff the edge is unoccupied and the destination node is unoccupied,
ADR 0005); `update_signals` recomputes it every tick. Costs are flat:
track 20, signal 50 (ADR 0007). `SetRoute` is now validated before
mutating (ADR 0011): the route must be non-empty, must start at the node
the train is at, and every consecutive node pair must be connected by an
edge; a rejected route leaves state unchanged.

## Consequences

- Builds always anchor to the existing graph: no free-form placement, and
  no grid coordinate system to store in state. The map stays a node/edge
  graph, so the snapshot-size argument of ADR 0004 holds.
- The 25 m snap radius tolerates the default map's 350 m cross-links: a
  grid step from a cross-link node lands 50 m off the lattice and creates
  a new node rather than merging. The grid is best-effort, not a hard
  lattice.
- Diagonal builds are 300√2 m — the grid trades arbitrary angles for a
  cheap spec (one node + one direction). That is the Factorio feel M1
  wants.
- Open questions, flagged not decided (DESIGN.md §8): deletion of placed
  track/signals with refund, sidings (cost already reserved in ADR 0007),
  the 300 m cell size, and a possible track cap.
- The client renders ghost previews for the 8 directions from the selected
  node (M1 client work, ADR 0002).
