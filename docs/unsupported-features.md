# Feature Gap TODO List

What is still missing between `bbb` and Minecraft Java 26.1. Nothing else.

This file holds no completion history. When a slice lands, delete the todo it
closed — never rewrite it into a "done" note. Completed work is recorded by the
code, its tests, `docs/goal-archive.md`, and `git log`.

## Update Rules

- Every entry states what is still missing, positively. Never define remaining
  work by exclusion ("X beyond A, B, C"): the exclusion list grows without bound
  and hides the actual gap. If you need to know what already exists, read the
  code.
- The slice that closes a todo deletes that todo in the same commit.
- Never add an entry describing what was built.
- Every todo carries an owner crate and, where the behavior is vanilla-derived,
  the vanilla 26.1 anchor (class / method / constant) it must follow.
- File and heading order are an ownership inventory, not an execution schedule.
  Ready todos join the global hardest-ready-first selection in `goal.md`; blocked,
  deferred, or prerequisite-incomplete work does not enter that candidate set.
- A ready todo may become an atomic slice or a multi-stage engineering slice. Breadth,
  cross-crate ownership, implementation size, and multiple normal commits are not
  exclusion reasons when the semantic boundary, final completion criteria, phase
  validation, and failure-localizing checkpoints are explicit. Excluding a large
  target requires a real blocker under `goal.md`, not merely "scope too large".
- Two states are not todos, and deleting one silently re-opens a decision that
  was already made:
  - **Deferred** — a real gap, intentionally not scheduled. It stays under its
    owning `# Todo` domain, uses the literal `deferred` marker, and carries an
    explicit `Reopen when ...` condition.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Architecture drift candidates

Owner: workspace production crates; census evidence owned by `scripts/docs_wiki_lint.py`

Matrix route: `world-client-level-decisions`

<!-- architecture-candidates:start -->
<!-- architecture-candidates:end -->
## ClientLevel world and effect decision surface

Owner: `bbb-world` + `bbb-native` + `bbb-renderer` + `bbb-audio`

Matrix route: `world-client-level-decisions`

- **The source-derived `ClientLevel` decision surface is `active`.** Packet,
  entity and registry coverage closes many inputs, but the census cannot infer
  complete chunk/global/map/time/weather/border/block-entity/effect/audio and
  dimension-lifecycle consumers from canonical state alone. Split the 121
  generated owners by homogeneous lifecycle, preserve one world source of
  truth, and require state→runtime→presentation evidence for every in-scope
  family.

## Local-player and multiplayer game-mode decisions

Owner: `bbb-world` + `bbb-native` + `bbb-net` + `bbb-protocol`

Matrix routes: `world-local-player-decisions`, `interaction-game-mode-decisions`

- **`LocalPlayer` and `MultiPlayerGameMode` decision parity is `active`.** The
  packet catalog and current movement/interaction tests do not by themselves
  prove every prediction, teleport, ability, collision, pose, respawn,
  block-breaking, use/attack/pick, carried-item, creative and spectator branch.
  Audit the 104 local-player and 39 game-mode owners against their actual
  command/state/presentation paths and close only after the applicable live
  interaction cases pass.

## Render-step, HUD and block-entity presentation closure

Owner: `bbb-renderer` + `bbb-native` + `bbb-world` + `bbb-pack`

Matrix routes: `rendering-block-entity-registration`, `rendering-hud-step-surface`, `rendering-game-step-surface`, `rendering-level-frame-graph`, `rendering-level-effect-lifecycle`, `rendering-level-diagnostics-gizmos`, `rendering-level-light-query`

- **The registered block-entity and remaining frame/HUD step surfaces are `active`.**
  Existing focused and pixel tests cover many branches, but do not yet provide
  one mechanically reachable evidence cell for every 27 block-entity
  registration, 70 GUI owner, 46 game-renderer owner and the one open
  LevelRenderer mechanism. Split only by semantic owner and preserve the
  single frame-extract/submission path.
  The generated inventory routes the 77 `LevelRenderer` methods into seven
  total, mutually exclusive mechanisms: backend rebuild 9, section lifecycle
  29, frame graph 14, scene projection 15, effect lifetime 4,
  diagnostics/gizmos 3 and light query 3. The one remaining open
  mechanism remains here until its own single-semantics slice supplies
  consumer evidence.
- **Smithing result equipment preview asset projection is `missing`.**
  `bbb-native::apply_smithing_result_equipment` does not publish the
  generation-matched equipment asset id consumed by the textured equipment
  renderer. Reopen as a behavior slice against Vanilla smithing preview
  ownership; do not restore a legacy material projection to mask this missing
  consumer path.

## Multiplayer screen surface closure

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

Matrix routes: `ui-chat-screen-surface`, `ui-death-screen-surface`,
`ui-pause-screen-surface`, `ui-screen-lifecycle`

- **The 17-owner `PauseScreen` decision surface is `active`.** Audit external-
  multiplayer pause menu, server dialog/link, reporting, disconnect, toast,
  render and close lifecycle against the existing pause-screen owner; classify
  singleplayer-only branches precisely rather than implementing them.
- **Generic multiplayer screen replacement is `active`.** The three
  `Minecraft.setScreen/setOverlay/canInterruptScreen` owners require one
  generation-aware replace/remove/init path, overlay priority and teardown
  interruption evidence across the existing external-session screens.

## Native input, accessibility and window lifecycle closure

Owner: `bbb-native` + `bbb-renderer` + `bbb-control`

Matrix routes: `input-keyboard-routing`, `input-mouse-routing`, `input-key-mapping`, `ui-accessibility-surface`, `platform-window-lifecycle`

- **Keyboard, mouse, key-mapping, narrator/clipboard/IME and window semantic
  parity is `active`.** Unit tests and the Wayland live gate do not close every
  focus/grab/repeat/text/scroll/fullscreen/DPI/resize/icon/accessibility branch.
  Implement all repo-owned semantic gaps and keep the separately recorded
  X11/interactive-seat and macOS/Windows runner limitations external; absence
  of those runners must not hide an internally testable mechanism.

## Persistence, profile and secure-session closure

Owner: `bbb-native` + `bbb-net` + `bbb-item-model`

Matrix routes: `persistence-client-files`, `identity-profile-key-lifecycle`,
`identity-skin-presentation`, `security-secure-chat-reporting`

- **Authenticated profile-key Services acceptance is `deferred`.** This runner has
  no authorized profile UUID/access token pair, and the repository has no
  credential-safe online-mode acceptance fixture, so the real
  `api.minecraftservices.com/player/certificates` response followed by encrypted
  Login, `ChatSessionUpdate`, signed message, reconfiguration and Transfer remains
  externally unverified. Reopen when an account owner provisions an ephemeral
  legal credential pair and controlled Vanilla 26.1 online-mode server runner that
  redacts process/log material and can assert that complete sequence.
