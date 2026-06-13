# AGENTS.md

## Development Goal

`bbb` is being rewritten as a native Minecraft Java 26.1 client with owned protocol,
world, renderer, control, platform, and runtime boundaries.

The target state is a repo-native implementation, not a wrapper around Bevy, Azalea,
or another client stack. Old files and code may be replaced when that moves the native
rewrite forward. Git history must not be cleaned or rewritten unless the user asks for
that explicitly.

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
- Worker agents own bounded coding tasks with explicit file/module write scopes.
- Explorer agents answer narrow codebase or vanilla-source questions when that can run
  in parallel with implementation.

The main agent should keep the critical path moving locally while workers handle
disjoint modules. Workers must not commit and must not rewrite git history.

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
6. Main agent integrates worker changes, resolves API mismatches, and reviews the full diff.
7. Run formatting, diff checks, and tests.
8. Make a normal commit after verification.

## Worker Prompt Requirements

Every worker prompt should include:

- Working directory.
- Exact owned files/modules.
- Required behavior and authoritative vanilla facts.
- Reminder that other agents may be editing concurrently.
- Instruction not to revert unrelated changes.
- Instruction not to commit or rewrite history.
- Required focused tests.
- Final report with changed paths and test results.

## Testing Gate

Before committing a slice, run:

```sh
cargo fmt
git diff --check
cargo test --workspace
```

Focused crate tests are useful while developing, but the default merge gate is the full
workspace test suite. If a command cannot run, state the reason and the residual risk.

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
