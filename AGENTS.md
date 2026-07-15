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
  `docs/code-organization-style.md`
- Crate boundaries, ownership rules, and state taxonomy:
  `docs/native-client-architecture.md`
- Build/target/cache policy and measured build baselines:
  `docs/cargo-build-performance.md`
- Current priorities, hard constraints, structural invariants, and the
  pre-commit gate: `goal.md`; the feature gap ledger: `docs/unsupported-features.md`
- For packet ids and wire formats, prefer vanilla `GameProtocols.java` and the
  relevant `Clientbound*Packet` / `Serverbound*Packet` class.
- For client behavior, inspect `ClientPacketListener`, `ClientLevel`,
  `LevelRenderer`, `LocalPlayer`, `MultiPlayerGameMode`, and nearby vanilla classes.
- If rendering details are ambiguous, compare against the official client behavior.
- Do not rely on memory for protocol details when a local vanilla source file exists.

## Execution Ownership

- The main agent owns the complete slice by default: task selection, planning,
  vanilla-source verification, implementation, focused tests, integration, review,
  the final gate, and the commit.
- Subagents are optional concurrency tools, not a default topology. Do not spawn one
  merely because a role or execution slot is available.
- Explorer agents are useful only for bounded, read-only questions that can be
  investigated while the main agent continues independent critical-path work.
- Worker agents are useful only for bounded implementation with explicit,
  non-overlapping file/module ownership and a frozen integration contract.
- Reviewer agents are useful for large, ambiguous, or high-risk integrated diffs;
  they are not required for every slice.

The main agent remains responsible for verifying critical vanilla facts and reviewing
all integrated code. Workers must not commit to `master`, merge into `master`, or
rewrite git history. If a slice explicitly allows worker commits, those commits stay
on temporary branches and the main agent remains the only merger and final committer.

## Orchestration Strategy

Use a single-slice pipeline rather than assigning several complete features at once.
The default path is solo: the main agent verifies the gap and vanilla behavior,
implements the slice, runs focused tests, updates documentation, reviews the diff,
runs the gate, and commits without delegation.

Spawn a subagent only when the expected critical-path saving is greater than the
setup, context-transfer, worktree, integration, and review cost. In particular, the
following should all be true for an implementation worker:

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
semantic ownership rather than checklist count.

For a parallel slice, run three stages:

1. **Evidence and contract**: the main agent confirms the ledger gap, verifies all
   critical vanilla facts, and freezes owned paths, public types/functions, ordering,
   fallback behavior, and focused test commands. A read-only explorer may gather
   additional evidence concurrently when it is not a prerequisite for all progress.
2. **Disjoint implementation**: workers edit non-overlapping modules in independent
   worktrees while the main agent advances a separate integration path. Use the
   fewest workers that produce real critical-path savings. Do not start another
   complete slice before the current slice is integrated and committed.
3. **Review and closeout**: workers run focused tests only. The main agent reviews the
   integrated diff, runs formatting, diff checks, the authoritative full gate, and
   the final commit once. Add a read-only reviewer only when diff size, ambiguity, or
   regression risk justifies the extra pass.

While the full gate runs, an otherwise idle read-only agent may audit the next slice
without editing files or launching Cargo, but the next implementation waits for the
current commit.

Parallelize by semantic ownership, not by checklist count. When a slice establishes
a reusable renderer or world surface, prefer the next ledger item that can consume
that surface without violating priority order; this amortizes verification and
reduces repeated architecture work.

## Standard Workflow

1. Start with `git status --short`.
2. Identify the next slice that moves the repo closer to the native 26.1 client.
3. Verify relevant vanilla wire format or behavior from local sources.
4. Identify the semantic ownership involved, usually:
   - protocol: packet ids, structs, encode/decode, protocol tests
   - world: canonical state, counters, query APIs, world tests
   - net: event/command wiring and offline probe behavior
   - native/control: drain handling, snapshot counters, runtime tests
   - renderer/input: visible or interactive behavior when the slice needs it
5. Keep the slice with the main agent unless it passes the parallelism criteria above.
   If it does, spawn workers only for non-overlapping modules.
   Prefer one independent git worktree per worker, for example:
   - `../bbb-wt-renderer`
   - `../bbb-wt-world`
   - `../bbb-wt-protocol`
   Use `scripts/worker-worktree.sh create <name>` for the standard path.
6. When workers are used, give each worker worktree a temporary branch and, when
   tests may run in parallel, a distinct external `CARGO_TARGET_DIR` to avoid Cargo
   lock contention and preserve build cache.