- **Client persistence and secure-chat/social/reporting decisions remain `active`.**
  The census has not proven the 158 persistence owners or 21 security owners and
  their atomic recovery, online/offline service boundary and live consumer behavior.
  Split each by one canonical owner; launcher token acquisition remains outside the
  client.

## Runtime performance parity and stable frame tails

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

Matrix route: `performance-gpu-execution-duration`

- API-neutral per-client GPU execution duration is `deferred`: the current
  kernel exposes `dma_fence:dma_fence_signaled` instead of the requested
  `fence:fence_signaled`, and the headless Xwayland path has no proven
  radeonsi/RADV client-context chain across submission, scheduler queue, fence
  signal, and presentation while excluding Weston. Reopen when one common
  capture proves that full association with acceptable overhead for both APIs.
  Do not relabel MangoHud application-frame intervals, `perf stat` process
  means, Weston surface commits, or GPU busy/load as GPU frame duration.

## P4 lifecycle, soak, and supported-platform release gates

Owner: `bbb-native` + `bbb-net` + `bbb-renderer` + `bbb-audio` + `bbb-control`

Matrix routes: `platform-linux-x11-seat-matrix`, `platform-macos-windows-release`

- Linux X11, live DPI-transition/fullscreen, and compositor-originated
  focus/keyboard/mouse/clipboard/IME release validation is `deferred`: the
  maintained headless runner has a real Wayland/Vulkan window but no
  interactive compositor seat/input injector or X11 fixture. Reopen when a
  maintained interactive Linux runner can capture those events and execute the
  same release/live matrix; do not infer them from the declared windowed
  Wayland row or semantic unit tests.
- macOS and Windows release validation is `deferred`: the repository has
  platform-specific URI/config paths and historical macOS build measurements,
  but no maintained host/runner with winit, wgpu, audio and input fixtures for
  either release row. Reopen when such a runner is provisioned and can execute
  the same declared platform matrix; do not infer support from cross-compilation
  or unit-only `cfg` coverage.

## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Matrix route: `interaction-server-authority-exclusions`

- Bottle dragon-breath client prediction is `not-needed`: `AreaEffectCloud.owner`
  is absent from both `ClientboundAddEntityPacket` and synchronized metadata, so
  the remote 26.1 client leaves that branch server-authoritative. Reopen if the
  owner becomes client-visible on the wire.

## Renderer scene parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`

Matrix route: `rendering-official-noop-surfaces`

Living-entity render state:

- Armor-stand pose interpolation is `not-needed`: vanilla 26.1
  `ArmorStandRenderer.extractRenderState` copies all six synced `Rotations`
  getters directly without partial-tick interpolation. Only `yRot` uses
  `Mth.rotLerp`, and hit `wiggle` adds `partialTicks`; bbb already covers both.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

Matrix routes: `debug-integrated-server-mirrors`, `movement-remote-type-exceptions`

- `visualize_chunks_on_server` is `not-needed`: vanilla 26.1
  `ChunkDebugRenderer.emitGizmos` refreshes data only when
  `Minecraft.getSingleplayerServer()` returns an `IntegratedServer`, and its
  `ChunkData` reads `ServerLevel` / `ServerChunkCache` directly on that server's
  executor. bbb is an external native client and owns no integrated-server or
  server chunk-cache surface, so vanilla-equivalent behavior is an empty
  renderer. Reopen if the repo gains an in-process integrated server boundary.
- Remote `visualize_entity_supporting_blocks` `fallDistance > 2.5` powder-snow
  support is `not-needed`: vanilla position/move/teleport packets update a
  server-authoritative remote entity through `moveOrInterpolateTo` and
  `setOnGround`; interpolation uses `setPos`, and only locally authoritative
  `Entity.move` calls `checkFallDamage`, so remote `fallDistance` remains its
  default zero. Local-player high-fall powder snow and the falling-block type
  branch are covered; locally controlled vehicle physics is owned by
  `bbb-world::entities::passengers` and does not make a remote entity's
  server-authoritative `fallDistance` observable.
- Entity-hitbox local-server green boxes and delta arrows are `not-needed`:
  vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` returns an entity
  only through `Minecraft.getSingleplayerServer()` -> `IntegratedServer` ->
  `ServerLevel.getEntity`; an external multiplayer client always receives
  `null` and emits the red `Missing Server Entity` 3D billboard instead. bbb
  owns no integrated server and implements that missing-label branch. Reopen
  with an in-process integrated server boundary.
## Item model range-dispatch and select projection

Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`

Matrix routes: `ui-context-entity-type`, `ui-local-time-icu`, `items-server-owned-effects`

Each item below plugs into the existing value-aware `RangeDispatch` / `Select`
resolver by adding a value provider. No new selection machinery is required.

- Additional `minecraft:context_entity_type` non-GUI wiring is `deferred`: the
  current generated held/head consumers carry their real living owner, while
  `SmithingScreen.updateArmorStandPreview` deliberately calls
  `updateForTopItem(..., level=null, owner=null, seed=0)` for both the head and
  left-hand preview. Reopen when a different current client consumer passes a
  real living owner to `ItemModelResolver`; non-living `ItemOwner` values remain
  null after Vanilla's `asLivingEntity` narrowing.
- Full `minecraft:local_time` ICU parity is `deferred`: the remaining localized
  symbols, locale-specific week data, IANA long `z`, generic `v`, and
  one-/four-letter `V` widths have no portable ICU owner compatible with the
  workspace Rust 1.80 target. Reopen when an approved repo-native provider can
  cover Vanilla's arbitrary `SimpleDateFormat` patterns, locale calendar data,
  and timezone names without raising the MSRV implicitly.
- `not-needed`: synchronized conditional value/entity/location enchantment
  effects, damage immunity, equipment-drop prevention, and crossbow charging
  sounds remain server-owned in Vanilla 26.1. Conditional execution enters
  through `ServerLevel`; crossbow loading sounds run only under
  `!level.isClientSide()`. Reopen if
  `test_enchantment_client_effect_partition.py` finds a client-side consumer.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

Matrix route: `interaction-server-authority-exclusions`

- Client-owned animal ambient/step/eat and cat/wolf state-trigger loops are
  `not-needed`: vanilla `Entity.playSound` delegates through
  `Level.playSound(null, ...)`, `ClientLevel.playSeededSound` only submits when
  the excluded entity is its local player, and `ServerLevel` supplies remote
  listeners with Sound/SoundEntity packets. Reopen only if a target animal path
  is shown to call `playLocalSound` or stops being server-packet-owned.

Audio unit tests must remain independent of an audio device.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

Matrix route: `registries-unsynchronized-fixed-identities`

Lifecycle invariants (not todos): vanilla, startup and server packs share one
validated low-to-high selected stack and immutable generation; CPU consumers
prepare from that stack, renderer/audio owners commit atomically, and control
publishes source/priority/failure plus bounded dynamic-texture observations.
Skin/cape/elytra requests share one process-lifetime CPU owner; resource/session
generations only own GPU delivery and cannot force a second HTTP/cache resolver.

