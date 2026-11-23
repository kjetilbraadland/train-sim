# 0011 — Rejected commands are atomic and leave state unchanged

- Status: Accepted
- Date: 2026-11-23
- Supersedes: —

## Context
`command::apply` can fail (insufficient funds, unknown node/train, unknown
map). A failed command must not partially mutate the state, or the sim
could silently lose money or spawn half-built objects — and determinism
tests would be ambiguous about what a rejected command did.

## Decision
A command is **all-or-nothing**: `apply` validates first and returns a
`CmdError` without mutating on failure; on success it applies the whole
effect. Examples: `BuyTrain` checks the node exists *and* funds before
deducting money and spawning; a rejected command leaves `money` and
`trains` untouched (covered by tests, e.g. `buy_train_rejected_when_broke`,
`new_game_unknown_map_rejected`).

## Consequences
- Invariant: after `apply` returns `Err`, the state is exactly as before —
  easy to reason about and to test.
- The client can surface the `CmdError` (e.g. "insufficient funds: need 200,
  have 100") directly to the player.
- Cost: each command must validate-then-apply (no interleaved partial
  mutation); multi-step commands must be written to this rule.
