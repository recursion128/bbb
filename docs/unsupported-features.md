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
  - **Deferred** — a real gap, intentionally not scheduled. Carries the
    condition that turns it back into a todo.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Runtime performance parity and stable frame tails

Owner: `bbb-native` + `bbb-renderer` + `bbb-world`

- Measure and reduce the remaining dynamic item-model property-selection,
  draw/pipeline state churn, and driver-deferred resident-release tails. Any
  change must preserve exact phase/translucent order and the explicit
  bounded-owner fallback rather than dropping visible instances.
- Complete equal-load official Java 26.1 CPU p50/p95/p99, GPU-frame, 1%-low,
  and chunk-update comparison captures; report any losing scene by measured
  owner instead of reducing visible work.

## Signed chat and chat acknowledgement production

Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`

- Authenticated Services block-list synchronization and player-report
  composition/submission remain external account/social boundaries. The native
  `ChatListener` now owns receive-ability and secure-only gates, bounded delayed
  delivery, trust tags, the 1,024-entry reporting log, rich HUD interaction, and
  narrator delivery; startup hidden/blocked UUIDs are not a substitute for a
  signed-in Services client or reporting UI.
- Exact `show_item` / `show_entity` hover payloads and custom font ids are
  retained in canonical Components, but chat currently renders bounded
  name/id/count hover fallbacks and the active default font when a custom font
  provider is unavailable. Server-supplied `open_file` remains deliberately
  inert; completing reusable item/entity tooltip rendering or additional font
  providers must consume those retained values rather than add a parallel
  Component schema.

## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Movement:
- Add sprint-swim camera, animation, and presentation nuance.

Commands:

- Bottle dragon-breath client prediction is `not-needed`: `AreaEffectCloud.owner`
  is absent from both `ClientboundAddEntityPacket` and synchronized metadata, so
  the remote 26.1 client leaves that branch server-authoritative. Reopen if the
  owner becomes client-visible on the wire.
Inventory:

- Creative hotbar presets (`HotbarManager`) are implemented end to end: the
  `C`/`X` activator-key lifecycle from `Minecraft.handleKeybinds` and
  `CreativeModeInventoryScreen.handleHotbarLoadOrSave`, nine persistent
  nine-slot groups, and the `inventory.hotbarSaved` overlay/narration. The
  store is a bbb-owned JSON file of `ItemStackSummary` rows
  (`hotbar-presets.json`) rather than vanilla's registry-coupled `hotbar.nbt`
  `ItemStack` codec, so preset files do not interchange with a vanilla
  installation. Reopen the file-format half only if full item NBT codec
  support with registry context lands.
- Implement the rich-tooltip `space` font provider, the remaining text styles,
  italic/complex component styles, and component-specific detail lines from
  `font/default.json`.

## Renderer scene parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`

Submission surfaces:

- Add renderer submission surfaces for remaining block-feature submissions,
  then wire their draw ordering into vanilla's
  ordered text phase. Texture-backed special geometry added by these surfaces
  must record complete submission metadata before atlas lookup rather than
  reuse the no-texture custom-geometry path.

Living-entity render state:

- Project the remaining conversion-time shaking input;
  `LivingEntityRenderer.isShaking` reads it, but `conversionTime` is currently
  server-side only.
- Armor-stand pose interpolation is `not-needed`: vanilla 26.1
  `ArmorStandRenderer.extractRenderState` copies all six synced `Rotations`
  getters directly without partial-tick interpolation. Only `yRot` uses
  `Mth.rotLerp`, and hit `wiggle` adds `partialTicks`; bbb already covers both.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

- `chunk_section_paths`, `chunk_section_octree`, and
  `chunk_section_visibility` are `deferred`: stable per-section CPU/GPU
  residency now exists, but a repo-native `SectionOcclusionGraph`/octree and
  compiled-section face-visibility matrix do not. Restart by wiring the stable
  resident section owners into vanilla node step/source directions, octree node
  bounds/depth/frustum state, and `facesCanSeeEachother` without reconstructing
  them heuristically.
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
  branch are covered; locally controlled vehicle physics stays tracked under
  the existing full boat/vehicle movement item.
- Entity-hitbox local-server green boxes and delta arrows are `not-needed`:
  vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` returns an entity
  only through `Minecraft.getSingleplayerServer()` -> `IntegratedServer` ->
  `ServerLevel.getEntity`; an external multiplayer client always receives
  `null` and emits the red `Missing Server Entity` 3D billboard instead. bbb
  owns no integrated server and implements that missing-label branch. Reopen
  with an in-process integrated server boundary.
## Item model range-dispatch and select projection

Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`

