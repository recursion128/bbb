# Native Client Architecture

This document defines the target architecture for the native Minecraft Java 26.1
client rewrite. The goal is to stay aligned with vanilla client semantics while
using Rust-friendly data ownership, testable systems, and performance-oriented
runtime boundaries.

The target is not a Java class hierarchy port. Vanilla is the behavioral
reference; this repo owns the implementation architecture.

## Goals

- Match Minecraft Java 26.1 protocol, asset, and client behavior where it
  affects observable client state.
- Keep canonical state deterministic, serializable, and easy to test without a
  window, GPU, audio device, or live server.
- Use `hecs` where entity component storage and bulk systems provide real value.
- Keep renderer, audio, platform, and control runtime resources out of
  canonical world state.
- Let `bbb-control` snapshots and counters be derived views, not independent
  sources of truth.
- Keep module ownership clear enough for parallel agents to work without
  creating artificial split-only churn.

## Non-Goals

- Do not wrap or embed another complete Minecraft client stack.
- Do not mirror vanilla Java packages one-to-one.
- Do not implement Microsoft/Mojang online account authentication; the playable
  networking target is offline-mode Java 26.1 servers.
- Do not add in-game configuration UI; runtime configuration is supplied at
  command-line startup.
- Do not put GPU buffers, audio handles, threads, windows, or platform objects in
  `bbb-world`.
- Do not put every state category into `hecs`. Use ECS for entity domain state,
  not for chunks, chat, scoreboard, recipes, assets, or UI state.
- Do not treat `NetCounters` as canonical gameplay or client state.

## High-Level Data Flow

```text
server bytes
  -> bbb-protocol decode
  -> bbb-net event stream / probe
  -> bbb-native dispatcher
  -> bbb-world apply APIs
  -> pure client systems
  -> derived command queues / projections
  -> bbb-renderer, bbb-audio, bbb-control
```

The important rule is one-way ownership:

- Packets enter at the protocol/net boundary.
- `bbb-world` owns canonical client state.
- Systems update canonical state or emit runtime commands.
- Renderer/audio/control read state or commands and keep backend-specific
  resources outside the world store.

## Crate Responsibilities

### `bbb-protocol`

Owns the wire format.

- Packet ids, structs, encode/decode, flags, enum ordinals, nullable fields.
- Tests with representative vanilla-shaped payloads and edge cases.
- No runtime behavior and no canonical world ownership.

Packet structs may be used by world apply APIs during the transition, but packet
decode details should not leak into renderer/audio/control code.

### `bbb-net`

Owns connection lifecycle and packet/event transport.

- Login/configuration/play flow, keepalive, ping, compression, resource-pack
  responses, cookie responses, and command sending.
- Online event stream and offline probe behavior.
- Converts decoded packets into `NetEvent` values.
- Does not own canonical client state.

Offline probes should apply every already-decoded packet that has a world apply
API. Ignoring packets in probe code is a temporary gap, not the target design.

### `bbb-world`

Owns canonical client state.

- Level metadata, dimension, time/weather/ticking.
- Chunks, terrain state, block entities, and chunk view.
- Entity state through `EntityStore` and `hecs` components.
- Client state such as chat, HUD, UI, audio events, visual effect events,
  scoreboard, player info, recipes, advancements, maps, waypoints, world border,
  registries, and resource-pack presentation state.
- Counters for received/applied/ignored world events.
- Narrow query APIs used by native runtime, renderer, audio, and control.

`WorldStore` must stay deterministic and serializable. It may store decoded
semantic state and lightweight event summaries. It must not store backend
handles.

### `bbb-pack`

Owns vanilla 26.1 resource-pack and asset loading.

- Registries and tag data needed by client behavior.
- `sounds.json`, sound event lookup, subtitles, and resolved `.ogg` resource
  paths.
- Blockstates, models, item model definitions, particle definitions, waypoint
  styles, textures, atlases, colormaps, language files, and render metadata.
- Stable id or handle lookup APIs for renderer/audio/world systems.

The asset pipeline should follow official 26.1 resource-pack semantics. Avoid
hard-coded resource paths except in tests or explicit bootstrap constants.

