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
- Two states are not todos, and deleting one silently re-opens a decision that
  was already made:
  - **Deferred** — a real gap, intentionally not scheduled. Carries the
    condition that turns it back into a todo.
  - **Not needed** — vanilla 26.1 has no such behavior, or bbb has no surface
    for it. Carries the vanilla evidence.
- This file does not overflow into sibling files. If it grows, the gap list grew.

---

# Todo

## Protocol coverage for remaining required 26.1 packet families

Owner: `bbb-protocol`

- Audit packet ids, field order, nullability, enum ordinals, and serverbound
  encoders against `<MC_CODE_ROOT>/sources/26.1/`, adding focused encode/decode
  tests with each packet slice.
- Cover the required paths end to end: login, configuration, play, movement,
  inventory, chat, resource-pack, interaction, command suggestion.

## Unknown clientbound packets in login, configuration, and play

Owner: `bbb-protocol` + `bbb-net` + `bbb-native`

- For each unsupported packet surfaced by probe/control diagnostics, verify it
  against local vanilla 26.1 sources, then either implement protocol decode plus
  world/runtime handling, or record why it is runtime-only.

## Native-owned business snapshots

Owner: `bbb-world` + `bbb-native` + `bbb-control`

- Move the remaining client-observable state into `WorldStore`, removing
  native-only `last_*` snapshots wherever a world owner exists or should exist.
- Keep `NetCounters` limited to connection/runtime status and command-queue
  projections.

## Signed chat and chat acknowledgement production

Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`

- Produce `ServerboundChatPacket` signatures.
- Produce non-empty `ServerboundChatCommandSignedPacket` argument signatures.
- Handle session/key state if offline-compatible servers require it.

## Crosshair entity interaction parity

Owner: `bbb-world` + `bbb-native` + `bbb-renderer`

- Validate any future `yRotA` source.
- Replace the debug target overlay with full entity model rendering and
  interaction feedback once renderer entity presentation exists.

## Native input, movement, interaction, inventory, and command flows

Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`

Movement:

- Cover the remaining vanilla survival physics beyond the native fixed-20Hz
  local movement cadence.
- Cover the remaining vanilla voxel collision shapes.
- Implement full vanilla post-`Entity.move` `deltaMovement` travel ordering.
- Add sprint-swim camera, animation, and presentation nuance.
- Implement the `minecraft:powder_snow` inside-block particle, extinguish, and
  fall-sound side effects.
- Handle the remaining `MovePlayer.Rot` vehicle movement send edge cases.
- Implement full vanilla boat physics, water/buoyancy/collision parity, and
  non-boat vehicle movement for mounted input.

Block destroy:

- Close destroy-profile gaps that `Blocks.java` property parsing cannot reach:
  constructor-level mutations and arbitrary helper/lambda evaluation
  (`InfestedBlock`, `InfestedRotatedPillarBlock`).
- Validate exact pose/fluid nuance for player destroy speed (standing-eye water
  probe).
- Implement block-specific `state.attack` callbacks and hit particles.
- Implement full model-shaped crack decals over `destroy_stage_0..9`.
- Close the remaining `STOP_DESTROY_BLOCK` sequencing gaps.

Commands:

- Add focused command-queue and encode tests for inventory, interaction, chat,
  command, and sign editing.
- Implement `ServerboundSignUpdatePacket` clipboard copy/cut/paste parity.

Inventory:

- Implement the rich-tooltip `space` font provider, the remaining text styles,
  italic/complex component styles, and component-specific detail lines from
  `font/default.json`.
- Implement recipe book and creative inventory variants.
- Implement container `0` crafting-result parity: composite / component-aware /
  special recipe display matching, and recipe-specific remainder items.
- Add the dedicated server-opened menu layouts not covered by the
  `generic_9xN` / `generic_3x3` baseline families.

## Particle runtime vanilla parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack`

- Implement full vanilla provider behavior and presentation parity. New gaps
  enter through the per-provider coverage matrix at the end of this file: add
  the row, or flip the cell back to `todo`, before cutting the slice.
- Implement option-driven atlas rendering and provider-specific option effects
  beyond spell, for `dust` and `dust_color_transition`.
- Finish the terrain/block particle GPU presentation refinements around
  `addDestroyBlockEffect` and terrain tint.
- Keep missing definition/sprite diagnostics: native spawn resolution and
  renderer counters must record them rather than dropping otherwise valid spawns.

