# Unsupported And Deferred Feature Ledger

This is the project ledger for native Minecraft Java 26.1 features known to be:

- unsupported
- partial
- diagnostic-only
- intentionally deferred

Each remaining item must have an owner, status, and next action before the
rewrite can be considered complete.

When an agent does any of the following, update this file in the same slice:

- Adds a new unsupported behavior.
- Preserves a diagnostic-only path.
- Discovers a vanilla feature gap that cannot be closed in the current slice.

## Status Key

- `covered`:
  - implemented for the currently claimed scope
  - future work stays derived from the documented owner
- `partial`:
  - some behavior exists
  - vanilla parity is not complete
- `diagnostic`:
  - unsupported input is intentionally reported
  - unsupported input is not implemented
- `deferred`:
  - intentionally left for a later owner
  - current runtime surface is not ready

## Ledger

### Unknown Clientbound Packets In Login, Configuration, And Play

- Owner: `bbb-protocol` + `bbb-net` + `bbb-native`
- Status: `diagnostic`
- Next action:
  - When an unsupported packet appears in probe/control diagnostics, verify it
    against local vanilla 26.1 sources.
  - Then either:
    - implement protocol decode plus world/runtime handling
    - record why it is runtime-only
- Evidence / boundary:
  - Unknown login/config/play packets:
    - preserve `packet_id` and `len`
    - emit `NetEvent::UnsupportedPacket`
    - project into `NetCounters` / `ProbeReport`

### Protocol Coverage For Remaining Required 26.1 Packet Families

- Owner: `bbb-protocol`
- Status: `partial`
- Next action:
  - Continue auditing protocol details against
    `<MC_CODE_ROOT>/sources/26.1/`:
    - packet ids
    - field order
    - nullability
    - enum ordinals
    - serverbound encoders
  - Add focused encode/decode tests with each packet slice.
- Evidence / boundary:
  - Phase 2 in `docs/full-native-rewrite-plan.md` remains open until required
    paths are covered:
    - login
    - configuration
    - play
    - movement
    - inventory
    - chat
    - resource-pack
    - interaction
    - command suggestion

### Offline Probe And Online Dispatcher Parity

- Owner: `bbb-net` + `bbb-native` + `bbb-world`
- Status: `partial`
- Next action:
  - Keep adding parity regression tests for every decoded packet that has a
    `WorldStore` apply API.
  - Prefer shared semantics or focused paired tests when probe and online
    handling differ.
- Evidence / boundary:
  - Probe and online paths now cover many packet families, including:
    - unsupported diagnostics
    - play -> configuration teardown
    - custom payload brand/unknown presentation state
    - item cooldown add/remove state
    - delete/disguised chat state
    - entity motion/head rotation/animation/hurt transient state
    - scoreboard objective/display/score/team updates
    - advancement update/select-tab state
    - recipe book add/remove/settings updates
    - recipe access `UpdateRecipes` property-set and stonecutter updates
    - command tree and registry tag updates
    - HUD system chat, action-bar, title/subtitle, and title timing updates
    - world border initialize/center/size/warning updates
    - level chunk insert, block/section updates, chunk view, and chunk forgets
    - terrain light and biome updates
    - inventory, container, cursor, and merchant-offer updates
    - entity position, metadata, equipment, attributes, status, link,
      passenger, take-item, and remove updates
    - minecart along-track lerp updates
  - The final criterion:
    - every supported decoded packet stays aligned

### Native-Owned Business Snapshots

- Owner: `bbb-world` + `bbb-native` + `bbb-control`
- Status: `partial`
- Next action:
  - Continue moving client-observable state into `WorldStore`.
  - Keep `NetCounters` only for:
    - connection/runtime status
    - command queue projections
- Evidence / boundary:
  - The architecture plan still calls out removing native-only `last_*`
    snapshots when:
    - a world owner exists
    - a world owner should exist

### Code Of Conduct Presentation

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `deferred`
- Next action:
  - Replace the native bitmap prompt when the renderer UI stack is mature with:
    - fuller vanilla screen rendering
    - fuller vanilla font rendering
- Evidence / boundary:
  - Canonical Code of Conduct UI state and control requests are covered.
  - Presentation parity is explicitly deferred in
    `docs/full-native-rewrite-plan.md`.

### Crosshair Entity Interaction Parity

- Owner: `bbb-world` + `bbb-native` + `bbb-renderer`
- Status: `partial`
- Next action:
  - Validate any future `yRotA` source.
  - Replace the debug target overlay when renderer entity presentation exists
    with:
    - full entity model rendering
    - interaction feedback
- Evidence / boundary:
  - `bbb-world` and `bbb-native` expose many verified pick bounds and
    interaction packets.
  - Native projects the current crosshair entity pick target into a
    renderer-visible wire outline.
  - The outline uses the same pick AABB as raycast selection.

### Particle Runtime Vanilla Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Implement remaining renderer slices for:
    - provider-specific behavior
    - light curves
    - particle sorting
    - collision/player-coupled physics
    - particle limits/settings
    - atlas mip animation
    - terrain/item particle option rendering
    - remaining level-event particle effects beyond the currently covered
      simple smoke/white-smoke/flame/explosion/cloud/block-face/trial-spawner
      side effects
  - Preserve missing definition/sprite diagnostics.
- Evidence / boundary:
  - Current runtime:
    - Drains level-particle spawn batches.
    - Emits renderer spawn batches for simple vanilla `LevelEventHandler`
      side effects:
      - event `1501`: eight `minecraft:large_smoke` particles above lava
        extinguish
      - event `1502`: five `minecraft:smoke` particles inside redstone torch
        burnout
      - event `1503`: sixteen `minecraft:smoke` particles above end portal
        frame fill
      - event `2000`: ten directionally emitted `minecraft:smoke` particles
      - event `2010`: ten directionally emitted `minecraft:white_smoke`
        particles
      - event `2004`: twenty paired `minecraft:smoke` and `minecraft:flame`
        particles around the block center
      - event `2008`: one centered `minecraft:explosion` particle
      - event `2009`: eight `minecraft:cloud` particles above the block
      - event `3000`: one always-visible centered
        `minecraft:explosion_emitter` particle
      - event `3002`: vanilla axis or block-face
        `minecraft:electric_spark` particles
      - event `3003`: vanilla block-face `minecraft:wax_on` particles
      - event `3004`: vanilla block-face `minecraft:wax_off` particles
      - event `3005`: vanilla block-face `minecraft:scrape` particles
      - event `3009`: vanilla block-face `minecraft:egg_crack` particles
      - event `3011`: trial spawner smoke plus normal/ominous flame spawn
        particles
      - event `3012`: trial spawner mob-spawn sound-side smoke plus
        normal/ominous flame spawn particles
      - event `3013`: normal trial spawner detected-player particles
      - event `3014`: trial spawner eject-item sound-side
        `minecraft:small_flame` and `minecraft:smoke` particles
      - event `3017`: trial spawner eject-item `minecraft:small_flame` and
        `minecraft:smoke` particles
      - event `3018`: cobweb-place `minecraft:poof` particles
      - event `3019`: ominous trial spawner detected-player particles
      - event `3020`: ominous trial spawner activation detected-player,
        `minecraft:trial_omen`, and `minecraft:soul_fire_flame` particles
      - event `3021`: trial spawner item-spawn sound-side smoke plus
        normal/ominous flame spawn particles
    - Advances CPU-side common particles.
    - Samples vanilla-shaped curves for common particle providers:
      - size
      - color
      - age-size
    - Uploads a stitched official particle atlas when assets are available.
    - Draws active particles as camera-facing textured billboards.
  - Follow-up work in the plan:
    - full vanilla provider behavior
    - presentation parity

