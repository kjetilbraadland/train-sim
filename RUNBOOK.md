# train-sim — Daily Improvement RUNBOOK

The cron follows this exactly. The owner (Dampkjetil) reviews on GitHub and merges.

## Reach the project
```
ssh -i ~/.ssh/kgpt_deploy root@192.168.1.63
export PATH=$HOME/.cargo/bin:$PATH
cd /opt/train-sim
```
- Server: systemd service `train-sim`, port 8771.
- GitHub: `kjetilbraadland/train-sim`.

## Source of truth
`DESIGN.md` in the repo (owner: Dampkjetil). Read it first. Implement strictly
against it. Use the terms from `GLOSSARY.md` in code and reports. When a
decision is non-obvious, check `docs/adr/` for the accepted ADR that governs
it; if none exists, that's a design gap — flag it, don't invent a decision.
**You are an engineer, not a game designer.** If the design is unclear or an
open question blocks you, flag it in the report (and note it in DESIGN.md
section 8), then proceed with the minimal reasonable assumption *explicitly
labeled*. Never invent design calls.

## Steps
1. **Health:** `systemctl status train-sim` and
   `curl -s http://127.0.0.1:8771/status`. If down: `journalctl -u train-sim -n 50`,
   fix, restart.
2. `git checkout main` and `git pull origin main`.
3. `cargo test` — note failures.
4. Read `engine/src`, `server/src`, `static`, `DESIGN.md`. Pick the **single
   highest-value improvement** toward the current phase:
   **M1 build system → M2 sandbox → M3 puzzle level**. Respect the v1 economy
   numbers in DESIGN.md.
5. `git checkout -b daily-YYYYMMDD-slug`. Implement. `cargo test`. If all pass,
   commit (`git -c user.name=hermes -c user.email=hermes@localhost commit -m message`)
   and **push the branch** (`git push origin daily-YYYYMMDD-slug`).
   **Do NOT push to main. Do NOT merge.**
6. **Report for the owner** (this is the delivered message):
   - branch name
   - one-line what changed
   - short diff summary (files + key hunks, not the full diff)
   - what-to-test (2–4 concrete steps, e.g. open http://192.168.1.63:8771, do X, expect Y)
   - any open questions flagged
   - end with: `REVIEW: merge or reject branch <name>`

If a build or test fails, report the exact error and do **not** push the broken
branch. If nothing changed, say what you found and recommend next.

## Hard rules
- Preserve engine determinism: no wall clock, no unseeded RNG in the engine.
- Keep the engine I/O-free.
- Only restart the service if it was down or you rebuilt the server binary
  (then `systemctl restart train-sim`).
- Never push to main. Never merge.
