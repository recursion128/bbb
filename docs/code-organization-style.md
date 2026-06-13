# Code Organization Style

This repo is a native Minecraft Java 26.1 client rewrite. Code should be organized
by semantic ownership, not by convenience. Large catch-all `lib.rs` and `main.rs`
files are transitional only; new work should move the codebase toward smaller,
named modules with clear boundaries.

## Goals

- Make protocol, world state, net wiring, renderer, platform, and native runtime
  code easy to scan independently.
- Keep vanilla-derived behavior close to the module that owns it.
- Make worker-agent ownership easy to assign without merge conflicts.
- Keep tests near the behavior they prove.

## Module Boundaries

Prefer modules that describe a stable concept:

- `protocol`: packet families, codecs, constants, and wire-format tests.
- `world`: canonical client state, packet application, counters, queries, and
  world-state tests.
- `net`: connection lifecycle, command/event transport, online/offline probe
  wiring, and network behavior tests.
- `native`: runtime orchestration, input, camera, control integration, upload
  scheduling, and end-to-end drain tests.
- `renderer`: mesh generation, HUD geometry, GPU-facing data, and rendering tests.
- `pack`: vanilla asset/model/atlas loading and asset tests.

Within each crate, split by feature family when a file grows beyond quick
navigation. Good module names are semantic and stable, for example:

- `player_info`, `scoreboard`, `world_border`, `chunks`, `entities`,
  `inventory`, `hud_state`, `movement`, `terrain`, `hud`, `input`.

Avoid modules named by implementation accident such as `misc`, `helpers`,
`stuff`, `new`, or `common` unless the contents are genuinely shared primitives.

## File Size And Growth

- New feature slices should not add large unrelated sections to existing root
  files.
- When touching a root file that already acts as an index, prefer moving a
  coherent existing section into a module before adding more code to that same
  area.
- Keep root `lib.rs` files focused on `mod` declarations, public re-exports,
  crate-level types, and small glue.
- When a crate-level re-export list grows beyond a quick scan, move it into
  semantic facade modules such as `world_state`, `terrain_state`, or
  `client_state`, then keep `lib.rs` as `mod` declarations plus `pub use`
  facade exports. Use `src/exports.rs` only when the crate has no clearer
  semantic grouping.
- Keep `main.rs` focused on startup and top-level runtime orchestration. Move
  reusable runtime behavior into modules.

The threshold is pragmatic: if a section needs its own heading in comments, has
multiple tests, or becomes a likely worker ownership boundary, it should usually
be a module.

## Public APIs

- Expose narrow, stable APIs from modules. Prefer `pub use` from crate root only
  for types that other crates actually use.
- Do not make helper functions public just to avoid moving code. Keep helpers
  private to the module that owns the behavior.
- Store protocol-facing data in structured types. Avoid raw byte blobs except
  for fields intentionally deferred and documented as raw payloads.
- Store enum-like world state using vanilla serialized names when the value is
  user-facing, snapshot-facing, or useful for renderer/control queries.

## Vanilla References

- Verify packet ids and wire order against local vanilla 26.1 sources before
  implementing packet logic.
- Put vanilla-specific conversion helpers next to the state or packet family
  that uses them.
- Do not rely on memory for fields, ordinals, flags, or nullable semantics when
  a local vanilla class exists.

## Tests

- Put unit tests in the module that owns the behavior.
- Protocol tests should prove wire order, nullable fields, enum ids, bounds, and
  representative payloads.
- World tests should prove vanilla-shaped state mutation, ignored unknown cases,
  counters, and query APIs.
- Native/net tests should prove event forwarding, drain behavior, queued commands,
  and cross-crate integration.
- Keep tests focused. Do not use large end-to-end tests as the only proof for a
  small parser or state transition.

## Agent Workflow Rules

- Main agent should split work by module ownership, not by random file ranges.
- Worker prompts must assign disjoint modules or files.
- Workers should avoid broad root-file edits unless their task is explicitly to
  extract a coherent module.
- When a worker adds a new feature family and a suitable module exists, it must
  use that module.
- When no module exists and the slice would add meaningful size, the worker
  should create the module and keep the root file as the index.
- Agents must not revert unrelated changes made by other agents or the user.
- Agents must not rewrite git history unless the user explicitly requests it.

## Refactoring Policy

Module extraction is encouraged when it reduces future conflicts and makes the
native rewrite easier to continue. Keep extractions mechanical and scoped:

- Move one semantic family at a time.
- Preserve public behavior and tests.
- Avoid mixing module extraction with unrelated behavior changes.
- Run focused tests for the moved module and the full workspace gate before
  committing.

For active feature work, prefer this sequence:

1. Add or identify the target module.
2. Move only the minimal existing code needed for that feature family.
3. Implement the new behavior inside that module.
4. Re-export only what downstream crates need.
5. Run `cargo fmt`, `git diff --check`, and `cargo test --workspace`.
