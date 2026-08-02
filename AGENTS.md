# AGENTS.md

## Development Goal

`bbb` is being rewritten as a native Minecraft Java 26.1 client with owned protocol,
world, renderer, control, platform, and runtime boundaries.

The target state is a repo-native implementation, not a wrapper around Bevy, Azalea,
or another client stack. Old files and code may be replaced when that moves the native
rewrite forward. Git history must not be cleaned or rewritten unless the user asks for
that explicitly.

Configuration is supplied at command-line startup. Do not implement in-game
configuration UI.

## Authoritative References

- Use local vanilla 26.1 sources first:
  `~/Work/mc-code/sources/26.1/`
- Follow the repo code organization and style rules in:
  `docs/wiki/development/architecture-rules.md`
- Crate boundaries, ownership rules, and state taxonomy:
  `docs/wiki/architecture/overview.md` and
  `docs/wiki/architecture/client-boundaries.md`
- Build/target/cache policy and measured build baselines:
  `docs/wiki/development/build-and-cache.md` and
  `docs/cargo-build-performance.md`
- Current priorities, hard constraints, structural invariants, slice shapes,
  ready contracts, hardest-ready-first selection, and the checkpoint/full-gate
  policy: `goal.md`. The feature gap ledger: `docs/unsupported-features.md`.
  Completed history: the milestone index in `docs/goal-archive.md` plus git log.
- For packet ids and wire formats, prefer vanilla `GameProtocols.java` and the
  relevant `Clientbound*Packet` / `Serverbound*Packet` class.
- For client behavior, inspect `ClientPacketListener`, `ClientLevel`,
  `LevelRenderer`, `LocalPlayer`, `MultiPlayerGameMode`, and nearby vanilla classes.
- If rendering details are ambiguous, compare against the official client behavior.
- Do not rely on memory for protocol details when a local vanilla source file exists.

## Execution Ownership

- The main agent owns the complete slice by default: task selection, planning,
  vanilla-source verification, implementation, focused tests, integration, review,
  commits, and the applicable checkpoint or full gate.
- Subagents are optional concurrency tools, not a default topology. Do not spawn one
  merely because a role or execution slot is available.
- Explorer agents suit bounded, read-only questions investigated while the main
  agent continues independent critical-path work. Worker agents suit bounded
  implementation with explicit, non-overlapping file/module ownership and a frozen
  integration contract. Reviewer agents suit large, ambiguous, or high-risk
  integrated diffs; they are not required for every slice.

The main agent remains responsible for verifying critical vanilla facts and reviewing
all integrated code. Workers must not commit to `master`, merge into `master`, or
rewrite git history. If a slice explicitly allows worker commits, those commits stay
on temporary branches and the main agent remains the only merger and final committer.

## Orchestration Strategy

Use a single-slice pipeline rather than assigning several complete features at once.
The default path is solo: verify the gap and vanilla behavior, implement, run focused
tests, update documentation, review the diff, run the per-commit checks, and commit
without delegation. Slice shapes (atomic vs engineering), ready contracts,
hardest-ready-first selection, and all checkpoint/gate rules are defined in `goal.md`
and are not duplicated here.

Spawn a subagent only when the expected critical-path saving is greater than the
setup, context-transfer, worktree, integration, and review cost. For an
implementation worker, all of the following should be true:

- the task has a bounded deliverable and exact owned paths;
- its integration API, ordering constraints, and fallback behavior are already
  frozen;
- it does not write the same files or unstable module as the main agent or another
  worker;
- the main agent has useful independent work to perform while it runs; and
- its result can be reviewed and integrated mechanically without reopening the
  design.

Prefer the solo path for small checklist items, cohesive single-owner changes,
serial vanilla investigation, central-module edits, and slices where the main agent
would otherwise wait for the subagent. Never fill available agent slots for their own
sake. When parallelism is justified, use the fewest agents needed and split by
semantic ownership rather than checklist count. A slice being a large engineering
effort does not by itself authorize subagents.

For a parallel slice, run three stages:

1. **Evidence and contract**: the main agent confirms the ledger gap, verifies all
   critical vanilla facts, and freezes owned paths, public types/functions, ordering,
   fallback behavior, and focused test commands. A read-only explorer may gather
   additional evidence concurrently when it is not a prerequisite for all progress.
2. **Disjoint implementation**: workers edit non-overlapping modules in independent
   worktrees while the main agent advances a separate integration path. Do not start
   another complete slice before the current slice reaches completion, an explicit
   blocker, or its predeclared safe stopping point.
3. **Review and closeout**: workers run focused tests only. The main agent reviews the
   integrated diff, runs the required per-commit checks, makes the commit, and applies
   the checkpoint/full-gate policy from `goal.md`.

While a checkpoint full gate runs, an otherwise idle read-only agent may audit the
next slice without editing files or launching Cargo, but the next implementation
waits for the current commit.

## Standard Workflow

1. Start with `git status --short`.
2. Run the P0 audit, then apply the hardest-ready-first policy from `goal.md`.
3. Verify relevant vanilla wire format or behavior from local sources.
4. Identify the semantic ownership involved (protocol / world / net /
   native-control / renderer-input); for an engineering slice, record the
   pre-start contract required by `goal.md`.