## Renderer scene parity

Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`

Submission surfaces:

- Add renderer submission surfaces for generic name-tag text, sign/display text,
  and block-feature submissions, then wire their draw ordering into vanilla's
  `order(1).submitText` phase.

Living-entity render state:

- Track and apply the fall-flying and swimming `head.xRot` overrides in the
  head-look projection; both currently default upright.
- Project the base-`Skeleton` freeze-conversion shake from client data.
  `LivingEntityRenderer.isShaking` reads it, but `conversionTime` is currently
  server-side only.
- Project guardian eye target tracking (`lookAtPosition` / `lookDirection` /
  `eyePosition`) from entity-side state.
- Implement armor-stand animation interpolation.

Models and layers:

- Replace the placeholder raw-cuboid `Humanoid` fallback with a vanilla-faithful
  part-list model.
- Implement the happy-ghast baby model (extra `inner_body` cube, `0.2375` baby
  scale), the `bodyItem` body squeeze (`0.9375` scale with harness), the harness
  equipment layer, and the rope/lead layer, per
  `HappyGhastModel.createBodyLayer(false, NONE)`.
- Port the keyframe `AnimationState` start-tick phase offset for the bat flap
  (`BatAnimation.BAT_FLYING`).
- Implement the remaining zombie-family and piglin-family armor nuances and
  held-item refinements.
- Implement player arrows/stingers stuck in body, name display, swim arm poses,
  and elytra `speedValue` poses. The client does not track them yet.
- Implement villager/illager live profiled-player skin presentation and the
  illager arm pose.
- Implement equine boost presentation and the remaining non-base-equine
  presentation.
- Implement slime and magma-cube particle/audio coupling and crumbling.

Attachment and spawner presentation:

- Wire the camel body-anchor passenger/leash attachment consumers. The anchor
  provider `Camel.getBodyAnchorAnimationYOffset` already exists.
- Implement the ominous-item-spawner about-to-spawn and spawn sounds.
- Add the exact falling-block `MovingBlockRenderState` biome / cardinal /
  random-seed details consumed by `FallingBlockRenderer`.

Environment:

- Expand custom-dimension `EnvironmentAttributes` maps instead of collapsing
  them to the built-in dimension profile.

## HUD overlay and screen render surfaces

Owner: `bbb-renderer` + `bbb-native` + `bbb-world`

- Render each debug entry that is off by default.
- Render the entity hitbox local-server mirror (green boxes, delta arrows) and
  the 3D debug-text billboards.
- Add the component-specific advanced-tooltip lines still missing against
  vanilla `ItemStack.addDetailsToTooltip`'s component dispatch, plus
  `TooltipDisplay` hide/hidden-component gating and options persistence.
- Extend F3+I local entity capture across the remaining vanilla
  `addAdditionalSaveData` entity families and saved state that needs local
  owners, private timer/reference projection, registry-backed variant
  projection, or codec-backed SNBT projection before it can be emitted.
- Cover the full vanilla profiler section set, and add the profiling metrics
  recorder/output.
- Finish `DebugOptionsScreen` narration, focus, and widget styling.
- Implement native pause tick-freeze eligibility and the remaining `PauseScreen`
  actions and subscreens.

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
- Parse custom enchantment effects, generalizing the registry/effect projection
  beyond `minecraft:quick_charge` and the direct/tag-key enchantment predicates.

## Audio runtime parity

Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`

- Validate source/category mapping, spatial and entity-following sounds, and
  stop semantics against vanilla.
- Cover device/runtime diagnostics.
- Keep unit tests independent of an audio device.

## Official 26.1 resource-pack coverage

Owner: `bbb-pack`

- Consume atlas mip animation metadata in the renderer, beyond the current
  static sprite use.
- Apply full item tint parity for dynamic sources across every item rendering
  path.
- Load equipment asset layers for armor/equipment rendering.

Loader invariants (not todos): keep the local vanilla coverage tests
(`loads_all_local_vanilla_atlases`, `loads_local_vanilla_item_model_catalog`,
`loads_local_vanilla_item_registry`,
`loads_local_vanilla_equipment_asset_catalog`) green when changing pack loaders;
add parser support only when an official asset or a focused resource-pack
fixture fails them; keep precedence/filter tests next to the loaders.

## Resource and dynamic texture generalization

Owner: `bbb-pack` + `bbb-native` + `bbb-renderer`