- Synchronized custom block/entity registry identities are `not-needed`:
  Vanilla 26.1 `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` contains neither
  `BLOCK` nor `ENTITY_TYPE`; their protocol identities remain fixed, and a
  resource pack cannot add either registry entry. Reopen if the target protocol
  begins synchronizing one of those registries.

## Container menu presentation parity

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

Matrix route: `ui-container-server-authority-exclusions`

- A matching-id authority check for ContainerClose is `not-needed`: vanilla
  26.1 carries an id but closes the current menu unconditionally.
- A synthetic menu fallback for mismatched ContainerSetData is `not-needed`:
  `ClientPacketListener.handleContainerSetData` accepts only the current
  `containerMenu` id.
- A synthetic menu fallback for mismatched nonzero ContainerSetSlot is
  `not-needed`: `ClientPacketListener.handleContainerSetSlot` rejects it.
- A combat-mode runtime transition for PlayerCombatEnter is `not-needed`: its
  26.1 client handler is empty.
- A combat-mode runtime transition for PlayerCombatEnd is `not-needed`: its
  decoded duration is diagnostic and its 26.1 client handler is empty.
## HUD and debug overlay

Owner: `bbb-native` + `bbb-renderer` + `bbb-audio`

Matrix route: `debug-integrated-server-mirrors`

- **Client-only debug entries with no HUD line** are `not-needed`:
  LocalDifficulty, EntitySpawnCounts, and ChunkGenerationStats read an
  integrated local-server mirror (`ServerLevel`,
  `NaturalSpawner.SpawnState`, `ChunkGenerator` / `RandomState` /
  `BiomeSource`) that an external native client does not own.

## Entity models and animation

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

Matrix route: `rendering-official-noop-surfaces`

### Model and animation details

- **Frog tongue prey-targeting is `not-needed`.**
  `DATA_TONGUE_TARGET_ID` is consumed by frog/AI behavior only. Vanilla
  `FrogRenderer.extractRenderState` does not project it, `FrogRenderState` has
  no target field, and `FrogModel.setupAnim` drives the tongue solely from the
  copied tongue `AnimationState`.
- **Rabbit `IDLE_HEAD_TILT` keyframe** is `not-needed`: vanilla starts it from
  the entity's client-local `random.nextInt(40) + 180` timeout, so a remote
  client's particular phase is not reconstructable and bbb must not fabricate
  a seed.
- **Minecart spawner animated block-entity display content is `not-needed`.**
  Vanilla renders a `MinecartSpawner` through the ordinary minecart display
  block only. No block-entity renderer calls
  `BaseSpawner.getOrCreateDisplayEntity` for that embedded spawner, so its
  `displayEntity` remains null and `clientTick` advances neither spin nor
  display particles.

## Particles and level events

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

Matrix route: `rendering-particle-provider-registration`

Routing invariant: terrain/item atlas rendering, block-entity client-effect
presentation, and audio-device/runtime parity stay with their owning domains;
the `LevelEventHandler` switch itself is not a second owner.

---

# Not needed

Vanilla 26.1 has no such behavior, or bbb has no surface for it. Each carries the
vanilla evidence. Deleting one of these re-opens work already decided against.

## Packet canonical state

Matrix routes: `scope-telemetry-upload`, `protocol-framing-no-world-state`

- **Mojang client telemetry callbacks.** `bbb` has no external telemetry sender.
  Vanilla `WorldSessionTelemetryManager.onServerBrandReceived` only stores the
  brand on `WorldLoadEvent` and attempts a Mojang telemetry send; server-brand
  protocol state and local diagnostics remain independently owned.

- **Bundle delimiter world state.** `ClientboundBundleDelimiterPacket` is
  consumed by `PacketBundlePacker` / `BundlerInfo` as connection framing before
  listener dispatch; it has no payload, listener handler, or canonical world
  mutation.

## Source inventory support owners

Matrix route: `scope-support-accessors`

- **Direct `Minecraft` accessors are not independent mechanisms.** The shared
  Tree-sitter inventory classifies only single-return field, parameter or
  constant bodies as `support-non-decision`; every branch, allocation, call or
  mutation must have an explicit mechanism route. Reopen when the CST body of
  any of the 64 classified owners gains a decision or side effect.

## Target-version item-model surface

Matrix route: `items-versioned-property-surface`

- **Additional ambient-context numeric `range_dispatch` properties.** Vanilla
  26.1 `RangeSelectItemModelProperties.bootstrap` registers exactly
  custom-model-data, bundle fullness, damage, cooldown, time, compass,
  crossbow pull, use cycle, use duration and count, all of which are present in
  the native resolver. Reopen only when the target version changes or that
  registration list differs.

## Resource and screen surface

Matrix routes: `resources-indexed-built-in-packs`, `resources-indexed-realms`, `resources-world-list-favicons`, `scope-title-screen`, `scope-realms`

- **Title splash reload listener.** Vanilla `SplashManager.prepare` reads
  `minecraft:texts/splashes.txt`, but its only consumer is
  `TitleScreen.init` calling `Minecraft.getSplashManager().getSplash()`.
  The CLI-direct client has no title screen. Reopen when a title screen enters
  the product; merely exposing the resource through the default pack does not
  create a consumer.
- **Indexed built-in resource-pack ZIPs.** `ClientPackSource` exposes
  `programmer_art` and `high_contrast` through the pack repository/options
  selection surface. bbb accepts explicit startup/server packs but has no
  built-in pack-id selection or in-game options UI, so the two ZIPs have no
  current consumer. Reopen when a built-in pack-id UI or CLI surface is added.
- **Indexed Realms language and carousel assets.** Vanilla's shared language
  manager can see `realms:lang/en_us`, while `AddRealmPopupScreen` enumerates
  `realms:textures/gui/images`; Realms service and screens are outside the
  external multiplayer client boundary. The 16 objects remain
  provider-reachable. Reopen when Realms service/screen support enters scope.

- **Broader non-profile dynamic textures.** The remaining Vanilla 26.1
  external-image producers are server/world-list favicons and pack-selection
  presentation. bbb exposes startup/server-pack configuration through the CLI
  and has neither screen; its current client surfaces already own map textures,
  profile skin/cape/elytra and synchronized pack-backed presentation. Reopen
  only if one of those screens enters the declared client scope or a new
  current-surface producer is identified.

## Non-player movement authority

Matrix route: `movement-remote-type-exceptions`

- **`POWDER_SNOW_WALKABLE_MOBS` collision exceptions.** Vanilla 26.1's tag
  contains only rabbit, endermite, silverfish and fox. None is a locally
  client-authoritative ridden type in the current client, so their collision is
  server-owned. Reopen specifically if one of those four types gains local
  movement authority.