5. Keep the slice with the main agent unless it passes the parallelism criteria
   above. If workers are used, create worktrees with
   `scripts/worker-worktree.sh create <name>` and give each a temporary branch
   plus a distinct external `CARGO_TARGET_DIR`.
6. The main agent integrates worker diffs via patch/cherry-pick/merge, resolves
   API mismatches, reviews the full integrated diff, and removes temporary
   worktrees and branches after integration or explicit abandonment.
7. Before each commit, run the per-commit checks from `goal.md`; commit clean,
   fix failures forward, never rewrite history; then continue or run the full
   gate where `goal.md` requires it.

## Worker Prompt Requirements

Every worker prompt should include:

- Working directory.
- Temporary branch/worktree name when using a worker worktree.
- `CARGO_TARGET_DIR` for focused tests when Cargo may run.
- Exact owned files/modules.
- Required behavior and authoritative vanilla facts.
- Reminder that other agents may be editing concurrently.
- Instruction not to revert unrelated changes.
- Instruction not to commit or rewrite history.
- Required focused tests.
- Required structured final report, using the applicable headings:
  - `FACTS`: authoritative vanilla sources and behavior
  - `CHANGED`: changed paths
  - `API`: added or changed integration surface
  - `TESTS`: exact commands and results
  - `RISKS`: unverified edges or residual risk
  - `INTEGRATION`: actions or compatibility notes for the main agent

## Worker Worktree Discipline

- Do not assign multiple workers to the same large file or unstable module.
- Renderer assets, world tests, protocol decode/encode, and narrow net wiring
  are good parallel worktree tasks; structural refactors are usually main-agent
  or single-worker tasks because concurrent edits conflict.
- Worker branches are integration inputs, not final history. The main agent
  owns the final reviewed commit on `master`.
- Use external target directories: the main worktree uses `/tmp/bbb-target-main`;
  workers use stable per-domain directories such as `/tmp/bbb-target-renderer`,
  `/tmp/bbb-target-world`, or `/tmp/bbb-target-net`.
- Standard worker setup:
  - `scripts/worker-worktree.sh create world`
  - `scripts/worker-worktree.sh status`
  - `scripts/worker-worktree.sh shell-env world`
  - `scripts/worker-worktree.sh cleanup world`
  The helper creates `../bbb-wt-<name>`, branch `worker/<name>`, and reports
  `CARGO_TARGET_DIR=/tmp/bbb-target-<name>`. Worker focused tests should prefer
  `scripts/cargo-dev.sh test -p <crate> <filter>`.
- Do not delete assigned Cargo target caches after every slice. For periodic
  cleanup of stale slice caches use `scripts/cargo-dev.sh prune` (dry-run by
  default; keep-list documented in `docs/wiki/development/build-and-cache.md`).
- Repo-local `target` directories must not be generated or committed.
- Do not force-remove a dirty worker worktree until its diff has been reviewed
  and confirmed integrated, duplicated, or intentionally abandoned.

## Testing Gate

The authoritative per-slice, batch, and full-gate policy is defined in `goal.md`
under “提交前门禁” and the checkpoint rules; do not copy the command list into
other files. `scripts/cargo-dev.sh gate` runs exactly that full gate against
`/tmp/bbb-target-main` and records the passing commit in `.git/bbb/last-gated`;
`scripts/cargo-dev.sh batch-gate` (defaulting to `<last-gated>..HEAD`) is a
failure-localization tool only.

deny-warnings is baked into the workspace lints table; never set
`RUSTFLAGS='-D warnings'` manually — toggling `RUSTFLAGS` invalidates the whole
shared target cache in both directions (see `docs/wiki/development/build-and-cache.md`).
Developers may optionally run `scripts/install-hooks.sh` for the seconds-only
local pre-commit checks; it never replaces the full gate.

For daily focused tests, prefer `scripts/cargo-dev.sh fast-test` with an
assigned external `CARGO_TARGET_DIR`; it exports the `RUST_MIN_STACK`
headroom that opt-level-0 frames need (see
`docs/wiki/development/build-and-cache.md`). `scripts/cargo-dev.sh` also covers focused
tests, timings, `sccache` evaluation, and target-size inspection.
Track the latest fully gated commit; any delivered or merged HEAD must have
passed the full gate.

## Implementation Guidance

- Keep changes scoped to the selected slice.
- Organize code by semantic modules per `docs/wiki/development/architecture-rules.md`; avoid
  growing catch-all `lib.rs`/`main.rs` sections.
- Do not split files just to reduce line count. Files under 1000 lines stay
  intact by default; a split needs a named semantic owner, a narrow API, focused
  tests or a clear test plan, and a concrete payoff. Keep cohesive
  single-caller helpers beside their caller; avoid one-function modules, vague
  bucket modules, and broad `pub` exposure created only to make a split compile.
- Use Rust 2018 module layout: `foo.rs` plus `foo/bar.rs`; do not add new
  `foo/mod.rs` files.
- Prefer existing crate boundaries and local patterns over new abstractions.
- Add structured packet/state types rather than ad hoc byte or string handling.
- Keep counters and query APIs useful for native snapshots and future renderer work.
- For packet handling, implement the full path when practical: protocol decode,
  net event, world/native application, and focused tests.
- Preserve tracked `Cargo.lock`.
- Do not introduce unrelated formatting churn or metadata changes.
