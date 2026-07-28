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
- Do not implement Microsoft/Mojang account sign-in or access-token acquisition.
  Authenticated sessions accept an existing profile UUID and access token at
  command-line startup; offline-mode Java 26.1 servers remain the reproducible
  playable-test target.
- Do not add in-game configuration UI; runtime configuration is supplied at
  command-line startup.
- Do not collect or upload Mojang client telemetry. Protocol fields that vanilla
  feeds only into telemetry remain decoded diagnostics; they do not create an
  external telemetry runtime owner.
- Do not put GPU buffers, audio handles, threads, windows, or platform objects in
  `bbb-world`.
- Do not put every state category into `hecs`. Use ECS for entity domain state,
  not for chunks, chat, scoreboard, recipes, assets, or UI state.
- Do not treat `NetCounters` as canonical gameplay or client state.

## Multiplayer-Only Authority Boundary

`bbb` is an external multiplayer client. It does not own an `IntegratedServer`,
singleplayer save, server chunk cache, or authoritative entity/world simulation.

| Client responsibility | Required behavior | Authority limit |
| --- | --- | --- |
| Local player prediction | input, movement, collision, fluids, jump/sprint and serverbound movement | server corrections and acknowledgements replace predicted state |
| Locally controlled vehicle | run the vanilla local-authority movement branch and send vehicle movement | remote vehicles consume server transforms and interpolation only |
| Remote entities | retain canonical packet state and interpolate transforms | no client-authoritative AI, pathfinding, collision, damage, spawning or loot |
| Client entity tick | run subclass ticks that `ClientLevel.tickEntities` executes for animation, particles, audio, interpolation and projectile presentation | hits, damage, explosions, block changes and authoritative removal remain server-owned |
| Debug/save inspection | ordinary F3+I uses the matching server tag-query response as complete NBT authority; Shift-local output may contain only actually observed client state | never fill server-private fields from constructor defaults, source expressions or heuristics |

`GoalSelector`, `PathNavigation`, mob decisions, breeding, spawning, loot,
authoritative combat, and server RNG state are not missing client features. A
client-side projectile tick may advance motion/trails between packets, but it
must not decide hits, damage, explosions, world changes, or authoritative
removal.

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
- Server-directed Transfer drains only its bounded StoreCookie/latest-Transfer
  closing tail, reconnects with the Transfer handshake intent, and re-enters
  Login/Configuration/Play in the same event stream and native process.
- Online event stream and offline probe behavior.
- Converts decoded packets into `NetEvent` values.
- Does not own canonical client state.

Play-state packets flow as `NetEvent::Play(packet)`; both the offline probe and
the online dispatcher delegate to the shared `WorldStore::apply_play_packet`
table, so per-packet apply arms exist in exactly one place. Connection-owned
packets (keepalive, cookies, chunk-batch feedback, movement responses,
configuration handoff) are returned to the caller and stay net-owned.

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

The synchronized `Commands` graph is instantiated as one immutable world-owned
dispatcher. `bbb-protocol` decodes the generated 57-entry argument-template
catalog; `bbb-world` performs parsing, redirect traversal, validation, signable
argument extraction, and completion against the exact frozen registries, tags,
enabled features, player/scoreboard state, and resource-derived sound-id view.
`bbb-native` owns only editing and suggestion presentation. It consumes the
world result directly, sends a request only for `minecraft:ask_server`, an
unknown provider, or an explicitly server-owned parser such as objective names,
and keeps the existing request-id gate for those responses.
The sound-id view is a serde-omitted `PublicationShared<Vec<String>>`: reload
replaces that derived session owner, unchanged publications share it, and old
world generations cannot observe its mutation. No parser, registry-order cache,
or command authority is independently mutable outside the simulation owner.

### ChatListener Presentation Ownership