- **Freeze-immune entity-type exceptions.** Vanilla
  `LivingEntity.tick` applies freeze damage only in `ServerLevel`, and
  `FREEZE_IMMUNE_ENTITY_TYPES` contains only stray, polar bear, snow golem and
  wither. Their remote frozen presentation remains server synchronized. Reopen
  specifically if one of those four types gains local freezing authority.
- **Sweet-berry fox/bee exceptions.** Vanilla
  `SweetBerryBushBlock.entityInside` skips fox and bee slowing, while its
  age-gated damage executes only in `ServerLevel`. Neither fox nor bee is a
  locally client-authoritative ridden type in the current client. Reopen if
  either exact type gains local movement authority.

## Entity models and layers

Matrix routes: `rendering-entity-renderer-registration`, `rendering-entity-layer-registration`, `rendering-official-noop-surfaces`

### Entity coverage matrix vanilla noop renderers

- Area-effect-cloud, marker, and interaction have no model/texture/layer
  presentation: vanilla registers each with `NoopRenderer`. Their other
  observable cells remain independently classified.

- **Chicken head look.** `ChickenModel` never animates the head.
- **Freezing white overlay.** The creeper is the only vanilla
  `getWhiteOverlayProgress` producer; the base renderer returns `0`.
  `isFullyFrozen` drives the `setupRotations` body shake, not an overlay.
- **Zoglin conversion shake.** `ZoglinRenderer` has no conversion override.
- **Snow-golem walk-driven swing.** Its `setupAnim` is the head-yaw twist/orbit.
- **Piglin-brute `ADMIRING_ITEM` pose.** The brute has no admire branch.
- **Fox baby model scale.** `Fox.BABY_SCALE = 0.6` scales only the bounding box,
  so the baby uses the standard `0.5` `LivingEntity.getAgeScale`.
- **Nautilus baby `0.7F` model scale.** `NautilusRenderer` passes `0.7F` as the
  `shadow` argument to `AgeableMobRenderer`, whose `submit` only selects the
  adult or baby model. Vanilla has no additional `0.7` baby-model transform.
- **Axolotl baby as a scaled adult.** The `0.5F` constructor argument is the
  shadow radius, not a scale; the baby has its own 32x32 geometry.
- **`SniffletModel.setupAnim` static baby transform** (`BABY_TRANSFORM` /
  `SNIFFER_BABY_FALL`). Vanilla `SnifferRenderer` does not consume it.
- **Baby saddle models** for pig, horse, donkey, mule, camel, strider, and living
  nautilus. Vanilla supplies no baby saddle model (`null` for strider; no baby
  model on `SimpleEquipmentLayer` for nautilus).
- **Baby horse body-armor layer.** Vanilla supplies no baby armor model.
- **Skeleton-horse body armor.** Vanilla
  `EntityTypeTags.CAN_WEAR_HORSE_ARMOR` includes only horse and zombie_horse.
- **Baby wandering-trader model/texture.** `WanderingTraderRenderer` is a plain
  `MobRenderer` always using `ModelLayers.WANDERING_TRADER` and
  `textures/entity/wandering_trader/wandering_trader.png`, so the inherited
  `AgeableMob.DATA_BABY_ID` selects nothing.
- **Pig boost/ridden-speed renderer pose.** `PigRenderState` carries saddle and
  variant only; boost lives in `ItemBasedSteering` / `Pig.tickRidden` movement
  control.
- **Area-effect-cloud, marker, and interaction models.** `EntityRenderers`
  registers all three to `NoopRenderer`: the cloud draws as particles, the marker
  is a server-side data entity, the interaction is an invisible click hitbox.
- **Invisible-gating `WolfArmorLayer`.** Vanilla does not gate that layer on
  `state.isInvisible`, so bbb keeps its armor/crack submissions in
  hidden/self-visible/glowing invisible states.
- **`CapeLayer` for invisible players.** Vanilla explicitly gates it on
  `!state.isInvisible`.
- **Separate glow-squid emissive overlay/texture.** Vanilla 26.1
  `GlowSquidRenderer` only selects the adult/baby texture and overrides
  `getBlockLightLevel`; it registers no independent emissive layer.
- **Exact guardian client RNG seed.** Vanilla seeds the per-spawn entity RNG
  outside the protocol. bbb uses one deterministic UUID/type-derived Java-LCG
  cursor for the constructor tail phase and subsequent spike/bubble/flop draws;
  draw order and ranges are exact, but a particular official-client sample is
  not reconstructable from wire state.
- **Guardian powder-snow stand-on-face exception.** The normal loaded collision
  UP-face path is exact. Vanilla can additionally expose powder snow as a full
  collision shape from entity-local fall distance or leather-boots context;
  remote Guardian fall distance is not synchronized, so bbb conservatively
  fails that rare flop-sound arming probe closed.
- **Exact enderman creepy root-jitter RNG seed.** Vanilla keeps that RNG inside
  the renderer and never exposes the seed over the protocol; bbb uses a
  deterministic Java-LCG-shaped seed from entity id + age while preserving the
  vanilla axis/scale behavior.
- **Exact horse `tailCounter` client seed.** Not protocol-visible; bbb uses a
  deterministic local Java LCG.

## Rendering and screenshots

Matrix route: `rendering-nondeterministic-golden-exclusions`

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
- **Lightning sky flash tinting water fog.** Vanilla does not tint it.

## Block entities

Matrix routes: `rendering-block-entity-registration`, `rendering-official-noop-surfaces`

- **Sign `is_waxed` render effect.** Vanilla has none; it gates editing.
- **Banner item / shield pattern path** (`Sheets.SHIELD_PATTERN_BASE`,
  `BannerRenderer.submitSpecial`). Item-model scope, not the block-entity
  renderer.
- **Shulker box open/close sound and interaction blocking.** Vanilla
  `ShulkerBoxBlock.useWithoutItem` / `canOpen` own collision admission, while
  `ShulkerBoxBlockEntity.startOpen` / `stopOpen` own the sounds;
  `ShulkerBoxRenderer` only consumes render state.
- **Bell resonation particle/glow chain** (`resonationTicks`, raider search).
  Gameplay-side, belongs with the raid features.

## Particles

Matrix routes: `rendering-particle-provider-registration`, `rendering-official-noop-surfaces`

- **Spawn-filtering for `minecraft:block_marker`.** Vanilla `BlockMarker.Provider`
  does not reject air, `moving_piston`, or `shouldSpawnTerrainParticles=false`
  states the way `TerrainParticle.createTerrainParticle` does.
- **`moving_piston` rejection in the destroy-block effect** (events `2001` /
  `3008`). Vanilla `addDestroyBlockEffect` does not reject it, unlike
  `TerrainParticle.Provider`.
- **xAux handling in `GustParticle.Provider`.** Vanilla ignores that xAux; the
  `GustSeedParticle.Provider` child still feeds child xAux `age / lifetime`.

## Input, movement, and interaction

