# Agent Development Workflow

This repo is advanced by small, verified native-client slices.

Configuration is supplied at command-line startup. Slices must not add in-game
configuration UI.

## Roles

- Main agent: owns scope selection, plan, vanilla-source checks, task split,
  integration, final tests, review, and commit.
- Worker agents: own bounded implementation tasks with explicit write scopes,
  normally in independent git worktrees and temporary branches. Workers do not
  commit to `master` and do not rewrite git history.
- Explorer agents: answer narrow codebase or vanilla-source questions when the
  main agent can keep implementing in parallel.

## Turn Loop

1. Main agent checks `git status --short` and identifies existing user or agent changes.
2. Main agent selects the next slice that moves the repo closer to a full
   native Minecraft Java 26.1 client.
3. Main agent verifies packet wire format or behavior against
   `<MC_CODE_ROOT>/sources/26.1/` when relevant.
4. Main agent splits work by disjoint file ownership and starts workers for non-blocking modules.
5. Each coding worker uses a separate git worktree and temporary branch when
   parallel implementation is expected, for example:
   - `../bbb-wt-renderer`
   - `../bbb-wt-world`
   - `../bbb-wt-protocol`
6. Main agent gives each worker an exact write scope and a distinct
   `CARGO_TARGET_DIR` when focused tests may run in parallel.
7. Main agent keeps one critical-path task local while workers run.
8. Workers edit only their assigned files, run focused tests, and report
   changed paths plus test results.
9. Main agent reviews worker diffs and integrates them by patch, cherry-pick, or
   merge from temporary branches.
10. Workers should remove their own worktree-local Cargo build output after
   reporting, for example `rm -rf target` or the assigned `CARGO_TARGET_DIR`.
11. Main agent removes temporary worktrees and branches after the worker diff is
   either integrated or explicitly abandoned.
12. Main agent resolves integration issues, runs `cargo fmt`,
   `git diff --check`, and `cargo test --workspace`.
13. Main agent commits with a normal commit after verification. It never cleans
   or rewrites git history unless explicitly instructed.

## Default Slice Shape

- Protocol: packet ids, packet structs, decode/encode, protocol tests.
- World: canonical state, counters, query APIs, world tests.
- Net: event/command wiring and offline probe behavior.
- Native/control: drain handling, snapshot counters, focused runtime tests.
- Renderer/input: only when the slice has a visible or interactive client behavior.

Do not create split-only slices unless the split satisfies
`docs/code-organization-style.md`. Parallel worker ownership should follow real
feature or semantic boundaries, not arbitrary line ranges.

When a slice includes module extraction, the main agent should state the split
rationale before workers start:

- The semantic owner being created or clarified.
- The files/modules each worker may edit.
- The behavior-preserving tests that must still pass.
- The current or near-term work made easier by the split.

If that rationale is only line count or aesthetics, keep the file intact.
If the rationale is only to create more parallel worker tasks, keep the file
intact and choose a smaller feature slice instead.

## Worker Contract

Every worker prompt must include:

- Working directory.
- Worktree path and temporary branch name when the worker is expected to edit
  outside the main worktree.
- `CARGO_TARGET_DIR` for that worktree when the worker may run Cargo tests.
- Exact file or module ownership.
- Required behavior and authoritative vanilla facts.
- Reminder that other agents may be editing the repo.
- No `master` commit, no direct merge to `master`, and no history rewriting.
- Required focused tests and final report format.

## Worktree Discipline

- Do not let multiple workers edit the same large file or unstable module.
- Prefer parallel worktrees for renderer/assets, world tests, protocol
  decode/encode, and narrow net wiring.
- Keep `lib.rs` extraction and other structural refactors single-owner because
  they create broad merge conflicts.
- Worker branches are integration inputs. The main agent owns the final diff,
  final tests, and final commit.
- Workers clean worktree-local Cargo outputs before they finish so temporary
  `target` directories do not accumulate.
- Delete or reuse temporary worktrees deliberately after integration so stale
  branches do not become an alternate source of truth.
- Do not force-remove a dirty worker worktree until the main agent has reviewed
  its diff and confirmed the changes are integrated, duplicated, or explicitly
  abandoned.
- When removing a completed worker worktree, also delete the temporary branch if
  it is merged or no longer needed.

## Merge Gate

A slice is ready to commit only when:

- The full diff matches the selected slice.
- No unrelated file churn is present.
- `cargo fmt` passes.
- `git diff --check` passes.
- `cargo test --workspace` passes, or any skipped command is explicitly justified.