`bbb-world::ClientChatState` is the only mutable chat-listener owner. Packet
receipt validates signatures and captures one bounded delivery value; the same
delivery function applies immediate, timed, explicit-next, and flush paths. It
alone updates visible history, trust/source tags, the reporting ring, last-seen
acknowledgement state, and language-neutral narration requests. Hidden/blocked,
secure-only, filter, delay, deletion, configuration handoff, and level/session
reset decisions therefore cannot diverge between online, probe, and fixture
paths.

`bbb-protocol` retains exact trusted Component NBT plus inherited styled runs,
click/hover payloads, and font identity. `bbb-native` resolves resource-pack
localization and projects one stable rich-message cache; `bbb-renderer` owns only
bounded wrapped glyph lines, hover geometry, and revision-scoped hit tokens.
Mouse actions return a narrow `(level, delivery, resource generation,
resource-loaded state, message, run)` token through the simulation mailbox. The
simulation owner revalidates it against the current canonical message before
sending a command or platform effect, so a level reset, resource reload, delayed
delivery, or retained old publication cannot retarget a click.

Chat/system narration drains after packet/input/tick mutation into the existing
lossless effect collector. The batch is sealed before its matching immutable
publication, preserving effect-before-publication even when render is blocked.
The native narrator process owns only queued speech and applies Vanilla's
off/all/chat/system gates; it is neither canonical chat state nor an audio-mixer
owner. Font reload transactionally rebuilds renderer-owned wrapped lines and
restores the exact prior cache on failure.

A fresh Login starts a new registry session, while Play reconfiguration carries
the frozen registry access. Every connection edge clears connection-owned
resource-pack tracking; level replacement clears chunk/entity/local-level
owners. Vanilla deliberately carries the prior server-list presentation across
Transfer because the target skips `ServerData` for a transferred login. These
transitions mutate the one simulation-owned `WorldStore`; renderer and control
observe their result through the same immutable publication generation.

### `bbb-pack`

Owns vanilla 26.1 resource-pack and asset loading.

- Registries and tag data needed by client behavior.
- The sole file-backed PNG decoder and nested per-image, per-atlas and shared
  resource-generation logical RGBA admission budgets; hostile image policy is
  specified in
  [`resource-input-safety-design.md`](resource-input-safety-design.md).
- `sounds.json`, sound event lookup, subtitles, and resolved `.ogg` resource
  paths.
- Blockstates, models, item model definitions, particle definitions, waypoint
  styles, textures, atlases, colormaps, language files, and render metadata.
- Stable id or handle lookup APIs for renderer/audio/world systems.