- Implement broader non-profile dynamic texture loading.
- Resolve player-head block-entity `profile` owner skins through that pipeline.
  Profileless player heads use the vanilla default player skin fallback.
- Compose resource-pack overrides, custom models, and datapack registry assets
  dynamically.
- Present custom/datapack `minecraft:chicken_variant` / `minecraft:pig_variant`
  / `minecraft:cow_variant` variant assets.
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
- **Creative inventory-tab entity preview.** Restart when creative screen state
  exists.
- **Entity-in-UI preview `item_layers` GPU drawing**, per-preview override-camera
  orientation, and PIP-camera glint scroll animation time
  (`HudEntityPreviewItemLayer` / `GuiEntityRenderer`). Hand/head item models need
  a native-side baked item-quad handoff. Restart with GUI surface work.
- **Entity-in-UI item/entity lighting contexts.** Restart with GUI surface work.
- **Mount (`MountScreenOpen`) entity preview rendering.** Restart with
  entity-in-UI presentation work.
- **Full Spectator GUI selection/menu behavior.** The wheel currently adjusts
  local flying speed while the Spectator GUI menu is inactive.

## Fonts and text

- **`unihex` / CJK.** The consumed assets tree ships no `font/unifont*.zip`
  archive, so the `include/unifont` reference cannot resolve its data;
  codepoints outside the bitmap pages degrade to the `?` replacement glyph.
  Restart when a unifont source exists in the consumed assets.
- **Bidirectional shaping.** Vanilla routes through ICU4J `ArabicShaping` /
  `Bidi` in `Font` / `StringSplitter`; current consumers render logical order
  only. Restart with the rest of rich text layout.

## Container menu presentation parity

- `BeaconMenu`: confirm/cancel hover state, effect highlighted hover state,
  labels/tooltips.
- `EnchantmentMenu`: animated book model, enchanting glyph text and its
  disabled/highlight coloring, hover tooltips.
- `CartographyTableMenu`: live map pixels and decorations in the preview area,
  invalid-transform prediction when item runtime or map state is unavailable.
- `LoomMenu`: pattern-item component/tag lists beyond the single-pattern vanilla
  items, highlighted pattern buttons, banner preview, max-pattern error overlay.
- `MerchantMenu`: generic button row backgrounds and hover/focus highlight,
  selected row state if a future vanilla source adds one, trade XP result bar,
  component-aware cost predicate rendering, full trade stack decorations and
  hover tooltips, deprecated tooltip behavior, discount strikethrough.
- `LecternMenu`: rich text styles and click events, exact vanilla font wrapping,
  text filtering toggle behavior.
- `SmithingMenu`: generic held-item / non-skull head-item projection for the
  armor stand preview, cycling empty-slot icons, tooltips.

## HUD and debug overlay

- **Advancement hover title/description boxes** and full scroll/scissor behavior.
- **Heart damage/heal blink flash.** Needs the untracked
  `player.invulnerableTime` (no client-side sync) and the wall-clock
  `displayHealth` / `lastHealthTime` hold, neither reproducible
  deterministically. `HudPlayerHealth` always draws `blinking = false`; sprite
  names are already complete for when it lands.
- **Air bubble-pop sound** (`playAirBubblePoppedSound`, `Gui.java:930-937`).
  Restart when a HUD-side sound sink exists.
- **Boss bar `darken_screen` / `create_world_fog` / `play_music`.** The first two
  sit behind `boss_overlay_should_*` queries with no sky/fog consumer; the third
  has no audio consumer.
- **Accessibility text backdrop** (`textWithBackdrop`, option default `0`).
- **Jukebox now-playing path** (`Gui.setNowPlaying`, the only `animate_color=true`
  producer). The flag is already carried end to end.
- **Sound-captions subtitle overlay** (`extractSubtitleOverlay`), distinct from
  the title subtitle drawn inside `extractTitle`.
- **Stratum/blur depth-clear** (`GuiRenderer.java`, before/after blur). Restart
  when a blur-backed screen exists.
- **Client-only debug entries with no HUD line**: LocalDifficulty,
  EntitySpawnCounts, ChunkGenerationStats, PostEffect. Restart when an integrated
  local-server mirror (`ServerLevel`, `NaturalSpawner.SpawnState`,
  `ChunkGenerator` / `RandomState` / `BiomeSource`) or a renderer
  post-chain/current-post-effect mirror exists.