### `bbb-renderer`

Owns GPU-facing rendering.

- Terrain mesh generation, render layers, light/tint data, atlas usage.
- Entity rendering, particles, overlays, HUD geometry, and selection outlines.
- Backend GPU buffers, pipelines, caches, and frame scheduling.

Renderer state is runtime state. It may cache data derived from `bbb-world` and
`bbb-pack`, but `bbb-world` remains canonical.

### `bbb-audio` or Native Audio Runtime

Target audio should be a separate playback boundary. This may be a future
`bbb-audio` crate or a narrow native runtime module until it grows.

- Input: audio commands derived from world/net events.
- Backend: Kira is appropriate here, not inside `bbb-world`.
- Owns audio manager, tracks, sound handles, spatial/following sound state,
  fade/stop behavior, and device errors.
- Uses `bbb-pack` to resolve vanilla sound events through `sounds.json` and
  resource-pack lookup.

`bbb-world::client::audio` should keep protocol/canonical audio event state only:
last sound event, last entity sound event, stop sound event, counters, and
vanilla no-op checks such as unknown sound entity handling.

### `bbb-control`

Owns external control-facing types, reports, and snapshots.

- `NetCounters` and snapshot structs are derived views.
- Control state must be synchronized from `WorldStore` and runtime status.
- New packet behavior should not add native-only snapshot ownership when a world
  canonical state can own it.

### `bbb-native`

Owns runtime orchestration.

- Main event loop, input/camera integration, upload scheduling, platform
  boundary use, command queues, and integration tests.
- Applies `NetEvent`s to `WorldStore`.
- Runs systems or forwards derived commands to renderer/audio/control.
- Does not duplicate world-owned canonical state.

### `bbb-item-model`

Owns value-aware item-model resolution.

- Icon selection, item model definition consumption, and display transforms,
  plus the profile/skin download runtimes that feed player item and entity
  rendering.
- Exposes `NativeItemRuntime` and its context/result types; consumed by the
  `bbb-native` runtime, scene, and HUD paths.
- Depends on `bbb-pack`, `bbb-protocol`, `bbb-renderer`, `bbb-world`, and
  `bbb-audio`; holds no runtime orchestration or canonical world state.

### `bbb-platform`

Owns OS/window/input integration primitives. It should expose narrow runtime
interfaces and avoid Minecraft-specific state ownership unless there is a clear
platform reason.

## Mapping From Vanilla Client Architecture

Vanilla class or manager names are behavioral references, not module names to
copy blindly.

| Vanilla 26.1 concept | BBB target owner |
| --- | --- |
| `ClientPacketListener` | `bbb-net` event stream plus `bbb-native` dispatcher |
| `ClientLevel` | `bbb-world` level/chunk/entity state plus pure systems |
| Java `Entity` subclass hierarchy | `hecs` entity components plus entity systems |
| `LocalPlayer` | `bbb-world::client::local_player` and native input systems |
| `SoundManager` | `bbb-audio` / native audio runtime |
| `ParticleEngine` | `bbb-renderer` particle runtime |
| `LevelRenderer` | `bbb-renderer` terrain/entity/effect rendering |
| `Gui` / HUD classes | `bbb-world` HUD/UI state plus renderer presentation |
| Resource managers and packs | `bbb-pack` |
| Client options affecting rendering/audio | Runtime config; derived effects stay outside `bbb-world` |

When vanilla behavior depends on a specific class check, keep that semantic
check in the Rust owner. Example: `ProjectilePower` updates only an existing
`AbstractHurtingProjectile`; unknown entities or non-projectile entities are
vanilla no-ops.

## State Categories

Classify new state before implementing it.

### Canonical State

State that represents the client's authoritative understanding of the server or
client view.

Examples:

- Entity transform, metadata, equipment, effects, attributes.
- Chunks, block updates, block entities, map state.
- Scoreboard, player info, recipes, advancements.
- Chat messages, deletion records, signature cache.
- Current title/action bar/HUD values.

Canonical state belongs in `bbb-world`.

### Derived Snapshot State