The asset pipeline should follow official 26.1 resource-pack semantics. Avoid
hard-coded resource paths except in tests or explicit bootstrap constants.
The selected-stack, reload-generation, staging and dynamic-texture migration
contract is [`resource-pack-lifecycle-design.md`](resource-pack-lifecycle-design.md).
Paths are resource locations only after a validated selected pack gives them a
source identity and priority; renderer/audio handles remain derived owners and
become visible only at the matching reload commit.
`PackRoots` carries an `Arc<ClientResourceGeneration>` for admitted runtime
packs. That generation owns the exact selected stack plus one precompiled
`PackResourceStack`: pack metadata bytes, overlay roots and resource-filter
regexes are admitted once, then every language/model/atlas/sound consumer gets
a cheap clone of the same immutable lookup owner. The directory list retained
for serde compatibility is a projection, not the runtime selection authority.
The same generation owns one 512 MiB logical decoded-RGBA counter shared by
atlas, standalone texture, colormap, font, entity, HUD, sky and profile-resource
consumers. Native derives the staged generation allowance from the accepted
generation's exact charge before preparation, enforcing a 1 GiB adjacent-owner
ceiling without releasing the rollback source. Failed decodes retain their
conservative charge until the staged generation is dropped. Exact current/peak
logical bytes are immutable diagnostics, not allocator or GPU-memory estimates;
the detailed contract and measurements are in
[`resource-image-memory-budget-design.md`](resource-image-memory-budget-design.md).
The generation also owns one independent 512 MiB logical resource-data counter.
Every production pack JSON read is admitted at the opened-file boundary, then
charges parsed nodes, decoded strings, bounded listing results and retained
model/blockstate/item/font/sound collection estimates to that exact owner.
Selected plus staged data generations are capped at 1 GiB without releasing the
rollback source. Semantic collection limits are checked before downstream bake
or atlas work; sticky rejection prevents an optional malformed-resource path
from committing a partial budget-exhausted generation. The complete limits and
local official-tree measurements are in
[`resource-data-audio-budget-design.md`](resource-data-audio-budget-design.md).
Renderer reload is a synchronous owner transaction: accepted GPU handles are
retained in a private checkpoint while the next particle/item, terrain,
HUD/font, entity and sky owners are built. Failure restores those exact handles
without old-pack re-upload; success drops the checkpoint only after item/map
generation invalidation. Audio commit and the simulation acknowledgement follow
that renderer barrier, so readers cannot observe a cross-generation mixture.
Downloaded skin/cape/elytra textures share one bounded item-runtime request
owner keyed by exact resource and connection-session generation. Renderer
dynamic entity images/atlases and pack-backed entity variants cross the same
generation edge before accepting new uploads; disconnect, reconnect and reload
therefore discard late results instead of reviving a prior handle.
That runtime admits at most 256 MiB of ready decoded pixels with LRU eviction of
non-pending owners. A rendezvous result handoff and one-result-per-render-pump
delivery bound prevent an unbounded decoded-result queue or drain clone; stale
or unadmittable values keep the established failed/fallback semantics.
Control receives only immutable observations: the selected pack list and last
reload result use one session-static `Arc` so publication cloning does not copy
pack ids/strings, while dynamic texture state/bytes are sampled into latest-value
backend telemetry without waking simulation. The JSON projection exposes no pack
filesystem path, decoded image, renderer/audio handle or mutable selection API.
Missing resource-id diagnostics and new non-profile texture producers extend
this observation surface; they do not create another lifecycle owner.

### `bbb-renderer`

Owns GPU-facing rendering.

- Terrain mesh generation, render layers, light/tint data, atlas usage.
- Entity rendering, particles, overlays, HUD geometry, and selection outlines.
- Backend GPU buffers, pipelines, caches, and frame scheduling.

Frame structure invariants (enforced by tests in `render.rs`):

- `render()` is a pure orchestrator over ordered step methods; the sequence is
  declared in `FRAME_STEPS` and a meta test pins call order, definition order,
  and the absence of direct GPU commands in the orchestrator.
- Render pipelines are created through `pipeline_builder::RenderPipelineBuilder`;
  per-frame rebuilt vertex/index streams reuse `frame_buffers::FrameDataBuffer`
  instead of allocating fresh buffers each frame.

Renderer state is runtime state. It may cache data derived from `bbb-world` and
`bbb-pack`, but `bbb-world` remains canonical.

### `bbb-audio` Playback Runtime

Audio is a separate playback boundary. The `bbb-audio` crate owns it, with Kira
behind a default `kira` feature and the device backend behind `device`.

- Input: audio commands derived from world/net events.
- Backend: Kira is appropriate here, not inside `bbb-world`.
- Owns audio manager, tracks, sound handles, spatial/following sound state,
  fade/stop behavior, generation-scoped decoded residency, and device errors.
- Uses `bbb-pack` to resolve vanilla sound events through `sounds.json` and
  resource-pack lookup.

Before playback, the exact selected OGG path is opened through a 64 MiB bounded
media source and must prove sample rate, mono/stereo shape, finite duration and
decoded-frame limits. Non-streaming sounds decode once into a cache-owned
`Arc<[Frame]>`; simultaneous instances clone only that owner and keep
volume/pitch/loop settings instance-local. The cache key is `(resource
generation, exact path)`, retains at most 65,536 owners / 512 MiB of decoded
frames, and evicts only entries whose frame Arc is not pinned by playback.
Streaming uses the same admission plus a 65,536-frame output-chunk ceiling, but
does not retain a complete decoded frame owner. Reload stops old handles, clears
delayed commands and cache ownership, then installs the new generation; an old
command cannot hit or repopulate the new cache. Current/peak bytes and
hit/miss/eviction/rejection counters are backend telemetry copied into the next
immutable publication, not canonical world state.