7. If worker changes exist, the main agent integrates them by reviewing diffs and
   using patch, cherry-pick, or merge from temporary branches. Main agent resolves
   API mismatches and reviews the full integrated diff.
8. Workers keep assigned external Cargo build caches unless the slice explicitly
   asks for cleanup or the cache is disposable.
9. Main agent removes temporary worktrees and branches after their diffs are
   integrated or explicitly abandoned.
10. Run formatting, diff checks, and tests.
11. Make a normal commit after verification.

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
  are good parallel worktree tasks.
- Structural refactors such as `lib.rs` extraction are usually main-agent work
  or a single-worker task because concurrent edits are likely to conflict.
- Worker branches are integration inputs, not final history. The main agent
  owns the final reviewed commit on `master`.
- Use external target directories for agent work. The main worktree should use
  `/tmp/bbb-target-main`; workers should use stable per-domain directories such
  as `/tmp/bbb-target-renderer`, `/tmp/bbb-target-world`, or
  `/tmp/bbb-target-net`.
- Standard worker setup:
  - `scripts/worker-worktree.sh create world`
  - `scripts/worker-worktree.sh status`
  - `scripts/worker-worktree.sh shell-env world`
  - `scripts/worker-worktree.sh cleanup world`
  The helper creates `../bbb-wt-<name>`, branch `worker/<name>`, and reports
  `BBB_CARGO_TARGET_NAME=<name>` plus
  `CARGO_TARGET_DIR=/tmp/bbb-target-<name>`. Worker focused tests should prefer
  `scripts/cargo-dev.sh test -p <crate> <filter>`.
- Do not delete assigned Cargo target caches after every slice. Clean them
  periodically or when measuring a clean build, reclaiming disk, or abandoning a
  disposable one-off target.
- Repo-local `target` directories should still not be generated or committed.
- Main agent should remove completed worker worktrees and temporary branches
  after integration.
- Do not force-remove a dirty worker worktree until its diff has been reviewed
  and confirmed integrated, duplicated, or intentionally abandoned.

## Testing Gate

The authoritative pre-commit gate is defined in `goal.md` under "提交前门禁".
Do not copy the command list into other files; point at `goal.md` instead.
`scripts/cargo-dev.sh gate` runs exactly that gate against `/tmp/bbb-target-main`.
Developers may optionally run `scripts/install-hooks.sh` to install the seconds-only local
pre-commit checks; it is never installed automatically and does not replace the full gate.

Focused crate tests are useful while developing, but the default merge gate is the full
workspace check + test suite. If a command cannot run, state the reason and the residual risk.
For daily focused tests, agents may use `cargo test --profile fast-test` with an
assigned external `CARGO_TARGET_DIR`; this does not replace the merge gate.
Agents may use `scripts/cargo-dev.sh` for focused tests, fast-test, timings,
`sccache` evaluation/status, target-size inspection, and the same final gate
commands.

## Implementation Guidance

- Keep changes scoped to the selected slice.
- Organize code by semantic modules and avoid growing catch-all `lib.rs` or
  `main.rs` sections; use `docs/code-organization-style.md` as the required
  module/layout policy.
- Do not split files just to reduce line count. Files under 1000 lines stay
  intact by default, and files over 1000 lines are reviewed rather than
  extracted automatically.
- A split must have a named semantic owner, a narrow API, focused tests or a
  clear test plan, and a concrete payoff for current or near-term work.
- The default is to preserve locality. A new module must make the current slice
  easier to review, test, or assign; line count, directory symmetry, and
  artificial worker parallelism are not valid split reasons.
- Before creating a module, pass the checklist in
  `docs/code-organization-style.md`: semantic owner, exact moved items, narrow
  visibility, colocated tests, immediate use, and mechanical reviewability.
- Keep cohesive single-caller helpers beside their caller. Avoid one-function
  modules, vague bucket modules, and broad `pub` exposure created only to make a
  split compile.
- Use Rust 2018 module layout: `foo.rs` plus `foo/bar.rs`; do not add new
  `foo/mod.rs` files.
- Prefer existing crate boundaries and local patterns over new abstractions.
- Add structured packet/state types rather than ad hoc byte or string handling.
- Keep counters and query APIs useful for native snapshots and future renderer work.
- For packet handling, implement the full path when practical: protocol decode,
  net event, world/native application, and focused tests.
- Preserve tracked `Cargo.lock`.
- Do not introduce unrelated formatting churn or metadata changes.