State formatted for external APIs, tests, logs, or probes.

Examples:

- `NetCounters.last_sound`.
- `NetCounters.last_player_chat`.
- Control report structs.

Derived state belongs in `bbb-control` or native projection code and must be
rebuilt from `bbb-world` where possible.

### Transient Observed Events

Events that vanilla handles immediately but are still useful for probes,
snapshots, or runtime command generation.

Examples:

- Sound and stop-sound packets.
- Explosion and level-particle packets.
- Debug/game-test packets.

Store lightweight summaries or bounded event buffers in `bbb-world` only when
they help testing, runtime command generation, or control projection. Do not
store large raw payloads indefinitely.

### Backend Runtime State

State tied to hardware, threads, windows, or frame/audio lifetimes.

Examples:

- GPU buffers, bind groups, pipelines, swapchains.
- Kira `AudioManager`, sound handles, mixer tracks.
- Window handles, input device state, upload queues.

Backend runtime state belongs in renderer/audio/native/platform, not in
`bbb-world`.

## Entity Architecture And `hecs`

`hecs` should remain the entity-domain storage backend. The current design uses
`EntityStore` with:

- `hecs::World` for component storage.
- `protocol entity id -> hecs::Entity` map for packet lookup.
- Stable protocol-order tracking for deterministic snapshots and tests.

This is valid, but new entity work should move toward real ECS usage instead of
only using `hecs` as a componentized object map.

### Component Ownership

Prefer small semantic components:

- `EntityIdentity`: protocol id, UUID, type id, spawn data.
- `EntityTransform`: position, rotation, velocity, on-ground state.
- `EntityMetadata`: metadata values.
- `EntityEquipment`: equipment slots.
- `EntityAttributes`: attribute state.
- `EntityMount`: vehicle/passenger graph state.
- `EntityLeash`: leash holder.
- `EntityMobEffects`: active effects.
- `EntityDamage`: last damage event state.
- `EntityMinecartLerp`: minecart interpolation steps.
- `HurtingProjectile` or `ProjectileAcceleration`: acceleration power and other
  projectile-specific state.

Do not add new fields to a catch-all `EntityState` first if a component already
owns the semantic area. `EntityState` is useful for serialization, tests, probe
output, and compatibility, but hot paths should avoid projecting full state.

### Entity Apply Rules

Packet application should usually follow this pattern:

1. Increment received packet counter.
2. Resolve protocol id to `hecs::Entity`.
3. Check required component or vanilla type semantics.
4. Mutate the narrow component.
5. Increment applied or ignored counter.
6. Leave full `EntityState` projection for query/report boundaries.

Example target for `ProjectilePower`:

- Received packet increments `projectile_power_packets`.
- If entity is missing, increment `projectile_power_updates_ignored`.
- If entity exists but is not a hurting projectile, increment ignored.
- If entity has `HurtingProjectile`, update `acceleration_power` and increment
  applied.
- `NetCounters.last_projectile_power` is projected from the world update summary
  or component state, not maintained as independent native state.

### Query And Performance Policy

Use direct component access for single-entity packet updates. Use `hecs` queries
for bulk systems.

Good uses:

- Single packet update: protocol id lookup, then `get::<&mut EntityTransform>`.
- Bulk movement/interpolation: query entities with transform and lerp
  components.
- Expiring effects: query entities with `EntityMobEffects`.
- Rendering extraction: query the exact renderable component tuple.

Renderer-facing entity extraction should read canonical `WorldStore`
projections instead of exposing `hecs` handles. The current basic visibility
path lets `bbb-native` convert `WorldStore::entity_pick_targets_at_partial_tick`
into a renderer-owned bounds outline, skipping the local player and current
camera entity. Dropped item visibility uses
`WorldStore::item_entity_stacks()` to expose only canonical item entity stack
metadata and position; `bbb-native` resolves those stacks through
`NativeItemRuntime`, then `bbb-renderer` draws camera-facing icon billboards
from the item atlas. These are temporary scene proxies; full entity models,
ground-context dropped item rendering, equipment, skins, lighting, animation,
culling, and draw ordering remain renderer-owned follow-up work.