`bbb-world::client::audio` should keep protocol/canonical audio event state only:
last sound event, last entity sound event, stop sound event, counters, and
vanilla no-op checks such as unknown sound entity handling.

### `bbb-native` Narrator Runtime

Narration is a native runtime/platform boundary, separate from game audio and
canonical world state.

- Input owners retain language-neutral narration payloads; native resolves them
  through the selected resource-pack language before submitting speech.
- `NarratorStatus` follows vanilla's off/all/chat/system gates. Interrupting
  system narration clears the active utterance before starting its replacement.
- Screen projections emit localized narration frames keyed by Vanilla's element
  kind and nesting depth. The native collector retains one value per key, orders
  title/position/hint/usage output, and only repeats values after a content change
  or a generation in which that key was absent.
- Screen narration timing is native runtime state: opening a screen forces the
  initial frame, mouse motion restarts the 750 ms delay, mouse/keyboard actions
  restart the 200 ms delay and clear initialization suppression, and ordinary
  initialization suppresses delayed repeats for two seconds.
- The TTS adapter is configured only at startup with `--narrator-command` and
  repeated `--narrator-command-arg` values. An exact `{text}` argument is
  replaced with the utterance; without a placeholder the utterance is appended.
- The process child and pending utterances are runtime state. They must not be
  stored in `bbb-world`, routed through the Kira audio mixer, or represented as
  chat/log output when no TTS adapter is configured.

### `bbb-control`

Owns external control-facing types, reports, and snapshots.

- `NetCounters` and snapshot structs are derived views.
- Native owns the mutable `WorldStore`; every named-thread
  `SimulationCore::step` seals an immutable `SimulationPublication` independently
  of render work. Renderer and control reuse the exact same world
  `Arc`, runtime counters, and generation through paired `ArcSwap` handles.
  Control reads retry on a generation mismatch and never lock or own the live
  world.
- `WorldStore::publication_snapshot` retains immutable owners for unchanged
  chunk columns, the committed registry set, and individual item-profile
  tables. Entity metadata/painting, equipment, attributes, passengers,
  effects, damage, and minecart-step payloads use the same per-component-owner
  sharing. Mutations copy only their semantic owner; each publication still
  rebuilds the entity ECS shell and protocol-id handles. This storage detail
  does not create a second canonical world or a control-specific copy.
- `SharedControlRequests` is a producer facade: native installs the single
  simulation mailbox sink before the control server starts, so all control
  writes share the same ordered command path as winit input. Its compatibility
  queues exist only for focused tests and compatibility adapters; a read
  snapshot never becomes a second gameplay authority.
- New packet behavior should not add native-only snapshot ownership when a world
  canonical state can own it.

### `bbb-native`

Owns runtime orchestration.

- Main event loop, input/camera integration, upload scheduling, platform
  boundary use, command queues, and integration tests.
- Routes `NetEvent::Play` into `WorldStore::apply_play_packet`, providing the
  runtime `PlayApplyEffects` sinks (audio, particles, acknowledgements).
- AddEntity admission is world-owned: the raw wire holder resolves through the
  frozen session EntityType registry and exact generated resource identity
  before the canonical entity is replaced. PlayerInfo and FishingHook owner
  gates run before replacement. Packet-caused LlamaSpit particles are sealed
  after insertion, while minecart/bee ticking sounds remain canonical,
  entity-bound world owners whose backend handles are discardable derivatives.
- Runs systems or forwards derived commands to renderer/audio/control.
- Does not duplicate world-owned canonical state.

