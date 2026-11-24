# Architecture Decision Records

ADRs record the **why** behind non-obvious decisions in this repo. Once a
decision is accepted here, it is binding: the cron and contributors implement
against it. To change a decision, write a new ADR that supersedes the old one
(do not silently edit history).

## Rules

1. One decision per ADR. Numbered sequentially, never reused.
2. Every ADR has: **Context** (the problem + forces), **Decision** (what we
   chose, in one paragraph), **Consequences** (tradeoffs, what it enables or
   prevents).
3. Status: `Proposed` → `Accepted` / `Rejected` / `Superseded by 00NN`.
4. ADRs are written by the owner (or on the owner's instruction). The cron
   **flags** a needed decision by opening a `Proposed` ADR draft and noting it
   in the daily report — it does not mark anything `Accepted`.
5. Keep each ADR short: a reader should finish it in under a minute.

## Template

```markdown
# 00NN — <decision title>

- Status: Proposed
- Date: YYYY-MM-DD
- Supersedes: —

## Context
<problem, constraints, forces>

## Decision
<what we chose, in one paragraph>

## Consequences
<tradeoffs, what it enables/prevents, follow-ups>
```

## Index

| # | Title | Status |
|---|-------|--------|
| [0001](0001-headless-deterministic-engine.md) | Headless, deterministic engine split from graphics | Accepted |
| [0002](0002-rust-stack.md) | Rust engine + axum server + three.js client | Accepted |
| [0003](0003-websocket-snapshot-protocol.md) | WebSocket: server broadcasts snapshots, client sends commands | Accepted |
| [0004](0004-grid-based-placement.md) | Grid-based (Factorio-style) placement | Accepted |
| [0005](0005-automatic-block-section-signals.md) | Signals are automatic block sections | Accepted |
| [0006](0006-auto-trade-trains.md) | Trains auto-load/unload; the player is a system designer | Accepted |
| [0007](0007-economy-v1.md) | Economy v1 numbers | Accepted |
| [0008](0008-branch-based-development.md) | Branch-based development: cron implements, owner reviews on GitHub | Accepted |
| [0009](0009-engine-owned-command-application.md) | Command application is engine-owned; the server owns no rules | Accepted |
| [0010](0010-empty-start-game.md) | New game starts empty; the player buys the first train | Accepted |
| [0011](0011-atomic-command-rejection.md) | Rejected commands are atomic and leave state unchanged | Accepted |
| [0012](0012-once-per-stop-auto-trade.md) | Auto-trade is once-per-stop, gated by `traded_at` | Accepted |
| [0013](0013-fixed-train-speed.md) | Train speed is fixed (no speed upgrade) | Accepted |
| [0014](0014-no-train-cap.md) | No train cap (train count limited only by money) | Accepted |
| [0015](0015-grid-build-commands.md) | Grid build commands: BuildTrack / BuildSignal | Accepted |