Avoid:

- Projecting every entity to `EntityState` during hot updates.
- Cloning large component collections just to update one field.
- Giving every entity every future component when the component is only valid
  for a small vanilla type family.

## Chunks And Terrain

Chunks should not use `hecs`.

Target chunk architecture:

- Region/chunk/section storage optimized for block-state lookup and updates.
- Palette and heightmap data shaped by vanilla 26.1 packet semantics.
- Block entities stored with chunk-local lookup.
- Renderer-facing extraction from chunk sections into mesh inputs.
- Counters for received/applied/ignored updates and chunk lifecycle.

World chunk state is canonical; renderer mesh caches are derived runtime state.

## Audio Architecture

Audio has two separate layers.

### World Audio State

`bbb-world` owns packet-derived audio observations:

- Last positioned sound event.
- Last entity-bound sound event.
- Last stop-sound event.
- Packet counters.
- Vanilla no-op semantics that depend on canonical world state, such as ignoring
  entity-bound sound for unknown entities.

This layer is deterministic and testable.

### Audio Runtime

The future audio runtime owns playback:

- Kira `AudioManager`.
- Decoded sound data cache.
- Sound instance handles.
- Category/track mapping for vanilla `SoundSource`.
- Entity-following sound handle updates.
- Stop-by-name, stop-by-source, and stop-all behavior.
- Runtime metrics and device errors.

The audio runtime should consume commands such as:

```text
AudioCommand::PlayPositionedSound
AudioCommand::PlayEntitySound
AudioCommand::StopSound
AudioCommand::TickEntitySoundPositions
```

These commands are derived from `bbb-world` and `bbb-pack`; they are not the
canonical state themselves.

## Visual Effects And Particles

Explosion and level-particle packets are transient client effects in vanilla.
They should not directly mutate canonical block state.

Target ownership:

- `bbb-world` may record lightweight observed-event state such as
  `last_explosion`, `last_level_particles`, packet counters, and optional
  bounded history.
- `bbb-native` resolves `LevelParticles` packets through vanilla 26.1 particle
  type order and `bbb-pack` particle definitions plus particle atlas sprites,
  then submits renderer-owned spawn batches. Missing particle sprites are
  reported as diagnostics while the spawn command is preserved for renderer
  fallback behavior.
- `bbb-renderer` keeps a bounded pending spawn queue so particle drawing consumes
  renderer-owned commands instead of native/world snapshots.
- `bbb-renderer` drains pending spawn commands into active CPU-side particle
  instances through the native runtime pump, advances instance age only on the
  native 20Hz client tick path, applies data-only provider/lifetime descriptors
  for common 26.1 particles, applies no-collision gravity/friction motion, tracks
  current sprite ids with vanilla SpriteSet age/random selection rules, samples
  provider-shaped billboard size/color and age-size curves for common particles,
  and reports active/intake/expired/drop counters. Native uploads a stitched
  official particle atlas when assets are available, and renderer draws active
  instances as camera-facing textured billboards. The active instance state and
  GPU resources are renderer runtime state, not canonical world state.
- `bbb-renderer` owns actual particle creation, culling, settings, distance
  limits, GPU buffers, lifetime ticking, and future vanilla presentation parity.
- Local-player knockback from explosion is a gameplay/client movement semantic;
  `bbb-world` applies finite knockback to canonical local-player delta movement
  while visual explosion presentation remains renderer-owned.

Do not mix visual effect state with audio state. Do not put projectile gameplay
updates into effect modules.

## Assets And Resource Packs

Asset behavior should align with official 26.1 resource-pack rules.

Target pipeline:

1. Load vanilla 26.1 assets as the baseline pack.
2. Layer server-provided or user-selected packs according to vanilla precedence.
3. Build lookup tables for registries, blockstates, models, item model
   definitions, particle definitions, particle atlas sprites, waypoint styles,
   textures, atlases, colormaps, sounds, language keys, and tags.
4. Expose stable ids or handles for renderer/audio/world logic.
5. Keep pack parsing and cache invalidation in `bbb-pack`.

