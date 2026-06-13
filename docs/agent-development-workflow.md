# Agent Development Workflow

This repo is advanced by small, verified native-client slices.

## Roles

- Main agent: owns scope selection, plan, vanilla-source checks, task split, integration, final tests, review, and commit.
- Worker agents: own bounded implementation tasks with explicit write scopes. Workers do not commit and do not rewrite git history.
- Explorer agents: answer narrow codebase or vanilla-source questions when the main agent can keep implementing in parallel.

## Turn Loop

1. Main agent checks `git status --short` and identifies existing user or agent changes.
2. Main agent selects the next slice that moves the repo closer to a full native Minecraft Java 26.1 client.
3. Main agent verifies packet wire format or behavior against `<MC_CODE_ROOT>/sources/26.1/` when relevant.
4. Main agent splits work by disjoint file ownership and starts workers for non-blocking modules.
5. Main agent keeps one critical-path task local while workers run.
6. Workers edit only their assigned files, run focused tests, and report changed paths plus test results.
7. Main agent reviews all diffs, resolves integration issues, runs `cargo fmt`, `git diff --check`, and `cargo test --workspace`.
8. Main agent commits with a normal commit after verification. It never cleans or rewrites git history unless explicitly instructed.

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
- Exact file or module ownership.
- Required behavior and authoritative vanilla facts.
- Reminder that other agents may be editing the repo.
- No commit and no history rewriting.
- Required focused tests and final report format.

## Merge Gate

A slice is ready to commit only when:

- The full diff matches the selected slice.
- No unrelated file churn is present.
- `cargo fmt` passes.
- `git diff --check` passes.
- `cargo test --workspace` passes, or any skipped command is explicitly justified.
