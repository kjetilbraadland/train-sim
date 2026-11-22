# 0002 — Rust engine + axum server + three.js client

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
We need a performant, safe, single-language core that compiles to a small
server binary, plus a web frontend that can render a live, growing network.
The owner chose Rust. The engine must share its data model with the client
without a heavyweight binding layer.

## Decision
- **Engine:** pure Rust crate (see ADR 0001).
- **Server:** Rust `axum`, a 10 Hz tick loop, WebSocket at `/ws`, static
  client served from `static/`. The server owns no game rules.
- **Client:** three.js (ES modules), rendering snapshots and sending commands.
- **Data model:** the engine's `SimState` is `serde`-serializable; the server
  broadcasts it as JSON each tick. The client consumes that JSON directly.
  No separate "API model" is maintained — the engine state *is* the wire
  format.

## Consequences
- One language for engine + server; trivially small binary.
- The client and engine share a single source of truth (the serialized state),
  so there is no drift between "what the sim has" and "what the client
  renders".
- Cost: every field the client needs must be `Serialize`d; adding a field is
  a (backward-compatible) schema change. Heavy state would be a large JSON
  payload each tick — see ADR 0003 for the mitigation (full snapshot now,
  consider delta later).