Each item below plugs into the existing value-aware `RangeDispatch` / `Select`
resolver by adding a value provider. No new selection machinery is required.

- Thread newly discovered ambient-context numeric `range_dispatch` properties
  through the icon resolver as that state becomes available to the GUI icon path.
- Wire `minecraft:context_entity_type` for non-GUI item consumers that gain a
  real living owner outside the current owner-backed generated held-item path.
- Complete `minecraft:local_time`: full localized symbols and the long-tail ICU
  pattern fields — locale-specific week data beyond the selected English
  regional groups, IANA long `z`, generic `v`, and one-/four-letter `V` widths.
- Extend `minecraft:component` select to complex object/list component values,
  style-sensitive `Component` equality, registry-backed component value codecs,
  custom/datapack component defaults, custom datapack component value decoding,
  and components without a persistent codec.
- Pass dynamic registry keys into the item resolver for the non-GUI item
  consumers that render component-bearing generated stacks where vanilla
  resolves registry-backed item-model properties. No-registry consumers still
  fall back.
- Parse and apply the remaining synchronized enchantment effect component
  families beyond attribute modifiers and deterministic
  `minecraft:crossbow_charge_time`: conditional value/entity/location effects
  still need their loot-context requirements, and `remove_binomial` needs the
  holder entity's mutable `RandomSource` rather than a fabricated random result.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

- Validate remaining synthesized/game-event sound mappings against vanilla.
- Client-owned animal ambient/step/eat and cat/wolf state-trigger loops are
  `not-needed`: vanilla `Entity.playSound` delegates through
  `Level.playSound(null, ...)`, `ClientLevel.playSeededSound` only submits when
  the excluded entity is its local player, and `ServerLevel` supplies remote
  listeners with Sound/SoundEntity packets. Reopen only if a target animal path
  is shown to call `playLocalSound` or stops being server-packet-owned.
- Clientbound SoundEntity holder/category/entity/volume/pitch/seed handling,
  missing or silent entity gates, fixed range, entity following, and removal stop
  are covered.
- Clientbound Sound holder/category/fixed-point position/volume/pitch/seed handling,
  no-distance-delay behavior, fixed range, registry resolution, seeded selection,
  and spatial attenuation are covered.
- Clientbound StopSound covers source/name/all four filter combinations and unknown
  flag bits. Filtered stops affect active sounds only; stop-all also clears queued
  distance-delayed sounds, matching SoundEngine rather than over-stopping the queue.
- Keep unit tests independent of an audio device.

## Official 26.1 resource-pack coverage

Owner: `bbb-pack` + `bbb-item-model` + `bbb-native` + `bbb-renderer`

Loader invariants (not todos): keep the local vanilla coverage tests
(`loads_all_local_vanilla_atlases`, `loads_local_vanilla_item_model_catalog`,
`loads_local_vanilla_item_registry`,
`loads_local_vanilla_equipment_asset_catalog`) green when changing pack loaders;
add parser support only when an official asset or a focused resource-pack
fixture fails them; keep precedence/filter tests next to the loaders.

## Playable live coverage beyond the session-lifecycle closure

Owner: `scripts/playable-gate.sh` + `scripts/playable_gate.py` + affected runtime
owners

- Add server-result assertions for entity interaction, hotbar, container,
  inventory, and chat/command flows beyond the existing movement and
  destroy/place closure.
- Add presentation evidence for representative audio/particle effects and a
  deterministic screenshot/readback. Renderer/audio counters must identify the
  exact lifecycle generation rather than merely proving that the processes
  stayed alive.
- Keep failure evidence bounded and stage-specific while adding those surfaces;
  do not turn the top-level opening `world_snapshot` into a misleading
  failure-time sample.

The current live gate already covers two official servers in one run: death and
respawn, both dimension replacements, production resource-pack push/pop,
Play-to-Configuration-to-Play, and same-process Transfer reconnect with a fresh
registry session. This boundary is an invariant for the remaining breadth; it
does not imply complete Play protocol, entity behavior, presentation, or
long-session coverage.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

Lifecycle invariants (not todos): vanilla, startup and server packs share one
validated low-to-high selected stack and immutable generation; CPU consumers
prepare from that stack, renderer/audio owners commit atomically, and control
publishes source/priority/failure plus bounded dynamic-texture observations.
Skin/cape/elytra requests share one exact resource/session-generation cache.

