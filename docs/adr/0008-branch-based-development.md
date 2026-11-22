# 0008 — Branch-based development: cron implements, owner reviews on GitHub

- Status: Accepted
- Date: 2026-11-22
- Supersedes: —

## Context
A daily cron autonomously improves the game. If it pushed straight to `main`
it could ship a broken or off-design change with no checkpoint. The owner
wants to stay the design authority and the merge gate.

## Decision
The cron works **branch-based**:
1. Read `DESIGN.md` (source of truth) and `RUNBOOK.md` (process).
2. Health check, `git pull origin main`, `cargo test`.
3. Pick the single highest-value improvement for the current phase
   (M1 build system → M2 sandbox → M3 puzzle level).
4. Implement on a branch `daily-YYYYMMDD-slug`, `cargo test`, commit,
   **push the branch only**.
5. Report to the owner: branch name, what changed, diff summary,
   what-to-test, open questions, ending `REVIEW: merge or reject branch X`.

The cron **never pushes to `main` and never merges**. The owner reviews on
GitHub and merges or rejects. The cron is an **engineer, not a game
designer**: when the design is unclear it flags it (report + `DESIGN.md`
§8) and proceeds with a *labeled* minimal assumption; it never invents
design calls.

## Consequences
- The owner keeps full design + merge authority; every change is a
  reviewable, revertible branch.
- `DESIGN.md` + `docs/adr/` + `GLOSSARY.md` are the shared vocabulary that
  keeps the cron and the owner aligned.
- Cost: changes wait for review (slower than auto-merge); rejected
  approaches must be noted so the cron doesn't repeat them.
