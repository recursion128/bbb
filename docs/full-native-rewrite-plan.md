# Full Native Rewrite Plan

This document is the project-level execution plan for rewriting this repo as a
native Minecraft Java 26.1 client. It is the entry point for scope, sequencing,
verification, and agent workflow.

Architecture details live in `docs/native-client-architecture.md`. Code
organization and split rules live in `docs/code-organization-style.md`. Agent
coordination rules live in `docs/agent-development-workflow.md`.

## Project Goal

Build a native Minecraft Java 26.1 client whose protocol handling, world state,
assets, rendering, audio, input, and control snapshots are owned by Rust modules
with clear semantic boundaries.

The target state is:

- `bbb-protocol` decodes and encodes vanilla 26.1 packets accurately.
- `bbb-world` owns deterministic, serializable canonical client state.
- `bbb-net` owns connection lifecycle, event streams, commands, and probes.
- `bbb-pack` owns official 26.1 resource-pack and asset resolution.
- `bbb-renderer` owns GPU-facing terrain, entity, particle, HUD, and overlay
  runtime state.
- `bbb-audio` or a narrow native audio runtime owns playback resources such as
  Kira managers, tracks, and sound handles.
- `bbb-control` exposes snapshots and counters derived from canonical world and
  runtime state.
- `bbb-native` orchestrates the runtime loop, input, camera, systems, command
  queues, renderer/audio/control integration, and tests.

Old scaffold code may be replaced when it blocks a correct native client. Do not
rewrite or clean git history unless the user explicitly asks for that operation.

## Source Of Truth

- Local vanilla reference: `<MC_CODE_ROOT>/sources/26.1/`.
- Official Minecraft Java 26.1 assets and resource-pack semantics.
- Existing repo architecture documents:
  - `docs/native-client-architecture.md`
  - `docs/code-organization-style.md`
  - `docs/agent-development-workflow.md`

Before changing packet wire format, packet ids, enum ordinals, nullable fields,
client semantics, entity type checks, resource lookup, rendering behavior, or
audio behavior, verify the relevant vanilla 26.1 source or official asset data.

When behavior cannot be fully verified in automated tests, record the vanilla
source path, asset path, or manual comparison required to finish the slice.

## Execution Principles

- Move toward canonical world ownership. New client state belongs in
  `bbb-world` unless it is clearly protocol-only, backend runtime state, or a
  control-only derived view.
- Keep `NetCounters` as projection. It should report state derived from
  `WorldStore` and runtime status, not independently own gameplay or client
  state.
- Keep backend resources out of `bbb-world`. GPU buffers, Kira audio handles,
  windows, threads, and device resources belong to renderer/audio/native/platform
  runtime layers.
- Use `hecs` for entity-domain state and bulk entity systems where it provides
  real value. Do not use ECS for chunks, assets, chat, scoreboard, recipes, HUD,
  or UI state unless a future design gives a concrete reason.
- Prefer semantic modules over large catch-all files. Do not split code only for
  line count, symmetry, or agent parallelism.
- Use Rust 2018 module style. Do not add new `mod.rs` files.
- Offline probes should apply every already-decoded packet that has a world
  apply API.

## Current Strategic Focus

The active architectural cleanup is:

1. Move native-owned snapshot state into `bbb-world`.
2. Make `bbb-native` apply packets to `WorldStore` and project counters from it.
3. Keep `bbb-control::NetCounters` as a derived reporting layer.
4. Add focused world tests and native projection tests for each migrated packet
   family.
5. Keep offline probe behavior aligned with online event handling.

Known priority areas:

- Finish `ProjectilePower` as entity projectile state, not visual effects.
  Vanilla applies it only to existing `AbstractHurtingProjectile` entities; an
  unknown entity or non-hurting-projectile entity is a no-op and should be
  counted as ignored.
- Move debug, game-rule, game-test, and test-instance summaries into canonical
  world-side state.
- Continue removing native-only `last_*` snapshots where a world owner exists or
  should exist.
- Continue applying ignored play/configuration packets in offline probes when a
  world apply API exists.
- Keep audio split into world-observed audio events and a future Kira-backed
  playback runtime.