- Implement broader non-profile dynamic texture loading.
- Connect synchronized custom block/entity registry identities to the existing
  resource-pack model/texture loaders. The selected-generation fixtures already
  cover custom item/cuboid model, PNG texture, font, sound and data-resource
  precedence; do not infer unsupported registry mappings from those loader tests.
- Give failed, pending, and ready states an explicit fallback. Never draw a
  stale texture.

## Bundle selected-item icon state

Owner: `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack`

- Extend renderer/UI coverage of the selected-item icon beyond hotbar icon
  snapshots.

---

# Deferred

Real gaps, intentionally not scheduled. Each carries the condition that turns it
back into a todo. Do not delete these.

## Screens and presentation

- **Code of Conduct screen presentation parity.** Canonical UI state and control
  requests are covered. Restart when the renderer UI stack grows fuller vanilla
  screen and font rendering.
- **Entity-in-UI preview `item_layers` GPU drawing**, per-preview override-camera
  orientation, and PIP-camera glint scroll animation time
  (`HudEntityPreviewItemLayer` / `GuiEntityRenderer`). Hand/head item models need
  a native-side baked item-quad handoff. Restart with GUI surface work.
- **Entity-in-UI item/entity lighting contexts.** Restart with GUI surface work.
## Fonts and text

- **`unihex` / CJK.** The consumed assets tree ships no `font/unifont*.zip`
  archive, so the `include/unifont` reference cannot resolve its data;
  codepoints outside the bitmap pages degrade to the `?` replacement glyph.
  Restart when a unifont source exists in the consumed assets.
- **Bidirectional shaping.** Vanilla routes through ICU4J `ArabicShaping` /
  `Bidi` in `Font` / `StringSplitter`; current consumers render logical order
  only. Restart with the rest of rich text layout.

## Container menu presentation parity

- Keep clientbound container lifecycle audits aligned with vanilla even when a
  packet field is redundant: ContainerClose carries an id but the 26.1 client
  closes its current menu unconditionally; matching-id behavior is not an
  additional authority check.
- ContainerSetData is gated by the current `containerMenu` id in vanilla.
  Mismatched property updates must remain ignored rather than creating a future
  synthetic menu or replacing the active one.
- ContainerSetSlot also rejects mismatched nonzero menu ids.
- PlayerCombatEnter is a strict unit packet whose 26.1 client handler is empty;
  its diagnostic counter is not evidence of an additional combat-mode runtime.
- PlayerCombatEnd likewise has an empty 26.1 handler; its decoded duration may
  remain diagnostic but does not establish a client combat-mode transition.
- `BeaconMenu`: render confirm/cancel hover state, selected-effect highlight and
  hover state, labels, and tooltips.
- `EnchantmentMenu`: animated book model, enchanting glyph text and its
  disabled/highlight coloring, hover tooltips.
- `CartographyTableMenu`: live map pixels and decorations in the preview area,
  invalid-transform prediction when item runtime or map state is unavailable.
- `LoomMenu`: render each ordered selectable-pattern banner thumbnail, the
  composited banner preview, and the max-pattern error overlay.
- `LecternMenu`: rich text styles and click events, exact vanilla font wrapping,
  text filtering toggle behavior.
- `SmithingMenu`: generic held-item / non-skull head-item projection for the
  armor stand preview, cycling empty-slot icons, tooltips.

## HUD and debug overlay

- **Heart damage/heal blink flash.** Needs the untracked
  `player.invulnerableTime` (no client-side sync) and the wall-clock
  `displayHealth` / `lastHealthTime` hold, neither reproducible
  deterministically. `HudPlayerHealth` always draws `blinking = false`; sprite
  names are already complete for when it lands.
- **Air bubble-pop sound** (`playAirBubblePoppedSound`, `Gui.java:930-937`).
  Restart when a HUD-side sound sink exists.
- **Accessibility text backdrop** (`textWithBackdrop`, option default `0`).
- **Jukebox now-playing path** (`Gui.setNowPlaying`, the only `animate_color=true`
  producer). The flag is already carried end to end.
- **Sound-captions subtitle overlay** (`extractSubtitleOverlay`), distinct from
  the title subtitle drawn inside `extractTitle`.