### Renderer Scene Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Replace proxies with full extraction from canonical world and pack data:
    - entity bounds
    - dropped-item icons
  - Extract complete entity presentation data:
    - precise vanilla meshes and textures beyond the source-verified
      player/player-slim/chicken normal/warm/cold adult and baby/
      pig temperate/warm/cold adult and baby/
      cow temperate/warm/cold adult and baby/
      boat/chest-boat/raft/chest-raft/
      sheep base/wool/undercoat layer geometry plus texture-backed layer
      passes with sheared state and dye color projection/
      wolf base/collar layer geometry plus texture-backed layer passes with
      tame flag and collar dye projection/
      base-horse/donkey/mule/
      skeleton-horse/zombie-horse/camel/camel-husk/llama/trader-llama/
      goat/polar-bear/hoglin/zoglin/ravager/villager/wandering-trader/zombie and husk/drowned/zombie-villager variants,
      piglin/piglin-brute/
      zombified-piglin variants, skeleton and stray/parched/wither-skeleton/
      bogged variants, creeper, spider, enderman, iron-golem, snow-golem,
      witch, and evoker/illusioner/pillager/vindicator illager body-layer
      geometry, armor
      stand normal/small body-layer geometry, slime/magma-cube body-layer
      geometry, cave-spider scaled body-layer geometry, primitive entity model
      families, and placeholder bounds
    - equipment
    - skin
    - animation
    - culling
    - ordering
  - Grow the renderer `EntityRenderState` projection as the single landing spot
    for vanilla `LivingEntityRenderState`/`EntityRenderState` per-frame fields,
    instead of adding ad hoc per-entity fields to `EntityModelInstance`:
    - now projected: `bodyRot` (body yaw, with the freezing shake folded in),
      `yRot`/`xRot` (net head-look yaw and head pitch), the sheep eat-grass head
      pose, the polar-bear standing-rear scale, `deathTime` (the death tip-over
      counter), `isAutoSpinAttack` (the riptide spin, carried as the lerped
      `auto_spin_age_ticks`), `isUpsideDown`/`boundingBoxHeight` (the Dinnerbone/Grumm
      flip, carried as `upside_down_height`), `hasPose(SLEEPING)` (the sleeping-in-bed
      pose, carried as `sleeping` with the resolved bed yaw and head offset), `scale`
      (the `SCALE`-attribute uniform model scale), `lightCoords` (block+sky packed
      light), `hasRedOverlay` (hurt/death red `OverlayTexture` flash), and
      `whiteOverlayProgress` (creeper swelling white flash)
    - `walkAnimationPos`/`walkAnimationSpeed` limb-swing: the client-side
      `WalkAnimationState` accumulator is implemented and tracked per living entity
      (see the dedicated bullet below), its lerped `position`/`speed` are projected
      through `EntityModelSourceState.walk_animation_position`/`_speed` to the
      renderer `EntityRenderState.walk_animation_pos`/`_speed`, the `QuadrupedModel`
      leg sway consumes them in the generic quadruped path and the dedicated
      `CowModel`, `PigModel`, `SheepModel`, `GoatModel`, and `PolarBearModel` paths (all
      variants, wool layers, baby layers, the goat's per-baby leg order and horn layer,
      the polar bear's standing rear composed on top of the swing), plus the custom
      `HoglinModel` (hoglin and zoglin, its own `1.2` amplitude no-frequency formula, plus
      the ear sway `±2π/9 ± speed * sin(pos)` for both adults and babies — the formula
      overrides the baby layer's wider rest angle to the same `±2π/9`), and the
      `HumanoidModel` leg sway
      consumes them in the zombie family
      (zombie, husk, drowned, zombie villager) and the skeleton family (skeleton,
      stray, parched, wither skeleton, bogged sheared/unsheared — body layer and the
      Stray/Bogged clothing overlay, since the overlay's layer `SkeletonModel` runs the
      same `setupAnim`) and the piglin family (piglin, piglin brute, zombified piglin,
      adult and baby). The illager family (evoker, vindicator, illusioner, pillager)
      consumes them too through a shared half-amplitude leg swing (a non-
      `HumanoidModel` swing with an extra `0.5` amplitude factor), as does the villager
      family (villager adult/baby, wandering trader, witch — `VillagerModel`/
      `WitchModel`, both also non-`HumanoidModel` with the same `0.5` factor), and the
      player model (`PlayerModel extends HumanoidModel`, remote players, colored and
      textured, wide and slim, the pants children riding the leg parts — and the inherited
      `HumanoidModel` **arm** counter-swing at `[2, 3]` with its sleeve children, since
      `PlayerModel` does not override the arms; the skeleton and non-zombified piglin
      families and the enderman share that inherited arm swing too), and the
      enderman (`EndermanModel extends HumanoidModel`, the inherited arm and leg swing
      halved and clamped to `[-0.4, 0.4]`, arms `[2, 3]`), and the iron golem
      (`IronGolemModel`, a triangle-wave
      gait swinging both legs and — its only walk-driven arm animation — the arms), and
      the ravager (`RavagerModel`, the `QuadrupedModel` diagonal phase at a shorter `0.4`
      amplitude, legs `[2, 3, 4, 5]`), the spider/cave spider (`SpiderModel`, the
      eight legs each sweeping about yRot and stepping about zRot, legs `[3..=10]`), and
      the wolf (`WolfModel`, the non-sitting `QuadrupedModel` diagonal leg swing, legs
      `[3, 4, 5, 6]` adult / `[2, 3, 4, 5]` baby, plus the non-angry tail wag at the last
      part — `tail.yRot = cos(pos * 0.6662) * 1.4 * speed`), the chicken (`ChickenModel`, the
      two-leg `HumanoidModel` phase, legs `[2, 3]` adult/cold / `[1, 2]` headless baby),
      and the llama/trader llama (`LlamaModel`, the `QuadrupedModel` diagonal phase, legs
      `[2, 3, 4, 5]` adult / `[4, 5, 6, 7]` with-chest / `[1, 2, 3, 4]` baby, colored and
      textured paths), and the equines (`AbstractEquineModel` horse/donkey/mule/skeleton-horse/
      zombie-horse, the front-`0.8`/hind-`0.5` gait, legs `[2, 3, 4, 5]` adult /
      `[1, 2, 3, 4]` baby horse, plus the neck head look/bob — yaw clamped to ±20°, pitch
      onto the π/6 tilt, and a `cos(pos * 0.8) * 0.15 * speed` walk bob — at `head_parts`
      `1` adult / `5` baby horse, and the tail walk lift `tail.xRot =
      getTailXRotOffset() + π/6 + speed * 0.75` with the `y += speed * ageScale` /
      `z += speed * 2 * ageScale` shift (body tail child, body subtree hand-emitted; the
      baby horse `getTailXRotOffset = −π/2` also overrides the layer rest angle and
      `ageScale = 0.5`), colored path; the baby donkey/mule nested legs and forced
      head pitch, the camel's sit/dash-entangled gait, and the tail's `ageInTicks` yRot wag
      stay deferred). The remaining
      slices consume them
      in the other model families' `setupAnim` (the camel; fish; other birds; etc., plus
      the `HumanoidModel`/illager/villager arm and ear/nose poses); the snow golem has no
      walk-driven swing (its `setupAnim` is the head-yaw twist/orbit, now implemented).
    - deferred slots to add with their own slices, each carrying real vanilla
      semantics and tests rather than tint fallbacks: `ageScale` (the baby `0.5`
      proportions applied in model `setupAnim`, distinct from the now-projected
      `SCALE`-attribute `scale`), unified `isInvisible`, and `outlineColor` glow
  - Entity packed-light shading is implemented end to end and no longer flat:
    `WorldStore::sample_block_light` samples the stored block+sky nibbles at the
    entity's floored light-probe block position (vanilla
    `EntityRenderer.getPackedLightCoords`), the native scene packs it into
    `EntityRenderState.lightCoords` with the on-fire block-light override, and
    the colored and textured entity shaders apply the same per-vertex lightmap as
    terrain (`max(block, sky * 0.95)` scaled into `0.16..=1.0`); the eyes pass
    stays emissive. Remaining lighting gaps: smooth/AO entity light, the colored
    block-light tint and gamma curve of the real vanilla `LightTexture`, and
    directional `Lighting.setupLevel` diffuse shading.
  - The hurt red damage overlay is implemented end to end as a real overlay pass,
    not a tint: `LivingEntity.hurtTime` is tracked client-side (set to
    `hurtDuration` = 10 by `apply_hurt_animation`/`apply_damage_event`, decremented
    each client tick), projected as `hasRedOverlay`, and the colored and textured
    entity shaders reproduce vanilla `OverlayTexture` per-vertex (the red row
    `y < 8` mixes toward red at alpha `179/255`, the white rows mix toward white at
    alpha `1 - u/15 * 0.75`) applied before the lightmap; the eyes pass is
    unaffected.
  - The creeper swelling white overlay is implemented end to end: the client
    tracks `Creeper.swell`/`oldSwell` from the synced `DATA_SWELL_DIR` (forced to
    `1` while `DATA_IS_IGNITED`), advancing it toward `maxSwell` = 30 each tick;
    `Creeper.getSwelling` is projected as `creeper_swelling`, the native scene maps
    it through `CreeperRenderer.getWhiteOverlayProgress`
    (`(int)(s*10) % 2 == 0 ? 0 : clamp(s, 0.5, 1.0)`) into the render-state
    `whiteOverlayProgress`, and the overlay coords' `u` column
    (`(int)(progress * 15)`) drives the shader white flash. The creeper is the only
    vanilla `getWhiteOverlayProgress` producer (the base renderer returns `0`):
    there is no freezing white overlay — `isFullyFrozen` drives the
    `setupRotations` body shake, not the overlay.
  - The death animation is implemented end to end, not a placeholder. World side:
    the client tracks vanilla `LivingEntity.deathTime` — when a living entity's
    synced health (`DATA_HEALTH_ID`, id `9`) reaches `<= 0`
    (`isDeadOrDying`) it starts the counter, each client tick increments it
    (`tickDeath`) capped at `20` (vanilla removes the entity there), and restoring
    health clears it; the gate is the existing `vanilla_living_entity_type`
    predicate so non-living entities never tip. It is projected as
    `EntityModelSourceState.death_time` (lerped `deathTime + partialTick`) and feeds
    both `hasRedOverlay` (`hurtTime > 0 || deathTime > 0`) and the render state.
    Renderer side: `EntityRenderState.death_time` rotates the model about the Z
    axis by `death_fall_factor(deathTime) * getFlipDegrees()`, where
    `death_fall_factor = min(sqrt(max((deathTime - 1) / 20 * 1.6, 0)), 1)` and
    `getFlipDegrees` is `90` for the base living renderer and `180` for the
    spider/cave spider (`SpiderRenderer`). The flip is inserted right after the
    `180 - bodyRot` yaw and before the `(-1, -1, 1)` flip in both shared living
    root transforms, so every colored and textured living model tips over, and is
    identity while alive.
  - The riptide auto-spin attack is implemented end to end. World side: a living
    entity (`vanilla_living_entity_type` gate) whose synced
    `DATA_LIVING_ENTITY_FLAGS` (id `8`) carries the `LIVING_ENTITY_FLAG_SPIN_ATTACK`
    bit (`4`) is marked `isAutoSpinAttack` in `EntityModelSourceState`; non-living
    entities have no living-entity-flags byte so they never spin. Native side
    projects `auto_spin_age_ticks = Some(ageInTicks + partialTick)` while spinning
    (`None` otherwise). Renderer side: the shared `entity_post_yaw_transform`
    reproduces the vanilla `setupRotations` else-if chain after the `180 - bodyRot`
    yaw — `deathTime > 0` tips over first (the death flip), `else if isAutoSpinAttack`
    applies `Axis.XP.rotationDegrees(-90 - xRot)` then
    `Axis.YP.rotationDegrees(ageInTicks * -75)`. Both branches are pure rotations
    about the post-yaw origin, so (like the death flip) they commute with the
    trailing uniform model scale and tip/spin every colored and textured living
    model; the death flip takes precedence over the spin, matching vanilla.
  - The Dinnerbone/Grumm upside-down easter egg is implemented end to end, for both
    the non-player and player render paths. World side: a non-player living entity
    (`vanilla_living_entity_type` gate) whose synced `DATA_CUSTOM_NAME` (id `2`) is
    `Dinnerbone`/`Grumm` (`LivingEntityRenderer.isUpsideDownName`) is marked
    `isUpsideDown` in `EntityModelSourceState`, which also carries the
    `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`, from the pick-bounds
    AABB). The player path (`AvatarRenderer.isEntityUpsideDown` →
    `isPlayerUpsideDown`) instead requires the cape model part to be shown
    (`DATA_PLAYER_MODE_CUSTOMISATION`, id `16`, `PlayerModelPart.CAPE` bit `0x01`) and
    the entity's `GameProfile` name — looked up from the player-info list by the
    entity UUID, not the custom name — to be `Dinnerbone`/`Grumm`. This spatial
    resolution lives in the `WorldStore` aggregation (`resolve_player_upside_down`,
    mirroring the packed-light / sleeping-bed lookups, since it needs the player-info
    list), and sets `isUpsideDown` on the player source. Native side negates the net
    head yaw and pitch while upside down (`extractRenderState`) and projects
    `upside_down_height = Some(boundingBoxHeight)`. Renderer side: the shared
    `entity_post_yaw_transform` adds the vanilla `setupRotations` else-if branch
    after death and the riptide spin — `translate(0, bbHeight + 0.1, 0)` then
    `Axis.ZP.rotationDegrees(180)`. The vanilla `(bbHeight + 0.1) / entityScale`
    divisor cancels the leading `scale(entityScale)`; our post-yaw frame is already
    in world units (the model scale is applied innermost), so the height is used as
    is, flipping every colored and textured living model (player included).
  - The sleeping-in-bed pose is implemented end to end. World side: a living entity
    (`vanilla_living_entity_type` gate) whose synced `Pose` (`DATA_POSE`, id `6`) is
    `SLEEPING` is marked `is_sleeping` in `EntityModelSourceState`. The bed
    orientation is resolved spatially by the `WorldStore` aggregation (which owns the
    block data, mirroring the packed-light sampling): `LivingEntity.getSleepingPos`
    (`SLEEPING_POS_ID`, id `14`) is looked up against the block world, and a bed
    block's `FACING` (`BedBlock.getBedOrientation`) yields the
    `sleepDirectionToRotation` yaw and the `submit` bed head-offset translate
    `[-stepX * (eyeHeight(STANDING) - 0.1), -stepZ * ...]` (the standing eye height
    is computed with the synced pose stripped so it does not collapse to
    `SLEEPING_DIMENSIONS`); a sleeping position that is not a bed leaves the yaw/offset
    at the no-bed fallback. Native side projects a `SleepingPose { yaw_angle, bed_offset }`
    (`yaw_angle` falling back to the body yaw when there is no bed). Renderer side:
    `entity_setup_rotations_transform` skips the `180 - bodyRot` yaw while sleeping
    and the shared `entity_post_yaw_transform` adds the else-if branch (after death and
    the riptide spin, before the upside-down flip) `Ry(yaw_angle) * Rz(getFlipDegrees)
    * Ry(270)`, with the bed offset applied as a pre-scale world-space translate, so
    every colored and textured living model lies down in its bed.
  - The `LivingEntityRenderState.scale` uniform model scale is implemented end to end.
    World side: a living entity's `LivingEntity.getScale` (the `SCALE` attribute id
    `25`, clamped to `[0.0625, 16.0]` and passed through the per-entity `sanitizeScale`
    overrides — `HappyGhast` ≤ 1.0, `Shulker` ≤ 3.0 via `entity_scale`) is projected as
    `EntityModelSourceState.scale` (`1.0` for default-size and non-living entities).
    Native side projects it to `EntityRenderState.scale`. Renderer side applies it as
    `poseStack.scale(scale, scale, scale)` before `setupRotations` (between the bed
    pre-scale translate and the rotation stage), matching vanilla. The death/spin/
    sleeping branches are pure rotations about the post-scale origin so they are
    unaffected; the upside-down branch divides its lift by the scale
    (`(bbHeight + 0.1) / entityScale`) so the world-space lift stays `bbHeight + 0.1`.
    The baby `ageScale` (the `0.5` head/body proportions applied in model `setupAnim`)
    is a separate value and stays deferred.
  - The `WalkAnimationState` limb-swing accumulator is implemented and tracked client
    side, projected end to end, and consumed by the `QuadrupedModel` legs. World side:
    each client tick `EntityStore::advance_client_animations` runs vanilla
    `LivingEntity.calculateEntityAnimation` for every living entity — it measures the
    per-tick feet travel (`Mth.length` of the position delta, including the vertical
    component only for `FlyingAnimal` Bee/Parrot, like
    `calculateEntityAnimation(this instanceof FlyingAnimal)`), then feeds the base
    `updateWalkAnimation` mapping (`targetSpeed = min(distance * 4, 1)`, `factor =
    0.4`, `positionScale = isBaby ? 3 : 1`) into the per-entity `WalkAnimationState`
    (`speedOld/speed/position/positionScale`). A passenger or a dead entity
    (`!isAlive`, mirrored by the client death-animation state) stops the swing
    (`walkAnimation.stop()`), matching vanilla. The lerped `position(partialTick)` /
    `speed(partialTick)` are projected onto
    `EntityModelSourceState.walk_animation_position` / `walk_animation_speed`, and the
    native projection carries them to `EntityRenderState.walk_animation_pos` /
    `walk_animation_speed`. Renderer side: the shared `quadruped_leg_swing_pose`
    applies the vanilla `QuadrupedModel.setupAnim` leg sway — each leg's `xRot =
    cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`, with the
    hind-left/front-right pair a half-cycle out of phase with the hind-right/front-left
    pair (resolved from each leg part's `x * z < 0` offset, so the differing leg
    order of the adult and baby layers does not matter). It is consumed by the generic
    `emit_quadruped_model` path and the dedicated `CowModel`, `PigModel`, `SheepModel`,
    `GoatModel`, and `PolarBearModel` paths (both the colored and textured renders,
    every variant, the wool layers, and the baby layers; `PigModel`/`SheepModel`/
    `GoatModel`/`PolarBearModel` extend `QuadrupedModel`, with `SheepModel.setupAnim`
    running `super.setupAnim` — the leg swing — before its eat-grass head pose,
    `GoatModel.setupAnim` running it before its horn visibility and ramming head tilt
    (legs at `[2, 3, 4, 5]` adult / `[0, 1, 2, 3]` baby), and `PolarBearModel.setupAnim`
    running it before the standing rear, which then adds `frontLeg.xRot -= standScale *
    π * 0.45` on top of the swing — applied in that order so the rear composes with the
    swing), so a walking quadruped's legs swing (`0.0` for a standing entity, every
    non-living entity, and the deferred overrides below). The hoglin family (hoglin and
    zoglin, adult and baby — `emit_hoglin_model` colored and `emit_hoglin_textured_model`
    textured) uses a dedicated `hoglin_leg_swing_pose`: `HoglinModel` is a custom
    `EntityModel` (not a `QuadrupedModel`) whose four legs swing `cos(pos [+ π]) * 1.2 *
    speed` — amplitude `1.2`, no `0.6662` factor, and the right-front/left-hind pair in
    phase (resolved from `x * z > 0`, the opposite sign of the `QuadrupedModel` rule),
    legs at `[2, 3, 4, 5]`. It also sways the ears (children of the head)
    `ear.zRot = ±2π/9 ± speed * sin(pos)` (`hoglin_ear_sway_pose`, right ear `−`, left
    `+`, ears at head children `[0, 1]`) in both render paths; because the ears' children
    list is static, the head subtree is hand-emitted with the swayed ears (the horns ride
    unchanged). Adult and baby hoglin/zoglin ears both sway: vanilla `HoglinModel.setupAnim`
    writes the literal `±2π/9` rest angle, overriding the wider angle baked into
    `BabyHoglinModel`'s layer, so `hoglin_ear_sway_pose` sets (not accumulates) the absolute
    angle and baby ears are always re-posed. The headbutt head tilt is deferred. The creeper
    (`emit_creeper_model` colored and `emit_creeper_textured_model` textured) is a custom
    `EntityModel` too, but its `setupAnim` leg swing is exactly the `QuadrupedModel`
    formula (legs at `[2, 3, 4, 5]`), so it reuses the shared quadruped swing; its
    swelling scale and powered charge layer are deferred. The
    `HumanoidModel` leg swing (`humanoid_leg_swing_pose`: the right leg, part offset
    `x < 0`, in phase and the left leg out of phase, since both legs sit at `z = 0`) is
    consumed by the zombie family (`emit_zombie_model`/`emit_zombie_variant_model` —
    zombie, husk, drowned, zombie villager, adult and baby) and the skeleton family
    (`emit_skeleton_model`/`emit_skeleton_variant_model` colored and
    `emit_skeleton_textured_model` → `emit_humanoid_textured_passes` — skeleton, stray,
    parched, wither skeleton, bogged sheared/unsheared; the Stray/Bogged clothing
    overlay swings too, since its layer `SkeletonModel` runs the same `setupAnim`).
    Both families inherit the `HumanoidModel` legs unchanged (`SkeletonModel`/
    `AbstractZombieModel extends HumanoidModel`); the zombie family then overrides the
    arms with its constant held-out pose (deferred), while the skeleton family keeps the
    inherited arm counter-swing in its default (non-aiming) state (implemented — see the
    arm-swing note below). The piglin
    family (`emit_piglin_model` — piglin, piglin brute, zombified piglin, adult and
    baby) also consumes it: `AbstractPiglinModel extends HumanoidModel`, whose
    `setupAnim` runs `super.setupAnim` (the inherited legs and arms) before swaying only
    the ears. The adult/baby piglin and the brute keep the inherited arm counter-swing in
    their default state (`PiglinModel` overrides the arms only in its deferred dance/
    attack/crossbow/admire poses), so the arm swing is implemented for them too; the
    zombified piglin instead overwrites the arms with `AnimationUtils.animateZombieArms`
    (the deferred held-out zombie pose), so only its legs swing. The illager family
    (`emit_illager_model`
    — evoker, vindicator, illusioner, pillager) uses a dedicated `illager_leg_swing_pose`
    (`cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5`): `IllagerModel` is not a
    `HumanoidModel` and adds an extra `0.5` amplitude factor (the shared
    `half_amplitude_leg_swing_pose`), and its body layers list the legs at `[3, 4]` for
    the crossed-arms layouts (evoker/vindicator/illusioner) and `[2, 3]` for the
    uncrossed pillager, resolved per family. The pillager also swings its *separate* arms
    with the exact `HumanoidModel` amplitude (`cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`,
    [`humanoid_arm_swing_pose`], arms at `[4, 5]`); the evoker/vindicator/illusioner show
    the static crossed `arms` part, which vanilla never animates (it swings the *invisible*
    separate arms), so their visible arms stay put. The villager family
    (`emit_villager_model`/`emit_wandering_trader_model`/`emit_witch_model` colored and
    `emit_villager_family_textured_passes` textured for the villager/wandering-trader, plus
    the witch's own `emit_witch_model`/`emit_witch_textured_model` that add the idle nose bob)
    shares the same `half_amplitude_leg_swing_pose`: `VillagerModel` and
    `WitchModel` are also `EntityModel` (not `HumanoidModel`) with the identical `* 0.5`
    formula and no riding branch, with legs at `[3, 4]` (adult villager/witch/trader)
    or `[1, 2]` (baby villager). The player model
    (`emit_player_model` colored and `emit_player_textured_model` — remote players, wide
    and slim) consumes the `HumanoidModel` legs unchanged (`PlayerModel.setupAnim` only
    toggles part visibility before `super.setupAnim`; the pants children ride the leg
    parts and the visibility-filtered part array keeps the legs at `[4, 5]`). The
    enderman (`emit_enderman_model` colored and `emit_enderman_textured_model` textured)
    uses dedicated `enderman_arm_swing_pose`/`enderman_leg_swing_pose`: `EndermanModel
    extends HumanoidModel`, so `super.setupAnim` sets the inherited arm and leg swing,
    then the enderman halves both (`*= 0.5`) and clamps them to `[-0.4, 0.4]` (arms at
    `[2, 3]`, legs at `[4, 5]`); the arm swing reuses the base
    [`humanoid_arm_swing_pose`] (the right arm — part offset `x < 0` — the half-cycle out
    of phase, counter to the same-side leg) before the halve/clamp. Its carried-block arm
    pose (`xRot = -0.5`, `zRot = ±0.05`) and creepy attack head/hat shift are deferred.
    The iron golem
    (`emit_iron_golem_model` colored and `emit_iron_golem_textured_model` textured) uses
    `iron_golem_walk_pose`: `IronGolemModel` is a custom `EntityModel` whose
    `setupAnim` swings both the legs (`±1.5 * Mth.triangleWave(pos, 13) * speed`) and —
    in its default non-attack/non-flower branch — the arms (`(-0.2 ± 1.5 *
    triangleWave(pos, 13)) * speed`), a triangle-wave gait rather than a cosine one; the
    arms sit at part offset `x = 0`, so the right/left role is fixed by slot (arms
    `[2, 3]`, legs `[4, 5]`). This is the first model whose **arm** swing is a pure
    walk-driven animation (so it is implemented); the attack swing and the offer-flower
    arm pose are deferred event animations. The ravager (`emit_ravager_model` colored and
    `emit_ravager_textured_model` textured) uses a dedicated `ravager_leg_swing_pose`:
    `RavagerModel` is a custom `EntityModel` whose `setupAnim` swings the four legs with
    the `QuadrupedModel` diagonal phase (`cos(pos * 0.6662 [+ π])`, in phase when
    `x*z < 0`) but a shorter `0.4` amplitude (`legRot = 0.4 * speed`) rather than the
    usual `1.4`; legs sit at `[2, 3, 4, 5]` and the swing only sets `xRot`, leaving the
    nested neck/head subtree (which the head-look pose drives) untouched. Its mouth-open
    attack pose, the stunned-shake `xRot`, and the roar/biting head animations are
    deferred event animations. The spider and cave spider (`emit_spider_model` /
    `emit_cave_spider_model` colored and `emit_spider_textured_model` textured, both base
    and eyes passes) use a dedicated `spider_leg_swing_pose`: `SpiderModel` is a custom
    `EntityModel` whose `setupAnim` accumulates onto each of the eight legs a horizontal
    sweep `yRot += -(cos(animationPos*2 + phase) * 0.4) * speed` and a vertical step
    `zRot += |sin(animationPos + phase) * 0.4| * speed` (with `animationPos =
    walkAnimationPos * 0.6662`); the right legs add both terms and the left legs subtract
    them, with the per-leg-pair `phase` `0`/`π`/`π/2`/`3π/2` for hind/middle-hind/
    middle-front/front (`spider_leg_swing_roles` maps body-layer indices `3..=10`). The
    cave spider shares the model, differing only by its smaller root transform. The swing
    accumulates onto the legs' resting splay and leaves `xRot` untouched; spiders have no
    other walk-driven animation. The wolf (`emit_wolf_model` colored and
    `emit_wolf_textured_model` textured, adult and baby, every pass) reuses
    `quadruped_leg_swing_pose`: `WolfModel` is a custom `EntityModel` whose `setupAnim`,
    in its non-sitting branch, swings the four legs with the exact `QuadrupedModel`
    diagonal phase (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, hind-right/front-left in
    phase, resolved from `x * z < 0`); `wolf_leg_part_indices` lists the legs at
    `[3, 4, 5, 6]` for the adult (head/body/mane at `0`/`1`/`2`) and `[2, 3, 4, 5]` for
    the baby (no mane). The tail is the last part (index `7` adult, `6` baby —
    `wolf_tail_part_index`). A non-angry wolf wags it with `wolf_tail_swing_pose`
    (`tail.yRot = cos(pos * 0.6662) * 1.4 * speed`, the same `QuadrupedModel` amplitude as
    the legs with no phase offset) and keeps the layer's `π/5` rest droop — exactly the
    `getTailAngle()` an untamed wolf returns. An angry wolf instead holds the tail straight
    and raised (`wolf_angry_tail_pose`: `tail.yRot = 0`, `tail.xRot = getTailAngle() =
    1.5393804`, overriding the rest droop even when standing), driven by the `isAngry`
    render state, in both the colored and textured paths. `isSitting` is a deferred AI state,
    so a standing wolf always takes the leg-swing branch. The tame `tail.xRot = tailAngle`
    health droop (it needs the wolf's health), the `shakeOffWater` body roll, and the sitting
    pose are deferred. Deferred:
    (1) the
    `Camel`/`Creaking`/`Frog` `updateWalkAnimation` overrides use different
    distance→speed mappings (and
    `Camel`/`Frog` gate on pose/jump/dash animation states the client does not yet
    track), so their limb swing is left at rest rather than approximated; (2) the base
    `HumanoidModel.setupAnim` arm swing is implemented for the player, the skeleton
    family, and the non-zombified piglin family
    (`humanoid_arm_swing_pose`/`humanoid_arm_swing_parts`, arms at `[2, 3]`, the
    counter-swing `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`; `SkeletonModel` and
    `(Abstract)PiglinModel` run `super.setupAnim` and override the arms only in their
    deferred pose branches, so the default arms swing — for the skeleton in both the
    colored and textured paths, every variant (skeleton, stray, parched, wither skeleton,
    bogged sheared/unsheared), and for the colored adult/baby piglin and brute); the
    per-subclass arm/ear/nose poses that override it stay deferred (the zombie held-out
    arms, the skeleton melee swing (`isAggressive && !isHoldingBow`) and bow-aiming
    `ArmPose`, the zombified piglin `AnimationUtils.animateZombieArms` held-out pose, the
    `PiglinModel` dance/attack/crossbow/admire poses (the `AbstractPiglinModel` ear flap is
    implemented for every piglin/zombified-piglin subclass — see below), the `IllagerModel`
    attack/spellcast/bow/crossbow/celebrate arm-pose overrides and riding sit pose (the
    default walk arm swing is implemented for the pillager), the `VillagerModel` unhappy
    head shake and the `WitchModel` `isHoldingItem` nose hold pose (the idle nose bob is
    implemented — see below), the `GoatModel` ramming head
    tilt, the `HoglinModel` headbutt head tilt, the `EndermanModel`
    carried-block arm pose and creepy attack pose, the `IronGolemModel`
    attack swing and offer-flower arm pose,
    item/attack/crouch/swim/elytra poses, and the always-on arm bob, and the
    player crouch/swim/elytra `speedValue` poses) are separate animations driven by
    states the client does not yet track;
    (3) consuming the projected values in the remaining model families' `setupAnim`
    (fish, other birds, etc.) are the next slices, plus the chicken wing flap (untracked
    `flap`/`flapSpeed`) and the several deferred event/tail poses noted above. (The snow
    golem has no walk-driven swing; its `setupAnim` head-yaw upper-body twist and arm orbit
    are implemented.) The `EntityRenderState.ageInTicks` (= entity `tickCount + partialTick`)
    is now projected for every entity (`with_age_in_ticks`, from the world's per-entity
    client-animation age), driving the continuous `AbstractPiglinModel.setupAnim` ear flap
    (`piglin_ear_flap_pose`, shared by every piglin/zombified-piglin subclass via
    `super.setupAnim`): `freq = ageInTicks * 0.1 + pos * 0.5`, `amp = 0.08 + speed * 0.4`,
    `leftEar.zRot = -default - cos(freq * 1.2) * amp`, `rightEar.zRot = default +
    cos(freq) * amp`, with `default` the `getDefaultEarAngleInDegrees()` of `30°` (adult/
    brute) or `5°` (baby). The ears are `&'static` head children, so the head subtree is
    hand-emitted with the flapped ears (colored path; piglins have no textured path); because
    the `±0.08` baseline and `ageInTicks` advance every frame, the ears never sit still.
    The same `ageInTicks` projection drives the continuous `WitchModel.setupAnim` idle nose
    bob (`witch_nose_bob_pose`): `speed = 0.01 * (entityId % 10)`, `nose.xRot =
    sin(ageInTicks * speed) * 4.5°`, `nose.zRot = cos(ageInTicks * speed) * 2.5°`, both SET
    absolutely on top of the head look and the half-amplitude leg swing. The nose is a
    `&'static` head child (and carries the mole as its own child), so the witch's head subtree
    is hand-emitted with the bobbed nose in both the colored and textured paths; because
    `cos` never reaches a zRot of `0`, the nose is always re-posed (there is no static fast
    path). Only the `isHoldingItem` nose hold pose (`setPos(0, 1, -1.5)`, `xRot = -0.9`) stays
    deferred, since it needs the witch's held-potion render state.
  - The `LivingEntityRenderer.setupRotations` body shake is implemented end to end.
    World side: a living entity (`vanilla_living_entity_type` gate) whose synced
    `ticksFrozen` (`DATA_TICKS_FROZEN`, id `7`) reaches `getTicksRequiredToFreeze()`
    = `140` is marked `isFullyFrozen` in `EntityModelSourceState`. Native side:
    `entity_shaking` reproduces vanilla `isShaking` — the base `isFullyFrozen` plus
    the per-renderer conversion overrides that are synced to the client:
    `AbstractZombieRenderer` ORs in `Zombie.isUnderWaterConverting()`
    (`DATA_DROWNED_CONVERSION_ID`, id `18`) for the whole zombie family, and
    `ZombieVillagerRenderer` additionally ORs in `ZombieVillager.isConverting()`
    (`DATA_CONVERTING_ID`, id `19`). While shaking, the scene folds
    `cos(floor(ageInTicks) * 3.25) * π * 0.4` (degrees) into the projected
    `body_rot`, computed against the integer `ageInTicks` (= `Mth.floor`, so no
    partial lerp); the net head-look yaw is taken against the unshaken body yaw, so
    the whole model jitters while the head turn relative to the body is unchanged.
    Remaining gap: the conversion shakes that are not a synced client flag — the
    hoglin/piglin zombification shake (environment-attribute derived, server-side)
    and the base-`Skeleton` freeze-conversion shake (server-side `conversionTime`),
    plus `StriderRenderer` cold (the strider model itself is still a placeholder).
  - The head-look projection is implemented as a reusable render-state field: the
    canonical `Entity.yHeadRot`/`getXRot` flow through `EntityModelSourceState`,
    the native scene derives `LivingEntityRenderState.yRot` =
    `Mth.wrapDegrees(yHeadRot - bodyRot)` (net head yaw) and `xRot` (head pitch),
    and `EntityRenderState.head_yaw`/`head_pitch` carry them in degrees. The
    shared `head_look_pose` applies the look that both `QuadrupedModel.setupAnim`
    and `HumanoidModel.setupAnim` set (`head.xRot = xRot * π/180`, `head.yRot =
    yRot * π/180`). Consumers so far: the `QuadrupedModel` family — pig and cow in
    both colored and textured paths, and the sheep, which additionally overrides
    `head.xRot = headEatAngleScale` (its non-eating branch is exactly the look
    pitch `getXRot * π/180`, `Sheep.getHeadEatAngleScale`) composing with the
    eat-grass dip; the `HumanoidModel` zombie family (zombie, husk, drowned, zombie
    villager — all colored+textured); and the skeleton family (skeleton,
    stray, parched, wither skeleton, bogged) in both the colored and textured
    paths; the piglin family (piglin, piglin brute, zombified piglin) in both the
    colored and textured paths; and the `VillagerModel`/`IllagerModel`/`WitchModel` family
    (villager and wandering trader colored+textured, witch colored+textured,
    illagers colored+textured) — the baby villager's index-3 head included. Remaining
    head-look work: now applied to every part-list humanoid and quadruped,
    including the wide/slim player model, and the standalone head-first models
    (creeper, spider/cave spider, enderman, iron golem, snow golem) — all colored
    and textured; and the goat (colored+textured, horn children rotating with the
    head) and wolf (colored+textured) `QuadrupedModel`/`EntityModel` head parts.
    The animation-/pose-driven head models are also covered: the polar bear
    (colored+textured) applies the `QuadrupedModel` look before the standing-rear
    delta, matching vanilla's `super.setupAnim` then `head.xRot += standScale * π *
    0.15` order; the hoglin/zoglin (colored+textured) apply the vanilla yaw-only
    look (`head.yRot = yRot * π/180`) while keeping `head.xRot` at the fixed
    headbutt-rest tilt `HOGLIN_HEAD_X_ROT`; and the ravager (colored+textured),
    whose head is `neck.getChild("head")` — the neck subtree is emitted by hand so
    the nested head carries the look while its horn/mouth children inherit it. The
    remaining head-look gaps are the fall-flying/swimming `head.xRot` overrides
    (currently untracked, default upright) and the placeholder raw-cuboid
    `Humanoid` fallback path (not yet a vanilla-faithful part-list model).
  - Keep covered sheep behavior derived from canonical renderer inputs:
    - custom-name `jeb_` color cycling from entity metadata, per-entity client
      age ticks, and renderer partial tick
    - vanilla shared-flags invisibility gating for the non-glowing wool and
      undercoat layer passes
    - eat-grass head animation from entity event `10`, projected from the
      canonical `Sheep.eatAnimationTick` countdown and renderer partial tick
      into the base, wool, and undercoat head part pose
  - Finish remaining sheep presentation parity:
    - extend the texture-backed sheep path with the white `OverlayTexture`
      progress (packed lighting and the hurt red overlay are now applied to every
      textured entity pass)
    - implement invisible glowing outline wool rendering
    - implement base-model invisibility/outline handling
  - Finish wolf presentation parity:
    - project registry-driven wolf variants beyond the default/pale texture set
    - add armor, wet tint, sitting/head/tail/shake/walk pose, base-model
      invisibility/outline handling, the white overlay, and remaining
      render-state extraction parity (packed lighting and the hurt red overlay
      are now applied)
  - Implement vanilla dropped-item follow-up rendering:
    - ground-context model rendering
    - bobbing
    - Y spin
    - count-based multiple copies
    - lighting
  - Continue renderer presentation work with deterministic tests or explicit
    manual comparison notes:
    - HUD
    - overlays
    - screenshots
    - interaction feedback
- Evidence / boundary:
  - Renderer draws:
    - terrain
    - HUD
    - particles
    - selection/block-destroy overlays
    - crosshair entity target outlines
    - a basic hecs-derived entity bounds scene proxy
    - dropped item entities as camera-facing item-icon billboards from:
      - canonical item entity stack metadata
      - the native item atlas
    - player and mannequin entities as renderer-owned vanilla 26.1
      `PlayerModel.createMesh(CubeDeformation.NONE, slim)` body-layer geometry
      from `PlayerModel`, `AvatarRenderer`, and `LayerDefinitions`, including
      base head/body/arms/legs plus hat/jacket/sleeves/pants children, the
      `AvatarRenderer` `0.9375F` render scale, texture-backed base layer pass
      emission, `ModelLayers.PLAYER` / `PLAYER_SLIM` selection, official
      wide/slim Steve PNG atlas upload/bind/sample path, and the vanilla
      `HumanoidModel.setupAnim` head-look yaw/pitch on the head part plus the
      inherited `HumanoidModel.setupAnim` walk swing — the legs at `[4, 5]`
      (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, pants children riding them) and the
      arms at `[2, 3]` (the counter-swing `cos(pos * 0.6662 [+ π]) * 2.0 * speed *
      0.5`, sleeve children riding them) — all applied once to the shared
      visibility-filtered part array (colored and textured); true
      `RenderTypes.entityTranslucent` alpha blending, UUID/default-skin
      selection, live skin downloads, automatic slim-vs-wide model selection
      from `PlayerSkin`, capes, ears, armor/equipment, held items,
      elytra/wings, shoulder parrots,
      arrows/stingers, spectator visibility, crouch/flying offsets, name
      display, the held-item/attack/crouch/swim arm poses, the `ageInTicks` idle
      arm bob, and the elytra `speedValue` poses remain unsupported
      (metadata-driven `DATA_PLAYER_MODE_CUSTOMISATION` projection now controls
      hat/jacket/sleeves/pants overlay visibility for the texture-backed base
      player/mannequin model, and the cape bit is preserved in renderer
      visibility state for the deferred cape layer)
    - wooden boat, chest boat, bamboo raft, and bamboo chest raft entities as
      renderer-owned vanilla 26.1 `BoatModel` / `RaftModel` body-layer
      geometry from `BoatModel`, `RaftModel`, `BoatRenderer`, `RaftRenderer`,
      `AbstractBoatRenderer`, and `LayerDefinitions`, including boat hull
      parts, raft bottom logs, paddles, chest bottom/lid/lock parts, official
      per-wood/per-bamboo texture references, and the vanilla boat root
      translate/rotate/scale/rotate renderer transform, texture-backed base
      layer pass emission, boat/chest-boat/raft/chest-raft model-layer
      selection, and official PNG atlas upload/bind/sample path; paddle rowing
      animation, hurt/damage roll, bubble wobble, underwater state and water
      mask submission, and lighting remain unsupported
    - chicken entities as renderer-owned vanilla 26.1
      `AdultChickenModel`, `ColdChickenModel`, and `BabyChickenModel` body-layer
      geometry from `ChickenModel`, `ChickenRenderer`, `ChickenVariants`, and
      `LayerDefinitions`, including metadata-driven temperate/warm/cold
      variant projection through the server-sent `minecraft:chicken_variant`
      registry order, official adult/baby variant texture references, and
      vanilla fallback to temperate when no variant metadata is present,
      texture-backed base layer pass emission, adult/baby/cold model-layer
      selection, and official PNG atlas upload/bind/sample path, and the vanilla
      `ChickenModel.setupAnim` two-leg walk swing (the `HumanoidModel` phase
      `cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at `[2, 3]` adult/cold and
      `[1, 2]` on the headless baby layer, on both render paths and every variant
      pass); the chicken has no head look in vanilla (`ChickenModel` never animates
      the head). The wing flap animation (driven by the untracked client-side
      `flap`/`flapSpeed` state), variant sound metadata, custom/datapack chicken
      variant asset decoding, and lighting remain unsupported
    - pig entities as renderer-owned vanilla 26.1
      `PigModel`, `ColdPigModel`, and `BabyPigModel` body-layer geometry from
      `PigModel`, `ColdPigModel`, `BabyPigModel`, `PigRenderer`,
      `PigVariants`, and `LayerDefinitions`, including normal/warm adult base
      model reuse, cold adult body overlay geometry, baked baby
      `CubeDeformation` bounds, metadata-driven temperate/warm/cold variant
      projection through the server-sent `minecraft:pig_variant` registry
      order, official adult/baby variant texture references, vanilla fallback
      to temperate when no variant metadata is present, texture-backed base
      layer pass emission, adult/baby/cold model-layer selection, and official
      PNG atlas upload/bind/sample path, and the vanilla
      `QuadrupedModel.setupAnim` head-look yaw/pitch on the head part; saddle
      equipment layer, boost/ridden/leg animation, variant sound metadata,
      custom/datapack pig variant asset decoding, and lighting remain unsupported
    - cow entities as renderer-owned vanilla 26.1 `CowModel`, `WarmCowModel`,
      `ColdCowModel`, and `BabyCowModel` body-layer geometry from `CowModel`,
      `WarmCowModel`, `ColdCowModel`, `BabyCowModel`, `CowRenderer`,
      `CowVariants`, and `LayerDefinitions`, including warm adult horn
      geometry, cold adult body overlay and nested horn geometry,
      metadata-driven temperate/warm/cold variant projection through the
      server-sent `minecraft:cow_variant` registry order, official adult/baby
      variant texture references, and vanilla fallback to temperate when no
      variant metadata is present, texture-backed base layer pass emission,
      adult/baby/warm/cold model-layer selection, official PNG atlas
      upload/bind/sample path, and the vanilla `QuadrupedModel.setupAnim`
      head-look yaw/pitch on the head part; variant sound metadata,
      custom/datapack cow variant asset decoding, walk animation, and lighting
      remain unsupported
    - sheep entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `SheepModel`, `BabySheepModel`, and `SheepRenderer`, with
      official base/wool/undercoat texture references, texture-backed base,
      wool, and undercoat layer passes, metadata-driven sheared state, and dye
      color projection, custom-name `jeb_` color cycling from entity metadata
      and renderer age ticks, vanilla shared-flags invisibility gating for
      non-glowing wool and undercoat layer passes, and the vanilla
      `SheepModel`/`SheepFurModel.setupAnim` eat-grass head pose (`head.y +=
      headEatPositionScale * 9.0 * ageScale`, `head.xRot = headEatAngleScale`)
      projected from entity event `10` and the canonical `eatAnimationTick`
      countdown into the base, wool, and undercoat head part; invisible glowing
      outline wool rendering, base-model invisibility/outline handling, the
      non-eating head-look pitch fallback, lighting, overlay, and remaining
      render-state extraction remain unsupported
    - wolf entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `AdultWolfModel`, `BabyWolfModel`, and `WolfRenderer`,
      including nested real-head and tail parts plus baked baby
      `CubeDeformation` bounds, official default/pale wild/tame/angry adult
      and baby texture references, adult/baby collar texture references,
      texture-backed base and collar layer passes, vanilla tame-over-angry
      texture selection, metadata-driven tame flag projection, collar dye
      tint projection, anger end-time projection against canonical client game
      time, vanilla shared-flags invisibility gating for the collar layer, and the
      vanilla `WolfModel.setupAnim` head-look yaw/pitch on the head part (colored
      and textured, with the head/ear children rotating with the head) and the
      vanilla `WolfModel.setupAnim` non-sitting leg walk swing (the `QuadrupedModel`
      diagonal phase `cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at `[3, 4, 5, 6]`
      adult / `[2, 3, 4, 5]` baby, on both render paths and every pass), the non-angry
      tail wag (`tail.yRot = cos(pos * 0.6662) * 1.4 * speed` on the last part, tail at
      `7` adult / `6` baby, both render paths), and the full `tail.xRot = getTailAngle()`
      droop (`WolfModel.setupAnim` unconditionally sets the tail `xRot` to the projected
      `wolf_tail_angle = Wolf.getTailAngle()`: the angry constant `1.5393804`, the tame
      health droop `(0.55 - (40 - health) / 40 * 0.4) * π` from the synced health, or the
      `π/5` untamed default — which also overrides the baby layer's `−π/6` tail rest pose,
      so the baby tail rests like the adult; both render paths, driven from the synced
      tame flag and `DATA_HEALTH_ID`), and the `WolfModel.setSittingPose` sitting fold
      (driven by the synced `TamableAnimal` `DATA_FLAGS_ID` sitting bit: the body tilts
      `xRot = π/4` and lifts, the hind legs tuck `xRot = 3π/2`, the front legs splay
      `xRot = 5.811947`, the tail lifts, all translation terms scaled by `ageScale`
      1.0 adult / 0.5 baby — the baby tilts the body a further `−π/2` — replacing the leg
      swing while the head still follows the look; both render paths); registry-driven wolf
      variants beyond the default/pale texture set, armor layer, wet tint,
      head-shake/begging tilt pose, the water-shake body roll, base-model
      invisibility/outline handling, lighting, overlay, and remaining render-state
      extraction remain unsupported
    - base horse entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `AbstractEquineModel.createBodyMesh(CubeDeformation.NONE)`,
      `BabyHorseModel.createBabyMesh(CubeDeformation.NONE)`, `HorseModel`, and
      `HorseRenderer`, with the adult `ModelLayers.HORSE`
      `MeshTransformer.scaling(1.1F)` root transform and default
      `Variant.WHITE` adult/baby texture references recorded from official
      assets, and the vanilla `AbstractEquineModel.setupAnim` walking leg swing
      (the equine gait `cos(pos * 0.6662 + π) * speed` at front amplitude `0.8` /
      hind `0.5`, legs at `[2, 3, 4, 5]` adult / `[1, 2, 3, 4]` on the re-parented
      baby layer), the default-branch neck head look/bob (`head_parts.yRot =
      clamp(yRot, -20, 20) * π/180`, `head_parts.xRot = π/6 + xRot * π/180 +
      (speed > 0.2 ? cos(pos * 0.8) * 0.15 * speed : 0)`, at `head_parts` `1` adult /
      `5` baby horse), and the tail walk lift (`tail.xRot = getTailXRotOffset() + π/6 +
      speed * 0.75`, `tail.y += speed * ageScale`, `tail.z += speed * 2 * ageScale`; the
      baby horse `getTailXRotOffset = −π/2` overrides the layer rest angle and `ageScale =
      0.5`, the body subtree hand-emitted so the tail child can swing, colored render path);
      horse variant textures, markings, armor, saddle, the ridden/eat/stand/mouth poses, the
      tail's `ageInTicks` yRot wag, the in-water leg-frequency scaling, and non-equine
      horse-fallback model parity remain unsupported
    - donkey and mule entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `DonkeyModel`, `BabyDonkeyModel`, and
      `DonkeyRenderer`, including adult `DONKEY_SCALE=0.87F` /
      `MULE_SCALE=0.92F` root scaling, metadata-driven adult chest visibility,
      the empty baby chest children from `BabyDonkeyModel.createBabyLayer()`,
      and official adult/baby donkey/mule texture references recorded from
      assets, and the adult `AbstractEquineModel.setupAnim` walking leg swing (the
      equine gait, legs at `[2, 3, 4, 5]`), the adult default-branch neck head
      look/bob (`head_parts` at `1`, the same yaw-clamp/pitch/walk-bob as the horse,
      since the adult `DonkeyModel` only adds chest visibility over the base
      `setupAnim`), and the adult tail walk lift (the same `getTailXRotOffset = 0`,
      `ageScale = 1` formula as the horse, the tail child swung with the chest children
      kept in place when present, colored path); the baby donkey/mule leg swing, head look,
      and tail (its legs are re-parented under the body and `BabyDonkeyModel.setupAnim`
      forces `xRot = -30°`), saddle equipment layer, the ridden/eat/stand/mouth poses, the
      tail's `ageInTicks` yRot wag, lighting, and GPU texture binding remain unsupported
    - skeleton horse and zombie horse entities as renderer-owned vanilla 26.1
      adult/baby body-layer geometry from `AbstractEquineModel`,
      `BabyHorseModel`, `HorseModel`, and `UndeadHorseRenderer`, including the
      unscaled `ModelLayers.SKELETON_HORSE` / `ZOMBIE_HORSE` adult layers,
      shared baby horse layer, official adult/baby skeleton/zombie horse
      texture references recorded from assets, and the shared
      `AbstractEquineModel.setupAnim` walking leg swing (the equine gait, legs at
      `[2, 3, 4, 5]` adult / `[1, 2, 3, 4]` baby), the default-branch neck head
      look/bob (`head_parts` at `1` adult / `5` baby horse, the same yaw-clamp/pitch/
      walk-bob as the horse it reuses), and the tail walk lift (the same formula as the
      horse, including the baby `getTailXRotOffset = −π/2` / `ageScale = 0.5` override,
      colored path); undead horse body-armor layer, saddle layer, the
      ridden/eat/stand/mouth poses, the tail's `ageInTicks` yRot wag, lighting, and GPU
      texture binding remain unsupported
    - camel and camel_husk entities as renderer-owned vanilla 26.1 body-layer
      geometry from `AdultCamelModel`, `BabyCamelModel`, `CamelRenderer`, and
      `CamelHuskRenderer`, including `ModelLayers.CAMEL` / `CAMEL_BABY` (the camel
      husk reuses the adult `camel#main` mesh), normal camel adult/baby model
      selection, camel_husk adult-only renderer selection, zero-thickness tail
      cubes, official camel (128×128) / camel_baby (64×64) / camel_husk (128×128)
      texture references, texture-backed base layer pass emission, and the official
      PNG atlas upload/bind/sample path (colored and textured, both at the static
      rest pose); saddle equipment layers, the `CamelModel.setupAnim` keyframe
      walk/sit/standup/idle/dash animations and the direct head yaw/pitch clamp
      (which would require the deferred `KeyframeAnimation` framework and camel head
      tracking), and lighting remain unsupported
    - llama and trader llama entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `LlamaModel`, `BabyLlamaModel`, and
      `LlamaRenderer`, including `ModelLayers.LLAMA` / `LLAMA_BABY` (the trader
      llama shares the same baked mesh under `ModelLayers.TRADER_LLAMA` /
      `TRADER_LLAMA_BABY`), official per-variant adult (128×64) / baby (64×64)
      texture references, texture-backed base layer pass emission, official PNG
      atlas upload/bind/sample path, metadata-driven adult chest visibility, baby
      chest suppression, and the vanilla `LlamaModel.setupAnim` head-look yaw/pitch
      on the head part plus the standard `QuadrupedModel` diagonal leg swing
      (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at `[2, 3, 4, 5]` adult /
      `[4, 5, 6, 7]` with-chest / `[1, 2, 3, 4]` baby, colored and textured); the
      trader llama's `LlamaDecorLayer` overlay and llama decor/body equipment
      layers, llama spit projectile model, and lighting remain unsupported
    - goat entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `GoatModel`, `BabyGoatModel`, and `GoatRenderer`,
      including `ModelLayers.GOAT` / `GOAT_BABY`, official adult/baby texture
      references, texture-backed base layer pass emission, official PNG atlas
      upload/bind/sample path, metadata-driven left/right horn visibility, and the
      vanilla `QuadrupedModel.setupAnim` head-look yaw/pitch on the head part plus the
      `QuadrupedModel` leg swing (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at
      `[2, 3, 4, 5]` adult / `[0, 1, 2, 3]` baby, colored and textured, with the horn
      children rotating with the head); screaming goat sounds and the
      ramming/lowering-head event animation (which would override the head pitch)
      remain unsupported
    - polar bear entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `PolarBearModel`, `BabyPolarBearModel`, and
      `PolarBearRenderer`, including `ModelLayers.POLAR_BEAR` /
      `POLAR_BEAR_BABY`, adult `MeshTransformer.scaling(1.2F)`, and official
      adult/baby texture references, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and the vanilla
      `PolarBearModel.setupAnim` standing-rear head/body/front-leg pose driven by
      the canonical `clientSideStandAnimation` countdown projected through
      `PolarBear.getStandingAnimationScale` and the renderer partial tick, and the
      vanilla `QuadrupedModel.setupAnim` head-look yaw/pitch applied before the
      standing-rear `head.xRot += standScale * π * 0.15` term (colored and
      textured, matching vanilla's `super.setupAnim`-then-rear order); walk
      animation and lighting remain unsupported
    - hoglin and zoglin entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `HoglinModel`, `BabyHoglinModel`,
      `AbstractHoglinRenderer`, `HoglinRenderer`, and `ZoglinRenderer`,
      including shared `ModelLayers.HOGLIN` / `ZOGLIN` and `HOGLIN_BABY` /
      `ZOGLIN_BABY` layers plus official adult/baby hoglin/zoglin texture
      references, texture-backed base layer pass emission, official PNG
      atlas upload/bind/sample path, the vanilla `HoglinModel.setupAnim`
      yaw-only head look (`head.yRot = yRot * π/180`, keeping `head.xRot` at the
      fixed headbutt-rest tilt `HOGLIN_HEAD_X_ROT`) on the head part, the
      `1.2`-amplitude leg swing (legs at `[2, 3, 4, 5]`), and the ear sway for both adults
      and babies (`ear.zRot = ±2π/9 ± speed * sin(pos)`, ears at head children `[0, 1]`, the
      head subtree hand-emitted; the formula sets the absolute `±2π/9`, overriding the wider
      rest angle of `BabyHoglinModel`'s layer) — all colored and textured; the headbutt
      attack animation, hoglin converting shake, and lighting remain unsupported
    - ravager entities as renderer-owned vanilla 26.1 `RavagerModel`
      body-layer geometry from `RavagerModel` and `RavagerRenderer`,
      including nested neck/head/horn/mouth parts, official
      `textures/entity/illager/ravager.png` texture reference, and
      `ModelLayers.RAVAGER`, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and the vanilla
      `RavagerModel.setupAnim` head look (`head.xRot/yRot = xRot/yRot * π/180`) on
      the neck-nested head part — the neck subtree is emitted by hand so the head
      carries the look while its horn/mouth children inherit it (colored and
      textured), and the vanilla `RavagerModel.setupAnim` leg walk swing
      (`ravager_leg_swing_pose`: the `QuadrupedModel` diagonal phase
      `cos(pos * 0.6662 [+ π])` at the shorter `0.4` amplitude, legs `[2, 3, 4, 5]`,
      `xRot` only so the neck/head subtree is untouched) on both render paths; attack
      neck motion, stunned neck/mouth animation, roar mouth animation, and lighting
      remain unsupported
    - villager entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `VillagerModel`, `BabyVillagerModel`, and
      `VillagerRenderer`, with the adult `MeshTransformer.scaling(0.9375F)`
      root transform, official base texture references, texture-backed base
      layer pass emission, adult/baby model-layer selection, and official PNG
      atlas upload/bind/sample path; wandering trader uses
      `ModelLayers.WANDERING_TRADER`, the same adult body layer, the official
      `textures/entity/wandering_trader/wandering_trader.png` reference,
      texture-backed base layer pass emission, official PNG atlas
      upload/bind/sample path, and the vanilla `VillagerModel.setupAnim`
      head-look yaw/pitch on the head part (colored and textured, including the
      baby villager index-3 head); villager type/profession/level overlays, hat
      metadata/no-hat model selection, crossed-arms item layer, custom head
      layer, unhappy animation, leg walk animation, lighting, and wandering
      trader baby presentation remain unsupported
    - base zombie entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `HumanoidModel`, `BabyZombieModel`, and `ZombieRenderer`,
      with a texture-backed cutout render path: the adult layer emits the vanilla
      `HumanoidModel.createMesh` UVs over `textures/entity/zombie/zombie.png` (the
      `texOffs(32, 0)` hat keeps its base 8x8x8 box as the UV source, and the left
      arm/leg mirror the right's `texOffs`), the baby layer emits the
      `BabyZombieModel.createBodyLayer` UVs over
      `textures/entity/zombie/zombie_baby.png` (each limb has its own `texOffs`,
      no mirroring), with official PNG atlas upload/bind/sample and the head-look /
      leg-swing animation on both render paths (the held-out `animateZombieArms`
      arm pose stays deferred, so the visible arms hold still as in the colored
      path); husk entities share that texture-backed render path through
      `HuskRenderer extends ZombieRenderer`: they reuse the zombie adult/baby body
      parts (so the husk geometry is byte-for-byte the zombie geometry) over
      `textures/entity/zombie/husk.png` / `textures/entity/zombie/husk_baby.png`,
      with the adult mesh scaled by the vanilla 26.1 `LayerDefinitions`
      `MeshTransformer.scaling(1.0625F)` (`huskScale`) at the model root, the baby
      reusing the unscaled shared baby zombie body layer, and the same official PNG
      atlas upload/bind/sample plus head-look / leg-swing animation on both render
      paths (arms deferred, as for the base zombie); drowned entities share that
      texture-backed render path through `DrownedRenderer extends
      AbstractZombieRenderer`: the adult layer emits the vanilla 26.1
      `DrownedModel.createBodyLayer(CubeDeformation.NONE)` UVs over
      `textures/entity/zombie/drowned.png` (the head/hat/body/right-limb UVs match
      the humanoid layer, but the left arm/leg take their own non-mirrored
      `texOffs(32, 48)` / `texOffs(16, 48)` regions; the geometry is identical to
      the humanoid limbs, so the colored geometry is unchanged), the baby layer
      forwards to `BabyDrownedModel.createBodyLayer` (= `BabyZombieModel`) UVs over
      `textures/entity/zombie/drowned_baby.png`, with official PNG atlas
      upload/bind/sample and the head-look / leg-swing animation on both render
      paths (the `DrownedOuterLayer`, the `setupRotations` / `setupAnim` swim
      re-pose that needs `swimAmount`, the trident throw arm pose that needs a held
      item, and the held-out `animateZombieArms` arms all stay deferred); zombie
      villagers share that texture-backed render path through `ZombieVillagerModel
      extends HumanoidModel`: the adult layer emits the vanilla 26.1
      `ZombieVillagerModel.createBodyLayer()` UVs over
      `textures/entity/zombie_villager/zombie_villager.png` (head `texOffs(0, 0)`,
      nose `texOffs(24, 0)`, the deformed villager hat `texOffs(32, 0)` over its
      base 8x10x8 box plus the rotated `texOffs(30, 47)` hat rim, body
      `texOffs(16, 20)` plus the `texOffs(0, 38)` robe overlay, arms
      `texOffs(44, 22)` with the left mirrored, legs `texOffs(0, 22)` with the left
      mirrored), the baby layer emits the
      `BabyZombieVillagerModel.createBodyLayer()` UVs over
      `textures/entity/zombie_villager/zombie_villager_baby.png` (each limb has its
      own `texOffs`, no mirroring), with official PNG atlas upload/bind/sample and
      the head-look / leg-swing animation on both render paths (the held-out
      `animateZombieArms` arms stay deferred); piglins, piglin brutes, and zombified
      piglins share a texture-backed render path through `AbstractPiglinModel`: all
      five families emit the shared vanilla 26.1 `AdultPiglinModel.createBodyLayer()`
      / `BabyPiglinModel.createBodyLayer()` geometry
      (`AdultZombifiedPiglinModel`/`BabyZombifiedPiglinModel` forward to those
      layers, and the brute reuses the adult layer) — the `addHead` snout head + ears
      (`texOffs(0, 0)` head, `texOffs(31, 1)` snout, `texOffs(2, 4)`/`texOffs(2, 0)`
      nostrils, `texOffs(51, 6)`/`texOffs(39, 6)` ears), the `texOffs(16, 16)` body
      (the `PlayerModel` jacket is cleared), and the shared `PlayerModel.createMesh`
      wide arm/sleeve/leg/pants UVs — over each family's own
      `textures/entity/piglin/{piglin,piglin_baby,piglin_brute,zombified_piglin,zombified_piglin_baby}.png`,
      with official PNG atlas upload/bind/sample and the vanilla
      `AbstractPiglinModel.setupAnim` head-look, leg swing, ear flap, and (for the
      non-zombified families) arm counter-swing on both render paths (the zombified
      piglin keeps its held-out `animateZombieArms` arms deferred);
      `DrownedOuterLayer`, drowned swim
      rotation, trident throw arm pose, zombie villager type/profession/level
      overlays, zombie villager no-hat model selection, zombie/piglin
      converting shake, zombie-family and piglin-family armor, custom head
      layers, held items, and attack/walk/dance/crossbow/admiring/zombie-arm
      animation remain unsupported;
      the zombie, husk,
      drowned, zombie-villager, piglin, piglin-brute, and zombified-piglin head
      parts now apply the vanilla `HumanoidModel.setupAnim` head-look yaw/pitch
      (the baby layout's index-1 head, and the baby piglin brute's adult-layout
      head, included), and the zombie and piglin families also apply the inherited
      `HumanoidModel` leg swing on their two leg parts, with the inherited arm
      counter-swing added on the two arm parts for the non-zombified piglin family
      (adult/baby piglin and brute; the zombified piglin's arms keep the deferred
      `animateZombieArms` held-out pose, and the `AbstractPiglinModel` ear sway and
      `PiglinModel` override arm poses stay deferred)
    - base skeleton, stray, parched, wither skeleton, and bogged entities as
      renderer-owned vanilla 26.1 skeleton-family geometry from
      `SkeletonModel.createBodyLayer()`,
      `SkeletonModel.createSingleModelDualBodyLayer()`, `BoggedModel`, and
      `LayerDefinitions`, including `ModelLayers.SKELETON` / `STRAY` /
      `PARCHED` / `WITHER_SKELETON` / `BOGGED`, wither skeleton
      `MeshTransformer.scaling(1.2F)`, bogged head mushroom children and
      metadata-driven sheared mushroom visibility, official base texture
      references, texture-backed base layer pass emission, stray
      `SkeletonClothingLayer` `stray_overlay.png` pass through
      `ModelLayers.STRAY_OUTER_LAYER`, bogged `SkeletonClothingLayer`
      `bogged_overlay.png` pass through `ModelLayers.BOGGED_OUTER_LAYER`,
      official PNG atlas upload/bind/sample path, and the vanilla
      `HumanoidModel.setupAnim` head-look yaw/pitch on the head part, the
      inherited `HumanoidModel` leg swing on the two leg parts, and the inherited
      `HumanoidModel` arm counter-swing on the two arm parts (colored and
      textured, including the parched body-first part order, the wither scaled
      transform, and the Stray/Bogged clothing overlay whose layer
      `SkeletonModel` runs the same `setupAnim`); skeleton-family armor, held
      bows/items, the `SkeletonModel` melee arm swing (`isAggressive &&
      !isHoldingBow`) and bow-aiming `ArmPose`, and lighting remain unsupported
    - creeper entities as renderer-owned vanilla 26.1
      `CreeperModel.createBodyLayer(CubeDeformation.NONE)` geometry, with the
      official `textures/entity/creeper/creeper.png` texture reference,
      `ModelLayers.CREEPER` selection, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and the vanilla
      `CreeperModel.setupAnim` head-look yaw/pitch on the head part and the
      `QuadrupedModel`-formula four-leg walk swing (colored and textured); powered
      armor layer, swelling model scale, and lighting remain unsupported
    - base spider entities as renderer-owned vanilla 26.1
      `SpiderModel.createSpiderBodyLayer()` geometry, with
      `ModelLayers.SPIDER`, the official
      `textures/entity/spider/spider.png` texture reference from
      `SpiderRenderer`, texture-backed base layer pass emission, and official
      PNG atlas upload/bind/sample path; cave spider entities use the same
      vanilla 26.1 body layer through `ModelLayers.CAVE_SPIDER`
      `MeshTransformer.scaling(0.7F)`, the official
      `textures/entity/spider/cave_spider.png` texture reference from
      `CaveSpiderRenderer`, texture-backed base layer pass emission, and
      official PNG atlas upload/bind/sample path; both spider and cave spider
      include the vanilla `SpiderEyesLayer` `spider_eyes.png` texture-backed
      eyes pass using the parent spider model parts, submit order `1`, and a
      `RenderTypes.eyes`-style translucent/depth-write-disabled GPU path, and the
      vanilla `SpiderModel.setupAnim` head-look yaw/pitch on the head part and the
      vanilla `SpiderModel.setupAnim` eight-leg walk swing (`spider_leg_swing_pose`:
      each leg sweeps `yRot += -(cos(animationPos*2 + phase) * 0.4) * speed` and steps
      `zRot += |sin(animationPos + phase) * 0.4| * speed`, right legs `+`/left legs `-`,
      per-pair phases `0`/`π`/`π/2`/`3π/2`, legs at `[3..=10]`) on both render paths and
      passes (colored and textured, both spider and cave spider); death
      flip and lighting remain unsupported
    - enderman entities as renderer-owned vanilla 26.1
      `EndermanModel.createBodyLayer()` geometry, including its
      `HumanoidModel.createMesh(CubeDeformation.NONE, -14.0F)` offsets,
      overwritten long limbs, shrunken hat cube, and the official
      `textures/entity/enderman/enderman.png` texture reference from
      `EndermanRenderer`, texture-backed base layer pass emission, and the
      vanilla `EnderEyesLayer` `enderman_eyes.png` texture-backed eyes pass
      using the parent Enderman model parts, submit order `1`, and a
      `RenderTypes.eyes`-style translucent/depth-write-disabled GPU path, and the
      vanilla `HumanoidModel.setupAnim` head-look yaw/pitch on the head part and
      the enderman walk animation — the inherited arm and leg swing halved and
      clamped to `[-0.4, 0.4]` (arms `[2, 3]`, legs `[4, 5]`) — (colored and
      textured); the carried-block layer, carried-block arm pose, creepy head
      offset, creepy render jitter, and lighting remain unsupported
    - iron golem entities as renderer-owned vanilla 26.1
      `IronGolemModel.createBodyLayer()` geometry, including its 128x128 body
      layer, baked `CubeDeformation(0.5F)` lower-body cube, and the official
      `textures/entity/iron_golem/iron_golem.png` texture reference from
      `IronGolemRenderer`, texture-backed base layer pass emission, and
      official PNG atlas upload/bind/sample path, and the vanilla
      `IronGolemModel.setupAnim` head-look yaw/pitch on the head part (colored and
      textured); crackiness overlay textures, flower block layer, attack arm
      pose, offer-flower arm pose, leg walk animation, and renderer body-wobble
      rotation remain unsupported
    - snow golem entities as renderer-owned vanilla 26.1
      `SnowGolemModel.createBodyLayer()` geometry, including its 64x64 body
      layer, baked `CubeDeformation(-0.5F)` snow body/arm/head cubes, and the
      official `textures/entity/snow_golem/snow_golem.png` texture reference
      from `SnowGolemRenderer`, texture-backed base layer pass emission, and
      official PNG atlas upload/bind/sample path, and the vanilla
      `SnowGolemModel.setupAnim` head-look yaw/pitch on the head part plus the
      upper-body quarter-yaw twist (`upperBody.yRot = headYaw * 0.25`) and the two
      stick arms orbiting that twist (`arm.yRot = upperBodyYRot [+ π]`, with `x`/`z`
      recomputed from cos/sin so the arms ride the body and collapse to `z = 0` when
      facing forward), on both render paths (colored and textured); carved pumpkin
      head block layer and pumpkin/no-pumpkin state projection remain unsupported
    - witch entities as renderer-owned vanilla 26.1
      `WitchModel.createBodyLayer()` geometry, including the
      `VillagerModel.createBodyModel()` body/arms/legs/nose, the four nested
      hat cuboids, baked hat-tip and mole `CubeDeformation` bounds,
      `LayerDefinitions`' `MeshTransformer.scaling(0.9375F)`, and the official
      `textures/entity/witch/witch.png` texture reference from
      `WitchRenderer`, `ModelLayers.WITCH`, texture-backed base layer pass
      emission, official PNG atlas upload/bind/sample path, and the vanilla
      `WitchModel.setupAnim` head-look yaw/pitch on the head part, the
      half-amplitude leg walk swing (legs at `[3, 4]`), and the continuous
      `ageInTicks`-driven idle nose bob (`nose.xRot = sin(ageInTicks * speed) *
      4.5°`, `nose.zRot = cos(ageInTicks * speed) * 2.5°`, `speed = 0.01 *
      (entityId % 10)`), all on both render paths (colored and textured);
      `WitchItemLayer`, the held-potion state, and the `isHoldingItem` nose hold
      pose remain unsupported
    - evoker, illusioner, pillager, and vindicator entities as renderer-owned
      vanilla 26.1 `IllagerModel.createBodyLayer()` geometry, including
      `LayerDefinitions`' shared `MeshTransformer.scaling(0.9375F)`, baked
      hat/body `CubeDeformation` bounds, official 64x64 texture references from
      their vanilla renderers, illusioner's renderer-enabled hat, idle crossed
      arms for evoker/illusioner/vindicator, and uncrossed base arms for
      pillager, with a texture-backed cutout render path: each family emits the
      shared `IllagerModel.createBodyLayer` UVs (head `texOffs(0, 0)`, nose
      `texOffs(24, 0)`, hat `texOffs(32, 0)` over its base 8x12x8 box, body
      `texOffs(16, 20)` plus the `texOffs(0, 38)` robe overlay keeping its base
      8x20x6 box, the folded `texOffs(44, 22)`/`texOffs(40, 38)` arms with the
      mirrored left-shoulder child, legs `texOffs(0, 22)`, and the uncrossed
      pillager arms `texOffs(40, 46)`) over its own
      `textures/entity/illager/{evoker,illusioner,pillager,vindicator}.png`, with
      official PNG atlas upload/bind/sample and the vanilla `IllagerModel.setupAnim`
      head-look yaw/pitch plus the half-amplitude leg swing (and the pillager's
      `HumanoidModel` arm swing on its separate arms) on both render paths; the
      item-in-hand/custom-head layers, spell/crossbow/attacking/
      celebrating/riding arm poses and animation, held item projection,
      illusioner clone offsets/invisible-body rendering, and renderer state
      extraction for dynamic arm visibility remain unsupported
    - armor stand entities as renderer-owned vanilla 26.1
      `ArmorStandModel.createBodyLayer()` geometry, including the normal layer,
      `ModelLayers.ARMOR_STAND_SMALL` `HumanoidModel.BABY_TRANSFORMER` root-part
      transform, official `textures/entity/armorstand/armorstand.png` texture
      reference, client flags for small/show-arms/no-baseplate, and head/body/
      arm/leg pose metadata projection; the textured base layer emits the vanilla
      `createBodyLayer` `texOffs` UVs (the small layer reuses the full-model UVs
      because `BABY_TRANSFORMER` only scales geometry, not texture coordinates),
      with official PNG atlas upload/bind/sample on both render paths;
      armor/equipment/custom-head/elytra/held-item layers, hurt wiggle,
      marker/invisible render-type nuances, and animation interpolation remain
      unsupported
    - slime entities as renderer-owned vanilla 26.1 `SlimeModel` inner
      `ModelLayers.SLIME` geometry plus outer `ModelLayers.SLIME_OUTER`
      geometry, official `textures/entity/slime/slime.png` texture reference,
      renderer size scaling from slime size metadata, texture-backed base and
      outer layer pass emission, the `SlimeOuterLayer` submit order `1`, and an
      alpha-blended translucent GPU bucket; squish interpolation, invisible
      glowing outline rendering, particle/audio coupling, lighting, overlay,
      crumbling, and full render-graph sorting parity remain unsupported
    - magma cube entities as renderer-owned vanilla 26.1
      `MagmaCubeModel.createBodyLayer()` segment/inside-cube geometry, official
      `textures/entity/slime/magmacube.png` texture reference, and renderer
      size scaling from inherited slime size metadata, texture-backed base layer
      pass emission, and official PNG atlas upload/bind/sample path; segment
      squish offsets, full-bright block light, particle/audio coupling,
      lighting, overlay, crumbling, and full render-graph sorting parity remain
      unsupported
    - ghast entities as renderer-owned vanilla 26.1 `GhastModel.createBodyLayer()`
      geometry: the 16x16x16 body at y 17.6 plus the nine tentacles at y 24.6,
      whose lengths are the fixed-seed `RandomSource(1660L)` (`nextInt(7) + 8`,
      reproduced via the Java legacy LCG → `[8, 13, 9, 11, 11, 10, 12, 9, 12]`) and
      whose xz offsets come from the vanilla index formula, scaled 4.5x by the
      `MeshTransformer.scaling(4.5F)` model-root transform; the official
      `textures/entity/ghast/ghast.png` texture reference, texture-backed base
      layer pass emission, official PNG atlas upload/bind/sample path, and the
      vanilla `GhastModel.setupAnim` tentacle wave (`tentacle.xRot = 0.2 *
      sin(ageInTicks * 0.3 + i) + 0.4`, driven by the projected `ageInTicks`, on
      both render paths). The `isCharging` shooting-texture variant
      (`ghast_shooting.png`), lighting, and overlay remain unsupported
    - happy ghast entities as renderer-owned vanilla 26.1
      `HappyGhastModel.createBodyLayer(false, NONE)` geometry: the 16x16x16 body at
      y 16 plus the nine tentacles parented under the body (world-space y 23) with
      hard-coded lengths `[5, 7, 4, 5, 5, 7, 8, 8, 5]`, scaled 4.0x by the
      `MeshTransformer.scaling(4.0F)` model-root transform; the official
      `textures/entity/ghast/happy_ghast.png` texture reference, texture-backed base
      layer pass emission, official PNG atlas upload/bind/sample path, and the
      vanilla `HappyGhastModel.setupAnim` tentacle wave (it reuses
      `GhastModel.animateTentacles` verbatim, `tentacle.xRot = 0.2 * sin(ageInTicks *
      0.3 + i) + 0.4`, driven by the projected `ageInTicks`, on both render paths).
      The baby model (the extra `inner_body` cube plus the 0.2375 baby scale), the
      `bodyItem` body squeeze (`0.9375` scale when a harness is equipped) with the
      harness equipment layer and the rope/lead layer, lighting, and overlay remain
      unsupported
    - blaze entities as renderer-owned vanilla 26.1 `BlazeModel.createBodyLayer()`
      geometry: the 8x8x8 head at `PartPose.ZERO` plus twelve `2x8x2` rods (the
      shared `texOffs(0, 16)` `rod` builder), with no `MeshTransformer` scaling (the
      unit entity model-root transform); the official
      `textures/entity/blaze/blaze.png` texture reference, texture-backed base layer
      pass emission, official PNG atlas upload/bind/sample path, the vanilla
      `BlazeModel.setupAnim` rod orbit (twelve rods in three rings of radius 9/7/5,
      their x/y/z offsets set every frame from the projected `ageInTicks`), and the
      shared head look (`head.yRot/xRot` from the net look angles), on both render
      paths. The `BlazeRenderer` full-bright block light (`getBlockLightLevel = 15`),
      lighting, and overlay remain unsupported
    - endermite entities as renderer-owned vanilla 26.1
      `EndermiteModel.createBodyLayer()` geometry: the four nested chitin segments
      from `BODY_SIZES`/`BODY_TEXS` (each `addBox(-sx/2, 0, -sz/2, sx, sy, sz)` posed
      at `(0, 24 - sy, placement)`), with no `MeshTransformer` scaling (the unit
      entity model-root transform); the official
      `textures/entity/endermite/endermite.png` texture reference, texture-backed
      base layer pass emission, official PNG atlas upload/bind/sample path, and the
      vanilla `EndermiteModel.setupAnim` segment wiggle (`segment.yRot = cos(phase) *
      π * 0.01 * (1 + |i - 2|)`, `segment.x = sin(phase) * π * 0.1 * |i - 2|`,
      `phase = ageInTicks * 0.9 + i * 0.15 * π`, driven by the projected
      `ageInTicks`, on both render paths). Lighting and overlay remain unsupported
    - silverfish entities as renderer-owned vanilla 26.1
      `SilverfishModel.createBodyLayer()` geometry: the seven nested body segments
      from `BODY_SIZES`/`BODY_TEXS` plus the three wider overlay layers riding
      segments 2/4/1 (`texOffs(20, 0/11/18)`, including the vanilla quirk where
      layer2 takes its z-min from `BODY_SIZES[4]` but its z-size from
      `BODY_SIZES[1]`), with no `MeshTransformer` scaling (the unit entity model-root
      transform); the official `textures/entity/silverfish/silverfish.png` texture
      reference, texture-backed base layer pass emission, official PNG atlas
      upload/bind/sample path, and the vanilla `SilverfishModel.setupAnim` segment
      wiggle (`segment.yRot = cos(phase) * π * 0.05 * (1 + |i - 2|)`, `segment.x =
      sin(phase) * π * 0.2 * |i - 2|`, `phase = ageInTicks * 0.9 + i * 0.15 * π`,
      with the overlay layers copying segments 2/4/1, driven by the projected
      `ageInTicks`, on both render paths). Lighting and overlay remain unsupported
    - vex entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `138` to the real `VexModel`, replacing
      the former placeholder box. Renderer-owned vanilla 26.1 `VexModel.createBodyLayer()`
      geometry: the `(0, -2.5, 0)` model root carrying the 5³ head and the body (a plain
      `texOffs(0, 10)` 3×4×2 box plus a `texOffs(0, 16)` 3×5×2 box inset by
      `CubeDeformation(-0.2)`), with the two arms (`texOffs(23, 0)`/`texOffs(23, 6)`, 2×4×2
      inset by `CubeDeformation(-0.1)`) and the two zero-thickness `0×5×8` wings
      (`texOffs(16, 14)`, the left wing's UV mirrored) parented under the body so the body
      tilt carries them; the non-charging `VexModel.setupAnim` idle pose (head look
      `yRot`/`xRot`, arms at `±(π/5 + cos(ageInTicks · 5.5°) · 0.1)` z-roll, body tilt
      `π/20`, and the wing flap `leftWing.yRot = 1.0995574 + cos(ageInTicks · 45.836624°) ·
      16.2°` mirrored on the right wing with both wings pitched/rolled `0.47123888`), driven
      by the projected head yaw/pitch and `age_in_ticks`, under the standard
      `LivingEntityRenderer.setupRotations`. The textured base layer draws the
      `textures/entity/illager/vex.png` atlas reference into the translucent mesh
      (`RenderTypes::entityTranslucent`), hand-emitted through the same animated body→arm/wing
      hierarchy as the colored path. The charging pose (`isCharging` texture swap to
      `vex_charging.png`, the charging arm poses and held items) and the constant full-bright
      `getBlockLightLevel` (→ 15) glow, lighting, and overlay remain unsupported
    - allay entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `2` to the real `AllayModel`, replacing
      the former placeholder box. Renderer-owned vanilla 26.1 `AllayModel.createBodyLayer()`
      geometry: the `(0, 23.5, 0)` model root carrying the 5³ head and the body (a plain
      `texOffs(0, 10)` 3×4×2 box plus a `texOffs(0, 16)` 3×5×2 box inset by
      `CubeDeformation(-0.2)`), with the two arms (`texOffs(23, 0)`/`texOffs(23, 6)`, 1×4×2
      inset by `CubeDeformation(-0.01)`) and the two zero-thickness `0×5×8` wings
      (`texOffs(16, 14)`, neither mirrored) parented under the body so the body tilt carries
      them; the non-dancing `AllayModel.setupAnim` idle / flying pose (head look `yRot`/`xRot`,
      arm idle roll `±(0.43633232 - cos(ageInTicks · 9° + 3π/2) · π · 0.075 · (1 -
      flyingFactor))`, wings flapping `yRot = ±π/4 ∓ (cos(ageInTicks · 20° + walkPos) · π ·
      0.15 + walkSpeed)` and pitched `0.43633232 · (1 - flyingFactor)`, body tilt
      `flyingFactor · π/4`, and the vertical root bob `23.5 + cos(ageInTicks · 9°) · 0.25 · (1
      - flyingFactor)` with `flyingFactor = min(walkSpeed / 0.3, 1)`), driven by the projected
      head yaw/pitch, walk animation, and `age_in_ticks`, under the standard
      `LivingEntityRenderer.setupRotations`. The textured base layer draws the
      `textures/entity/allay/allay.png` atlas reference into the translucent mesh
      (`RenderTypes::entityTranslucent`), hand-emitted through the same animated body→arm/wing
      hierarchy as the colored path. The dance pose (`isDancing`/`isSpinning`,
      `spinningProgress`), the held-item arm poses (`holdingAnimationProgress` scaling the arm
      roll to zero and adding the `±0.27925268` arm yaw plus the flying-lerped arm pitch and
      held item), and the constant full-bright `getBlockLightLevel` (→ 15) glow, lighting, and
      overlay remain unsupported
    - strider entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `129` to the real `AdultStriderModel` /
      `BabyStriderModel`, replacing the former horse-quadruped fallback, keyed off the synced
      `AgeableMob.DATA_BABY_ID` (index 16, default adult). Renderer-owned vanilla 26.1
      geometry: the adult `createBodyLayer()` (atlas 64×128) — the `texOffs(0, 0)` 16×14×16
      body, the two `texOffs(0, 32)`/`texOffs(0, 55)` 4×16×4 legs, and the six zero-thickness
      `12×0×16` bristles (right mirrored, `texOffs(16, 33/49/65)`) parented under the body — and
      the baby `createBodyLayer()` (atlas 32×32) — the `texOffs(0, 0)` 7×7×8 body, two
      `texOffs(0/8, 24)` 2×4×2 legs, and three zero-thickness `7×3×0` bristles. The shared
      `StriderModel.setupAnim` (body sway `zRot = 0.1·sin(pos·1.5)·4·speed`, leg swing
      `xRot = sin(pos·0.75 + phase)·2·speed` and roll `zRot = (π/18)·cos(pos·0.75 + phase)·speed`
      with `speed = min(walkSpeed, 0.25)`) plus `customAnimations` (body bob `base -
      mul·cos(pos·1.5)·2·speed`, leg lift `base + 2·sin(pos·0.75 + phase)·2·speed`, and the
      bristle flow `cos(pos·1.5 + π)·speed` with the per-bristle `0.6/1.2/1.3` weights and the
      `0.1·sin(age·0.4)` / `0.1·sin(age·0.2)` / `0.05·sin(age·-0.4)` idle ripple — adult bristles
      flow on `zRot`, baby bristles on `xRot`) are driven by the projected look angles, walk
      animation, and `age_in_ticks`, under the standard `LivingEntityRenderer.setupRotations`.
      The textured base layer draws the `textures/entity/strider/strider.png` /
      `strider_baby.png` atlas references into the cutout mesh (default
      `RenderTypes::entityCutout`), hand-emitted through the same animated leg/body/bristle
      hierarchy as the colored path. The ridden pose (`isRidden` zeroing the body look), the
      saddle equipment layer, and the cold/suffocating texture swap (`strider_cold.png` /
      `strider_cold_baby.png`) and shake remain unsupported
    - turtle entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `137` to the real `AdultTurtleModel` /
      `BabyTurtleModel`, replacing the former placeholder box, keyed off the synced
      `AgeableMob.DATA_BABY_ID` (index 16, default adult). Renderer-owned vanilla 26.1 geometry:
      the adult `createBodyLayer()` (atlas 128×64) — the `texOffs(3, 0)` 6×5×6 head, the body's
      `texOffs(7, 37)` 19×20×6 shell + `texOffs(31, 1)` 11×18×3 belly under a fixed `Rx(π/2)`,
      and the four legs (hind `4×1×10`, front `13×1×5`) — and the baby `createBodyLayer()` (atlas
      16×16) — the `texOffs(0, 0)` 4×2×4 body, the `texOffs(0, 6)` 3×3×3 head, and four
      zero-height `2×0×1` leg planes. The shared `QuadrupedModel.setupAnim` head look + diagonal
      leg swing (`leg.xRot = cos(pos·0.6662 + phase)·1.4·speed`) plus `TurtleModel.setupAnim`'s
      land/water branch — on land the legs add a `yRot` walk swing (`±cos(pos·5)·{8 front, 3
      hind}·speed`), in water they paddle (hind `xRot = cos(pos·0.39972)·0.5·speed`, front the
      same on `zRot`) — driven by the projected look angles, walk animation, and the real
      `isOnLand = !isInWater && onGround` (both the water overlap and the synced `Entity.onGround`
      flag are now projected into the entity render state), under the standard
      `LivingEntityRenderer.setupRotations`. The textured base layer draws the
      `textures/entity/turtle/turtle.png` / `turtle_baby.png` atlas references into the cutout
      mesh (default `RenderTypes::entityCutout`), hand-emitted through the same animated
      head/body/leg hierarchy as the colored path. The egg-laying leg amplitude (`isLayingEgg`,
      the `layEgg`/`layEggAmplitude` multipliers) and the `egg_belly` overlay shell (`hasEgg`)
      remain unsupported
    - bat entities are wired end to end on both render paths off the real vanilla 26.1
      `BatModel`: the native entity scene (`entity_scene.rs`) projects vanilla type id `10` to the
      new `EntityModelKind::Bat`, replacing the former placeholder box. Renderer-owned vanilla
      `BatModel.createBodyLayer()` geometry (atlas 32×32) — the `texOffs(0, 0)` 3×5×2 body and
      `texOffs(0, 7)` 4×3×2 head at `+17`, the zero-thickness ear (`texOffs(1, 15)` /
      `texOffs(8, 15)`), wing (`texOffs(12, 0)` / `texOffs(12, 7)`), wing-tip (`texOffs(16, 0)` /
      `texOffs(16, 8)`) and feet (`texOffs(16, 16)`) planes hand-emitted through the bind-pose
      hierarchy (ears parented under the head, wings and feet under the body, each wing tip under
      its wing). This slice also introduces a renderer-owned port of the vanilla
      `net.minecraft.client.animation` keyframe framework (`entity_models/keyframe.rs`):
      `AnimationDefinition` / `AnimationChannel` / `Keyframe` with the `KeyframeAnimations.posVec`
      (y negated) and `degreeVec` (degrees→radians) helpers and LINEAR-interpolation sampling (the
      `Mth.binarySearch` previous/next lookup plus the clamped `lerp(prev.post, next.pre,
      alpha)·scale`) verbatim. The looping 0.5s `BatAnimation.BAT_FLYING` definition over its seven
      bones (head, body, feet, both wings, both wing tips) drives the flap, sampled at
      `ageInTicks·0.05` seconds and added to the bind pose as vanilla `applyStatic` offsets. The
      textured base layer draws the `textures/entity/bat/bat.png` atlas reference into the cutout
      mesh (vanilla `RenderTypes::entityCutoutCull`), hand-emitted through the same animated
      head/body/wing hierarchy as the colored path. The `isResting` branch and the `BAT_RESTING`
      resting animation, the idle head-look pose, the `AnimationState` start-tick phase offset, and
      the keyframe `CATMULLROM` interpolation / `Scale` target (only `LINEAR` + position/rotation
      are ported so far) remain unsupported
    - bee entities are wired end to end on both render paths off the real vanilla 26.1
      `AdultBeeModel` / `BabyBeeModel`: the native entity scene (`entity_scene.rs`) projects vanilla
      type id `11` to the new `EntityModelKind::Bee { baby }`, keyed off the synced
      `AgeableMob.DATA_BABY_ID` (index 16, default adult), replacing the former placeholder box.
      Renderer-owned vanilla geometry: the adult `createBodyLayer()` (atlas 64×64) — the empty
      `bone` pivot at `+19` parenting the `texOffs(0, 0)` 7×7×10 body (carrying the
      `texOffs(26, 7)` zero-thickness stinger and the `texOffs(2, 0)` / `texOffs(2, 3)` antennae),
      the two `CubeDeformation(0.001)`-inflated `texOffs(0, 18)` wing planes (the left mirrored), and
      the three `texOffs(26, 1/3/5)` zero-depth leg planes — and the baby `createBodyLayer()` (atlas
      32×32) — the two-cube `bone` (`texOffs(6, 12)` / `texOffs(0, 12)`), the 4×4×5 body, the
      stinger, the `0.2182`-pitched wings (the left at the vanilla negative `texOffs(-3, 9)`), and
      the three leg planes (no antennae). The procedural `BeeModel.setupAnim` is hand-emitted through
      the bone→body/wings/legs hierarchy: while airborne (`!isOnGround`, read from the synced
      `Entity.onGround`) the wings flap (`zRot = cos(ageInTicks·120.32113°)·π·0.15`, the left
      mirrored) and the non-angry `bobUpAndDown` rocks the bone pivot (`xRot`, `y`), the front/back
      legs, and — on adults — the antennae, with the middle leg held at `π/4`; on the ground the
      model rests at its bind pose. The textured base layer draws the `textures/entity/bee/bee.png` /
      `bee_baby.png` atlas references into the cutout mesh (vanilla `RenderTypes::entityCutoutCull`),
      hand-emitted through the same animated hierarchy as the colored path (which approximates the
      striped texture with a single representative yellow). The anger pose (`isAngry`), the rolled-up
      fall pose (`rollAmount`, `Mth.rotLerpRad` toward `3.0915928`), the stinger-loss visibility
      (`hasStinger`) and the nectar/angry texture swaps remain unsupported
    - breeze entities are wired end to end on both render paths off the real vanilla 26.1
      `BreezeModel`: the native entity scene (`entity_scene.rs`) projects vanilla type id `17` to the
      new `EntityModelKind::Breeze`, replacing the former placeholder box. Renderer-owned vanilla
      base body layer geometry (`createBodyLayer` retained to `head` + `rods`, atlas 32×32) — the
      head (`texOffs(4, 24)` 10×3×4 jaw plate + `texOffs(0, 0)` 8×8×8 cube) and the three
      `texOffs(0, 17)` 2×8×2 rods at their compound bind rotations, under the `body` pivot. This
      slice extends the keyframe framework (`entity_models/keyframe.rs`) with the cubic `CATMULLROM`
      interpolation (vanilla `AnimationChannel.Interpolations.CATMULLROM` sampling the four
      surrounding `postTarget`s through `Mth.catmullrom`), the second keyframe entity after the bat
      and the first to need it. The looping 2.0s `BreezeAnimation.IDLE` base-body channels drive the
      model — the head bobs on a CATMULLROM position spline while the rods spin a full `1080°` of yaw
      per cycle (LINEAR) and bob on a LINEAR position spline — sampled from `ageInTicks` (the idle
      `AnimationState` runs continuously). The textured base layer draws the
      `textures/entity/breeze/breeze.png` atlas reference into the translucent mesh (vanilla
      `BreezeModel` uses `RenderTypes::entityTranslucent`), hand-emitted through the same animated
      hierarchy as the colored path (which approximates the translucent wind body with a single
      representative slate). The swirling `breeze_wind.png` wind layer, the emissive
      `breeze_eyes.png` eyes, and the shoot/slide/inhale/jump action animations remain unsupported
    - dolphin entities are wired end to end on both render paths off the real vanilla 26.1
      `DolphinModel`: the native entity scene (`entity_scene.rs`) projects vanilla type id `35` to
      the new `EntityModelKind::Dolphin { baby }`, keyed off the synced `AgeableMob.DATA_BABY_ID`
      (index 16, default adult), replacing the former placeholder box. Renderer-owned vanilla
      `DolphinModel.createBodyLayer()` geometry (atlas 64×64) — the `texOffs(22, 0)` 8×7×13 `body`
      root child parenting the back fin, the two mirrored side fins (at their compound
      `Rx(π/3)·Rz(±2π/3)` bind rotations), the tail (with its tail fin), and the head (with its
      nose). The procedural `DolphinModel.setupAnim` is hand-emitted: the `body` steers by the
      projected look pitch/yaw (`body.xRot = state.xRot`, `body.yRot = state.yRot`) and, while moving
      (`isMoving`, projected from the synced `Entity.getDeltaMovement().horizontalDistanceSqr() >
      1e-7` into the entity render state), adds the swim body tilt
      (`xRot += -0.05 - 0.05·cos(ageInTicks·0.3)`) and the tail / tail-fin wave (`tail.xRot =
      -0.1·cos`, `tailFin.xRot = -0.2·cos`); at rest the tail holds its `-0.10471976` bind pitch. The
      baby uses the `MeshTransformer.scaling(0.5)` body layer (the shared mesh-transformer root
      scale, like the squid baby). The textured base layer draws the
      `textures/entity/dolphin/dolphin.png` / `dolphin_baby.png` atlas references into the cutout
      mesh (the `DolphinModel` default `RenderTypes::entityCutoutNoCull`), hand-emitted through the
      same animated hierarchy as the colored path (which approximates the texture with a single
      representative grey). The held-item carry layer (`DolphinCarryingItemLayer`) remains
      unsupported
    - guardian / elder guardian entities as renderer-owned vanilla 26.1
      `GuardianModel.createBodyLayer()` geometry on the colored path: the native entity scene
      (`entity_scene.rs`) projects vanilla type ids `63` (guardian) and `40` (elder guardian) to
      the new `EntityModelKind::Guardian { elder }`, keyed purely off the entity type id (no
      synced data), replacing the former placeholder boxes. The whole model hangs off one `head`
      part (`PartPose.ZERO`, atlas 64×64): the body shell (the 12×12×16 box, two mirrored 2×12×12
      side plates, the bottom/top 12×2×12 plates), the twelve spikes (a shared 2×9×2 box
      instanced at `getSpike{X,Y,Z}(i, 0, 0)` with rotation `PI · SPIKE_{X,Y,Z}_ROT[i]` from the
      verbatim `SPIKE_*` tables), the eye, and the nested three-segment tail. The elder guardian
      is the same mesh scaled 2.35× by `GuardianModel.ELDER_GUARDIAN_SCALE`
      (`MeshTransformer.scaling(2.35)`, composed at the root exactly like the squid/dolphin baby
      scale). The procedural `GuardianModel.setupAnim` is deferred — the model renders at its
      `createBodyLayer` rest pose — namely the head look (`head.yRot/xRot = state.yRot/xRot`), the
      spike age pulse (`getSpikeOffset = 1 + cos(ageInTicks · 1.5 + i) · 0.01`) and the
      `spikesAnimation` withdrawal (`(1 - spikesAnimation) · 0.55`), the eye target tracking
      (`lookAtPosition`/`lookDirection`/`eyePosition`), the tail sway (`tailAnimation`), and the
      `GuardianRenderer` attack beam (`attackTargetPosition`/`attackTime`/`attackScale`) — all of
      which read entity-side state not yet projected. The texture-backed path
      (`textures/entity/guardian/guardian.png`) and its lighting/overlay also remain unsupported
      (this is a colored-first slice; the colored debug path approximates the body with a single
      teal tint and the eye with a pink tint)
    - frog entities as renderer-owned vanilla 26.1 `FrogModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `55` to
      the new `EntityModelKind::Frog`, replacing the former placeholder box. The static
      rest-pose hierarchy is emitted directly (atlas 48×48): the `root` part at `offset(0, 24, 0)`
      parents `body` (the 7×3×9 box + 7×0×9 underside plane) and the two legs; `body` parents the
      head (7×0×9 plane + 7×3×9 box) with its `eyes` pivot and two 3×2×3 eyes, the tongue, and the
      two 2×3×3 arms, each carrying an 8×0×8 webbed hand; each leg carries an 8×0×8 foot — fifteen
      visible cubes (the `croaking_body` is hidden at rest, so it is omitted). The frog is the
      first keyframe-animated entity rendered at its `createBodyLayer` rest pose: every
      `FrogModel.setupAnim` animation — the jump, croak (and the `croaking_body` it reveals),
      tongue, swim/walk (`applyWalk`), and idle-in-water keyframe animations — is deferred
      entity-side state. The texture-backed path and the three frog texture variants
      (temperate/warm/cold, `FrogVariant`) also remain unsupported (this is a colored-first slice;
      the colored debug path approximates the body with one orange-tan tint and the eyes with a
      gold tint)
    - creaking entities as renderer-owned vanilla 26.1 `CreakingModel.createBodyLayer()` geometry
      on the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id
      `31` to the new `EntityModelKind::Creaking`, replacing the former placeholder box. The static
      rest-pose hierarchy is emitted directly (atlas 64×64): the `root` part at `offset(0, 24, 0)`
      parents the `upper_body` pivot and the two legs; `upper_body` parents the head (the 6×10×6
      skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes), the body (6×13×5 trunk + 6×7×5
      block), and the two asymmetric arms (the right a 3×21×3 limb + hand, the left a 3×16×3 limb +
      two blocks); each leg carries a 5×0×9 foot plane and the right leg an extra 3×3×3 hip block —
      sixteen cubes. Every `CreakingModel.setupAnim` animation is deferred (fittingly, the creaking
      freezes into a statue while observed): the head look (`head.xRot/yRot = state.xRot/yRot`), the
      walk (`applyWalk`), and the attack / invulnerable / death keyframe animations. The emissive
      eyes layer (`createEyesLayer`, the `head` part only) and the texture-backed path also remain
      unsupported (this is a colored-first slice; the colored debug path approximates the whole
      model with one dark-bark tint)
    - sniffer entities as renderer-owned vanilla 26.1 `SnifferModel.createBodyLayer()` geometry on
      the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `119`
      to the new `EntityModelKind::Sniffer`, replacing the former cow-quadruped approximation (the
      sniffer no longer borrows the `CowModel`). The static rest-pose hierarchy is emitted directly
      (atlas 192×192): the `bone` part at `offset(0, 5, 0)` parents the body (the 25×29×40 trunk, a
      25×24×40 inner block inflated by `CubeDeformation(0.5)` — geometry `min -= 0.5`, `size += 1`,
      baked into the colored cube exactly like the vex/illager deformed cubes — and the 25×0×40
      belly plane) and the six 7×10×8 legs; the body parents the head (13×18×11 skull + top plane)
      which parents the two 1×19×7 ears, the 13×2×9 nose pad, and the 13×12×9 lower beak — fifteen
      cubes. Every `SnifferModel.setupAnim` animation is deferred: the head look (`head.xRot/yRot =
      state.xRot/yRot`), the search/walk (`applyWalk`), and the dig / long-sniff / stand-up / happy
      / scenting keyframe animations. The texture-backed path remains unsupported (this is a
      colored-first slice; the colored debug path approximates the body with one brown tint and the
      nose pad with a pink tint)
    - warden entities as renderer-owned vanilla 26.1 `WardenModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `142` to the
      new `EntityModelKind::Warden`, replacing the former placeholder bounds box. The static rest-pose
      hierarchy is emitted directly (atlas 128×128): the `bone` part at `offset(0, 24, 0)` parents the
      body (the 18×21×11 torso) and the two 6×13×6 legs (differing only in X origin, ±5.9); the body
      parents the two 9×21×0 ribcage planes, the head (16×16×10 skull) and the two mirrored 8×28×8
      arms; the head parents the two 16×16×0 tendril planes — ten cubes. Every `WardenModel.setupAnim`
      animation is deferred: the head look (`head.xRot/yRot`), the walk (`animateWalk`), the idle-pose
      wobble (`animateIdlePose`), the tendril sway (`animateTendrils`), and the attack / sonic-boom /
      digging / emerging / roar / sniff keyframe animations. The four emissive overlay layers
      (`WardenEmissiveLayer` for the tendrils, heart, bioluminescent spots, and pulsating spots, each
      keyed off the danger/heartbeat/anger animation state) are deferred. The texture-backed path
      remains unsupported (this is a colored-first slice; the colored debug path approximates the body
      with one dark-teal tint and the tendrils with a brighter cyan tint)
    - armadillo entities as renderer-owned vanilla 26.1 `AdultArmadilloModel` /
      `BabyArmadilloModel.createBodyLayer()` geometry on the colored path: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `4` to the new `EntityModelKind::Armadillo { baby }`,
      replacing the former placeholder bounds box. The synced `AgeableMob.DATA_BABY_ID` flag (entity-data
      index 16, defaulting to adult) selects the baby body layer, matching the vanilla
      `AgeableMobRenderer` two-model dispatch. Both static rest-pose hierarchies are emitted directly
      (atlas 64×64): the root parents the body and the four legs directly (no wrapping bone); the body
      parents the tail and the head, and the head parents the head cube and the two ear planes. The
      adult body is a `CubeDeformation(0.3)`-inflated 8×8×12 shell over the bare box (baked into the
      colored cube exactly like the vex/illager/sniffer deformed cubes), with a 1×6×1 tail, a 3×5×2
      head snout, two 2×5×0 ears, and four 2×3×2 legs — ten cubes; the baby is the smaller 5×4×7 / 5×4×6
      shell with a 1×1×4 tail stub, a 2×2×4 snout, two 2×3×0 ears parented to the head cube, and four
      2×2×2 legs (front legs at vanilla's swapped X origins) — ten cubes. Every `ArmadilloModel.setupAnim`
      animation is deferred: the clamped head look (`head.xRot/yRot`), the `applyWalk` leg sway, and the
      roll-out / roll-up / peek keyframe animations. The shell-ball `cube` part and the `isHidingInShell`
      visibility swap (which hides the body/legs/tail and shows the 10×10×10 ball) are deferred entity-side
      state, so the non-hiding rest pose is emitted. The texture-backed path remains unsupported (this is a
      colored-first slice; the colored debug path approximates the armored body/legs with one brown tint and
      the soft head/ears/tail with a tan tint)
    - axolotl entities as renderer-owned vanilla 26.1 `AdultAxolotlModel` /
      `BabyAxolotlModel.createBodyLayer()` geometry on the colored path: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `7` to the new `EntityModelKind::Axolotl { baby }`,
      replacing the former placeholder bounds box. The synced `AgeableMob.DATA_BABY_ID` flag (entity-data
      index 16, defaulting to adult) selects the baby body layer, matching the vanilla
      `AgeableMobRenderer` two-model dispatch (the `0.5F` constructor argument is the shadow radius, not a
      scale, so the baby uses its own 32×32 geometry rather than a scaled adult). Both static rest-pose
      hierarchies are emitted directly: the adult (atlas 64×64) body (an 8×4×10 trunk plus a 0×5×9 dorsal
      fin) parents the head (an 8×5×5 skull, `CubeDeformation(0.001)` fudge baked into the colored cubes
      exactly like the other deformed cubes) which parents the three gill planes (8×3×0 top, two 3×7×0
      side frills), the four 3×5×0 leg planes (right/left legs at the -2/-1 origins), and the 0×5×12 tail
      fin — eleven cubes; the baby (atlas 32×32) wraps the body under a `root` bone at (0, 24, 0), with a
      4×2×6 trunk, four 3×0×1 horizontal leg planes (the right hind leg a doubly-rotated pivot/cube pair),
      a 0×3×8 tail, a 6×3×4 head, and the three gill planes — eleven cubes. Every `setupAnim` animation is
      deferred: the body yaw, the adult swimming / water-hovering / ground-crawling / lay-still procedural
      sways, the baby swim / walk / idle keyframe animations, the play-dead pose, and the mirror-leg copy.
      The five `Axolotl.Variant` color variants (lucy / wild / gold / cyan / blue, each with adult and baby
      textures) live on the deferred texture-backed path, so the colored debug path renders the lucy (pink)
      body with one body tint and one gill tint. The texture-backed path remains unsupported (this is a
      colored-first slice)
    - tadpole entities as renderer-owned vanilla 26.1 `TadpoleModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `130` to the new
      `EntityModelKind::Tadpole`, replacing the former placeholder bounds box. The static rest-pose
      hierarchy is emitted directly (atlas 16×16): two sibling root parts — a 3×2×3 body box at
      `offset(0, 22, -3)` and a 0×2×7 tail fin plane at `offset(0, 22, 0)` — two cubes. The only
      `TadpoleModel.setupAnim` motion, the tail yaw sway (`tail.yRot = -amplitude * 0.25 *
      sin(0.3 * ageInTicks)`, amplitude `1.0` in water / `1.5` on land), is deferred. The texture-backed
      path remains unsupported (this is a colored-first slice; the colored debug path approximates the body
      with one dark tint and the tail fin with a lighter tint)
    - parrot entities as renderer-owned vanilla 26.1 `ParrotModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `98` to the new
      `EntityModelKind::Parrot`, replacing the former placeholder bounds box. The static STANDING rest-pose
      hierarchy is emitted directly (atlas 32×32): seven sibling root parts — the 3×6×3 body (pitched
      0.4937 rad), the 3×4×1 tail (pitched 1.015 rad), the two 1×5×3 wings (pitched -0.6981 rad and flipped
      `yRot = -π`), the 2×3×2 head, and the two 1×2×1 legs (pitched -0.0299 rad) — with the head parenting
      the 2×1×4 upper-head block, the two 1×2×1 beak halves, and the 0×5×4 crest feather (pitched -0.2214
      rad) — eleven cubes. Every `ParrotModel.setupAnim` motion is deferred: the head look (`head.xRot/yRot`),
      the per-pose `prepare` offsets (the FLYING leg pitch, the SITTING crouch), the leg/tail walk swing, the
      wing flap (`zRot = ±(0.0873 + flapAngle)`), the body/tail/head flap bob, and the PARTY dance. The five
      `Parrot.Variant` colors (red_blue / blue / green / yellow_blue / gray) live on the deferred
      texture-backed path, so the colored debug path renders one body tint plus a beak tint. The
      texture-backed path remains unsupported (this is a colored-first slice)
    - shulker entities as renderer-owned vanilla 26.1 `ShulkerModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `112` to the new
      `EntityModelKind::Shulker`, replacing the former placeholder bounds box. The static closed rest-pose
      hierarchy is emitted directly (atlas 64×64): three sibling root parts — the 16×12×16 lid and the
      16×8×16 base (both at `offset(0, 24, 0)`), and the 6×6×6 head at `offset(0, 12, 0)` — three cubes.
      The closed pose equals the bind pose: `ShulkerModel.setupAnim` resets the lid to
      `y = 16 + sin((0.5 + peekAmount) * π) * 8`, which is exactly `24` at `peekAmount = 0`, so the peek
      open/close (`lid.setPos` + the `lid.yRot` wobble) and the head look (`head.xRot/yRot`) are deferred.
      The `ShulkerRenderer.setupRotations` attach-face rotation (`attachFace.getOpposite().getRotation()`,
      the identity for a floor shulker) and the `bodyRot + 180` body-yaw inversion read the entity-side
      `attachFace`/yaw state, which the native scene does not yet project, so the floor rest pose is emitted
      (the geometry is exact; only the wall/ceiling attach orientation is deferred). The sixteen dye-color
      variants live on the deferred texture-backed path, so the colored debug path renders a purple shell
      tint plus a yellow head tint. The texture-backed path remains unsupported (this is a colored-first
      slice)
    - wither entities as renderer-owned vanilla 26.1
      `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` geometry on the colored path: the native
      entity scene (`entity_scene.rs`) projects vanilla type id `145` to the new `EntityModelKind::Wither`,
      replacing the former placeholder bounds box. The static bind rest-pose hierarchy is emitted directly
      (atlas 64×64): six sibling root parts — the 20×3×3 shoulders bar, the ribcage (a 3×10×3 spine plus
      three 11×2×2 rib bars, at `offset(-2, 6.9, -0.5)` pitched 0.20420352 rad), the 3×6×3 hanging tail
      (at the bind position `(-2, 6.9 + cos(0.20420352)·10, -0.5 + sin(0.20420352)·10)` pitched 0.83252203
      rad), the 8×8×8 center head, and the two 6×6×6 side heads — nine cubes. Every
      `WitherBossModel.setupAnim` motion is deferred: the procedural ribcage/tail breathing sway
      (`cos(ageInTicks · 0.1)`), the center-head look (`yRot`/`xRot`), and the two side heads' target
      tracking. The `WITHER_ARMOR` invulnerable-shimmer overlay layer (the same mesh re-rendered with
      `INNER_ARMOR_DEFORMATION`) and the texture-backed path are deferred, so the colored debug path renders
      a dark body tint plus a lighter head tint (this is a colored-first slice)
    - giant entities as renderer-owned vanilla 26.1 `GiantZombieModel` geometry on the colored path: the
      native entity scene (`entity_scene.rs`) projects vanilla type id `59` to the new
      `EntityModelKind::Giant`, replacing the former placeholder bounds box. `GiantZombieModel` is the
      standard humanoid (zombie) body layer baked through `humanoidBodyLayer.apply(MeshTransformer.scaling(
      6.0))` (`LayerDefinitions` registers `ModelLayers.GIANT` this way; `EntityRenderers` registers the
      `GiantMobRenderer` with scale `6.0`), so the giant reuses the adult zombie body parts emitted through
      the shared `mesh_transformer_scaled_model_root_transform` at the 6.0 factor — exactly the husk's
      `MeshTransformer` pattern but with the giant's larger factor and no baby variant. The head look and
      the limb swing match the zombie (the giant extracts the same `ZombieRenderState`). The
      `HumanoidArmorLayer`, the `ItemInHandLayer`, and the zombie texture-backed path are deferred (this is a
      colored-first slice; the giant reuses the zombie body tints)
    - end crystal entities as renderer-owned vanilla 26.1 `EndCrystalModel.createBodyLayer()` geometry on
      the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `45` to the new
      `EntityModelKind::EndCrystal`, replacing the former placeholder bounds box. The static rest-pose
      hierarchy is emitted directly (atlas 64×32): the 12×4×12 base slab at the model origin plus the nested
      glass stack at `offset(0, 24, 0)` — the unscaled 8×8×8 `outer_glass`, the `inner_glass` at
      `PartPose.ZERO.withScale(0.875)` (a centred 7×7×7 box), and the core `cube` at the cumulative
      `0.875 · 0.765625 = 0.669921875` scale (a centred 5.359375³ box) — four cubes. Because the three glass
      boxes share the `(0, 24, 0)` centre and the rest pose has no rotation, the per-part `withScale` is
      baked into the centred cube dimensions (a scaled centred cube is a smaller centred cube), exactly
      reproducing the static pose. `EndCrystalRenderer` is a plain `EntityRenderer` (not a
      `LivingEntityRenderer`, so no body-yaw / setup-rotations flip), applying only `scale(2.0)` +
      `translate(0, -0.5, 0)`; this is captured by the dedicated `end_crystal_model_root_transform`. Every
      `EndCrystalModel.setupAnim` motion is deferred — the `outer_glass`/`inner_glass`/`cube` diagonal spin
      (`Axis.YP.rotationDegrees(ageInTicks · 3) · ...`), the `EndCrystalRenderer.getY` vertical bob, the
      `base.visible = showsBottom` toggle (the base is emitted at its default-visible rest), and the
      `submitCrystalBeams` beam to the dragon. The texture-backed path is deferred, so the colored debug path
      renders the magenta glass, the bright core, and the dark base with three tints
    - phantom entities as renderer-owned vanilla 26.1
      `PhantomModel.createBodyLayer()` geometry: the nested body (parenting the tail
      chain, the two mirrored wing chains, and the head) on a 64x64 texture, with the
      vanilla `PhantomRenderer` transform overrides — the `scale(1 + 0.15 * size)`
      and `translate(0, 1.3125, 0.1875)` from the synced `ID_SIZE` (entity-data index
      16, defaulting to 0) and the extra `Axis.XP.rotationDegrees(state.xRot)` body
      pitch; the official `textures/entity/phantom/phantom.png` texture reference,
      texture-backed base layer pass emission, official PNG atlas upload/bind/sample
      path, and the vanilla `PhantomModel.setupAnim` flap (`flapTime = id*3 +
      ageInTicks`; wings `zRot = ±cos(anim)·16°`, tail `xRot = -(5° + cos(2·anim)·5°)`,
      `anim = flapTime · 7.448451 · π/180`, on both render paths) plus the vanilla
      `PhantomEyesLayer` — an emissive `EyesLayer` re-rendering the whole model with
      `textures/entity/phantom/phantom_eyes.png` in the eyes render type. Lighting and
      overlay remain unsupported
    - pufferfish entities as renderer-owned vanilla 26.1
      `PufferfishSmallModel`/`PufferfishMidModel`/`PufferfishBigModel.createBodyLayer()`
      geometry: the small (6-cube), medium (11-cube), and big (13-cube) body layers
      on a 32x32 texture, selected by the synced `PUFF_STATE` int (entity-data index
      17, defaulting to 0; `0` small, `1` medium, `>=2` big, matching
      `PufferfishRenderer.submit`), with the vanilla `PufferfishRenderer.setupRotations`
      vertical bob (`translate(0, cos(ageInTicks · 0.05) · 0.08, 0)`); the official
      `textures/entity/fish/pufferfish.png` texture reference, texture-backed cutout
      emission, official PNG atlas upload/bind/sample path, and the shared vanilla
      `setupAnim` pectoral/blue fin wiggle (`right.zRot = -0.2 + 0.4 · sin(ageInTicks ·
      0.2)`, left negated, set absolutely over the rest pose, on both render paths).
      Lighting and overlay remain unsupported
    - squid and glow squid entities are wired end to end: the native entity scene
      (`entity_scene.rs`) projects vanilla type ids `127` (squid) / `61` (glow squid)
      to the real `SquidModel` (the glow variant keyed off the type id and the baby flag
      off the synced `AgeableMob.DATA_BABY_ID`), replacing the former placeholder boxes.
      Renderer-owned vanilla 26.1
      `SquidModel.createBodyLayer()` geometry: the `CubeDeformation(0.02)` 12x16x12
      body plus the procedural ring of eight `texOffs(48, 0)` 2x18x2 tentacles, each
      placed at `(cos(i·2π/8)·5, 15, sin(i·2π/8)·5)` and yawed `-i·2π/8 + π/2`, on a
      64x32 texture; the `SquidModel.setupAnim` tentacle sweep
      (`tentacle.xRot = tentacleAngle` on all eight, from the lerped
      `SquidRenderState.tentacleAngle`), the `BABY_TRANSFORMER`
      (`MeshTransformer.scaling(0.5)`) baby body layer, the glow-squid variant, and
      the full `SquidRenderer.setupRotations` (the `0.5/1.2` adult, `0.25/0.6` baby
      body translate, the `180 - bodyRot` yaw, and the swim body tilt
      `Axis.XP.rotationDegrees(xBodyRot)` pitch then `Axis.YP.rotationDegrees(zBodyRot)`
      roll, both lerped into the render state), with no death tip-over (the squid
      override replaces `LivingEntityRenderer.setupRotations`); the official squid/glow_squid adult
      (64x32) / baby (32x32) texture references, the hand-emitted texture-backed
      render path over the procedural ring, and the official PNG atlas
      upload/bind/sample path (colored and textured). The glow-squid emissive overlay
      and `GlowSquidRenderer` darken-ticks light boost, the entity-side movement
      projection that populates `xBodyRot`/`zBodyRot`/`tentacleAngle` (the renderer
      consumes them but the `Squid.aiStep` swim integration is not yet projected),
      lighting, and overlay remain unsupported
    - cod entities are wired end to end: the native entity scene (`entity_scene.rs`)
      projects vanilla type id `27` to the real `CodModel`, replacing the former
      placeholder box. Renderer-owned vanilla 26.1 `CodModel.createBodyLayer()`
      geometry: the seven-part body/head/nose/two side fins (`zRot ±π/4`)/tail
      fin/top fin layer (the side, tail, and top fins are zero-thickness planes) on a
      32x32 texture; the `CodModel.setupAnim` tail-fin sway
      (`tailFin.yRot = -amplitude · 0.45 · sin(0.6 · ageInTicks)`, amplitude `1.0`
      in water / `1.5` out), and the full `CodRenderer.setupRotations` (the standard
      body yaw plus the swim wiggle `Axis.YP.rotationDegrees(4.3 · sin(0.6 ·
      ageInTicks))` and the out-of-water flop `translate(0.1, 0.1, -0.1)` +
      `Axis.ZP.rotationDegrees(90)`), both reading the projected `in_water`
      render-state flag (`Entity.isInWater()`, computed per frame as the vanilla
      `wasTouchingWater` AABB-vs-water overlap) and the projected `age_in_ticks`;
      the official `textures/entity/fish/cod.png` texture reference, texture-backed
      base layer pass emission (the top fin keeps its negative `texOffs(20, -6)` V
      origin), and the official PNG atlas upload/bind/sample path (colored and
      textured). Lighting and overlay remain unsupported
    - salmon entities are wired end to end: the native entity scene (`entity_scene.rs`)
      projects vanilla type id `110` to the real `SalmonModel`, decoding the synced
      `Salmon.Variant` size (`DATA_TYPE`, index 17, ids `0/1/2` clamped, default MEDIUM)
      to the small/medium/large body layer, replacing the former placeholder box.
      Renderer-owned vanilla 26.1
      `SalmonModel.createBodyLayer()` geometry: the five-part body-front (carrying a
      flat top fin) / body-back (carrying the flat tail fin and a flat rear top fin) /
      head / two side fins (`zRot ±π/4`) layer (the tail, top, and side fins are
      zero-thickness planes) on a 32x32 texture; the `SalmonModel.setupAnim` back-body
      sway (`bodyBack.yRot = -amplitude · 0.25 · sin(angle · 0.6 · ageInTicks)`) which
      carries the tail + rear top fin subtree, the small/medium/large size variants
      (`Salmon.Variant` ids `0/1/2`, clamped, selecting the
      `MeshTransformer.scaling(0.5/1.0/1.5)` body layer), and the full
      `SalmonRenderer.setupRotations` (the standard body yaw plus the swim wiggle
      `Axis.YP.rotationDegrees(amplitude · 4.3 · sin(angle · 0.6 · ageInTicks))` and the
      out-of-water flop `translate(0.2, 0.1, 0.0)` + `Axis.ZP.rotationDegrees(90)`),
      where `amplitude`/`angle` are `(1.0, 1.0)` in water and `(1.3, 1.7)` out, both
      reading the projected `in_water` render-state flag (`Entity.isInWater()`, the
      vanilla `wasTouchingWater` AABB-vs-water overlap) and the projected `age_in_ticks`;
      the official `textures/entity/fish/salmon.png` texture reference, texture-backed
      base-layer pass emission (per-size `ModelLayers.SALMON`/`SALMON_SMALL`/`SALMON_LARGE`
      keys, the right fin keeping its negative `texOffs(-4, 0)` U origin), and the official
      PNG atlas upload/bind/sample path (colored and textured). Lighting and overlay
      remain unsupported
    - tropical fish entities are wired end to end: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `136` to the real tropical fish model,
      decoding the body shape from the synced packed variant (`DATA_ID_TYPE_VARIANT`,
      index 17): `TropicalFish.getPattern(packed & 0xFFFF).base()` selects the kob-style
      small or flopper-style large body (the default packed `0` = KOB/white/white is the
      small body), replacing the former placeholder box. Renderer-owned vanilla 26.1
      `TropicalFishSmallModel`/`TropicalFishLargeModel.createBodyLayer(CubeDeformation.NONE)`
      geometry: the kob-style small body (five-part body / tail / two side fins
      (`yRot ±π/4`) / top fin) and the flopper-style large body (the same plus a sixth
      bottom fin), where the tail, top, and bottom fins are zero-thickness planes and the
      shape is selected by `TropicalFish.Pattern.base()` (`SMALL=0`/`LARGE=1`); the
      `TropicalFish{Small,Large}Model.setupAnim` tail sway (`tail.yRot = -amplitude · 0.45
      · sin(0.6 · ageInTicks)`, amplitude `1.0` in water / `1.5` out, identical to the cod
      tail), and the full `TropicalFishRenderer.setupRotations` (the standard body yaw plus
      the swim wiggle `Axis.YP.rotationDegrees(4.3 · sin(0.6 · ageInTicks))` and the
      out-of-water flop `translate(0.2, 0.1, 0.0)` + `Axis.ZP.rotationDegrees(90)`), both
      reading the projected `in_water` render-state flag and `age_in_ticks`; the official
      `textures/entity/fish/tropical_a.png` (small) / `tropical_b.png` (large) base
      texture references, per-shape texture-backed base-layer pass emission
      (`ModelLayers.TROPICAL_FISH_{SMALL,LARGE}` keys, the tail/top fins keeping their
      negative `texOffs` V origins), and the official PNG atlas upload/bind/sample path
      (colored and textured); the per-entity base body tint
      (`TropicalFishRenderer.getModelTint = getBaseColor().getTextureDiffuseColor()`, the
      base dye decoded `DyeColor.byId((packedVariant >> 16) & 0xFF)` from the synced packed
      variant and applied as the body color on both render paths — the grayscale base
      texture is multiplied by the base dye's diffuse color instead of left white); and the
      full `TropicalFishPatternLayer` overlay: the twelve patterns
      (`TropicalFish.Pattern.byId(packedVariant & 0xFFFF)`, sparse-decoded to KOB on an
      unknown id) each select one of the official `tropical_{a,b}_pattern_{1..6}.png`
      textures, drawn over the body as a second cutout pass on the
      `ModelLayers.TROPICAL_FISH_{SMALL,LARGE}_PATTERN` geometry (the body mesh inflated by
      `LayerDefinitions.FISH_PATTERN_DEFORMATION = CubeDeformation(0.008)`, keeping the base
      box for UVs) and tinted by the pattern color (`getPatternColor().getTextureDiffuseColor()`,
      `DyeColor.byId((packedVariant >> 24) & 0xFF)`). Only the colored debug path omits the
      pattern overlay (a cutout texture whose shape comes from the texture alpha cannot be
      approximated by a solid-color box); its lighting/overlay remain the standard deferred
      entity lighting
    - minecart entities as renderer-owned vanilla 26.1
      `MinecartModel.createBodyLayer()` geometry: the `texOffs(0, 10)` 20x16x2 floor
      panel laid flat plus the four `texOffs(0, 0)` 16x8x2 wall panels boxed in, on a
      64x32 texture; the official `textures/entity/minecart/minecart.png` texture
      reference, texture-backed cutout emission, official PNG atlas upload/bind/sample
      path, and the static `MinecartModel` (no `setupAnim`) shared by both render
      paths. The `AbstractMinecartRenderer` rail-follow transform (along-track
      position lerp, slope tilt, hover, the TNT/spawner `displayOffset` and 0.75x
      block-content scale), the chest/furnace/hopper/command-block/TNT/spawner content
      models, lighting, and overlay remain unsupported
    - every vanilla 26.1 entity type id `0..=156` maps to a deterministic
      renderer model key; unknown future ids use an explicit
      `todo_unknown_entity_type_bounds` placeholder
    - primitive renderer-owned model families for humanoids and quadrupeds, plus
      named placeholder bounds for remaining entity types
  - Backend GPU resources stay outside `WorldStore`.
  - Full entity presentation remains phase 6 work, including texture assets,
    variants, equipment, skins, animation, lighting, custom/datapack cow/pig
    variant asset presentation, pig saddle presentation, sheep
    head-look-pitch presentation,
    wolf variant/armor/wet-tint/pose presentation,
    boat/raft paddle animation, damage roll, bubble wobble, and water-mask
    presentation,
    horse variant/markings/saddle/armor/animation, donkey/mule saddle and
    animation presentation, undead horse body-armor/saddle/animation
    presentation, and remaining non-base-equine presentation,
    villager profession/type/held-item/custom-head presentation,
    illager held-item/custom-head/arm-pose presentation, zombie-family
    armor/drowned outer-layer/swim/trident/zombie-villager overlays/no-hat/
    converting-state/piglin-family armor/custom-head/arm-pose/converting-state
    presentation, skeleton armor, held-item, and animation presentation,
    creeper swelling/powered overlays,
    spider walk-animation presentation (the 180-degree death flip is implemented),
    enderman carried-block/creepy
    presentation, iron golem crackiness/flower/animation presentation, and
    snow golem pumpkin/animation presentation, armor stand equipment/custom
    layers/wiggle/marker presentation, slime/magma-cube squish/full
    render-state lighting/sorting presentation, and precise vanilla mesh parity
    for primitive/placeholder entity families.

### Audio Runtime Parity

- Owner: `bbb-audio` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Continue validating audio behavior against vanilla.
  - Keep unit tests independent of an audio device.
  - Cover:
    - source/category mapping
    - spatial/entity-following sounds
    - stop semantics
    - remaining level-event audio that is still coupled to unimplemented
      particle/provider side effects
    - device/runtime diagnostics
- Evidence / boundary:
  - `bbb-audio` has:
    - Kira-backed command/runtime boundaries
    - pack-driven sound lookup
    - native dispatcher playback for fixed-pitch vanilla `LevelEventHandler`
      sounds:
      - dispenser dispense/fail/launch
      - firework rocket shoot
      - chorus flower grow/death
      - brewing stand brew
      - crafter craft/fail
      - end portal frame fill
      - bone meal use
      - honeycomb wax on
    - native dispatcher playback for randomized vanilla `LevelEventHandler`
      sounds using a runtime-local `LegacyRandomSource`-shaped `nextFloat()`:
      - fire extinguish / generic extinguish
      - ghast/blaze/dragon/wither/zombie/skeleton/phantom hostile effects
      - anvil, grindstone, book, smithing table, dripstone, wind charge
      - lava extinguish and redstone torch burnout sounds
      - splash/instant-effect potion break sounds for events `2002` and `2007`
      - dragon fireball explode sound for event `2006` when `data == 1`
      - distance-delayed trial spawner sounds for events `3012`, `3013`,
        `3014`, `3019`, `3020`, and `3021`
      - end gateway spawn and ender dragon growl sounds
      - sculk charge sounds for event `3006`, including the fixed pop branch
        and the randomized charged branch
      - lava extinguish and redstone torch burnout now share the dispatcher
        path with renderer smoke side effects for events `1501` and `1502`
      - cobweb place event `3018` consumes the vanilla poof-particle random
        sequence before recording/playing:
        - `minecraft:block.cobweb.place`
        - `SoundSource.BLOCKS`
        - volume `1.0`
        - pitch `(nextFloat - nextFloat) * 0.2 + 1.0`
        - `distanceDelay=true`
    - native dispatcher and offline probe recording/playback for
      `LevelEventHandler` portal travel local ambience:
      - event `1032`
      - `minecraft:block.portal.travel`
      - ambient source
      - non-spatial local sound command
    - canonical world/probe/native observed audio state for LevelEvent-derived
      positioned sounds:
      - fixed-pitch and randomized `LevelEventHandler` positioned sounds update
        `world.client_audio.last_sound`
      - vanilla `globalLevelEvent` sounds update `world.client_audio.last_sound`
        when a camera pose is available
      - LevelEvent-derived sound recording does not increment Sound-packet
        counters
      - positioned sound state and audio commands preserve vanilla
        `distanceDelay` metadata
      - Kira runtime queues distance-delayed positioned sounds using the
        vanilla `distanceDelay && distance^2 > 100` threshold and
        `floor(distance / 2)` tick delay
    - native dispatcher playback for vanilla `globalLevelEvent` sounds:
      - wither spawn event `1023`
      - ender dragon death event `1028`
      - end portal spawn event `1038`
      - camera-relative sound position two blocks toward the event
    - canonical world/probe/native state for `LevelEventHandler` jukebox
      start/stop:
      - event `1010` records active jukebox song registry id at block position
      - event `1011` stops the active jukebox song at that block position
      - active jukebox songs are cleared with client-level teardown
    - Kira-backed jukebox playback commands for `LevelEventHandler` jukebox
      start/stop:
      - event `1010` resolves vanilla 26.1 `JukeboxSongs.bootstrap` registry
        ids to sound events
      - registry data for `minecraft:jukebox_song` updates the runtime mapping
        for known vanilla song entry ids
      - play commands use `SoundSource.RECORDS`, volume `4.0`, pitch `1.0`,
        and the jukebox block center
      - stop commands target the matching jukebox block position without
        stopping all record sounds
  - Full vanilla playback parity remains phase 7 work.

### Official 26.1 Resource-Pack Coverage

- Owner: `bbb-pack`
- Status: `partial`
- Next action:
  - Keep parser coverage aligned with official assets by running local vanilla
    coverage tests when changing pack loaders:
    - `loads_all_local_vanilla_atlases`
    - `loads_local_vanilla_item_model_catalog`
    - `loads_local_vanilla_item_registry`
    - `loads_local_vanilla_equipment_asset_catalog`
  - Add new parser support only when an official asset or resource-pack
    declaration fails those coverage tests or a focused resource-pack fixture.
  - Advance renderer/runtime consumption of parsed assets:
    - atlas mip animation metadata beyond current static sprite use
    - full item tint parity for dynamic sources in every item rendering path
    - equipment asset layers for future armor/equipment rendering
  - Keep resource-pack precedence/filter tests close to loaders.
- Evidence / boundary:
  - Local vanilla 26.1 parser coverage passes for:
    - atlases
    - item model declarations
    - item tint source declarations
    - item registry declarations
    - equipment assets
  - Loaders still reject malformed or unsupported custom resource-pack
    declarations intentionally.
  - Equipment assets under `assets/minecraft/equipment/*.json` are parsed into
    `EquipmentAssetCatalog`, including:
    - dyeable armor layers
    - `use_player_texture` elytra layers
    - wolf/horse/body armor layer declarations
  - Current audio use is covered by:
    - sounds
    - generated vanilla fallback
    - resource-pack filters

### Bundle Selected-Item Icon State

- Owner: `bbb-protocol` + `bbb-world` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Expand renderer/UI coverage beyond hotbar icon snapshots.
- Evidence / boundary:
  - Vanilla `BundleHasSelectedItem` checks
    `BundleItem.getSelectedItem(itemStack) != null`.
  - `BundleContents.STREAM_CODEC` sends the item template list but not the
    selected index.
  - `bbb-protocol` preserves bundle item-template summaries.
  - `bbb-world` stores the local selected index per inventory/container slot.
  - Control pumping and native bundle mouse helpers:
    - update canonical state
    - queue `ServerboundSelectBundleItemPacket`
    - route mouse-wheel selection through hovered local inventory and
      supported server-opened container slot hit-tests
    - clear selected bundle items before `QUICK_MOVE` and `SWAP`
      container clicks
  - The GUI item icon runtime:
    - evaluates `minecraft:bundle/has_selected_item`
    - resolves `minecraft:bundle/selected_item` from the selected template

### Native Input, Movement, Interaction, Inventory, And Command Flows

- Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`
- Status: `partial`
- Next action:
  - Movement:
    - Extend the current basic AABB collision and gravity/jump slice to cover:
      - remaining vanilla survival physics details beyond the current native
        fixed 20Hz local movement cadence
      - remaining vanilla voxel collision shapes
      - remaining fluid movement work beyond current still water/lava support:
        - sprint-swim camera, animation, and presentation nuance beyond
          canonical swimming pose selection and pitch-coupled vertical travel
      - remaining block-state movement factors beyond the synced frozen-tick
        and local powder-snow contact slowdown/collision slices:
        - remaining powder-snow behavior beyond local player collision:
          - non-player `POWDER_SNOW_WALKABLE_MOBS` entity collision if locally
            controlled non-player entities are added later
          - inside-block particle/extinguish/fall-sound side effects
        - remaining powder-snow `canFreeze` nuance beyond local player:
          - non-player freeze-immune entity type exceptions if locally
            controlled entity freezing is added later
        - remaining no-collision hazard `entityInside` effects:
          - sweet berry bush age-gated damage and fox/bee exceptions if locally
            controlled non-player entities are added later
        - full vanilla post-`Entity.move` `deltaMovement` travel ordering
          beyond the current direct local walking prediction
      - remaining vehicle movement send edge cases beyond the vanilla-shaped
        walking and passenger player packet thresholds
  - Block destroy:
    - Close remaining block destroy profile gaps outside the mechanically
      parsed `Blocks.java` property declarations:
      - constructor-level mutations beyond the covered `InfestedBlock` /
        `InfestedRotatedPillarBlock` host destroy-time rule
      - arbitrary helper/lambda evaluation not covered by the current parser
    - Close remaining vanilla player destroy-speed gaps:
      - validate exact pose/fluid nuance beyond the current standing-eye
        water probe
    - remaining hit effects beyond local block hit/break sounds:
      - block-specific `state.attack` callbacks
      - hit particles
    - full model-shaped crack decals with vanilla crumbling blend/depth-bias
      behavior
    - any remaining `STOP_DESTROY_BLOCK` sequencing gaps
  - Commands:
    - Continue adding focused command queue and encode tests for:
      - inventory
      - interaction
      - chat
      - command
      - sign editing
    - Sign editing follow-up work:
      - renderer presentation for the vanilla sign edit screen
      - clipboard parity for copy/cut/paste editing
    - Gameplay Q/Ctrl+Q drop input now follows vanilla 26.1 modifier
      semantics:
      - Ctrl, not sprint, selects `DROP_ALL_ITEMS`.
      - The selected hotbar stack is locally predicted in canonical inventory
        state.
      - Main-hand swing is queued only when a non-empty stack was dropped.
      - Spectator mode suppresses gameplay drop and swap-offhand actions.
      - Spectator left-click on an entity queues `SpectateEntity`; spectator
        left-click on blocks does not attack, destroy, or swing.
      - Middle-click pick block/entity uses Ctrl, not sprint, for include-data.
      - Spectator hotbar number keys and wheel do not send held-slot packets;
        wheel input adjusts local flying speed when the Spectator GUI menu is
        inactive. Full Spectator GUI selection/menu behavior remains follow-up
        presentation/control work.
      - Spectator mode automatically enables local flying when server-synced
        abilities allow flight, and jump double-tap does not toggle spectator
        flying off.
      - Spectator right-click without a target does not send `UseItem`.
      - Spectator right-click on a block sends main-hand `UseItemOn` even when
        offhand would be preferred for non-spectator item use.
      - Main-hand `attack_range` item components:
        - are decoded from network item patches
        - are parsed from official 26.1 default item declarations
        - participate in crosshair entity selection for extended custom reach
        - use custom min/max reach, creative reach, hitbox margin, and block
          outline clipping before entity hits
        - follow vanilla `AttackRange.getClosesetHit` movement extension by
          adding the positive local movement component along the look vector to
          crosshair entity search reach
        - suppress out-of-range `AttackEntity` packets for the current entity
          hit
  - Inventory:
    - Implement:
      - remaining rich tooltip behavior:
        - non-ASCII font providers
        - bidirectional text shaping
        - official tooltip background/frame sprites
        - italic and complex component styles
        - component-specific detail lines
      - remaining dedicated server-opened menu layouts not already covered by
        these baseline menu families:
        - `generic_9xN`
        - `generic_3x3`
        - anvil
        - beacon
        - brewing stand
        - cartography table
        - enchanting table
        - furnace/blast furnace/smoker
        - crafting table
        - grindstone
        - hopper
        - lectern
        - loom
        - merchant/villager
        - shulker box
        - smithing table
        - stonecutter
      - recipe book/creative variants
      - remaining local crafting result parity for container `0`:
        - composite/component-aware/special recipe display matching beyond the
          current direct-item and crafting-requirement shaped/shapeless 2x2
          subset
        - recipe-specific remainder items
    - Higher-value next slices should prioritize missing player-visible flows
      such as recipe book/creative variants or container `0` crafting result
      parity over additional baseline layout/probe coverage for the menu
      families listed above.
- Evidence / boundary:
  - Movement:
    - Native movement projects world-computed `on_ground` and
      `horizontal_collision` into serverbound move commands.
    - Native unmounted local player movement accumulates elapsed time into
      fixed 20Hz physics steps while preserving per-frame look updates.
    - Serverbound movement packets use vanilla's strict position threshold and
      20-sendPosition-call position reminder semantics; rotation-only and
      status-only packets do not reset the reminder.
    - It clips local player movement with a basic AABB solver against simple
      full-block terrain plus common non-full-block shapes:
      - slab
      - stair
      - door
      - trapdoor
      - fence
      - fence gate
      - bars/pane
      - wall
      - leaves
      - mangrove roots / muddy mangrove roots
      - bamboo stalk
      - chorus plant / chorus flower
      - conduit
      - azalea / flowering azalea
      - snow layer
      - flat carpet
      - pale moss carpet base
      - flowers / bushes / crops / thin ground overlays
      - cactus
      - farmland / dirt path
      - soul sand / mud
      - honey block
      - cake
      - lily pad
      - amethyst cluster / bud
      - flower pot
      - candle / candle cake
      - cocoa
      - shelf
      - sea pickle
      - pointed dripstone
      - skull / wall skull
      - turtle egg
      - sniffer egg
      - dried ghast
      - chain
      - ladder
      - rod
      - campfire
      - fire / redstone wire / tripwire
      - copper grate
      - piston / sticky piston
      - piston head
      - big dripleaf leaf
      - end portal frame
      - daylight detector
      - sculk sensor / calibrated sculk sensor / sculk shrieker
      - heavy core
      - copper golem statue
      - dragon egg
      - decorated pot
      - rail / powered rail / detector rail / activator rail
      - torch / wall torch
      - lever / button
      - sign / hanging sign
      - banner / wall banner
      - nether portal plane
      - end portal / end gateway
      - structure void / light
      - chest
      - bed
      - cauldron
      - hopper
      - composter
      - lectern
      - grindstone
      - brewing stand
      - bell
      - enchanting table
      - stonecutter
      - anvil
      - `moving_piston` without a canonical moving-piston block entity is
        treated as empty, matching vanilla's empty fallback when no block entity
        supplies dynamic collision.
      - `big_dripleaf_stem` is treated as empty, matching vanilla
        `noCollision`; `big_dripleaf` leaf collision follows vanilla
        `tilt=none|unstable|partial|full` heights.
      - `pale_moss_carpet` follows vanilla `MossyCarpetBlock` collision:
        `bottom=true` uses the default 1px carpet base and `bottom=false`
        is empty even when side outline faces are present.
      - `bamboo` follows vanilla `BambooStalkBlock` 3px offset collision
        column; `bamboo_sapling` is empty, matching its vanilla
        `noCollision` registration.
      - `mangrove_roots` and `muddy_mangrove_roots` use vanilla default
        full-block collision even though their render material is cutout.
      - `chorus_plant` follows vanilla `PipeBlock(10.0F)` center cube plus
        connected six-direction arms; `chorus_flower` uses vanilla default
        full-block collision.
      - `conduit` follows vanilla `Block.cube(6.0)` collision.
      - `azalea` and `flowering_azalea` follow vanilla `AzaleaBlock` collision:
        an 8px-tall full-width crown plus a 4px center stem.
      - `cocoa` follows vanilla `CocoaBlock` age-scaled wall-pod collision:
        age `0..2` uses a north-facing baseline at z `1..5/7/9` px and is
        rotated by the `facing` property.
      - `shelf` follows vanilla `ShelfBlock` collision:
        a horizontal-facing backboard plus top and bottom lips from
        `Shapes.rotateHorizontal`.
      - `amethyst_cluster`, `large_amethyst_bud`, `medium_amethyst_bud`, and
        `small_amethyst_bud` follow vanilla `AmethystClusterBlock` collision:
        `Block.boxZ(width, 16 - height, 16)` rotated with `Shapes.rotateAll`.
      - `copper_golem_statue` variants follow vanilla
        `CopperGolemStatueBlock` collision: fixed
        `Block.column(10.0, 0.0, 14.0)` independent of pose, facing,
        waterlogged, weathered, or waxed state.
      - common vanilla `.noCollision()` vegetation, crops, thin ground
        overlays, fire, redstone wire, and tripwire are classified as cutout
        non-blocking terrain for local movement. Covered examples include:
        - ordinary flowers and dry vegetation
        - bushes, fungi, sprouts, propagules, and dripleaf stems
        - wheat, carrots, potatoes, beetroots, nether wart, stems, and
          sugar cane
        - pink petals, leaf litter, sculk vein, glow lichen, resin clump, and
          frogspawn
      - standing signs, wall signs, ceiling hanging signs, and wall hanging
        signs follow vanilla `.noCollision()` registration; their outline
        shapes remain renderer/crosshair selection data only.
      - standing banners and wall banners follow vanilla `.noCollision()`
        registration; their banner pole/cloth outline remains selection data
        and does not block local movement.
      - Rails, torches, wall torches, levers, and buttons follow vanilla
        `.noCollision()` registration; their outline shapes remain
        renderer/crosshair selection data only.
      - `nether_portal` is translucent terrain and follows vanilla
        `.noCollision()` local movement. Portal travel remains server/runtime
        behavior driven by entity-inside semantics, not terrain collision.
      - `end_portal` and `end_gateway` are invisible portal blocks and follow
        vanilla `.noCollision()` local movement. End portal/gateway travel,
        smoke/portal particles, and gateway cooldown remain server/runtime or
        entity-inside behavior, not terrain collision.
      - `structure_void` and `light` are invisible non-blocking blocks for local
        movement. `barrier` remains a full collision block.
      - `cobweb` and `sweet_berry_bush` use vanilla `noCollision`
        registration so they do not block local movement.
      - `cobweb` applies vanilla `entityInside` stuck movement multipliers:
        normal players use `(0.25, 0.05, 0.25)` and players with the synced
        Weaving effect use `(0.5, 0.25, 0.5)`.
      - `sweet_berry_bush` applies vanilla local-player stuck movement
        multiplier `(0.8, 0.75, 0.8)`; server-authored damage remains
        authoritative.
    - It applies:
      - synced local player `gravity` attribute id `14` and basic gravity
      - synced local player `NoGravity` entity metadata data id `5`, which
        suppresses local gravity in air, water, and lava travel
      - synced local player `TicksFrozen` entity metadata data id `7` as the
        vanilla `minecraft:powder_snow` movement-speed `ADD_VALUE` modifier:
        - amount is `-0.05 * min(ticks_frozen, 140) / 140`
        - skipped when the server already syncs the `minecraft:powder_snow`
          movement-speed modifier
      - local player in-world `minecraft:powder_snow` contact updates synced
        `TicksFrozen` metadata:
        - increments by `1` per local physics step while inside powder snow
        - thaws by `2` per local physics step outside powder snow
        - clamps to `0..140`
        - respects vanilla local player `canFreeze` immunity:
          - spectator mode thaws instead of freezing
          - armor/body items in the official
            `minecraft:freeze_immune_wearables` item tag thaw instead of
            freezing
      - vanilla block speed factor for local walking prediction:
        - `minecraft:soul_sand` and `minecraft:honey_block` apply `0.4`
        - synced local player `movement_efficiency` attribute id `21`
          interpolates the block factor back toward `1.0`
        - `minecraft:water` and `minecraft:bubble_column` at the current
          player block position do not fall back to the block below
      - vanilla block jump factor for local jumps:
        - `minecraft:honey_block` applies `0.5` to the base jump strength
        - Jump Boost remains additive after the block jump factor
      - vanilla slime block landing bounce:
        - downward collision reverses local player Y velocity
        - sneak input suppresses the bounce
      - basic vanilla climbable movement for ladder and vine-family blocks:
        - resets fall distance while inside the climbable block
        - clamps horizontal velocity to `0.15` per tick
        - clamps downward velocity to `-0.15` per tick
        - sneak input suppresses non-scaffolding climbable sliding
        - jump or horizontal collision applies the vanilla upward climb velocity
      - scaffolding climbable movement and context-sensitive collision:
        - counts as climbable for fall-distance reset and velocity clamps
        - sneak input does not suppress downward slide
        - side entry no longer treats scaffolding as a full block
        - stable top shape supports the player when above and not descending
        - sneak/descending input bypasses the stable top shape
        - unstable bottom support shape applies for bottom scaffolding with
          non-zero distance
      - vanilla local player powder snow context-sensitive collision:
        - ordinary local player movement sinks through `minecraft:powder_snow`
        - feet slot item must be the injected official
          `minecraft:leather_boots` protocol id to stand on top
        - freeze-immune wearable items do not imply powder-snow walkability
        - sneak/descending input bypasses the top collision
        - `fall_distance > 2.5` uses the vanilla full-width `0.9`-high falling
          collision shape
        - leather boots plus jump or horizontal collision while inside powder
          snow applies vanilla `0.2` climb-out Y velocity before gravity/friction
      - vanilla open trapdoor-as-ladder climbable behavior when:
        - the current block is an open trapdoor
        - the block below is a ladder
        - both blocks have the same horizontal facing
      - jumps only from ground
      - vanilla sprint jump horizontal impulse:
        - adds `0.2` horizontal velocity in the local yaw forward direction
        - uses the same effective sprint predicate as local movement
      - local player `movement_speed` / `sneaking_speed` attributes with the
        vanilla default sneaking-speed reduction
      - synced local player Speed effect id `0` and Slowness effect id `1`
        as `movement_speed` `ADD_MULTIPLIED_TOTAL` modifiers scaled by
        `amplifier + 1`
      - synced local player Jump Boost effect id `7` as a jump impulse bonus
        of `0.1 * (amplifier + 1)`
      - synced local player Slow Falling effect id `27` as the vanilla falling
        gravity clamp
      - synced local player Levitation effect id `24` as the vanilla vertical
        velocity target
      - synced local player Blindness effect id `14` as the vanilla mobility
        restriction that prevents sprinting
      - vanilla local sprint eligibility for player food and forward impulse:
        - requires food level above `6`
        - treats synced `mayfly` / `can_fly` abilities as enough food
        - suppresses local sprint speed, sprint-swim pose, and sprinting fluid
          drag when sprinting is not eligible
      - vanilla passenger sprint eligibility for local-authoritative vehicles:
        - camel and camel_husk can sprint
        - horse-family and boat mounts cannot sprint
        - the local player must be the controlling passenger
        - player food is ignored in the mounted vehicle branch
      - abilities-driven flying movement with no ordinary gravity
      - vanilla flying horizontal input:
        - uses synced abilities `flying_speed` as per-tick acceleration
        - doubles acceleration while sprinting
        - applies vanilla `0.91` post-move air drag to horizontal velocity
      - jump/sneak vertical controls while flying
      - vanilla 0.6 Y-velocity damping while flying
      - vanilla default 0.6 step-up onto bottom slabs/stairs and low ground
        shapes without auto-stepping full blocks
      - a basic vanilla-shaped sneak edge backoff
      - the vanilla near-ground `fallDistance < maxUpStep` branch of sneak edge
        backoff, backed by canonical local pose `fall_distance`
      - canonical local player `sneaking` pose state when:
        - focused sneak is active and the player is not flying
        - a low ceiling blocks standing bounds but allows crouching bounds
      - canonical local player `swimming` visual pose state when:
        - sprinting while underwater in water contact
        - standing and crouching bounds are blocked but swimming bounds fit
      - vanilla-shaped sprint-swim vertical travel:
        - adjusts water swimming Y velocity toward `getLookAngle().y`
        - uses approach `0.085` when looking down below `-0.2`
        - otherwise uses approach `0.06`
        - upward look only auto-rises while jumping or when fluid remains at
          the vanilla `y + 0.9` swim-head probe
      - vanilla local player standing/crouching/swimming eye heights:
        - standing `1.62`
        - crouching `1.27`
        - swimming/crawling `0.4`
      - vanilla local player standing/crouching/swimming body heights:
        - standing `1.8`
        - crouching `1.5`
        - swimming/crawling `0.6`
      - local player collision, step-up, support probing, fluid body contact,
        fluid jump-out clearance, and bubble-column contact all consume the
        canonical local body height
      - local player camera pose, audio listener, crosshair ray, fluid eye probing,
        and `LookAt` eye anchor all consume the canonical local eye height
      - vanilla-shaped local player fluid contact probing for water/lava
        height and eye-in-fluid checks:
        - `FlowingFluid` height uses `amount / 9.0`
        - same-kind fluid above makes the lower fluid column full height
        - the local player fluid interaction box is deflated by `0.001`
      - water contact resets canonical local player `fall_distance`
      - basic still-fluid travel for local players affected by fluids:
        - water and lava `moveRelative(0.02)` acceleration
        - water drag `0.8`
        - sprinting-water drag `0.9`
        - lava drag `0.5`
        - liquid jump impulse `+0.04`
        - water sneak descent impulse `-0.04`
        - vanilla-shaped fluid gravity scaling
        - creative/spectator flying movement ignores fluid travel
        - water movement efficiency attribute id `32`:
          - interpolates water input speed toward local player movement speed
          - interpolates horizontal water drag toward `0.54600006`
          - applies at half strength when not on ground
        - Dolphin's Grace effect id `29` overrides horizontal water drag to
          `0.96`
        - water/lava `jumpOutOfFluid(oldY)` collision assist:
          - checks post-move `horizontalCollision`
          - probes the current player bounds offset by:
            `delta_movement.y + 0.6 - current_y + old_y`
          - requires the probed bounds to be free of block collision and any
            fluid block
          - sets vertical velocity to `0.3` when clear
        - `minecraft:bubble_column` local player velocity:
          - reads vanilla `drag` block-state property
          - inside upward columns add `0.06` Y velocity capped at `0.7`
          - above upward columns add `0.1` Y velocity capped at `1.8`
          - inside drag-down columns subtract `0.03` Y velocity capped at
            `-0.3`
          - above drag-down columns subtract `0.03` Y velocity capped at
            `-0.9`
          - players with flying abilities active ignore the bubble-column
            push, matching vanilla `Player` overrides
      - vanilla-shaped flow current push for local players affected by fluids:
        - `FlowingFluid.getFlow` horizontal own-height gradients
        - empty-neighbor falling-hole lookup with the vanilla `8/9` offset
        - falling-fluid downward current against solid neighbor faces
        - per-fluid accumulated current averaged for players
        - low contact height `< 0.4` current scaling
        - water current scale `0.014`
        - normal lava current scale `0.0023333333333333335`
        - Nether fast-lava current scale `0.007`
        - vanilla minimum current push `0.0045` when horizontal velocity is
          below `0.003`
      - vanilla-shaped walking player movement packet thresholds:
        - position delta squared greater than `(2.0E-4)^2`
        - rotation/status-only packets when only:
          - look changes
          - collision flags change
        - 20 tick position reminder that resets only when a position packet is
          sent
      - vanilla-shaped passenger `MovePlayer.Rot` packets while mounted:
        - sent on the local movement tick even when look did not change
        - forced to the Rot-only packet variant instead of position/status
          variants
  - Commands:
    - Native sprint command queuing is derived from the same world-owned
      effective sprint predicate used by local movement:
      - forward impulse is required before `START_SPRINTING`
      - vanilla default 7 tick double-tap-forward sprint window is supported
      - low food suppresses unmounted `START_SPRINTING`
      - camel and camel_husk mounts can queue `START_SPRINTING` while horse and
        boat mounts only queue raw `PlayerInput`
      - releasing focus or input queues `STOP_SPRINTING` when effective sprint
        was active
    - Existing input modules queue many serverbound packets, including
      vanilla-shaped boat/raft paddle-state packets from local mounted input.
    - They queue `START_RIDING_JUMP` player commands for vanilla
      `PlayerRideableJumping` vehicle types using:
      - the 26.1 charge scale
      - jump release, including focus/UI release cleanup
    - They toggle local creative/spectator-style flying with the vanilla
      double-jump window when synced abilities allow flight, then queue
      `ServerboundPlayerAbilitiesPacket`.
    - Creative middle-click clone in local and server-opened containers queues
      `ServerboundContainerClickPacket` with `ContainerInput.CLONE`, an empty
      changed-slot set, and a carried item copied to the vanilla max stack
      size.
    - Control/native can queue `ServerboundSetCreativeModeSlotPacket` for:
      - creative inventory slot updates
      - creative drop requests
      - empty or componentless item stacks
      Full item component payload encoding remains follow-up before
      component-rich creative stacks can be sent.
    - They queue `START_FALL_FLYING` player commands when an airborne local
      player has an elytra-equipped chest slot.
    - They queue `STOP_SLEEPING` player commands when wake-up input is pressed
      while the local player entity has sleeping pose metadata.
    - They queue `OPEN_INVENTORY` player commands instead of opening local
      inventory when the local player is riding a vanilla
      `HasCustomInventoryScreen` vehicle:
      - `AbstractHorse`
      - `AbstractNautilus`
      - `AbstractChestBoat`
    - Block-target right-click queues `ServerboundUseItemOnPacket` with:
      - main hand when the selected hotbar slot is non-empty
      - offhand when the selected hotbar slot is empty and offhand has an item
    - Chat entry paths:
      - send offline unsigned `ServerboundChatPacket` messages
      - request `ServerboundCommandSuggestionPacket` completions:
        - with the leading slash
        - while typing slash commands
        - after middle-of-line edits that change a slash command
      - submit `ServerboundChatCommandPacket` payloads without:
        - the leading slash
      - edit chat and command text before submit with:
        - left/right/home/end cursor movement
        - delete/backspace
        - Ctrl+left/right word movement
        - Ctrl+delete/backspace word deletion
        - Ctrl+A selection replacement before chat or slash-command submit
      - queue explicit `ServerboundClientCommandPacket` commands:
        - perform-respawn
        - request-stats
        - request-game-rule-values
        - from native/control input
      - avoid auto-respawning on dead health
    - Sign editor input paths:
      - open from `ClientboundOpenSignEditorPacket`
      - initialize from canonical sign block entity front/back text when
        available
      - edit four pending lines
      - move the line cursor with left/right/home/end and edit at the cursor
        with text input/backspace/delete
      - Ctrl+A selects the current line, and text/backspace/delete replace or
        clear the selected range
      - cycle lines with vanilla-shaped up/down/confirmation keys
      - close by queueing `ServerboundSignUpdatePacket`
      - release cursor capture and suppress gameplay mouse input while open
  - Inventory:
    - Native opens the ordinary local inventory as container `0`.
    - While the local inventory is open, it:
      - releases cursor capture
      - closes with E/Esc by queueing `ServerboundContainerClosePacket(0)`
    - For container `0`, it:
      - renders the centered vanilla survival inventory background with item
        icons and slot hover highlights
      - hit-tests the fixed slot layout
      - routes left/right pickup and outside-drop clicks through a basic local
        `PICKUP` simulation
      - routes Shift-click slots through a local `QUICK_MOVE` simulation for:
        - armor/offhand auto-equip from official item default equipment slots
        - main-inventory/hotbar/container-zero ranges
        - repeated crafting result movement while the result stack remains the
          same item/components and the original input slots can still be
          consumed
        - result-slot prediction only when default crafting remainders are
          known and involved inputs have no recipe-specific crafting remainder;
          default item crafting remainders are locally placed back into the
          crafting grid or visible player inventory, otherwise the click is
          queued server-authoritatively without mutating the local snapshot
        - local 2x2 input clicks recompute the result slot from server-authored
          recipe-book shaped/shapeless displays when the result is a direct
          item or item-stack display and inputs are direct items or
          `crafting_requirements` holder sets, including item tags and mirrored
          shaped patterns
        - vanilla-shaped result-to-player transfer order
      - routes hovered-slot Q/Ctrl+Q through a basic local `THROW` simulation
      - routes hovered-slot number/F keys through a basic local `SWAP`
        simulation
      - routes rapid same-slot left double-clicks through a basic local
        `PICKUP_ALL` simulation
      - routes local left/right drag distribution through vanilla-shaped
        `QUICK_CRAFT` start/add/end clicks
    - It also:
      - renders stack count labels:
        - for hotbar item icons
        - for local inventory item icons
        - using official 26.1 `font/ascii.png` digit glyphs
        - with vanilla item-count placement
      - renders vanilla-shaped durability bars for item icons with
        `damage` / `max_damage` component summaries:
        - in the hotbar
        - in local inventory screens
        - in supported server-opened screens
      - renders vanilla-shaped cooldown overlays for item icons from canonical
        cooldown groups and client tick progress:
        - in the hotbar
        - in local inventory screens
        - in supported server-opened screens
      - updates cursor/slot state
      - fills `ServerboundContainerClickPacket(0)` changed-slot hashes
      - hashes local container-click stacks with empty/default component
        patches, removed-only component patches, and scalar integer
        component patches (`max_stack_size`, `max_damage`, `damage`, and
        `map_id`), including integer patches plus removed components
      - supports bundle wheel selection on hovered local inventory and
        server-opened container slots
      - clears selected bundle items before local and server-opened
        `QUICK_MOVE` / `SWAP` container clicks
      - projects hovered-slot default item tooltip names from official language
        assets and protocol item ids
      - projects hovered-slot custom tooltip names from decoded
        `custom_name`, `item_name`, and written-book title component summaries
      - projects decoded lore component summaries as basic tooltip lines
      - applies vanilla-shaped base tooltip colors for:
        - rarity-colored item names
        - enchanted rarity upgrades
        - lore dark-purple lines
        - written-book author and generation detail lines
        - unbreakable component detail line
      - renders hovered-slot item tooltip names as basic ASCII text using
        official 26.1 `font/ascii.png` glyphs
    - It renders and hit-tests supported server-opened screens:
      - `generic_9x1` through `generic_9x6` ChestMenu screens with official
        `generic_54.png` background slices
      - `generic_3x3` DispenserMenu screens with official `dispenser.png`
      - CrafterMenu screens with official:
        - `crafter.png` background
        - `container/crafter/disabled_slot`
        - powered/unpowered redstone sprites
      - AnvilMenu screens with official:
        - `anvil.png` background
        - `container/anvil/text_field`
        - `text_field_disabled`
        - error sprites
        - rename text input that queues `ServerboundRenameItemPacket`
        - Ctrl+A selection replacement/clearing in the rename text field
      - BeaconMenu screens with official:
        - `beacon.png` background
        - payment slot layout
        - confirm/cancel button sprites
        - beacon effect button sprites loaded for future effect selection
      - BrewingStandMenu screens with official `brewing_stand.png`
      - CartographyTableMenu screens with official:
        - `cartography_table.png` background
        - result map sprite
        - scaled-map sprite
        - duplicated-map sprites
        - locked-map overlay sprite
        - invalid-transform error sprite
      - CraftingMenu screens with official `crafting_table.png`
      - EnchantmentMenu screens with official:
        - `enchanting_table.png` background
        - empty lapis slot sprite
        - enchantment option slot sprites
        - level cost sprites
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens with official
        furnace-family backgrounds
      - GrindstoneMenu screens with official `grindstone.png` background and
        `container/grindstone/error` overlay sprite
      - HopperMenu screens with official `hopper.png`
      - MountScreenOpen screens with official:
        - `horse.png` background for horse-family mounts
        - `nautilus.png` background for nautilus-family mounts
        - generic mount equipment slot sprite
        - saddle/body empty-slot sprites:
          - `container/slot/saddle`
          - `container/slot/horse_armor`
          - `container/slot/llama_armor`
          - `container/slot/nautilus_armor_inventory`
        - horse chest-slot sprite clipped by `inventoryColumns`
      - LecternMenu screens with official:
        - `book.png` background
        - backward/forward page button sprites
      - LoomMenu screens with official:
        - `loom.png` background
        - empty banner/dye/pattern slot sprites
        - disabled scroller sprite
      - MerchantMenu screens with official:
        - `villager.png` background
        - payment/result slot layout
        - villager trade-list sprites loaded for future trade presentation
      - ShulkerBoxMenu screens with official `shulker_box.png`
      - SmithingMenu screens with official:
        - `smithing.png` background
        - `container/smithing/error` overlay sprite
      - StonecutterMenu screens with official `stonecutter.png`
    - LecternMenu page-button clicks queue vanilla container-button ids:
      - `1` for previous page
      - `2` for next page
    - LecternMenu Done closes the active container.
    - LecternMenu Take Book queues vanilla container-button id `3`.
    - LecternMenu renders the current book page as basic ASCII text from
      decoded writable/written book item components and server page data.
    - `ClientboundOpenBook` opens a canonical active book screen when the
      requested hand holds decoded writable/written book item components.
      The native client renders the book background/page text, handles local
      page buttons and PageUp/PageDown, and closes it with Escape/E/Done
      without queuing container commands.
    - MountScreenOpen opens a server-controlled container and hit-tests:
      - saddle slot
      - body armor slot
      - horse-family chest slots from `inventoryColumns`
      - player inventory slots
      - hotbar slots
    - Mount saddle/body slot rendering and hit-testing respect vanilla
      active-slot conditions from entity type tags plus baby/tame metadata.
    - Mount Shift-click routes locally for:
      - mount-owned slots back to the player inventory with vanilla
        reverse-fill ordering
      - default saddle items into an active saddle slot
      - known default horse/llama/nautilus body armor into matching active
        body slots
      - non-equipment player items into mount chest slots when present
      - player main-inventory/hotbar range movement when no mount slot accepts
        the stack
    - Mount component-patched or otherwise entity-specific equippable
      predicates remain server-authoritative.
    - Mount entity preview rendering remains follow-up presentation work.
    - FurnaceMenu/BlastFurnaceMenu/SmokerMenu screens also render official
      progress sprites:
      - lit-progress
      - burn-progress
    - BrewingStandMenu screens also render official progress sprites:
      - fuel-length
      - brew-progress
      - bubbles
    - Those progress sprites use:
      - canonical `ContainerSetData` values
      - vanilla `AbstractFurnaceMenu` / `BrewingStandScreen` progress formulas
    - It queues basic left/right `PICKUP` container clicks for those supported
      fixed-slot screens.
    - It queues server-authoritative keyboard container clicks for those
      supported fixed-slot screens:
      - hovered-slot Q/Ctrl+Q as `THROW`
      - hovered-slot number keys and F as `SWAP`
    - Opening a server-driven runtime screen, including containers, dialogs,
      sign editors, and book views, releases active movement, item use, and
      block destroy input through the native runtime, matching vanilla
      `Minecraft.setScreen` screen-open input release.
    - MerchantMenu visible trade-row clicks queue
      `ServerboundSelectTradePacket` for the current visible offer window.
    - MerchantMenu mouse wheel input updates the local trade-list scroll offset
      for offer lists longer than seven rows.
    - MerchantMenu scroller drag updates the local trade-list scroll offset for
      offer lists longer than seven rows.
    - MerchantMenu renders the current visible server-provided offer window with:
      - cost/result item icons
      - normal/out-of-stock trade arrows
      - enabled/disabled scroller sprite at the current local scroll offset
      - selected-offer out-of-stock overlay
      - current villager XP bar
    - It also queues Shift-click `QUICK_MOVE` container clicks for:
      - supported generic containers
      - `generic_3x3`
      - AnvilMenu:
        - rename field text renders from native HUD text labels
        - rename input initializes from the default item hover name when item
          runtime assets are available
        - XP cost and `Too Expensive!` labels render from server
          `ContainerSetData` cost plus local game mode / experience state
        - input slots quick-move back to the player inventory with vanilla
          forward-fill ordering
        - player-inventory quick-move into the two input slots is locally
          predicted with vanilla `ItemCombinerMenu` ordering
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits, cost plus local
          experience/creative state allow pickup, the additional slot is empty,
          and the single input is consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when cost plus local experience/creative
          state allow pickup, the additional slot is empty, and the single input
          is consumed to empty.
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - repair/enchantment result prediction
          - repair-cost component updates
          - material/sacrifice consumption
          - XP and cost data side effects
          - blocked or partial result transfers
          - anvil damage side effects
      - BeaconMenu:
        - primary/secondary effect button grid renders official button sprites
          and mob-effect icons.
        - effect buttons track vanilla disabled and selected states from beacon
          level data plus local primary/secondary selection.
        - effect clicks update local primary/secondary selection.
        - confirm/cancel buttons render official sprites.
        - cancel closes the active container.
        - confirm queues `ServerboundSetBeaconPacket` from the current local
          primary/secondary selection when payment and primary selection make
          the vanilla button active, then closes the container.
        - quick-move paths are locally predicted for:
          - vanilla slot ranges
          - single-item payment routing from `minecraft:beacon_payment_items`
          - max-stack-one payment slot behavior
        - confirm locally predicts the single payment-slot consumption and
          primary/secondary effect data update before closing the container.
        - beacon block entity updates and active player effects remain
          server-authoritative.
        - deferred presentation parity work:
          - confirm/cancel hover state
          - effect highlighted hover state
          - labels/tooltips
      - CraftingMenu:
        - non-result slots
        - result-slot primary pickup with an empty cursor and result-slot
          quick-move are locally predicted from the current server result when
          default crafting-remainder metadata is loaded and every non-empty
          crafting-grid input is not one of the known vanilla recipe-specific
          remainder inputs for banner duplication or book cloning. Default item
          crafting remainders are locally placed back into the crafting grid
          when that is sufficient; cases that require adding a remainder to
          hidden player inventory slots remain server-authoritative. Empty-cursor
          primary pickup leaves the current result in place when the same input
          slots remain populated; quick-move repeats while all resulting stacks
          and default remainders stay representable in the active container.
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - blocked or partial result transfers
          - recipe recomputation
          - recipe-specific remainders are locally modeled
      - CrafterMenu grid/player inventory transfers:
        - vanilla slot ranges
        - disabled grid slots from `ContainerSetData`
        - result-preview slot 45 kept server-authoritative until crafter recipe
          preview and crafting side effects are locally modeled
      - EnchantmentMenu:
        - option button clicks queue `ServerboundContainerButtonClickPacket`
          when the server-provided option cost is nonzero
        - option slots render enabled/disabled level sprites and cost text from
          server `ContainerSetData`, lapis slot count, and local experience
        - input and lapis slots quick-move back to the player inventory with
          vanilla reverse-fill ordering
        - player-inventory quick-move routing is locally predicted when
          native item runtime provides the vanilla lapis lazuli protocol id:
          - lapis lazuli stacks route to the lapis slot
          - other item stacks move one item into an empty enchantment input
            slot, matching vanilla slot0 behavior
        - enchantment result side effects remain server-authoritative.
        - deferred presentation parity work:
          - animated book model
          - enchanting glyph text and its disabled/highlight coloring
          - hover tooltips
      - BrewingStandMenu:
        - quick-move paths are locally predicted for:
          - vanilla slot ranges
          - brewing fuel item tag routing
          - default potion/glass-bottle item routing
          - default vanilla ingredient item routing
          - max-stack-one bottle slots
        - remaining brewing parity work:
          - full feature-flag-sensitive `PotionBrewing` bootstrap parity
          - local brew-result stack prediction
          - brew completion side effects
      - CartographyTableMenu:
        - shift-click from the map and additional slots back to the player
          inventory is locally predicted when changed stacks have no component
          hash requirements.
        - player-inventory quick-move paths are locally predicted for:
          - filled-map routing into the map slot when the decoded stack summary
            contains hashable scalar integer component patches such as
            `map_id` for serverbound changed slots
          - paper/map/glass-pane additional input routing when native item
            runtime provides the vanilla protocol ids
          - ordinary player inventory/hotbar range movement
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits and the single map and
          additional inputs are consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when the single map and additional
          inputs are consumed to empty.
        - result-slot and component-hash-unsupported filled-map paths are kept
          server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - component-hash-unsupported result stacks
          - local result stack prediction
          - result recomputation after partial input consumption
          - blocked or partial result transfers
          - take-result sound side effects
        - result-state sprites are projected from:
          - input map id components
          - result-slot map post-processing components
          - known paper/map/glass-pane additional inputs
          - known native map locked/scale state
        - deferred presentation parity work:
          - live map pixels and decorations in the preview area
          - invalid-transform prediction when item runtime or map state is not
            available
      - GrindstoneMenu:
        - input slots back to player inventory
        - default damageable player inventory items route into open input slots
          when the native item registry provides vanilla max-damage ids
        - player inventory/hotbar range movement when both input slots are
          occupied
        - result-slot quick-move into the player inventory is locally
          predicted from the current server-provided result stack when the
          transfer fully fits and the input/result stacks are hashable.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server-provided result stack; the vanilla take path
          clears both input slots while XP and level-event side effects remain
          server-authoritative.
        - component/enchantment-only player-to-input, secondary or carried-cursor
          result-slot pickup, partial/full-inventory result transfers, XP, and
          level-event side effects are kept server-authoritative
          until:
          - component-hashable grindstone item/result prediction
          - full result recomputation parity
      - HopperMenu:
        - quick-move paths are locally predicted for:
          - hopper slots back to player inventory with vanilla reverse-fill
            ordering
          - player inventory/hotbar slots into the hopper with vanilla
            forward-fill ordering
      - LoomMenu:
        - selectable pattern grid renders official pattern button sprites when
          banner and dye slots are populated:
          - 32 vanilla no-item-required patterns when the pattern slot is empty
          - one selectable pattern when a vanilla pattern item is present
        - pattern clicks queue `ServerboundContainerButtonClickPacket` with the
          vanilla selectable pattern index.
        - pattern grid mouse wheel input and scroller drag update the local
          visible pattern window.
        - selected pattern buttons and active/disabled scroller state render
          from native HUD state.
        - quick-move paths are locally predicted for:
          - input slots back to player inventory
          - player inventory/hotbar range movement
          - banner/dye/pattern input routing from vanilla item tags
          - result-slot quick-move into the player inventory when the current
            server result fully fits, the pattern slot is empty, and the single
            banner/dye inputs are consumed to empty
          - result-slot primary pickup with an empty cursor when the current
            server result is used, the pattern slot is empty, and the single
            banner/dye inputs are consumed to empty
        - result-slot paths are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - component-aware selectable banner pattern state
          - result recomputation after partial input consumption
          - pattern-item result quick-move
          - pattern-item result pickup
          - blocked or partial result transfers
          - take-result sound side effects
        - deferred presentation parity work:
          - pattern-item component/tag lists beyond the single-pattern vanilla
            items
          - highlighted pattern buttons
          - banner preview
          - max-pattern error overlay
      - MerchantMenu:
        - payment slots quick-move back to the player inventory
        - player inventory/hotbar quick-move range movement
        - trade-row clicks locally autofill payment slots from
          component-predicate-free `MerchantOffer` costs after returning
          existing payment items to player inventory
        - result-slot quick-move locally predicts selected
          component-predicate-free `MerchantOffer`s when the current result
          matches the selected sell stack, payment slots satisfy the modified
          cost counts in vanilla normal or swapped order, and the result fully
          fits the player inventory; payment remainders are kept and the result
          slot is repopulated when the same selected offer remains in stock and
          payable
        - result-slot primary pickup with an empty cursor locally predicts the
          same component-predicate-free selected-offer payment/remainder path
          and places the sold item on the cursor
        - result-slot secondary pickup, carried-cursor pickup, alternate-offer
          payment remainder recomputation, and component-predicate
          trade-row/payment result prediction are kept server-authoritative
          until:
          - full active-offer search across remaining payment slots
          - trade sound/XP side effects
          - component-aware `ItemCost` predicate matching
        - deferred presentation parity work:
          - rendered generic button row backgrounds and hover/focus highlight
          - selected row state if a future vanilla source adds one
          - future trade XP result bar from payment/result prediction
          - component-aware cost predicate rendering
          - full trade stack decorations and hover tooltips
          - deprecated tooltip behavior
          - discount strikethrough
      - LecternMenu:
        - page buttons queue vanilla previous/next button ids.
        - Done closes the active container.
        - Take Book queues vanilla button id `3`.
        - current page text renders from decoded book components.
        - deferred presentation parity work:
          - rich text styles and click events
          - exact vanilla font wrapping
          - text filtering toggle behavior
      - ShulkerBoxMenu:
        - quick-move paths are locally predicted for:
          - shulker slots back to player inventory with vanilla reverse-fill
            ordering
          - player inventory/hotbar slots into the shulker box with vanilla
            forward-fill ordering
      - SmithingMenu:
        - input slots quick-move back to the player inventory with vanilla
          forward-fill ordering
        - player-inventory quick-move paths are locally predicted when
          `UpdateRecipes` provides the three vanilla smithing property sets:
          - template/base/addition input slot routing
          - ordinary player inventory/hotbar range movement
        - result-slot quick-move into the player inventory is locally predicted
          when the current server result fully fits and the single
          template/base/addition inputs are consumed to empty.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server result when the single template/base/addition
          inputs are consumed to empty.
        - result-slot clicks are kept server-authoritative until:
          - carried-cursor or secondary result-slot pickup
          - result prediction
          - result recomputation after partial input consumption
          - partial/full-inventory result transfers
          - level event side effects
        - deferred presentation parity work:
          - armor stand preview
          - cycling empty-slot icons
          - tooltips
      - StonecutterMenu:
        - non-result slots
        - vanilla slot ranges
        - decoded item-id recipe input routing
        - result-slot quick-move into the player inventory is locally
          predicted from the current server-provided result stack when the
          transfer fully fits and the input/result stacks are hashable.
        - result-slot primary pickup with an empty cursor is locally predicted
          from the current server-provided result stack when the single input is
          consumed to empty and the input/result stacks are hashable.
        - recipe grid wheel scroll and button clicks queue
          `ServerboundContainerButtonClickPacket`
        - recipe option icons and enabled/disabled scroller sprites are rendered
          from native HUD state
        - result-slot pickups with remaining input, carried-cursor pickup,
          partial/full-inventory result transfers, recipe used bookkeeping,
          input-specific result recomputation, and take-result sound side
          effects remain server-authoritative.
      - FurnaceMenu/BlastFurnaceMenu/SmokerMenu, including:
        - vanilla slot ranges
        - result-to-player transfer order
        - recipe-property-set input routing
        - official `FuelValues` fuel routing when vanilla assets are loaded
    - Control/native can still build a basic `ServerboundContainerClickPacket`
      from the active container id, state id, slot id, and cursor item for
      server-opened containers when the carried stack is hash-safe.
    - CrafterMenu empty grid-slot toggles queue
      `ServerboundContainerSlotStateChangedPacket`; rendered disabled/powered
      state remains driven by server `ContainerSetData`.
  - Block destroy:
    - Native block destroy progress records the starting main-hand item
      signature and restarts the destroy sequence when the selected
      item/components change.
    - Default item mining profiles are derived from official 26.1 item
      declarations and block tags for vanilla tool-like items:
      - pickaxe
      - axe
      - hoe
      - shovel
      - sword
      - shears
    - Default block destroy profiles are derived from official 26.1
      `Blocks.java` declarations for:
      - direct `strength` / `destroyTime` / `instabreak` chains
      - `requiresCorrectToolForDrops`
      - `ofLegacyCopy` / `ofFullCopy` inheritance
      - `InfestedBlock` / `InfestedRotatedPillarBlock` host destroy-time
        halving
      - common helper registrations for:
        - logs
        - stems
        - leaves
        - buttons
        - flower pots
        - candles
        - beds
        - stained glass
        - shulker boxes
        - pistons
        - stairs
    - Local destroy progress applies:
      - selected main-hand item profile
      - vanilla-shaped mining speed
      - `correct_for_drops` rule order
    - It applies synced local player destroy-speed state:
      - `mining_efficiency` attribute id `20` when item speed is above `1`
      - Haste effect id `2` and Conduit Power effect id `28`, using the max
        amplifier
      - Mining Fatigue effect id `3` with vanilla scale
      - `block_break_speed` attribute id `5`
      - `submerged_mining_speed` attribute id `29` when the local player's
        standing eye position is in water
      - airborne slowdown
    - Vanilla Efficiency and Aqua Affinity affect destroy speed through
      synced attributes, not local component fallback:
      - Efficiency contributes to `mining_efficiency`.
      - Aqua Affinity contributes to `submerged_mining_speed`.
      - Native keeps `UpdateAttributes` as the authoritative path.
    - It tracks vanilla-shaped local destroy stages in canonical interaction
      state and clears them on completion/abort/restart.
    - Left-click attack and block destroy input is suppressed while canonical
      local `using_item` is active.
    - Block destroy and block-target item use are suppressed when the hit block
      position is outside the canonical world border bounds.
    - It projects destroy progress to batched renderer-visible cube crack
      overlays from:
      - local stages
      - server `BlockDestruction` progress
    - Those overlays use:
      - official `destroy_stage_0..9` block atlas textures
      - keeping the highest stage when multiple overlays target one block
        position
      - expiring server destruction entries after:
        - the vanilla-shaped 400 render tick window
    - It predicts the locally destroyed block before queuing:
      - stop destroy packets
      - instant destroy packets
    - Prediction targets:
      - air
      - legacy water/lava state
    - It reconciles local block-destroy predictions by:
      - deferring server block updates into pending prediction state
      - resolving predictions on `BlockChangedAck`
      - storing the prediction-start local player position
      - snapping the local player back to that position when a rejected ack
        restores a colliding block, matching vanilla `ClientLevel.syncBlockState`
    - Local destroy ticking emits vanilla-shaped block hit sounds through the
      native audio runtime:
      - `SoundType.getHitSound()`
      - `SoundSource.BLOCKS`
      - `(volume + 1) / 8`
      - `pitch * 0.5`
      - every 4 destroy ticks
    - Level event `2001` emits vanilla-shaped block break sounds through the
      native audio runtime:
      - `Block.stateById(data)`
      - `SoundType.getBreakSound()`
      - `SoundSource.BLOCKS`
      - `(volume + 1) / 2`
      - `pitch * 0.8`
  - Completion requires full vanilla movement and these flows to work through
    encoded serverbound packets end to end.

### Signed Chat And Chat Acknowledgement Production

- Owner: `bbb-protocol` + `bbb-net` + `bbb-world` + `bbb-native`
- Status: `partial`
- Next action:
  - Implement remaining signed chat payload production:
    - `ServerboundChatPacket` signatures
    - non-empty `ServerboundChatCommandSignedPacket` argument signatures
    - session/key handling if offline-compatible servers require it
- Evidence / boundary:
  - Covered pieces:
    - `ServerboundChatAckPacket` id 6 and VarInt `offset` encoding
    - offline unsigned `ServerboundChatPacket` encoding/sending
    - `NetCommand::ChatAcknowledgement` sending
    - canonical processed-signature offset tracking
    - online drain queueing
    - offline probe ack sending after vanilla's `offset > 64` threshold
    - play -> configuration re-entry flushes pending signed-chat
      acknowledgement before `ServerboundConfigurationAcknowledgedPacket`
    - canonical outbound last-seen tracker for unsigned chat messages:
      - 20-entry vanilla ring order
      - offset clearing
      - fixed 20-bit acknowledgement bitset
      - checksum byte
      - full-signature pending delete ignore
    - native normal chat submission consumes the canonical last-seen update
    - slash commands keep vanilla string-only `ServerboundChatCommandPacket`
      when the canonical command tree has no signable argument path
    - slash commands with a `minecraft:message` signable argument path send
      `ServerboundChatCommandSignedPacket` with:
      - timestamp
      - salt
      - empty argument signatures for offline/no-profile-key mode
      - canonical outbound last-seen update
  - Full signed chat payload generation remains follow-up work.

### Manual Visual/Audio Comparisons

- Owner: relevant runtime owner
- Status: `deferred`
- Next action:
  - Whenever visual or audio behavior cannot be proven by automated tests,
    record the required proof to close the slice:
    - vanilla source path
    - asset path
    - screenshot
    - smoke test
    - manual comparison
- Evidence / boundary:
  - The project gate allows manual or screenshot/audio smoke checks outside
    normal unit tests, but they must be documented when required.

Mounted boat input now has a basic locally authoritative path:

- Updates local look while mounted.
- Advances a simple root-boat transform from local input.
- Queues both paddle-state and `MoveVehicle` commands.
- Leaves the following covered by the native input/movement ledger row above:
  - full vanilla boat physics
  - water/buoyancy/collision parity
  - non-boat vehicle movement

## Update Rules

- Do not remove a row unless the current slice proves the feature is fully
  covered for its stated scope and the proof is referenced in code or tests.
- Prefer splitting a broad row into narrower rows as soon as a feature has a
  concrete owner and testable next action.
- Keep rows scoped to semantic ownership. Do not add arbitrary line-count or
  agent-parallelism work here.