## Block entities and terrain

- **Block-entity `breakProgress` crumbling** and per-block-entity distance/frustum
  culling. A cross-cutting boundary shared by every block-entity renderer slice:
  block entities currently submit like entities, unculled.
- **Banner pattern no-depth-write pipeline.** Vanilla's `bannerPattern` pipeline
  disables depth writes; bbb's shared translucent pipeline keeps them on.
  Equal-depth `LessEqual` layering is unaffected. Restart only if a dedicated
  no-depth-write pass matters.
- **Sign glowing 8-way outline glyph pass.** Glowing text renders full-bright
  without outlines.
- **Sign underline/strikethrough effect bars.** They need a white pixel outside
  the single font-atlas draw.
- **Sign obfuscated glyph cycling.** The face currently draws literal glyphs.
- **Sign `filtered_messages` decoding.**
- **Sign POLYGON_OFFSET font display mode**, currently approximated by the fixed
  `TEXT_OFFSET` z gap.
- **General block-entity record removal on block change.** Only `sign_text`
  prunes today.
- **Chest christmas Dec 24-26 texture swap.** There is no wall-clock input;
  `bbb-world` serializable state must not introduce one.
- **Trial-spawner display-state behavior**, and custom `SpawnData` entity NBT
  variants, which render through default entity metadata. Restart when broader
  synthetic entity NBT projection exists.
- **Biome blend for block-break particles.** They still sample the single centre
  biome, unlike terrain tints which blend over `biomeBlendRadius`.

## Entity models and animation

- **Baby `ageScale`** — the `0.5` proportions applied in model `setupAnim`,
  distinct from the projected `SCALE`-attribute `scale`.
- **Armor trims and the remaining mob-specific armor models.**
- **Datapack/custom item prototype default `SWING_ANIMATION`** beyond the
  resolved vanilla spear ids.
- **Exact humanoid arm-pose idle-bob ordering.** The halved `arm.xRot` folds
  bbb's idle bob in, where vanilla applies the bob afterwards. The full
  bob-reorder that makes the multiply exact is the shared deferred convention.
- **Off-arm `EMPTY` reset use-item arm-pose edge.** A near-no-op: at rest the off
  arm's `yRot` is already `0`.
- **GPU outline presentation for `submitOnlyOutline` entity-attached block-model
  layers** (snow-golem carved pumpkin, mooshroom mushroom). Outline-only
  attachment metadata is recorded and ordinary block-quad baking is suppressed.
- **Guardian out-of-water spike withdrawal.** The out-of-water branch
  (`spikesAnimation = random.nextFloat()` each tick) uses unseeded client RNG and
  is not reconstructable, so the value is held steady out of water rather than
  faked.
- **Frog tongue prey-targeting** (`DATA_TONGUE_TARGET_ID`). Not part of the model
  animation.
- **Creaking tearing-down death-flicker** (`hasGlowingEyes`, a client-tick
  toggle).
- **Armadillo roll-up / roll-out shell `cube` channels**, and the baby-specific
  roll keyframes. The shell `cube` stays static while hiding; baby rolls share
  the adult roll defs.
- **Baby axolotl swim/walk/idle keyframe animations.** Vanilla `BabyAxolotlModel`
  is a separate keyframe model.
- **`EnderDragonModel.setupAnim` procedural animation**: neck/tail re-placement
  from `DragonFlightHistory`, wing flap (`flapTime`), jaw open, and the root
  `bounce` y / `z = -48` / `xRot` adjustments. The model renders at the straight
  bind layout, pitch and bounce deferred to identity at rest.
- **Zombie-nautilus variant dynamic-registry reorder path.** The variant resolves
  from synced `DATA_VARIANT_ID` by bootstrap order.
- **Nautilus `NautilusAnimation.SWIMMING` looping undulation** and the
  `AgeableMobRenderer` baby render scale (`0.7`). Both need the keyframe
  machinery plus an `AnimationState`.
- **Rabbit `IDLE_HEAD_TILT` keyframe.** `shouldPlayIdleAnimation` is gated on a
  `random.nextInt(40) + 180` timeout and is not reconstructable client-side.
- **Local TNT movement / fuse countdown simulation.** The current slice consumes
  world entity position and synced/default fuse metadata. Restart with broader
  entity-physics parity.