Renderer/audio code should ask `bbb-pack` for resolved resources. Avoid
duplicating resource-path logic in runtime systems.

## Control Snapshots And Counters

`NetCounters` is a control/reporting projection.

Rules:

- New world-relevant packet state should be stored in `bbb-world`.
- Native control projection should call a `sync_*_counters` helper that copies
  counters and last-state summaries from `WorldStore`.
- Native-only counters are allowed only for connection/session/runtime state
  that `bbb-world` should not own, such as connection status or command queue
  lengths.
- When a native-only field becomes world semantic state, migrate it into
  `bbb-world` and leave `NetCounters` as derived projection.

This avoids stale snapshots where native counters and world state disagree.

## Systems Layer

Pure systems should be introduced when behavior is more than a direct packet
application.

Candidate systems:

- Entity interpolation and movement.
- Minecart lerp stepping.
- Local-player movement and look-at updates.
- Projectile acceleration/ticking.
- Mob-effect ticking when needed.
- Audio command extraction.
- Render extraction.

Systems should be deterministic functions over world state and small runtime
inputs. They should be testable without network, renderer, or audio backends.

## Testing Strategy

### Protocol Tests

- Packet ids and dispatch.
- Wire order and nullable fields.
- Enum ordinals and flags.
- Boundary lengths and representative payloads.

### Pack Tests

- Vanilla asset path resolution.
- `sounds.json` parsing and sound-event lookup.
- Model parent and texture resolution.
- Atlas and colormap inputs.

### World Tests

- Packet apply APIs.
- Canonical state updates.
- Applied/ignored counters.
- Unknown entity no-op behavior.
- Serialization and deterministic query output.

### Entity/System Tests

- Component-level updates without full `EntityState` projection.
- `hecs` query behavior for bulk systems.
- Vanilla type checks such as projectile-specific updates.

### Native/Net Tests

- Event forwarding and drain behavior.
- Probe applying already-decoded packets.
- Command queue behavior.
- Control projection from `WorldStore`.

### Renderer/Audio Tests

- Use trait or mock backends for logic tests.
- Do not require a real GPU or audio device for normal unit tests.
- Manual or screenshot/audio smoke checks may exist separately from the default
  workspace gate.

## Migration Plan

### Immediate

- Keep moving native-owned snapshot state into `bbb-world`.
- Make `NetCounters` derive from world state for chat, audio, visual effects,
  debug/game-test, combat, block ack, and entity update summaries.
- Fix known canonical gaps such as `ResetChat` clearing world chat state.
- Keep `ProjectilePower` out of visual effects; migrate it into entity projectile
  state.

### Short Term

- Add focused world modules for remaining packet families only when they have a
  real semantic owner.
- Add applied/ignored counters where vanilla does no-op checks.
- Convert hot entity update paths to direct component mutation.
- Use `hecs` queries for bulk entity systems.
- Ensure offline probe applies all packets that have world apply APIs.

### Medium Term

- Introduce an explicit systems layer if behavior outgrows direct apply methods.
- Build `bbb-pack` lookup APIs around official 26.1 assets.
- Add the audio runtime boundary, with Kira behind a feature or isolated crate.
- Add renderer extraction APIs that avoid large world clones.

### Long Term

- Complete vanilla asset/render alignment.
- Replace transitional protocol-packet exposure in world APIs with narrower
  semantic update structs where that improves stability.
- Keep `WorldStore` serializable and backend-independent.
- Keep control snapshots purely derived from world/runtime state.

## Implementation Checklist For New Features

Before implementing a new clientbound behavior, answer:

1. What vanilla class or method defines the behavior?
2. Is the result canonical world state, transient observed event, backend runtime
   state, or control-only derived state?
3. Which crate owns it?
4. Does an unknown entity, missing registry entry, disabled option, or type check
   make vanilla no-op?
5. Are applied/ignored counters needed?
6. Can the hot path update a narrow component instead of projecting full state?
7. Which focused tests prove the behavior?
8. Does the change need a new module under the rules in
   `docs/code-organization-style.md`, or should it stay near the existing owner?

If the answers are unclear, add the smallest deterministic world state first and
defer runtime backend integration.
