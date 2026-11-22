# 0003 — WebSocket: server broadcasts snapshots, client sends commands

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
The client must render a live simulation and send player intents (place
track, buy a train, set a route). We need a single transport that carries
both directions, is easy to debug, and works over plain HTTP(S) without
extra infra.

## Decision
A single WebSocket at `/ws` carries both directions:
- **Server → client:** a full JSON snapshot of `SimState`, broadcast to all
  connected clients once per tick (10 Hz).
- **Client → server:** JSON `Command` values (tagged enum: `NewGame`,
  `SetPaused`, `SetSpeed`, `SetRoute`, `Load`, `Unload`, and the build
  commands added in M1). The server decodes and applies them to the state.

The server is the single authority: it owns the tick loop and the state;
clients are stateless renderers. A `/status` endpoint exposes a lightweight
`Status` summary for health checks.

## Consequences
- Simple mental model: one channel, one direction each way.
- The server is the source of truth; no client-side prediction is required
  (trains are slow enough that 10 Hz is smooth).
- Cost: a full snapshot every tick is O(state size) on the wire. For the
  current small map this is fine; if the network grows large, we will
  switch to delta/dirty-object updates (a follow-up ADR).