- **Client-only debug entries with no HUD line**: LocalDifficulty,
  EntitySpawnCounts, ChunkGenerationStats. Restart when an integrated
  local-server mirror (`ServerLevel`, `NaturalSpawner.SpawnState`,
  `ChunkGenerator` / `RandomState` / `BiomeSource`) exists.

## Block entities and terrain

- **Banner pattern no-depth-write pipeline.** Vanilla's `bannerPattern` pipeline
  disables depth writes; bbb's shared translucent pipeline keeps them on.
  Equal-depth `LessEqual` layering is unaffected. Restart only if a dedicated
  no-depth-write pass matters.
- **Sign glowing 8-way outline glyph pass.** Glowing text renders full-bright
  without outlines.
- **Sign underline/strikethrough effect bars.** They need a white pixel outside
  the single font-atlas draw.
- **Sign obfuscated glyph cycling.** The face currently draws literal glyphs.
- **Sign POLYGON_OFFSET font display mode**, currently approximated by the fixed
  `TEXT_OFFSET` z gap.
- **Chest christmas Dec 24-26 texture swap.** There is no wall-clock input;
  `bbb-world` serializable state must not introduce one.
- **Trial-spawner display-state behavior**, and custom `SpawnData` entity NBT
  variants, which render through default entity metadata. Restart when broader
  synthetic entity NBT projection exists.
- **Biome blend for block-break particles.** They still sample the single centre
  biome, unlike terrain tints which blend over `biomeBlendRadius`.

## Entity models and animation

### Deferred model and animation details

- **Remaining mob-specific armor models.**
- **Datapack/custom item prototype default `SWING_ANIMATION`** beyond the
  resolved vanilla spear ids.
- **GPU outline presentation for `submitOnlyOutline` entity-attached block-model
  layers** (snow-golem carved pumpkin, mooshroom mushroom). Outline-only
  attachment metadata is recorded and ordinary block-quad baking is suppressed.
- **Frog tongue prey-targeting** (`DATA_TONGUE_TARGET_ID`). Not part of the model
  animation.
- **Zombie-nautilus variant dynamic-registry reorder path.** The variant resolves
  from synced `DATA_VARIANT_ID` by bootstrap order.
- **Rabbit `IDLE_HEAD_TILT` keyframe — not-needed.** Vanilla starts it from the
  entity's client-local `random.nextInt(40) + 180` timeout, so a remote client's
  particular phase is not reconstructable and bbb must not fabricate a seed.
- **Local TNT movement / fuse countdown simulation.** The current slice consumes
  world entity position and synced/default fuse metadata. Restart with broader
  entity-physics parity.
- **Client projectile presentation movement / gravity / drag tick parity.** Canonical
  `Projectile` parent lifecycle now consumes the current transform, delta, real
  type bounds, and owner mount graph, but bbb does not yet advance every remote
  projectile subclass between server movement packets. This covers only the
  motion/interpolation/trail/lifecycle work performed by the multiplayer client;
  hit resolution, damage, explosions, block changes, and authoritative removal
  remain server-owned. `LeftOwner` is sticky
  and exact once the canonical projectile box separates; completing per-subclass
  client motion will make that separation occur at vanilla's intermediate tick
  instead of the next authoritative transform update.
- **Minecart spawner animated block-entity display content.** Classified as
  block-entity special renderer presentation.
## GPU and resources

- **Fine-grained/coalesced GPU render-state split.** The backend folds compatible
  submissions into atlas buckets. Later GPU work should split the remaining
  coalesced render-type state, dynamic `LightTexture` / darkness-adjusted gamma,
  and diffuse visual parity. Not a blocker for the CPU submission graph.
- **Standalone mip/sampler generalization**, full vanilla mip generation, and
  standalone texture sampler parity. Belongs to resource parity work.
## Movement and interaction

- **Non-player `POWDER_SNOW_WALKABLE_MOBS` entity collision.** Restart when
  locally controlled non-player entities are added.
- **Non-player freeze-immune entity type exceptions** (`canFreeze` nuance beyond
  the local player). Restart when locally controlled entity freezing is added.
- **No-collision hazard `entityInside` effects**: sweet berry bush age-gated
  damage, fox/bee exceptions. Restart when locally controlled non-player entities
  are added.
## Particles and level events

- **Broader collision-clipping parity** for special contexts beyond the covered
  `WakeParticle` case.
- **Player-coupled particle emitters** beyond the currently covered scoped cases.
- **Idle/tick block-entity client effects** beyond packet-owned level events (the
  vault event `3015` context remains the audit anchor).