The frozen simulation/render ownership and migration contract is
[`simulation-render-thread-design.md`](simulation-render-thread-design.md).
Its target gives the mutable world and canonical input/tick path to one
simulation thread; the winit thread retains window, renderer, GPU upload,
platform, and backend resources and consumes immutable publications plus
ordered effects. Migration phases must not create selectable direct/threaded
gameplay paths. The named `bbb-simulation` thread's `SimulationCore` is the sole
field owner for the live world, input, network receiver/counters, and tick state.
`SimulationCore::step` borrows no renderer, audio backend, particle renderer,
or resource uploader. A condition-variable scheduler runs it at the next client
tick deadline and wakes immediately after ordered mailbox or forwarded network
work becomes visible. It drains one monotonic `SimulationMailbox` after network
application and before client ticks; winit events, control writes, immutable font
metrics, platform results, reload acknowledgements, and shutdown all use that
path. It resolves packet/tick/platform side effects into one owned,
sequence-numbered unbounded `RenderEffectBatch` stream; the winit owner consumes audio,
particles, window, clipboard, URI, stats-screen narration input, debug-renderer,
screenshot, and reload work at most once in FIFO order before render extraction.
The stream records current and peak pending output counts without blocking its
producer. Slow-frame diagnostics pair those counts with the newest immutable
publication's generation lag and ordered-effect sequence lag, so renderer/GPU
stalls are measurable while the simulation remains deadline-driven.
Resource reload is the terminal batch effect: render-owned asset/GPU work returns a
generation acknowledgement, and only a matching mailbox acknowledgement
commits world defaults and the simulation particle resolver. The
simulation-side resolver and render-side particle atlas runtime are distinct
owners. Item and terrain pack data are copied into independent simulation
projections at startup/reload; renderer async/GPU caches are never shared or
locked across the boundary. Render extraction, particles,
terrain upload, cursor/overlay decisions, narration, and control reads consume
one immutable publication; a source guard rejects event-loop reads of live
simulation fields. Its effect sequence prevents render from adopting a newer
publication until all preceding effects have been consumed. The event loop
joins clean shutdowns and propagates caught worker failures; `SimulationThread`
also joins on drop. `SimulationRenderContext` carries backend/cache owners only.
The event loop has no fallback direct pump and does not consult the control
snapshot as a second lifecycle authority.

`SimulationPublisher` builds the immutable world only through the structural
snapshot boundary and records both complete publication construction and the
contained entity-shell snapshot duration. The tick owner separately records
the most recent non-empty entity client-tick batch. Latest/process-peak
nanoseconds and logical scanned, mutated, and rebuilt entity counts travel in
the same generation's `NetCounters`; observation neither blocks the simulation
nor introduces a separate reader publication.

The publication carries `RenderInputPublication`, an explicit immutable
projection of only keys/buttons and screen/debug presentation state consumed by
render or the event loop. Movement accumulators, click timers, network request
queues, debug query/crash state, and other simulation-only heaps are not cloned
at this boundary. `SimulationFrameState` is borrowed from the loaded generation
during extraction; render does not clone its particle context vectors.

Entity rendering uses a render-owned `EntityRenderExtractionScratch`. One hecs
batch query fills narrow identity/transform/bounds/culling rows, and the native
owner applies vanilla distance, frustum, compiled-terrain, camera/local-player,
spectator, passenger, leash, and generated renderer-class exceptions before
full model-source projection. Only survivors are projected. Candidate, survivor,
id-index, model-source, and final entity-instance allocations are cleared and
reused; the renderer swaps its previous instance allocation back to this owner.
The latest/peak phase durations, row counts, and capacity-growth count are
published in the next immutable generation's renderer telemetry.

Heap-heavy non-entity extraction uses a separate render-thread scratch owner.
Bed, sign board/text, and lectern-book projections are retained behind exact
level, terrain/light, block-entity-data, resource, and text-filter edges;
crumbling and every partial-tick/camera/animation-dependent block entity remain
frame-local. Dynamic debug vectors are cleared and swapped back from the
renderer, while revision-keyed GameTest boxes/text remain separate until both
lanes enter the same ordered mesh builders.

