# Standards review — implementation in progress

Fixed point: `3bd95b97e0b1fa5aa89fc8d42034f81b6218a7e7`; working-tree mode.
No commits after the fixed point. Includes pre-existing working-tree changes
and new untracked implementation/test/assets files. This is a checkpoint,
not approval to merge or a claim that all tickets are complete.

Sources: `AGENTS.md`, `docs/agents/domain.md`, ADR-0001, ADR-0002,
ADR-0003, and the installed code-review skill's smell baseline.

1. **Possible Primitive Obsession / Mysterious Name:**
   `crates/core/src/simulation.rs` represents a finished Car in AI perception
   as `(Vec2::splat(1.0e6), Vec2::ZERO)`. The coordinate sentinel hides the
   actual concept of an inactive competitor and can affect pass statistics.
   Prefer explicit participation in the perception boundary while preserving
   stable Car indices. This is a design judgment, not a documented violation.

2. **Possible Duplicated Code:** `tick` and `snapshots` each assign
   `snapshot.finish_window_ticks = self.winner_tick.map(...)`.
   Keep the finish-window projection in one place to prevent divergence
   during the pending pause/recovery changes. This is a small maintenance
   judgment, not a documented violation.

No hard documented-standard violation identified in this pass. Engine-free
rules remain in core; effect budgets and geometry are in the shell; the
physics constants and fixed pixel scale are preserved. Formatting and lint
findings are handled by tooling and are excluded from these findings.

Total: 2 heuristic findings; highest concern is the inactive-Car sentinel.