- **LevelEvent-adjacent parity owned elsewhere**: terrain/item atlas rendering,
  block-entity client-effect presentation, and audio-device/runtime parity. The
  `LevelEventHandler` switch cases themselves are covered.

---

# Not needed

Vanilla 26.1 has no such behavior, or bbb has no surface for it. Each carries the
vanilla evidence. Deleting one of these re-opens work already decided against.

## Packet canonical state

- **Mojang client telemetry callbacks.** `bbb` has no external telemetry sender.
  Vanilla `WorldSessionTelemetryManager.onServerBrandReceived` only stores the
  brand on `WorldLoadEvent` and attempts a Mojang telemetry send; server-brand
  protocol state and local diagnostics remain independently owned.

- **Bundle delimiter world state.** `ClientboundBundleDelimiterPacket` is
  consumed by `PacketBundlePacker` / `BundlerInfo` as connection framing before
  listener dispatch; it has no payload, listener handler, or canonical world
  mutation. Bundle assembly itself remains a connection-runtime todo above.

## Entity models and layers

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
- **Separate glow-squid emissive overlay/texture.** 26.1 has none; the glow is
  purely a block-light override.
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

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Fire-resistance lava fog — deferred.** Vanilla widens lava fog only when the
  active camera is actually in lava and has Fire Resistance. Restart when the
  camera environment owns a real lava fog medium; an effect-only approximation
  would incorrectly alter atmospheric and water fog.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
- **Lightning sky flash tinting water fog.** Vanilla does not tint it.

## Block entities

- **Sign `is_waxed` render effect.** Vanilla has none; it gates editing.
- **Banner item / shield pattern path** (`Sheets.SHIELD_PATTERN_BASE`,
  `BannerRenderer.submitSpecial`). Item-model scope, not the block-entity
  renderer.
- **Shulker box open/close sound and interaction blocking.** Gameplay-side, not
  the block-entity renderer.
- **Bell resonation particle/glow chain** (`resonationTicks`, raider search).
  Gameplay-side, belongs with the raid features.

## Particles

- **Spawn-filtering for `minecraft:block_marker`.** Vanilla `BlockMarker.Provider`
  does not reject air, `moving_piston`, or `shouldSpawnTerrainParticles=false`
  states the way `TerrainParticle.createTerrainParticle` does.
- **`moving_piston` rejection in the destroy-block effect** (events `2001` /
  `3008`). Vanilla `addDestroyBlockEffect` does not reject it, unlike
  `TerrainParticle.Provider`.
- **xAux handling in `GustParticle.Provider`.** Vanilla ignores that xAux; the
  `GustSeedParticle.Provider` child still feeds child xAux `age / lifetime`.

## Input, movement, and interaction

- **Server-owned FishingHook catching/removal outcomes.** The client tick now
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
  Loaders reject them intentionally.

---

# Appendix: per-provider particle coverage matrix

Not history: this is the entry point for particle work. A new provider gap
is opened by adding a row or flipping a cell back to `todo`, before the
slice is cut. `not-needed` cells carry the vanilla evidence and must not be
deleted.


Usage: no open todo cells remain — every cell is `covered` or `not-needed`.
When a new provider behavior gap is found, first add the row (or flip the
affected cell back to `todo` with a root-cause tag), then cut a slice for
it; when the slice lands, flip the cell to `covered` and append the commit
hash to the row's notes. `not-needed` means vanilla 26.1 has no such
behavior for that provider (reason noted); do not pick those rows. Every row was judged
against the vanilla 26.1 class under
`~/Work/mc-code/sources/26.1/net/minecraft/client/particle/` (line references
in notes), cross-checked against the bbb runtime
(`crates/bbb-renderer/src/particles/descriptors.rs` — `collision_size()`,
`moves_without_collision()`, `drip_fluid()`, `required_fluid()`,
`air_downward_acceleration()`, `tick_motion()` —
`crates/bbb-renderer/src/particles/instance.rs`, and the completion history
earlier in this file).

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
  are tracked by those histories, not here.
- removal-gates: removal (or motion gates) driven by block / fluid / entity
  state queries during tick. Collision-driven removal (e.g. leaves
  `onGround`) belongs to the collision column.

Shared todo root causes (one slice may clear several rows):