Item and map rendering use a second discardable render-owned residency boundary,
not another gameplay owner. `bbb-item-model` resolves an exact model identity
and dynamic properties without naming GPU resources; native extraction supplies
visibility, ordered targets and narrow transform/tint/light/overlay/foil
instances. `bbb-renderer` keys immutable model-space CPU geometry and persistent
vertex/index owners by the exact resource generation, atlas generation, model
identity, material/geometry class and static phase. Entity and level ids never
identify a mesh. Dynamic pose, camera, partial tick and glint time update only a
grow-only instance stream. Special sheeted-decal foil derives UVs in the shader
from retained model-space coordinates and instance scale.

Canonical map colors remain publication-shared world data with a content
revision and exact patch. The renderer owns bounded fixed-page GPU texture
residency per map lineage and resource-owned background, decoration and font
atlases. An unchanged revision uploads no pixels; a consecutive patch writes
only its rectangle; level reset, unload or resource reload invalidates the
matching derived owner. Map surface, decoration and text geometry use static
quads plus narrow instances. Item/map residency telemetry follows the non-
waking backend-counter path into the next immutable publication, so observation
does not create a render-to-simulation feedback loop.

Terrain extraction keeps immutable CPU snapshots and meshes in the render
owner, never in simulation or control. Meshes are split by section and layer;
the GPU owner key is client-level generation plus chunk/section/layer, and a
render-local monotonic revision replaces only rebuilt section owners. The
canonical world publishes a small COW revision record per chunk: a full-height
baseline plus exact section overrides. Block and standalone-light mutations
copy only affected revision records; chunk palette/light payloads and unchanged
sibling sections stay shared. Chunk disappearance removes only its owners,
resource reload advances every resident section revision, and translucent
camera resort writes index buffers without retransmitting vertices. Logical
section job/upload/resident counters cross through the non-waking backend
telemetry path and are published with the next immutable generation.

One lazy bounded terrain worker may consume only render-materialized immutable
snapshot/atlas Arcs plus the exact dirty section id/revision rows. Its four-entry
local-prediction/nearest-first queue and five-entry result channel are derived
caches, not another world owner. Level/resource/chunk aggregate revisions gate
every result before the render owner merges those sections into its cached
layers; queued reset work is cancelled and in-flight stale work is discarded.
Wgpu resources and GPU upload never leave the render thread. The worker is
named, panic-propagating, stopped, and joined with `TerrainUploadState`
lifecycle.

Remote debug subscriptions follow the same boundary. The simulation thread is
the only request-set/tick/expiry owner; typed block, chunk, UUID, and event maps
live in `WorldStore` and are cleared by their connection/level/chunk/entity
lifecycle hooks. Renderer and control see those maps through the same world
`Arc`. The render owner retains only vanilla's last-looked-at brain UUID, keyed
by client-level generation, and lowers selected values into backend-ready debug
primitives. It never retains subscription maps or reads the live simulation
world.

### `bbb-item-model`

Owns value-aware item-model resolution.

- Icon selection, item model definition consumption, and display transforms,
  plus the profile/skin download runtimes that feed player item and entity
  rendering.
- Exposes `NativeItemRuntime` and its context/result types; consumed by the
  `bbb-native` runtime, scene, and HUD paths.
- Depends on `bbb-pack`, `bbb-protocol`, `bbb-render-types`, and `bbb-world`;
  holds no runtime orchestration or canonical world state, and must not depend
  on `bbb-renderer`.

### `bbb-render-types`

Owns pure value types shared between the renderer and native projection code:
player skin/texture PNG decode (vanilla `SkinTextureDownloader` semantics),
entity skin/equipment texture refs, sprite alpha masks and atlas rects, HUD
glyph metrics, item-frame map decoration textures, and the shared vanilla map
decoration identity/render-metadata catalog.

- Leaf crate: no internal dependencies; `bbb-renderer` re-exports everything at
  its pre-existing paths.
- Exists so `bbb-item-model` can name renderer-facing data without depending on
  the GPU renderer. New shared value types go here, not into `bbb-renderer`.

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

- Control-server query methods such as `audio.last_sound`, which read canonical
  world state (`bbb-world::client::audio`) on demand rather than caching it.