Matrix route: `interaction-server-authority-exclusions`

- **Server-owned FishingHook catching/removal outcomes.** The bbb client tick
  owns Vanilla flight, Bobbing, hooked-target following, collision/deflection,
  metadata biting impulse, and Breeze deflect audio. `catchingFish`, loot,
  open-water/server timers, rod durability/pull results, and canonical removal
  remain incoming server/packet outcomes; an invalid-owner client instance is
  frozen until that authoritative removal instead of creating a second local
  removal path.
- **Local component fallback for Efficiency and Aqua Affinity destroy speed.**
  Vanilla routes both through synced attributes: Efficiency contributes to
  `mining_efficiency` (attribute id `20`), Aqua Affinity to
  `submerged_mining_speed` (attribute id `29`). `UpdateAttributes` stays the
  authoritative path.
- **Local terrain-collision handling of portal travel** (`nether_portal`,
  `end_portal`, `end_gateway`). All three are vanilla `.noCollision()`; portal
  travel, smoke/portal particles, and gateway cooldown are server/runtime or
  entity-inside behavior.
- **Dynamic collision for `moving_piston`** when no canonical moving-piston block
  entity exists. Treated as empty, matching vanilla's empty fallback.
- **Local prediction of Mount component-patched or entity-specific `equippable`
  predicates, and `sweet_berry_bush` damage.** Both are server-authoritative.
- **Touchscreen split-stack and snapback animation.** Vanilla gates both entirely
  behind `Options.touchscreen` (`AbstractContainerScreen.java:336, 342, 411, 489`;
  snapback interpolation `:146-158`) and bbb has no touch input mode or plan.

## HUD and configuration

Matrix routes: `diagnostics-client-only-inspection-exclusions`, `scope-options-ui`,
`resources-runtime-mipmap-update`, `scope-singleplayer-integrated`,
`debug-integrated-server-mirrors`

- **Shift-local F3+I server-private entity save residuals.** Not needed on an
  external client: ordinary F3+I queries the authoritative server tag, while
  Shift-local capture can project only synchronized metadata, spawn/equipment
  components, client-owned runtime, and protocol-backed references. Unretained
  Living impulse context, mutable owner/target/RNG/timer fields, non-empty
  entity inventories or loot tables, server-loaded spawner-minecart
  `SpawnData` / `SpawnPotentials` and non-default counters, and command-block
  minecart execution results have no clientbound owner. Reopen a field only
  when a protocol or client-owned mutation path makes its exact value
  observable.
- **Shift-local F3+I fixed-registry holder reconstruction for direct
  jukebox-song and painting values.** Not needed: vanilla
  `JukeboxSong.CODEC` and `PaintingVariant.CODEC` are named-only
  `RegistryFixedCodec`s, while their network holders may be direct and have no
  persistent resource key. Unknown direct holders omit the containing value
  rather than guessing an identity.
- **F3+I non-default `PrimedTnt.explosionPower`, item-frame `dropChance` /
  `fixed`, and `SpectralArrow.duration`.** Not needed on an external client:
  vanilla 26.1 changes all four only in `readAdditionalSaveData`; none is in the
  add-entity packet or synched entity data, and client tick/interaction does not
  mutate them. A newly recreated remote `PrimedTnt` therefore always keeps
  `explosionPower = 4.0` (whose save field is conditionally omitted), item/glow
  frames keep `dropChance = 1.0` and `fixed = false` (both always written), and
  spectral arrows keep `duration = 200` (always written). Reopen only if a
  future protocol synchronizes one of these private fields.
- **F3+I local fishing-bobber save NBT.** Not needed: vanilla
  `EntityType.FISHING_BOBBER` is `.noSave().noSummon()` (`EntityType.java`),
  `FishingHook.addAdditionalSaveData` is an empty override that does not call
  `Projectile.addAdditionalSaveData`, and `Projectile.isPickable` only returns
  true for `EntityTypeTags.REDIRECTABLE_PROJECTILE`; the 26.1 bootstrap tag
  contains fireball/wind-charge variants, not `FISHING_BOBBER`.
- **F3+F4 `GameModeSwitcherScreen` mouse-release path.** It only uses
  `mouseReleased` for `keyDebugModifier.matchesMouse(event)`, and
  `KeyMapping.matchesMouse` is true only for a `MOUSE`-type mapping. bbb's debug
  modifier is the hard-coded F3 keyboard path.
- **Container-loot loot-table/seed NBT payload.** Vanilla tooltip output
  (`SeededContainerLoot.addToTooltip` -> `item.container.loot_table.unknown`)
  depends only on presence.
- **In-game options UI and vanilla options-file persistence.** Configuration
  stays at command-line startup (for example `--advanced-item-tooltips`); bbb
  does not add an in-game configuration UI or call vanilla `Options.save()`.
  This also owns `Minecraft.updateMaxMipLevel`: its only Vanilla 26.1 caller is
  `VideoSettingsScreen`, while BBB's terrain generation already uses the
  Vanilla default mip level 4. Reopen the runtime update mechanism when a CLI
  or supported in-game surface can change mipmap levels after startup; that
  slice must rebuild the complete resource/GPU generation atomically rather
  than mutate the atlas behind its owner.
- **Vanilla OpenGL GPU warnlist reload listener.**
  `GpuWarnlistManager.Preparations.apply` returns warnings only when
  `GpuDevice.getBackendName()` is exactly `OpenGL`; its behavioral consumer is
  the improved-transparency branch in `Options`/`VideoSettingsScreen`, with an
  optional copy in `Minecraft.fillSystemReport`. BBB ships the wgpu primary
  Vulkan/Metal/DX12 backends and has no in-game options UI, so the Vanilla
  predicate is always empty. Reopen when an OpenGL backend or the corresponding
  improved-transparency warning choice enters the supported product; then the
  diagnostics copy must be wired in the same slice.
- **Custom/datapack attribute tooltip definitions.** Vanilla 26.1 registers
  `minecraft:attribute` through `BuiltInRegistries.registerSimple`; `Attribute`
  has no direct element codec and the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  The fixed 35-entry vanilla catalog is generated; this item covers non-vanilla
  definitions rather than built-in id/resource metadata.
  Attribute holders therefore use the fixed client built-in registry, so no
  server/datapack description or sentiment payload exists to project.
- **Custom/datapack mob-effect definitions.** Vanilla 26.1 registers
  `minecraft:mob_effect` through `BuiltInRegistries.registerSimple`, and the
  registry is absent from both `RegistryDataLoader.WORLDGEN_REGISTRIES` and
  `SYNCHRONIZED_REGISTRIES`. The fixed 40-entry vanilla id/resource/category
  catalog is generated; effect holders therefore resolve against the built-in
  client registry rather than server-defined effect metadata.