- `[bounds]`: vanilla per-provider collision AABB (`setSize` /
  `scale`-derived) vs the former default 0.2x0.2 fallback in
  `descriptors.rs collision_size()`. RESOLVED (this slice): the 24 static
  per-provider sizes are now table-driven in `collision_size()` — drip family
  0.01 (DripParticle.java:25), rain / splash 0.01 (WaterDropParticle.java:16),
  bubble / bubble column 0.02 (BubbleParticle.java:22 /
  BubbleColumnUpParticle.java:24), soul 0.3 and firefly 0.3 (`scale(1.5F)` →
  `Particle.scale` `setSize(0.2F * 1.5F)` = 0.3, Particle.java:77-80). The
  remaining collision `todo` rows are `[leaf-bounds]` and `[wake-grow]`.
- `[leaf-bounds]`: FallingLeaves uses a per-spawn random size
  (`setSize(size, size)`, `size = scale * (0.05F | 0.075F)`,
  FallingLeavesParticle.java:41-43) — needs per-instance collision size, not
  a static table entry. RESOLVED (this slice): `from_spawn_command_*` sets the
  collision box to the already-sampled `visual.base_quad_size` (= vanilla
  `size`) for all three leaf providers, reusing the existing `nextBoolean`
  draw so the spawn RNG sequence is unchanged.
- `[wake-grow]`: WakeParticle re-sizes its collision box every tick
  (`setSize(life * 0.001F)`, WakeParticle.java:46-47). RESOLVED (this slice):
  the Wake tick arm updates `collision_width`/`collision_height` to
  `life * 0.001` after `move`, so the growth trails the move exactly as in
  vanilla (initial 0.01 box still comes from `collision_size()`).
- `[nearest-player]`: PlayerCloudParticle pulls toward
  `level.getNearestPlayer(x, y, z, 2.0, false)` — the nearest of all
  players, not only the local one (PlayerCloudParticle.java:51-58).
  RESOLVED (this slice): the native pump now projects a candidate list
  (`particle_player_motion_contexts` — the local player pose plus every
  `VANILLA_ENTITY_TYPE_PLAYER_ID` entity transform, spectators excluded on
  both paths per `EntitySelector.NO_SPECTATORS`, creative kept because
  `filterOutCreative=false`, EntityGetter.java:95-98), and the renderer
  cloud/sneeze tick resolves the strict nearest candidate within 2.0 per
  particle (`dist < range * range`, minimum squared distance,
  EntityGetter.java:74-88; the former local-only path also accepted
  `dist == 4.0`, now excluded). Remote player `delta_movement` comes from
  the entity transform state, matching the pull's
  `player.getDeltaMovement().y` read.

