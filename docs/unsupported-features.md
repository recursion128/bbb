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
    - dropped-item icons (3D block/item model renderer in progress — see the
      "3D Block-Model / Item-Model Rendering" evidence bullet below: the
      renderer-side geometry baking layer now exists; the GPU draw pass and
      native consumer wiring are the remaining slices)
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
      light), `hasRedOverlay` (hurt/death red `OverlayTexture` flash),
      `whiteOverlayProgress` (creeper swelling white flash), `isAggressive`
      (`Mob.isAggressive`, the zombie-model family's held-out-arm raise), and the
      enderman `carriedBlock`-non-empty / `isCreepy` flags (carried as
      `enderman_carrying`/`enderman_creepy`, driving the held-out arm pose and the
      creepy head/hat shift), the bat `isResting` flag (carried as
      `bat_resting`, swapping the bat to the `BAT_RESTING` hanging pose with a head look),
      the bee `hasStinger` flag (carried as `bee_has_stinger`, hiding the stinger
      cube once the bee has stung), the bee `isAngry` state (carried as `bee_angry`,
      the synced `NeutralMob` anger-end time vs the world game time, suppressing the
      bee's `bobUpAndDown`), and the player `isCrouching` pose (carried as
      `is_crouching`, the synced `Pose.CROUCHING`, applying the `HumanoidModel` sneak pose)
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
      head pitch, the camel's dash-entangled gait (the colored and textured
      `CAMEL_WALK` / `CAMEL_BABY_WALK`, the sit-down / seated / stand-up transitions, and the looping
      `CAMEL_DASH` gallop are now reproduced; only `CAMEL_IDLE` stays deferred), and the tail's `ageInTicks` yRot
      wag stay deferred). The remaining
      slices consume them
      in the other model families' `setupAnim` (fish; other birds; etc., plus
      the `HumanoidModel`/illager/villager arm and ear/nose poses); the snow golem has no
      walk-driven swing (its `setupAnim` is the head-yaw twist/orbit, now implemented).
    - `isInvisible` is now projected uniformly into `EntityRenderState.invisible`
      for every entity (vanilla `LivingEntityRenderer.isBodyVisible`): a
      normally-invisible entity (Invisibility effect / `setInvisible`) skips the
      base body for a client that cannot see it, matching vanilla's null body
      `getRenderType`; non-base layers then follow their own vanilla gates, so
      invisible-gated layers skip while layer-specific exceptions are tested
      separately. The texture-backed
      `isInvisible && !isInvisibleToPlayer` branch now records the base body as
      `entityTranslucentCullItemTarget` with the vanilla `0x26ffffff`
      force-transparent alpha while invisible-gated layers still skip. World/native
      projection now clears `isInvisibleToPlayer` for spectator viewers, matching
      `Entity.isInvisibleTo(player)`'s spectator branch. The shared glowing bit
      (`Entity.DATA_SHARED_FLAGS_ID` bit 6 / client `isCurrentlyGlowing()`) is now
      projected as `appears_glowing`; an invisible living textured base that is still
      invisible to this client records vanilla `RenderTypes.outline(texture)` submission
      metadata for the base body, and the vanilla-specific invisible-glowing
      sheep wool and slime outer overlay submissions are also recorded as
      `RenderTypes.outline(...)`; snow-golem carved-pumpkin and mooshroom mushroom
      block attachments now record outline-only metadata for their vanilla
      `submitOnlyOutline` path while skipping ordinary block-quad baking.
      `EntityRenderState.outlineColor` is now projected from the scoreboard team
      color used by `Entity.getTeamColor()` (ordinary entities by UUID scoreboard
      name, players by GameProfile name) and recorded on texture-backed
      submissions; missing/reset team color uses vanilla opaque white.
      Same-team friendly-invisible visibility now follows `PlayerTeam` option
      bit 2 (`canSeeFriendlyInvisibles`) for the local player's team, clearing
      `isInvisibleToPlayer` for living entities that vanilla would render
      translucent. Other invisible-gated non-base layers still skip, while the
      adult `WolfArmorLayer` exception now keeps its armor equipment/crack
      submissions in hidden, self-visible translucent, and glowing-outline
      invisible states because vanilla does not gate that layer on
      `state.isInvisible`. Texture-backed player/humanoid invisible branches now
      likewise keep vanilla ungated `HumanoidArmorLayer`, `WingsLayer`, and
      `CustomHeadLayer` skull submissions in hidden no-base and glowing-outline
      states, preserving texture/render-type/tint/transform/light/no-overlay/
      outline-color/order metadata; the native item-model pass also keeps
      ordinary humanoid main-hand items plus generic non-skull HEAD items in
      visible, hidden, and hidden-glowing states. `CapeLayer` remains suppressed
      for invisible players because vanilla explicitly gates it on
      `!state.isInvisible`. Texture-backed static-atlas outline submissions now
      also retain CPU-side folded outline geometry when the texture is present.
      Colored-path force-transparent output and GPU outline presentation remain
      deferred under the `outlineColor` slot.
    - deferred slots to add with their own slices, each carrying real vanilla
      semantics and tests rather than tint fallbacks: `ageScale` (the baby `0.5`
      proportions applied in model `setupAnim`, distinct from the now-projected
      `SCALE`-attribute `scale`)
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
    The player-only `SpinAttackEffectLayer` visual shell is also recorded as an
    explicit submission while auto-spinning, now through
    `player_spin_attack_effect_layer_pass` with vanilla `ModelLayers.PLAYER_SPIN_ATTACK`:
    `trident_riptide.png`, `entityCutout`, no overlay, `order(0)`,
    same-order sequence after the currently implemented WingsLayer path, and
    submission-first missing-atlas behavior. Its two vanilla
    `SpinAttackEffectModel` boxes use the 64×64 riptide texture atlas, per-box scale
    `0.75`/`1.5`, and `ageInTicks * -50°/-55°` y-rotation.
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
    angle and baby ears are always re-posed. The headbutt head ram is implemented too
    (`apply_hoglin_headbutt`, always applied — at rest it re-sets the baked down-tilt):
    `Hoglin`/`Zoglin.handleEntityEvent` event `4` sets `attackAnimationRemainingTicks = 10`
    (projected as the RAW int — vanilla `AbstractHoglinRenderer` does not partial-lerp it),
    decremented each tick; `animateHeadbutt` SETs `head.xRot = lerp(1 - |10 - 2·tick|/10,
    0.87266463, -π/9)` (the head rises from its rest down-tilt to `-π/9` at the `tick = 5`
    peak), and the baby additionally lifts `head.y += factor·2.5`. The creeper
    (`emit_creeper_model` colored and `emit_creeper_textured_model` textured) is a custom
    `EntityModel` too, but its `setupAnim` leg swing is exactly the `QuadrupedModel`
    formula (legs at `[2, 3, 4, 5]`), so it reuses the shared quadruped swing; its
    `CreeperRenderer.scale` swell (the inflate-and-flicker before exploding —
    `wobble = 1 + sin(swelling·100)·swelling·0.01`, `g = clamp(swelling, 0, 1)⁴`,
    `x/z *= (1 + g·0.4)·wobble`, `y *= (1 + g·0.1)/wobble`, identity at `swelling = 0`)
    is folded into `creeper_model_root_transform` at the per-renderer `this.scale()` hook.
    The `CreeperPowerLayer` energy swirl is now wired: a charged creeper (the synced
    `Creeper.DATA_IS_POWERED`, entity-data index `17`, projected onto `creeper_powered`) draws the
    inflated `CREEPER_ARMOR` model (`CubeDeformation(2.0)`, `CreeperModel::new_armor`, driven by the
    same `setup_anim` so it tracks the body pose) through the new additive scrolling pipeline (vanilla
    `RenderTypes.energySwirl`): `creeper_armor.png` (`CREEPER_ARMOR_TEXTURE_REF`) scrolling both axes by
    `xOffset(ageInTicks) % 1 = (ageInTicks · 0.01) % 1`, tinted by the vanilla `0xFF808080` half-grey,
    `BlendFunction.ADDITIVE`, emissive, `ALPHA_CUTOUT 0.1` — the same shader-side `fract` atlas-wrap as
    the wind charge's `breezeWind`, just additively blended. `creeper_textured_layer_passes` now records
    both vanilla `ModelLayers.CREEPER` and `ModelLayers.CREEPER_ARMOR`, with base `entityCutout` and armor
    `energySwirl` texture/render-type/tint/order metadata; the base dispatch consumes only the body pass,
    while the charged overlay helper consumes the armor pass and still gates it on `isPowered`. The
    `HumanoidModel` leg swing (`humanoid_leg_swing_pose`: the right leg, part offset
    `x < 0`, in phase and the left leg out of phase, since both legs sit at `z = 0`) is
    consumed by the zombie family (`ZombieModel` / shared-dispatch `ZombieVariantModel` —
    zombie, husk, drowned, zombie villager, adult and baby) and the skeleton family
    (`emit_skeleton_model`/`emit_skeleton_variant_model` colored and
    `emit_skeleton_textured_model` → `emit_humanoid_textured_passes` — skeleton, stray,
    parched, wither skeleton, bogged sheared/unsheared; the Stray/Bogged clothing
    overlay swings too, since its layer `SkeletonModel` runs the same `setupAnim`).
    Both families inherit the `HumanoidModel` legs unchanged (`SkeletonModel`/
    `AbstractZombieModel extends HumanoidModel`); the zombie family then overrides the
    arms with the held-out `animateZombieArms` pose (implemented — see the zombie-arms note
    below), while the skeleton family keeps the inherited arm counter-swing in its default
    (non-aiming) state (implemented — see the arm-swing note below). The piglin
    family (shared-dispatch `PiglinModel` — piglin, piglin brute, zombified piglin,
    adult and baby) also consumes it: `AbstractPiglinModel extends HumanoidModel`, whose
    `setupAnim` runs `super.setupAnim` (the inherited legs and arms) before swaying only
    the ears. The adult/baby piglin and the brute keep the inherited arm counter-swing in
    their default state (`PiglinModel` overrides the arms in its `DANCING`, `CROSSBOW_HOLD`,
    `CROSSBOW_CHARGE`, `ATTACKING_WITH_MELEE_WEAPON`, and `ADMIRING_ITEM` poses — all implemented,
    see the piglin dance/crossbow/attack/admire note below), so the
    arm swing is implemented for them too; the
    zombified piglin instead overwrites the arms with `AnimationUtils.animateZombieArms`
    (implemented via the same held-out zombie-arm helper), so only its legs use the
    inherited walk swing while its arms stay in the zombie pose. The illager family
    (`emit_illager_model`
    — evoker, vindicator, illusioner, pillager) uses a dedicated `illager_leg_swing_pose`
    (`cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5`): `IllagerModel` is not a
    `HumanoidModel` and adds an extra `0.5` amplitude factor (the shared
    `half_amplitude_leg_swing_pose`), and its body layers list the legs at `[3, 4]` for
    the crossed-arms layouts (evoker/vindicator/illusioner) and `[2, 3]` for the
    uncrossed pillager, resolved per family. The pillager also swings its *separate* arms
    with the exact `HumanoidModel` amplitude (`cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`,
    [`humanoid_arm_swing_pose`], arms at `[4, 5]`); the idle evoker/vindicator/illusioner show
    the static crossed `arms` part, which vanilla never animates (it swings the *invisible*
    separate arms), so their visible arms stay put. The SPELLCASTING arm pose is now projected:
    `SpellcasterIllager.isCastingSpell()` (the synced `DATA_SPELL_CASTING_ID` byte > 0, data id
    `17`, gated to the evoker/illusioner) swaps those two to the uncrossed layout (hiding the
    crossed `arms` part, the illusioner keeping its hat) and raises both separate arms —
    `xRot = cos(ageInTicks · 0.6662) · 0.25`, `zRot = ±3π/4`, holding the bind offset
    `x = ∓5` — on both render paths. The pillager `CROSSBOW_HOLD` arm pose is also
    projected: a pillager holding a crossbow (`main_hand_holds_crossbow`, the main-hand
    item resolved to `minecraft:crossbow`) that is not drawing (`!is_charging_crossbow`,
    the synced `IS_CHARGING_CROSSBOW` boolean, data id `17`, gated to the pillager type)
    levels the weapon along the head look — `AnimationUtils.animateCrossbowHold`,
    `holdingInRightArm`: right (holding) arm `xRot = -π/2 + head.xRot + 0.1`,
    `yRot = -0.3 + head.yRot`; left (shooting) arm `xRot = -1.5 + head.xRot`,
    `yRot = 0.6 + head.yRot` — overwriting the walk swing on both render paths (the held
    crossbow mesh rides the leveled hand). The pillager `CROSSBOW_CHARGE` pull-back draw is
    now projected too: while the synced `IS_CHARGING_CROSSBOW` flag is set, a per-tick
    use-item counter (`CrossbowChargeAnimationState`, the client reconstruction of
    `LivingEntity.getTicksUsingItem()` — it counts up while drawing and resets to `0` the
    moment the flag clears) feeds `AnimationUtils.animateCrossbowCharge(right, left,
    maxChargeDuration = floor(1.25·20) = 25, ticksUsingItem, holdingInRightArm = true)`: the
    right (holding) arm sits at `yRot = -0.8`, `xRot = -0.97079635`; the left (pulling) arm
    lerps `yRot 0.4 → 0.85` and `xRot -0.97079635 → -π/2` across the draw (clamped at full
    charge), overwriting the level hold pose on both render paths. The regular piglin reuses the SAME shared
    `animateCrossbowHold` pose (`Piglin.getArmPose` → `CROSSBOW_HOLD`): a piglin holding a
    *charged* crossbow (`isHolding(CROSSBOW) && CrossbowItem.isCharged(weaponItem)` — the
    main-hand item resolved to `minecraft:crossbow` with a non-empty decoded
    `minecraft:charged_projectiles` component), not dancing (top priority) and not drawing
    (`DATA_IS_CHARGING_CROSSBOW`, data id `18`, gated to the regular piglin), levels the
    crossbow. The regular piglin also drives the `CROSSBOW_CHARGE` pull-back draw with the SAME shared
    `animateCrossbowCharge` pose as the pillager (`Piglin.getArmPose` → `CROSSBOW_CHARGE` while
    `isChargingCrossbow()`, the `DATA_IS_CHARGING_CROSSBOW` id `18` flag): the same per-tick use-item
    counter (reset to `0` when the flag clears) feeds `maxChargeDuration = 25`, lerping the pulling arm
    from rest to full draw — ranked below `ATTACKING`/`ADMIRING`/`DANCING`, above `CROSSBOW_HOLD`. The
    piglin and the piglin brute also drive `ATTACKING_WITH_MELEE_WEAPON`
    (`Piglin`/`PiglinBrute.getArmPose`: `isAggressive() && isHoldingMeleeWeapon()`): aggression is
    the synced `Mob.DATA_MOB_FLAGS_ID` (id `15`) bit `4` (the `is_aggressive` projection now covers
    the piglin/brute alongside the zombie family), and a melee weapon is a main-hand item carrying
    the `minecraft:tool` data component (`getMainHandItem().has(DataComponents.TOOL)` — the decoded
    component patch lists wire type `28` in its added types, so no item-registry lookup is needed).
    `PiglinModel` then raises the weapon overhead at rest (`holdWeaponHigh`, main right arm
    `xRot = -1.8`, overwriting only the pitch) and chops it down mid-swing
    (`AnimationUtils.swingWeaponDown` over the projected `attack_anim`, the SAME shared
    [`apply_humanoid_weapon_swing_down`] the vindicator axe uses). The regular piglin also drives
    `ADMIRING_ITEM` (`Piglin.getArmPose`: `PiglinAi.isLovedItem(getOffhandItem())` =
    `getOffhandItem().is(ItemTags.PIGLIN_LOVED)`): a piglin-loved item in the OFFHAND tilts the head down
    to it (`head.xRot = 0.5`, `head.yRot = 0`) and lifts the off (left) arm to show it (`leftArm.xRot =
    -0.9`, `yRot = 0.5`). The `minecraft:piglin_loved` membership is the offhand item's id appearing in the
    network-loaded `minecraft:item` tag set (no item-registry lookup). Vanilla precedence is DANCING >
    ADMIRING > ATTACKING > CROSSBOW, so a loved offhand item suppresses the attack/crossbow poses (the
    regular-piglin gates carry `&& !admiring`); the brute has no admire branch and the zombified piglin
    uses its implemented `animateZombieArms` pose. The illusioner `BOW_AND_ARROW` draw is also
    projected (`Illusioner.getArmPose`: `!casting && isAggressive` → BOW_AND_ARROW): the
    uncrossed arms aim the bow along the head look with the illager bracing the off hand —
    right arm `xRot = -π/2 + head.xRot`, `yRot = -0.1 + head.yRot`; left arm
    `xRot = -0.9424779 + head.xRot`, `yRot = head.yRot - 0.4`, `zRot = π/2`. The evoker /
    vindicator `CELEBRATING` victory dance is projected too (`Raider.isCelebrating()`, the
    synced `IS_CELEBRATING` boolean, data id `16`, gated to the evoker/vindicator and
    suppressed while casting / aggressive): both separate arms bob
    `xRot = cos(ageInTicks · 0.6662) · 0.05` and raise asymmetrically (right `zRot =
    2.670354`, left `zRot = -3π/4`). These swaps to the uncrossed separate-arm layout
    mirror vanilla `crossedArms = pose == CROSSED`. The vindicator melee `ATTACKING` swing is
    projected too (`Vindicator.getArmPose`: `isAggressive` → ATTACKING). `IllagerModel.setupAnim`
    selects the branch from the rendered main-hand item state: an empty hand uses
    `AnimationUtils.animateZombieArms(left, right, true, state)`, while an armed hand uses
    `AnimationUtils.swingWeaponDown(mainArm = RIGHT)` over the projected `attack_anim`. The armed
    right arm raises overhead (`xRot = -1.8849558 + cos(age·0.09)·0.15`) and chops with
    `+= sin(t·π)·2.2 - sin((1-(1-t)²)·π)·0.4`, the left arm trails
    (`xRot = cos(age·0.19)·0.5 + sin(t·π)·1.2 - …·0.4`), both yawing apart `±π/20` with the
    shared `bobArms` roll. `IllagerModel.isRiding` is projected from `Entity.isPassenger()` too:
    riding illagers use the vanilla fixed seated preset (`arms.xRot = -π/5`, legs at
    `xRot = -1.4137167`, `yRot = ±π/10`, `zRot = ±0.07853982`) before the arm-pose
    branch runs, so crossbow / spell / celebrate / attack arms can still overwrite it. `IllagerModel`
    is not a `HumanoidModel`, so there is no body twist (no `setupAttackAnimation`). The villager family
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
    parts and the visibility-filtered part array keeps the legs at `[4, 5]`). The player
    also runs the **melee attack swing** (`HumanoidModel.setupAttackAnimation`, the default
    WHACK): the `ClientboundAnimate` packet (action `0` main / `3` off hand) arms a 6-tick
    `LivingEntity.swing` ramp — a new client-tick `AttackSwingAnimationState` advancing
    `attackAnim` exactly as `updateSwingTime` — projected as `attack_anim`
    (`getAttackAnim(partialTick)`, the partial-lerp) + `attack_arm_off_hand`. `setupAnim`
    applies it last (after the walk swing / crouch), via the shared
    [`apply_humanoid_attack_animation`]: the body twists `yRot = sin(sqrt(t)·2π)·0.2` (off arm
    negated), both arm anchors swing around it (`x = ∓cos · 5`, `z = ±sin · 5`), and the
    attacking arm whacks down (`xRot -= sin(outQuart(t)·π)·1.2 + sin(t·π)·-(headXRot-0.7)·0.75`,
    `yRot += bodyYRot·2`, `zRot += sin(t·π)·-0.4`). The same `apply_humanoid_attack_animation`
    helper is wired into the skeleton (with the `SkeletonModel` melee arm override), the zombie
    family applies the `animateZombieArms` arm-swing terms, and the vindicator chops with the
    `IllagerModel` `swingWeaponDown` axe pose, and the piglin/brute reuse that same
    `swingWeaponDown` for their `ATTACKING_WITH_MELEE_WEAPON` pose (see the piglin note above); the
    default `HumanoidModel` body-twist whack for a non-`ATTACKING` piglin (empty hand / non-tool
    item) and the per-item swing duration are deferred (every swing is the default 6-tick whack). The
    `STAB` swing type IS implemented for the player: a remote player whose main-hand item is one of the
    seven spears (`wooden`/`stone`/`copper`/`iron`/`golden`/`diamond`/`netherite_spear`, whose item
    prototype sets `SWING_ANIMATION = STAB` via `Item.Properties.spear(...)`) lunges with
    `SpearAnimations.thirdPersonAttackHand` instead of the whack — the shared body twist + arm anchors
    run, then the attacking arm's pitch drives `xRot += (90·inOutSine(progress(t,0,0.05)) −
    120·inQuad(progress(t,0.05,0.2)) + 30·inOutExpo(progress(t,0.4,1.0)))·π/180` (the prologue's
    body-twist additions on the arm rotations are undone, so the off arm keeps its resting pitch). The
    STAB default lives on the item prototype (not the network component patch), so it is detected by the
    resolved item id (gated to the player kind; a datapack-overridden `SWING_ANIMATION` on a non-spear
    item, the `NONE` swing type, and the STAB pose on non-player humanoids — which use their own arm
    poses — stay deferred). The per-tick `thirdPersonHandUse` hold sway (`KINETIC_WEAPON`/`ticksUsingItem`)
    also stays deferred. The
    enderman (`emit_enderman_model` colored and `emit_enderman_textured_model` textured)
    uses dedicated `enderman_arm_swing_pose`/`enderman_leg_swing_pose`: `EndermanModel
    extends HumanoidModel`, so `super.setupAnim` sets the inherited arm and leg swing,
    then the enderman halves both (`*= 0.5`) and clamps them to `[-0.4, 0.4]` (arms at
    `[2, 3]`, legs at `[4, 5]`); the arm swing reuses the base
    [`humanoid_arm_swing_pose`] (the right arm — part offset `x < 0` — the half-cycle out
    of phase, counter to the same-side leg) before the halve/clamp. Carrying a block then
    *overrides* both arms (`enderman_carried_arm_pose`: `xRot = -0.5`, `zRot = ±0.05`, held
    out front), and the creepy stare drops the head `y -= 5` while raising its hat child
    `y += 5` (`ENDERMAN_HEAD_CHILDREN_CREEPY`, so the outer head layer keeps its world
    position as the inner head opens downward) — both gated on the projected
    `enderman_carrying`/`enderman_creepy`. The held block's own block-model render is
    implemented through the entity-attached block-model path; only the creepy render jitter stays
    deferred. The iron golem
    (`emit_iron_golem_model` colored and `emit_iron_golem_textured_model` textured) uses
    `iron_golem_walk_pose`: `IronGolemModel` is a custom `EntityModel` whose
    `setupAnim` swings both the legs (`±1.5 * Mth.triangleWave(pos, 13) * speed`) and —
    in its default non-attack/non-flower branch — the arms (`(-0.2 ± 1.5 *
    triangleWave(pos, 13)) * speed`), a triangle-wave gait rather than a cosine one; the
    arms sit at part offset `x = 0`, so the right/left role is fixed by slot (arms
    `[2, 3]`, legs `[4, 5]`). This is the first model whose **arm** swing is a pure
    walk-driven animation (so it is implemented). The attack swing and the offer-flower
    arm pose are implemented too (`apply_iron_golem_arm_events`, overriding the walk arms
    after the swing): `IronGolem.handleEntityEvent` event `4` sets `attackAnimationTick = 10`
    (projected, partial-lerped, as `attackTicksRemaining`) → both arms `xRot = -2 + 1.5 *
    triangleWave(tick, 10)` (the two-fisted smash); events `11`/`34` set/clear `offerFlowerTick`
    (`400`-tick countdown) → right arm `xRot = -0.8 + 0.025 * triangleWave(tick, 70)`, left arm
    `xRot = 0` (holding a poppy out); attack takes priority, and the legs keep the walk swing
    under both. The held poppy block render is implemented through the entity-attached block-model
    path: native resolves `Blocks.POPPY.defaultBlockState()` and the renderer-owned attachment
    transform mirrors `IronGolemFlowerLayer`'s right-arm bone plus
    `translate(-1.1875,1.0625,-0.9375)`, center translate, `scale(0.5)`, `rotateX(-90°)`,
    `translate(-0.5,-0.5,-0.5)`. The ravager (`emit_ravager_model` colored and
    `emit_ravager_textured_model` textured) uses a dedicated `ravager_leg_swing_pose`:
    `RavagerModel` is a custom `EntityModel` whose `setupAnim` swings the four legs with
    the `QuadrupedModel` diagonal phase (`cos(pos * 0.6662 [+ π])`, in phase when
    `x*z < 0`) but a shorter `0.4` amplitude (`legRot = 0.4 * speed`) rather than the
    usual `1.4`; legs sit at `[2, 3, 4, 5]` and the swing only sets `xRot`, leaving the
    nested neck/head subtree (which the head-look pose drives) untouched. Its neck/mouth
    attack, stun, and roar poses are implemented too (`apply_ravager_combat`, always applied
    so a resting jaw cracks open `π·0.01`): `Ravager.handleEntityEvent` event `4` sets
    `attackTick = 10` and event `39` sets `stunnedTick = 40`; the client `aiStep` decrements
    all three timers and, when the stun ends, arms `roarTick = 20` (so a roar always follows a
    stun). The partial-lerped `attackTicksRemaining` lunges the neck forward (`neck.z = -6.5 +
    ((1 + triangleWave(tick, 10))·0.5)³·12`) and snaps the jaw in two phases; otherwise a
    `stunnedTicksRemaining` tilts the neck (`xRot = 0.21991149`), side-shakes it (`neck.x =
    sin(stunned/40·10)·3`) and opens the jaw `π·0.05`, or a `roarAnimation` (`(20 - roarTick +
    partial)/20`) gapes it (`mouth.xRot = π/2·sin(roar·π/4)`). Only the roar particle/knockback
    effects (event `69`) are deferred. The spider and cave spider (`emit_spider_model` /
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
    render state, in both the colored and textured paths. `isSitting`, tame
    `tail.xRot = tailAngle` health droop, and the `shakeOffWater` water-shake roll are now
    projected and applied on both render paths. Deferred:
    (1) the
    `Camel`/`Frog` `updateWalkAnimation` overrides use different
    distance→speed mappings AND gate on pose/jump animation states the client does not
    fully track yet (the camel dash and frog jump/swim-idle/croak/tongue triggered
    animations ARE now driven, but the camel idle remains deferred), so their
    distance→speed walk input is left at the base mapping rather than the override. The
    `Creaking` override is pure (`min(distance · 25, 3)`, factor `0.4`) and IS now driven —
    its limb swing ramps ~3× faster than the base mapping (`walk_update_target_speed`); (2) the base
    `HumanoidModel.setupAnim` arm swing is implemented for the player, the skeleton
    family, and the non-zombified piglin family
    (`humanoid_arm_swing_pose`/`humanoid_arm_swing_parts`, arms at `[2, 3]`, the
    counter-swing `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5`; `SkeletonModel` and
    `(Abstract)PiglinModel` run `super.setupAnim` and override the arms only in their
    deferred pose branches, so the default arms swing — for the skeleton in both the
    colored and textured paths, every variant (skeleton, stray, parched, wither skeleton,
    bogged sheared/unsheared), and for the colored adult/baby piglin and brute). The
    always-on `HumanoidModel.setupAnim` idle arm bob (`AnimationUtils.bobModelPart` →
    `humanoid_arm_bob_pose`: `arm.zRot += scale * (cos(ageInTicks * 0.09) * 0.05 + 0.05)`,
    `arm.xRot += scale * sin(ageInTicks * 0.067) * 0.05`, scale `+1` right arm / `-1` left)
    rides on top of that swing every frame for the same three families — the player and
    skeleton in both render paths and the piglin colored and textured — so their arms never
    sit perfectly still (there is no static rest fast path, and the bob is kept out of the
    shared `humanoid_arm_swing_pose` so the pillager's separate arms do not get it). The
    enderman now composes the bob too: its `enderman_arm_swing_pose` applies the swing AND
    the bob, then halves+clamps only `xRot` (`[-0.4, 0.4]`) the vanilla way, leaving the
    bob's `zRot` to survive the clamp so the long arms gently splay (even the resting mesh's
    X extent widens to `±0.5494`). The `SPYGLASS`-pose bob skip IS now honored for the player:
    a player using a spyglass (`isUsingItem` + the using hand holds `minecraft:spyglass`) raises the
    holding arm to the eye via `apply_humanoid_spyglass_pose` (`HumanoidModel.poseRightArm`/`poseLeftArm`
    `SPYGLASS`): the arm pitch is `clamp(head.xRot − 1.9198622 − (crouch?π/12), −2.4, 3.3)`, the yaw is
    `head.yRot ∓ π/12`, and the arm `zRot` resets to its bind so the idle bob is skipped on that arm only;
    it is applied before the crouch block so the crouch `arm.xRot += 0.4` still lands on top. The using
    hand is read from the synced `DATA_LIVING_ENTITY_FLAGS` byte (id `8`, bit `1` = isUsingItem, bit `2` =
    off hand). The `TOOT_HORN` pose is likewise implemented (`apply_humanoid_toot_horn_pose`): a player
    tooting a goat horn (`isUsingItem` + the using hand holds `minecraft:goat_horn`, the only `TOOT_HORN`
    use-animation item) raises the holding arm to the mouth — `xRot = clamp(head.xRot, −1.2, 1.2) −
    1.4835298`, `yRot = head.yRot ∓ π/6` — keeping the idle bob (unlike the spyglass), applied before the
    crouch block. The `BRUSH` pose is likewise implemented (`apply_humanoid_brush_pose`): a player brushing
    (`isUsingItem` + the using hand holds `minecraft:brush`, the only `BRUSH` use-animation item) lowers the
    holding arm to the block — `xRot = arm.xRot · 0.5 − π/5`, `yRot = 0`. (Like every bbb posed arm, the
    halved `arm.xRot` carries bbb's small folded-in idle bob rather than vanilla's bob-applied-after; the
    full bob-reorder that would make the multiply exact is the shared deferred convention.) The generic
    main-hand `ITEM` hold pose IS now implemented too (`apply_humanoid_item_hold_pose`, the
    `AvatarRenderer.getArmPose` fallback `ArmPose.ITEM` → `HumanoidModel.poseRightArm` ITEM case): a player
    holding a plain item in the main hand and NOT using it lowers/halves the arm — `xRot = arm.xRot · 0.5 −
    π/10`, `yRot = 0` — applied before the crouch/attack blocks (vanilla `poseArm` precedes
    `setupAttackAnimation`, and crouch/attack are additive offsets that commute). It is gated to the player
    kind (`HumanoidMobRenderer.getArmPose` never returns `ITEM`), a non-empty main hand, and `!isUsingItem`,
    and excludes spears (→ `SPEAR`) and charged crossbows (→ `CROSSBOW_HOLD`) so their dedicated poses win; a
    non-charged crossbow or a held (not drawn) bow correctly falls through to `ITEM`. The OFF-hand `ITEM`
    hold pose IS now implemented too (the same `apply_humanoid_item_hold_pose` posed onto the left arm, the
    `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback → `HumanoidModel.poseLeftArm` ITEM case): a player
    holding a plain off-hand item (shield/totem/block/food) lowers/halves the left arm, gated to a non-empty
    off hand and suppressed only when the OFF hand is the using hand (its use poses win; using the MAIN hand
    leaves the off hand on `ITEM`), excluding off-hand spears/charged crossbows. The `BLOCK` use-item arm
    pose IS now implemented too (`apply_humanoid_block_pose`, `HumanoidModel.poseRightArm`/`poseLeftArm`
    `BLOCK` → `poseBlockingArm`): while a player raises a non-consumable `DataComponents.BLOCKS_ATTACKS` item
    (`isUsingItem` + using hand; vanilla shield by resolved `minecraft:shield` id, datapack/patch-granted
    blockers by component type id 37 in `added_type_ids`; `CONSUMABLE` type id 24 still wins and routes to
    EAT/DRINK) the holding arm tucks the blocking item forward along the head look — `xRot = arm.xRot · 0.5 −
    0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)`, `yRot = (right ? −π/6 : π/6) + clamp(head.yRot, −π/6,
    π/6)` — applied before the crouch block. The `THROW_TRIDENT` use-item arm
    pose IS now implemented too (`apply_humanoid_throw_trident_pose`, the same `poseRightArm`/`poseLeftArm`
    `THROW_TRIDENT` case the drowned reaches via aggression): while a player charges a trident throw
    (`isUsingItem` + the using hand holds a trident, `TridentItem.getUseAnimation() == TRIDENT`) the holding
    arm raises the trident straight overhead — `xRot = arm.xRot · 0.5 − π`, `yRot = 0`. The two-handed
    `BOW_AND_ARROW` draw IS now implemented too (`apply_humanoid_bow_pose`, `HumanoidModel.poseRightArm` /
    `poseLeftArm` `BOW_AND_ARROW`): while a player draws a bow (`isUsingItem` + the using hand holds a bow,
    `BowItem.getUseAnimation() == BOW`) BOTH arms raise along the head look — both arms pitch to
    `−π/2 + head.xRot`; the main-hand branch yaws right/left to `−0.1 + head.yRot` /
    `0.1 + head.yRot + 0.4`, while the off-hand branch mirrors the brace offset to
    `−0.1 + head.yRot − 0.4` / `0.1 + head.yRot`. Because `BOW_AND_ARROW`/`THROW_TRIDENT`/
    `CROSSBOW_CHARGE` are `affectsOffhandPose=true`, vanilla's `setupAnim` skips the opposite arm's
    `poseArm` when either hand draws them, so the projection now suppresses the opposite hand's `ITEM`
    fallback symmetrically (`main_hand_use_affects_offhand` / `off_hand_use_affects_offhand`). The two-handed
    `CROSSBOW_CHARGE` draw IS now implemented too (`apply_crossbow_charge_pose_for_hand`, the same
    `AnimationUtils.animateCrossbowCharge` the pillager/piglin use): while a player draws an uncharged
    crossbow (`isUsingItem` + the using hand holds a crossbow, `CrossbowItem.getUseAnimation() == CROSSBOW`,
    excluding an already-charged one — that is `CROSSBOW_HOLD`) the holding arm braces and the opposite arm
    pulls the string back over `crossbow_charge_ticks / 25`; the off-hand branch uses vanilla's
    `holdingInRightArm=false` mirror. The draw counter is the
    shared `getTicksUsingItem` reconstruction, now advanced for the player off its `isUsingItem` bit in the
    world tick loop (`getTicksUsingItem` is item-agnostic, so the same counter serves; the native layer gates
    the pose to the crossbow). The `CROSSBOW_HOLD` pose IS now implemented for the player too
    (`apply_crossbow_hold_pose` / `apply_crossbow_hold_pose_for_hand`, the same
    `AnimationUtils.animateCrossbowHold` the pillager levels): a player holding a CHARGED main-hand or
    off-hand crossbow while not mid-swing (`AvatarRenderer.getArmPose`: `!swinging && crossbow && isCharged`,
    checked before the use-item branch) levels the crossbow along the head look, setting both arms with the
    mirrored `holdingInRightArm` branch. The `swinging` boolean is now projected (`LivingEntity.swinging`, off
    the attack-swing state) so the swing wins as in vanilla, and the main-hand pose runs after the ITEM blocks
    so it overwrites the off-hand `ITEM` exactly as vanilla's `poseRightArm`-runs-last does for that case. The
    off-hand hold is suppressed when the main hand already has an affecting pose (`BOW_AND_ARROW`,
    `THROW_TRIDENT`, `CROSSBOW_CHARGE`/`HOLD`, or `SPEAR`), matching `poseLeftArm` being skipped. The
    `EAT`/`DRINK`
    (and any using-non-special) route to `ITEM` IS now handled too: the `ITEM` gate no longer keys off
    `!isUsingItem` but off "this hand is NOT using a special-pose item" (`main_hand_use_is_special` /
    `off_hand_use_is_special` over the bow/crossbow/trident/shield/spyglass/horn/brush set), so a player
    eating food, drinking a potion, or using any plain item correctly shows the lowered `ITEM` arm. The
    `affectsOffhandPose` skip is now symmetric (`main_hand_use_affects_offhand` /
    `off_hand_use_affects_offhand`), so an `affectsOffhandPose` draw in either hand suppresses the OPPOSITE
    hand's `ITEM`. The remaining use-item arm pose edge on the same dispatch (the off-arm `EMPTY` reset — a
    near-no-op since at rest the off arm's `yRot` is already `0`) stays deferred. (The `getArmPose`
    `isTwoHanded`-forces-off-hand-to-`ITEM` branch
    needs no implementation: every `isTwoHanded` pose is also `affectsOffhandPose`, so `setupAnim` always
    SKIPS the forced arm's `poseArm` and the forced value is never rendered.) The
    per-subclass arm/ear/nose poses that override it are tracked separately (the zombie held-out
    arms' attack swing — the resting held-out pose, the synced `Mob.isAggressive`
    arm-raise, and the `animateZombieArms` melee swing over the projected `attack_anim` —
    is implemented, see the zombie-arms note below; the skeleton `BOW_AND_ARROW`
    aim and the melee swing (`isAggressive && !isHoldingBow`, the raised-and-chopping arms
    driven by the projected `attack_anim`) are both implemented — see the skeleton note
    below; the zombified piglin `AnimationUtils.animateZombieArms` held-out pose is implemented; the
    `PiglinModel` `DANCING`, `CROSSBOW_HOLD`, `ATTACKING_WITH_MELEE_WEAPON`, and `ADMIRING_ITEM` poses
    and the `AbstractPiglinModel` ear flap are implemented for the piglin/brute — see below),
    the `WitchModel` idle nose bob and `isHoldingItem` nose hold pose are
    implemented — see below; the `VillagerModel` unhappy head shake is
    implemented — see below; the `GoatModel` ramming head
    tilt is implemented — see the goat note below; the `HoglinModel` headbutt head ram is implemented — see the hoglin note above), (the `EndermanModel`
    carried-block arm pose and creepy head/hat shift are implemented — see the
    enderman note above; the `IronGolemModel`
    attack swing and offer-flower arm pose are implemented — see the iron golem note above), the `HumanoidModel`
    item/attack/swim/elytra poses, and the
    player swim/elytra `speedValue` poses) are separate animations driven by
    states the client does not yet track (the `HumanoidModel` crouch sneaking
    pose is implemented for the player — see below);
    (3) consuming the projected values in the remaining model families' `setupAnim`
    (fish, other birds, etc.) are the next slices, plus the several deferred event/tail poses
    noted above. (The chicken wing flap is covered by the pure client-side `flap`/`flapSpeed`
    accumulator described in the chicken entry below. The snow golem has no walk-driven swing;
    its `setupAnim` head-yaw upper-body twist and arm orbit are implemented.) The
    `EntityRenderState.ageInTicks` (= entity `tickCount + partialTick`)
    is now projected for every entity (`with_age_in_ticks`, from the world's per-entity
    client-animation age), driving the continuous `AbstractPiglinModel.setupAnim` ear flap
    (`piglin_ear_flap_pose`, shared by every piglin/zombified-piglin subclass via
    `super.setupAnim`): `freq = ageInTicks * 0.1 + pos * 0.5`, `amp = 0.08 + speed * 0.4`,
    `leftEar.zRot = -default - cos(freq * 1.2) * amp`, `rightEar.zRot = default +
    cos(freq) * amp`, with `default` the `getDefaultEarAngleInDegrees()` of `30°` (adult/
    brute) or `5°` (baby). The ears are `&'static` head children, so the head subtree is
    hand-emitted with the flapped ears on both colored and textured paths; because
    the `±0.08` baseline and `ageInTicks` advance every frame, the ears never sit still.
    The regular piglin's `PiglinModel.setupAnim` `DANCING` pose (`Piglin.isDancing()`, the synced
    `DATA_IS_DANCING` boolean id `19`, projected as `piglin_dancing` and gated to the piglin type —
    the brute and zombified piglin never dance) is implemented (`apply_piglin_dance`): over
    `dancePos = ageInTicks / 60` it overwrites the ear sway (`±π/6 ∓ trig(dancePos·30)·10°`), bobs
    the head (`x += sin(dancePos·10)`, `y += sin(dancePos·40) + 0.4`) and body (`y +=
    sin(dancePos·40)·0.35`), and raises both arms overhead wagging (`rightArm.zRot = (70° +
    cos(dancePos·40)·10°)`, the left mirrored, `y += sin(dancePos·40)·0.5 ∓ 0.5`), running after the
    inherited walk + ear flap on the reset bind pose. The `PiglinArmPose` `ATTACKING_WITH_MELEE_WEAPON`
    (the `swingWeaponDown` axe chop over `attack_anim` + a `DataComponents.TOOL` main-hand check),
    `CROSSBOW_HOLD` (the held charged crossbow), `CROSSBOW_CHARGE` (the pull-back draw over the synced
    `DATA_IS_CHARGING_CROSSBOW` use-item counter, shared with the pillager), and `ADMIRING_ITEM` (the
    loved off-hand item via the `minecraft:piglin_loved` tag) poses are all implemented — see the piglin
    note above. The piglin family is now fully arm-posed against vanilla 26.1.
    The same `ageInTicks` projection drives the continuous `WitchModel.setupAnim` idle nose
    bob (`witch_nose_bob_pose`): `speed = 0.01 * (entityId % 10)`, `nose.xRot =
    sin(ageInTicks * speed) * 4.5°`, `nose.zRot = cos(ageInTicks * speed) * 2.5°`, both SET
    absolutely on top of the head look and the half-amplitude leg swing. The nose is a
    `&'static` head child (and carries the mole as its own child), so the witch's head subtree
    is hand-emitted with the bobbed nose in both the colored and textured paths; because
    `cos` never reaches a zRot of `0`, the nose is always re-posed (there is no static fast
    path). The `isHoldingItem` nose hold pose is projected from a non-empty main hand and
    applied after the idle bob (`setPos(0, 1, -1.5)`, `xRot = -0.9`), preserving the bobbed
    `zRot` for `WitchItemLayer`'s potion branch.
  - The `LivingEntityRenderer.setupRotations` body shake is implemented end to end.
    World side: a living entity (`vanilla_living_entity_type` gate) whose synced
    `ticksFrozen` (`DATA_TICKS_FROZEN`, id `7`) reaches `getTicksRequiredToFreeze()`
    = `140` is marked `isFullyFrozen` in `EntityModelSourceState`. Native side:
    `entity_shaking` reproduces vanilla `isShaking` — the base `isFullyFrozen` plus
    the per-renderer conversion overrides that are synced to the client:
    `AbstractZombieRenderer` ORs in `Zombie.isUnderWaterConverting()`
    (`DATA_DROWNED_CONVERSION_ID`, id `18`) for the whole zombie family, and
    `ZombieVillagerRenderer` additionally ORs in `ZombieVillager.isConverting()`
    (`DATA_CONVERTING_ID`, id `19`). `StriderRenderer` additionally ORs in
    `StriderRenderState.isSuffocating` from synced `Strider.DATA_SUFFOCATING`
    (id `19`), the same flag used by the cold texture swap. While shaking, the scene folds
    `cos(floor(ageInTicks) * 3.25) * π * 0.4` (degrees) into the projected
    `body_rot`, computed against the integer `ageInTicks` (= `Mth.floor`, so no
    partial lerp); the net head-look yaw is taken against the unshaken body yaw, so
    the whole model jitters while the head turn relative to the body is unchanged.
    Remaining gap: the conversion shakes that are not a synced client flag — the
    hoglin/piglin zombification shake (environment-attribute derived, server-side)
    and the base-`Skeleton` freeze-conversion shake (server-side `conversionTime`).
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
    - texture-backed sheep base submissions preserve entity light plus the full
      hurt/white `OverlayTexture`, while wool and undercoat layer submissions
      preserve entity light and clear the white overlay column like vanilla
      `renderColoredCutoutModel(... getOverlayCoords(state, 0.0F))`
  - Finish remaining sheep presentation parity:
    - implement GPU invisible glowing outline presentation; base and wool outline
      submission metadata plus CPU-side folded outline geometry are now recorded
    - implement colored-path force-transparent output and remaining base-model
      outline handling
  - Finish wolf presentation parity:
    - registry-driven wolf variants are DONE: the synced `Wolf.DATA_VARIANT_ID`
      (index 23) `Holder<WolfVariant>` is resolved (dynamic `wolf_variant`
      registry order, static `WolfVariants.bootstrap` fallback, `Pale` default)
      into `EntityModelKind::Wolf { variant }`, which selects the full vanilla
      `Wolf.getTexture` set — all nine coats (pale/spotted/snowy/black/ashen/
      rusty/woods/chestnut/striped) × wild/tame/angry × adult/baby
      (`bee[...]`→`wolf_<coat>[_tame|_angry][_baby].png`), the 48 new biome faces
      joining the master atlas array (→359)
    - finish colored-path force-transparent / outline presentation and remaining
      render-state extraction parity (armor, sitting/head/tail/walk pose, wet shade
      tint, water-shake roll pose, packed lighting, white overlay, and the hurt
      red overlay are now applied)
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
    - textured entity model submits now carry vanilla-shaped submission
      metadata alongside the existing mesh buckets: `render_type`
      distinguishes `entitySolid`, `armorCutoutNoCull`, `armorTranslucent`,
      `entityCutout`, `entityCutoutCull`,
      `entityCutoutZOffset`, `entityTranslucent`,
      `entityTranslucentCullItemTarget`, `Eyes`,
      `breezeWind`, `energySwirl`, and `end_crystal_beam`; `order` mirrors
      `SubmitNodeCollector.order(n)`; `submit_sequence` preserves
      same-order layer order; and `light` / `overlay` preserve per-submit
      `submitModel(... lightCoords, overlayCoords, ...)` inputs, including
      explicit `OverlayTexture.NO_OVERLAY` overrides for vanilla eyes,
      `breezeWind`, energy-swirl, equipment, cape/wings, wolf-collar layers,
      object/projectile renderer submits (arrows, tridents, evoker fangs, llama spit,
      shulker bullets, wither skulls, and end-crystal bodies), and crystal-beam submissions, while the GPU
      backend still folds compatible submits into shared meshes and now fills standard folded mesh
      vertices from each submission's resolved light / overlay instead of a later per-entity overwrite.
      NO_OVERLAY selection is keyed by pass identity rather than texture alone, so `wither.png` can be
      no-overlay for `WitherSkullRenderer` without clearing the Wither boss body overlay.
      The render-type expression is pinned by
      vanilla-name and mesh-bucket tests, so `entityCutout`,
      `entityCutoutCull`, `entityCutoutZOffset`, `entityTranslucent`,
      `entityTranslucentCullItemTarget`, `Eyes`, `breezeWind`, and
      `energySwirl` stay distinct at the submission boundary even when the
      current backend can fold compatible output into shared mesh buffers.
      Texture-backed invisible-but-visible-to-client living base bodies now
      override their base submission to `entityTranslucentCullItemTarget` with
      the vanilla `38/255` alpha before folding into the translucent mesh; layer
      passes that vanilla gates on `state.isInvisible` still do not submit.
      WindCharge `breezeWind` scroll submits now run through the shared dispatch sink rather than a
      residual textured arm, preserving vanilla order-0 submission metadata and folding through the
      shared scrolled helper only after atlas lookup. Breeze wind and `energySwirl` overlay helpers
      likewise use explicit layer-pass metadata through the pass-backed no-overlay emitter before the
      scroll helper, with missing-atlas tests pinning that submission metadata is recorded before
      folded geometry is suppressed; Guardian
      attack beams now consume explicit `GuardianBeam` pass metadata through the same pass-backed emitter before
      recording vanilla `entityCutout` submissions and folding their tiled custom geometry into the scroll bucket through the
      custom scroll-geometry submission helper; End Crystal
      now creates its vanilla `entityCutout` submission before the residual
      bob/spin geometry is folded through the standard submission helper, and
      crystals with a beam target now consume explicit `EndCrystalBeam` pass metadata through the pass-backed emitter before
      recording vanilla `end_crystal_beam` submissions and folding their tiled prism geometry into the scroll
      bucket with preserved light / no-overlay metadata. Ender dragon nearest-crystal healing beams
      now project the
      bobbed crystal `beamOffset`, consume explicit `EnderDragonBeam` pass metadata through that emitter,
      record the same vanilla `end_crystal_beam` submission after body+eyes, and then fold the
      shared eight-quad prism
      into the scroll bucket with preserved light / no-overlay metadata.
      Uniform layer passes, Creaking base+eyes submits, Warden retained
      emissive layers, Breeze base/eyes/wind, Shulker color/default base
      submits, Shulker bullet's two submits, WindCharge `breezeWind`,
      wither normal/invulnerable base submits, charged-creeper / wither `energySwirl`,
      humanoid armor `armorCutoutNoCull`, horse/donkey/undead-horse
      base+saddle/body-armor submits, horse markings, villager and zombie-villager
      profession/type/level overlay layer passes, custom-head skull submissions, player
      profile cape plus player WINGS/elytra and spin-attack-effect submissions, strider
      base/saddle/no-baby-saddle submits, armor stand
      visibility/scale base submits, vex/allay/cod/pufferfish/turtle/bat/bee/dolphin
      dispatch-local base submits (now with explicit vanilla model-layer metadata instead of the
      old residual base-pass helper), axolotl color/age base submits,
      feline cat/ocelot base plus collar submits, fox type/age/sleeping base
      submits, frog temperature base submits, panda gene/age base submits,
      guardian/elder base plus attack-beam submits, parrot variant base submits, pufferfish
      puff-state base submits, rabbit color/age/Toast base submits,
      sniffer base submits, and the
      Guardian beam / End Crystal body+beam / Ender Dragon body+eyes+beam paths are covered by source-verified
      texture/render-type/tint/transform/order tests, including the Guardian,
      End Crystal, and Ender Dragon beam missing-atlas cases where submission
      metadata survives while folded geometry is suppressed.
    - dropped item entities as camera-facing item-icon billboards from:
      - canonical item entity stack metadata
      - the native item atlas
    - 3D Block-Model / Item-Model Rendering (all four consumers wired): a
      renderer-owned baking layer turns parsed block/item models into 3D textured
      quad meshes sampling the same blocks/items atlas as terrain, for all four
      item consumers (dropped item entities, held items, item frames/armor-stand,
      HUD 3D inventory icons) — every consumer is now implemented; the remaining
      items are refinements (first-person viewmodel, combat arm poses, custom
      ground transforms). Done:
      - `ItemModelQuad`/`ItemModelMesh`/`bake_item_model_mesh`
        (`item_models.rs`): corners in vanilla `0..=16` model space normalized
        to the unit cube under a caller `transform`, atlas-absolute UVs, vertex
        color `tint × Direction.getShade`.
      - block-item baker (`terrain/mesh/item_bake.rs`
        `bake_block_item_quads`): reuses the terrain box/quad geometry, atlas UV
        mapping, and directional shade to turn a block's `TerrainRenderShape`
        (`Cube`/`Box`/`Boxes`/`Quads`) into item-model quads with no neighbour
        culling or AO; `Cross`/`Crosses` bake nothing.
      - generated-item extrusion (`generated_item.rs`
        `bake_generated_item_quads`): faithful vanilla `ItemModelGenerator` — a
        `builtin/generated` `layerN` sprite becomes a `1/16`-thick slab (front
        `SOUTH` + back `NORTH` over `0..=16`, plus per-pixel side faces tracing
        the alpha silhouette), corners via `FaceInfo`/`FaceBakery` and UVs via
        `CuboidFace` `R0`, rendered un-culled.
      - GPU item-model draw pass: the renderer draws baked `ItemModelMesh`es
        with one item-model pipeline against two atlases — block-items sample
        the blocks atlas (terrain bind group), flat/generated items sample the
        item atlas (the dropped-item billboard bind group) — via
        `set_block_item_model_meshes` / `set_flat_item_model_meshes`, solid
        (depth-tested + depth-writing) and un-culled, in a `Load` pass before
        the billboards.
      - first consumer DONE (dropped items, both paths): every dropped item
        entity renders as a 3D model instead of a billboard (native
        `item_models`). Block items bake their block render shape over the
        blocks atlas; all other items extrude their flat sprite into a
        `1/16`-thick slab over the item atlas (the per-sprite alpha silhouette
        is read from the item atlas, inverting the half-texel UV inset; opaque
        iff `alpha != 0`). Both are placed by vanilla `ItemEntityRenderer`'s bob
        + Y spin composed with the model's GROUND display transform (block
        `[0,3,0]/16` scale `0.25`; flat `[0,2,0]/16` scale `0.5` with vanilla's
        `minOffsetY` ground-seating lift), clocked by world game time + partial
        tick with a per-entity phase. A stack renders the vanilla cluster of
        `1..=5` jittered copies (`getRenderedAmount` by stack size;
        `submitMultipleFromCount` scatters thick models in 3D and stacks thin
        ones back-to-front, jittered by a faithful Java LCG seeded on the item
        id). These entities are excluded from the billboard path; thrown-item
        projectiles keep their billboard.
      - second consumer DONE (held items): a humanoid carrying an item renders
        it as a 3D model attached to the posed arm bone, for both hands. The
        renderer exposes the hand world transform via
        `humanoid_hand_attach_transform` (`entity_models/held_item.rs`): vanilla
        `ItemInHandLayer` + `HumanoidModel.translateToHand` =
        `root.translateAndRotate · arm.translateAndRotate · rotX(-90°) ·
        rotY(180°) · T((left?-1:1), 2, -10)/16` (built on
        `ModelPart::try_child_attach_transform`). It dispatches over the
        weapon-holding adult humanoid families — players, zombies (+husk /
        drowned / zombie-villager), skeletons (+stray / bogged / wither),
        piglins (+brute / zombified), and illagers — posing each family's own
        model + root transform and reading its `right_arm` / `left_arm` bone
        (degrading to no item if a family lacks a standard arm). Baby zombies,
        zombie variants, and piglins hold items too: vanilla bakes their reduced
        proportions into an explicit baby mesh with no part scale
        (`BabyZombieModel` / `BabyPiglinModel.createBodyLayer`, vs the scale-based
        `BABY_TRANSFORMER` only ARMOR_STAND_SMALL uses), so the baby attach reuses
        the same `root · arm` formula on the baby model and only swaps the
        `ItemInHandLayer` `useBabyOffset` offsets to `(0, 1, -4.5)/16` (X drops to
        0, so the left/right split comes only from the arm bone). The off-hand item
        attaches to the left arm and uses the item's `thirdperson_lefthand`
        transform with vanilla's left-hand fix (`display_matrix` negates
        `translation.x`, `rotation.y`, `rotation.z`). Native `held_item_models`
        iterates every entity instance and resolves
        the held stack to the same block/flat quads as the dropped path and
        applies the item's own retained third-person display transform for that
        hand (see per-item display transforms below), so a held sword angles
        on `item/handheld`'s `[0,-90,55]`/`[0,4,0.5]/16` scale `0.85`, a block
        tilts on `block/block`'s `[75,45,0]`/`[0,2.5,0]/16` scale `0.375`, and a
        generated item lies flat on `item/generated`'s `[0,3,1]/16` scale `0.55`,
        merging into the same two atlas draws.
      - per-item display transforms retained: `NativeItemRuntime` now keeps each
        item's resolved model `BlockModelDisplayTransforms` (from the first
        cuboid model, shared across an item's conditional variants) keyed by
        resource id, exposed as `item_display_transform(protocol_id, context)`.
        Native `display_matrix` builds the vanilla `ItemTransform.apply` matrix
        (`T(t)·Rxyz·S·T(-0.5)`, translation already in world units) from any
        context; held items use `thirdperson_righthand`, frames use `fixed`, with
        the parent-model default as the no-model fallback.
      - third consumer DONE (item frames): every item-frame / glow-item-frame
        entity renders as the 3D wooden border plus the framed item (vanilla
        `ItemFrameRenderer`), replacing the placeholder bounds box (now
        `NoRender`). `WorldStore::item_frame_render_states` exposes each frame's
        wall-mounted center, facing wall, `0..=7` item rotation, glow flag,
        framed item, and map flag (from `DATA_DIRECTION` / `DATA_ITEM` /
        `DATA_ROTATION`). Native `item_frames` bakes the border by transcribing
        `block/template_item_frame` (four `birch_planks` bars + the
        `item_frame` / `glow_item_frame` back panel) into the blocks atlas via
        the existing `Boxes` item-bake path, and the framed item to block/flat
        quads with its `FIXED` display transform. The facing wall orients the
        model (`Rx(xRot)·Ry(yRot)`), the item is pushed `0.4375` out and spun by
        its rotation at scale `0.5`. Deferred: the filled-map full-frame render
        (a map frame shows only its border) and the `0.5`-vs-`0.4375` invisible
        offset; the back panel's `15.5` depth is rounded to `15`.
      - fourth consumer DONE (HUD 3D inventory icons): each hotbar slot holding
        a block item renders its block model as a 3D icon (vanilla 3D inventory
        item rendering) instead of the flat 2D sprite. Native
        `hotbar_block_item_models` resolves each slot's block-model quads (over
        the blocks atlas) plus its `gui` display transform into a
        `HudBlockItemModel`; `set_hud_hotbar_block_item_models` hands them to the
        renderer. A dedicated GUI item pass draws after the 2D HUD with its own
        `gui_ortho` camera (vanilla `setupOrtho(0, w, h, 0, -1000, 1000)`,
        invert-Y), a separate `gui_item_camera_buffer` (so the same-submit world
        camera write isn't clobbered), color `Load` + a fresh depth `Clear` (per
        vanilla's per-slot depth clear so each icon's faces sort within its slot),
        reusing the existing item-model pipeline against the resident blocks
        atlas. `gui_item_slot_placement` seats each model in its slot pixel rect
        (`translate(slot_center)·scale(px, -px, px)`); the model's own `gui`
        display transform centers and tilts it. Verified end-to-end by a headless
        llvmpipe readback test (`hud_block_item_renders_visible_pixels_in_its_slot`)
        that renders a block icon and asserts visible non-background pixels in the
        slot. Flat / generated items keep their 2D sprite (no 3D model). The 2D
        HUD layer's flat block-texture stand-in is suppressed for any slot that
        renders a 3D model (`push_hud_item_icon`'s `skip_layers`), keeping the
        count / durability / cooldown overlays the 3D pass doesn't draw — so the
        flat square no longer peeks out behind the iso block's silhouette.
      - inventory-screen 3D block icons DONE: the same pass also renders the open
        inventory / container screen's block items as 3D — every container slot
        plus the floating merchant-trade and stonecutter-recipe preview items.
        `HudInventorySlot` / `HudInventoryItem` carry an optional `block_model`;
        the native `hud_inventory_screen_with_local_state` (+ merchant / stonecutter
        floating producers) resolve it via the shared `block_item_3d_model`, and
        `collect_hud_block_item_mesh` bakes each at `inventory_slot_item_hud_rect`.
        The cursor-carried item is whatever the container slots hold, so it is
        covered too.
      - dropped-item GROUND transform now per-item: each dropped item uses its own
        retained `ground` display transform (custom rotation / scale / offset),
        falling back to the vanilla `block/block` or `item/generated` default. The
        ground-seating lift is computed per-model from the rendered bounds —
        vanilla `minOffsetY = -modelBoundingBox.minY + 1/16` over the baked quads
        under the ground matrix — and the same bounds drive the cluster layout
        (`getZsize()`), so a custom transform or a non-full-height sprite seats and
        clusters exactly as vanilla (replacing the old hardcoded
        block=0 / flat=0.1875 lift, which the new path reproduces for defaults).
      - armor-stand held items DONE (full and small): armor stands render their
        hand items as 3D models on their posed arm bone (vanilla
        `ArmorStandRenderer`'s `ItemInHandLayer`; `useBabyOffset` is false for
        ARMOR_STAND, so both sizes take the adult `(1, 2, -10)/16` offset). The
        held-item dispatch builds the posed `ArmorStandModel` and reads its
        `right_arm` / `left_arm`; for `ModelLayers.ARMOR_STAND_SMALL`, the hand
        transform carries `HumanoidModel.BABY_TRANSFORMER`'s 0.5 arm-part scale,
        so the item rides the same small-model scale as vanilla. The world already
        exposes armor-stand hand equipment (`EntityEquipment`, no living-entity
        gate).
      - villager / wandering-trader crossed-arms held items DONE:
        `CrossedArmsItemLayer` is reproduced for `VillagerModel` /
        `BabyVillagerModel` / `WanderingTraderModel`: renderer builds and poses
        the matching model, reads the combined `arms` part (`translateToArms`),
        applies the shared `Rx(0.75) · scale(1.07) · T(0,0.13,-0.34) · Rx(π)`
        transform, and native bakes the main-hand stack with
        `ItemDisplayContext.GROUND`.
      - generic non-skull custom-head items DONE:
        `CustomHeadLayer` now renders head-slot items that are not humanoid
        armor and not skull block items. Native mirrors
        `LivingEntityRenderer.extractRenderState`: items with a humanoid armor
        equipment asset stay on `HumanoidArmorLayer`, skull block items are
        reserved for the dedicated `SkullBlockRenderer` path, and everything
        else in `EquipmentSlot.HEAD` is baked with `ItemDisplayContext.HEAD`.
        Renderer exports the posed head transform for the vanilla custom-head
        carriers (players, zombie/skeleton/piglin families, illagers,
        villagers/wandering traders, armor stands, and copper golems),
        including piglin horizontal scale, villager y-offset, and the copper
        golem `body -> head -> T(0,0.125,0) -> scale(1.0625)` override.
      - custom-head static mob skulls, player-head default skins, dragon head, and piglin head DONE:
        `CustomHeadLayer`'s skull branch now renders `skeleton_skull`,
        `wither_skeleton_skull`, `zombie_head`, and `creeper_head` in the
        HEAD slot using vanilla `SkullModel.createMobHeadLayer()` geometry,
        the `SKULL_SCALE = 1.1875` transform, villager skull y-offset, and
        the matching entity textures via vanilla `entityCutoutZOffset`
        submissions generated from `custom_head_skull_layer_pass`, including
        vanilla `ModelLayers.SKELETON_SKULL` / `WITHER_SKELETON_SKULL` /
        `ZOMBIE_HEAD` / `CREEPER_HEAD` / `PLAYER_HEAD` / `DRAGON_SKULL` /
        `PIGLIN_HEAD`. The custom-head skull regressions now pin submission
        metadata for static, piglin, dragon, default-player, profiled-default,
        and dynamic-player heads: selected texture or dynamic skin handle,
        `entityCutoutZOffset` or `entityTranslucent`, white tint, skull
        transform, entity `lightCoords`, vanilla `OverlayTexture.NO_OVERLAY`,
        and `(order, submit_sequence) == (0, 0)` before folded
        cutout/translucent/dynamic geometry checks; missing-atlas coverage now
        proves static skull submissions survive without folded stale skull
        geometry. Native resolves these skull
        block items from the item
        registry into `EntityRenderState.custom_head_skull`. A `player_head`
        whose stack has no active `DataComponents.PROFILE` component is
        rendered with vanilla `DefaultPlayerSkin`
        (`textures/entity/player/slim/steve.png`) and the humanoid head+hat
        skull layer. A profiled `player_head` now follows
        `PlayerSkinRenderCache`'s default fallback: it picks the vanilla
        `DefaultPlayerSkin.get(UUID)` slot from an explicit profile UUID, an
        offline-name UUID, or the nil UUID, and applies a `PlayerSkin.Patch`
        body only when it names one of the built-in default player-skin
        resources; profiled default-player heads use vanilla
        `PlayerSkinRenderCache.renderType()` semantics and therefore record
        `entityTranslucent` submissions even while sampling a fallback default
        skin. A `piglin_head` renders with vanilla
        `PiglinHeadModel` head/ear geometry and uses
        `wornHeadAnimationPos` for the skull ear flap, including vanilla's
        riding-a-living-entity branch that reads the vehicle walk animation. A
        `dragon_head` renders with vanilla
        `DragonHeadModel` head/jaw geometry, the skull head `scaled(0.75)` part
        pose, the `dragon.png` texture, and `wornHeadAnimationPos` jaw
        animation. Protocol now preserves `DataComponents.PROFILE` (type id
        70) as a structured `ResolvableProfileSummary`, including full vs
        partial profile, UUID/name, profile properties, `PlayerSkin.Patch`
        resource texture/model overrides, and the unpacked profile `textures`
        property URLs for skin/cape/elytra plus the vanilla slim/wide model
        selection (`metadata.model=slim`, otherwise wide when a skin URL is
        present). Native/render-state can now distinguish profileless default
        skins from profiled fallback skins and can carry a dynamic skin handle,
        the fallback default skin, the slim/wide model, and explicit
        loading/ready/failed status. Native also keeps a small dynamic
        profile-skin cache keyed by texture URL; async download/upload work
        replaces the request handle with a resolved texture handle only after
        renderer upload succeeds. Renderer now has
        the vanilla downloaded-skin PNG post-process primitive: it rejects
        non-PNG and non-64x64/64x32 skins, expands legacy 64x32 skins through
        `SkinTextureDownloader.processLegacySkin`'s copy rectangles, and applies
        the opaque-base / Notch transparency alpha rules to produce 64x64 RGBA
        data for dynamic upload. Native now also has a fetcher-backed
        player-skin runtime cache that reuses decoded skins from memory, loads
        cached PNG bytes from disk before fetching, writes fetched bytes to disk,
        and feeds every hit/miss through the vanilla-compatible PNG post-process.
        A blocking reqwest/rustls HTTP fetcher is available behind that boundary
        and is covered with loopback HTTP tests. Native now also has a
        vanilla-shaped `ResolvableProfile` resolution/cache primitive: only
        dynamic partial profiles with empty properties and exactly one of
        name/UUID are resolved; invalid names and misses keep the default
        fallback, and the HTTP fetcher parses Mojang name->UUID plus session
        profile/properties responses. The native main loop explicitly enables
        an async profile-resolution worker and drains completed results; a
        `player_head` keeps the fallback while pending/failed and uses the
        resolved profile/properties once available. The native main loop also
        enables an async player-skin download worker with a configurable cache
        directory; dynamic skin URLs queue PNG download/cache/post-process work,
        failed downloads mark the profile skin failed, and successful downloads
        are uploaded into a renderer-owned dynamic player-skin atlas before the
        profile skin is marked Ready. Ready `player_head` submissions use
        vanilla `entityTranslucent` with the dynamic atlas mesh; Loading/Failed
        or missing atlas entries keep sampling the fallback default skin. Player
        entity bodies now also use vanilla player skins: without PlayerInfo they
        reproduce `DefaultPlayerSkin.get(uuid)` across the 18 built-in defaults;
        with PlayerInfo profile skins the native model kind carries
        `EntityPlayerSkin`, picks wide/slim from `PlayerSkin.model`, and Ready
        dynamic body submissions sample the dynamic player-skin atlas through a
        cutout mesh while preserving vanilla `entityCutout` submission metadata.
        Native now also has the ordinary profile-texture half for cape/elytra:
        renderer decodes non-skin profile PNGs without legacy skin
        post-processing, native keeps separate memory/disk caches for cape and
        elytra textures, profile cape/elytra URLs queue async downloads, and
        the main loop uploads successful results into a renderer-owned
        variable-size dynamic player texture atlas. `PlayerSkin.Patch`
        resource-texture overrides are also covered for body/cape/elytra:
        native resolves the patch `texture_path` through the pack resource
        stack, decodes body skins into the dynamic player-skin upload path,
        decodes cape/elytra PNGs without skin post-processing, and avoids the
        remote profile URL download for overridden textures. Renderer can now
        record dynamic player texture submissions and draw cutout/translucent dynamic
        profile-texture buckets bound to that atlas, with static-atlas fallback
        when the dynamic entry is absent. Profile `CapeLayer` presentation is
        now covered for player entities: native projects the PlayerInfo cape URL as an
        `EntityDynamicPlayerTextureKind::Cape`, and renderer emits the cape
        layer only when the cape model part is visible and the dynamic atlas
        entry is ready. That submission now consumes `player_cape_layer_pass`
        with vanilla `ModelLayers.PLAYER_CAPE`, recording `entitySolid`, the
        dynamic cape handle, white tint, root transform, default order 0 plus
        the layer submit sequence, and vanilla `OverlayTexture.NO_OVERLAY`;
        missing atlas entries wait instead of drawing stale geometry. Pack/native
        now preserve item equippable asset ids and query equipment asset layers,
        so the cape is suppressed for chest WINGS equipment and nudged by the
        vanilla HUMANOID chest-equipment translation, with the nudged `entitySolid`
        submission also pinning texture, dynamic handle, white tint, light/no-overlay
        metadata, and folded cape vertex inheritance. Player `WingsLayer` / elytra
        presentation is also covered for vanilla elytra equipment: native projects
        the chest WINGS layer texture/use-player-texture metadata, renderer now consumes
        `wings_layer_pass` for an `ElytraModel` `armorCutoutNoCull` submission with vanilla
        `ModelLayers.ELYTRA` / `ModelLayers.ELYTRA_BABY`, order 0, the vanilla
        `z=0.125` layer transform, and `OverlayTexture.NO_OVERLAY`, prefers ready profile elytra texture over cape,
        falls back to a ready profile cape when the cape part is visible, uses the
        static equipment elytra texture when no profile override exists, and waits
        when an override texture has not been uploaded. The profile elytra/cape
        WINGS branches now also pin entity light plus no-overlay metadata on both
        the submission and folded dynamic texture vertices. World/native also project
        vanilla `LivingEntity.elytraAnimationState` rotX/Y/Z into the renderer,
        and the same WINGS path now covers humanoid mobs, armor stands (small
        armor stands select the baby layer through `ArmorStand.isBaby()`), and
        baby humanoid mob `ELYTRA_BABY` geometry; their static equipment wings
        submissions now also pin entity light plus `OverlayTexture.NO_OVERLAY`,
        with folded elytra vertices inheriting that metadata, and missing-atlas
        coverage proves the static player and non-player WINGS submissions survive without
        stale elytra geometry. Player cloak interpolation is
        now also covered: world tracks the vanilla `ClientAvatarState` cloak
        lag, partial lerps it, applies `AvatarRenderer.extractCapeState`
        flap/lean/lean2 clamps and fall-flying lean suppression, and native
        forwards those values into the renderer cape layer. Still deferred:
        broader non-profile dynamic texture loading.
      - fox held item DONE: `FoxHeldItemLayer` is reproduced through the same
        item-model pass. Renderer exposes `fox_held_item_transform`, which builds
        and poses the vanilla adult/baby `FoxModel`, reads the posed `head` part,
        applies the layer's baby `0.75` scale, sleep/non-sleep mouth offsets, the
        `rotX(90°)` upright rotation, and the sleeping `rotZ(90°)` spin. Native
        reads the fox main-hand equipment stack, uses vanilla
        `HoldingEntityRenderState`'s `ItemDisplayContext.GROUND`, and bakes block
        or generated item quads with the retained ground display transform.
      - family combat arm poses, first pose DONE (skeleton bow-aim): the
        `BOW_AND_ARROW` `ArmPose` raises both arms forward along the head look when
        a skeleton `isAggressive() && getMainHandItem().is(Items.BOW)`. The
        cross-crate state is now projected — `entity_model_instance` reads
        `EntityEquipment` via `NativeItemRuntime` to set `main_hand_holds_bow`
        (gated to the skeleton kinds), pairing with the already-projected
        `is_aggressive`; `SkeletonModel`/`SkeletonClothingModel.setupAnim` apply it
        after the walk swing. See the skeleton entry below. The held bow *mesh*
        comes for free: the held-item attach reads the same posed model, so the
        bow in the right hand rises to the horizontal aim with the arm (no extra
        wiring).
      - melee attack swing DONE for the player (`HumanoidModel.setupAttackAnimation`
        WHACK + the spear `STAB` lunge): the swing timer is now projected
        (`attack_anim`/`attack_arm_off_hand` from the `ClientboundAnimate` packet, see
        the player entry above). Non-player default WHACK arm poses are also covered where vanilla
        uses model-specific branches: zombie-family `animateZombieArms`, skeleton melee,
        and vindicator empty/armed `ATTACKING`; illager riding sit pose is also covered
        through the projected passenger state. STAB/NONE swing-type parity on non-player
        humanoids remains separate work.
      - remaining slices: held-item refinements (first-person viewmodel;
        broader non-profile dynamic texture loading; the
        STAB swing pose on non-player humanoid models; the `NONE` swing type; the
        attack swing on the non-player humanoid models). Item lighting
        context (GUI front-lit vs world diffuse) is an open point — the baked
        `shade` currently uses the terrain cardinal `Direction.getShade` for both
        block- and generated-items.
    - thrown-item projectiles (egg, snowball, ender pearl, eye of ender, splash/lingering potion,
      experience bottle, large fireball, small fireball) as camera-facing item-icon billboards on the
      same path: vanilla's `ThrownItemRenderer` draws each as the item sprite of its carried
      `DATA_ITEM_STACK` (the same data id `8` the dropped item uses), centered on the entity (no
      dropped-item lift offset) at that renderer's sprite scale (`poseStack.scale(scale)`: `1.0` for all
      but the large fireball `3.0` and small fireball `0.75`, carried per-billboard on
      `ItemEntityBillboard::scale`). The native entity scene maps these types to
      `EntityModelKind::NoRender` (the 3D model scene draws nothing) and the billboard layer
      (`item_entity_billboards_from_world` over `THROWN_ITEM_PROJECTILE_BILLBOARDS` via
      `WorldStore::item_stacks_for_entity_types`) emits the sprite. The `fullBright` lighting flag (eye of
      ender, fireballs) is a no-op on the unlit billboard shader
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
      0.5`, sleeve children riding them) plus the always-on `HumanoidModel.setupAnim`
      idle arm bob (`AnimationUtils.bobModelPart` → `humanoid_arm_bob_pose`, applied to
      both arms every frame on top of the swing so even a standing player's arms move
      with `ageInTicks`) — all applied once to the shared
      visibility-filtered part array (colored and textured); true
      `RenderTypes.entityTranslucent` alpha blending, armor/equipment, held items,
      arrows/stingers, spectator visibility, the elytra flying offsets, name
      display, the held-item/attack/swim arm poses, and the elytra
      `speedValue` poses remain unsupported; the `HumanoidModel` crouch
      (`isCrouching`/`Pose.CROUCHING`, projected as `is_crouching`) sneaking pose is
      implemented on both render paths — the body leans (`xRot = 0.5`) and drops, the
      head drops, the arms tilt (`xRot += 0.4`) and the legs tuck back (`z += 4`)
      (metadata-driven `DATA_PLAYER_MODE_CUSTOMISATION` projection now controls
      hat/jacket/sleeves/pants overlay visibility for the texture-backed base
      player/mannequin model, and the cape bit is preserved in renderer
      visibility state. Static player textured regressions now route through
      `entity_model_textured_meshes`, pinning the base `entityCutout` submission's
      default wide/slim Steve texture, white tint, `player_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)` plus vanilla `LivingEntityRenderer`
      `lightCoords` and hurt/white overlay metadata before folded cutout checks for UVs,
      folded vertex light/overlay inheritance, overlay part visibility, head-look,
      walk swing, idle arm bob, and crouch; dynamic player-skin base submissions now
      consume the same `player_textured_layer_passes_with_texture` metadata before
      selecting the ready dynamic skin atlas. Deadmau5 ears are implemented from the
      exact lowercase player-info GameProfile name `"deadmau5"` through
      `Deadmau5EarsLayer` / `PlayerEarsModel`: visible players submit an
      `entitySolid` same-skin layer with zero-white overlay at player same-order
      sequence 1, now through `player_extra_ears_layer_pass_with_texture` with vanilla
      `ModelLayers.PLAYER_EARS`, including ready dynamic player skin atlas support. Player profile
      cape presentation is covered by the
      dynamic `entitySolid` cape layer through `player_cape_layer_pass` with vanilla
      `ModelLayers.PLAYER_CAPE`, and WingsLayer/elytra presentation now consumes
      `wings_layer_pass` with vanilla `ModelLayers.ELYTRA` / `ModelLayers.ELYTRA_BABY` plus
      elytra animation-state projection are covered for vanilla elytra equipment
      on players, humanoid mobs, armor stands, and baby humanoid mobs; player
      cloak interpolation now feeds the cape flap/lean/lean2 values. Player shoulder
      parrots are now projected from `Player.DATA_SHOULDER_PARROT_LEFT/RIGHT`
      metadata ids `19`/`20` (`OPTIONAL_UNSIGNED_INT`) into `AvatarRenderState`
      equivalents; `ParrotOnShoulderLayer` is represented as explicit left/right
      same-order submissions after `WingsLayer` and before riptide spin, using
      vanilla `ModelLayers.PARROT`, `ParrotRenderer.getVariantTexture`, `entityCutout`,
      white tint, player light, `OverlayTexture.NO_OVERLAY`, transforms
      `translate(±0.4, isCrouching ? -1.3 : -1.5, 0)`, and sequences `4` / `5`.
      The riptide `SpinAttackEffectLayer` therefore moves to sequence `6`.)
    - wooden boat, chest boat, bamboo raft, and bamboo chest raft entities as
      renderer-owned vanilla 26.1 `BoatModel` / `RaftModel` body-layer
      geometry from `BoatModel`, `RaftModel`, `BoatRenderer`, `RaftRenderer`,
      `AbstractBoatRenderer`, and `LayerDefinitions`, including boat hull
      parts, raft bottom logs, paddles, chest bottom/lid/lock parts, official
      per-wood/per-bamboo texture references, and the vanilla boat root
      translate/rotate/scale/rotate renderer transform, texture-backed base
      layer pass emission, boat/chest-boat/raft/chest-raft model-layer
      selection, and official PNG atlas upload/bind/sample path. Tests now pin
      explicit base submission metadata for vanilla `entityCutout`, selected
      texture, white tint, `boat_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)`, with renderer light plus
      `OverlayTexture.NO_OVERLAY` inherited by folded cutout vertices; `boat_textured_layer_passes`
      now also records the above-water `ModelLayers.BOAT_WATER_PATCH` submit as vanilla `waterMask`
      metadata at `(0, 1)` with the same texture/transform/light and `OverlayTexture.NO_OVERLAY`, but its
      depth-only GPU presentation is still deferred. Paddle rowing animation is
      projected from `AbstractBoat` metadata ids 11/12 plus the controlling-passenger
      gate and rendered through `AbstractBoatModel.animatePaddle`; hurt/damage roll
      is projected from `VehicleEntity` metadata ids 8/9/10 and folded into the boat
      root transform. Bubble wobble is projected from `AbstractBoat.DATA_ID_BUBBLE_TIME`
      id 13 into the vanilla multiplier/angle tick state and folded into the boat root
      transform with the renderer-side underwater gate. Underwater state is projected
      from the vanilla boat top-slice water-surface test and now gates both bubble wobble
      and above-water `waterMask` submission. Water-mask GPU presentation and lighting
      remain unsupported
    - chicken entities as renderer-owned vanilla 26.1
      `AdultChickenModel`, `ColdChickenModel`, and `BabyChickenModel` body-layer
      geometry from `ChickenModel`, `ChickenRenderer`, `ChickenVariants`, and
      `LayerDefinitions`, including metadata-driven temperate/warm/cold
      variant projection through the server-sent `minecraft:chicken_variant`
      registry order, official adult/baby variant texture references, and
      vanilla fallback to temperate when no variant metadata is present,
      texture-backed base layer pass emission, adult/baby/cold model-layer
      selection, official PNG atlas upload/bind/sample path, and tests that now pin
      emitted base submissions as vanilla `entityCutout` with selected variant texture,
      white tint, `entity_model_root_transform`, and `(order, submit_sequence) == (0, 0)`,
      plus `MobRenderer` / `LivingEntityRenderer` `lightCoords` and hurt/white overlay metadata,
      before folded cutout geometry checks, and the vanilla
      `ChickenModel.setupAnim` two-leg walk swing (the `HumanoidModel` phase
      `cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at `[2, 3]` adult/cold and
      `[1, 2]` on the headless baby layer, on both render paths and every variant
      pass), plus the vanilla wing flap: world mirrors `Chicken.aiStep`'s client
      `flap`/`oFlap`/`flapSpeed`/`oFlapSpeed`/`flapping` accumulator from `onGround`,
      store/native project the partial-lerped `ChickenRenderState.flap` and `flapSpeed`,
      and `ChickenModel.setupAnim` applies `(sin(flap) + 1) * flapSpeed` to
      `right_wing.zRot` / `-left_wing.zRot` on adult, cold, and baby models. The
      chicken has no head look in vanilla (`ChickenModel` never animates the head).
      Variant sound metadata, custom/datapack chicken variant asset decoding, and broader lighting presentation
      remain unsupported
    - pig entities as renderer-owned vanilla 26.1
      `PigModel`, `ColdPigModel`, and `BabyPigModel` body-layer geometry from
      `PigModel`, `ColdPigModel`, `BabyPigModel`, `PigRenderer`,
      `PigVariants`, and `LayerDefinitions`, including normal/warm adult base
      model reuse, cold adult body overlay geometry, baked baby
      `CubeDeformation` bounds, metadata-driven temperate/warm/cold variant
      projection through the server-sent `minecraft:pig_variant` registry
      order, official adult/baby variant texture references, vanilla fallback
      to temperate when no variant metadata is present, texture-backed base
      layer pass emission, explicit base submission metadata for vanilla
      `entityCutout`, the selected variant texture, white tint,
      `entity_model_root_transform`, `(order, submit_sequence) == (0, 0)`, and
      `MobRenderer` / `LivingEntityRenderer` light plus hurt/white overlay metadata,
      adult/baby/cold model-layer selection, official PNG atlas upload/bind/sample path,
      and the vanilla
      `QuadrupedModel.setupAnim` head-look yaw/pitch on the head part plus the
      standard diagonal leg walk swing on adult, baby, warm, and cold models.
      The adult pig saddle equipment layer is implemented from `EquipmentSlot.SADDLE`
      through the default item equipment-slot map, using vanilla `PIG_SADDLE` /
      `PigModel.createBodyLayer(CubeDeformation(0.5F))` and
      `textures/entity/equipment/pig_saddle/saddle.png`, with submission metadata
      generated from `equipment_layer_pass` for vanilla `ModelLayers.PIG_SADDLE`,
      `armorCutoutNoCull`, white tint, `entity_model_root_transform`, and
      `(order, submit_sequence) == (0, 1)`, preserving entity light while forcing the vanilla equipment
      `OverlayTexture.NO_OVERLAY`; missing saddle atlas data now preserves that
      submission while suppressing only the folded saddle geometry; baby pigs intentionally
      skip it because vanilla provides no baby saddle model. Boost/ridden
      animation, variant sound metadata, custom/datapack pig variant asset
      decoding, and broader lighting presentation remain unsupported
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
      upload/bind/sample path, and explicit base submission metadata for vanilla
      `entityCutout`, selected texture, white tint, `entity_model_root_transform`,
      `(order, submit_sequence) == (0, 0)`, and `MobRenderer` / `LivingEntityRenderer`
      light plus hurt/white overlay metadata before folded cutout checks, and the
      vanilla `QuadrupedModel.setupAnim`
      head-look yaw/pitch on the head part plus the standard diagonal leg walk
      swing (`cos(pos * 0.6662 [+ pi]) * 1.4 * speed`) on adult, baby,
      warm, and cold textured models; variant sound metadata,
      custom/datapack cow variant asset decoding, and broader lighting presentation remain
      unsupported
    - sheep entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `SheepModel`, `BabySheepModel`, and `SheepRenderer`, with
      official base/wool/undercoat texture references, texture-backed base,
      wool, and undercoat layer passes, metadata-driven sheared state, and dye
      color projection, custom-name `jeb_` color cycling from entity metadata
      and renderer age ticks. Shared dispatch now owns the sheep base body,
      wool, and undercoat submissions instead of residual colored/textured emit
      helpers. Texture-backed tests now pin explicit submission
      metadata for the vanilla `entityCutout` base, wool, and undercoat submits:
      selected adult/baby texture, wool or `jeb_` tint, root transform, entity
      light, base hurt/white overlay versus wool/undercoat zero-white overlay,
      and the vanilla `SubmitNodeCollector.order` / sequence split (base
      `(0,0)`, adult wool `(0,2)`, adult undercoat `(1,1)`, baby wool `(1,2)`)
      before folded UV/light/overlay/visibility/eat-head/walk geometry checks.
      Missing-atlas coverage now proves adult wool, adult undercoat, and baby
      wool submissions are still recorded without their wool textures while
      only folded wool/undercoat geometry is suppressed, before vanilla
      shared-flags invisibility gating for non-glowing wool and undercoat layer
      passes, and the vanilla
      `SheepModel`/`SheepFurModel.setupAnim` eat-grass head pose (`head.y +=
      headEatPositionScale * 9.0 * ageScale`, `head.xRot = headEatAngleScale`)
      projected from entity event `10` and the canonical `eatAnimationTick`
      countdown into the base, wool, and undercoat head part, including the
      non-eating `Sheep.getHeadEatAngleScale` head-look fallback through the
      shared projection (`getXRot(partialTick) * PI/180` while not eating), and
      the texture-backed invisible-but-visible-to-client base body branch
      (`entityTranslucentCullItemTarget`, `38/255` alpha, base order `(0,0)`)
      while wool/undercoat layers remain skipped by `state.isInvisible`; invisible
      glowing sheep now records vanilla base and `SheepWoolLayer` wool outline
      submissions (adult wool order `(0,2)`, baby wool order `(1,2)`) with
      `outlineColor` metadata while `SheepWoolUndercoatLayer` still skips. The
      renderer now retains CPU-side folded outline geometry for the base plus
      wool passes with each submission's tint/light/overlay metadata. GPU
      invisible glowing outline presentation, colored-path force-transparent /
      outline handling, and remaining render-state extraction remain
      unsupported; outline submission metadata is recorded from the shared
      glowing flag and scoreboard team color
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
      swing while the head still follows the look; both render paths). The full
      `Wolf.getTexture` biome-variant texture swap is now implemented (the synced
      `DATA_VARIANT_ID` index-23 `Holder<WolfVariant>` resolved to one of the nine
      coats × wild/tame/angry × adult/baby faces; see the wolf presentation parity
      list above), plus `Wolf.getWetShade(partialTick)` wet-shade tint on the
      base model (`WolfRenderer.getModelTint`: wet wolves start at `0.75` and
      brighten with the `shakeAnim += 0.05` drying timer; the collar layer keeps
      its own dye tint/order), and the water-shake roll pose (`WolfRenderState.shakeAnim`
      feeds `getBodyRollAngle(offset)` for the adult body / real-head / upper-body / real-tail
      and the baby body / head / tail; the collar layer reuses the same rolled pose), and the
      begging/head-roll tilt (`Wolf.DATA_INTERESTED_ID` index 20 drives the client
      `interestedAngleO/interestedAngle` `0.4` ease, projected as
      `WolfRenderState.headRollAngle` and added to adult `real_head` / baby `head`
      on top of `getBodyRollAngle(0)`), and the adult-only wolf armor layer
      (`WolfArmorLayer`: `ModelLayers.WOLF_ARMOR` with `CubeDeformation(0.2)`,
      body-slot `wolf_armor` resolved through the `armadillo_scute` equipment
      asset, the `wolf_body` base and dyeable overlay layers as
      `equipment_layer_pass`-generated `armorCutoutNoCull` submissions at orders `1`/`2`, undyed overlay
      suppression, all wolf armor/collar submissions preserving vanilla
      `OverlayTexture.NO_OVERLAY`, and the low/medium/high durability crack overlays as
      `equipment_layer_pass`-generated `armorTranslucent` submissions at order `3`; unlike
      `WolfCollarLayer`, vanilla `WolfArmorLayer` does not check `state.isInvisible`, so hidden
      invisible wolves still submit armor/crack layers, self-visible invisible wolves submit the
      force-transparent base plus armor/cracks, and invisible-glowing wolves submit the base outline
      plus armor/cracks). Textured wolf UV, head-look, leg-swing,
      tail-wag, tail-droop, sitting, angry-tail, wet-shade, collar, and armor regressions now
      route through `entity_model_textured_meshes`, pinning selected wild/tame/angry/variant
      base textures, adult/baby collar textures, armor/crack textures, `entityCutout` /
      `armorCutoutNoCull` / `armorTranslucent` render type names, tints, `entity_model_root_transform`,
      base entity light plus hurt/white overlay versus collar/armor/crack
      `OverlayTexture.NO_OVERLAY`, and explicit `(order, submit_sequence)` before
      folded cutout/translucent light/overlay geometry checks; missing-atlas coverage now pins that
      adult/baby collar submits are still recorded without `wolf_collar*.png`, suppressing only
      folded collar geometry, and that the medium-damage `armorTranslucent` crack submit is still
      recorded after the base armor layers when only the crack texture is absent, suppressing only
      folded translucent crack geometry, including the texture-backed invisible states:
      the invisible-but-visible-to-client base body branch (`entityTranslucentCullItemTarget`,
      `38/255` alpha, base order `(0,0)`) and the hidden/glowing wolf-armor layer
      exception while the collar layer remains skipped by `state.isInvisible`;
      missing-atlas coverage pins that this force-transparent base submit is still recorded when
      `wolf_tame.png` is absent, suppressing only folded translucent geometry.
      Invisible-glowing wolf base outline submissions now also retain CPU-side
      folded outline geometry with the submission tint/light/overlay metadata,
      including the armor-equipped invisible exception path. Colored-path
      force-transparent / GPU outline presentation, glint/foil, and remaining
      render-state extraction remain unsupported
    - base horse entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `AbstractEquineModel.createBodyMesh(CubeDeformation.NONE)`,
      `BabyHorseModel.createBabyMesh(CubeDeformation.NONE)`, `HorseModel`, and
      `HorseRenderer`, now rendered on the **textured path** with all seven coat
      colors: vanilla `HorseRenderer.LOCATION_BY_VARIANT` then
      `state.isBaby ? variant.baby : variant.adult` picks one of
      `horse_{white,creamy,chestnut,brown,black,gray,darkbrown}(_baby).png`
      (64×64) from the synced `Horse.DATA_ID_TYPE_VARIANT & 0xFF` coat color
      (`Variant.byId`, WRAP), all wired into the entity atlas; the textured body
      reuses the shared `ADULT_HORSE_PARTS_TEXTURED` / `BABY_HORSE_PARTS_TEXTURED`
      trees (same as the undead horses) with the adult `ModelLayers.HORSE`
      `MeshTransformer.scaling(1.1F)` root transform and the unscaled re-parented
      baby layer, and the vanilla `AbstractEquineModel.setupAnim` walking leg swing
      (the equine gait `cos(pos * 0.6662 + π) * speed` at front amplitude `0.8` /
      hind `0.5`, legs at `[2, 3, 4, 5]` adult / `[1, 2, 3, 4]` on the re-parented
      baby layer), the default-branch neck head look/bob (`head_parts.yRot =
      clamp(yRot, -20, 20) * π/180`, `head_parts.xRot = π/6 + xRot * π/180 +
      (speed > 0.2 ? cos(pos * 0.8) * 0.15 * speed : 0)`, at `head_parts` `1` adult /
      `5` baby horse), and the tail walk lift (`tail.xRot = getTailXRotOffset() + π/6 +
      speed * 0.75`, `tail.y += speed * ageScale`, `tail.z += speed * 2 * ageScale`; the
      baby horse `getTailXRotOffset = −π/2` overrides the layer rest angle and `ageScale =
      0.5`, the body subtree hand-emitted so the tail child can swing) drive both the
      textured base body and the colored full-mesh fallback, plus the white-markings
      overlay (`HorseMarkingLayer`): a translucent `horse_markings_{white,whitefield,
      whitedots,blackdots}(_baby).png` copy of the same posed model drawn over the coat
      when the `(DATA_ID_TYPE_VARIANT & 0xFF00) >> 8` markings nibble is non-`NONE`
      (`Markings.NONE` → `INVISIBLE_TEXTURE`, no overlay). The adult horse saddle equipment
      layer is implemented from `EquipmentSlot.SADDLE`, using vanilla `HORSE_SADDLE`,
      `EquineSaddleModel.createSaddleLayer()` with `livingHorseScale=1.1`, the family-specific
      `textures/entity/equipment/horse_saddle/saddle.png`, and ridden-only bridle line
      visibility from passenger state; its submission is now generated from `equipment_layer_pass`
      with vanilla `ModelLayers.HORSE_SADDLE`. Baby horses intentionally skip it because vanilla
      supplies no baby saddle model. The adult horse body-armor equipment layer is also
      implemented from `EquipmentSlot.BODY`, using vanilla `HORSE_BODY`,
      `HorseModel(ModelLayers.HORSE_ARMOR)` with `AbstractEquineModel.createBodyMesh(CubeDeformation(0.1F))`
      plus `livingHorseScale=1.1`, and the official
      `textures/entity/equipment/horse_body/{leather,leather_overlay,copper,iron,gold,diamond,netherite}.png`
      textures; leather uses the dyeable base layer plus white overlay, submissions are generated
      from `equipment_layer_pass` with vanilla `ModelLayers.HORSE_ARMOR`, and baby horses skip it
      because vanilla supplies no baby armor model. The ridden/eat/stand/mouth poses, the
      tail's `ageInTicks` yRot wag, and the in-water leg-frequency scaling remain unsupported
    - donkey and mule entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `DonkeyModel`, `BabyDonkeyModel`, and
      `DonkeyRenderer`. The ADULT donkey/mule now renders on the **textured
      path** with the official `donkey` / `mule` 64×64 textures wired into the
      entity atlas and selected per family/age: the textured trees reuse the
      shared horse `createBodyMesh` textured cubes 1:1 and add only the bigger
      donkey ears (`texOffs(0,12)`, replacing the horse ears) and the two side
      chest boxes (`texOffs(26,21)`, shown when `hasChest`), at the
      `DONKEY_SCALE=0.87F` / `MULE_SCALE=0.92F` mesh-transformer scale, with the
      adult `AbstractEquineModel.setupAnim` walking leg swing (the equine gait,
      legs at `[2, 3, 4, 5]`), the adult default-branch neck head look/bob
      (`head_parts` at `1`, the same yaw-clamp/pitch/walk-bob as the horse, since
      the adult `DonkeyModel` only adds chest visibility over the base
      `setupAnim`), and the adult tail walk lift (the same `getTailXRotOffset = 0`,
      `ageScale = 1` formula as the horse, the tail child swung with the chest
      children kept in place when present) driving both the textured base body and
      the colored full-mesh fallback. The BABY donkey/mule also renders on the
      **textured path** now: vanilla `BabyDonkeyModel.createBabyLayer()` is a
      distinct re-parented mesh (10 cubes nested under the body, with per-leg and
      per-ear `texOffs` and a mirrored right ear) on the `donkey` / `mule` 64×64
      textures, emitted STATIC and unscaled — its `setupAnim` forces `xRot = -30°`,
      so the equine gait/head/tail posing is deferred (matching the colored baby
      path), and the empty chest children make `hasChest` immaterial. The adult donkey/mule
      saddle equipment layers are implemented from `EquipmentSlot.SADDLE`, using vanilla
      `DONKEY_SADDLE` / `MULE_SADDLE`, `DonkeyModel.createSaddleLayer(0.87F/0.92F)`, the
      family-specific `textures/entity/equipment/{donkey_saddle,mule_saddle}/saddle.png`,
      ridden-only bridle line visibility from passenger state, entity light, and
      `OverlayTexture.NO_OVERLAY`; their submissions are generated from `equipment_layer_pass`
      with vanilla `ModelLayers.DONKEY_SADDLE` / `MULE_SADDLE`, while the base `entityCutout` submits keep
      vanilla entity light plus hurt/white overlay coords. Folded cutout vertices
      inherit the corresponding base or saddle submission metadata; baby donkey/mule entities
      intentionally skip the layer because vanilla supplies no baby saddle model. The baby leg
      swing / head look / tail, the ridden/eat/stand/mouth poses, the tail's `ageInTicks`
      yRot wag, and broader lighting presentation remain unsupported
    - skeleton horse and zombie horse entities as renderer-owned vanilla 26.1
      adult/baby body-layer geometry from `AbstractEquineModel`,
      `BabyHorseModel`, `HorseModel`, and `UndeadHorseRenderer`, now rendered on
      the **textured path** (`UndeadHorseRenderer extends HorseRenderer`) with the
      official `horse_skeleton` / `horse_skeleton_baby` / `horse_zombie` /
      `horse_zombie_baby` 64×64 textures wired into the entity atlas and selected
      per `(family, baby)`; the textured trees mirror the colored
      `ADULT_HORSE_PARTS` / `BABY_HORSE_PARTS` geometry 1:1 (identical deformed
      `min`/`size`, the per-cube `uv_size`/`tex`/`mirror` from
      `createBodyMesh` / `createBabyMesh`, mane/upper-mouth, mirrored adult left
      legs), and the shared `AbstractEquineModel.setupAnim` walking leg swing (the
      equine gait, legs at `[2, 3, 4, 5]` adult / `[1, 2, 3, 4]` baby), the
      default-branch neck head look/bob (`head_parts` at `1` adult / `5` baby
      horse, the same yaw-clamp/pitch/walk-bob as the horse it reuses), and the
      tail walk lift (the same formula as the horse, including the baby
      `getTailXRotOffset = −π/2` / `ageScale = 0.5` override) all drive both the
      textured base body and the colored full-mesh fallback; undead horse
      adult saddle layers are implemented from `EquipmentSlot.SADDLE`, using vanilla
      `SKELETON_HORSE_SADDLE` / `ZOMBIE_HORSE_SADDLE`, `EquineSaddleModel.createSaddleLayer()`,
      the family-specific `textures/entity/equipment/{skeleton_horse_saddle,zombie_horse_saddle}/saddle.png`,
      and ridden-only bridle line visibility from passenger state; their submissions are generated from
      `equipment_layer_pass` with vanilla `ModelLayers.SKELETON_HORSE_SADDLE` /
      `ZOMBIE_HORSE_SADDLE`. Baby undead horses skip
      the layer because vanilla supplies no baby saddle model. The zombie-horse body-armor layer
      is implemented from `EquipmentSlot.BODY`, using vanilla `HORSE_BODY`,
      `HorseModel(ModelLayers.UNDEAD_HORSE_ARMOR)` with the unscaled horse armor mesh and the
      same `horse_body` equipment textures; submissions are generated from `equipment_layer_pass`
      with vanilla `ModelLayers.UNDEAD_HORSE_ARMOR`; skeleton horses intentionally do not project horse
      armor because vanilla `EntityTypeTags.CAN_WEAR_HORSE_ARMOR` includes only horse and
      zombie_horse. Textured equine regressions now route through
      `entity_model_textured_meshes`, pinning horse/donkey/mule/skeleton-horse/
      zombie-horse base submissions, `HorseMarkingLayer` translucent overlays,
      equine saddle layers, and horse/zombie-horse body armor layers with vanilla
      render types (`entityCutout`, `entityTranslucent`, `armorCutoutNoCull`),
      selected textures, model-layer metadata, white or leather-dye tint, root transforms (horse
      `livingHorseScale`, donkey/mule scale, undead unscaled), and explicit
      `SubmitNodeCollector.order` / submit-sequence metadata; horse and undead-horse
      base submits now also pin vanilla entity light plus hurt/white overlay,
      `HorseMarkingLayer` keeps entity light and zeroes the white overlay column,
      and saddle/body-armor equipment submits keep entity light while forcing
      `OverlayTexture.NO_OVERLAY` before folded cutout/translucent geometry checks;
      missing-atlas coverage pins that a marked horse still records the
      `entityTranslucent` `horse_markings_whitedots.png` submission before only
      the folded markings geometry is suppressed, and that a saddled horse still records the
      `armorCutoutNoCull` `horse_saddle/saddle.png` submission before only the
      folded saddle geometry is suppressed. Missing-atlas coverage now also pins
      that an adult iron horse body-armor submission is still recorded before
      only the folded armor geometry is suppressed.
      Horse, donkey/mule, and undead-horse base submissions plus the horse
      markings overlay now come from explicit equine `EntityModelLayerPass`
      lists (`horse_textured_layer_passes`, `donkey_textured_layer_passes`,
      `undead_horse_textured_layer_passes`) with vanilla `ModelLayers.*`
      metadata, texture refs, render type names, white tint, and
      `(order, submit_sequence)` pinned before the custom equine hand-walk folds
      geometry. A self-visible invisible marked horse now uses the shared
      force-transparent base submission path (`entityTranslucentCullItemTarget`,
      `38/255` alpha) and skips the `HorseMarkingLayer`, matching vanilla's
      `!state.isInvisible` layer gate.
      The ridden/eat/stand/mouth poses, the tail's `ageInTicks` yRot wag, and broader lighting presentation remain
      unsupported
    - camel and camel_husk entities as renderer-owned vanilla 26.1 body-layer
      geometry from `AdultCamelModel`, `BabyCamelModel`, `CamelRenderer`, and
      `CamelHuskRenderer`, including `ModelLayers.CAMEL` / `CAMEL_BABY` (the camel
      husk reuses the adult `camel#main` mesh), normal camel adult/baby model
      selection, camel_husk adult-only renderer selection, zero-thickness tail
      cubes, official camel (128×128) / camel_baby (64×64) / camel_husk (128×128)
      texture references, shared-dispatch texture-backed base submission emission, the official
      PNG atlas upload/bind/sample path, and the `CamelModel.applyHeadRotation`
      clamped head yaw/pitch tracking applied to the body-nested head (`yRot` clamped
      to [-30, 30], `xRot` to [-25, 45] degrees) on both the colored and textured
      paths; the adult/husk `CamelModel.setupAnim` walk (`CamelAnimation.CAMEL_WALK`,
      the looping 1.5 s cycle sampled via `applyWalk(walkAnimationPos, walkAnimationSpeed,
      2.0, 2.5)`) is reproduced on BOTH the colored and textured paths — the `root`
      channel rolls the whole model, the four legs swing (rotation + position), the head
      pitch ADDS onto the clamped look, the two ears flap, and the tail swishes (a still
      camel samples amplitude 0, collapsing to the bind pose plus the head look); the baby
      camel walk (`CamelBabyAnimation.CAMEL_BABY_WALK`, which adds a `body` y-dip and a
      `head` position nudge and reorders the legs/ears) is reproduced too on both paths via
      a shared per-variant `CamelWalkLayout`. The `CamelModel.setupAnim` sit-down / seated-hold /
      stand-up keyframe animations (`CamelAnimation.CAMEL_SIT` 2.0 s, `CAMEL_SIT_POSE` 1.0 s,
      `CAMEL_STANDUP` 2.6 s — all non-looping) are now reproduced on BOTH the colored and textured
      paths, ADDED onto the walk pose; their timing is a PURELY projection-time computation from the
      synced `Camel.LAST_POSE_CHANGE_TICK` (data id 20, a `Long` whose sign encodes sitting) and the
      world game time (`getPoseTime = gameTime - |lastPoseChangeTick|`), mirroring
      `Camel.setupAnimationStates()` / `isCamelVisuallySitting` / `isVisuallySittingDown` /
      `isInPoseTransition` (the 40-tick sit-down and 52-tick stand-up windows) — no client-side
      accumulator is needed. The adult camel and camel_husk saddle equipment layers are implemented from
      `EquipmentSlot.SADDLE`: vanilla `CamelRenderer.createCamelSaddleLayer` / `CamelHuskRenderer`
      add `SimpleEquipmentLayer(CAMEL_SADDLE/CAMEL_HUSK_SADDLE)` with
      `CamelSaddleModel(ModelLayers.CAMEL*_SADDLE)`, `CamelSaddleModel.createSaddleLayer()` starts from
      the adult camel mesh and appends the saddle, bridle, and ridden-only reins, and bbb renders the
      family-specific `textures/entity/equipment/{camel_saddle,camel_husk_saddle}/saddle.png` (128×128).
      The textured camel regressions now pin vanilla submissions before folded
      geometry checks: base camel/camel_baby/camel_husk passes use `entityCutout`,
      selected texture, white tint, `entity_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)` with vanilla entity light plus
      hurt/white overlay; adult camel and camel_husk saddle passes use
      `equipment_layer_pass` with vanilla `ModelLayers.CAMEL_SADDLE` /
      `CAMEL_HUSK_SADDLE`, `armorCutoutNoCull`, the family-specific saddle texture, white tint, the
      same transform, `(0, 1)`, entity light, and `OverlayTexture.NO_OVERLAY`.
      Folded cutout vertices inherit the corresponding base or saddle submission
      metadata; missing-atlas coverage pins that the adult camel saddle submission is still
      recorded without `camel_saddle/saddle.png` while only folded saddle geometry is
      suppressed. Baby camels intentionally skip this layer because vanilla
      supplies no baby saddle model. The camel
      `CAMEL_IDLE` keyframe animation (driven by a client-side `random.nextInt(40) + 80` timer, not
      derivable from synced state), the body-anchor sit/stand y-offset
      (`Camel.getBodyAnchorAnimationYOffset`), and the `jumpCooldown`
      extra-pitch head boost (needs the un-projected jump-cooldown state) remain
      unsupported
    - llama and trader llama entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `LlamaModel`, `BabyLlamaModel`, and
      `LlamaRenderer`, including `ModelLayers.LLAMA` / `LLAMA_BABY` (the trader
      llama shares the same baked mesh under `ModelLayers.TRADER_LLAMA` /
      `TRADER_LLAMA_BABY`), official per-variant adult (128×64) / baby (64×64)
      texture references, shared-dispatch texture-backed base submission emission, official PNG
      atlas upload/bind/sample path, metadata-driven adult chest visibility, baby
      chest suppression, and the vanilla `LlamaModel.setupAnim` head-look yaw/pitch
      on the head part plus the standard `QuadrupedModel` diagonal leg swing
      (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at `[2, 3, 4, 5]` adult /
      `[4, 5, 6, 7]` with-chest / `[1, 2, 3, 4]` baby, colored and textured). The
      texture-backed tests now pin explicit base submission metadata for vanilla
      `entityCutout`, selected adult/baby variant texture, white tint, root
      transform, and `order(0)` before folded UV/decor/head-look/walk checks. The
      vanilla `LlamaDecorLayer` is implemented for `LLAMA_BODY`: adult carpet body
      equipment renders the matching `textures/entity/equipment/llama_body/<color>.png`
      overlay from `Equippable.llamaSwag(DyeColor)`, adult trader llamas fall back
      to `trader_llama.png` when no carpet item overrides it, and baby trader llamas
      use `trader_llama_baby.png` while ignoring body-item carpets. The decor overlay
      records vanilla `EquipmentLayerRenderer` submission metadata generated from
      `equipment_layer_pass` (`ModelLayers.LLAMA_DECOR` / `LLAMA_BABY_DECOR`,
      `armorCutoutNoCull`, `order(1)`, submit sequence 1, white tint, base transform, entity light, and
      `OverlayTexture.NO_OVERLAY`) before folding into the cutout bucket. The base
      `entityCutout` submission now also pins vanilla `LivingEntityRenderer`
      light plus hurt/white overlay metadata, and folded cutout vertices inherit
      the corresponding base or decor submission metadata. Missing-atlas coverage
      pins that adult carpet and baby trader decor submissions are still recorded
      before only the folded decor geometry is suppressed. The llama spit
      projectile is covered separately below with object-renderer no-overlay
      submission metadata
    - goat entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `GoatModel`, `BabyGoatModel`, and `GoatRenderer`,
      including `ModelLayers.GOAT` / `GOAT_BABY`, official adult/baby texture
      references, texture-backed base layer pass emission, official PNG atlas
      upload/bind/sample path, metadata-driven left/right horn visibility, and the
      vanilla `QuadrupedModel.setupAnim` head-look yaw/pitch on the head part plus the
      `QuadrupedModel` leg swing (`cos(pos * 0.6662 [+ π]) * 1.4 * speed`, legs at
      `[2, 3, 4, 5]` adult / `[0, 1, 2, 3]` baby, colored and textured, with the horn
      children rotating with the head) plus the ramming/lowering-head event animation:
      `Goat.handleEntityEvent` events `58`/`59` toggle `isLoweringHead`, the client `aiStep`
      advances `lowerHeadTick` (`++` while lowering, `-= 2` otherwise, clamped `[0, 20]`), and
      `Goat.getRammingXHeadRot()` (`lowerHeadTick/20 · (baby ? 52.5° : 30°) · π/180`, the baby
      scale resolved in the native layer) drives `GoatModel.setupAnim`'s `if rammingXHeadRot != 0
      { head.xRot = rammingXHeadRot }` head-down tilt, overwriting the head-look pitch during a
      ram. The textured goat regressions now pin the vanilla base submission metadata for both
      adult and baby paths: `entityCutout`, selected adult/baby texture, white tint,
      `entity_model_root_transform`, entity light, hurt/white overlay, and
      `(order, submit_sequence) == (0, 0)` before checking folded UVs, horn visibility,
      head look, and leg swing. Folded cutout vertices inherit the base submission's
      light/overlay metadata; only the screaming-goat sounds remain
      unsupported
    - polar bear entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `PolarBearModel`, `BabyPolarBearModel`, and
      `PolarBearRenderer`, including `ModelLayers.POLAR_BEAR` /
      `POLAR_BEAR_BABY`, adult `MeshTransformer.scaling(1.2F)`, and official
      adult/baby texture references, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and the vanilla
      `PolarBearModel.setupAnim` standing-rear head/body/front-leg pose driven by
      the canonical `clientSideStandAnimation` countdown projected through
      `PolarBear.getStandingAnimationScale` and the renderer partial tick, and the
      vanilla `QuadrupedModel.setupAnim` head-look yaw/pitch and diagonal leg walk
      swing applied before the standing-rear deltas (`head.xRot += standScale *
      π * 0.15`, front-leg rear offset layered on top of the walk) on adult/baby
      colored and textured models, matching vanilla's `super.setupAnim`-then-rear
      order. The textured polar bear regressions now pin the vanilla base
      submission metadata before folded geometry checks: adult `polarbear.png`
      uses `entityCutout`, white tint, `polar_bear_model_root_transform`
      (`MeshTransformer.scaling(1.2F)`), and `(order, submit_sequence) == (0, 0)`;
      baby `polarbear_baby.png` uses `entityCutout`, white tint,
      `entity_model_root_transform`, `(0, 0)`, vanilla entity light, and
      hurt/white overlay coords, with folded cutout vertices inheriting the
      submission metadata
    - hoglin and zoglin entities as renderer-owned vanilla 26.1 adult/baby
      body-layer geometry from `HoglinModel`, `BabyHoglinModel`,
      `AbstractHoglinRenderer`, `HoglinRenderer`, and `ZoglinRenderer`,
      including shared `ModelLayers.HOGLIN` / `ZOGLIN` and `HOGLIN_BABY` /
      `ZOGLIN_BABY` layers plus official adult/baby hoglin/zoglin texture
      references, shared-dispatch texture-backed base submission emission, official PNG
      atlas upload/bind/sample path, the vanilla `HoglinModel.setupAnim`
      yaw-only head look (`head.yRot = yRot * π/180`, keeping `head.xRot` at the
      fixed headbutt-rest tilt `HOGLIN_HEAD_X_ROT`) on the head part, the
      `1.2`-amplitude leg swing (legs at `[2, 3, 4, 5]`), and the ear sway for both adults
      and babies (`ear.zRot = ±2π/9 ± speed * sin(pos)`, ears at head children `[0, 1]`, the
      head subtree hand-emitted; the formula sets the absolute `±2π/9`, overriding the wider
      rest angle of `BabyHoglinModel`'s layer), and the event-driven headbutt head ram
      (`apply_hoglin_headbutt`, from entity event 4 — see the hoglin note above) — all
      colored and textured. The textured hoglin/zoglin regressions now pin the
      vanilla base submission metadata for adult and baby paths: `entityCutout`,
      selected hoglin/zoglin texture, white tint, `entity_model_root_transform`,
      `(order, submit_sequence) == (0, 0)`, vanilla entity light, and hurt/white
      overlay coords before folded UV, yaw-only head look, leg swing, and
      ear-sway checks. Folded cutout vertices inherit each corresponding
      submission's metadata; the hoglin converting shake remains unsupported
    - ravager entities as renderer-owned vanilla 26.1 `RavagerModel`
      body-layer geometry from `RavagerModel` and `RavagerRenderer`,
      including nested neck/head/horn/mouth parts, official
      `textures/entity/illager/ravager.png` texture reference, and
      `ModelLayers.RAVAGER`, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and explicit base submission metadata
      for vanilla `entityCutout`, `ravager.png`, white tint, `entity_model_root_transform`,
      packed light, overlay coords, and `(order, submit_sequence) == (0, 0)` before
      folded cutout checks. Folded cutout vertices inherit the base submission's
      light/overlay metadata, and the vanilla `RavagerModel.setupAnim` head look (`head.xRot/yRot = xRot/yRot * π/180`) on
      the neck-nested head part — the neck subtree is emitted by hand so the head
      carries the look while its horn/mouth children inherit it (colored and
      textured), and the vanilla `RavagerModel.setupAnim` leg walk swing
      (`ravager_leg_swing_pose`: the `QuadrupedModel` diagonal phase
      `cos(pos * 0.6662 [+ π])` at the shorter `0.4` amplitude, legs `[2, 3, 4, 5]`,
      `xRot` only so the neck/head subtree is untouched) plus the event-driven attack
      neck-lunge, stunned neck shake, and roar mouth-gape poses (`apply_ravager_combat`
      — see the ravager note above) on both render paths; only the roar
      particle/knockback effects and full vanilla LightTexture/gamma parity remain unsupported
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
      baby villager index-3 head) plus the half-amplitude leg walk swing
      (`cos(pos * 0.6662 [+ PI]) * 1.4 * speed * 0.5`) on adult/baby
      villager and wandering-trader colored and textured paths, and the unhappy
      head shake (`head.xRot = 0.4`, `head.zRot = 0.3 * sin(0.45 *
      ageInTicks)`) projected from `AbstractVillager.DATA_UNHAPPY_COUNTER` id
      `18` for villagers and wandering traders. Villager
      `VillagerProfessionLayer` type,
      profession, and level-badge overlays are implemented on the textured path:
      native reads `VillagerData` at entity-data id `19`, resolves
      `villager_type` / `villager_profession` through the dynamic registries
      with vanilla bootstrap-order fallback, emits baby type robes only for
      baby villagers, skips profession/level layers for babies, skips the level
      badge for `NONE`/`NITWIT`, clamps badge texture selection to levels
      `1..=5`, and applies the vanilla hat metadata/no-hat rule. The textured
      villager regressions now pin vanilla submission metadata before folded
      geometry checks: base robes use `entityCutout`, selected base texture,
      white tint, adult `villager_adult_model_root_transform` or baby
      `entity_model_root_transform`, and `(order, submit_sequence) == (0, 0)`;
      type, profession, and level overlays now consume explicit data-layer pass
      metadata, including vanilla `VILLAGER_NO_HAT` / `VILLAGER_BABY_NO_HAT`
      model-layer identity for hidden type hats, `entityCutout`, white tint, the
      same transform, and vanilla `VillagerProfessionLayer` orders `(1, 1)`,
      `(2, 2)`, and `(3, 3)`, preserving entity light while clearing the white
      overlay column via `getOverlayCoords(state, 0.0F)`. Missing-atlas coverage
      now proves adult type/profession/level overlays and baby type overlays
      still record their vanilla `entityCutout` submissions while only folded
      overlay geometry is suppressed. Base villager and wandering trader
      submissions preserve entity light plus full hurt/white overlay. Wandering
      trader textured regressions pin its
      single `wandering_trader.png` base submission with
      `villager_adult_model_root_transform` and `(0, 0)`. Crossed-arms
      item layer is implemented for adult/baby villagers and wandering traders
      through the shared item-model pass; generic non-skull custom-head items
      are implemented through `CustomHeadLayer`'s `HEAD` item display transform,
      and static skeleton/wither-skeleton/zombie/creeper skulls plus
      profileless default-player heads, profiled default-skin player heads,
      dynamic profiled-player heads, dragon heads, and piglin heads render
      through the skull branch via `custom_head_skull_layer_pass`; wandering trader baby presentation remains unsupported
    - worn humanoid armor as a renderer-owned vanilla 26.1 `HumanoidArmorLayer` overlay (framework
      slice 1, renderer-side): the inflated `HumanoidArmorModel`
      (`HumanoidModel.createBaseArmorMesh` / `createArmorMeshSet`) is built per equipment slot as a
      four-piece subset — helmet (head + hat, `OUTER_ARMOR_DEFORMATION 1.0`, hat `g.extend(0.5)`),
      chestplate (body + arms, `1.0`), leggings (body + legs, `INNER_ARMOR_DEFORMATION 0.5`, legs
      `g.extend(-0.1)`), boots (legs, `1.0`, legs `g.extend(-0.1)`) — exactly the vanilla
      `retainExactParts` part sets and `CubeDeformation`s. Each piece is draped on the host humanoid's
      already-posed limbs via `ModelPart::copy_child_poses_from` (vanilla `HumanoidModel.copyPropertiesTo`),
      so the armor inherits the host `setup_anim` without re-running it, and is drawn into the cutout pass
      in the vanilla submit order (chest, legs, feet, head) as `armorCutoutNoCull`
      submissions generated from `humanoid_armor_layer_pass` at `order(1)` with the selected
      vanilla armor model set (`<host>#helmet/chestplate/leggings/boots`), entity light plus
      `OverlayTexture.NO_OVERLAY`;
      folded armor vertex segments inherit the matching submission light/overlay, and missing-atlas
      coverage now proves both adult full-armor and baby `HUMANOID_BABY` submissions survive
      when only the matching base zombie body texture is stitched. All eight equipment-asset materials
      (`ArmorMaterials.<MAT>` → `EquipmentAssets.<MAT>`: leather, copper, chainmail, iron, gold, diamond,
      turtle_scute, netherite) resolve to their `textures/entity/equipment/humanoid/<asset>.png`
      (head/chest/feet) and `humanoid_leggings/<asset>.png` (legs) textures, stitched into the entity
      atlas. The humanoid armor wearers are covered (`emit_worn_humanoid_armor` dispatch): the zombie
      family (zombie, husk with `HUSK_SCALE`, drowned, zombie villager), the skeleton family (skeleton,
      stray, parched, wither skeleton, bogged), the player, and the piglin family (piglin, piglin brute,
      zombified piglin) —
      each rebuilds its posed host model and drapes the armor. The piglin family uses the same base armor
      mesh grown by the piglin `1.02` outer deformation (`build_tree(outer)`, vanilla
      `AbstractPiglinModel.createArmorMeshSet` = `PlayerModel.createArmorMeshSet(..).map(removeEars)`, the
      removed ears / empty player sleeves carrying no geometry) rather than the standard `1.0`. Standard
      zombie / husk / drowned baby armor is also covered: the renderer builds vanilla
      `HumanoidModel.createBabyArmorMeshSet` with `BABY_OUTER_ARMOR_DEFORMATION = (-0.1, 0.5, 0.3)`,
      `BABY_INNER_ARMOR_DEFORMATION = (-0.1, 0.3, 0.3)`, the distinct waist / nested-feet retained trees,
      and `EquipmentClientInfo.LayerType.HUMANOID_BABY` 64x64 equipment textures for every slot. Baby piglin
      and baby zombified-piglin armor are covered through
      `AbstractPiglinModel.createBabyArmorMeshSet`: uniform `CubeDeformation(0.7)`, the vanilla
      `BABY_PIGLIN_ARMOR_ARM_OFFSET = (0.5, -0.5, 0)`, and the same `HUMANOID_BABY` equipment texture layer.
      Baby zombie-villager armor is covered through the dedicated vanilla
      `ModelLayers.ZOMBIE_VILLAGER_BABY_ARMOR` set, which `LayerDefinitions` builds from inherited
      `ZombieVillagerModel.createBabyArmorMeshSet(..., PartPose.ZERO)`: the same standard baby humanoid
      armor topology and `HUMANOID_BABY` equipment texture layer, posed from `BabyZombieVillagerModel`.
      The cross-crate
      equipment projection is now wired end-to-end (framework slice 2): `bbb_pack`'s item registry parses
      each `.humanoidArmor(ArmorMaterials.<MAT>, ...)` item to its equipment-asset name
      (`humanoid_armor_asset`), the native layer installs an item id → material table
      (`set_item_armor_materials`), and `WorldStore::entity_model_sources_at_partial_tick` resolves the
      worn item in each armor slot of the entity's `SetEquipment` to a material, projecting
      `head/chest/legs/feet_armor` onto the render source which the native scene maps to the renderer's
      `EntityArmorMaterial`. So iron-clad zombies, skeletons, and players now render their armor live.
      Leather armor tints by the worn item's `DyedItemColor` when custom-dyed and otherwise by its vanilla
      default undyed color (`DyedItemColor.LEATHER_COLOR` 0xA06540): the per-slot `dyed_color` component is
      projected end-to-end (`WorldStore` reads `EntityEquipment`'s item `component_patch.dyed_color` into
      `head/chest/legs/feet_armor_dye`, native carries it onto the renderer's `*_armor_dye`), and
      `armor_layer_tint` forces it opaque and applies it only to leather — exactly vanilla
      `DyedItemColor.getOrDefault` → `EquipmentLayerRenderer.getColorForLayer` (every other material renders
      white, vanilla color `-1`, ignoring any stray dye). STILL DEFERRED: the enchant-glint and armor-trim
      passes and any remaining mob-specific armor models
    - base zombie entities as renderer-owned vanilla 26.1 adult/baby body-layer
      geometry from `HumanoidModel`, `BabyZombieModel`, and `ZombieRenderer`,
      with a texture-backed cutout render path: the adult layer emits the vanilla
      `HumanoidModel.createMesh` UVs over `textures/entity/zombie/zombie.png` (the
      `texOffs(32, 0)` hat keeps its base 8x8x8 box as the UV source, and the left
      arm/leg mirror the right's `texOffs`), the baby layer emits the
      `BabyZombieModel.createBodyLayer` UVs over
      `textures/entity/zombie/zombie_baby.png` (each limb has its own `texOffs`,
      no mirroring), with official PNG atlas upload/bind/sample and the head-look /
      leg-swing animation plus the held-out `animateZombieArms` resting arm pose
      (`armDrop = -π / (isAggressive ? 1.5 : 2.25)` from the synced `Mob.isAggressive`
      flag, `yRot ∓0.1`, `zRot 0`, then the idle bob) plus the `animateZombieArms` melee
      swing over the projected `attack_anim` (`attackYRot = sin(t·π)` toward center,
      `xRot += attackYRot·1.2 - sin((1-(1-t)²)·π)·0.4`) on both render paths (only the
      inherited `setupAttackAnimation` body twist / arm-anchor reposition stays deferred); husk entities share that texture-backed render path through
      `HuskRenderer extends ZombieRenderer`: they reuse the zombie adult/baby body
      parts (so the husk geometry is byte-for-byte the zombie geometry) over
      `textures/entity/zombie/husk.png` / `textures/entity/zombie/husk_baby.png`,
      with the adult mesh scaled by the vanilla 26.1 `LayerDefinitions`
      `MeshTransformer.scaling(1.0625F)` (`huskScale`) at the model root, the baby
      reusing the unscaled shared baby zombie body layer, and the same official PNG
      atlas upload/bind/sample plus head-look / leg-swing animation and the held-out
      arms on both render paths (as for the base zombie); drowned entities share that
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
      paths plus the held-out `animateZombieArms` resting arms and the `THROW_TRIDENT`
      raised-arm pose (`DrownedRenderer.getArmPose`: `getMainArm() == arm && isAggressive()
      && item.is(Items.TRIDENT)` → `DrownedModel.setupAnim` raises the main right arm
      `xRot = xRot*0.5 - π`, `yRot = 0` after the held-out arms; projected as
      `drowned_throw_trident` from the synced aggressive flag + the resolved main-hand
      trident) plus the always-on `DrownedOuterLayer`: a second white cutout copy of
      `DrownedModel.createBodyLayer(CubeDeformation(0.25F))` over
      `textures/entity/zombie/drowned_outer_layer.png` (the head/hat/body/right-limb cubes are the
      shared inflated `HumanoidModel.createMesh(0.25)` geometry, the left arm/leg take the drowned's
      own non-mirrored `texOffs(32, 48)` / `texOffs(16, 48)`), driven by a `DrownedOuterModel` posed
      by the SAME `ZombieModel.setupAnim` + trident-throw animator as the base so the inflated shell
      tracks the limbs (vanilla `coloredCutoutModelCopyLayerRender(..., -1, 1)`, white full-alpha),
      and the baby renders its own distinct outer shell from
      `BabyDrownedModel.createBodyLayer(CubeDeformation(0.25F))` (= `BabyZombieModel`, the baby-zombie
      inflated mesh — NOT the drowned left-limb overrides) over
      `textures/entity/zombie/drowned_outer_layer_baby.png`; the drowned `swimAmount` path is also
      implemented end-to-end: world tracks `LivingEntity.swimAmountO/swimAmount` from synced
      `Pose.SWIMMING` with the vanilla `±0.09` tick step, native forwards it, `DrownedRenderer`
      `setupRotations` pitches the whole body around `boundingBoxHeight / 2 / entityScale`, and both the
      base and outer `DrownedModel.setupAnim` copies fold the arms/legs with the vanilla swim sine);
      zombie
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
      the head-look / leg-swing animation plus the held-out `animateZombieArms` resting
      arms on both render paths; piglins, piglin brutes, and zombified
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
      with the base submission now generated by `dispatch_uniform_entity_model` instead of residual
      colored/textured emit helpers, official PNG atlas upload/bind/sample, and explicit base submission metadata for vanilla
      `entityCutout`, selected texture, white tint, `entity_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)`, plus `LivingEntityRenderer` entity light and
      hurt/white overlay metadata before folded geometry checks, plus the vanilla
      `AbstractPiglinModel.setupAnim` head-look, leg swing, ear flap, and (for the
      non-zombified families) arm counter-swing on both render paths, while the zombified
      piglin uses the held-out `animateZombieArms` arms;
      the `DrownedOuterLayer` (adult and baby), drowned swim re-pose, and
      non-player/baby WingsLayer/elytra presentation ARE implemented (see the
      drowned and WINGS notes above); zombie/piglin converting shake, remaining
      zombie-family and piglin-family armor nuances, and held-item refinements
      remain unsupported
      (generic non-skull head-slot items and static skeleton/wither-skeleton/
      zombie/creeper skulls plus profileless default-player heads, profiled
      default-skin player heads, dynamic profiled-player heads, dragon heads,
      and piglin heads are covered by
      the shared custom-head paths); zombie
      villager type/profession/level overlays ARE implemented via
      `VillagerProfessionLayer` parity, reading `VillagerData` at entity-data id
      `20`, using the zombie-villager overlay textures, baby type robes, level
      badges for non-`NONE`/`NITWIT` adult professions, and vanilla no-hat model
      selection through explicit data-layer passes that record
      `ZOMBIE_VILLAGER_NO_HAT` / `ZOMBIE_VILLAGER_BABY_NO_HAT` when the type hat
      is hidden; shared dispatch now owns the husk/drowned/zombie-villager base
      submissions and the drowned textured-only outer layer instead of residual
      textured emit helpers; textured zombie-family regressions now route through
      `entity_model_textured_meshes`, pinning zombie/husk/drowned/zombie-villager
      base submissions plus the drowned outer layer and zombie-villager data
      overlays as vanilla `entityCutout` submits with selected textures, white
      tint, root transforms (`entity_model_root_transform`, adult husk
      `huskScale`, drowned swim pivot), and explicit `(order, submit_sequence)`
      metadata, plus `LivingEntityRenderer` entity light and the base full
      hurt/white overlay versus `DrownedOuterLayer` / `VillagerProfessionLayer`
      zero-white overlay split before folded cutout geometry, tint, transform, and
      animation checks; missing-atlas coverage now proves adult and baby
      `DrownedOuterLayer` submissions are still recorded without outer-layer
      textures while only folded outer-shell geometry is suppressed, and proves
      zombie-villager adult type/profession/level overlays and baby type
      overlays still record their vanilla `entityCutout` submissions while only
      folded overlay geometry is suppressed; the piglin
      dance/attack/crossbow-hold/crossbow-charge/admiring arm poses ARE all implemented
      (see the piglin note);
      the zombie-arm attack swing IS implemented (the
      held-out arms, the `Mob.isAggressive` arm-raise, and the
      `animateZombieArms` melee swing over the projected `attack_anim` — only the
      inherited `setupAttackAnimation` body twist / arm-anchor reposition and the
      STAB swing-type skip stay deferred for the zombie family); the drowned
      `THROW_TRIDENT` raised-arm pose and swimAmount re-pose ARE implemented (see the drowned note above);
      the zombie, husk,
      drowned, zombie-villager, piglin, piglin-brute, and zombified-piglin head
      parts now apply the vanilla `HumanoidModel.setupAnim` head-look yaw/pitch
      (the baby layout's index-1 head, and the baby piglin brute's adult-layout
      head, included), and the zombie and piglin families also apply the inherited
      `HumanoidModel` leg swing on their two leg parts. The zombie family (zombie, husk,
      drowned, zombie villager, and the 6× giant) applies the held-out
      `animateZombieArms` resting pose on its two arm parts; the non-zombified piglin
      family (adult/baby piglin and brute) instead applies the inherited arm
      counter-swing (the zombified piglin's arms use the implemented
      `animateZombieArms` held-out pose; the `AbstractPiglinModel` ear sway and the
      `PiglinModel` `DANCING` / `ATTACKING_WITH_MELEE_WEAPON` / `CROSSBOW_HOLD` /
      `CROSSBOW_CHARGE` / `ADMIRING_ITEM` arm poses are all implemented)
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
      `SkeletonModel` runs the same `setupAnim`), and the `BOW_AND_ARROW`
      aim `ArmPose` — vanilla `AbstractSkeletonRenderer.getArmPose` raising both
      arms forward along the head look when the skeleton `isAggressive() &&
      getMainHandItem().is(Items.BOW)` (the off arm splayed `0.4` outward),
      driven by the projected `is_aggressive` + `main_hand_holds_bow` render
      state and applied to the base body and the Stray/Bogged clothing overlay
      alike (the held bow mesh tracks the aimed hand through the shared posed
      model), and the melee swing `ArmPose` — vanilla `SkeletonModel.setupAnim`'s
      `isAggressive && !isHoldingBow` raising both arms to `-π/2` and chopping with
      the projected `attack_anim` (the right arm yawing in, the left out), over the
      shared `setupAttackAnimation` body twist, on the base body and the clothing
      overlay alike. Shared dispatch now owns the skeleton-family base submissions
      and the Stray/Bogged textured-only clothing overlay instead of a residual textured
      emit helper. Textured skeleton-family regressions now route through
      `entity_model_textured_meshes`, pinning base and Stray/Bogged clothing submissions'
      selected textures, `entityCutout` render type, white tint, root transform
      (`wither_skeleton_model_root_transform` for wither skeleton, generic
      `entity_model_root_transform` otherwise), and explicit `(order, submit_sequence)`
      pairs `(0, 0)` / `(1, 1)`, plus vanilla entity light and the body full
      hurt/white overlay versus `SkeletonClothingLayer` zero-white overlay split before
      folded cutout geometry checks. Missing-atlas coverage now proves stray and
      bogged clothing overlay submissions are still recorded without their
      overlay textures while only folded overlay geometry is suppressed.
      Skeleton-family
      armor is covered by `emit_worn_humanoid_armor`
      for skeleton, stray, parched, wither skeleton, and bogged
    - creeper entities as renderer-owned vanilla 26.1
      `CreeperModel.createBodyLayer(CubeDeformation.NONE)` geometry, with the
      official `textures/entity/creeper/creeper.png` texture reference,
      `ModelLayers.CREEPER` selection, texture-backed base layer pass emission,
      official PNG atlas upload/bind/sample path, and the vanilla
      `CreeperModel.setupAnim` head-look yaw/pitch on the head part, the
      `QuadrupedModel`-formula four-leg walk swing, and the `CreeperRenderer.scale`
      swell (the inflate-and-flicker `this.scale()` non-uniform scale before
      exploding, driven by the projected `creeper_swelling`; identity for a calm
      creeper) (colored and textured). The texture-backed tests now pin explicit
      base submission metadata for vanilla `entityCutout`, `creeper.png`, white
      tint, the creeper swell root transform, and `order(0)` before checking
      folded UV/head-look/walk/swell geometry, with cutout vertices inheriting
      the base light/overlay. Charged creepers also emit the
      vanilla `CreeperPowerLayer` / `EnergySwirlLayer` submission at `order(1)`
      with `creeper_armor.png`, `energySwirl`, the vanilla half-grey tint, and
      the same creeper root transform, preserving the per-entity light and
      vanilla `OverlayTexture.NO_OVERLAY` before folding the inflated armor
      model into the additive scroll mesh with matching vertex metadata; full
      scroll-mesh lighting presentation parity remains deferred
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
      eyes pass using the parent spider model parts, explicit base/eyes submission
      metadata for `entityCutout` then `eyes`, white tint, spider/cave-spider root
      transforms, `spider.png` or `cave_spider.png`, `spider_eyes.png`, and
      `(order, submit_sequence) == (0, 0)` then `(1, 1)`, plus a
      `RenderTypes.eyes`-style translucent/depth-write-disabled GPU path. Tests
      now also pin base entity light plus hurt/white overlay and `SpiderEyesLayer`
      entity light plus `OverlayTexture.NO_OVERLAY` on both submissions and
      folded buckets; missing-atlas coverage proves both spider renderers still
      record the eyes submission without `spider_eyes.png` while only folded
      emissive geometry is suppressed. The
      vanilla `SpiderModel.setupAnim` head-look yaw/pitch on the head part and the
      vanilla `SpiderModel.setupAnim` eight-leg walk swing (`spider_leg_swing_pose`:
      each leg sweeps `yRot += -(cos(animationPos*2 + phase) * 0.4) * speed` and steps
      `zRot += |sin(animationPos + phase) * 0.4| * speed`, right legs `+`/left legs `-`,
      per-pair phases `0`/`π`/`π/2`/`3π/2`, legs at `[3..=10]`) on both render paths and
      passes (colored and textured, both spider and cave spider); death
      flip and broader lighting presentation remain unsupported
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
      clamped to `[-0.4, 0.4]` (arms `[2, 3]`, legs `[4, 5]`) — plus the
      carried-block arm pose (`enderman_carried_arm_pose`, both arms out front when
      the projected `enderman_carrying` is set) and the creepy head/hat shift
      (head `y -= 5` / hat `y += 5` when `enderman_creepy` is set) (colored and
      textured). The textured enderman regressions now pin both vanilla
      submissions before folded geometry checks: base `entityCutout`
      `enderman.png` at `(order, submit_sequence) == (0, 0)` and eyes
      `RenderTypes.eyes` / `enderman_eyes.png` at `(1, 1)`, both with white
      tint, `entity_model_root_transform`, and matching entity light, while the
      eyes submit now preserves vanilla `OverlayTexture.NO_OVERLAY` even when the
      base body carries hurt/white overlay; folded cutout/eyes vertices inherit
      their respective submission light/overlay metadata. Missing-atlas coverage
      now pins that `EnderEyesLayer` still records the order-1 `Eyes` /
      `enderman_eyes.png` submit when only the base enderman texture is atlas-backed,
      suppressing only folded eyes geometry. The held block's own
      block-model render is implemented through `CarriedBlockLayer`'s vanilla root
      transform, while the creepy render jitter and lighting remain unsupported
    - iron golem entities as renderer-owned vanilla 26.1
      `IronGolemModel.createBodyLayer()` geometry, including its 128x128 body
      layer, baked `CubeDeformation(0.5F)` lower-body cube, and the official
      `textures/entity/iron_golem/iron_golem.png` texture reference from
      `IronGolemRenderer`, texture-backed base layer pass emission, and
      official PNG atlas upload/bind/sample path, and the vanilla
      `IronGolemModel.setupAnim` head-look yaw/pitch on the head part, the
      triangle-wave leg/arm walk swing, and the event-driven attack smash /
      offer-flower arm poses (colored and textured — see the iron golem note
      above). The `IronGolemCrackinessLayer` damage-crack overlay is now
      implemented: `EntityModelKind::IronGolem` carries a `crackiness` projected
      from the synced `LivingEntity.DATA_HEALTH_ID` (index 9) via
      `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(health / 100.0)`
      (`<0.25` high, `<0.5` medium, `<0.75` low, else none), appending a white
      Cutout overlay pass binding `iron_golem_crackiness_{low,medium,high}.png`
      (the three faces join the master atlas array → 364) over the same model
      layer. The held flower block layer is now implemented through the entity-attached
      block-model path while `offerFlowerTick > 0`, using `Blocks.POPPY.defaultBlockState()`
      and the vanilla `IronGolemFlowerLayer` right-arm transform. The renderer
      body-wobble rotation is now applied at the vanilla setup-rotation point
      (`6.5 * triangleWave(walkAnimationPos + 6, 13)` degrees around Z when
      `walkAnimationSpeed >= 0.01`), and the base/crack submissions plus the
      held poppy block transform share that root. The textured golem regressions
      now pin the vanilla submission metadata too: base and crack overlays are
      `entityCutout`, bind the selected vanilla texture, use white tint, carry
      `iron_golem_model_root_transform`, keep the entity light, preserve full
      hurt/white overlay on the base body, clear only the crack layer's white
      overlay column via `getOverlayCoords(state, 0.0F)` while preserving the
      red row, and preserve `(order, submit_sequence)` as `(0, 0)` for base plus
      `(1, 1)` for the crack layer. Folded cutout vertices inherit the matching
      base/crack submission light and overlay metadata
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
      facing forward), on both render paths (colored and textured). The
      textured snow-golem regressions now pin the base submission as
      `entityCutout`, the selected vanilla snow-golem texture, white tint,
      `entity_model_root_transform`, and `(order, submit_sequence) == (0, 0)`. The
      `SnowGolemHeadLayer` carved-pumpkin block model is now implemented through
      the entity-attached block-model path: bbb-world projects `SnowGolem.DATA_PUMPKIN_ID`
      (index 16 BYTE, bit 16, vanilla default on), native resolves
      `Blocks.CARVED_PUMPKIN.defaultBlockState()` (`facing=north`) through the terrain
      block-model catalog, and the renderer-owned head attachment transform mirrors
      vanilla's head-bone transform plus `translate(0,-0.34375,0)`, `rotateY(180°)`,
      `scale(0.625,-0.625,-0.625)`, `translate(-0.5,-0.5,-0.5)`. The invisible-glowing
      `submitOnlyOutline` path now records outline-only attachment metadata and suppresses
      ordinary block-quad baking while GPU outline presentation remains deferred
    - copper golem entities as renderer-owned vanilla 26.1
      `CopperGolemModel.createBodyLayer()` geometry, including the mesh-root
      `(0,24,0)` transform, body/head/arm/leg cuboids, the deformed head and
      antenna cubes, the official 64x64
      `textures/entity/copper_golem/copper_golem*.png` weathering texture set,
      and the matching emissive eyes texture set. `EntityModelKind::CopperGolem`
      now carries `WeatheringCopper.WeatherState` projected from
      `CopperGolem.DATA_WEATHER_STATE` (data id 16,
      `WEATHERING_COPPER_STATE`) and selects unaffected / exposed / weathered /
      oxidized body and eyes passes like `CopperGolemRenderer`. Textured tests pin the
      `CopperGolemBase` `entityCutout` submission's entity light plus hurt/white overlay and the
      `CopperGolemEyes` `LivingEntityEmissiveLayer` submission's entity light plus
      `getOverlayCoords(state, 0.0F)` red-row/zero-white overlay, including folded cutout/eyes
      vertex metadata; missing-atlas coverage proves a weathered eyes submission is still recorded
      without `copper_golem_eyes_weathered.png` while only folded emissive geometry is suppressed.
      The head look and
      standard `ItemInHandLayer` are projected: non-empty main/off-hand equipment
      clamps both arms into `CopperGolemModel.poseHeldItemArmsIfStill`, the
      renderer exports the `translateToHand` IDLE hand branch (`body -> arm`,
      ±90° Y, `translate(0, 0, 0.125)`), and native bakes both hands with the
      retained third-person left/right item display transforms. The antenna
      block decoration is now implemented through the same entity-attached
      block-model path: protocol preserves the item stack's
      `DataComponents.BLOCK_STATE` string map, world exposes arbitrary
      equipment-slot item queries, native reads `CopperGolem.EQUIPMENT_SLOT_ANTENNA`
      (`EquipmentSlot.SADDLE`), resolves block-item resource ids plus component
      properties, and the renderer-owned transform mirrors
      `CopperGolemModel.applyBlockOnAntennaTransform` plus
      `BlockDecorationLayer`'s unit-cube antenna matrix. Generic non-skull
      custom-head items are implemented via the shared `CustomHeadLayer` item
      path, including the copper golem `translateToHead` override; static
      skeleton/wither-skeleton/zombie/creeper skulls also use that override in
      the skull branch. Profileless, profiled-default, and dynamic
      profiled-player heads use the same implemented skull path. The keyframe
      walk/walk-with-item/idle/interaction animations remain unsupported
    - witch entities as renderer-owned vanilla 26.1
      `WitchModel.createBodyLayer()` geometry, including the
      `VillagerModel.createBodyModel()` body/arms/legs/nose, the four nested
      hat cuboids, baked hat-tip and mole `CubeDeformation` bounds,
      `LayerDefinitions`' `MeshTransformer.scaling(0.9375F)`, and the official
      `textures/entity/witch/witch.png` texture reference from
      `WitchRenderer`, `ModelLayers.WITCH`, texture-backed base layer pass
      emission, explicit base submission metadata for vanilla `entityCutout`,
      `witch.png`, white tint, `villager_adult_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)`, plus `MobRenderer` /
      `LivingEntityRenderer` light plus hurt/white overlay metadata, official PNG atlas upload/bind/sample path,
      and the vanilla
      `WitchModel.setupAnim` head-look yaw/pitch on the head part, the
      half-amplitude leg walk swing (legs at `[3, 4]`), and the continuous
      `ageInTicks`-driven idle nose bob (`nose.xRot = sin(ageInTicks * speed) *
      4.5°`, `nose.zRot = cos(ageInTicks * speed) * 2.5°`, `speed = 0.01 *
      (entityId % 10)`), all on both render paths (colored and textured);
      `WitchRenderState.isHoldingItem` projection from a non-empty main hand, the
      `isHoldingItem` nose hold pose (`setPos(0, 1, -1.5)`, `xRot = -0.9`),
      `WitchRenderState.isHoldingPotion` resolution for `minecraft:potion`, and
      `WitchItemLayer` main-hand item rendering with `ItemDisplayContext.GROUND`
      on both vanilla branches: the nose-attached potion transform and the
      crossed-arms generic-item transform
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
      explicit base submission metadata for vanilla `entityCutout`, the selected
      renderer texture, white tint, `villager_adult_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)`, plus `MobRenderer` /
      `LivingEntityRenderer` light plus hurt/white overlay metadata, official PNG atlas upload/bind/sample,
      and the vanilla `IllagerModel.setupAnim`
      head-look yaw/pitch plus the half-amplitude leg swing (and the pillager's
      `HumanoidModel` arm swing on its separate arms) on both render paths. The
      standard `ItemInHandLayer` is covered by the shared humanoid held-item
      dispatch, which builds the posed `IllagerModel` and reads `right_arm` /
      `left_arm`; generic non-skull custom-head items are implemented by the
      shared `CustomHeadLayer` item path, and static skeleton/wither-skeleton/
      zombie/creeper skulls plus profileless default-player heads and profiled
      default-skin player heads and dynamic profiled-player heads are
      implemented by the skull branch; illusioner clone offsets/invisible-body
      rendering, and renderer state extraction for dynamic arm visibility
      remain unsupported. Spellcasting,
      crossbow hold/charge, illusioner bow aim, evoker/vindicator celebrating,
      vindicator empty/armed attacking, and riding sit arm/leg poses are implemented.
    - armor stand entities as renderer-owned vanilla 26.1
      `ArmorStandModel.createBodyLayer()` geometry, including the normal layer,
      `ModelLayers.ARMOR_STAND_SMALL` `HumanoidModel.BABY_TRANSFORMER` root-part
      transform, official `textures/entity/armorstand/armorstand.png` texture
      reference, client flags for small/show-arms/no-baseplate, and head/body/
      arm/leg pose metadata projection; the textured base layer emits the vanilla
      `createBodyLayer` `texOffs` UVs (the small layer reuses the full-model UVs
      because `BABY_TRANSFORMER` only scales geometry, not texture coordinates),
      with official PNG atlas upload/bind/sample on both render paths, explicit vanilla
      `ModelLayers.ARMOR_STAND` / `ARMOR_STAND_SMALL` base-pass metadata, and the
      standard held-item layer for both full and small stands (small hand items
      ride the `BABY_TRANSFORMER` 0.5 arm-part scale), plus the generic
      non-skull custom-head item path and static skeleton/wither-skeleton/
      zombie/creeper skulls plus profileless default-player heads, profiled
      default-skin player heads, dynamic profiled-player heads, piglin heads,
      and dragon heads; marker client flags now project into the render kind,
      marker stands keep vanilla 0x0 non-pickable dimensions while still
      producing model sources, and marker base submissions follow vanilla
      `ArmorStandRenderer.getRenderType`
      (`entityCutout` while visible, no submission when hidden, and
      `entityTranslucent` with the vanilla force-transparent alpha when invisible
      but visible to this client). The marker override also ignores
      `appearGlowing` for hidden-to-player marker stands, so they record no base
      outline submission, while non-marker invisible-glowing armor stands still
      use the inherited living `RenderTypes.outline(texture)` branch. Both paths
      preserve inherited `LivingEntityRenderer` light/overlay/outline-color
      metadata on their submissions and folded vertices where geometry is emitted.
      `HumanoidArmorLayer` now submits full and small armor-stand armor with the
      vanilla `ModelLayers.ARMOR_STAND_ARMOR` / `ARMOR_STAND_SMALL_ARMOR`
      helmet/chestplate/leggings/boots model-layer metadata, adult humanoid /
      leggings equipment textures, `armorCutoutNoCull`, no-overlay semantics,
      order-1 chest/legs/feet/head submit sequences, small-stand
      `HumanoidModel.BABY_TRANSFORMER` scaling, and the vanilla invisible-layer
      behavior where hidden glowing marker stands still keep armor submissions
      even though their base body records no submission. Armor-stand
      `WingsLayer` and the skull branch of `CustomHeadLayer` now also keep their
      texture-backed submissions through the marker hidden/glowing no-base path
      and inherited living invisible branches, preserving vanilla texture,
      render type, transform, light, no-overlay, outline-color, order, and
      submit-sequence metadata. The native item-model pass is now covered for
      marker hidden/glowing armor-stand held items and generic non-skull
      `CustomHeadLayer` HEAD items as well: `held_item_models` still bakes the
      main-hand and HEAD item meshes when the base body records no submission.
      Hurt wiggle and animation interpolation remain unsupported
    - slime entities as renderer-owned vanilla 26.1 `SlimeModel` inner
      `ModelLayers.SLIME` geometry plus outer `ModelLayers.SLIME_OUTER`
      geometry, official `textures/entity/slime/slime.png` texture reference,
      renderer size scaling from slime size metadata, the client-reconstructed
      `Slime.tick` squish accumulator (`squish`/`oSquish`/`targetSquish`/
      `wasOnGround` driven by the `onGround()` jump transitions, lerped per the
      partial tick) projected into the `SlimeRenderer.scale` non-uniform body
      stretch (`ss = squish / (size * 0.5 + 1)`, `w = 1/(ss + 1)`, scale
      `[w, 1/w, w] * size`), texture-backed base and outer layer pass emission,
      the `SlimeOuterLayer` submit order `1`, explicit submission metadata for
      the base `entityCutout` and outer `entityTranslucent` passes (texture,
      white tint, slime root transform, `LivingEntityRenderer` light plus body hurt/white overlay for
      the base, `getOverlayCoords(state, 0.0F)` zero-white overlay for the outer layer, and
      `(order, submit_sequence)`) with folded cutout/translucent vertices inheriting the matching
      metadata, shared dispatch ownership instead of a residual textured emit helper, and an
      alpha-blended translucent GPU bucket. Invisible glowing slime now records the
      vanilla base and order-1 `SlimeOuterLayer` outline submissions with
      `outlineColor` metadata while folded/GPU outline presentation remains
      deferred; particle/audio coupling, broader lighting
      presentation, crumbling, and full render-graph sorting parity remain unsupported
    - magma cube entities as renderer-owned vanilla 26.1
      `MagmaCubeModel.createBodyLayer()` segment/inside-cube geometry, official
      `textures/entity/slime/magmacube.png` texture reference, renderer
      size scaling from inherited slime size metadata, the shared client-side
      squish accumulator driving both the `MagmaCubeRenderer.scale` non-uniform
      body stretch and the `LavaSlimeModel.setupAnim` per-segment vertical spread
      (`cubeN.y = -(4 - N) * max(0, squish) * 1.7`), texture-backed base layer
      pass emission with explicit submission metadata for vanilla `entityCutout`,
      selected texture, white tint, magma-cube root transform, `LivingEntityRenderer` light plus
      hurt/white overlay, and `(0, 0)`, with folded cutout vertices inheriting the matching metadata,
      and official PNG atlas upload/bind/sample path; the full-bright block light
      (`MagmaCubeRenderer.getBlockLightLevel = 15`) IS now applied (`entity_light_coords`).
      Particle/audio coupling, broader lighting presentation, crumbling, and
      full render-graph sorting parity remain unsupported
    - ghast entities as renderer-owned vanilla 26.1 `GhastModel.createBodyLayer()`
      geometry: the 16x16x16 body at y 17.6 plus the nine tentacles at y 24.6,
      whose lengths are the fixed-seed `RandomSource(1660L)` (`nextInt(7) + 8`,
      reproduced via the Java legacy LCG → `[8, 13, 9, 11, 11, 10, 12, 9, 12]`) and
      whose xz offsets come from the vanilla index formula, scaled 4.5x by the
      `MeshTransformer.scaling(4.5F)` model-root transform; the official
      `textures/entity/ghast/ghast.png` texture reference, texture-backed base
      layer pass emission (vanilla `GhastModel` calls `EntityModel`'s default
      `RenderTypes::entityCutout`) while preserving explicit submission metadata for texture, white
      tint, scaled root transform, `order(0)`, and `MobRenderer` / `LivingEntityRenderer`
      light plus hurt/white overlay metadata, official PNG atlas upload/bind/sample path, and the
      vanilla `GhastModel.setupAnim` tentacle wave (`tentacle.xRot = 0.2 *
      sin(ageInTicks * 0.3 + i) + 0.4`, driven by the projected `ageInTicks`, on
      both render paths), and the vanilla `GhastRenderer.getTextureLocation`
      `isCharging` texture swap (`ghast.png` → `ghast_shooting.png`), driven by
      the projected `Ghast.DATA_IS_CHARGING` synced boolean (index 16, since
      `Ghast extends Mob` directly). Broader lighting presentation remains unsupported
    - happy ghast entities as renderer-owned vanilla 26.1
      `HappyGhastModel.createBodyLayer(false, NONE)` geometry: the 16x16x16 body at
      y 16 plus the nine tentacles parented under the body (world-space y 23) with
      hard-coded lengths `[5, 7, 4, 5, 5, 7, 8, 8, 5]`, scaled 4.0x by the
      `MeshTransformer.scaling(4.0F)` model-root transform; the official
      `textures/entity/ghast/happy_ghast.png` texture reference, texture-backed base
      layer pass emission (vanilla `HappyGhastModel` calls `EntityModel`'s default
      `RenderTypes::entityCutout`) while preserving explicit submission metadata for
      texture, render type, white tint, scaled root transform, `order(0)`, packed
      light, and overlay, with the folded cutout vertices inheriting the same
      submission light/overlay, official PNG atlas upload/bind/sample path, and the
      vanilla `HappyGhastModel.setupAnim` tentacle
      wave (it reuses `GhastModel.animateTentacles` verbatim, `tentacle.xRot = 0.2 *
      sin(ageInTicks * 0.3 + i) + 0.4`, driven by the projected `ageInTicks`, on both
      render paths).
      The baby model (the extra `inner_body` cube plus the 0.2375 baby scale), the
      `bodyItem` body squeeze (`0.9375` scale when a harness is equipped) with the
      harness equipment layer and the rope/lead layer remain unsupported
    - blaze entities as renderer-owned vanilla 26.1 `BlazeModel.createBodyLayer()`
      geometry: the 8x8x8 head at `PartPose.ZERO` plus twelve `2x8x2` rods (the
      shared `texOffs(0, 16)` `rod` builder), with no `MeshTransformer` scaling (the
      unit entity model-root transform); the official
      `textures/entity/blaze/blaze.png` texture reference, texture-backed base layer
      pass emission (vanilla `BlazeModel` calls `EntityModel`'s default
      `RenderTypes::entityCutout`) while preserving explicit submission metadata for
      texture, white tint, unit root transform, `order(0)`, full-bright projected
      block-light input, and hurt/white overlay metadata, official PNG atlas
      upload/bind/sample path, the vanilla `BlazeModel.setupAnim` rod orbit (twelve
      rods in three rings of radius 9/7/5, their x/y/z offsets set every frame from
      the projected `ageInTicks`), and the shared head look (`head.yRot/xRot` from the
      net look angles), on both render paths. The `BlazeRenderer` full-bright block
      light (`getBlockLightLevel = 15`) IS now applied (in `entity_light_coords`,
      forcing the packed block light to 15 for the blaze type); broader lighting
      presentation remains unsupported
    - endermite entities as renderer-owned vanilla 26.1
      `EndermiteModel.createBodyLayer()` geometry: the four nested chitin segments
      from `BODY_SIZES`/`BODY_TEXS` (each `addBox(-sx/2, 0, -sz/2, sx, sy, sz)` posed
      at `(0, 24 - sy, placement)`), with no `MeshTransformer` scaling (the unit
      entity model-root transform); the official
      `textures/entity/endermite/endermite.png` texture reference, texture-backed
      base layer pass emission (vanilla `EndermiteModel` calls `EntityModel`'s
      default `RenderTypes::entityCutout`) while preserving explicit submission
      metadata for texture, white tint, unit root transform, `order(0)`, and the
      `MobRenderer` / `LivingEntityRenderer` light plus hurt/white overlay metadata, official
      PNG atlas upload/bind/sample path, and the vanilla `EndermiteModel.setupAnim`
      segment wiggle (`segment.yRot = cos(phase) * π * 0.01 * (1 + |i - 2|)`,
      `segment.x = sin(phase) * π * 0.1 * |i - 2|`, `phase = ageInTicks * 0.9 + i *
      0.15 * π`, driven by the projected `ageInTicks`, on both render paths).
      Broader lighting presentation remains unsupported
    - silverfish entities as renderer-owned vanilla 26.1
      `SilverfishModel.createBodyLayer()` geometry: the seven nested body segments
      from `BODY_SIZES`/`BODY_TEXS` plus the three wider overlay layers riding
      segments 2/4/1 (`texOffs(20, 0/11/18)`, including the vanilla quirk where
      layer2 takes its z-min from `BODY_SIZES[4]` but its z-size from
      `BODY_SIZES[1]`), with no `MeshTransformer` scaling (the unit entity model-root
      transform); the official `textures/entity/silverfish/silverfish.png` texture
      reference, texture-backed base layer pass emission (vanilla `SilverfishModel`
      calls `EntityModel`'s default `RenderTypes::entityCutout`) while preserving
      explicit submission metadata for texture, white tint, unit root transform, and
      `order(0)`, plus `MobRenderer` / `LivingEntityRenderer` light plus hurt/white
      overlay metadata, official PNG atlas upload/bind/sample path, and the vanilla
      `SilverfishModel.setupAnim` segment wiggle (`segment.yRot = cos(phase) * π *
      0.05 * (1 + |i - 2|)`, `segment.x = sin(phase) * π * 0.2 * |i - 2|`, `phase =
      ageInTicks * 0.9 + i * 0.15 * π`, with the overlay layers copying segments
      2/4/1, driven by the projected `ageInTicks`, on both render paths). Broader
      lighting presentation remains unsupported
    - vex entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `138` to the real `VexModel`, replacing
      the former placeholder box. Renderer-owned vanilla 26.1 `VexModel.createBodyLayer()`
      geometry: the `(0, -2.5, 0)` model root carrying the 5³ head and the body (a plain
      `texOffs(0, 10)` 3×4×2 box plus a `texOffs(0, 16)` 3×5×2 box inset by
      `CubeDeformation(-0.2)`), with the two arms (`texOffs(23, 0)`/`texOffs(23, 6)`, 2×4×2
      inset by `CubeDeformation(-0.1)`) and the two zero-thickness `0×5×8` wings
      (`texOffs(16, 14)`, the left wing's UV mirrored) parented under the body so the body
      tilt carries them; the full `VexModel.setupAnim` pose (head look `yRot`/`xRot`, idle
      arms at `±(π/5 + cos(ageInTicks · 5.5°) · 0.1)` z-roll, idle body tilt `π/20`, and the
      wing flap `leftWing.yRot = 1.0995574 + cos(ageInTicks · 45.836624°) · 16.2°` mirrored on
      the right wing with both wings pitched/rolled `0.47123888`), driven by the projected head
      yaw/pitch and `age_in_ticks`, under the standard `LivingEntityRenderer.setupRotations`.
      The charging pose is now projected: `Vex.isCharging` (the synced `DATA_FLAGS_ID & 1`, data
      id `16`, gated to the vex type by `vanilla_is_vex` → `vex_charging`) levels the body
      (`xRot = 0`) and runs `setArmsCharging` on both render paths: if both hands are empty both
      arms pitch to `xRot = -1.2217305`, yaw to `±π/12`, and roll to `∓0.47123888 ∓ bob`; if a
      hand item state is non-empty, that arm instead pitches to the held-item `xRot = π*7/6` while
      the empty hand keeps the pre-charging rest roll. Native projects the default RIGHT Vex main
      hand from `EquipmentSlot.MAINHAND` and the LEFT hand from `EquipmentSlot.OFFHAND` into
      `vex_right_hand_item_non_empty` / `vex_left_hand_item_non_empty`. The textured base layer
      draws into the translucent mesh
      (`RenderTypes::entityTranslucent`) while preserving explicit `entityTranslucent`
      submission metadata for `ModelLayers.VEX`, texture, white tint, root transform, `order(0)`, the projected
      full-bright block-light input, and hurt/white overlay metadata before folding; it uses the
      same animated body→arm/wing hierarchy as the colored path,
      and the `isCharging` texture swap IS now applied
      (`VexRenderer.getTextureLocation`: `EntityModelKind::Vex { charging }` selects
      `textures/entity/illager/vex_charging.png` over `vex.png`, the same 32×32 model, driven by the
      already-projected `charging` bit). The constant full-bright `getBlockLightLevel` (→ 15) glow IS now applied
      (`entity_light_coords` forces the packed block light to 15 for the vex type). Broader lighting
      presentation remains unsupported
    - allay entities are wired end to end on both render paths: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `2` to the real `AllayModel`, replacing
      the former placeholder box. Renderer-owned vanilla 26.1 `AllayModel.createBodyLayer()`
      geometry: the `(0, 23.5, 0)` model root carrying the 5³ head and the body (a plain
      `texOffs(0, 10)` 3×4×2 box plus a `texOffs(0, 16)` 3×5×2 box inset by
      `CubeDeformation(-0.2)`), with the two arms (`texOffs(23, 0)`/`texOffs(23, 6)`, 1×4×2
      inset by `CubeDeformation(-0.01)`) and the two zero-thickness `0×5×8` wings
      (`texOffs(16, 14)`, neither mirrored) parented under the body so the body tilt carries
      them; the `AllayModel.setupAnim` idle / flying pose (head look `yRot`/`xRot`,
      arm idle roll `±(0.43633232 - cos(ageInTicks · 9° + 3π/2) · π · 0.075 · (1 -
      flyingFactor))`, wings flapping `yRot = ±π/4 ∓ (cos(ageInTicks · 20° + walkPos) · π ·
      0.15 + walkSpeed)` and pitched `0.43633232 · (1 - flyingFactor)`, body tilt
      `flyingFactor · π/4`, and the vertical root bob `23.5 + cos(ageInTicks · 9°) · 0.25 · (1
      - flyingFactor)` with `flyingFactor = min(walkSpeed / 0.3, 1)`), driven by the projected
      head yaw/pitch, walk animation, and `age_in_ticks`, under the standard
      `LivingEntityRenderer.setupRotations`. The dance pose IS now driven: while the synced
      `DATA_DANCING` flag is set the head-look is replaced by the dance head tilt (`head.yRot =
      cos(danceSpeed) · 30° · (1 - spinningProgress)`, `head.zRot = cos(danceSpeed) · 14° · (1 -
      spinningProgress)`) and the body sway (`root.zRot = cos(danceSpeed) · 16° · (1 -
      spinningProgress)`, `danceSpeed = ageInTicks · 8° + walkSpeed`), whirling the whole root
      `root.yRot = 4π · spinningProgress` during the `isSpinning` sub-window
      (`dancingAnimationTicks % 55 < 15`); the world client tick reconstructs the
      `dancingAnimationTicks` / `spinningAnimationTicks` accumulators from the `DATA_DANCING`
      rising edge and projects `isDancing` / `isSpinning` / `spinningProgress`. The textured base
      layer draws the `textures/entity/allay/allay.png` atlas reference into the translucent mesh
      (`RenderTypes::entityTranslucent`) while preserving explicit `entityTranslucent`
      submission metadata for `ModelLayers.ALLAY`, texture, white tint, root transform, `order(0)`, the projected
      full-bright block-light input, and hurt/white overlay metadata before folding; it uses the
      same animated body→arm/wing hierarchy as the colored path.
      The held-item arm poses (`holdingAnimationProgress` scaling
      the arm roll to zero and adding the `±0.27925268` arm yaw plus the flying-lerped arm pitch
      and held item) remain unsupported; the constant full-bright `getBlockLightLevel` (→ 15) glow IS
      now applied (`entity_light_coords` forces the packed block light to 15 for the allay type).
      Broader lighting presentation remains unsupported
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
      animation, and `age_in_ticks`, under the standard `LivingEntityRenderer.setupRotations`;
      `StriderRenderState.isRidden` is projected from passengers and zeros body pitch/yaw, matching
      vanilla when the strider is a vehicle.
      The textured base layer draws the `textures/entity/strider/strider.png` /
      `strider_baby.png` atlas references into the cutout mesh (default
      `RenderTypes::entityCutout`) while preserving explicit base submission metadata for
      `ModelLayers.STRIDER` / `STRIDER_BABY`, texture, white tint, root transform, entity light,
      hurt/white overlay, and `order(0)`;
      folded base vertices inherit that metadata. It is hand-emitted through the
      same animated leg/body/bristle hierarchy as the colored path. The cold/suffocating
      texture swap IS wired:
      `StriderRenderer.getTextureLocation` returns `strider_cold.png` / `strider_cold_baby.png` when
      `isSuffocating()`, projected onto `cold` from the synced `DATA_SUFFOCATING` flag (19) and selected
      via `strider_texture_ref(baby, cold)` (the strider texture set grows to four). The adult saddle
      equipment layer is implemented from `EquipmentSlot.SADDLE`: vanilla `StriderRenderer` adds
      `SimpleEquipmentLayer(STRIDER_SADDLE)` with `AdultStriderModel(ModelLayers.STRIDER_SADDLE)`,
      `LayerDefinitions` maps that layer to the same adult strider body layer, and the renderer draws
      `textures/entity/equipment/strider_saddle/saddle.png` (64×128) as an `armorCutoutNoCull`
      submission generated from `equipment_layer_pass` for vanilla `ModelLayers.STRIDER_SADDLE`
      at the same collector order with `submit_sequence = 1`, entity light, and
      `OverlayTexture.NO_OVERLAY`, after the base submit. Folded saddle vertices inherit the saddle metadata;
      missing-atlas coverage pins that the saddle submission is still recorded without
      `strider_saddle/saddle.png` while only folded saddle geometry is suppressed.
      Baby striders intentionally skip this layer because vanilla supplies `null` for the baby
      saddle model, and tests pin that the baby saddle path still has only the base submission.
      The suffocating shake is also
      covered: native mirrors `StriderRenderer.isShaking = super.isShaking || state.isSuffocating` and
      folds the existing `setupRotations` body-shake formula into `body_rot` from the same synced
      `DATA_SUFFOCATING` flag.
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
      mesh while preserving explicit submission metadata for `ModelLayers.TURTLE` / `TURTLE_BABY`,
      texture, white tint, root/egg-drop
      transform, entity light, hurt/white overlay, and `order(0)`, plus the vanilla render type
      split: adult `AdultTurtleModel` uses `RenderTypes::entityCutout`, while baby
      `BabyTurtleModel` uses `RenderTypes::entityCutoutCull`. Both are hand-emitted through the
      same animated head/body/leg hierarchy as the colored path, and folded cutout vertices inherit
      the base submission's light/overlay metadata. When the adult turtle carries an egg, the
      `egg_belly` overlay shell (`AdultTurtleModel`'s `texOffs(70, 33)` 9×18×1 plane at the body
      pose) is drawn on both paths and the whole model is shifted by `this.root.y--`, gated on the
      synced `Turtle.HAS_EGG` boolean (data id 18) and cleared for babies (`hasEgg = !isBaby() &&
      hasEgg()`), with native projection + renderer tests proving the cube count, the root shift,
      and the baby exclusion. While a turtle `isLayingEgg` (synced `Turtle.LAYING_EGG` boolean,
      data id 19), the shared `TurtleModel.setupAnim` land branch quadruples the front legs' yaw
      frequency (`layEgg = 4`) and doubles their amplitude (`layEggAmplitude = 2`) to mime digging
      the nest, leaving the hind legs and the water paddle untouched; this is applied on both
      paths for adults and babies alike (the amplitude lives in the base model, so it is NOT
      baby-gated), with tests pinning the exact `cos(4·pos·5)·8·speed·2` front-leg curve. The
      turtle is now fully aligned with vanilla 26.1
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
      mesh (vanilla `RenderTypes::entityCutoutCull`) while preserving explicit submission metadata
      for `ModelLayers.BAT`, texture, white tint, root transform, `LivingEntityRenderer` light plus hurt/white overlay,
      and `order(0)`; folded cutout vertices inherit the matching metadata before the model is
      hand-emitted through the same animated
      head/body/wing hierarchy as the colored path. The `isResting` branch is implemented on both
      paths: while the projected `bat_resting` (the synced `Bat.DATA_ID_FLAGS & 1`) is set the model
      swaps to the static `BAT_RESTING` hanging pose (head/body flipped 180° about X plus `+0.5` y,
      wings folded `±10°`/tips `∓120°`) and `applyHeadRotation` turns the head by the look yaw. The
      `AnimationState` start-tick phase offset and the keyframe `CATMULLROM` interpolation / `Scale`
      target (only `LINEAR` + position/rotation are ported so far) remain unsupported
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
      mirrored) and — unless the bee is angry — `bobUpAndDown` rocks the bone pivot (`xRot`, `y`),
      the front/back legs, and — on adults — the antennae, with the middle leg held at `π/4`; on the
      ground the model rests at its bind pose. The anger gate is implemented on both paths: the
      projected `bee_angry` (`Bee.isAngry()`, the synced `NeutralMob` `DATA_ANGER_END_TIME` vs the
      world game time) suppresses `bobUpAndDown`, so an angry airborne bee keeps flapping but holds
      its body, legs (at `π/4`) and antennae still. The textured base layer draws the selected
      `textures/entity/bee/bee[_angry][_nectar][_baby].png` atlas reference into the cutout mesh
      (vanilla `BeeModel` calls `EntityModel`'s default `RenderTypes::entityCutout`) while preserving
      explicit base submission metadata for `ModelLayers.BEE` / `BEE_BABY`, texture, white tint,
      root transform, and `order(0)`;
      it is hand-emitted through the same animated hierarchy as the colored path (which approximates
      the striped texture with a single representative yellow). Vanilla's separate
      `BeeStingerModel` uses `RenderTypes::entityCutoutCull`; bbb still folds the visible carried
      stinger cube into the base BeeModel tree for now. The stinger-loss visibility is implemented on
      both paths: the stinger cube is drawn only while the projected `bee_has_stinger`
      (`!Bee.hasStung()`, the synced `DATA_FLAGS_ID & 4`) is set,
      dropping to eight cubes once the bee stings. The barrel-roll somersault is implemented on both
      paths: the projected `bee_roll_amount` (`Bee.getRollAmount(partialTick)`, the client
      `updateRollAmount` accumulator easing toward the synced `DATA_FLAGS_ID & 2` roll flag — `+0.2`
      while rolling, `-0.24` otherwise) tips the `bone` pivot onto its back via `BeeModel.setupAnim`'s
      final `bone.xRot = rotLerpRad(rollAmount, bone.xRot, 3.0915928)`. The full
      `BeeRenderer.getTextureLocation` eight-way texture swap is implemented (`EntityModelKind::Bee`
      now carries `angry` / `has_nectar`): the projected `hasNectar` (the synced `DATA_FLAGS_ID & 8`,
      index 18) and `isAngry` (the synced `DATA_ANGER_END_TIME`, index 19, vs the world game time)
      select among `bee[_angry][_nectar][_baby].png` (the six new variant references join the master
      atlas array → 311), so a pollen-laden or aggravated bee shows the matching face on the textured
      path while the model/animation stay identical. Textured bee base submissions now also pin
      vanilla entity light plus hurt/white overlay metadata, and folded cutout vertices inherit that
      metadata under non-default light/overlay state
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
      representative slate). The pose-driven action animations are now reproduced too: the synced
      `DATA_POSE` (id 6) drives the `SHOOT` (1.125s, `Pose.SHOOTING`), `INHALE` (2.0s, `INHALING`),
      `SLIDE` (0.2s, `SLIDING`), and `JUMP` (0.5s, `LONG_JUMPING`) one-shots — each `animateWhen(pose
      == X)` the vanilla way (`onSyncedDataUpdated` `resetAnimations` + `startIfStopped`) — plus the
      `SLIDE_BACK` (0.1s) return fired on the falling edge of `Pose.SLIDING`, all projected as elapsed
      seconds (`-1.0` stopped) and applied additively over the idle in vanilla `setupAnim` order to the
      base body layer's `body`/`head`/`rods` bones (the actions' `wind_*` channels target the deferred
      wind layer's parts, which are absent, so they are skipped). The non-looping actions clamp past
      their length to the final frame (world + renderer tests pin the pose state machine including the
      slideBack transition, the five definitions, and each action re-posing the body model). The two
      overlay layers are now reproduced too. The emissive eyes (vanilla `BreezeEyesLayer`) are an
      always-on second textured pass re-rendering the base body with the `breeze_eyes.png` reference in
      the eyes (emissive) render type — transparent except at the head's eye UVs — mirroring the
      creaking eyes pass but ungated. The swirling wind body (vanilla `BreezeWindLayer`) is a SEPARATE
      `BreezeWindModel` (vanilla `createWindLayer`, atlas 128×128) — the `wind_body` pivot → `wind_bottom`
      → `wind_mid` → `wind_top` shell chain, three concentric shells per tier — rendered into the
      translucent scrolling-overlay mesh with the `breeze_wind.png` reference, its U coordinate scrolled
      by `ageInTicks · 0.02` (vanilla `RenderTypes.breezeWind`), exactly like the wind charge folds into
      the same scroll pass. Its `setup_anim` applies the same idle (the `wind_top`/`wind_mid` LINEAR
      position sways, now transcribed into `BreezeAnimation.IDLE`) plus the action one-shots' `wind_*`
      rotation/position swirls and the `JUMP`/`INHALE` `wind_body`/`wind_bottom` `SCALE` pulses (folded
      onto the reset scale, vanilla `ModelPart.offsetScale`), so the wind body moves with the base body.
      The colored debug path keeps the base body only, consistent with every other scrolling overlay
      (the energy swirls, the guardian beam). Renderer tests pin the wind geometry, the eyes pass,
      the wind body folding into the scroll mesh and U-scrolling past the looped idle, plus vanilla
      submission metadata: `breeze_textured_layer_passes` records vanilla `ModelLayers.BREEZE`,
      `ModelLayers.BREEZE_WIND`, and `ModelLayers.BREEZE_EYES`, with texture, render type,
      `vanilla_name()`, white tint, and order/sequence coverage. The base `entityTranslucent` submit
      keeps entity light plus hurt/white overlay, while `BreezeEyesLayer` and `BreezeWindLayer`
      preserve entity light and force `OverlayTexture.NO_OVERLAY` before folded eyes and scroll
      vertices inherit their submit metadata; `BreezeWindLayer` now also pins the vanilla `breezeWind`
      render-type name, and missing-atlas coverage proves the wind submit survives without
      `breeze_wind.png` while only folded scroll geometry is suppressed, and the eyes submit survives
      without `breeze_eyes.png` while only folded emissive eyes geometry is suppressed.
      Breeze is now fully aligned with vanilla 26.1
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
      mesh (`DolphinModel` calls `EntityModel`'s default `RenderTypes::entityCutout`) while
      preserving explicit base submission metadata for `ModelLayers.DOLPHIN` / `DOLPHIN_BABY`,
      texture, white tint, adult/baby
      mesh-transformer root transform, `LivingEntityRenderer` light plus hurt/white overlay, and
      `order(0)`; folded cutout vertices inherit the matching metadata, and the model is hand-emitted
      through the same animated hierarchy as the colored path (which approximates the texture with a
      single representative grey). This already covers the older goal-listed dolphin
      swim re-pose item. The held-item carry layer (`DolphinCarryingItemLayer`) is now implemented
      through the shared item-model pass: renderer exposes `dolphin_carried_item_transform`, which keeps
      the item in the unscaled entity root frame and applies the vanilla `xRot`-dependent `(0, y, z)`
      offset, while native reads the main-hand stack and bakes it with `ItemDisplayContext.GROUND`.
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
      scale). The head look (`head.yRot/xRot = state.yRot/xRot`) IS reproduced — every part hangs off
      `head`, so folding `head_look_pose` into `head_t` turns the whole guardian with the projected
      `head_yaw` / `head_pitch`, with a test pinning that both the guardian and the scaled elder turn.
      `GuardianModel.setupAnim` also pulses each spike in and out with the projected `ageInTicks`
      (`getSpikeOffset = 1 + cos(ageInTicks · 1.5 + i) · 0.01`, the spikes being the head's first
      twelve children), with a test pinning that the spikes move with the age phase and that
      `ageInTicks = 0` reproduces the baked bind pose. The three-segment tail sway IS reproduced —
      `Guardian.aiStep`'s client `clientSideTailAnimation` accumulator runs world-side each client
      tick (its `clientSideTailAnimationSpeed` ramps to `2.0` out of water, snaps toward `0.5` while
      moving in water, and settles toward `0.125` while idle, off the per-tick `isInWater()` resolved
      from the chunk fluid state and the synced `DATA_ID_MOVING` `isMoving()` flag), and the lerped
      `tailAnimation` drives `tailParts[i].yRot = sin(swim) · π · {0.05, 0.1, 0.15}`, with world and
      renderer tests pinning the three speed branches and the off-bind tail sway. The starting tail
      phase is `0.0` (vanilla seeds it with a per-spawn `random.nextFloat()`, which is
      non-deterministic — only the starting phase is approximated; the sway dynamics are exact). The
      textured guardian/elder base submission now pins vanilla `LivingEntityRenderer.submit`
      metadata through explicit `GuardianBase` passes for `ModelLayers.GUARDIAN` /
      `ModelLayers.ELDER_GUARDIAN`: texture, `entityCutout` render type, white tint,
      root/elder-scale transform,
      entity light, hurt/white overlay, and `order(0)`, and folded cutout vertices inherit that
      base submit metadata. The
      spike WITHDRAWAL is now reproduced too: the same `Guardian.aiStep` eases
      `clientSideSpikesAnimation` (spawn `0`) IN WATER toward `0` while `isMoving()` (by `0.25`, the
      spikes retract as it swims) or toward `1` while idle (by `0.06`, the spikes fully extend), driven
      off the same per-tick `isInWater()` + `DATA_ID_MOVING`, and `GuardianModel.setupAnim` subtracts
      `withdrawal = (1 - spikesAnimation) · 0.55` from every spike offset (world + renderer tests pin the
      in-water ease branches and the off-bind retraction). The out-of-water branch (`spikesAnimation =
      random.nextFloat()` each tick) is the one piece deferred — its unseeded client RNG is not
      reconstructable, so the value is held steady out of water rather than faked (a flopping/dying
      guardian edge case). The eye target tracking (`lookAtPosition`/`lookDirection`/`eyePosition`) still
      reads entity-side state not yet projected. The `GuardianRenderer` attack beam GEOMETRY is now built (renderer slice,
      `emit_guardian_beam`): when the `guardian_beam` render state is set, the vanilla `renderBeam`
      12-vertex twisted prism (two crossed inner-radius `0.2` strips + a `0.282` twisting top cap, spun
      by `rot = attackTime · 0.05 · -1.5`, tinted by the `colorScale = attackScale²` ramp) is drawn in a
      world-aligned frame (`translate(pos) · translate(0, eyeHeight, 0) · rotY(yRot) · rotX(xRot)`,
      orienting local +Y onto the world `eye_to_target` vector — no body yaw, matching vanilla where the
      beam draws after `super.submit` pops `setupRotations`), folded into the scroll (fract-wrap) pass so
      `guardian_beam.png` tiles vertically over `length · 2.5`; the submission now consumes explicit
      `GuardianBeam` pass metadata and pins the vanilla `entityCutout` render-type name, texture,
      attack-scale tint, order `(0, 1)`, beam transform,
      `setLight(15728880)` full-bright, and `OverlayTexture.NO_OVERLAY` instead of inheriting entity
      hurt/white overlay or sampled entity light, including a missing-atlas regression where only the
      folded scroll geometry is suppressed. The WORLD projection that fills
      `guardian_beam` is now wired end-to-end: the synced `DATA_ID_ATTACK_TARGET` (idx 17, the int right
      after `DATA_ID_MOVING`) drives a client-side `GuardianAttackAnimationState.clientSideAttackTime`
      counter (`Guardian.aiStep` ramps it to `getAttackDuration()` = `80` guardian / `60` elder while a
      target is locked, reset on target change), and `WorldStore::model_source` resolves the target
      entity cross-entity (`self.transform` + `self.pick_bounds` for `bbHeight · 0.5`) to project
      `eye_to_target = targetCenter − guardianEye`, `eye_height`, `attackTime = clientSideAttackTime +
      partialTicks`, and `attackScale = getAttackAnimationScale`, which the native scene maps onto the
      renderer's `GuardianBeamRenderState`. So a guardian locked onto a live target now fires its beam.
      The base texture is bound on the textured path (`GUARDIAN_TEXTURE_REF` / `GUARDIAN_ELDER_TEXTURE_REF`
      / `GUARDIAN_BEAM_TEXTURE_REF`). The colored debug path stays as a fallback (it approximates the body
      with a single teal tint and the eye with a pink tint)
    - frog entities as renderer-owned vanilla 26.1 `FrogModel.createBodyLayer()` geometry on the
      textured path: the native entity scene (`entity_scene.rs`) projects vanilla type id `55` to
      `EntityModelKind::Frog { variant }`, replacing the former placeholder box. The static
      rest-pose hierarchy is emitted directly (atlas 48×48): the `root` part at `offset(0, 24, 0)`
      parents `body` (the 7×3×9 box + 7×0×9 underside plane) and the two legs; `body` parents the
      head (7×0×9 plane + 7×3×9 box) with its `eyes` pivot and two 3×2×3 eyes, the `croaking_body`
      pouch, the tongue, and the two 2×3×3 arms, each carrying an 8×0×8 webbed hand; each leg carries
      an 8×0×8 foot — fifteen visible cubes plus the `croaking_body` pouch (hidden until the frog
      croaks). The looping `FrogAnimation.FROG_WALK` keyframe cycle is reproduced: `FrogModel.setupAnim`
      samples it via `applyWalk(walkAnimationPos, walkAnimationSpeed, 1.5, 2.5)` — the walk position
      drives the sample time (`(long)(pos·50·1.5)` ms, wrapped by the 1.25 s length) and the walk speed
      scales the amplitude (`min(speed·2.5, 1)`), so a still frog collapses to the bind pose — and the
      sampled per-bone position/rotation offsets are folded onto the `body`, the two arms, and the two
      legs (the spine is hand-walked). The triggered `FrogAnimation.FROG_CROAK` pouch animation is also
      reproduced as the first consumer of the triggered-keyframe tier: the client `frog_croak`
      `KeyframeAnimationState` (mirroring vanilla `AnimationState`) is started/stopped from the synced
      `Pose.CROAKING` (id `8`) — `animateWhen(pose == CROAKING, ageTicks)` — and projects the elapsed
      seconds since the croak started (or the `-1.0` not-croaking sentinel). While croaking,
      `FrogModel.setupAnim` shows the `croaking_body` pouch and samples `FROG_CROAK`'s POSITION channel
      (the pouch lifts) and SCALE channel (it puffs `(1.3, 2.1, 1.6)` twice and rests collapsed) via the
      new `AnimationChannel.Targets.SCALE` keyframe target and per-part `ModelPart.scale`. The triggered
      `FrogAnimation.FROG_JUMP` long-jump animation is reproduced the same way: the client `frog_jump`
      `KeyframeAnimationState` is started/stopped from the synced `Pose.LONG_JUMPING` (id `6`) and
      projects the elapsed seconds since the long-jump started (or the `-1.0` not-jumping sentinel).
      While long-jumping, `FrogModel.setupAnim` applies `FROG_JUMP` (0.5 s, not looping) before the
      croak — a static hold pose that tips the `body` back `-22.5°`, tucks the two arms back `-56.14°`
      and lifts them `+1` y, and cocks the two legs `45°`, folded additively onto the walk pose. The
      looping `FrogAnimation.FROG_IDLE_WATER` in-water idle is reproduced too: unlike the pose-driven
      croak/jump, the client `frog_swim_idle` `KeyframeAnimationState` is driven each client tick by
      `Frog.tick`'s `animateWhen(isInWater() && !walkAnimation.isMoving(), tickCount)` — `isInWater()`
      is the per-tick world fact resolved from the chunk fluid state (the frog joins the guardian as a
      consumer of `entity_animation_uses_in_water`), and `walkAnimation.isMoving()` (`speed > 1e-5`)
      reads the prior tick's limb-swing speed (the walk accumulator advances after the per-type match);
      the frog's `updateWalkAnimation` override is now reproduced (`targetSpeed =
      jumpAnimationState.isStarted() ? 0 : min(distance * 25, 1)`, with the usual 0.4 walk-state
      low-pass), so a moving in-water frog stops the idle hover on the next tick. It projects the
      elapsed seconds since the idle started (or the `-1.0` dry/moving sentinel). While idling underwater,
      `FrogModel.setupAnim` applies `FROG_IDLE_WATER` (3.0 s, looping, all-CATMULLROM) LAST (after the
      walk/swim, jump, and croak) — the `body` dips `-10°`, the two arms splay `±22.5°→±45°` z and sink
      `-0.5` y, and the two legs swing out and sink `-1` y, folded additively onto the walk/swim pose.
      The moving ground and in-water cycles are covered too: `FrogModel.setupAnim` samples
      `FROG_WALK.applyWalk(walkAnimationPos, walkAnimationSpeed, 1.5, 2.5)` while dry and
      `FROG_SWIM.applyWalk(..., 1.0, 2.5)` while `FrogRenderState.isSwimming` (`entity.isInWater()`),
      with renderer tests pinning the `FROG_SWIM` definition and branch selection. The
      `FROG_TONGUE` lash (0.5 s, NOT looping) is reproduced the same way: the client `frog_tongue`
      `KeyframeAnimationState` is started/stopped by the synced `Pose.USING_TONGUE` (id 9) exactly like
      the croak (id 8), projecting the elapsed seconds (or the `-1.0` sentinel); `FrogModel.setupAnim`
      then dips the `head` `-60°` xRot and lashes the `tongue` part forward via a z-SCALE to 5×
      (`scaleVec(0.5, 1, 5)`), with the vanilla quirk that the head SCALE channel uses `degreeVec`
      (a tiny `~0.0174` scale offset) transcribed exactly. The cross-entity prey-targeting that visually
      aims the tongue at the eaten entity (`DATA_TONGUE_TARGET_ID`) is NOT part of the model animation
      and stays deferred. The textured path now binds the three `FrogVariant` temperature
      textures (`frog_temperate`/`frog_warm`/`frog_cold.png`): the native scene reads `DATA_VARIANT_ID`
      (18, `Holder<FrogVariant>`) and resolves the registry id against the synced
      `minecraft:frog_variant` registry (static `FrogVariants.bootstrap` fallback
      temperate=0/warm=1/cold=2), so `FrogRenderer.getTextureLocation`'s per-variant asset is matched;
      textured regressions now pin the `FrogBase` pass identity, vanilla `entityCutout` render
      type/name, vanilla `ModelLayers.FROG` (`minecraft:frog#main`), white tint, root transform,
      `(order, submit_sequence) == (0, 0)`, and the
      `MobRenderer` / `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata, with
      folded cutout vertices inheriting the same metadata;
      only the tongue prey-targeting stays deferred
    - creaking entities as renderer-owned vanilla 26.1 `CreakingModel.createBodyLayer()` geometry
      on the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id
      `31` to the new `EntityModelKind::Creaking`, replacing the former placeholder box. The static
      rest-pose hierarchy is emitted directly (atlas 64×64): the `root` part at `offset(0, 24, 0)`
      parents the `upper_body` pivot and the two legs; `upper_body` parents the head (the 6×10×6
      skull, the 6×3×6 brow, and two 9×14×0 antler/branch planes), the body (6×13×5 trunk + 6×7×5
      block), and the two asymmetric arms (the right a 3×21×3 limb + hand, the left a 3×16×3 limb +
      two blocks); each leg carries a 5×0×9 foot plane and the right leg an extra 3×3×3 hip block —
      sixteen cubes. Two motions are reproduced by hand-walking the spine: the head look
      (`head.xRot/yRot = state.xRot/yRot`, the head nested root → upper_body → head) and the looping
      `CreakingAnimation.CREAKING_WALK` keyframe cycle, sampled via
      `applyWalk(walkAnimationPos, walkAnimationSpeed, 1, 1)` — it offsets the upper_body, head, the
      two arms (rotation), and the two legs (rotation + position), and the head channel ADDS onto the
      look the head already tracks (tests pin the head-only look, the 53-keyframe definition, and the
      look composing onto the walking head). The `canMove` freeze gate is now PROJECTED and consumed:
      the synced `CAN_MOVE` boolean (id 16, default true) is read directly into the model source, and
      `setupAnim` skips `applyWalk` when it is false — a creaking observed mid-step holds the bind pose
      plus its look and turns to a statue. The three triggered combat/death keyframe one-shots are now
      reproduced the vanilla way too: `Creaking.handleEntityEvent(4)`/`(66)` seed
      `attackAnimationRemainingTicks = 15` / `invulnerabilityAnimationRemainingTicks = 8`, the client
      `aiStep` decrements both each tick (BEFORE `setupAnimationStates`, unlike the rabbit), and
      `setupAnimationStates` `animateWhen`s the looping 0.7083 s `CREAKING_ATTACK` lunge and the
      non-looping 0.2917 s `CREAKING_INVULNERABLE` stagger on `ticks > 0`; the 2.25 s `CREAKING_DEATH`
      collapse `animateWhen`s on the synced `isTearingDown()` (`IS_TEARING_DOWN`, id 18) directly. The
      three definitions are transcribed exactly (68 / 19 / 52 keyframes), projected as elapsed seconds
      (or the `-1.0` stopped sentinel) and applied additively over the walk in vanilla order, with the
      attack/death `SCALE` channels folded onto the part scale (tests pin the tick windows, the three
      definitions, the freeze gate, and each one-shot re-posing off the bind pose). The base texture is
      now bound on the textured path
      (`CREAKING_TEXTURE_REF`), together with the emissive eyes overlay
      (`CREAKING_EYES_TEXTURE_REF`, vanilla `CreakingRenderer`'s `LivingEntityEmissiveLayer`): an
      eyes-render-type pass over vanilla `CreakingModel.createEyesLayer()`'s retained `head` subset at
      vanilla `order(1)`, gated on `eyes_glowing` projected from the synced `IS_ACTIVE` flag (17).
      `creaking_textured_layer_passes` now records vanilla `ModelLayers.CREAKING`
      (`minecraft:creaking#main`) for the base and `ModelLayers.CREAKING_EYES`
      (`minecraft:creaking#eyes`) for the head-only emissive layer. Tests pin the base
      `CreakingBase` `entityCutout` order 0 submission and the `CreakingEyes` eyes order 1
      submission, including the retained head visibility, base entity light plus hurt/white overlay,
      and the emissive eyes pass's entity light plus
      `getOverlayCoords(state, 0.0F)` red-row/zero-white overlay, before checking folded
      cutout/eyes geometry; missing-atlas coverage proves the eyes submission is still recorded without
      `creaking_eyes.png` while only folded emissive geometry is suppressed. Only the tearing-down
      death-flicker (`hasGlowingEyes`, a client-tick toggle) stays deferred. The colored debug path
      stays as a fallback (it approximates the whole model with one dark-bark tint)
    - sniffer entities as renderer-owned vanilla 26.1 `SnifferModel.createBodyLayer()` geometry on
      the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `119`
      to the new `EntityModelKind::Sniffer`, replacing the former cow-quadruped approximation (the
      sniffer no longer borrows the `CowModel`). The static rest-pose hierarchy is emitted directly
      (atlas 192×192): the `bone` part at `offset(0, 5, 0)` parents the body (the 25×29×40 trunk, a
      25×24×40 inner block inflated by `CubeDeformation(0.5)` — geometry `min -= 0.5`, `size += 1`,
      baked into the colored cube exactly like the vex/illager deformed cubes — and the 25×0×40
      belly plane) and the six 7×10×8 legs; the body parents the head (13×18×11 skull + top plane)
      which parents the two 1×19×7 ears, the 13×2×9 nose pad, and the 13×12×9 lower beak — fifteen
      cubes. The head look (`head.xRot/yRot = state.xRot/yRot`) IS reproduced: the head is nested
      two levels under the root (bone → body → head), and the head's ear/nose/beak children ride with
      the turn, with a test pinning that only the head subtree moves. The default walk is also
      reproduced: while not searching, `setupAnim` samples the looping 2.0 s `SnifferAnimation.SNIFFER_WALK`
      via `applyWalk(walkAnimationPos, walkAnimationSpeed, 9.0, 100.0)` — the six legs swing (rotation +
      position), the body sways with a y-dip, the two ears roll (CatmullRom), and the head pitches
      (CatmullRom) ADDING onto the look it already tracks (a still sniffer samples amplitude 0,
      collapsing to the bind pose plus the head look). The bone → body → head spine and the six legs are
      hand-walked. The synced-`DATA_STATE` (id 18, the `Sniffer.State` ordinal VarInt) one-shot
      keyframe animations ARE now reproduced, driven the vanilla way: `Sniffer.onSyncedDataUpdated`
      `resetAnimations()`s and `startIfStopped`s the matching one-shot on each state change, so the
      world projects a compact `(sniffer_animation_id, sniffer_animation_seconds)` pair (the active
      state ordinal + elapsed seconds, `(-1, -1.0)` when idle/searching) that the renderer matches to
      layer the one-shot over the walk: DIGGING(5)→`SNIFFER_DIG` (8 s), SNIFFING(3)→`SNIFFER_LONGSNIFF`
      (1 s), RISING(6)→`SNIFFER_STAND_UP` (3 s), FEELING_HAPPY(1)→`SNIFFER_HAPPY` (2 s looping),
      SCENTING(2)→`SNIFFER_SNIFFSNIFF` (8 s looping); a state change restarts the timer from 0. The
      `SNIFFER_SNIFF_SEARCH` search-walk variant IS now reproduced: `Sniffer.isSearching()` (the synced
      `DATA_STATE == SEARCHING`, projected as `sniffer_is_searching` — the "un-synced" label was wrong;
      it is the same enum the one-shots read) swaps the looping 2.0 s `SNIFFER_SNIFF_SEARCH` in for the
      base `SNIFFER_WALK` under the same `applyWalk(..., 9, 100)`, adding a head-down shift and a `nose`
      SCALE puff over the eleven animated bones. Only the baby-transform
      (`BABY_TRANSFORM`/`SNIFFER_BABY_FALL`) stays deferred. The adult base texture is now bound on the
      textured path (`SNIFFER_TEXTURE_REF`), the primary now-wired path, with explicit submission
      metadata pinned for the `SnifferBase` pass identity, vanilla `entityCutout` render type/name,
      vanilla `ModelLayers.SNIFFER` (`minecraft:sniffer#main`), white tint, root transform,
      `(order, submit_sequence) == (0, 0)`, and the
      `AgeableMobRenderer` / `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata, with
      folded cutout vertices inheriting that metadata.
      The colored debug path stays as a fallback (it approximates the body with one brown tint and the
      nose pad with a pink tint)
    - warden entities as renderer-owned vanilla 26.1 `WardenModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `142` to the
      new `EntityModelKind::Warden`, replacing the former placeholder bounds box. The static rest-pose
      hierarchy is emitted directly (atlas 128×128): the `bone` part at `offset(0, 24, 0)` parents the
      body (the 18×21×11 torso) and the two 6×13×6 legs (differing only in X origin, ±5.9); the body
      parents the two 9×21×0 ribcage planes, the head (16×16×10 skull) and the two mirrored 8×28×8
      arms; the head parents the two 16×16×0 tendril planes — ten cubes. Four non-keyframe motions are
      reproduced: the head look (`WardenModel.animateHeadLookTarget` sets `head.xRot/yRot` from the
      projected `head_pitch/head_yaw`, so the body-nested head and its two tendrils track the look target),
      the always-on idle wobble (`animateIdlePose` rolls the body `xRot/zRot += 0.025·cos/sin(age·0.1)`
      and the head `xRot/zRot += 0.06·sin/cos(age·0.1)` off the projected `age_in_ticks`, hand-walking the
      `bone → body → head` spine so the body roll carries its subtree), the walk (`animateWalk` swings
      the head, body, two legs, and two arms off the projected `walk_animation_pos/speed` — `speedModifier
      = min(0.5, 3·speed)`, `adjustedPos = pos·0.8662` — via `warden_walk_pose`, applied as an additive
      `xRot/zRot` layer over the look/idle pose since the motions compose additively, with tests
      pinning the sampled offsets against the vanilla arithmetic and that walking swings the otherwise-still
      legs and arms), and the tendril sway (`animateTendrils` swings the two head tendrils' `xRot` by
      `tendrilAnimation·cos(age·2.25)·π·0.1` — the right negated — off a newly projected `tendril_animation`
      pulse: the canonical client-side `Warden.tendrilAnimation` countdown, reset to `10` by entity event
      `61` and decremented each client tick in `bbb-world`, exposed lerped `/10` like `Warden.getTendrilAnimation`,
      with tests pinning the countdown, the sway formula, and the world→render projection). Four of the
      six triggered combat/threat keyframe animations are now reproduced and applied additively in the
      vanilla `setupAnim` order (attack → sonic_boom → dig → emerge → roar → sniff): the roar
      (`WARDEN_ROAR`, 4.2s), sniff (`WARDEN_SNIFF`, 4.16s), emerge (`WARDEN_EMERGE`, 6.68s, 200 keyframes) and
      dig (`WARDEN_DIG`, 5.0s, 108 keyframes) are pose-driven — `Warden.onSyncedDataUpdated` `.start()`s the
      matching one-shot when the synced `DATA_POSE` (id 6) CHANGES to `Pose.ROARING` (11) / `Pose.SNIFFING` (12)
      / `Pose.EMERGING` (13) / `Pose.DIGGING` (14), tracked via a `prev_pose` on the new
      `WardenCombatAnimationState`; the attack (`WARDEN_ATTACK`, 0.33333s) and sonic boom (`WARDEN_SONIC_BOOM`,
      3.0s) are event-driven — `handleEntityEvent(4)` starts the attack and stops the roar,
      `handleEntityEvent(62)` starts the boom. Each one-shot is a reusable `KeyframeAnimationState` projected
      as an independent elapsed-seconds value (`warden_roar/sniff/attack/sonic_boom/emerge/dig_seconds`, `-1.0`
      when stopped); all six are non-looping, so the renderer clamps past the def length to the resting final
      frame — mirroring vanilla's "hold the last frame" (no auto-stop on pose leave). The emerge/dig
      spawn/despawn tables (transcribed via the big-table script approach, mixing LINEAR/CATMULLROM) are the
      only two that animate the legs — `apply_combat` now runs over the legs too, where the other four defs
      carry no leg bone and add zero. World tests pin the pose/event transitions, the attack stopping the roar,
      and the emerge→dig pose handoff; renderer tests pin all six def lengths/looping/bone-counts and that
      roaring/attacking/booming/emerging/digging re-pose vs bind and differently from each other (emerge/dig
      also moving the legs). The base texture is now bound on the textured path (`WARDEN_TEXTURE_REF`), together with all five
      `WardenEmissiveLayer`s as eyes-render-type passes (the eyes pipeline being emissive + alpha-blended, so a
      pass `tint[3]` scales output alpha directly, matching vanilla `entityTranslucentEmissive` — no new
      pipeline). Each overlay is baked by vanilla
      `WardenModel.create{Bioluminescent,PulsatingSpots,Tendrils,Heart}Layer` as a `retainExactParts` subset of
      the one body mesh, reproduced by a new `ModelPart::render_textured_retained` (a retained part draws its own
      cubes and its whole subtree is dropped — vanilla `clearRecursively` — so a retained ancestor short-circuits
      its descendants) selected per pass via `EntityModelLayerVisibility::RetainedParts`: the always-on
      bioluminescent overlay (`WARDEN_BIOLUMINESCENT_TEXTURE_REF`, alpha 1.0, head/arms/legs); the two
      pulsating-spots overlays (`WARDEN_PULSATING_SPOTS_1/2_TEXTURE_REF`, body/legs — `body`'s retention drops its
      head/arm children — each fading on `max(0, cos(ageInTicks·0.045 + phase)·0.25)`, phase `0` and `π`, off the
      projected `age_in_ticks`); the tendril overlay (reusing `WARDEN_TEXTURE_REF` over the two tendril planes at
      the lerped `tendril_animation` alpha); and the heart overlay (`WARDEN_HEART_TEXTURE_REF`, body only, at the
      lerped `heart_animation` alpha). Zero-alpha emissive layers are now skipped before submission, matching
      `LivingEntityEmissiveLayer`'s `alpha <= 1e-5` gate; textured tests pin the remaining base/emissive submissions'
      texture, internal render type (`entityCutout` / `eyes`, used here for vanilla `entityTranslucentEmissive`),
      vanilla model layers (`WARDEN`, `WARDEN_BIOLUMINESCENT`, `WARDEN_PULSATING_SPOTS`,
      `WARDEN_TENDRILS`, `WARDEN_HEART`), tint alpha, root transform, entity light, base hurt/white overlay, emissive
      `getOverlayCoords(state, 0.0F)` red-row/zero-white overlay, folded cutout/eyes vertex metadata,
      and explicit `(order, submit_sequence)`; missing-atlas coverage proves the always-on bioluminescent
      submission is still recorded without `warden_bioluminescent_layer.png` while only that layer's
      folded retained geometry is suppressed. `heart_animation` mirrors the client-side `Warden.heartAnimation`/`O`
      heartbeat: `bbb-world` resets it to `10` whenever `tickCount % getHeartBeatDelay() == 0` (the delay
      `40 - floor(clamp(clientAngerLevel/80, 0, 1)·30)` shrinking from `40` calm to `10` fully angry off the
      synced `CLIENT_ANGER_LEVEL`), decrements it each client tick, and exposes it lerped `/10` like
      `Warden.getHeartAnimation`. With every `WardenEmissiveLayer` now wired, the colored debug path remains only
      as a fallback (it approximates the body with one dark-teal tint and the tendrils with a brighter cyan tint)
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
      2×2×2 legs (front legs at vanilla's swapped X origins) — ten cubes. The `isHidingInShell` visibility
      swap is now projected through the full state machine: the synced `Armadillo.ARMADILLO_STATE` (data id
      `18`, the `ArmadilloState` enum — `IDLE`=0/`ROLLING`=1/`SCARED`=2/`UNROLLING`=3, each carrying an
      `animationDuration` and `shouldHideInShell(ticksInState)`) is tracked client-side together with the
      reconstructed `inStateTicks` (the `age_ticks` recorded at the last state change), exactly as vanilla
      `Armadillo.setupAnimationStates()` reads `inStateTicks` (reset to `0` on a state change, `++` each
      tick). `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)` is projected as
      `ArmadilloRenderState.isHidingInShell` and drives `setupAnim`'s shell pose — `body.skipDraw` hides the
      body cubes, the tail and both hind legs hide, and the `cube` ball shows, while the head (+ ears) and
      both front legs stay drawn (adult 10×10×10 ball / baby 6×6×6 + `CubeDeformation(0.3)` ball; six cubes
      rolled up). The hide window is now faithful for every state: `IDLE` never hides, `SCARED` always hides,
      `ROLLING` hides once `inStateTicks > 5`, and `UNROLLING` un-hides at `inStateTicks >= 26`. The two
      transition keyframe animations are reproduced and ADD onto the walk pose during the visible not-hiding
      window: `ARMADILLO_ROLL_UP` (0.5 s, non-looping; started on entry to `ROLLING`, projected as
      `armadillo_roll_up_seconds`) curls the body/tail/head/four legs in during the first ~5 ticks before the
      ball takes over, and `ARMADILLO_ROLL_OUT` (1.5 s, non-looping; started on entry to `UNROLLING`,
      projected as `armadillo_roll_out_seconds`) un-curls them once the ball un-hides at tick 26 (the
      `body` channel is POSITION-only). Both transitions' `cube` channels stay deferred — the shell `cube`
      itself stays static for roll-up / roll-out while hiding, but the visible head/front-leg roll keyframes
      are now applied in the rolled tree too. `ARMADILLO_PEEK` SCARED animation (2.5 s, non-looping) is now reproduced: vanilla
      `start`s it then `fastForward(50, 1.0)`s it on the first SCARED setup tick, and entity event `64`
      (`peekReceivedClient`) restarts it on the next client tick. World projects `armadillo_peek_seconds`
      from the synced state + reconstructed `inStateTicks` with a signed start tick for the fast-forward
      baseline, and the renderer applies the vanilla head / front-leg / shell-`cube` keyframes even while the
      armadillo is hiding in its shell. The baby rolls share the adult roll defs (same bone names; the
      baby-specific roll keyframes stay deferred). While NOT hiding, the clamped head
      look is reproduced: `setupAnim` clamps the projected look to vanilla's bounds (pitch `head.xRot` to
      [-22.5, 25], yaw `head.yRot` to [-32.5, 32.5] degrees) and turns the body-nested head pivot so the
      snout and both ears inherit the turn; the look is skipped while hiding (the head is balled up). The
      `applyWalk` is also reproduced: while not hiding, `setupAnim` samples the looping 1.4583 s walk cycle
      via `applyWalk(walkAnimationPos, walkAnimationSpeed, 16.5, 2.5)` — the body rolls (a CatmullRom z-sway
      with a small y-bob) carrying the tail and head, the four legs swing (rotation + position), the tail
      rocks, and the head channel adds a small z-roll onto the look it already tracks (a still armadillo
      samples amplitude 0, collapsing to the bind pose plus the head look). Both the adult
      (`ArmadilloAnimation.ARMADILLO_WALK`) and the baby (`BabyArmadilloAnimation.ARMADILLO_BABY_WALK`, the
      same seven bones at slightly different timestamps) walks are reproduced, sharing one hand-walked
      `body → tail/head` + four-leg pass. The base texture is now bound on the textured path
      (`ARMADILLO_TEXTURE_REF` / `ARMADILLO_BABY_TEXTURE_REF`), the primary now-wired path, with nothing
      left deferred on the texture side. Adult and baby textured regressions now pin the `ArmadilloBase`
      pass identity, vanilla `entityCutout` render type/name, vanilla `ModelLayers.ARMADILLO` /
      `ARMADILLO_BABY` (`minecraft:armadillo#main` / `minecraft:armadillo_baby#main`), white tint, root
      transform, `(order, submit_sequence) == (0, 0)`, and the `AgeableMobRenderer` / `LivingEntityRenderer`
      `lightCoords` plus hurt/white overlay metadata. The colored debug path stays as a fallback (it approximates the
      armored body/legs with one brown tint and the soft head/ears/tail with a tan tint)
    - axolotl entities as renderer-owned vanilla 26.1 `AdultAxolotlModel` /
      `BabyAxolotlModel.createBodyLayer()` geometry on the textured path: the native entity scene
      (`entity_scene.rs`) projects vanilla type id `7` to `EntityModelKind::Axolotl { baby, variant }`,
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
      a 0×3×8 tail, a 6×3×4 head, and the three gill planes — eleven cubes. The full adult
      `AdultAxolotlModel.setupAnim` IS reproduced: the body yaw look (`body.yRot += yRot·π/180`) plus all
      five factor-blended procedural sub-animations — swimming (moving in water), water-hovering (still in
      water), ground-crawling (moving on ground), lay-still (still on ground), and play-dead — and the
      mirror-leg copy. The four blend factors come from the world client-tick `BinaryAnimator(10,
      IN_OUT_SINE)` accumulators (`playingDead` / `inWater` / `onGround` / `moving`), reconstructed from the
      synced `DATA_PLAYING_DEAD` (id 19), the per-tick `isInWater()`, `onGround()`, and
      `walkAnimation.isMoving()` (OR'd with a synced rotation change) exactly as
      `Axolotl.tickAdultAnimations` selects its mutually-exclusive `PLAYING_DEAD → IN_WATER → ON_GROUND →
      IN_AIR` state. The baby swim / walk / idle keyframe animations stay deferred (vanilla
      `BabyAxolotlModel` is a separate keyframe model).
      The five `Axolotl.Variant` color variants (lucy / wild / gold / cyan / blue, each with adult and baby
      textures) are now bound on the textured path: the native scene reads `DATA_VARIANT` (18, int) and
      `Axolotl.Variant.byId` selects the colour, crossed with the age, matching
      `AxolotlRenderer.TEXTURE_BY_TYPE` (`axolotl_<name>.png` / `axolotl_<name>_baby.png`) — ten textures.
      Adult and baby textured regressions now pin the `AxolotlBase` pass identity, vanilla
      `entityCutout` render type/name, vanilla `ModelLayers.AXOLOTL` / `AXOLOTL_BABY`
      (`minecraft:axolotl#main` / `minecraft:axolotl_baby#main`), white tint, root transform,
      `(order, submit_sequence) == (0, 0)`,
      and the `AgeableMobRenderer` / `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata,
      with folded cutout vertices inheriting that metadata
    - tadpole entities as renderer-owned vanilla 26.1 `TadpoleModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `130` to the new
      `EntityModelKind::Tadpole`, replacing the former placeholder bounds box. The static rest-pose
      hierarchy is emitted directly (atlas 16×16): two sibling root parts — a 3×2×3 body box at
      `offset(0, 22, -3)` and a 0×2×7 tail fin plane at `offset(0, 22, 0)` — two cubes. The only
      `TadpoleModel.setupAnim` motion, the tail yaw sway (`tail.yRot = -amplitude * 0.25 *
      sin(0.3 * ageInTicks)`, amplitude `1.0` in water / `1.5` on land), IS reproduced from the projected
      `age_in_ticks` + `in_water` (mirroring the cod/salmon tail-fin sway), with a test pinning the exact
      curve and that only the tail moves. The base texture is now bound on the textured path
      (`TADPOLE_TEXTURE_REF`; vanilla `TadpoleModel` constructs with
      `RenderTypes::entityCutout`) while preserving explicit submission metadata for texture, white
      tint, root transform, and `order(0)`. The pass now has an explicit `TadpoleBase` identity and
      records vanilla `ModelLayers.TADPOLE` (`minecraft:tadpole#main`) instead of an empty model-layer
      placeholder. Tests use non-default packed light plus hurt/white overlay to pin the vanilla
      `LivingEntityRenderer.submit` path: tadpole base submissions inherit `lightCoords` and
      `getOverlayCoords(...)` instead of using the object-renderer `OverlayTexture.NO_OVERLAY`
      override, and folded cutout vertices inherit that submission metadata. Nothing is left deferred on the
      texture side.
      The colored debug path stays as a fallback (it approximates the body with one dark tint and the
      tail fin with a lighter tint)
    - parrot entities as renderer-owned vanilla 26.1 `ParrotModel.createBodyLayer()` geometry on the
      textured path: the native entity scene (`entity_scene.rs`) projects vanilla type id `98` to
      `EntityModelKind::Parrot { variant }`, replacing the former placeholder bounds box. The static STANDING rest-pose
      hierarchy is emitted directly (atlas 32×32): seven sibling root parts — the 3×6×3 body (pitched
      0.4937 rad), the 3×4×1 tail (pitched 1.015 rad), the two 1×5×3 wings (pitched -0.6981 rad and flipped
      `yRot = -π`), the 2×3×2 head, and the two 1×2×1 legs (pitched -0.0299 rad) — with the head parenting
      the 2×1×4 upper-head block, the two 1×2×1 beak halves, and the 0×5×4 crest feather (pitched -0.2214
      rad) — eleven cubes. The SITTING perch pose is now projected: `Parrot.isInSittingPose()` (the synced
      `TamableAnimal.DATA_FLAGS_ID` bit 1, data id `18`, gated to the parrot type) runs `prepare(SITTING)` —
      every part raises `y += 1.9`, the legs fold `xRot += π/2`, the tail pitches `xRot += π/6`, and the wings
      tuck to `zRot = ±0.0873` (the `setupAnim` SITTING branch adds nothing more). The head look is now
      reproduced: `setupAnim` sets `head.xRot/yRot` from the projected `head_pitch/head_yaw` before the
      per-pose switch, so the top-level head part (and its beak/crest children) turn at projected normal poses
      (STANDING and SITTING); the PARTY branch now correctly overwrites head x/y look to zero. The STANDING walk swing is
      reproduced too: the legs add `xRot += cos(walkAnimationPos·0.6662 [+π])·1.4·walkAnimationSpeed` (left in
      phase, right out) and the tail adds `xRot += cos(walkAnimationPos·0.6662)·0.3·walkAnimationSpeed` onto
      their baked pitch, gated off the projected `walk_animation_pos/speed` and skipped while sitting (the
      vanilla SITTING branch breaks before it). The PARTY, wing flap, and FLYING poses are now projected too:
      `ParrotModel.getPose` is derived in the renderer from `parrot_party`, `parrot_sitting`, and the synced `on_ground` flag
      (PARTY while a playing jukebox is within `BlockPos.closerToCenterThan(entity.position(), 3.46)`, else
      SITTING when sitting, else FLYING when airborne since `isFlying() = !onGround()`, else STANDING). A
      `ParrotFlapAnimationState` client accumulator mirrors `Parrot.calculateFlapping` — per tick
      `flapSpeed += (!onGround() && !isPassenger() ? 4 : -1)·0.3` clamped `[0,1]`, `flapping` re-seeds to `1`
      whenever airborne then decays `·0.9`, and `flap += flapping·2` (`flapping` initializes to `1.0`); it is
      the chicken flap plus the `!isPassenger()` term, so a parrot riding a shoulder/mount settles its wings.
      `ParrotRenderer.extractRenderState` lerps `flap`/`flapSpeed` separately then combines them into the
      single projected `parrot_flap_angle = (sin(flap) + 1)·flapSpeed`, which `setupAnim` feeds (in the
      STANDING/FLYING fall-through) to the wings (`leftWing.zRot = -0.0873 - flapAngle`,
      `rightWing.zRot = 0.0873 + flapAngle`) and the body/head/tail/wing/leg bob (`y += flapAngle·0.3`); a
      grounded parrot has `flapSpeed → 0`, so the wings settle to `zRot = ±0.0873` and the bob vanishes.
      `prepare(FLYING)` additionally pitches both legs `xRot += 2π/9`, and FLYING skips the STANDING leg walk
      swing. PARTY mirrors vanilla `prepare(PARTY)` plus the switch branch: the legs splay to `zRot = ∓π/9`,
      head/body/wings/tail move by `cos(ageInTicks)` / `sin(ageInTicks)`, the head rolls by
      `sin(ageInTicks)·0.4`, and the wings still consume `parrot_flap_angle`; PARTY overrides both sitting and
      the normal head look. The ON_SHOULDER pose is now projected through the player shoulder layer:
      it copies the player age/walk/head-look state into a temporary parrot render state, skips both
      STANDING leg walk and FLYING leg pitch, but still runs the shared head-look, tail swing,
      bob/wing block with `parrot_flap_angle` (normally `0` for a shoulder rider). The five
      `Parrot.Variant` colors (red_blue / blue / green / yellow_blue / gray) are
      bound on the textured path: the native scene reads the synced `DATA_VARIANT_ID` (20, int — after
      `AgeableMob.AGE_LOCKED` at 17 and the two `TamableAnimal` accessors at 18/19) and `Parrot.Variant.byId`
      selects the per-colour texture (`parrot_red_blue` / `_blue` / `_green` / `_yellow_blue` / `parrot_grey.png`),
      matching `ParrotRenderer.getVariantTexture`. The five textured variant regressions now pin the
      explicit `ParrotBase` submission identity, vanilla `entityCutout` render type/name, white tint,
      vanilla `ModelLayers.PARROT` (`minecraft:parrot#main`), root transform,
      `(order, submit_sequence) == (0, 0)`, and the `MobRenderer` / `LivingEntityRenderer`
      `lightCoords` plus hurt/white overlay metadata, with folded cutout vertices inheriting that metadata. The
      PARTY regression additionally pins that the dance changes folded geometry while preserving that same
      texture/render-type/tint/root-transform/light/overlay/order submission metadata.
    - shulker entities as renderer-owned vanilla 26.1 `ShulkerModel.createBodyLayer()` geometry on the
      textured path: the native entity scene (`entity_scene.rs`) projects vanilla type id `112` to
      `EntityModelKind::Shulker { color }`, replacing the former placeholder bounds box. The hierarchy is emitted
      (atlas 64×64): three sibling root parts — the 16×12×16 lid and the 16×8×16 base (both at
      `offset(0, 24, 0)`), and the 6×6×6 head at `offset(0, 12, 0)` — three cubes. The lid peek open/close
      is now projected: `Shulker.getClientPeekAmount(partialTick)` (the synced `DATA_PEEK_ID` byte 17 fed
      through the existing client peek state machine, lerped per partial tick → `shulker_peek`) drives
      `ShulkerModel.setupAnim`'s `lid.y = 16 + sin((0.5 + peek)·π)·8` (plus the `sin(ageInTicks·0.1)·0.7`
      open-lid bob once past half-open) and the `lid.yRot = (−1 + sin(bs))⁴ · π · 0.125` twist above
      `peek > 0.3`; at `peek = 0` the lid sits back at its `y = 24` bind offset, so the closed pose equals
      the bind pose. The head look is now applied: `head.xRot = xRot`, `head.yRot = (yHeadRot − 180 −
      yBodyRot)`, which equals the projected `head_yaw − 180`. `ShulkerRenderState.attachFace` is now
      projected too: world reads `Shulker.DATA_ATTACH_FACE_ID` (16, DIRECTION; default `DOWN`), native
      forwards it to renderer state, and the renderer reproduces `ShulkerRenderer.setupRotations` by
      passing `bodyRot + 180` to the living setup (non-sleeping yaw becomes `−bodyRot`) and then applying
      `attachFace.getOpposite().getRotation()` around `(0, 0.5, 0)`. Wall and ceiling shulkers therefore
      use the vanilla root orientation instead of the old floor-only fallback. The sixteen dye-color
      variants are now bound on the textured path: the native scene reads `DATA_COLOR_ID` (18, byte) and
      `Shulker.getColor()` (0..=15 → the dye, the default byte 16 → `null`) selects the texture, matching
      `ShulkerRenderer.getTextureLocation` (the uncolored `shulker.png` plus the sixteen `shulker_<color>.png`)
      — seventeen textures. The textured regression now pins the vanilla `ShulkerModel`
      `entityCutoutZOffset` base submission's `ShulkerBase` identity, `ModelLayers.SHULKER`
      (`minecraft:shulker#main`), texture, white tint, root/attach-face transform, entity light,
      hurt/white overlay, and `order(0)`, including folded cutout vertex metadata;
      missing-atlas coverage pins that a red wall shulker still records the `entityCutoutZOffset`
      submit before folded cutout geometry is suppressed
    - wither entities as renderer-owned vanilla 26.1
      `WitherBossModel.createBodyLayer(CubeDeformation.NONE)` geometry on both the colored and textured
      paths: the native
      entity scene (`entity_scene.rs`) projects vanilla type id `145` to the new `EntityModelKind::Wither`,
      replacing the former placeholder bounds box. The static bind rest-pose hierarchy is emitted directly
      (atlas 64×64): six sibling root parts — the 20×3×3 shoulders bar, the ribcage (a 3×10×3 spine plus
      three 11×2×2 rib bars, at `offset(-2, 6.9, -0.5)` pitched 0.20420352 rad), the 3×6×3 hanging tail
      (at the bind position `(-2, 6.9 + cos(0.20420352)·10, -0.5 + sin(0.20420352)·10)` pitched 0.83252203
      rad), the 8×8×8 center head, and the two 6×6×6 side heads — nine cubes. The ribcage and tail breathe
      with the projected `ageInTicks` exactly as `WitherBossModel.setupAnim`: `anim = cos(ageInTicks · 0.1)`
      pitches the ribcage to `(0.065 + 0.05·anim)·π`, re-hangs the tail from that pitch (`tail.setPos(-2,
      6.9 + cos(ribcage.xRot)·10, -0.5 + sin(ribcage.xRot)·10)`) and pitches the tail to `(0.265 + 0.1·anim)·π`
      (the `anim = 0` rest equals the baked layer pose), via `wither_breathing_poses`, with tests pinning both
      the sampled poses against the vanilla arithmetic and that two ages re-pose only the ribcage and tail. The
      center head (part 3) follows the plain head look (`centerHead.yRot/xRot = state.yRot/xRot`), reproduced
      through the instance's `head_yaw` / `head_pitch` and the shared `head_look_pose`, with a test pinning that
      only the center-head vertices turn. The wither is rendered at the vanilla `WitherBossRenderer.scale`
      flat `2.0×` minus `invulnerableTicks / 220 * 0.5` during the spawn charge (`wither_model_root_transform`),
      and swaps to `wither_invulnerable.png` via the vanilla `getTextureLocation` flicker (solid above 80
      ticks, then alternating every 5 ticks) off the projected `WitherRenderState.invulnerableTicks`
      (`DATA_ID_INV`, lerped `invulnerableTicks - partialTicks`), with both the colored and textured paths
      wired; tests pin the selected normal/invulnerable texture, vanilla `entityCutout`, white tint,
      root transform, and `order(0)` submission metadata before checking folded geometry. The
      `WitherArmorLayer` powered energy-swirl overlay is now wired: when `isPowered`
      (vanilla `WitherBoss.isPowered() = getHealth() <= getMaxHealth() / 2`, projected from the synced
      `LivingEntity.DATA_HEALTH_ID` against the wither's `300` base max-health), the inflated `WITHER_ARMOR`
      model (`WitherBossModel.createBodyLayer(INNER_ARMOR_DEFORMATION)` = `CubeDeformation(0.5)`, driven by
      the same `setup_anim` so it breathes with the body) is drawn through the additive emissive
      `RenderTypes.energySwirl`: `wither_armor.png` (`WITHER_ARMOR_TEXTURE_REF`) tinted by the vanilla
      `0xFF808080` half-grey, its U scrolled by the oscillating `cos(ageInTicks · 0.02) · 3 % 1` (distinct
      from the creeper's linear scroll) and V by `ageInTicks · 0.01 % 1`, sharing the same per-fragment
      `fract` atlas-wrap scroll pipeline as the charged creeper. `wither_textured_layer_passes` now records
      vanilla `ModelLayers.WITHER` and `ModelLayers.WITHER_ARMOR`, with base `entityCutout` and armor
      `energySwirl` texture/render-type/tint/order metadata; dispatch consumes only the body pass, while
      the powered overlay helper consumes the armor pass and still gates it on `isPowered`. Its submission
      preserves per-entity light and vanilla `OverlayTexture.NO_OVERLAY`, and missing-atlas coverage now
      pins that the order-1 `energySwirl` submission survives when `wither_armor.png` is absent while only
      folded additive scroll geometry is suppressed. The remaining `WitherBossModel.setupAnim`
      side-head target tracking is now wired too: bbb-world reads `DATA_TARGET_B/C` (`17`/`18`), resolves
      tracked target eye positions, applies vanilla `WitherBoss.aiStep` `rotlerp` limits (`40°` pitch /
      `10°` yaw per tick, yaw-only fallback to `yBodyRot` when a target is missing), native forwards
      `WitherRenderState.xHeadRots/yHeadRots`, and the renderer applies
      `setupHeadRotation` (`yHeadRot - bodyRot`, `xHeadRot`) to the right/left side heads. Tests cover
      target ids, missing-target fallback, native forwarding, side-head pose, and the base wither
      submission texture/render type/tint/transform/order
    - giant entities as renderer-owned vanilla 26.1 `GiantZombieModel` geometry on the textured path: the
      native entity scene (`entity_scene.rs`) projects vanilla type id `59` to the new
      `EntityModelKind::Giant`, replacing the former placeholder bounds box. `GiantZombieModel` is the
      standard humanoid (zombie) body layer baked through `humanoidBodyLayer.apply(MeshTransformer.scaling(
      6.0))` (`LayerDefinitions` registers `ModelLayers.GIANT` this way; `EntityRenderers` registers the
      `GiantMobRenderer` with scale `6.0`), so the giant reuses the adult zombie body parts emitted through
      the shared `mesh_transformer_scaled_model_root_transform` at the 6.0 factor — exactly the husk's
      `MeshTransformer` pattern but with the giant's larger factor and no baby variant. The head look, the
      limb swing, and the held-out `animateZombieArms` resting arm pose match the zombie (`GiantZombieModel
      extends ZombieModel`, the giant extracts the same `ZombieRenderState`). The base texture is now bound
      on the textured path (the giant binds the zombie texture via the shared `zombie_textured_layer_passes`),
      the primary now-wired path, with explicit submission metadata for vanilla `entityCutout`, white tint,
      `mesh_transformer_scaled_model_root_transform(..., 6.0)`, zombie base layer key,
      `LivingEntityRenderer` light plus hurt/white overlay, and `(order, submit_sequence) == (0, 0)`;
      folded cutout vertices inherit the matching metadata. The vanilla extra layers are now wired too:
      `ItemInHandLayer` uses the renderer-owned hand attach transform from the same 6.0-scaled zombie
      model so native bakes main/off hands through the standard third-person item contexts, and
      `HumanoidArmorLayer` rebuilds the scaled host pose, draping standard adult humanoid armor as
      `armorCutoutNoCull` submissions through `humanoid_armor_layer_pass` at order `1`
      with vanilla slot sequences. Tests pin the giant armor texture/render type/tint/transform/order and
      the held-item hand basis scaling (the `attack_anim` melee swing is implemented via the shared
      zombie-family anim). The
      colored debug path stays as a fallback (the giant reuses the zombie body
      tints; the `Mob.isAggressive` arm-raise is implemented)
    - end crystal entities as renderer-owned vanilla 26.1 `EndCrystalModel.createBodyLayer()` geometry on
      the colored and textured paths: the native entity scene (`entity_scene.rs`) projects vanilla type id `45` to the new
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
      `EndCrystalModel.setupAnim` `base.visible = showsBottom` toggle IS reproduced: the bottom slab
      (`END_CRYSTAL_PARTS[0]`) is gated on the synced `EndCrystal.DATA_SHOW_BOTTOM` boolean (data id 9,
      default `true`), so a crystal with `ShowBottom = false` (e.g. the four end-spike crystals that heal the
      dragon) drops the slab while the glass/core stack stays, with native-projection + renderer tests pinning
      the face/vertex drop and the default-true. The `EndCrystalModel.setupAnim` glass motion is now
      reproduced off the projected `age_in_ticks`: the `outer_glass`/`inner_glass`/`cube` diagonal spin
      (the π/3 tilt about the `(sin45, 0, sin45)` axis composed with `Ry(age·3°)` — `outer_glass` as
      `Ry·TILT`, `inner_glass`/`cube` as `TILT·Ry`, hand-walked through the flattened glass stack so the
      inner shells inherit the outer rotation) and the `EndCrystalRenderer.getY` vertical bob
      (`getY(age)·16/2` lifting the whole glass stack). The textured path now binds
      `textures/entity/end_crystal/end_crystal.png` as the vanilla default `entityCutout` submit with
      vanilla render-type name coverage, collector order `0`, sequence `0`, white tint, vanilla light coords,
      `OverlayTexture.NO_OVERLAY`, and the same `scale(2)·translate(0,-0.5,0)` root
      transform. The body submit metadata now comes from `end_crystal_textured_layer_passes`
      with the vanilla `ModelLayers.END_CRYSTAL` key before the bespoke bob/spin hand-walk folds
      geometry, with folded cutout vertices inheriting that metadata; missing-atlas coverage pins that the
      submission is still recorded before folded cutout geometry is suppressed. The colored debug path stays as the
      missing-atlas fallback with separate glass/core/base tints. The `EndCrystal.DATA_BEAM_TARGET` custom
      beam is now wired too: world projects
      `EndCrystal.DATA_BEAM_TARGET` (Optional<BlockPos> data id 8) as
      `Vec3.atCenterOf(target) - entity.getPosition(partialTicks)`, native forwards it as
      `EndCrystalRenderState.beamOffset`, and the renderer consumes explicit `EndCrystalBeam` pass
      metadata before recording
      `RenderTypes.endCrystalBeam(textures/entity/end_crystal/end_crystal_beam.png)` at order `0`,
      sequence `1` with vanilla light coords and `OverlayTexture.NO_OVERLAY` before folding the
      eight-quad black/white prism into the tiled scroll mesh with matching vertex light/no-overlay
      metadata; missing-atlas coverage pins that the beam submission survives when
      `end_crystal_beam.png` is absent while only folded scroll geometry is suppressed.
    - evoker fangs entities as renderer-owned vanilla 26.1 `EvokerFangsModel.createBodyLayer()` geometry on
      the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `47` to the new
      `EntityModelKind::EvokerFangs`, replacing the former placeholder bounds box. The static closed-jaw
      rest-pose hierarchy is emitted directly (atlas 64×32): the 10×12×10 base block at `offset(-5, 24, -5)`
      parents the two jaws (a shared 4×14×8 box) at their bind rotations — `upper_jaw` at `offset(6.5, 0, 1)`
      with `zRot = 0.65π = 2.042035`, `lower_jaw` at `offset(3.5, 0, 9)` with `yRot = π` and `zRot = 1.35π =
      4.2411504` — three cubes. The bind rotations are exactly the `setupAnim` closed-jaw rest at
      `biteProgress = 0` (`upperJaw.zRot = π - 0.35π`, `lowerJaw.zRot = π + 0.35π`). The full
      `EvokerFangsModel.setupAnim` is now wired from a client-reconstructed `biteProgress`: entity event `4`
      (`EvokerFangs.handleEntityEvent`) sets `clientSideAttackStarted`, after which the `lifeTicks` countdown
      (initially `22`) drives `getAnimationProgress`'s `0..1` ramp; `setupAnim` turns it into the cubic
      ease-out jaw snap (`upper/lower_jaw.zRot = π ∓ biteAmount·0.35π`), the rise out of the ground
      (`base.y -= (biteProgress + sin(biteProgress·2.7))·7.2`), and the final vanish (`root.y = 24 -
      20·preScale`, `root` scale `→ 0` over the last 10%). The whole model is hidden while `biteProgress == 0`
      (vanilla `EvokerFangsRenderer` skips the render). The fang also now carries its vanilla
      `sized(0.5, 0.8)` pick bounds (type id `47`), so it is enumerated and rendered at all. `EvokerFangsRenderer`
      applies the standard flip and `-1.501` y-offset but a distinct `Ry(90 - yRot)` yaw (captured by
      `evoker_fangs_model_root_transform`). The base texture is bound on the textured path
      (`EVOKER_FANGS_TEXTURE_REF`), the primary path, with explicit submission metadata for vanilla
      `entityCutout`, white tint, light coords, `OverlayTexture.NO_OVERLAY`, the renderer root transform,
      and `(order, submit_sequence) == (0, 0)`, with folded cutout vertices inheriting that metadata;
      `evoker_fangs_textured_layer_passes` now records vanilla `ModelLayers.EVOKER_FANGS`
      (`minecraft:evoker_fangs#main`) instead of an empty model-layer placeholder, and tests pin that
      alongside the texture/render-type/tint/order fields.
      missing-atlas coverage pins that a nonzero-`biteProgress` submit is still recorded before folded
      cutout geometry is suppressed.
      The colored debug path stays as a fallback (it renders a grey base and lighter-bone jaws)
    - leash knot entities as renderer-owned vanilla 26.1 `LeashKnotModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `76` to the new
      `EntityModelKind::LeashKnot`, replacing the former placeholder bounds box. The mesh root holds a single
      `knot` part at ZERO with one 6×8×6 box — one cube. `LeashKnotModel` has no `setupAnim`, so the geometry
      is complete (nothing deferred on the geometry side). `LeashKnotRenderer` is a plain `EntityRenderer`
      that applies only the model flip (`scale(-1, -1, 1)`, no yaw / y-offset / scale), captured by
      `leash_knot_model_root_transform`. The base texture is now bound on the textured path
      (`LEASH_KNOT_TEXTURE_REF`), the primary now-wired path, with explicit submission metadata for vanilla
      `entityCutout`, white tint, the renderer root transform, entity light coords,
      `OverlayTexture.NO_OVERLAY`, and `(order, submit_sequence) == (0, 0)`, with folded cutout vertices
      inheriting that metadata. `leash_knot_textured_layer_passes` now records vanilla
      `ModelLayers.LEASH_KNOT` (`minecraft:leash_knot#main`) instead of an empty model-layer placeholder.
      The colored debug path stays as a fallback (it renders the knot with one brown tint)
    - arrow and spectral arrow entities as renderer-owned vanilla 26.1 `ArrowModel.createBodyLayer()`
      geometry on the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type ids `6`
      (arrow) and `123` (spectral arrow) to the new `EntityModelKind::Arrow { texture }` (they share one model,
      differing only in the bound image), replacing the former placeholder bounds box. The static
      rest-pose hierarchy is emitted directly (atlas 32×32): three sibling planes — the `back` arrowhead (a
      0×5×5 YZ plane at `offset(-11, 0, 0)`, pitched π/4, with `withScale(0.8)` baked into its cube → a 0×4×4
      box) and the two crossed fletching planes (`cross_1`/`cross_2`, each a 16×4×0 XY plane pitched π/4 and
      3π/4) — three cubes. The whole mesh is baked through `mesh.transformed(pose -> pose.scaled(0.9))`; that
      0.9 lives in `arrow_model_root_transform`. `ArrowModel.setupAnim` now applies the impact-shake `root.zRot`
      wobble (`-sin(shake·3)·shake` degrees) from world-projected `AbstractArrow.shakeTime`: bbb-world starts
      the seven-tick countdown on post-first-tick `IN_GROUND` metadata updates, decrements it each client tick,
      and projects `ArrowRenderState.shake = shakeTime - partialTick` through native into the renderer.
      `ArrowRenderer` is a plain `EntityRenderer` that orients the arrow along its flight with `Ry(yRot - 90)`
      then `Rz(xRot)` (no flip), projected through the instance's `body_rot` / `head_pitch`. All three arrow
      images are now bound on the textured path:
      `ArrowModelTexture::{Normal,Tipped,Spectral}` selects `arrow.png` / `arrow_tipped.png` / `arrow_spectral.png`
      via `arrow_texture_ref` — a tipped arrow (`TippableArrowRenderer`, `getColor() > 0` off the synced
      `ID_EFFECT_COLOR` 11) binds the tipped image, and the spectral-arrow type binds the spectral image.
      Tests now pin explicit submission metadata for vanilla `entityCutoutCull`, white tint,
      light coords, `OverlayTexture.NO_OVERLAY`, `arrow_model_root_transform`, and
      `(order, submit_sequence) == (0, 0)`, with folded cutout vertices inheriting that
      light/no-overlay metadata before checking the folded shake-posed mesh; the pass generator also records
      vanilla `ModelLayers.ARROW` (`minecraft:arrow#main`) for all three arrow textures; missing-atlas
      coverage pins that the spectral-arrow `entityCutoutCull` submit is still recorded before
      folded cutout geometry is suppressed.
      The colored debug path stays as a fallback (it renders the shaft cross and the head
      with two tints)
    - thrown trident entities as renderer-owned vanilla 26.1 `TridentModel.createLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `135` to the new
      `EntityModelKind::Trident`, replacing the former placeholder bounds box. The static hierarchy is
      emitted directly (atlas 32×32): the `pole` (a 1×25×1 shaft) parents the `base` crossguard (3×2×1) and
      the three spikes (left / middle / right, each 1×4×1), all at ZERO — five cubes. `TridentModel` is a
      `Model<Unit>` with no animation, so the geometry is complete. `ThrownTridentRenderer` is a plain
      `EntityRenderer` that orients the trident along its flight with `Ry(yRot - 90)` then `Rz(xRot + 90)`
      (the `+90` points the upright pole along the flight axis), projected through the instance's `body_rot`
      / `head_pitch` and captured by `trident_model_root_transform`. The base texture is now bound on the
      textured path (`TRIDENT_TEXTURE_REF`), the primary now-wired path. `trident_textured_layer_passes`
      now records vanilla `ModelLayers.TRIDENT` for both the order-0 base `entityCutout` submit and the
      synced `ID_FOIL` order-1 `entityGlint` submit with `textures/misc/enchanted_glint_item.png`, white
      tint, same flight transform, light coords, and `OverlayTexture.NO_OVERLAY`; GPU glint presentation
      remains deferred. The base submission explicitly records vanilla `order(0)`, `entityCutout`,
      white tint, texture, light coords, `OverlayTexture.NO_OVERLAY`, and the
      flight-orientation transform, with folded cutout vertices inheriting that base
      light/no-overlay metadata. Missing-atlas coverage now pins that a foiled trident still records
      both the order-0 base `entityCutout` submit and the order-1 `entityGlint` submit before folded
      base geometry is suppressed. The colored debug path stays as a fallback (it
      renders the pole/base in teal and the spikes lighter)
    - wither skull entities as renderer-owned vanilla 26.1 `WitherSkullRenderer.createSkullLayer()`
      (`SkullModel`) geometry on the colored path: the native entity scene (`entity_scene.rs`) projects
      vanilla type id `147` to the new `EntityModelKind::WitherSkull`, replacing the former placeholder
      bounds box. The static `head` part is emitted directly (atlas 64×64): one 8×8×8 box
      (`addBox(-4, -8, -4, 8, 8, 8)`) at ZERO — a single cube. `SkullModel.setupAnim` turns the head by
      the projectile's flight `yRot`/`xRot`; since the part sits at ZERO that facing folds into the root
      transform, together with the `WitherSkullRenderer` `scale(-1, -1, 1)` flip — `scale(-1, -1, 1) ·
      Ry(yRot) · Rx(xRot)`, projected through the instance's `body_rot` / `head_pitch` and captured by
      `wither_skull_model_root_transform` (a plain `EntityRenderer`, so no `-1.501` y-offset or render
      scale). The normal and dangerous skull textures are now bound on the textured path: native reads
      vanilla `WitherSkull.DATA_DANGEROUS` at synced data id `8` and projects
      `EntityModelKind::WitherSkull { dangerous }`; `false` selects `WITHER_TEXTURE_REF` (`wither.png`)
      and `true` selects `WITHER_INVULNERABLE_TEXTURE_REF` (`wither_invulnerable.png`). The colored debug
      path stays as a fallback (it renders the skull as one dark tint). Both textured variants now pin
      explicit `order(0)`, `entityTranslucent`, white tint, texture, light coords,
      `OverlayTexture.NO_OVERLAY`, `ModelLayers.WITHER_SKULL` (`minecraft:wither_skull#main`), and
      transform submission metadata, while the Wither boss base test
      proves the same shared `wither.png` / `wither_invulnerable.png` texture refs still preserve body
      overlay when submitted by `WitherBossRenderer`; missing-atlas coverage pins that the dangerous
      skull's `entityTranslucent` / `wither_invulnerable.png` submit is still recorded before folded
      translucent geometry is suppressed.
    - llama spit entities as renderer-owned vanilla 26.1 `LlamaSpitModel.createBodyLayer()` geometry on the
      colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `79` to the new
      `EntityModelKind::LlamaSpit`, replacing the former placeholder bounds box. The static `main` part is
      emitted directly (atlas 64×32): seven 2×2×2 boxes (all `texOffs(0, 0)`) forming a cross — a centre cube
      and one neighbour stepping out along each of ±X / ±Y / ±Z. `LlamaSpitModel` has no `setupAnim`, so the
      geometry is complete. `LlamaSpitRenderer` is a plain `EntityRenderer` that lifts the spit and orients it
      along its flight with `translate(0, 0.15, 0)` then `Ry(yRot - 90)` then `Rz(xRot)`, projected through
      the instance's `body_rot` / `head_pitch` and captured by `llama_spit_model_root_transform`. The base
      texture is now bound on the textured path (`LLAMA_SPIT_TEXTURE_REF`), the primary now-wired path, with
      explicit submission metadata for vanilla `entityCutout`, white tint, light coords,
      `OverlayTexture.NO_OVERLAY`, the renderer root transform, vanilla `ModelLayers.LLAMA_SPIT`
      (`minecraft:llama_spit#main`), and `(order, submit_sequence) == (0, 0)`.
      Folded cutout vertices now inherit that submission light and no-overlay metadata rather than the
      instance's hurt/white overlay coordinates; missing-atlas coverage pins that the submission is still
      recorded before folded cutout geometry is suppressed.
      The colored debug path stays as a fallback (it renders the cross
      with one tint)
    - shulker bullet entities as renderer-owned vanilla 26.1 `ShulkerBulletModel.createBodyLayer()` geometry
      on the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `113` to the
      new `EntityModelKind::ShulkerBullet`, replacing the former placeholder bounds box. The static `main`
      part is emitted directly (atlas 64×32): three interlocking slabs (`texOffs(0, 0)` 8×8×2,
      `texOffs(0, 10)` 2×8×8, `texOffs(20, 0)` 8×2×8). `ShulkerBulletModel.setupAnim` orients `main` by the
      bullet's yaw/pitch — reproduced through the instance's `body_rot` / `head_pitch` — and the
      `ShulkerBulletRenderer.submit` `translate(0, 0.15, 0)` + the `ageInTicks`-driven tumble
      (`Ry(sin(t·0.1)·180°) · Rx(cos(t·0.1)·180°) · Rz(sin(t·0.15)·360°)`) + `scale(-0.5, -0.5, 0.5)` are
      captured by `shulker_bullet_model_root_transform`. The textured path now reproduces both vanilla
      submits over `SHULKER_BULLET_TEXTURE_REF`: the base `entityCutout` submit at order `0`, then the same
      posed model multiplied by `scale(1.5)` as `entityTranslucent` at order `1` with packed color
      `0x26ffffff`. Submission metadata tests pin the texture, render types, alpha tint, transform, light,
      `OverlayTexture.NO_OVERLAY`, order, vanilla render-type names, and folded cutout/translucent vertex
      light/no-overlay inheritance; a missing-atlas regression pins that both submissions are still recorded
      before folded geometry is suppressed. The shared dispatch path now owns both base/shell submissions
      instead of a residual textured emit helper; the colored debug path stays as a fallback (it renders
      the three slabs with one tint)
    - wind charge and breeze wind charge entities as renderer-owned vanilla 26.1
      `WindChargeModel.createBodyLayer()` geometry through shared dispatch: the native entity scene
      (`entity_scene.rs`) projects vanilla type ids `143` (wind charge) and `18` (breeze wind charge) — both
      registered to `WindChargeRenderer` in vanilla — to the new `EntityModelKind::WindCharge`, replacing the
      former placeholder bounds boxes. The hierarchy is emitted directly (atlas 64×32): the `bone`
      root (no cubes) parents the `wind` shell (the `texOffs(15, 20)` 8×2×8 and `texOffs(0, 9)` 6×4×6 boxes,
      `yRot = -π/4` bind) and the `wind_charge` core (the `texOffs(0, 0)` 4×4×4 box). `WindChargeRenderer` is
      a plain `EntityRenderer` that applies no extra transform, captured by the position-only
      `wind_charge_model_root_transform`. The `WindChargeModel.setupAnim` counter-spin is reproduced off the
      projected `age_in_ticks`: `wind.yRot = age·16°` (a *set* that overwrites the -π/4 bind) and
      `windCharge.yRot = -age·16°`, so the two halves continuously counter-rotate. The whole model is now
      rendered through the scrolling `breezeWind` overlay (`WIND_CHARGE_TEXTURE_REF`) — vanilla
      `WindChargeRenderer` draws it with `RenderTypes.breezeWind(texture, xOffset(ageInTicks) % 1, 0)`, a
      texture-matrix `OffsetTextureTransform` (`xOffset(t) = t·0.03`) over a `GL_REPEAT` texture, translucent
      and `ALPHA_CUTOUT 0.1`. Because our textures share one atlas (no per-texture `REPEAT`), the textured
      sink now consumes the dispatch-owned `WindChargeModel` / `WIND_CHARGE` pass tuple and reproduces the
      scroll in the shader: the model is rendered once with the normal atlas UVs, then
      folded into a dedicated scroll mesh (`EntityModelScrollVertex`) whose per-vertex local UV carries the
      baked per-instance U offset and the texture's atlas sub-rect, and `ENTITY_MODEL_SCROLL_SHADER` does
      `atlas_uv = uv_rect_min + fract(local_uv)·uv_rect_size` (the per-fragment `fract` recreating the `REPEAT`
      seam) with the `0.1` alpha cutout, translucent-blended and depth-writing. The one simplification is
      lighting: vanilla `breezeWind` is lightmap-lit with `NO_CARDINAL_LIGHTING`, while the scroll shader is
      full-bright (a glowing projectile reads the same in practice). The wind charge records this through
      `wind_charge_textured_layer_passes` as a `breezeWind` submit with vanilla
      `ModelLayers.WIND_CHARGE`, texture ref, render-type name coverage, white tint, collector
      order `0`, and vanilla `OverlayTexture.NO_OVERLAY`, with folded
      scroll vertices now retaining that submission light/no-overlay metadata. Breeze's separate
      `BreezeWindLayer` is likewise recorded through `breeze_textured_layer_passes` with order `1`,
      vanilla `ModelLayers.BREEZE_WIND`, and the same no-overlay submit metadata, ahead of the
      same-order eyes layer per `BreezeRenderer.addLayer` order. The colored debug path stays as a fallback
      (it renders the spinning wind shell and core as opaque tinted geometry)
    - ender dragon entities as renderer-owned vanilla 26.1 `EnderDragonModel.createBodyLayer()` geometry on
      the colored path: the native entity scene (`entity_scene.rs`) projects vanilla type id `43` to the new
      `EntityModelKind::EnderDragon`, replacing the former placeholder bounds box. The straight bind layout
      is emitted directly (atlas 256×256): 19 root parts — the head (six cubes: the upper lip, the upper
      head, and the mirrored scale/nostril pairs) parenting the jaw; the five neck segments at
      `offset(0, 20, -12 - i·10)` and the twelve tail segments at `offset(0, 10, 60 + i·10)`, each the
      shared 10×10×10 vertebra plus its 2×4×6 dorsal scale; and the body (the 24×24×64 torso plus three
      dorsal scales) parenting the two wings (each a 56×8×8 bone, a 56×0×56 membrane plane, and a wing tip)
      and the four three-segment legs (leg → leg-tip → foot, with the vanilla bind rotations 1.3/-0.5/0.75
      front and 1.0/0.5/0.75 hind) — sixty-five cubes. The whole `EnderDragonModel.setupAnim` is procedural:
      every neck/tail segment is re-placed from the `DragonFlightHistory` path each frame, the wings flap
      (`flapTime`), the jaw opens, and the root gets the `bounce` y / fixed `z = -48` / `xRot` adjustments —
      all deferred (mirroring the guardian's deferred procedural tail), so the model renders at the straight
      bind layout. `EnderDragonRenderer` is a plain `EntityRenderer` that applies the flight-history yaw
      (`Ry(-yr)`), a flight-history pitch, a fixed `translate(0, 0, 1)`, and the standard flip / `-1.501`
      y-offset (captured by `ender_dragon_model_root_transform`, with the pitch and bounce deferred to
      identity at rest and the yaw projected through `body_rot`). The base texture is now bound on the
      textured path (`ENDER_DRAGON_TEXTURE_REF`), the primary now-wired path, together with the always-on
      emissive `dragon_eyes.png` eyes overlay (`ENDER_DRAGON_EYES_TEXTURE_REF`, an eyes-render-type pass
      re-rendering the whole model, matching vanilla `EnderDragonRenderer.EYES`). Tests pin both
      submissions' texture, render type, white tint, root transform, and same-order sequences
      `(0, 0)` / `(0, 1)`, with folded base/eyes vertices inheriting their respective
      light/overlay metadata; missing-atlas coverage pins that the eyes submission still records
      `dragon_eyes.png` even when the folded emissive geometry is suppressed. The nearest-crystal healing beam is
      now source-projected from the
      closest tracked end crystal intersecting the vanilla
      inflated search box, includes the `EndCrystalRenderer.getY` bob in `beamOffset`, and consumes
      explicit `EnderDragonBeam` pass metadata before recording
      `RenderTypes.endCrystalBeam(end_crystal_beam.png)` after body+eyes with preserved light and
      `OverlayTexture.NO_OVERLAY` before folding the shared prism geometry into the scroll mesh with
      matching vertex metadata; missing-atlas coverage pins that the beam
      submission survives when `end_crystal_beam.png` is absent while only
      folded scroll geometry is suppressed. The base body keeps the projected vanilla red overlay input; the eyes and
      healing beam record no-overlay submits. The dying-dissolve render type stays deferred. The colored
      debug path stays as a fallback (it renders the body dark and the wing membranes a lighter tint)
    - area effect cloud, marker, and interaction entities now resolve to `EntityModelKind::NoRender`,
      which emits no geometry — exact parity with vanilla, whose `EntityRenderers` registers all three to
      `NoopRenderer` (the area effect cloud is drawn as particles, not a model; the marker is a pure
      server-side data entity; the interaction is an invisible click hitbox). This replaces the former
      placeholder boxes, which incorrectly drew a debug box where vanilla draws nothing. These three are
      therefore deliberately unsupported *as models* (there is no vanilla model to render), and the native
      scene no longer projects placeholder bounds for them
    - phantom entities as renderer-owned vanilla 26.1
      `PhantomModel.createBodyLayer()` geometry: the nested body (parenting the tail
      chain, the two mirrored wing chains, and the head) on a 64x64 texture, with the
      vanilla `PhantomRenderer` transform overrides — the `scale(1 + 0.15 * size)`
      and `translate(0, 1.3125, 0.1875)` from the synced `ID_SIZE` (entity-data index
      16, defaulting to 0) and the extra `Axis.XP.rotationDegrees(state.xRot)` body
      pitch; the official `textures/entity/phantom/phantom.png` texture reference,
      texture-backed base layer pass emission (vanilla `PhantomModel` calls
      `EntityModel`'s default `RenderTypes::entityCutout`) while preserving explicit
      submission metadata for texture, white tint, renderer root transform,
      `order(0)`, light, and hurt/white overlay, official PNG atlas
      upload/bind/sample path, and the vanilla
      `PhantomModel.setupAnim` flap (`flapTime = id*3 + ageInTicks`; wings `zRot =
      ±cos(anim)·16°`, tail `xRot = -(5° + cos(2·anim)·5°)`, `anim = flapTime ·
      7.448451 · π/180`, on both render paths) plus the vanilla `PhantomEyesLayer` —
      an emissive `EyesLayer` re-rendering the whole model with
      `textures/entity/phantom/phantom_eyes.png` in the eyes render type at `order(1)`
      / `submit_sequence = 1`, with the same transform, entity light, white tint, and
      `OverlayTexture.NO_OVERLAY`; missing-atlas coverage proves the eyes submission is
      still recorded without `phantom_eyes.png` while only folded emissive geometry is
      suppressed. Broader lighting presentation remains unsupported
    - pufferfish entities as renderer-owned vanilla 26.1
      `PufferfishSmallModel`/`PufferfishMidModel`/`PufferfishBigModel.createBodyLayer()`
      geometry: the small (6-cube), medium (11-cube), and big (13-cube) body layers
      on a 32x32 texture, selected by the synced `PUFF_STATE` int (entity-data index
      17, defaulting to 0; `0` small, `1` medium, `>=2` big, matching
      `PufferfishRenderer.submit`), with the vanilla `PufferfishRenderer.setupRotations`
      vertical bob (`translate(0, cos(ageInTicks · 0.05) · 0.08, 0)`); the official
      `textures/entity/fish/pufferfish.png` texture reference, texture-backed cutout
      emission with explicit `entityCutout` submission metadata for
      `ModelLayers.PUFFERFISH_SMALL` / `PUFFERFISH_MEDIUM` / `PUFFERFISH_BIG`, texture, white
      tint, renderer root transform, `order(0)`, light, and hurt/white overlay,
      official PNG atlas upload/bind/sample path, and the shared vanilla
      `setupAnim` pectoral/blue fin wiggle (`right.zRot = -0.2 + 0.4 · sin(ageInTicks ·
      0.2)`, left negated, set absolutely over the rest pose, on both render paths).
      Broader lighting presentation remains unsupported
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
      (64x32) / baby (32x32) texture references, the shared-dispatch texture-backed
      base submission over the procedural ring, and the official PNG atlas
      upload/bind/sample path (colored and textured). Renderer tests now also pin the generated
      squid submissions' vanilla `entityCutout` render type, selected texture, white tint,
      root transform, `order(0)`, light, and hurt/white overlay metadata rather than only
      counting folded mesh vertices, including the tentacle-sweep textured regression path.
      The `GlowSquidRenderer.getBlockLightLevel`
      darken-ticks light boost IS now applied (`entity_light_coords` reads the synced
      `DATA_DARK_TICKS_REMAINING` int at index 18 and boosts the packed block light to
      `max(block, (int)clampedLerp(1 − darkTicks/10, 0, 15))` — full bright while undamaged, dimming for
      ~100 ticks after a hurt and ramping back over the final 10). Note 26.1 has NO separate glow-squid
      emissive overlay/texture; the glow is purely this block-light override. Broader lighting
      presentation remains unsupported. The `Squid.aiStep` swim integration IS now projected client-side
      (a `SquidAnimationState` accumulator in `entities/animations.rs`: `tentacleSpeed`
      seeded from the entity id via the Java `Random` LCG, the `tentacleMovement`
      half-cycle clamped at `2π` with the server event-`19` reset, `tentacleAngle =
      sin(s²·π)·π·0.25`, `rotateSpeed` evolving `1.0`/`·0.8`/`·0.99`, `zBodyRot +=
      π·rotateSpeed·1.5`, and `xBodyRot` easing toward `-atan2(horizontal, dm.y)` from
      the synced velocity; out of water, it switches to the vanilla suffocating
      branch (`tentacleAngle = abs(sin(tentacleMovement)) * π * 0.25`, `xBodyRot`
      easing toward `-90°`) — all lerped by partial tick and projected world →
      native → renderer). The movement-derived body yaw (`yBodyRot`) is also now
      projected: it is seeded from the add-entity head yaw like vanilla
      `LivingEntity.recreateFromPacket`, in water eases by
      `(-atan2(dm.x, dm.z) * 180 / π - yBodyRot) * 0.1`, out of water remains
      untouched, and native uses the lerped value as squid/glow-squid renderer
      `bodyRot` while preserving the canonical synced `yRot`
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
      base layer pass emission (vanilla `CodModel` calls `EntityModel`'s default
      `RenderTypes::entityCutout`; the top fin keeps its negative `texOffs(20, -6)` V
      origin) while preserving explicit submission metadata for `ModelLayers.COD`, texture, white tint,
      water/beached root transform, per-entity light/overlay coords, and `order(0)`, and the official PNG atlas
      upload/bind/sample path (colored and textured). Full vanilla lighting/gamma parity remains deferred
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
      keys, vanilla `SalmonModel` default `RenderTypes::entityCutout`, and the right fin
      keeping its negative `texOffs(-4, 0)` U origin) while preserving explicit
      submission metadata for texture, white tint, water/beached/size root transform,
      `order(0)`, light, and hurt/white overlay, and the official PNG atlas upload/bind/sample path (colored and
      textured). Broader lighting presentation remains unsupported
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
      texture references, shared-dispatch per-shape texture-backed base/pattern submission emission
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
      `DyeColor.byId((packedVariant >> 24) & 0xFF)`). Tests now pin the emitted base and pattern
      submissions' textures, vanilla `entityCutout` render type, base/pattern dye tints,
      `tropical_fish_model_root_transform`, light, overlay metadata (base keeps the full
      hurt/white overlay; `TropicalFishPatternLayer` preserves the red row but clears the white
      overlay via `getOverlayCoords(state, 0.0F)`), and explicit `(order, submit_sequence)` pairs
      `(0, 0)` and `(1, 1)` before checking the folded cutout mesh animation/flop behavior.
      Missing-atlas coverage proves small/large pattern submissions are still recorded without the
      selected `tropical_{a,b}_pattern_*.png` while only folded pattern geometry is suppressed. Only the colored
      debug path omits the
      pattern overlay (a cutout texture whose shape comes from the texture alpha cannot be
      approximated by a solid-color box); its lighting/overlay remain the standard deferred
      entity lighting
    - nautilus and zombie-nautilus entities as renderer-owned vanilla 26.1
      `NautilusModel.createBodyMesh()` / `createBabyBodyLayer()` geometry on the colored path: the native
      entity scene (`entity_scene.rs`) now splits vanilla type ids `88` (nautilus) and the zombie nautilus
      out of the horse-shaped quadruped proxy — the living nautilus (adult and baby) maps to the new
      `EntityModelKind::Nautilus { baby }`, and the zombie nautilus reuses the same adult body
      (`ModelLayers.ZOMBIE_NAUTILUS` bakes to `NautilusModel.createBodyLayer()`, a plain `MobRenderer` so
      never a baby) → the dedicated `EntityModelKind::ZombieNautilus { coral }`, replacing the horse-shaped
      stand-in with the real mesh of this new rideable mob. Both `ZombieNautilusVariant`s are now rendered
      (`ZombieNautilusRenderer.getTextureLocation` / `submit`, resolved from the synced
      `DATA_VARIANT_ID` holder at index 21 by the bootstrap order — id ≥ 1 → `WARM`): the
      `NORMAL`/`TEMPERATE` default textures the shared adult body with
      `textures/entity/nautilus/zombie_nautilus.png` (fixing the earlier wrong-skin, where it drew the
      living `nautilus.png`), and the `WARM` variant renders the `ZombieNautilusCoralModel` — the same
      adult body plus the `corals` subtree (four clusters of textured-only cross-planes, eight cubes
      under `shell`) — over `textures/entity/nautilus/zombie_nautilus_coral.png`. The adult living
      nautilus and zombie nautilus saddle/body-armor equipment layers are now wired. A non-empty
      `EquipmentSlot.SADDLE` item resolving to `ItemEquipmentSlot::Saddle` renders
      `NautilusSaddleModel(ModelLayers.NAUTILUS_SADDLE)` over
      `textures/entity/equipment/nautilus_saddle/saddle.png`; a non-empty `EquipmentSlot.BODY`
      nautilus armor item whose default equipment asset resolves to copper/iron/gold/diamond/netherite
      renders `NautilusArmorModel(ModelLayers.NAUTILUS_ARMOR)` over the matching
      `textures/entity/equipment/nautilus_body/<asset>.png` (all 128×128). Baby living nautilus skip
      both layers because vanilla passes no baby models. The `corals.visible = bodyArmorItem.isEmpty()`
      gate is reproduced, so the warm zombie nautilus coral cluster hides while body armor is present.
      Missing-atlas coverage pins that the adult living nautilus saddle and body-armor submissions
      are still recorded without `nautilus_saddle/saddle.png` or `nautilus_body/iron.png`
      while only the corresponding folded equipment geometry is suppressed.
      The dynamic-registry reorder path stays deferred. The adult rest-pose
      hierarchy is emitted directly (atlas 128×128): one cubeless `root` pivot at
      `offset(0, 29, -6)` parenting the `shell` at `offset(0, -13, 5)` (the 14×10×16 dome, the 14×8×20
      whorl, and a 14×8×0 rear fin plane) and the `body` at `offset(0, -8.5, 12.3)` (the 10×8×14 trunk
      plus its 10×8×0 fin plane), the body parenting the three mouth boxes (`upper_mouth` / `lower_mouth`
      deflated by the vanilla `CubeDeformation(-0.001)`, `inner_mouth` undeformed) — eight cubes. The baby
      (atlas 64×64) is the same `root → shell + body → three mouths` structure in hatchling proportions
      (the shell shrinks to 7×4×7 / 7×4×9, the body to 5×4×7), also eight cubes. The
      `NautilusModel.setupAnim` body look is reproduced on both: `applyBodyRotation` sets `body.yRot/xRot`
      from the projected `head_yaw/head_pitch` clamped to ±10°, turning the body and its mouths (the shell
      holds). The looping `NautilusAnimation.SWIMMING` keyframe undulation (always on, applied via
      `applyWalk` with the idle baseline `walkAnimationSpeed + 0.2`) needs the keyframe machinery plus an
      `AnimationState`, so it stays deferred, as does the `AgeableMobRenderer` baby render scale (`0.7`).
      The base texture is now bound on the textured path (`NAUTILUS_TEXTURE_REF` /
      `NAUTILUS_BABY_TEXTURE_REF`), the primary now-wired path; the zombie coral variant plus adult
      saddle/body-armor overlays are wired. Textured nautilus/zombie-nautilus regressions now route
      through `entity_model_textured_meshes` and pin vanilla submission metadata before folded cutout
      geometry checks: living/baby/zombie/coral base passes are explicit `EntityModelLayerPass`
      metadata with vanilla `ModelLayers.NAUTILUS` / `NAUTILUS_BABY` / `ZOMBIE_NAUTILUS` /
      `ZOMBIE_NAUTILUS_CORAL`, `entityCutout`, selected texture, white tint,
      `entity_model_root_transform`, light, hurt/white overlay, and `(order, submit_sequence) == (0, 0)`;
      adult living nautilus and
      all zombie-nautilus saddle/body-armor equipment layers are `equipment_layer_pass`-generated
      `armorCutoutNoCull` submissions for vanilla `ModelLayers.NAUTILUS_SADDLE` / `NAUTILUS_ARMOR`
      with white tint and the same transform, entity light, and `OverlayTexture.NO_OVERLAY` at `(0, 1)`,
      with saddle advancing to `(0, 2)` when a valid body-armor layer is also present. Baby living nautilus remain base-only
      because vanilla `SimpleEquipmentLayer` has no
      baby model. The colored debug path stays as a fallback (it renders a tan shell over a pale body)
    - fox entities (adult and baby) as renderer-owned vanilla 26.1 `AdultFoxModel.createBodyLayer()` /
      `BabyFoxModel.createBodyLayer()` geometry on the textured path: the native entity scene
      (`entity_scene.rs`) now splits vanilla type id `54` out of the cat/ocelot/fox wolf-shaped quadruped
      proxy — both map to `EntityModelKind::Fox { baby, variant }` (`baby` selecting the layout), replacing the
      wolf-shaped stand-in with the real fox mesh. The adult static rest-pose hierarchy is emitted directly
      (atlas 48×32): six root parts — the `head` at `offset(-1, 16.5, -3)` (the 8×6×6 skull, the two 2×2×1
      ears, and the 4×2×3 snout, the ears/snout at the head origin), the `body` at `offset(0, 16, -6)`
      pitched `π/2` (the 6×11×6 trunk) parenting the `tail` (the 4×9×5 brush pitched back `-0.05235988`),
      and the four legs at `offset(±{5,1}, 17.5, {7,0})` (each the shared 2×6×2 box inflated by the vanilla
      `CubeDeformation(0.001)` fudge and built off-center at `+2` X) — ten cubes. The baby uses the flatter
      `BabyFoxModel` layout (atlas 32×32): the head bakes the ears/snout as cubes (no child parts), the
      body has no pitch, and the root child order is head / four legs / body — also ten cubes. The full
      `FoxModel.setupAnim` (with its `AdultFoxModel` overrides) is now mirrored end to end on both render
      paths off the synced `Fox.DATA_FLAGS_ID` (data id `19`) and the two eased client accumulators (the
      bee-roll cross-crate pattern): the always-run `setWalkingPose` tilts the head by the projected
      `fox_head_roll_angle` (`Fox.getHeadRollAngle`, the lerped `interestedAngle` accumulator easing
      toward the synced `FLAG_INTERESTED` bit `8` by `* 0.4`/tick, scaled `0.11 · π`), keeps all four legs
      visible, and sweeps the adult gait (`cos·1.4·speed` keyed left/right by leg NAME — the fox builds all
      four legs at negative pivot X, so it cannot reuse the `QuadrupedModel` `x·z` helper, so the
      back-right / front-left diagonal swings in phase and the other half a cycle out, consuming the
      projected `walk_animation_pos/speed`). Then exactly one of `setCrouchingPose` (the synced
      `FLAG_CROUCHING` bit `4`: body pitch + `head.y += crouchAmount · ageScale` + adult `body.y +=
      crouchAmount` / baby `+ crouchAmount/6` + the `cos(ageInTicks)·0.05` body/leg wiggle, the
      `crouchAmount` accumulator climbing `0.2`/tick to `5.0` and resetting instantly to `0`),
      `setSleepingPose` (the synced `FLAG_SLEEPING` bit `32`: HIDES all four legs via `ModelPart::visible`,
      mirroring the bee stinger, and folds the body onto its side), or `setSittingPose` (the synced
      `FLAG_SITTING` bit `1`: folds the body down and the legs back); then the adult `setPouncingPose`
      `body/head.y -= crouchAmount/2` drop (the synced `FLAG_POUNCING` bit `16`); the resting head look
      (suppressed while sleeping / faceplanted / crouching); the sleeping head wobble
      (`head.zRot = cos(ageInTicks·0.027)/22`); and the faceplant leg twitch (the synced `FLAG_FACEPLANTED`
      bit `64`: `cos(ageInTicks·0.67·0.4662 [+π])·0.1`, the diagonals out of phase). `ageScale` is the
      standard `0.5` baby / `1.0` adult (`LivingEntity.getAgeScale`; `Fox.BABY_SCALE = 0.6` only scales the
      bounding box, not the model). The baby `FoxBabyAnimation.FOX_BABY_WALK` keyframe gait is now
      reproduced (`BabyFoxModel.setWalkingPose` → `applyWalk(walkPos, walkSpeed, 1.0, 2.5)`): the 0.5s
      looping `FOX_BABY_WALK` (7 bones) trots the four legs ±35° in the diagonal pairing, holds each leg
      forward/up (POSITION) and stretched 1.15× on y (SCALE), lifts the head, and cocks the tail -2.5°,
      driven off the projected `walk_animation_pos/speed` via `keyframe_walk_sample` (zero amplitude at
      rest). Renderer tests pin the def (0.5s looping, 7 bones, the leg kick/stretch, the head lift) and
      that a walking baby scampers (head included, unlike the adult swing) while a standing one holds
      bind. `FoxRenderer.setupRotations`'s pounce / faceplant body-pitch branch is now reproduced:
      after the standard living-entity setup rotation and before the model flip, `fox_model_root_transform`
      applies `Rx(-state.xRot)` while `isPouncing || isFaceplanted`, and the fox held-item transform uses
      the same root so carried items pitch with the body. The textured path now binds the full `FoxRenderer.getTextureLocation` matrix: the
      native scene reads `DATA_TYPE_ID` (18, int) and `Fox.Variant.byId` selects red/snow, crossed
      with the age (`fox`/`fox_baby`) and the projected `fox_is_sleeping` flag (`fox_sleep`/
      `fox_snow_sleep` and their `_baby` cells) — eight textures total. Adult and baby textured regressions
      now pin the `FoxBase` pass identity, vanilla `entityCutout` render type/name, vanilla
      `ModelLayers.FOX` / `FOX_BABY` (`minecraft:fox#main` / `minecraft:fox_baby#main`), white tint, root
      transform including the pounce / faceplant root-pitch branch, `(order, submit_sequence) == (0, 0)`,
      and the `AgeableMobRenderer` / `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata.
      The held-item layer is now
      implemented through the shared item-model pass.
    - cat and ocelot entities (adult and baby) as renderer-owned vanilla 26.1
      `AdultFelineModel.createBodyMesh()` / `BabyFelineModel.createBodyMesh()` geometry on the colored
      path: the native entity scene (`entity_scene.rs`) now splits vanilla type ids `21` (cat) and `91`
      (ocelot) out of the cat/ocelot/fox wolf-shaped quadruped proxy — both adults and both babies map to
      the new `EntityModelKind::Feline { cat, baby, cat_variant }` (the `cat` flag selecting the adult cat
      layer's 0.8 `MeshTransformer.scaling` via the root transform; the ocelot and both babies are unscaled),
      replacing the wolf-shaped stand-in with the real feline mesh (the fox keeps the wolf proxy —
      `FoxModel` is a distinct non-feline mesh). The adult rest-pose hierarchy is emitted directly (atlas
      64×32, `CubeDeformation.NONE`): eight root parts — the `head` at `offset(0, 15, -9)` (the 5×4×5
      skull, the 3×2×2 nose, and the two 1×1×2 ears), the `body` at `offset(0, 12, -10)` pitched `π/2`
      (the 4×16×6 trunk), the two tail segments (`tail1` pitched `0.9`, `tail2` deflated by the vanilla
      `CubeDeformation(-0.02)`), and the four legs (hind 2×6×2, front 2×10×2) — eleven cubes. The baby
      (atlas 32×32) is a flatter, all-upright layout — eight root parts: the `head` at
      `offset(0, 20, -3.125)` (a 5×4×4 skull, two 1×1×2 ears, a 3×2×1 nose), three legs, the upright
      4×3×7 `body`, the fourth leg, the single `tail1` segment pitched `-0.567232`, and a cubeless `tail2`
      pivot — ten cubes; the baby cat and baby ocelot share it unscaled. The shared `setupAnim` head look
      (`head.xRot/yRot` set from the projected `head_yaw/head_pitch`, turning only the head) is reproduced
      on both, as is the adult's not-sitting standing tail droop (`tail2.xRot = 1.7278761`, a real change
      from the bind that the deferred walk wobble would add onto); the baby's identical `tail2` assignment
      is a no-op (its lower tail is cubeless), so the baby's only reproduced standing pose is the head
      look. The bespoke feline walk leg swing is now reproduced too (each leg `xRot = cos(pos·0.6662
      [+π])·1.0·speed` at the shorter `1.0` amplitude, keyed by leg NAME to the MIRROR of the
      `QuadrupedModel` diagonal — left-hind/right-front in phase — consuming the projected
      `walk_animation_pos/speed`, on both adult and baby). The rest stays deferred: the `tail2` walk wobble
      that adds onto the droop, plus the `isCrouching` / `isSprinting` / `isSitting` / `lieDownAmount` / `relaxStateOneAmount` poses,
      all reading un-projected `FelineRenderState` fields, as does the `AgeableMobRenderer` `0.4` baby
      render scale. The textured path is now wired: `Cat.DATA_VARIANT_ID` (20, `Holder<CatVariant>`) is
      projected — via the registry-holder mapping shared with chicken/cow/pig/frog — onto one of the eleven
      vanilla breeds (tabby/black/red/siamese/british_shorthair/calico/persian/ragdoll/white/jellie/all_black),
      falling back to the bootstrap order (tabby=0..all_black=10, default BLACK) before the dynamic
      `cat_variant` registry arrives; the ocelot keeps its single breed-less texture. The tame cat's dyed
      collar (`CatCollarLayer`) is wired too: a second cutout pass binds `cat_collar.png` / `cat_collar_baby.png`
      tinted by the dye's diffuse color, projected onto `collar` mirroring `CatRenderer`
      (`isTame() ? getCollarColor() : null`) off the `TamableAnimal` tame flag and `Cat.DATA_COLLAR_COLOR`
      (23, `DyeColor.byId`, default RED); the ocelot never carries one. This makes the feline texture set a
      26-entry matrix (eleven breeds × adult/baby + ocelot × adult/baby + the collar × adult/baby).
      Textured regressions now pin the `FelineBase` and `FelineCollar` pass identities, vanilla
      `entityCutout` render type/name, vanilla base model layers (`CAT`, `CAT_BABY`, `OCELOT`,
      `OCELOT_BABY`) and collar layers (`CAT_COLLAR`, `CAT_BABY_COLLAR`), base white tint, collar dye tint,
      adult-cat scale transform,
      `(order, submit_sequence) == (0, 0)` for base and `(1, 1)` for `CatCollarLayer`, and
      base `AgeableMobRenderer` / `LivingEntityRenderer` light plus hurt/white overlay versus
      `CatCollarLayer` entity light with zero-white overlay via
      `getOverlayCoords(state, 0.0F)`, including folded cutout vertex metadata. Missing-atlas coverage
      proves adult/baby collar submissions are still recorded without `cat_collar*.png` while only
      folded collar geometry is suppressed. Nothing on the cat base/collar path stays deferred
      (only the feline pose animations above remain)
    - mooshroom entities (adult and baby) as renderer-owned vanilla 26.1 cow-body geometry on the colored
      path: the native entity scene (`entity_scene.rs`) now maps vanilla type id `86` (adult and baby) to
      the new `EntityModelKind::Mooshroom` (`baby` selecting the layout), replacing the generic six-cube
      quadruped stand-in with the real cow body. This was the last entity still on the generic
      `EntityModelKind::Quadruped` proxy, so the native entity scene no longer emits it at all — every
      vanilla entity type now resolves to a dedicated vanilla model (the renderer keeps the generic
      quadruped path for its own tests). Vanilla `MushroomCowRenderer` renders the mooshroom with the
      shared `CowModel` /
      `BabyCowModel` mesh (`ModelLayers.MOOSHROOM` bakes to the same temperate `cowBodyLayer` as
      `ModelLayers.COW`, `MOOSHROOM_BABY` to `BabyCowModel.createBodyLayer()`), so the mooshroom reuses
      the dedicated temperate-cow geometry directly — ten cubes (the head with horns / muzzle, the pitched
      body, four legs) for the adult, the smaller `BabyCowModel` layout for the baby, with the shared
      `QuadrupedModel` head look and leg swing already reproduced by the cow path. The full
      `MushroomCowRenderer.getTextureLocation` red/brown texture swap is now implemented:
      `EntityModelKind::Mooshroom` carries a `variant` projected from the synced `MushroomCow.DATA_TYPE`
      (index 20 INT, `ByIdMap` CLAMP, `Red` default) which binds `mooshroom_{red,brown}[_baby].png` on
      the textured path (the two brown faces join the master atlas array → 361); tests now pin all
      red/brown adult/baby body submissions with the `MooshroomBase` pass identity, vanilla `entityCutout`
      render type/name, vanilla `ModelLayers.MOOSHROOM` / `MOOSHROOM_BABY`
      (`minecraft:mooshroom#main` / `minecraft:mooshroom_baby#main`), white tint, the shared living root
      transform, `(order, submit_sequence) == (0, 0)`,
      and `AgeableMobRenderer` / `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata before
      comparing the folded cow geometry.
      The adult-only
      mushroom block-model layer (`MushroomCowMushroomLayer`) is now implemented through the
      entity-attached block-model path: red mooshrooms resolve `Blocks.RED_MUSHROOM.defaultBlockState()`,
      brown mooshrooms resolve `Blocks.BROWN_MUSHROOM.defaultBlockState()`, the two back mushrooms use
      vanilla's hardcoded entity-frame transforms, and the head mushroom follows the posed cow head bone.
      The invisible-glowing `submitOnlyOutline` variant now records outline-only attachment metadata
      and suppresses ordinary block-quad baking while GPU outline presentation remains deferred.
      The colored debug path stays as a fallback (it shows the cow-brown body tint)
    - panda entities (adult and baby) as renderer-owned vanilla 26.1 `PandaModel.createBodyLayer()` /
      `BabyPandaModel.createBodyLayer()` geometry on the colored path: the native entity scene
      (`entity_scene.rs`) now splits vanilla type id `96` out of the mooshroom/panda cow-shaped quadruped
      proxy — both map to the new `EntityModelKind::Panda` (`baby` selecting the layout), replacing the
      cow-shaped stand-in (a completely wrong silhouette) with the real panda mesh. The adult static
      rest-pose hierarchy is emitted directly (atlas 64×64) in the `QuadrupedModel` six-part layout: the
      `head` at `offset(0, 11.5, -17)` (the 13×10×9 skull, the 7×5×2 muzzle, and the two 5×4×1 ears), the
      `body` at `offset(0, 10, 0)` pitched `π/2` (the 19×26×13 trunk), and the four legs at
      `offset(±5.5, 15, ±9)` (each the shared 6×9×6 box) — nine cubes. The baby uses the `BabyPandaModel`
      layout (the `QuadrupedModel` baby convention lists the body first then the head, and the baby body
      carries no pitch) — also nine cubes. Because
      `PandaModel extends QuadrupedModel`, the shared base `setupAnim` is reproduced: the head turns by the
      projected `head_yaw/head_pitch` (`head.xRot/yRot` set from the look) and the four legs swing off the
      projected `walk_animation_pos/speed` (`leg.xRot = cos(pos·0.6662 [+π])·1.4·speed`, the diagonal pair
      in phase), via the same `apply_head_look` / `apply_quadruped_leg_swing` helpers the cow / pig / sheep /
      polar-bear paths use. The `isUnhappy` head shake + front-leg paddle and the `isSneezing` head dip
      are projected and applied (`apply_panda_emotes`): `isUnhappy = Panda.getUnhappyCounter() > 0` (the
      synced `UNHAPPY_COUNTER` int id 18) sets `head.yRot = head.zRot = 0.35·sin(0.6·ageInTicks)`
      (overwriting the look yaw) and `frontLeg.xRot = ∓0.75·sin(0.3·ageInTicks)` (overwriting the walk
      swing); `isSneezing = Panda.isSneezing()` (the `DATA_ID_FLAGS` byte id 23 bit `0x02`) dips
      `head.xRot` to `-π/4` over the `sneezeTime` ramp (`SNEEZE_COUNTER` int id 19, ticks 0..14, then
      holds — vanilla's `(sneezeTime-15)/5` integer division makes the 15..19 ease-back term 0). The
      remaining panda-specific client-tick poses are now covered: bbb-world mirrors vanilla
      `Panda.tick` for `sitAmount/onBackAmount/rollAmount` (`+0.15` while the synced flag is active,
      `-0.19` while inactive) and `rollCounter`; native forwards `PandaRenderState.sitAmount`,
      `lieOnBackAmount`, adult-only `rollAmount`, and `rollTime`; renderer applies
      `PandaRenderer.setupRotations`'s whole-body roll tumble / sitting tilt / lie-on-back tilt plus
      `PandaModel.setupAnim`'s adult and baby sitting folds, eating/scared sitting overrides,
      lie-on-back limb/head pose, and adult `rollAmount` somersault limb/head pose. Baby pandas keep
      vanilla's split behavior: `rollAmount` is forced to `0.0` but `rollTime` still tumbles the whole
      baby model. The
      textured path IS wired with the seven `Panda.Gene`
      variants: the displayed gene is `Panda.Gene.getVariantFromGenes(mainGene, hiddenGene)` off the two
      synced gene bytes (`MAIN_GENE_ID` 21 / `HIDDEN_GENE_ID` 22) — a dominant main gene always shows, a
      recessive main gene (`BROWN`/`WEAK`) shows only when both genes match, else `NORMAL` — and
      `PandaRenderer.getTextureLocation` keys the 14-entry texture matrix (seven genes × adult/baby, with
      the inconsistent vanilla baby filenames `panda_baby.png` / `lazy_panda_baby.png` / … preserved) off
      it, bumping the master `ENTITY_MODEL_TEXTURE_REFS` array to 272. Adult and baby textured regressions
      now pin the `PandaBase` pass identity, vanilla `entityCutout` render type/name, white tint, root
      transform, vanilla `ModelLayers.PANDA` / `PANDA_BABY`
      (`minecraft:panda#main` / `minecraft:panda_baby#main`), `(order, submit_sequence) == (0, 0)`, and the `AgeableMobRenderer` /
      `LivingEntityRenderer` `lightCoords` plus hurt/white overlay metadata. Nothing on the panda base path stays
      deferred. `PandaHoldsItemLayer` is also covered:
      native projects `PandaRenderState.isEating` from synced `EAT_COUNTER` int id 20, `isSitting` from
      `DATA_ID_FLAGS` byte id 23 bit `0x08`, and `isScared = isWorried() && level.isThundering()` (displayed
      `WORRIED` gene plus vanilla `rain_level * thunder_level > 0.9` weather-capable-level gate). The
      renderer exports the entity-root transform for the main-hand stack resolved in `ItemDisplayContext.GROUND`:
      render only while sitting and not scared, translate to `(0.1, 1.4, -0.6)`, and while eating apply the
      vanilla bob (`z -= 0.2*sin(age*0.6)+0.2`, `y -= 0.09*sin(age*0.6)`) before bbb-native bakes the item
      through the shared block/flat item-model pass.
    - rabbit entities (adult and baby) as renderer-owned vanilla 26.1 `AdultRabbitModel.createBodyLayer()`
      / `BabyRabbitModel.createBodyLayer()` geometry on the textured path: the native entity scene
      (`entity_scene.rs`) now splits vanilla type id `108` out of the cat/ocelot/fox wolf-shaped quadruped
      proxy — both map to `EntityModelKind::Rabbit { baby, variant, toast }` (`baby` selecting the body layout),
      replacing the wolf-shaped stand-in with the real rabbit mesh. The adult static rest-pose hierarchy is emitted
      directly (atlas 64×64): two root parts — the `body` (an 8×6×10 torso pitched `-0.3927` at
      `offset(0, 23, 4)`) parenting the 4×4×4 `tail`, the 5×5×5 `head` (pitched `0.3927`, parenting the two
      2×5×1 ears) and the cubeless `frontlegs` pivot (parenting the two 2×4×2 front legs, both pitched
      `0.3927`); and the cubeless `backlegs` pivot (at `offset(0, 23, 4)`, parenting the two cubeless
      hind-leg pivots, each parenting a 2×1×6 `haunch` yawed `±0.3927`) — nine cubes. The baby uses the
      deeper `BabyRabbitModel` layout (atlas 32×32) where every cube hangs off an `_r1` rotation
      intermediate and the head is `body`'s third child — also nine cubes. The head look is reproduced:
      `RabbitModel.setupAnim` sets `head.yRot/xRot` from the projected `head_yaw/head_pitch` (an assignment
      that overwrites the head's baked pitch, gated on the idle-head-tilt `AnimationState` that bbb never
      starts, so the look applies every frame), turning only the head and its two ears, then applies the
      looping `RabbitAnimation.HOP` (0.75s, 11 bones, 110 keyframes) additively over every bone while the
      rabbit is mid-jump. The hop is reconstructed client-side: entity event `1` (`Rabbit.handleEntityEvent`)
      seeds `jumpDuration = 15; jumpTicks = 0`, then each client tick mirrors vanilla's order —
      `setupAnimationStates` `startIfStopped`/`stop`s the hop on `jumpTicks > 0` (a reused
      `KeyframeAnimationState`) BEFORE `aiStep` lifts `jumpTicks` toward `jumpDuration` and wraps it back to
      `0` — so the hop runs for exactly one 15-tick window (= one loop) per jump. Projected as
      `rabbit_hop_seconds` (`-1.0` when stopped). The two ears were renamed from the positional `"0"`/`"1"`
      to the vanilla `right_ear`/`left_ear` (right-then-left, vertex order preserved) in both the adult and
      baby trees so the per-ear HOP channels apply to both. World tests pin the seed→1-tick-delay→15-tick
      window→stop sequence; renderer tests pin the HOP def (0.75s looping, 11 bones) and that a hopping
      rabbit (adult and baby) re-poses off bind and swings the hind legs. DEFERRED: the random-timed
      `IDLE_HEAD_TILT` keyframe (`shouldPlayIdleAnimation` gated on a `random.nextInt(40) + 180` timeout) is
      not reconstructable client-side. The seven `Rabbit.Variant` color/texture variants are now bound on the textured path:
      the native scene reads `DATA_TYPE_ID` (18, int) and `Rabbit.Variant.byId` (sparse; EVIL = 99 → the
      `caerbannog` texture) selects the colour, crossed with the age and the `Toast` custom-name override
      (`checkMagicName(entity, "Toast")` → `rabbit_toast`/`_baby`), matching
      `RabbitRenderer.getTextureLocation` — sixteen textures. Adult and baby textured regressions now pin
      the `RabbitBase` pass identity, vanilla `entityCutout` render type/name, vanilla
      `ModelLayers.RABBIT` / `RABBIT_BABY` (`minecraft:rabbit#main` / `minecraft:rabbit_baby#main`), white tint, root transform,
      `(order, submit_sequence) == (0, 0)`, and the `AgeableMobRenderer` / `LivingEntityRenderer`
      `lightCoords` plus hurt/white overlay metadata
    - minecart entities as renderer-owned vanilla 26.1
      `MinecartModel.createBodyLayer()` geometry: the `texOffs(0, 10)` 20x16x2 floor
      panel laid flat plus the four `texOffs(0, 0)` 16x8x2 wall panels boxed in, on a
      64x32 texture; the official `textures/entity/minecart/minecart.png` texture
      reference, texture-backed cutout emission, official PNG atlas upload/bind/sample
      path, and the static `MinecartModel` (no `setupAnim`) shared by both render
      paths. Tests now pin explicit submission metadata for vanilla `entityCutout`,
      white tint, `entity_model_root_transform`, `minecraft:minecart#main`, and
      `(order, submit_sequence) == (0, 0)`, including entity light coords and
      vanilla `OverlayTexture.NO_OVERLAY`, with folded cutout vertices inheriting that
      metadata. The `AbstractMinecartRenderer` rail-follow transform (along-track
      position lerp, slope tilt, hover, the TNT/spawner `displayOffset` and 0.75x
      block-content scale), the chest/furnace/hopper/command-block/TNT/spawner content
      models, and lighting remain unsupported
    - every vanilla 26.1 entity type id `0..=156` maps to a deterministic
      renderer model key; unknown future ids use an explicit
      `todo_unknown_entity_type_bounds` placeholder
    - primitive renderer-owned model families for humanoids and quadrupeds, plus
      named placeholder bounds for remaining entity types
  - Backend GPU resources stay outside `WorldStore`.
  - Full entity presentation remains phase 6 work, including texture assets,
    variants, equipment, skins, animation, lighting, custom/datapack cow/pig
    variant asset presentation, sheep
    head-look-pitch presentation,
    wolf colored force-transparent / GPU outline presentation,
    boat/raft water-mask presentation and lighting (paddle rowing animation,
    hurt/damage roll, bubble wobble, underwater state, and above-water water-mask
    gating are projected and rendered),
    horse animation, donkey/mule animation presentation,
    undead horse animation presentation, and remaining non-base-equine presentation,
    villager live/dynamic profiled-player skin presentation (crossed-arms
    held items, generic non-skull head items, static mob skulls, profileless
    default-player heads, profiled default-skin player heads, dragon heads, and piglin heads are implemented),
    illager live/dynamic profiled-player skin/arm-pose presentation
    (standard held items, generic non-skull head items, static mob skulls, and
    profileless default-player heads, profiled default-skin player heads, dragon heads, and piglin heads are implemented), zombie-family
    armor/zombie-villager converting-state/piglin-family
    armor/live/dynamic profiled-player skin/arm-pose/converting-state
    presentation (the drowned outer layer — adult and baby — plus the trident-throw arm
    pose and swimAmount re-pose ARE implemented),
    skeleton armor, held-item, and animation presentation,
    creeper swelling/powered overlays,
    spider walk-animation presentation (the 180-degree death flip is implemented),
    enderman creepy render jitter (the carried-block arm pose, held-block block-model render,
    and creepy head/hat shift are implemented),
    copper golem keyframe/live/dynamic profiled-player skin presentation
    (the base model, weathering texture swap, emissive eyes, standard held-item
    layer, antenna block decoration, generic non-skull head items, static mob
    skulls, profileless default-player heads, profiled default-skin player heads, dragon heads, and piglin heads are implemented),
    armor stand equipment/live/dynamic profiled-player/
    custom layers/wiggle presentation, slime/magma-cube squish/full
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

### Item Model Range-Dispatch And Select Projection

- Owner: `bbb-protocol` + `bbb-native` + `bbb-pack`
- Status: `partial`
- Next action:
  - Thread the ambient-context numeric `range_dispatch` properties through the
    icon resolver as that state becomes available to the GUI icon path:
    - `minecraft:compass` (needle direction to spawn/lodestone target)
    - `minecraft:time` (daytime / moon-phase clock dial, with the wobbler)
    - `minecraft:cooldown` (item cooldown group progress)
    - `minecraft:crossbow/pull`, `minecraft:use_duration`, `minecraft:use_cycle`
      (local `using_item` use-tick state)
  - Wire the remaining ambient-context `select` properties onto the same
    resolver:
    - `minecraft:context_dimension`, `minecraft:local_time`,
      `minecraft:context_entity_type`, `minecraft:main_hand`
  - Add a typed value representation for `minecraft:component` select cases
    (`ComponentContents.get`, which dispatches case decoding through the selected
    data component's own codec) before wiring it as a stack-only select provider.
  - Project `minecraft:trim_material` onto the dropped-item billboard and
    item-frame surfaces too (currently only the GUI icon path receives the
    trim-material registry keys; dropped/frame paths pass `None`).
  - Each plugs into the existing value-aware `RangeDispatch` / `Select`
    resolver by adding a value provider; no new selection machinery is required.
- Evidence / boundary:
  - `bbb-protocol` now decodes the `minecraft:custom_model_data` `floats` list
    (`CustomModelDataFloats`, bit-exact `Eq`) plus the strings/colors lists, the
    `minecraft:block_state` property map, the `minecraft:charged_projectiles`
    item templates (`charged_projectiles_items`), and the `minecraft:trim`
    material holder reference id (`armor_trim_material_id`), so the
    `CustomModelDataProperty.getFloat(index)`,
    `CustomModelDataProperty.getString(index)`, `ItemBlockState.get`,
    `Charge.get`, and `TrimMaterialProperty.get` inputs are preserved on the wire.
  - `bbb-native` resolves `minecraft:range_dispatch` item models with the exact
    vanilla `RangeSelectItemModel.update` selection:
    - `value = property.get(...) * scale`
    - `NaN` selects the fallback
    - `lastIndexLessOrEqual` over thresholds sorted ascending (inclusive
      `<=`; `-1` selects the fallback)
  - `bbb-native` resolves value-aware `minecraft:select` item models by matching
    the projected property value against each case's `when` values (vanilla
    `SelectItemModel`), falling back when no case matches.
  - The context-free properties are projected from the item stack with vanilla
    math:
    - `minecraft:damage` — `Damage.get` (`damage / max_damage` normalized, or
      `clamp(damage, 0, max_damage)`), reading the component patch over the item
      prototype `max_damage` default
    - `minecraft:custom_model_data` — `CustomModelDataProperty.get`
      (`floats[index]`, or `0.0` when absent)
    - `minecraft:count` — `Count.get`, reading `ItemStack.getCount()` and the
      effective max stack size (component patch over item prototype), with
      `normalize` defaulting to `true`
    - `minecraft:bundle/fullness` — `BundleItem.getFullnessDisplay`, summing
      `BundleContents` weights: ordinary entries weigh `count / max_stack_size`,
      nested bundle entries add the fixed `1/16` bundle item weight even when
      empty, and stacks with non-empty `minecraft:bees` count as full-weight
      bundle entries
    - `minecraft:block_state` — `ItemBlockState.get`, reading the selected
      property from the stack's `minecraft:block_state` property map and falling
      back when the component/property is absent
    - `minecraft:custom_model_data` string select —
      `CustomModelDataProperty.getString(index)`, matching `strings[index]`
      against the case values and falling back when absent/out of range
    - `minecraft:charge_type` — `Charge.get` (`ROCKET` when any charged
      projectile is `minecraft:firework_rocket`, `ARROW` when charged otherwise,
      else `NONE`), using the native item registry to identify the projectile
    - `minecraft:trim_material` — `TrimMaterialProperty.get`, projecting the
      armor trim material holder id through the `minecraft:trim_material` dynamic
      registry (`bbb-world` registry keys threaded into the GUI icon resolver) to
      the material key (e.g. `minecraft:quartz`) matched against each case
  - A value-aware `RangeDispatch` / `Select` is treated as a runtime condition so
    it is resolved per stack rather than collapsed at model-build time.
  - The trim-material registry keys are projected into the GUI icon path
    (`hud_item_icon_for_stack`); the dropped-item billboard and item-frame paths
    still pass `None`, so a dropped/framed trimmed-armor icon falls back to the
    untrimmed model (documented follow-up).
  - `bbb-protocol` now preserves the `minecraft:bees` component occupant count
    (`DataComponents.BEES`, id 77) so bundle-fullness weight can distinguish
    beehive-like full-weight entries from ordinary stack-size weighted entries.
  - The remaining numeric properties (`compass`, `time`, `cooldown`,
    `crossbow/pull`, `use_cycle`, `use_duration`) and the remaining ambient
    select properties (`context_dimension`, `local_time`,
    `context_entity_type`, `main_hand`) still collapse to the fallback/first entry
    because their value needs ambient `ClientLevel` / `ItemOwner` / use-tick
    context the GUI icon resolver does not yet receive. `minecraft:component`
    also remains deferred until the runtime carries typed component values for
    case matching. This is the documented follow-up.

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
