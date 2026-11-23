# 0009 — Command application is engine-owned; the server owns no rules

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
Originally the server decoded JSON into a `Command` and applied it to the
state in its own `apply_command`. That put game rules (costs, clamping,
id assignment) in the I/O layer, which contradicts ADR 0001 (the engine is
the only place rules live) and made those rules untestable without a
running server.

## Decision
All command application lives in the engine: a single pure function
`command::apply(state, &Command) -> Result<(), CmdError>`. The server
decodes JSON into a `Command` and calls `apply`; it owns **no** rules.
Economy numbers are engine constants (`economy.rs`). A command either fully
applies or is rejected with a `CmdError` and leaves the state unchanged
(see ADR 0011).

## Consequences
- Every rule (costs, clamping, validation, id assignment) is unit-testable
  without a server; the server is a thin codec + tick loop.
- Reinforces ADR 0001/0002: one place for rules, shared by tests, server,
  and any headless harness.
- Cost: the engine's `Command` enum must cover every player intent; adding
  a command is an engine change (which is the point).