- `NetCounters` connection/session/runtime-status fields.
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

Components are the source of truth. `apply_add_entity` spawns components
directly from the packet, re-adds reset components in place, and
`EntityStore::clone` copies component-by-component; full `EntityState`
projection exists only at the probe/serialize/debug boundary
(`insert_or_replace` remains the serde deserialization path).

Immutable publications rebuild one hecs entity and protocol-id handle per live
entity, but heap-heavy component payloads retain immutable
`PublicationShared<T>` owners. A later write reaches `Arc::make_mut` for only
that metadata/equipment/attribute/passenger/effect/damage/minecart-step field.
Direct transform and animation components remain values, so dense client ticks
do not trigger whole-entity or whole-store COW. The generated metadata schema
resolver uses a process-static reference-only inheritance index; it contains no
copied accessors, defaults, type ids, or independently authored membership.

`ClientboundSetEntityDataPacket` crosses two explicit ownership stages.
`bbb-protocol` preserves exact rich Component state and decodes metadata
ItemStacks through the same bounded component catalog as inventory and
equipment; registry holders remain unresolved wire references there.
`bbb-world` validates every row against the generated accessor schema and then
atomically binds each holder to its exact resource id, source, and frozen
registry revision before mutating `EntityMetadata`. The identity lives inside
the metadata value rather than in a side table. Renderer, audio, debug, and
control therefore cannot reinterpret an old holder ordinal against a newer
session, and a retained publication keeps the complete old value even after
reload, removal, or protocol-id reuse. Renderer, audio, particle, save, debug,
and control consumers stay mechanically covered by the fixed entity coverage
matrix; a new entity or consumer category must reopen that generated proof
boundary.

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
- `EntityDamage`: last damage source/stamp and client invulnerability timer.
- `EntityMinecartLerp`: minecart interpolation steps.
- `EntityHurtingProjectile`: acceleration power and other projectile-specific
  state.

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
- If entity has `EntityHurtingProjectile`, update `acceleration_power` and
  increment applied.
- The control server's `world.last_projectile_power` reads the world update
  summary on demand; it is not maintained as independent native state.

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

The audio runtime owns playback:

- Kira `AudioManager`.
- Decoded sound data cache.
- Exact resource-generation/path cache keys, bounded file/metadata/frame
  admission, inactive-only LRU eviction, and immutable frame sharing.
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
- Native control projection splits two ways:
  `apply_control_projection_event` populates runtime status from `NetEvent`s,
  and control-server methods read `WorldStore` on demand. Neither copies
  world state into a cached snapshot.
- Native-only counters are allowed only for connection/session/runtime state
  that `bbb-world` should not own, such as connection status or command queue
  lengths. Publication build duration is also native runtime state and is
  reported here with the immutable generation it measured.
- When a native-only field becomes world semantic state, migrate it into
  `bbb-world` and leave `NetCounters` as derived projection.

This avoids stale snapshots where native counters and world state disagree.

## Systems Layer

Pure systems should be introduced when behavior is more than a direct packet
application.

Existing systems:

- Render extraction: `bbb-native/src/runtime/render_extract.rs` owns the pure
  `*_for_world` world->renderer projections and the per-frame `RendererFrame`
  snapshot. The pump extracts each field at its client-tick sequence point and
  commits the whole frame through one `apply_renderer_frame` call; new
  world->renderer state goes through `RendererFrame`, not ad-hoc
  `renderer.set_*` calls.

Candidate systems:

- Entity interpolation and movement.
- Minecart lerp stepping.
- Local-player movement and look-at updates.
- Projectile acceleration/ticking.
- Mob-effect ticking when needed.
- Audio command extraction.

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

- Extend the explicit systems layer (render extraction landed 2026-07-02) to
  audio command extraction and entity movement systems as they outgrow direct
  apply methods.
- Build `bbb-pack` lookup APIs around official 26.1 assets.
- Verify each `RendererFrame` field's extraction timing against the vanilla
  tick->render order (fields currently read at their historical sequence
  points, documented on the struct).

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