- Native crosshair entity interaction is partially wired: `bbb-world` exposes
  verified 26.1 base pick bounds for vanilla `LivingEntity` types, boats,
  minecarts, TNT, falling blocks, end crystals, shulker bullets,
  `interaction` entity metadata width/height, and the `redirectable_projectile`
  tag (`fireball`, `wind_charge`,
  `breeze_wind_charge`); `bbb-native` routes left/right/middle mouse actions to
  attack/interact/pick entity packets for those targets. Owner: `bbb-world` +
  `bbb-native`; status: partial; next action: apply dynamic bounding-box
  semantics for baby/pose/scale variants, armor stand marker state, player
  spectator state, slime/magma cube size, and item frames/paintings/leash knots.

## Phases

### 1. Stabilize Architecture And Module Boundaries

Goal: make the codebase navigable and safe for parallel work without artificial
split churn.

Deliverables:

- Root `lib.rs` and `main.rs` files converge toward indexes and thin glue.
- Feature behavior lives in semantic modules with local tests.
- Large files are split only when `docs/code-organization-style.md` justifies the
  boundary.
- Agents can own disjoint modules without creating avoidable conflicts.

Done when:

- New feature slices do not add unrelated sections to mixed root files.
- Existing extracted modules have narrow APIs and tests near behavior.
- No new `mod.rs` files are introduced.

### 2. Complete Protocol Coverage

Goal: decode and encode required Minecraft Java 26.1 login, configuration, play,
and command packets correctly.

Deliverables:

- Packet ids and dispatch tables verified against vanilla 26.1.
- Struct fields match vanilla order, nullability, flags, and enum ordinals.
- Decode tests cover representative vanilla-shaped payloads and edge cases.
- Encode tests exist for clientbound responses and serverbound commands that the
  native client sends.

Done when:

- Required packet families for login, configuration, play, movement, inventory,
  chat, resource packs, interaction, and command suggestions are covered by
  focused protocol tests.
- Unknown or unsupported packets are handled intentionally and reported where
  appropriate.

### 3. Build Canonical World State

Goal: make `bbb-world` the authoritative owner of client-observable state.

Deliverables:

- Level, dimension, time, weather, ticking, chunks, block entities, chunk view,
  maps, recipes, advancements, scoreboard, player info, HUD, UI, chat,
  waypoints, world border, audio observations, visual effect observations, and
  entity state are represented in `WorldStore`.
- Packet apply APIs update narrow semantic state and counters.
- Applied/ignored counters exist where vanilla no-ops based on unknown entity,
  type mismatch, missing registry entry, disabled option, or invalid state.
- Entity-domain hot paths use `hecs` component access instead of full
  `EntityState` projection where practical.
- Query APIs expose the state needed by native, renderer, audio, and control
  without leaking backend details.

Done when:

- Implemented clientbound packet families with client-observable semantics have
  world apply APIs or documented reasons they are runtime-only.
- `bbb-native` no longer owns independent business snapshots for migrated state.
- `NetCounters` fields for world-owned behavior are synchronized from
  `WorldStore`.
- `WorldStore` remains deterministic and serializable.

### 4. Replace Scaffold Networking With Native Client Behavior

Goal: make `bbb-net` and `bbb-native` drive the actual vanilla-shaped client
connection lifecycle.

Deliverables:

- Login, configuration, play transition, compression, keepalive, ping, cookies,
  code of conduct, resource packs, transfer, and disconnect behavior are handled
  intentionally.
- Event streams feed `WorldStore` through the native dispatcher.
- Serverbound movement, interaction, inventory, chat command, and command
  suggestion packets are encoded from native state and input.
- Offline probes apply decoded packets consistently with online handling.

Done when:

- A native runtime can connect to compatible 26.1 servers through login,
  configuration, and play without scaffold assumptions.
- Probe reports are derived from world/control state and do not silently ignore
  supported packet families.

### 5. Implement Official 26.1 Asset Pipeline

Goal: resolve vanilla resources through official 26.1 resource-pack semantics.

Deliverables:

- `bbb-pack` loads vanilla 26.1 baseline assets.
- Resource-pack layering and precedence are represented.
- Blockstates, model parents, texture references, atlases, colormaps, language
  files, sounds, subtitles, tags, and registry-adjacent data have lookup APIs.
- Renderer and audio runtime ask `bbb-pack` for resolved resources instead of
  duplicating path logic.

Done when:

- Renderer and audio can resolve needed vanilla resources through `bbb-pack`.
- Hard-coded asset paths are limited to tests or explicit bootstrap constants.
- Asset tests cover representative official 26.1 data.