- **Dedicated dragon-fireball / fishing-hook / experience-orb renderers**: their
  textured camera quads and renderer-specific lighting/tint/line behavior.
- **Minecart spawner animated block-entity display content.** Classified as
  block-entity special renderer presentation.
- **Painting custom geometry** and the exact entity-solid shader/cull split for
  block/painting atlas consumers.

## GPU and resources

- **Fine-grained/coalesced GPU render-state split.** The backend folds compatible
  submissions into atlas buckets. Later GPU work should split the remaining
  coalesced render-type state, dynamic `LightTexture` / darkness-adjusted gamma,
  and diffuse visual parity. Not a blocker for the CPU submission graph.
- **Standalone mip/sampler generalization**, full vanilla mip generation, and
  standalone texture sampler parity. Belongs to resource parity work.
- **Custom-pack `EnvironmentAttribute` generalization.** Restart when a concrete
  renderer surface exists.

## Movement and interaction

- **Non-player `POWDER_SNOW_WALKABLE_MOBS` entity collision.** Restart when
  locally controlled non-player entities are added.
- **Non-player freeze-immune entity type exceptions** (`canFreeze` nuance beyond
  the local player). Restart when locally controlled entity freezing is added.
- **No-collision hazard `entityInside` effects**: sweet berry bush age-gated
  damage, fox/bee exceptions. Restart when locally controlled non-player entities
  are added.
- **Full item component payload encoding for
  `ServerboundSetCreativeModeSlotPacket`.** The queue handles empty/componentless
  stacks only. Restart when component-rich creative stacks must be sent.

## Particles and level events

- **Broader collision-clipping parity** for special contexts beyond the covered
  `WakeParticle` case.
- **Player-coupled particle emitters** beyond the currently covered scoped cases.
- **Broader entity-event particle/audio parity** outside the covered fixed attack
  sounds and Fox family.
- **Broader firework presentation** outside the Starter particle/audio path and
  the `FireworkRocketEntity` client-tick trail.
- **Idle/tick block-entity client effects** (the vault event `3015` context).
- **LevelEvent-adjacent parity owned elsewhere**: terrain/item atlas rendering,
  block-entity client-effect presentation, audio-device/runtime parity. The
  `LevelEventHandler` switch cases themselves are covered.

---

# Not needed

Vanilla 26.1 has no such behavior, or bbb has no surface for it. Each carries the
vanilla evidence. Deleting one of these re-opens work already decided against.

## Entity models and layers

- **Chicken head look.** `ChickenModel` never animates the head.
- **Freezing white overlay.** The creeper is the only vanilla
  `getWhiteOverlayProgress` producer; the base renderer returns `0`.
  `isFullyFrozen` drives the `setupRotations` body shake, not an overlay.
- **Zoglin conversion shake.** `ZoglinRenderer` has no conversion override.
- **Snow-golem walk-driven swing.** Its `setupAnim` is the head-yaw twist/orbit.
- **Piglin-brute `ADMIRING_ITEM` pose.** The brute has no admire branch.
- **Fox baby model scale.** `Fox.BABY_SCALE = 0.6` scales only the bounding box,
  so the baby uses the standard `0.5` `LivingEntity.getAgeScale`.
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
- **Exact guardian tail starting phase.** Vanilla seeds it with a per-spawn
  `random.nextFloat()`, which is non-deterministic. bbb starts at `0.0`; the sway
  dynamics are exact.
- **Exact enderman creepy root-jitter RNG seed.** Vanilla keeps that RNG inside
  the renderer and never exposes the seed over the protocol; bbb uses a
  deterministic Java-LCG-shaped seed from entity id + age while preserving the
  vanilla axis/scale behavior.
- **Exact horse `tailCounter` client seed.** Not protocol-visible; bbb uses a
  deterministic local Java LCG.

## Rendering and screenshots

- **Exact golden pixel parity to vanilla screenshots.** No golden PNG or
  full-frame hash: driver and float jitter make it unreliable.
- **Item-pipeline overlay-texture (`Sampler1`) sampling.** Vanilla's
  `ITEM_SNIPPET` does not bind `Sampler1`; `core/item` samples only the item
  atlas and lightmap, and `RenderTypes.itemCutout` / `itemTranslucent` never call
  `useOverlay()`.
- **Biome `BLOCK_LIGHT_TINT` / `NIGHT_VISION_COLOR` overrides.** Vanilla biome
  JSON does not provide them.
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