| Provider | collision | player-coupled | sounds | removal-gates | Notes |
| --- | --- | --- | --- | --- | --- |
| AshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (AshParticle.java:19, BaseAshSmoke last ctor arg); curves covered in history above |
| AttackSweepParticle.Provider | not-needed | not-needed | not-needed | not-needed | tick never calls `move` (AttackSweepParticle.java:28-37) |
| BlockMarker.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` + zero velocity (BlockMarker.java:14-17); vanilla has no spawn filter here — bbb matches |
| BreakingItemParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default 0.2 bounds correct; generic collision callback; gravity 1.0 |
| BreakingItemParticle.SlimeProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.CobwebProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BreakingItemParticle.SnowballProvider | covered | not-needed | not-needed | not-needed | as BreakingItemParticle.Provider |
| BubbleColumnUpParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.02F)` (BubbleColumnUpParticle.java:24) now in `collision_size()` (this slice); non-water removal :35-37 covered (`required_fluid`, descriptors.rs:2071-2078) |
| BubbleParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.02F)` (BubbleParticle.java:22) now in `collision_size()` (this slice); +0.002 rise / 0.85 damping covered; non-water removal :43-45 covered |
| BubblePopParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; direct-gravity-no-friction tick covered |
| CampfireSmokeParticle.CosyProvider | covered | not-needed | not-needed | not-needed | 0.25 bounds special-cased (descriptors.rs:2048-2050); `alpha<=0` removal + random drift covered |
| CampfireSmokeParticle.SignalProvider | covered | not-needed | not-needed | not-needed | as CosyProvider |
| CritParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (CritParticle.java:35) |
| CritParticle.DamageIndicatorProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| CritParticle.MagicProvider | not-needed | not-needed | not-needed | not-needed | as CritParticle.Provider |
| DragonBreathParticle.Provider | covered | not-needed | not-needed | not-needed | `hasPhysics=false` (DragonBreathParticle.java:34) — the `onGround`/`hasHitGround` branch (:48-55) is dead in vanilla because `move` never sets `onGround` without physics; bbb models the tick formula including the branch |
| DripParticle.LavaHangProvider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.01F)` (DripParticle.java:25) now in `collision_size()` (this slice); lava-surface removal :58-64 covered (`drip_fluid`); cooling RGB + hang damping + fall child covered |
| DripParticle.LavaFallProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` → land child covered; lava-surface removal covered |
| DripParticle.LavaLandProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); lava-surface removal covered |
| DripParticle.WaterHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); water-surface removal covered |
| DripParticle.WaterFallProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` → splash child covered; water-surface removal covered |
| DripParticle.HoneyHangProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` → no fluid gate (DripParticle.java:366-371) |
| DripParticle.HoneyFallProvider | covered | not-needed | covered | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `BEEHIVE_DRIP` on land (DripParticle.java:313-320) covered via renderer particle sound events; `Fluids.EMPTY` |
| DripParticle.HoneyLandProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.NectarFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `onGround` removal covered; `Fluids.EMPTY` |
| DripParticle.SporeBlossomFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.ObsidianTearHangProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY`; glow light covered |
| DripParticle.ObsidianTearFallProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.ObsidianTearLandProvider | covered | not-needed | not-needed | not-needed | `[bounds]` 0.01 now in `collision_size()` (this slice); `Fluids.EMPTY` |
| DripParticle.DripstoneWaterHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); water-surface removal covered |
| DripParticle.DripstoneWaterFallProvider | covered | not-needed | covered | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `POINTED_DRIPSTONE_DRIP_WATER` on land (DripParticle.java:156-162) covered |
| DripParticle.DripstoneLavaHangProvider | covered | not-needed | not-needed | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); cooling RGB covered; lava-surface removal covered |
| DripParticle.DripstoneLavaFallProvider | covered | not-needed | covered | covered | `[bounds]` 0.01 now in `collision_size()` (this slice); `POINTED_DRIPSTONE_DRIP_LAVA` on land covered |
| DustParticle.Provider | covered | not-needed | not-needed | not-needed | no `setSize` → default bounds; `hasPhysics=true` + `speedUpWhenYMotionIsBlocked` via generic path |
| DustColorTransitionParticle.Provider | covered | not-needed | not-needed | not-needed | as DustParticle.Provider; transition lerp covered |
| DustPlumeParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (DustPlumeParticle.java:22, BaseAshSmoke last arg); gravity/friction decay covered |
| ElderGuardianParticle.Provider | not-needed | not-needed | not-needed | not-needed | zero velocity, gravity 0 — base tick moves nothing; special-group render covered |
| EndRodParticle.Provider | not-needed | not-needed | not-needed | not-needed | collision-free `move` override (EndRodParticle.java:22-25); in `moves_without_collision` |
| ExplodeParticle.Provider | covered | not-needed | not-needed | not-needed | default bounds; physics, gravity -0.1, friction 0.9 |
| FallingDustParticle.Provider | covered | not-needed | not-needed | not-needed | custom tick move + `onGround` roll reset covered; spawn `INVISIBLE` rejection + tint covered; no removal gate in vanilla |
| FallingLeavesParticle.CherryProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` per-spawn `setSize(size, size)` (0.05/0.075) now driven by the sampled `base_quad_size` in `from_spawn_command_*` (this slice); `onGround` + horizontal-block removal + first-tick grace covered — removal is collision-driven, no state query |
| FallingLeavesParticle.PaleOakProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` (0.1/0.15) covered (this slice); rest as CherryProvider |
| FallingLeavesParticle.TintedLeavesProvider | covered | not-needed | not-needed | not-needed | `[leaf-bounds]` (0.1/0.15) covered (this slice); ARGB tint covered |
| FireflyParticle.FireflyProvider | covered | not-needed | not-needed | covered | `[bounds]` provider `scale(1.5F)` → `setSize(0.3F)` (FireflyParticle.java:94, Particle.java:77-80) now in `collision_size()` (this slice); generic move + non-air removal :50-51 covered |
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
| PlayerCloudParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (PlayerCloudParticle.java:32); `[nearest-player]` nearest-of-all-players pull covered (this slice): native projects local + remote player candidates minus spectators, renderer picks the strict nearest within 2.0 per particle |
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
| SoulParticle.Provider | covered | not-needed | not-needed | not-needed | `[bounds]` `scale(1.5F)` → `setSize(0.3F)` (SoulParticle.java:17 + Particle.java:77-80) now in `collision_size()` (this slice); rising motion covered |
| SoulParticle.EmissiveProvider | covered | not-needed | not-needed | not-needed | as SoulParticle.Provider; `[bounds]` 0.3 covered (this slice) |
| SpellParticle.Provider | not-needed | covered | not-needed | not-needed | `hasPhysics=false` (SpellParticle.java:33); scoping alpha (:35-37, :49-53, :62-69) covered — vanilla itself only checks the local player |
| SpellParticle.InstantProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; RGB + power covered |
| SpellParticle.MobEffectProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; ARGB covered |
| SpellParticle.WitchProvider | not-needed | covered | not-needed | not-needed | as SpellParticle.Provider; magenta brightness covered |
| SpitParticle.Provider | covered | not-needed | not-needed | not-needed | ExplodeParticle physics, gravity 0.5, default bounds |
| SplashParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` inherits WaterDrop `setSize(0.01F)` (WaterDropParticle.java:16 via `super`) now in `collision_size()` (this slice); `onGround` 50% removal + collision-shape/fluid-surface gate :39-55 covered |
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
| WakeParticle.Provider | covered | not-needed | not-needed | not-needed | `[wake-grow]` initial `setSize(0.01F)` covered (descriptors.rs:2051) + per-tick `setSize(life * 0.001F)` growth :46-47 now applied after `move` in the Wake tick arm (this slice) |
| WaterCurrentDownParticle.Provider | not-needed | not-needed | not-needed | covered | `hasPhysics=false` (WaterCurrentDownParticle.java:17) — the `onGround` half of :45 is dead in vanilla; non-water removal covered |
| WaterDropParticle.Provider | covered | not-needed | not-needed | covered | `[bounds]` `setSize(0.01F)` (WaterDropParticle.java:16) now in `collision_size()` (this slice); move + `onGround` 50% removal + collision-shape/fluid-surface gate covered |
| WhiteAshParticle.Provider | not-needed | not-needed | not-needed | not-needed | `hasPhysics=false` (WhiteAshParticle.java:22) |
| WhiteSmokeParticle.Provider | covered | not-needed | not-needed | not-needed | as SmokeParticle.Provider |
| TrackingEmitter (code-spawned) | not-needed | not-needed | not-needed | not-needed | NoRender emitter, no `move`; entity-AABB sampling covered for the wired paths (entity event 35, animate 4/5); other vanilla construction sites belong to their entity-event slices |
| FireworkParticles.Starter (code-spawned) | not-needed | covered | covered | not-needed | no `move`; camera-distance far-variant selection (FireworkParticles.java:284-287) covered; life-0 blast / large_blast (:216-238) + delayed twinkle (:273-281) covered |
| ItemPickupParticle (code-spawned) | not-needed | covered | not-needed | not-needed | follows target entity midpoint (ItemPickupParticle.java:45-49); covered incl. 3-tick lifetime and quadratic extract |

Row and cell counts (2026-07-05; player-coupled counts updated after the
nearest-player slice cleared `[nearest-player]`, collision counts after the
dynamic-collision-size slice cleared `[leaf-bounds]` + `[wake-grow]`):

- 113 rows: 110 distinct provider classes registered by vanilla 26.1
  `ParticleResources.registerProviders()` (lines 56-172; 117 registrations
  minus 7 duplicate-class registrations) plus 3 code-spawned particles
  (`TrackingEmitter`, `FireworkParticles.Starter`, `ItemPickupParticle`).
- collision: 56 covered / 57 not-needed / 0 todo (the dynamic-collision-size
  slice flipped the last 4: 3 `[leaf-bounds]` leaf providers now sized from the
  sampled `base_quad_size`, and `[wake-grow]` Wake now grows `life * 0.001`
  per tick)
- player-coupled: 8 covered / 105 not-needed / 0 todo (the nearest-player
  slice flipped the last 2: PlayerCloud + Sneeze now pull toward the
  nearest of all players, not only the local one)
- sounds: 4 covered / 109 not-needed / 0 todo
- removal-gates: 18 covered / 95 not-needed / 0 todo
- 0 todo cells — the table is fully `covered` / `not-needed`; new provider
  behavior gaps re-enter as new `todo` cells (see Usage above).