- **Custom/datapack potion registry definitions.** Vanilla 26.1 registers
  `minecraft:potion` through `BuiltInRegistries.registerSimple` with
  `Potions::bootstrap`; the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  Potion holder ids and their base effect lists therefore come from the fixed
  client built-in registry rather than a server-remappable registry payload.
- **Custom/datapack villager-type and villager-profession definitions.**
  Vanilla 26.1 registers both defaulted registries in `BuiltInRegistries`
  through `VillagerType::bootstrap` / `VillagerProfession::bootstrap`; neither
  appears in `RegistryDataLoader.WORLDGEN_REGISTRIES` or
  `SYNCHRONIZED_REGISTRIES`. Their `VillagerData.STREAM_CODEC` holders resolve
  through the connection's static registry layer, whose ids/order come from
  the local built-in catalogs rather than configuration `RegistryData`. The
  fixed 7-entry type and 15-entry profession identity/ordinal catalogs are
  generated directly from those vanilla bootstrap registrations.
- **Custom/datapack decorated-pot pattern definitions.** Vanilla 26.1 registers
  `minecraft:decorated_pot_pattern` through `BuiltInRegistries.registerSimple`
  with `DecoratedPotPatterns::bootstrap`, and the registry is absent from both
  `RegistryDataLoader.WORLDGEN_REGISTRIES` and `SYNCHRONIZED_REGISTRIES`.
  Item-to-pattern identity and pattern asset ids therefore come from the fixed
  client built-in registry rather than a server-remappable registry payload.
- **Custom/datapack menu-type definitions.** Vanilla 26.1 registers
  `minecraft:menu` through `BuiltInRegistries.registerSimple`, and the registry
  is absent from both `RegistryDataLoader.WORLDGEN_REGISTRIES` and
  `SYNCHRONIZED_REGISTRIES`. The fixed 25-entry vanilla menu id/resource catalog
  is generated; OpenScreen menu ids therefore resolve against the built-in
  client registry rather than server-defined menu metadata.
- **Custom difficulty enum values.** Vanilla 26.1 `Difficulty` is a closed Java
  enum with four explicit ids/names and `ByIdMap.OutOfBoundsStrategy.WRAP`;
  protocol/world consume the generated catalog and there is no server/datapack
  definition surface to load.
- **Custom equipment-slot or slot-group enum values.** Vanilla 26.1 defines
  both as closed Java enums, not registries or datapack codecs. Their distinct
  declaration ordinals / explicit ids and serialized names are generated from
  `EquipmentSlot.java` and `EquipmentSlotGroup.java`; the packet-facing typed
  equipment-slot enum and wire ordinal lookup share that generated owner. There
  is no server-owned extension surface to load.
- **Custom entity-pose enum values.** Vanilla 26.1 `Pose` is a closed Java enum
  whose id-mapped stream codec uses its 18 explicit ids. The fixed id/name
  catalog is generated from `Pose.java`; it has no registry or datapack
  extension surface.
- **Custom entity-data serializers.** Vanilla 26.1 assigns the built-in ids in
  the static `EntityDataSerializers` registration block. Its public code-level
  registration helper is not a server- or datapack-synchronized registry. The
  fixed 43-entry vanilla id/order catalog is generated from that block;
  serializer wire codecs remain explicit protocol semantics rather than
  dynamically supplied definitions.
- **Custom client game-event packet types.** Vanilla 26.1 declares the 14
  `ClientboundGameEventPacket.Type` instances and their byte ids directly in
  the packet class; there is no server/datapack registration surface. The
  generated fixed catalog intentionally does not treat vibration game events
  or entity-event ids as members of this packet type map.
- **Custom sound-source enum values.** Vanilla 26.1 `SoundSource` is a closed
  Java enum whose wire value is its declaration ordinal. The generated fixed
  11-entry ordinal/name catalog has no server or datapack extension surface.
- **Custom item registry definitions.** Vanilla 26.1 item protocol ids come
  from the fixed client built-in `Items` registration order, not a synchronized
  server/datapack registry. The generated 1506-entry catalog covers the vanilla
  mapping; modded clients that change item registration order remain outside
  the native vanilla-client target.
- **Custom block or block-state registry definitions.** Vanilla 26.1 block and
  global block-state ids come from the fixed client built-in registration; the
  checked-in 29,873-state report is the native lookup authority. Modded clients
  that alter block registration or state definitions remain outside the vanilla
  client target.
- **Accepting malformed or unsupported custom resource-pack declarations.**
  Vanilla resource loaders parse with `StrictJsonParser` plus typed codecs and
  reject invalid declarations; bbb likewise fails the selected generation
  instead of inventing fallback schema.

---

# Appendix: per-provider particle coverage matrix

This is the current provider fact matrix. Open behavior is represented by a
`todo` cell; `covered` names a current native owner, and `not-needed` carries
the Vanilla reason that no owner is required. When a gap is found, add its row
or flip the affected cell to `todo` before selecting a slice. Delete no
`not-needed` evidence and do not append completion records or commit hashes.

Every row is judged against the Vanilla 26.1 class under
`~/Work/mc-code/sources/26.1/net/minecraft/client/particle/` (line references
in notes), cross-checked against the bbb runtime
(`crates/bbb-renderer/src/particles/descriptors.rs` — `collision_size()`,
`moves_without_collision()`, `drip_fluid()`, `required_fluid()`,
`air_downward_acceleration()`, `tick_motion()` —
`crates/bbb-renderer/src/particles/instance.rs`).

Column definitions:

- collision: world collision/move path, collision AABB bounds
  (`setSize` / `Particle.scale`), `onGround` behavior, `stoppedByCollision`,
  `speedUpWhenYMotionIsBlocked`. `not-needed` when the particle has
  `hasPhysics=false` (clipping and `onGround` are dead in vanilla — `move`
  only translates), a collision-free `move` override, or a tick that never
  calls `move`.
- player-coupled: local player / nearest player / camera position, velocity,
  or view coupling inside the particle or provider tick.
- sounds: sounds triggered by the particle's own tick (`playLocalSound`).
  Spawn-site sounds emitted by packet / level-event / entity-event handlers
  stay with those handler domains, not this matrix.
- removal-gates: removal (or motion gates) driven by block / fluid / entity
  state queries during tick. Collision-driven removal (e.g. leaves
  `onGround`) belongs to the collision column.

