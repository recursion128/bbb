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
  `/Users/zhangguyu/Work/mc-code/sources/26.1/`
- Follow the repo code organization and style rules in:
  `docs/code-organization-style.md`
- For packet ids and wire formats, prefer vanilla `GameProtocols.java` and the
  relevant `Clientbound*Packet` / `Serverbound*Packet` class.
- For client behavior, inspect `ClientPacketListener`, `ClientLevel`,
  `LevelRenderer`, `LocalPlayer`, `MultiPlayerGameMode`, and nearby vanilla classes.
- If rendering details are ambiguous, compare against the official client behavior.
- Do not rely on memory for protocol details when a local vanilla source file exists.

## Agent Roles

- Main agent owns task selection, planning, vanilla-source verification, worker
  orchestration, integration, review, final tests, and commits.
- Worker agents own bounded coding tasks with explicit file/module write scopes,
  normally in independent git worktrees and temporary branches.
- Explorer agents answer narrow codebase or vanilla-source questions when that can run
  in parallel with implementation.

The main agent should keep the critical path moving locally while workers handle
disjoint modules. Workers must not commit to `master`, merge into `master`, or
rewrite git history. If a slice explicitly allows worker commits, those commits
stay on temporary branches and the main agent remains the only merger and final
committer.

## Standard Workflow

1. Start with `git status --short`.
2. Identify the next slice that moves the repo closer to the native 26.1 client.
3. Verify relevant vanilla wire format or behavior from local sources.
4. Split implementation by disjoint ownership, usually:
   - protocol: packet ids, structs, encode/decode, protocol tests
   - world: canonical state, counters, query APIs, world tests
   - net: event/command wiring and offline probe behavior
   - native/control: drain handling, snapshot counters, runtime tests
   - renderer/input: visible or interactive behavior when the slice needs it
5. Spawn worker agents for non-overlapping modules when parallelism helps.
   Prefer one independent git worktree per worker, for example:
   - `../bbb-wt-renderer`
   - `../bbb-wt-world`
   - `../bbb-wt-protocol`
   Use `scripts/worker-worktree.sh create <name>` for the standard path.
6. Give each worker worktree a temporary branch and, when tests may run in
   parallel, a distinct external `CARGO_TARGET_DIR` to avoid Cargo lock
   contention and preserve build cache.
7. Main agent integrates worker changes by reviewing diffs and using patch,
   cherry-pick, or merge from temporary branches. Main agent resolves API
   mismatches and reviews the full integrated diff.
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
- Final report with changed paths and test results.

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
  The helper creates `../bbb-wt-<name>`, branch `bbb-worker-<name>`, and reports
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

Before committing a slice, run:

```sh
cargo fmt --check
git diff --check
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace
```

Focused crate tests are useful while developing, but the default merge gate is the full
workspace test suite. If a command cannot run, state the reason and the residual risk.
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