### 6. Build Renderer, Particles, HUD, And Interaction

Goal: render and interact with the canonical world using native GPU/runtime
systems.

Deliverables:

- Terrain mesh extraction by render layer with vanilla-shaped geometry, tint,
  light, and atlas metadata.
- Entity rendering extraction from `hecs` components.
- Particle runtime for visual effects and particle packets.
- HUD, title/action bar, scoreboard, chat, debug overlays, selection outline,
  camera pose, screenshots, and interaction feedback.
- Renderer caches and GPU resources remain outside `bbb-world`.

Done when:

- Core world scenes render from canonical world and asset data.
- Visual behavior is covered by deterministic renderer tests where feasible, or
  manually compared with the official client when automation is insufficient.

### 7. Build Audio Runtime

Goal: play vanilla-shaped audio without polluting canonical world state.

Deliverables:

- World audio observations remain in `bbb-world::client::audio`.
- A Kira-backed `bbb-audio` crate or narrow native audio runtime consumes derived
  audio commands.
- Sound event lookup, source/category mapping, spatial sounds, entity-following
  sounds, stop-by-name/source/all behavior, and runtime errors are represented.
- Audio device and handle state stays outside `WorldStore`.

Done when:

- Sound, entity sound, and stop-sound packets can be converted into playback
  commands using official 26.1 asset lookup.
- Normal unit tests do not require an audio device.

### 8. Harden Runtime, Tests, And Control Surface

Goal: make the native client maintainable and verifiable.

Deliverables:

- Deterministic event draining and command queue tests.
- Integration tests around probe, snapshot, world apply, command loop, and
  runtime state transitions.
- Control snapshots and reports derived from world/runtime state.
- Clear diagnostics for unsupported packets, missing assets, renderer/audio
  backend errors, and protocol mismatches.

Done when:

- Workspace tests pass consistently.
- Remaining unsupported features are documented with owner and next action.
- Control output does not diverge from canonical world state.

## Slice Definition

Every implementation slice should have a narrow, testable purpose.

A good slice states:

- Vanilla behavior or asset source being implemented.
- Crate and module owner.
- World state, runtime state, or derived projection boundary.
- Applied/ignored semantics.
- Tests that prove the behavior.
- Probe and control projection changes, if relevant.

Avoid slices that mix broad refactors with behavior changes. If a module
extraction is needed, keep it semantic and justified by
`docs/code-organization-style.md`.

## Verification Gates

Every slice should run focused tests for affected crates. Before commit, run:

```sh
cargo fmt --check
git diff --check
cargo test --workspace
```

If a command cannot be run, record why. If behavior depends on local vanilla
sources, record the source path or asset path used as evidence. If visual or
audio behavior cannot be proven by tests, document the manual comparison or
runtime smoke check still required.

## Agent Workflow

Use `docs/agent-development-workflow.md` as the detailed workflow. The summary
is:

- Main agent owns planning, scope, vanilla-source checks, task split, review,
  integration, full test gate, and commits.
- Worker agents own bounded implementation tasks with explicit file/module
  ownership. Workers do not commit.
- Explorer agents answer narrow codebase or vanilla-source questions while the
  main agent continues critical-path work.
- Agents must work with existing dirty changes and must not revert user or other
  agent work.
- No agent rewrites git history unless the user explicitly requests it.
- Parallelism follows real semantic module boundaries, not arbitrary line ranges.

## Completion Criteria For The Rewrite

The rewrite is complete only when current evidence proves all of the following:

- Required 26.1 protocol paths are implemented and tested.
- Canonical client state lives in `bbb-world` or has a documented non-world
  runtime owner.
- `bbb-native` no longer owns independent business snapshots for world-owned
  behavior.
- Offline probes apply every supported decoded packet consistently with online
  handling.
- Official 26.1 assets are loaded and resolved through `bbb-pack`.
- Renderer and audio runtime consume world/pack-derived data without storing
  backend resources in `WorldStore`.
- Native input, movement, interaction, inventory, and command flows work through
  encoded serverbound packets.
- Control snapshots and counters are derived from canonical world/runtime state.
- `cargo fmt --check`, `git diff --check`, and `cargo test --workspace` pass.
- Any remaining unsupported features are explicitly documented with owner,
  status, and next action.