| Provider | collision | player-coupled | sounds | removal-gates | Notes |
| --- | --- | --- | --- | --- | --- |
| AshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (AshParticle.java:19, BaseAshSmoke last ctor arg) |
| AttackSweepParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (AttackSweepParticle.java:28-37) |
| BlockMarker.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` + zero velocity (BlockMarker.java:14-17); vanilla has no spawn filter here — bbb matches |
| BreakingItemParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default 0.2 bounds correct; generic collision callback; gravity 1.0 |
| BreakingItemParticle.SlimeProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.CobwebProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.SnowballProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BubbleColumnUpParticle.Provider | covered | not-needed | not-needed | covered | `setSize(0.02F)` (BubbleColumnUpParticle.java:24) maps to `collision_size()`; non-water removal :35-37 maps to `required_fluid()` |
| BubbleParticle.Provider | covered | not-needed | not-needed | covered | `setSize(0.02F)` (BubbleParticle.java:22) maps to `collision_size()`; +0.002 rise / 0.85 damping and non-water removal :43-45 are covered |
| BubblePopParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; direct-gravity-no-friction tick covered |
| CampfireSmokeParticle.CosyProvider | covered | not-needed | not-needed | not-needed | 0.25 bounds special-cased (descriptors.rs:2048-2050); `alpha<=0` removal + random drift covered |
| CampfireSmokeParticle.SignalProvider | covered | not-needed | not-needed | not-needed | as CosyProvider |
| CritParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (CritParticle.java:35) |
| CritParticle.DamageIndicatorProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| CritParticle.MagicProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| DragonBreathParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics=false` (DragonBreathParticle.java:34) — the `onGround`/`hasHitGround` branch (:48-55) is dead in vanilla because `move` never sets `onGround` without physics; bbb models the tick formula including the branch |
| DripParticle.LavaHangProvider | covered | not-needed | not-needed | covered | `setSize(0.01F)` (DripParticle.java:25); lava-surface removal :58-64, cooling RGB, damping and fall child are covered |
| DripParticle.LavaFallProvider | covered | not-needed | not-needed | covered | 0.01 bounds; `onGround` land child and lava-surface removal are covered |
| DripParticle.LavaLandProvider | covered | not-needed | not-needed | covered | 0.01 bounds; lava-surface removal is covered |
| DripParticle.WaterHangProvider | covered | not-needed | not-needed | covered | 0.01 bounds; water-surface removal is covered |
| DripParticle.WaterFallProvider | covered | not-needed | not-needed | covered | 0.01 bounds; `onGround` splash child and water-surface removal are covered |
| DripParticle.HoneyHangProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY` means no fluid gate (DripParticle.java:366-371) |
| DripParticle.HoneyFallProvider | covered | not-needed | covered | not-needed | 0.01 bounds; `BEEHIVE_DRIP` on land (DripParticle.java:313-320); `Fluids.EMPTY` |
| DripParticle.HoneyLandProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY` |
| DripParticle.NectarFallProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `onGround` removal; `Fluids.EMPTY` |
| DripParticle.SporeBlossomFallProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY` |
| DripParticle.ObsidianTearHangProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY`; glow light is covered |
| DripParticle.ObsidianTearFallProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY` |
| DripParticle.ObsidianTearLandProvider | covered | not-needed | not-needed | not-needed | 0.01 bounds; `Fluids.EMPTY` |
| DripParticle.DripstoneWaterHangProvider | covered | not-needed | not-needed | covered | 0.01 bounds; water-surface removal is covered |
| DripParticle.DripstoneWaterFallProvider | covered | not-needed | covered | covered | 0.01 bounds; `POINTED_DRIPSTONE_DRIP_WATER` on land (DripParticle.java:156-162) |
| DripParticle.DripstoneLavaHangProvider | covered | not-needed | not-needed | covered | 0.01 bounds; cooling RGB and lava-surface removal are covered |
| DripParticle.DripstoneLavaFallProvider | covered | not-needed | covered | covered | 0.01 bounds; `POINTED_DRIPSTONE_DRIP_LAVA` on land is covered |
| DustParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; `hasPhysics=true` + `speedUpWhenYMotionIsBlocked` via generic path |
| DustColorTransitionParticle.Provider | covered | not-needed | not-needed | not-needed | as DustParticle.Provider; transition lerp covered |
| DustPlumeParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (DustPlumeParticle.java:22, BaseAshSmoke last arg); gravity/friction decay covered |
| ElderGuardianParticle.Provider | not-needed | not-needed | not-needed | not-needed | zero velocity, gravity 0 — base tick moves nothing; special-group render covered |
| EndRodParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` override (EndRodParticle.java:22-25); in `moves_without_collision` |
| ExplodeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; physics, gravity -0.1, friction 0.9 |
| FallingDustParticle.Provider | covered | not-needed | not-needed | not-needed | custom tick move + `onGround` roll reset covered; spawn `INVISIBLE` rejection + tint covered; no removal gate in vanilla |
| FallingLeavesParticle.CherryProvider | covered | not-needed | not-needed | not-needed | per-spawn `setSize(size, size)` uses sampled `base_quad_size` (0.05/0.075); collision removal and first-tick grace are covered |
| FallingLeavesParticle.PaleOakProvider | covered | not-needed | not-needed | not-needed | sampled 0.1/0.15 bounds; otherwise as CherryProvider |
| FallingLeavesParticle.TintedLeavesProvider | covered | not-needed | not-needed | not-needed | sampled 0.1/0.15 bounds; ARGB tint is covered |
| FireflyParticle.FireflyProvider | covered | not-needed | not-needed | covered | provider `scale(1.5F)` gives 0.3 bounds (FireflyParticle.java:94, Particle.java:77-80); generic move and non-air removal :50-51 are covered |
| FireworkParticles.FlashProvider | not-needed | not-needed | not-needed | not-needed | zero velocity, lifetime 4; overlay alpha/size covered |
| FireworkParticles.SparkProvider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; SimpleAnimated physics via generic path; trail/twinkle children + fade covered; sounds live on the Starter row |
| FlameParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` override (FlameParticle.java:29-32); in `moves_without_collision` |
| FlameParticle.SmallFlameProvider | not-needed | not-needed | not-needed | not-needed | as FlameParticle.Provider |
| FlyStraightTowardsParticle.OminousSpawnProvider | not-needed | not-needed | not-needed | not-needed | empty `move` override (FlyStraightTowardsParticle.java:56-57) + `hasPhysics=false`; curve covered |
| FlyTowardsPositionParticle.EnchantProvider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (FlyTowardsPositionParticle.java:74-77) + `hasPhysics=false` |
| FlyTowardsPositionParticle.NautilusProvider | not-needed | not-needed | not-needed | not-needed | as EnchantProvider |
| FlyTowardsPositionParticle.VaultConnectionProvider | not-needed | not-needed | not-needed | not-needed | as EnchantProvider; `LifetimeAlpha` covered |
| GlowParticle.GlowSquidProvider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (GlowParticle.java:19) |
| GlowParticle.WaxOnProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.WaxOffProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.ElectricSparkProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GlowParticle.ScrapeProvider | not-needed | not-needed | not-needed | not-needed | as GlowSquidProvider |
| GustParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (GustParticle.java:30-36); `setSize(1.0F)` has no behavioral effect |
| GustParticle.SmallProvider | not-needed | not-needed | not-needed | not-needed | as GustParticle.Provider |
| GustSeedParticle.Provider | not-needed | not-needed | not-needed | not-needed | NoRender emitter, no move; child gust emission covered |
| HeartParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (HeartParticle.java:20); speed-up flag dead without collision |
| HeartParticle.AngryVillagerProvider | not-needed | not-needed | not-needed | not-needed | as HeartParticle.Provider; `y + 0.5` offset covered |
| HugeExplosionParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` |
| HugeExplosionSeedParticle.Provider | not-needed | not-needed | not-needed | not-needed | NoRender emitter; child explosion emission covered |
| LargeSmokeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; smoke-family collision + blocked-Y speed-up covered |
| LavaParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds physics; child smoke emission covered |
| NoteParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics` stays true in vanilla; default bounds; generic collision + speed-up |
| PlayerCloudParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (PlayerCloudParticle.java:32); native projects non-spectator local and remote players and the renderer selects the strict nearest within 2.0 |
| PlayerCloudParticle.SneezeProvider | not-needed | covered | not-needed | not-needed | as PlayerCloudParticle.Provider |
| PortalParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (PortalParticle.java:48-51); curve covered |
| ReversePortalParticle.ReversePortalProvider | not-needed | not-needed | not-needed | not-needed | inherits portal collision-free `move` |
| SculkChargeParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (SculkChargeParticle.java:18); roll covered |
| SculkChargePopParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (SculkChargePopParticle.java:18) |
| ShriekParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics` stays true, default bounds, `yd=0.1`; delay decoded into `initial_delay_ticks`; dual-quad extract covered |
| SimpleVerticalParticle.PauseMobGrowthProvider | covered | not-needed | not-needed | not-needed | default bounds physics; -0.03 y offset covered |
| SimpleVerticalParticle.ResetMobGrowthProvider | covered | not-needed | not-needed | not-needed | default bounds physics; +0.03 y offset covered |
| SmokeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; blocked-Y speed-up via collision callback covered |
| SnowflakeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; post-tick damping covered |
| SonicBoomParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (inherits HugeExplosionParticle) |
| SoulParticle.Provider | covered | not-needed | not-needed | not-needed | `scale(1.5F)` gives 0.3 bounds (SoulParticle.java:17, Particle.java:77-80); rising motion is covered |
| SoulParticle.EmissiveProvider | covered | not-needed | not-needed | not-needed | as SoulParticle.Provider, including 0.3 bounds |
| SpellParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (SpellParticle.java:33); scoping alpha (:35-37, :49-53, :62-69) covered — vanilla itself only checks the local player |
| SpellParticle.InstantProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; RGB + power covered |
| SpellParticle.MobEffectProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; ARGB covered |
| SpellParticle.WitchProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; magenta brightness covered |
| SpitParticle.Provider | covered | not-needed | not-needed | not-needed | ExplodeParticle physics, gravity 0.5, default bounds |
| SplashParticle.Provider | covered | not-needed | not-needed | covered | inherits WaterDrop 0.01 bounds; `onGround` 50% removal and collision-shape/fluid-surface gate :39-55 are covered |
| SquidInkParticle.Provider | not-needed | not-needed | not-needed | covered | `hasPhysics=false`; in-air downward drift gate :43-45 covered (`air_downward_acceleration`) |
| SquidInkParticle.GlowInkProvider | not-needed | not-needed | not-needed | covered | as SquidInkParticle.Provider |
| SuspendedParticle.UnderwaterProvider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false`; `setSize(0.01F)` has no behavioral effect without physics |
| SuspendedParticle.SporeBlossomAirProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider; `ParticleLimit.SPORE_BLOSSOM` covered |
| SuspendedParticle.CrimsonSporeProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider |
| SuspendedParticle.WarpedSporeProvider | not-needed | not-needed | not-needed | not-needed | as UnderwaterProvider; `setSize(0.001F)` no effect |
| SuspendedTownParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` (SuspendedTownParticle.java:38-41); in `moves_without_collision` |
| SuspendedTownParticle.HappyVillagerProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.ComposterFillProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.DolphinSpeedProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| SuspendedTownParticle.EggCrackProvider | not-needed | not-needed | not-needed | not-needed | as SuspendedTownParticle.Provider |
| TerrainParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds, gravity 1.0, generic collision; spawn filter (air / moving_piston / `shouldSpawnTerrainParticles`, TerrainParticle.java:92) covered |
| TerrainParticle.DustPillarProvider | covered | not-needed | not-needed | not-needed | as TerrainParticle.Provider; pillar speed/lifetime covered |
| TerrainParticle.CrumblingProvider | covered | not-needed | not-needed | not-needed | as TerrainParticle.Provider; zero speed + lifetime covered |
| TotemParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; SimpleAnimated physics, friction 0.6 / gravity 1.25 |
| TrailParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick lerps toward fixed target, no `move` |
| TrialSpawnerDetectionParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics=true` (TrialSpawnerDetectionParticle.java:39), default bounds; LOOKAT_Y covered |
| VibrationSignalParticle.Provider | not-needed | not-needed | not-needed | covered | no `move`; entity-target-missing removal :77-79 covered via native entity target context |
| WakeParticle.Provider | covered | not-needed | not-needed | not-needed | initial 0.01 bounds plus per-tick `setSize(life * 0.001F)` after `move` (WakeParticle.java:46-47) |
| WaterCurrentDownParticle.Provider | not-needed | not-needed | not-needed | covered | `hasPhysics=false` (WaterCurrentDownParticle.java:17) — the `onGround` half of :45 is dead in vanilla; non-water removal covered |
| WaterDropParticle.Provider | covered | not-needed | not-needed | covered | `setSize(0.01F)` (WaterDropParticle.java:16); move, `onGround` 50% removal and collision-shape/fluid-surface gate are covered |
| WhiteAshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (WhiteAshParticle.java:22) |
| WhiteSmokeParticle.Provider | covered | not-needed | not-needed | not-needed | as SmokeParticle.Provider |
| TrackingEmitter (code-spawned) | not-needed | not-needed | not-needed | not-needed | NoRender emitter, no `move`; all five 26.1 construction sites audited: entity event 35 and animate 4/5 are covered; `LocalPlayer.crit` cannot follow a successful client `hurtOrSimulate` because its required Living target keeps base `hurtClient=false`, while `LocalPlayer.magicCrit` is unreachable because inherited `getEnchantedDamage` returns the input damage unchanged |
| FireworkParticles.Starter (code-spawned) | not-needed | covered | covered | not-needed | no `move`; camera-distance far-variant selection (FireworkParticles.java:284-287) covered; life-0 blast / large_blast (:216-238) + delayed twinkle (:273-281) covered |
| ItemPickupParticle (code-spawned) | not-needed | covered | not-needed | not-needed | follows target entity midpoint (ItemPickupParticle.java:45-49); covered incl. 3-tick lifetime and quadratic extract |
