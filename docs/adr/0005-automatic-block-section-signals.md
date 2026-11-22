# 0005 — Signals are automatic block sections

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
Trains share track; without protection two trains can occupy the same
section. We need a simple, robust, deterministic rule for when a train may
enter a section. The player places signals and chooses which
section/direction they protect.

## Decision
A **signal** protects one direction of one section (edge). It is an
**automatic block section**: it is **green** iff the section it protects is
free of trains **and** the destination node is not occupied by a train;
otherwise it is **red**. A train may enter the section only while the
signal is green. A train **already on** the section is never stopped by it
(it has passed the signal) — this prevents a livelock where the occupying
train keeps its own signal red.

## Consequences
- Simple, deterministic, and easy to test (signal-blocks-train is a unit
  test).
- The player's throughput tool is *where* to place signals and sidings, not
  manual aspect control.
- Cost: no manual override in v1; if we later want timed/interlocking
  signals, that is a new ADR.
