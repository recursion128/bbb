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
- Status: `covered`
- Next action:
  - Preserve `WorldStore::apply_play_packet` as the single shared decoded
    play-packet application path. When adding a new play-packet side effect,
    keep sink-less `PlayApplyEffects` callers and native sink-driven callers on
    the same deterministic random/context path.
- Evidence / boundary:
  - `WorldStore::apply_play_packet` is the single shared clientbound
    play-packet -> canonical-world mapping. Both the offline probe
    (`bbb-net/src/probe/play.rs`) and the online dispatcher
    (`bbb-native/src/runtime/events/dispatcher.rs`, via `NetEvent::Play`)
    delegate to it; per-family packet arms exist only there.
  - Runtime side effects (audio/particle sinks, chat/vehicle
    acknowledgements) flow through the `PlayApplyEffects` trait; the
    deterministic level-event sound/particle random stream advances
    identically for sink-less callers, including potion break, dragon
    fireball, wax-on, vault, trial spawner, and sculk charge events that the
    probe previously skipped.
  - Sink-less LevelEvent random context now uses world-owned read-only chunk
    probes: sculk-charge pop asks the existing world collision shape path
    whether the event block is a full collision block, and plant-growth
    randoms use the same vanilla `BoneMealItem.addGrowthParticles` mode
    classification (`NEIGHBOR_SPREADER` / `GROWER` / water) before sound
    seeds are drawn. Probe tests cover both a loaded full-block sculk pop
    context and a loaded water growth context.
  - Connection-owned packets (keepalive/ping, chunk batch feedback, cookies,
    configuration handoff, resource-pack responses, movement responses,
    disconnects, unknown packets) are returned to the caller and stay
    net-context-owned by design.
  - The final criterion:
    - every supported decoded packet stays aligned

### Renderer Frame Extraction Timing

- Owner: `bbb-native`
- Status: `current field list audited`
- Next action:
  - No known RendererFrame / adjacent renderer-state extraction interleaves
    remain in the current audit. When adding a new world -> renderer field or
    state update, verify its extraction point against the vanilla tick -> render
    frame order in the same slice.
  - A verified field either keeps its position with a vanilla citation on the
    binding, or its `let` moves across the relevant tick advance with the same
    citation.
- Evidence / boundary:
  - `bbb-native/src/runtime/render_extract.rs` owns `RendererFrame` and
    `apply_renderer_frame`.
  - The sky-flash-dependent `lightmap_environment`, `clear_color`,
    `fog_environment`, `sky_environment`, and `cloud_environment` extraction
    now reads after `advance_sky_flash_time`, matching vanilla
    `Minecraft.tick` -> `ClientLevel.tick` -> `GameRenderer.extract` order:
    `ClientLevel.tick` decrements `skyFlashTime`, and render extraction then
    samples the resulting `EnvironmentAttributes` / lightmap state.
  - HUD local-player values (`hud_health`, `hud_food`,
    `hud_experience_progress`, `hud_selected_slot`, hotbar icons, and inventory
    screen projection) now have an explicit source-order test and binding
    comment: vanilla `Minecraft.tick` handles gameplay keybinds before
    `GameRenderer.extractGui` calls `Gui.extractRenderState` /
    `Gui.extractItemHotbar`, so bbb reads these fields after
    `advance_player_input`, destroy/use input advancement, and
    `advance_local_using_item_ticks`.
  - Dropped item models, item entity billboards, entity model instances, held
    item models, item-frame models, and entity block-item models now have a
    source-order test and binding comment: vanilla `Minecraft.tick` advances
    keybinds, `gameRenderer.tick`, and `level.tickEntities` before
    `GameRenderer.extract` calls `LevelRenderer.extractLevel` /
    `extractVisibleEntities`, so bbb reads these fields after entity animation,
    client-time, item-cooldown, input, and local use-item tick advancement.
  - Block-destroy overlays now have source-order and merge coverage: vanilla
    `LevelRenderer.extractBlockDestroyAnimation` reads block-breaking render
    state during render extract, after the client tick, and
    `destroyBlockProgress` stores local `MultiPlayerGameMode` stages and
    server progress in the same per-position sorted set before extracting the
    highest progress. bbb reads `block_destroy_overlays` after
    `advance_block_destruction_render_ticks` and projects the highest
    local/server stage through the official `destroy_stage_0..9` atlas
    entries.
  - Selection outline, entity-scene outline, and entity-target outline now have
    a source-order test and binding comment: vanilla `Minecraft.renderFrame`
    calls `pick(partialTicks)` before `GameRenderer.extract`, and
    `LevelRenderer.extractBlockOutline` then reads `hitResult` plus the current
    camera, so bbb extracts these outline fields after input/use-item/entity
    tick advancement and after the frame camera pose is bound.
  - `cloud_frame` now has a source-order test and binding comment: vanilla
    `LevelRenderer.renderLevel` samples `level.getGameTime()`, the frame
    partial tick, and `cameraRenderState.pos` for `addCloudsPass`, so bbb
    extracts `cloud_frame` after `advance_client_time`, after computing
    `entity_partial_tick`, and after binding the frame camera pose.
  - `weather_render_state` now has a source-order test and binding comment:
    vanilla `LevelRenderer.extractLevel` calls
    `WeatherEffectRenderer.extractRenderState(level, ticks, deltaPartialTick,
    cameraPos, ...)`, and that helper samples rain level, column animation
    ticks, terrain light, and precipitation around the camera, so bbb extracts
    weather after `advance_client_time`, after computing `entity_partial_tick`,
    and after binding the frame camera pose.
  - Particle light refresh now has a source-order test and binding comment:
    vanilla `Minecraft.tick` handles gameplay input before
    `ParticleEngine.tick`, then `LevelRenderer.extractLevel` calls
    `ParticleEngine.extract`; `SingleQuadParticle.extractRotatedQuad` samples
    `getLightCoords(partialTicks)` from the particle's current `BlockPos`.
    bbb now advances particles after input/use-item advancement and refreshes
    particle light after frame extraction inputs are bound, before the renderer
    can collect particle vertices.
  - The renderer receives the whole frame in one commit, so reorders are pure
    extraction-timing questions and cannot introduce partial-frame states.

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
  - Work the todo rows of the per-provider tracking table established
    2026-07-05 in the detail file (section "Per-provider tracking table"):
    6 todo cells on 6 of 113 provider rows — 4 collision
    (`[leaf-bounds]` FallingLeaves per-spawn size x3, `[wake-grow]` per-tick
    wake growth x1) and 2 player-coupled (`[nearest-player]` PlayerCloud
    pull); sounds and removal-gates have no open todo. The shared `[bounds]`
    root cause (vanilla per-provider `setSize` collision AABBs vs the former
    default 0.2x0.2) was closed by flipping its 24 rows — drip family 0.01,
    rain / splash 0.01, bubble / bubble-column 0.02, soul and firefly 0.3 —
    to `collision_size()` (down from the original 30 todo / 28 collision).
    Flip cells to `covered` with commit hashes as slices land; goal.md no
    longer duplicates the list.
  - Implement remaining renderer slices for provider-specific behavior,
    non-particle-atlas terrain/item particle layer sorting, and
    collision/player-coupled physics (world collision clipping, cloud/sneeze
    local-player context, and totem/crit/enchanted-hit/entity-event-driven
    tracking emitters are covered so far).
  - Preserve missing definition/sprite diagnostics.
  - Follow-up work in the plan: full vanilla provider behavior and
    presentation parity.
- Evidence / boundary:
  - Current runtime drains level-particle spawn batches, records vanilla
    particle render-plan metadata for covered single-quad providers (atlas
    opaque/translucent split, `ParticleEngine` group order, terrain/item atlas
    sub-rect UVs), uploads a stitched official particle atlas when assets are
    available, and draws active particles as camera-facing textured
    billboards. Per-target ownership follows vanilla
    `ParticleFeatureRenderer`'s
    `useParticleTarget = particleTarget != null && translucent`: opaque
    particles draw into the main color+depth target before the feature-target
    depth copies, and only translucent particles render into the dedicated
    particles target.
- Detailed per-slice history: docs/unsupported/particle-runtime-vanilla-parity.md

### Renderer Scene Parity

- Owner: `bbb-renderer` + `bbb-native` + `bbb-pack` + `bbb-world`
- Status: `partial`
- Next action:
  - Sheep/wolf presentation parity and vanilla dropped-item follow-up
    rendering remain open; continue renderer presentation work with
    deterministic tests or explicit manual-comparison notes.
  - Full entity presentation remains phase 6 work: texture assets, variants,
    equipment, skins, and animation for the remaining entity families
    (boat/raft visual parity beyond the completed water-mask GPU path,
    equine/camel presentation, villager/illager/zombie-family/piglin-family
    live-profile and armor presentation, skeleton armor/held-item/animation,
    creeper overlays, spider walk-animation, enderman primitive presentation,
    copper golem keyframe presentation, armor stand equipment/layers/wiggle,
    slime/magma-cube squish, and precise vanilla mesh parity for
    primitive/placeholder entity families), plus custom/datapack variant
    asset presentation (reclassified as P3 resource/datapack
    generalization).
- Evidence / boundary:
  - P0 GPU submission/pipeline closeout is largely audited: texture-backed
    residual mesh-emitting arms are gone (`entity_models/dispatch.rs` only
    documents non-textured colored fallback/debug geometry), the
    target/post-chain render-graph order (main/translucent/itemEntity/
    particles/weather/clouds/entity_outline) is pinned by tests, and cull /
    translucent / outline GPU submission buckets follow vanilla constants.
    Screenshot/readback now gates the surface format before the copy.
    The world border forcefield now draws into the weather target after
    rain/snow with vanilla `WorldBorderRenderer` mesh/alpha/tint/UV-scroll
    formulas and `RenderPipelines.WORLD_BORDER` state (2026-07-05, detail in
    the per-slice history file).
  - Entity presentation migration off wrong-model proxies is complete; the
    `EntityRenderState` projection carries packed-light shading, the hurt red
    overlay, creeper swelling, death animation, riptide spin, the
    Dinnerbone/Grumm upside-down easter egg, sleeping pose, uniform model
    scale, walk-animation limb swing, body-shake rotation, and head-look —
    all implemented end to end.
  - Backend GPU resources stay outside `WorldStore`.
- Detailed per-slice history: docs/unsupported/renderer-scene-parity.md

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
    - future level-event audio changes when vanilla sources or supported side
      effects expand
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
      - bone meal use for event `1505`, emitted after
        `BoneMealItem.addGrowthParticles` random consumption
      - honeycomb wax on for event `3003`, emitted after the vanilla
        block-face `minecraft:wax_on` particle random sequence
      - brush-block-complete event `3008` for suspicious sand/gravel, using
        the event-data `BrushableBlock.getBrushCompletedSound()` mapping and
        `SoundSource.PLAYERS`
    - native dispatcher playback for vanilla entity-event local positioned
      sounds:
      - totem use entity event `35`, emitted after the totem
        `TrackingEmitter` side effect at the entity position as
        `minecraft:item.totem.use`, with source/category mapped from
        `Entity.getSoundSource()`
      - armadillo peek entity event `64`, emitted at the entity position as
        `minecraft:entity.armadillo.peek`, with neutral source and fixed
        volume/pitch `1.0`
      - armor-stand hit entity event `32`, emitted at the entity position as
        `minecraft:entity.armor_stand.hit`, with neutral source, volume `0.3`,
        and pitch `1.0`
      - armor-stand death entity event `3`, emitted at the entity position as
        `minecraft:entity.armor_stand.break`, with the generic LivingEntity
        silent gate and death-event pitch randomization
      - zombie death entity event `3`, emitted at the entity position as
        `minecraft:entity.zombie.death`, with hostile source, the generic
        LivingEntity silent gate, and death-event pitch randomization
      - zombie-villager death entity event `3`, emitted at the entity position
        as `minecraft:entity.zombie_villager.death`, with hostile source, the
        generic LivingEntity silent gate, and death-event pitch randomization
      - ravager and iron-golem death entity event `3`, emitted at the entity
        position as `minecraft:entity.ravager.death` /
        `minecraft:entity.iron_golem.death`, with hostile / neutral source,
        the generic LivingEntity silent gate, and death-event pitch randomization
      - witch and villager death entity event `3`, emitted at the entity
        position as `minecraft:entity.witch.death` /
        `minecraft:entity.villager.death`, with hostile / neutral source, the
        generic LivingEntity silent gate, and death-event pitch randomization
      - skeleton, stray, and bogged death entity event `3`, emitted at the
        entity position as `minecraft:entity.skeleton.death` /
        `minecraft:entity.stray.death` / `minecraft:entity.bogged.death`, with
        hostile source, the generic LivingEntity silent gate, and death-event
        pitch randomization
    - native dispatcher playback for randomized vanilla `LevelEventHandler`
      sounds using a runtime-local `LegacyRandomSource`-shaped `nextFloat()`:
      - fire extinguish / generic extinguish
      - ghast/blaze/dragon/wither/zombie/skeleton/phantom hostile effects
      - anvil, grindstone, book, smithing table, dripstone, wind charge
      - lava extinguish and redstone torch burnout sounds
      - splash/instant-effect potion break sounds for events `2002` and
        `2007`, emitted after the item-break and spell-particle random draws
      - dragon fireball explode sound for event `2006` when `data == 1`,
        emitted after the 200 dragon-breath particle random draws
      - distance-delayed trial spawner sounds for events `3012`, `3013`,
        `3014`, `3019`, `3020`, and `3021`; audio-only dispatch advances the
        vanilla post-sound spawn / detect-player / eject-item /
        become-ominous particle random streams before later LevelEvent sound
        seeds
      - distance-delayed vault activate/deactivate sounds for events `3015`
        and `3016`, emitted after the local vault particle random sequence;
        `3015` is gated on a loaded vault block entity at the event position
      - end gateway spawn and ender dragon growl sounds
      - sculk charge sounds for event `3006`, including the fixed pop branch
        and the randomized charged branch; audio-only dispatch advances the
        post-sound charged block-face or pop-particle random stream before
        later LevelEvent sound seeds, including the full-block-context 40-pop
        branch
      - sculk-shrieker sound for event `3007`, gated off when the loaded event
        block state has `waterlogged=true`, using
        `SculkShriekerBlock.TOP_Y`, volume `2.0`, and
        `0.6 + random.nextFloat() * 0.4` pitch after the shriek particles
      - sculk charge pop particles for event `3006` `data >> 6 == 0`: native
        level-event particle context now carries the target block full-shape
        classification, and the resolver emits `minecraft:sculk_charge_pop`
        with vanilla `20` / `40` count, `0.25` / `0.45` spread, and `0.07`
        velocity scale
      - sculk-shrieker event `3007` now emits the vanilla ten
        `minecraft:shriek` particles at `SculkShriekerBlock.TOP_Y` with
        `ShriekParticleOption(i * 5)` delays, then records/plays the
        waterlogged-gated `SCULK_SHRIEKER_SHRIEK` positioned sound when the
        loaded event block state is not waterlogged
      - lava extinguish and redstone torch burnout now share the dispatcher
        path with renderer smoke side effects for events `1501` and `1502`;
        audio-only dispatch also advances the post-sound smoke particle random
        stream for events `1501`, `1502`, and `1503` before later LevelEvent
        sound seeds
      - particle-only events `2000`, `2003`, `2004`, `2009`, and `2010`
        advance their vanilla smoke / ender-eye item-break / blaze smoke /
        cloud / white-smoke random streams in audio-only dispatch before later
        LevelEvent sound seeds
      - particle-only block-face / axis events `3002`, `3004`, `3005`, and
        `3009` advance their vanilla electric-spark, wax-off, scrape, and
        egg-crack random streams in audio-only dispatch before later LevelEvent
        sound seeds
      - cobweb place event `3018` consumes the vanilla poof-particle random
        sequence before recording/playing:
        - `minecraft:block.cobweb.place`
        - `SoundSource.BLOCKS`
        - volume `1.0`
        - pitch `(nextFloat - nextFloat) * 0.2 + 1.0`
        - `distanceDelay=true`
      - particle descriptors map `SpitParticle.Provider` to the
        `ExplodeParticle` random gray tint, age sprite, quad-size and lifetime
        formulas while overriding gravity to `0.5`; `ExplodeParticle.Provider`
        / `minecraft:poof` uses vanilla `command + +/-0.05` velocity and
        `0.9` friction.
      - particle descriptors map `FireworkParticles.SparkProvider` for
        `minecraft:firework` to age sprites, vanilla `SimpleAnimatedParticle`
        friction `0.91`, gravity `0.1`, full-bright light, translucent
        particle layer, command velocity, `0.75` quad-size scale, fixed
        initial alpha `0.99`, `48 + random.nextInt(12)` lifetime, and the
        half-lifetime alpha fade formula. Firework rocket entity event `17`
        with empty/no explosions now emits vanilla `minecraft:poof` particles;
        non-empty explosions now project `FireworkParticles.Starter` small /
        large ball, star, creeper, and burst base spark shapes, center `flash`,
        per-spark fade-colors, trail child spark duplication, and twinkle
        visibility gating, plus the life-0 blast / large_blast local ambient
        sound and delayed twinkle / twinkle_far local ambient sound.
      - particle descriptors map `DripParticle.HoneyHangProvider`,
        `HoneyFallProvider`, and `HoneyLandProvider` for `dripping_honey`,
        `falling_honey`, and `landing_honey` to random sprites, vanilla
        DripParticle opaque layer, zero initial velocity, physics metadata,
        fixed honey tints, `0.98` friction, direct gravity motion,
        hang-particle `0.02` post-move damping, lifetimes `100`,
        `64/(random*.8+.2)`, and `128/(random*.8+.2)`, with gravity
        `0.000012`, `0.01`, and `0.06`. The falling provider now removes on
        `onGround` through the collision-backed `move` path. The landing
        provider now uses collision-backed `DripParticle` move/friction without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates. Falling honey ground hits
        now emit positioned audio through renderer particle sound events for
        `minecraft:block.beehive.drip`.
      - particle descriptors map `DripParticle.ObsidianTearHangProvider`,
        `ObsidianTearFallProvider`, and `ObsidianTearLandProvider` for
        `dripping_obsidian_tear`, `falling_obsidian_tear`, and
        `landing_obsidian_tear` to random sprites, vanilla DripParticle opaque
        layer, zero initial velocity, physics metadata, fixed purple tint,
        `0.98` friction, direct gravity motion, hang-particle `0.02`
        post-move damping, glowing block-light override, lifetimes `100`,
        `64/(random*.8+.2)`, and `28/(random*.8+.2)`, with gravity
        `0.000012`, `0.01`, and `0.06`. The falling provider now removes on
        `onGround` through the collision-backed `move` path. The landing
        provider now uses collision-backed `DripParticle` move/friction without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates.
      - particle descriptors map `BubblePopParticle.Provider` to fixed lifetime
        `4`, age sprites, command velocity, default `SingleQuadParticle` white
        tint / quad-size sampling, gravity `0.008`, and its custom tick path
        that subtracts full gravity without applying default friction.
      - particle descriptors map `AttackSweepParticle.Provider` to xAux-derived
        quad size, random gray tint, fixed lifetime `4`, age sprites, vanilla
        zero-aux `Particle` constructor velocity sampling, and its no-motion
        tick path plus full-bright light coords.
      - particle descriptors map `SuspendedParticle.UnderwaterProvider` to
        vanilla `y - 0.125` initial position, random sprite selection, fixed
        blue tint, `SingleQuadParticle` quad-size sampling times `0.2..0.8`,
        `8/(random*.8+.2)` lifetime, zero velocity, friction `1.0`, and
        no-physics metadata.
      - particle descriptors map `SculkChargeParticle.Provider` to command
        velocity, alpha `1.0`, `1.5` quad-size scaling, age sprites,
        `8..=19` lifetime, friction `0.96`, no-physics metadata,
        translucent particle layer, full-bright block override, and decoded
        `SculkChargeParticleOptions.roll` billboard rotation.
      - particle descriptors map `TrialSpawnerDetectionParticle.Provider` for
        `trial_spawner_detection` and `_ominous` to age sprites, the
        `BaseAshSmokeParticle` `dir=(0.0, 0.9, 0.0)` base-spread velocity with
        the command velocity threaded through (no offset),
        `scale(1.5)` over the vanilla `0.75` single-quad scale,
        `12/(0.5+random*.5)` lifetime, opaque particle layer, full-block
        light, grow-to-base size curve, physics metadata, and vanilla
        `FacingCameraMode.LOOKAT_Y` vertex orientation with world-Y up.
      - particle descriptors map `DustPlumeParticle.Provider` for
        `dust_plume` to age sprites, `BaseAshSmokeParticle` `0.75`
        quad-size scale, `7/(random*.8+.2)` lifetime, command velocity plus
        `0.15` y offset, opaque particle layer, no-physics metadata, `0.5`
        initial gravity, `0.96` friction, `ARGB(0xBAB1C2)-random*.2`
        tint, grow-to-base size curve, and provider tick motion that decays
        gravity by `0.88` and friction by `0.92` before default motion.
      - particle descriptors map `AshParticle.Provider` and
        `WhiteAshParticle.Provider` for `ash` / `white_ash` to age sprites,
        `BaseAshSmokeParticle` `0.75` quad-size scale, `20/(random*.8+.2)`
        lifetime, opaque particle layer, no-physics + speed-up-when-Y-blocked
        metadata, `0.96` friction, and the vanilla `BaseAshSmokeParticle`
        base-spread velocity: the `Particle` 7-arg zero-aux normalized spread
        scaled per axis by `dir = (0.1, -0.1, 0.1)` (`xd *= dirX; yd *= dirY;
        zd *= dirZ`) so the y component is negated and `0.1`-damped, then the
        provider velocity added (`xd += xa; yd += ya; zd += za`). `ash` adds
        `(0, 0, 0)` with `0.5` random-gray tint and `0.1` gravity; `white_ash`
        ignores the command velocity and adds its own
        `xa = rand*-1.9*rand*.1`, `ya = rand*-0.5*rand*.1*5.0`,
        `za = rand*-1.9*rand*.1` with fixed `ARGB(0xBAB1C2)` tint and
        `0.0125` gravity. Vanilla passes `hasPhysics=false` for both providers,
        so there is no provider-specific collision removal path.
      - particle descriptors map `WaterDropParticle.Provider` and
        `SplashParticle.Provider` for `rain` / `splash` to random sprites,
        vanilla single-quad size, `8/(random*.8+.2)` lifetime, opaque
        particle layer, physics metadata, `0.98` friction, direct gravity
        motion, and damping. `rain` uses constructor random x/z velocity
        damped by `0.3`, `0.1..0.3` y velocity, and `0.06` gravity;
        `splash` uses `0.04` gravity and the vanilla horizontal command
        branch `(xa, 0.1, za)`. Runtime ticks now use collision-backed `move`
        and apply vanilla `onGround` 50% random removal plus X/Z ground damping;
        block/fluid in-block removal now queries the world surface height as
        `max(collisionShape.max(Y, localX, localZ), fluidState.height)` and
        removes `rain` / `splash` below that surface.
      - particle descriptors map `WakeParticle.Provider` for `fishing` to
        first sprite initialization, vanilla single-quad size,
        `8/(random*.8+.2)` lifetime, command velocity, opaque particle layer,
        physics metadata, `0.98` friction, zero gravity,
        `setSize(0.01F,0.01F)` collision bounds, collision-backed `move`,
        damping, and wake sprite cycling via `SpriteSet.get((60-lifetime)%4, 4)`
        during runtime ticks.
      - particle descriptors map `FallingLeavesParticle.CherryProvider`,
        `PaleOakProvider`, and `TintedLeavesProvider` for `cherry_leaves` /
        `pale_oak_leaves` / `tinted_leaves` to random sprites, fixed `300`
        lifetime, opaque particle layer, `1.0` friction, physics metadata,
        vanilla `scale * (0.05 | 0.075)` quad-size choice, cherry flow-away
        parameters `(fall=0.25, side=2.0, startVelocity=0.0)`, pale/tinted
        swirl parameters `(fall=0.07, side=10.0, startVelocity=0.021)`,
        tinted `ColorParticleOption` ARGB command decode with renderer RGB
        tinting and default alpha preservation, gravity
        `fallAcceleration * 1.2 * 0.0025`, flow/swirl acceleration, and roll
        spin acceleration. Runtime ticking now routes the leaf move through the
        world collision callback, removes on `onGround`, removes on horizontal
        axis blocking after the first tick, and preserves the vanilla first-tick
        horizontal-block grace.
      - particle descriptors map
        `FlyStraightTowardsParticle.OminousSpawnProvider` for
        `ominous_spawning` to random sprites, command velocity, initial
        position at `spawn + velocity` with `spawn` retained as the
        interpolation start, vanilla `0.1*(random*.5+.2)` quad size followed
        by `scale(randomBetween(3,5))`, `25+random*5` lifetime, opaque
        particle layer, no-physics metadata, full-block light, and the
        straight-toward tick path plus `ARGB.srgbLerp` from `0xFF45AEFE` to
        white.
      - particle descriptors map `ShriekParticle.Provider` to
        `ShriekParticleOption.delay` carried in
        `ParticleSpawnCommand.initial_delay_ticks`; delayed instances do not
        tick or emit vertices while `delay > 0`, then use vanilla random sprite
        selection, fixed `0.85` quad size, `30` lifetime, fixed `(0, 0.1, 0)`
        velocity, translucent layer, full-block light override,
        `0.85 * clamp((age + partial) / lifetime * 0.75, 0, 1)` size curve,
        linear alpha fade, and the vanilla two rotated quads from
        `ShriekParticle.extract` (`rotationX(-1.0472)` and
        `rotationYXZ(-PI, 1.0472, 0)`).
      - particle descriptors map `EndRodParticle.Provider` to command velocity,
        `0.75` quad-size scaling, age sprites, `60..=71` lifetime, friction
        `0.91`, gravity `0.0125`, full-bright light coords, translucent
        particle layer, `SimpleAnimatedParticle` half-lifetime alpha fade, and
        `setFadeColor(15916745)` RGB fade toward `0xF2DEC9` by 20% per tick
        after half lifetime. Runtime ticks preserve vanilla `hasPhysics=true`
        metadata while using the EndRod-specific collision-free `move` override
        so world collision callbacks cannot stop its motion.
      - particle descriptors map `LavaParticle.Provider` to random sprite
        selection, constructor-random horizontal velocity damped by `0.8`,
        random upward velocity `0.05..0.45`, `0.2..2.2` quad-size scaling,
        vanilla `1 - progress^2` shrinking size curve,
        `16/(random*.8+.2)` lifetime, friction `0.999`, gravity `0.75`,
        physics metadata, full-bright block light, and child smoke emission
        using the command-carried smoke SpriteSet and vanilla
        `random.nextFloat() > age / lifetime` post-tick odds.
      - particle descriptors map `CampfireSmokeParticle.CosyProvider` and
        `SignalProvider` to random sprites, constructor `scale(3.0)`, alpha
        `0.9` / `0.95`, `80..129` / `280..329` lifetime, command x/z velocity
        plus `yAux + random.nextFloat() / 500.0`, gravity `3.0E-6`, physics
        metadata, translucent particle layer, vanilla `0.25` x `0.25`
        collision size, random x/z drift, collision-backed `move`, alpha-`<= 0`
        pre-motion removal, and final 60-tick alpha fade.
      - particle descriptors map `SnowflakeParticle.Provider` to age sprites,
        fixed pale-blue tint, `0.1 * (random * random + 1.0)` quad size,
        command velocity plus random `+-0.05` per axis,
        `16/(random*.8+.2)+2` lifetime, friction `1.0`, gravity `0.225`,
        physics metadata, opaque particle layer, and vanilla post-tick damping
        (`xd *= 0.95`, `yd *= 0.9`, `zd *= 0.95`).
      - particle descriptors map `SuspendedParticle.CrimsonSporeProvider` and
        `WarpedSporeProvider` to random sprites, `y - 0.125` initial position,
        `0.6..1.2` quad-size multiplier, `16/(random*.8+.2)` lifetime, no
        physics, friction `1.0`, zero gravity, opaque layer, crimson gaussian
        micro-drift with `[0.9, 0.4, 0.5]` tint, and warped downward random
        drift with `[0.1, 0.1, 0.3]` tint.
      - particle descriptors map `SquidInkParticle.Provider` and
        `GlowInkProvider` to age sprites, fixed `0.5` quad size, black /
        glow-ink tint, command velocity, `6/(random*.8+.2)` lifetime, friction
        `0.92`, zero gravity, no-physics metadata, and full-bright light coords.
        The translucent particle layer and `SimpleAnimatedParticle`
        half-lifetime runtime alpha fade are represented. Runtime ticks now
        apply the vanilla post-`super.tick()` `yd -= 0.0074F` downward drift
        when the post-move world block sample is air.
      - particle descriptors map `SimpleVerticalParticle.PauseMobGrowthProvider`
        and `ResetMobGrowthProvider` to random sprites, random `0.5..1.1`
        quad-size scaling, fixed lifetime `8`, command velocity with
        `-0.03` / `+0.03` y offset, default `0.98` friction, zero gravity, and
        physics metadata plus vanilla opaque particle layer.
      - particle descriptors map `FallingDustParticle.Provider` for
        `minecraft:falling_dust` to age sprites, zero constructor velocity,
        vanilla `32/(random*.8+.2)` lifetime scaled by `0.9`, `0.67499995`
        quad-size multiplier, grow-to-base size curve, default `0.98`
        friction, zero gravity, physics metadata, ordinary opaque particle
        atlas layer, sampled roll / rotSpeed, and tick motion that rotates,
        moves by current velocity, subtracts `0.003` from Y velocity, and clamps
        it to `-0.14`. Native spawn resolution now mirrors the provider's
        `!state.isAir() && state.getRenderShape() == RenderShape.INVISIBLE`
        rejection for water/lava, bubble column, barrier, structure void, end
        portal/gateway, light, and moving piston while preserving packet sample
        RNG consumption before the rejected provider result. Its
        `FallingBlock#getDustColor` branch is
        projected into `ParticleSpawnCommand.option_color` for sand/red_sand,
        gravel, dragon_egg, anvils, and concrete_powder states; non-FallingBlock
        vanilla `BlockColors.createDefault()` layer-0 tint is also installed for
        constant, default-colormap, redstone power, stem age, and lily pad
        world-color sources. Vanilla static mapColor fallback now covers
        foundational non-tinted stone/dirt/planks, wood/log/bamboo axis states,
        wooden stairs/slabs/pressure plates/doors/trapdoors/fences/fence gates/
        signs/hanging signs/shelves, banner/wall banner `WOOD`, button,
        glass/glass pane/iron bars/iron chain/copper bars/copper chains,
        redstone/slime/bone/frosted-ice/dirt-path/petrified-slab misc static
        blocks, ladder/torch/end rod, rail/redstone fixture, skull/head, non-tinted
        potted, cake, air / cave_air / void_air, and test_instance_block
        default-NONE groups, crimson/warped stem/hyphae colors,
        DyeColor / colored terracotta families, bed/candle/shulker decorative
        families, cave/emissive block families, copper weathering families,
        nether flora / blackstone static families, quartz/prismarine/End static families, construction
        stone/brick static families, deepslate construction variants,
        infested stone CLAY variants, resin/pale garden static families,
        plant/dripstone/moss/root/mud natural static families, non-tinted
        foliage static families, crop/succulent static families,
        produce/fungus static families, utility/mechanical static families
        including stone/weighted pressure plates, utility fixtures, functional
        blocks, and redstone utility/control blocks, aquatic/coral static
        families, bamboo/honey/campfire utility static families,
        water plant/egg static families, flower/tall flower static families,
        fire/cocoa/creaking heart static families, glowstone/enchanting/beacon
        static families,
        ore/deepslate/nether colors, mineral/natural static block families, and
        the final accepted vanilla static states (mycelium, packed mud, nether
        brick fence, nether portal default NONE, stripped pale oak wood, and
        copper lantern weathering/waxed variants). Full mapColor catalog
        coverage is now pinned by the registry-wide falling-dust color test;
        biome-aware per-spawn BlockColors now use the same spawn-position
        world probe path as terrain particles. On-ground roll reset is now
        covered by collision-backed particle ticking.
      - native spawn resolution mirrors `TerrainParticle.createTerrainParticle`
        for definition-less `minecraft:block`, `minecraft:dust_pillar`, and
        `minecraft:block_crumble` submissions: air, `moving_piston`, and
        `shouldSpawnTerrainParticles=false` block states return no particle
        while preserving packet sample RNG consumption. Their spawn commands now
        carry the block state's terrain particle material sprite id, matching
        vanilla `TerrainParticle` construction through
        `BlockStateModelSet.getParticleMaterial(blockState).sprite()`.
        The same providers also carry `0.6 *
        BlockTintSource.colorAsTerrainParticle` for vanilla layer-0 block color
        sources; `minecraft:block_marker` remains sprite-only, matching vanilla
        `BlockMarker`.
        LevelEvent `addDestroyBlockEffect` / brush-complete `minecraft:block`
        commands now reuse the same terrain tint catalog with the event block
        position as the vanilla `TerrainParticle` tint position.
        `minecraft:block_marker` remains unfiltered, matching vanilla
        `BlockMarker.Provider`, and its spawn commands now carry the same
        block-state terrain particle material sprite id because vanilla
        `BlockMarker` also constructs with
        `BlockStateModelSet.getParticleMaterial(blockState).sprite()`.
      - renderer fixed `BreakingItemParticle` providers resolve their vanilla
        `ItemStackTemplate` sprite ids from local 26.1 assets:
        `minecraft:item_slime` -> `minecraft:item/slime_ball`,
        `minecraft:item_cobweb` -> `minecraft:block/cobweb`, and
        `minecraft:item_snowball` -> `minecraft:item/snowball`. The generic
        `minecraft:item` provider now decodes the `ItemStackTemplate`
        `DataComponentPatch` into the protocol component summary and resolves
        concrete GROUND particle material active-layer sprite ids through the
        native item runtime, including component-driven root item-model
        changes. The renderer uses random sprite selection for
        `BreakingItemParticle` providers.
      - renderer descriptor tests now cover the full vanilla 26.1
        `ParticleResources.registerProviders()` id list and reject any entry
        that falls back to generic `Particle`; remaining particle gaps are
        terrain/item atlas rendering, world-coupled collision/tint, LevelEvent
        branches, or the remaining component-rich / generic entity branches of
        `ItemPickupParticle` carried-entity submit.
      - particle descriptors map `ElderGuardianParticle.Provider` to
        definition-less `minecraft:elder_guardian`, fixed lifetime `30`, zero
        aux/motion/gravity provider metadata, translucent
        `entityTranslucent(textures/entity/guardian/guardian_elder.png)`
        intent, and `ParticleRenderType.ELDER_GUARDIANS`; atlas billboard
        vertices are limited to `SINGLE_QUADS`, and actual model drawing is now
        covered by particle-target entity translucent submission using the
        vanilla elder guardian texture, alpha curve, full-bright light, no
        overlay, camera-facing rotation, `0.42553192` particle scale, translated
        model pose, and `2.35` elder baked-layer scale.
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
    - The 2026-07-02 LevelEvent audio audit rechecked the local vanilla
      `LevelEventHandler` switch against current world/native tests and
      confirmed the covered set includes fixed-pitch local sounds, randomized
      local sounds, particle-before-sound ordering for potion break, dragon
      fireball, wax-on, bone-meal, vault, sculk-shrieker, and cobweb place,
      post-sound particle random-stream advancement for smoke, trial-spawner,
      sculk, simple particle-only, and block-face events, camera-relative
      `globalLevelEvent` sounds, portal-travel local ambience, and jukebox
      start/stop. P1-5 no longer tracks a LevelEvent-audio-specific open item.
    - Totem entity event `35` local positioned audio is covered separately from
      LevelEvent audio, including native resolver coverage for the
      `minecraft:item.totem.use` command.
    - 2026-07-04 determinism: the client sound-seed source no longer draws from
      wall-clock time. Vanilla seeds `Level.soundSeedGenerator` /
      `ClientLevel.random` from `RandomSupport.generateUniqueSeed()`
      (`SEED_UNIQUIFIER ^ System.nanoTime()`) and derives per-sound seeds via
      `RandomSource.nextLong()`. `bbb-world` now keeps the vanilla
      `LegacyRandomSource.nextLong()` advancement but seeds it from a fixed local
      seed on `ClientAudioState.sound_seed_random` (`WorldStore::next_sound_seed`),
      so the serializable world state stays deterministic (same packet stream ->
      same seeds/pitches). `LevelEventSoundRandomState::default()` likewise uses a
      fixed seed; native's level-event pitch stream is seeded from
      `world.next_sound_seed()`. The field is `#[serde(default)]` for snapshot
      compatibility. This removes the last `SystemTime::now()` determinism leak in
      `bbb-world`.
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
  - Thread any newly discovered ambient-context numeric `range_dispatch`
    properties through the icon resolver as that state becomes available to the
    GUI icon path.
  - Wire the remaining ambient-context `select` properties onto the same
    resolver:
    - `minecraft:context_entity_type` for any future non-GUI item consumer that
      gains a real living owner but is not routed through the current
      owner-backed generated held-item path
    - remaining `minecraft:local_time` coverage beyond the supported
      root/en plus selected English regional week-data ICU numeric/date-time /
      timezone-offset subset (which now includes `u` proleptic year,
      supported-English `Y` week-year, `G` era, `D` day-of-year, and `Q`/`q`
      quarter fields, root/en `M`/`L` month widths 1..=5, `g` Julian day,
      `F`
      day-of-week-in-month, supported-English `w`/`W` week numbers,
      supported-English `e`/`c` local weekdays, `A` milliseconds-in-day,
      root/en `a` AM/PM widths
      1..=5, plus localized-GMT `O` offsets, `Z`/`X`/`x` offset widths
      1..=5, short `z` zone
      abbreviations, `VV` zone IDs, and `VVV` exemplar cities, plus
      fixed/UTC long `z` names, with root/en `w` year-end boundary coverage,
      Sunday-first regional week coverage, and Monday/minimal-days=4 Jan 1
      previous-week-year / previous-month `W` coverage):
      full localized symbols
      and long-tail ICU pattern fields (locale-specific week data beyond the
      selected English regional groups, IANA long `z`, generic `v`, and
      one-/four-letter `V` widths)
  - Audit remaining item consumers that vanilla renders with a living owner and
    pass that owner context into the item resolver. `minecraft:main_hand` and
    `minecraft:context_entity_type` are now wired for owner-backed generated
    item attachments and GUI/HUD item icons that use the local-player owner
    context.
  - Extend `minecraft:component` select beyond the current decoded scalar /
    enum / simple literal text subset plus vanilla item/block default-name
    translatable keys as protocol/runtime summaries become available: complex
    object/list component values, style-sensitive Component equality,
    registry-backed component value codecs, custom/datapack component defaults
    beyond parsed vanilla item properties, custom datapack component value
    decoding, and components without a persistent codec remain follow-up.
  - Audit remaining non-GUI item consumers that can render component-bearing
    generated item stacks and pass dynamic registry keys where vanilla resolves
    registry-backed item-model properties. Dropped-item `GROUND`, item-frame
    `FIXED`, owner-backed third-person held generated items, and GUI/HUD icons
    now carry trim-material keys and direct enchantment keys; no-registry
    consumers still fall back.
  - Vanilla Quick Charge-modified crossbow charge duration is now wired for
    GUI/HUD local-player icons and owner-backed third-person generated held-item
    paths when the synced `minecraft:enchantment` registry identifies
    `minecraft:quick_charge`. The same registry projection now feeds direct-key
    and vanilla/static tag-key enchantment component predicates for GUI/HUD,
    dropped `GROUND`, item-frame `FIXED`, and owner-backed third-person
    generated held-item icons; custom enchantment effect parsing remains later
    registry/effect generalization.
  - Each plugs into the existing value-aware `RangeDispatch` / `Select`
    resolver by adding a value provider; no new selection machinery is required.
- Evidence / boundary:
  - `bbb-protocol` now decodes the `minecraft:custom_model_data` `floats` list
    (`CustomModelDataFloats`, bit-exact `Eq`) plus the flags/strings/colors
    lists, the `minecraft:block_state` property map, the
    `minecraft:charged_projectiles` item templates
    (`charged_projectiles_items`), and the `minecraft:trim` material holder
    reference id (`armor_trim_material_id`), and the
    `minecraft:consumable` `consume_seconds` value (`consumable`), plus the
    `minecraft:item_model` resource id and `minecraft:lodestone_tracker`
    target `GlobalPos` / `tracked` flag, so the
    `CustomModelDataProperty.getFloat(index)`,
    `CustomModelData.getBoolean(index)`,
    `CustomModelDataProperty.getString(index)`, `ItemBlockState.get`,
    `Charge.get`, `TrimMaterialProperty.get`,
    `DataComponents.ITEM_MODEL`, `DataComponents.LODESTONE_TRACKER`, and
    `ItemStack.getUseDuration(owner)` consumable input are preserved on the wire.
  - `bbb-native` resolves `minecraft:range_dispatch` item models with the exact
    vanilla `RangeSelectItemModel.update` selection:
    - `value = property.get(...) * scale`
    - `NaN` selects the fallback
    - `lastIndexLessOrEqual` over thresholds sorted ascending (inclusive
      `<=`; `-1` selects the fallback)
  - `bbb-native` resolves value-aware `minecraft:select` item models by matching
    the projected property value against each case's `when` values (vanilla
    `SelectItemModel`), falling back when no case matches.
  - `bbb-native` resolves `minecraft:display_context` item-model selects from
    vanilla `DisplayContext.get`, matching the current `ItemDisplayContext`
    serialized name against `when` values. GUI/HUD item icons pass `gui`,
    dropped-item generated layers pass `ground`, item-frame generated layers
    pass `fixed`, owner-backed third-person held generated layers pass their
    hand display context, and nested bundle selected items inherit the parent
    context. Tests pin texture selection across those contexts.
  - `bbb-native` resolves `minecraft:using_item` conditions for
    third-person/entity-owned generated item attachments by matching the
    submitted logical hand to vanilla `LivingEntity.getUseItem()` (`isUsingItem`
    plus `getUsedItemHand`); using an item in the other hand keeps the submitted
    stack on the false branch.
  - `bbb-native` resolves `minecraft:has_component` conditions with vanilla
    `HasComponent.get`: ordinary conditions use `ItemStack.has` so prototype
    defaults such as `minecraft:max_stack_size`, `minecraft:item_model`,
    `minecraft:rarity`, and common empty `minecraft:enchantments` count as
    present; vanilla `minecraft:enchanted_book` also counts its default empty
    `minecraft:stored_enchantments` component unless removed. The
    `ignore_default=true` path uses `ItemStack.hasNonDefault` / patch presence
    so added and removed component patches both select the true branch. Tests
    pin texture selection for default, added, and removed cases.
  - `bbb-native` resolves the root item model from the effective
    `DataComponents.ITEM_MODEL` value before evaluating the item-model tree:
    unpatched stacks use the vanilla default item id initialized by
    `Item.Properties.finalizeInitializer`, patched stacks select the overridden
    root model like `ItemModelResolver.appendItemLayers`, and removed component
    id 10 yields no item layers. The prototype item id remains the source for
    default `max_damage` / `max_stack_size` context values. Tests pin default,
    alternate, and removed behavior through texture UV selection.
  - Retained item display transforms are read from the same effective
    `DataComponents.ITEM_MODEL` root before applying the current display
    context, matching `ItemModelResolver.appendItemLayers` feeding
    `ModelRenderProperties.applyToLayer`. Dropped-item `GROUND`, item-frame
    `FIXED`, owner-backed held-item contexts, and HUD GUI block-item icons all
    query stack-aware transforms. Tests pin the default item root, a patched
    alternate root, and removed component id 10 returning no retained transform.
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
    - `minecraft:custom_model_data` condition —
      conditional `CustomModelDataProperty.get`, matching
      `flags[index] == true`; missing, false, out-of-range, or removed
      `minecraft:custom_model_data` component id 17 selects the false branch
    - `minecraft:selected` — `IsSelected.get`, for HUD hotbar item icons:
      the local selected hotbar slot resolves the true branch and non-selected
      hotbar slots resolve false. Local inventory GUI hotbar slots also resolve
      true for slot `36 + selected_hotbar_slot`, while same-stack non-selected
      slots remain false instead of matching by item contents. Recognized
      server-opened container GUI hotbar slots now resolve from the vanilla menu
      slot layout's player hotbar start plus selected hotbar index.
    - `minecraft:carried` — `IsCarried.get`, projected as an explicit
      local-player carried-stack context bit in the native item icon resolver:
      ordinary HUD/GUI slot and generated recipe/offer item icons keep the
      false branch, while call sites that own the actual
      `containerMenu.getCarried()` stack can resolve the true branch without
      matching by item contents. GUI inventory cursor-carried items now use
      that true branch while projecting vanilla
      `AbstractContainerScreen.extractCarriedItem`'s non-dragging
      `mouseX - 8`, `mouseY - 8` floating item position; local quick-craft
      drag preview now applies vanilla `getQuickCraftPlaceCount` /
      `quickCraftingRemainder` to the floating cursor stack. Touchscreen
      split-stack and snapback animation remain broader GUI surface follow-up.
    - `minecraft:component` condition — `ComponentMatches.get`, currently for
      the `DataComponentPredicate` component-type / `AnyValue` branch plus the
      concrete `minecraft:damage` predicate. Native maps the AnyValue
      component id through the same decoded component table as `has_component`:
      default prototype components such as `minecraft:rarity` count as present,
      removed components select false, and non-default patched components such
      as `minecraft:enchantment_glint_override` select true regardless of their
      boolean payload. The damage predicate matches vanilla
      `DamagePredicate.matches` over the stack's `minecraft:damage` and
      `durability = max_damage - damage` bounds. Empty single-component
      predicates for `minecraft:bundle_contents`, `minecraft:container`,
      `minecraft:trim`, `minecraft:firework_explosion`, `minecraft:fireworks`,
      and `minecraft:jukebox_playable` also match the vanilla
      component-present branch. `minecraft:firework_explosion` shape / trail /
      twinkle constraints are decoded from the component and matched against
      vanilla `FireworkExplosionPredicate`; `minecraft:fireworks` decodes and
      matches `FireworksPredicate.flightDuration` `MinMaxBounds.Ints` plus
      `explosions.size` `CollectionPredicate.size` against the decoded
      explosions count. `minecraft:trim`
      direct registry-key or trim-material-tag constraints match the decoded
      `ArmorTrim.material()` holder id through dynamic trim-material registry
      keys and native trim-material tag catalog. `minecraft:jukebox_playable`
      now matches the optional `song` HolderSet against the decoded
      `JukeboxPlayable.song()` holder id through vanilla
      `JukeboxSongs.bootstrap` order, including direct vanilla registry keys and
      native jukebox-song tag entries. `minecraft:potion_contents` now matches
      vanilla `PotionsPredicate` HolderSets against the decoded
      `PotionContents.potion()` holder id through vanilla `Potions`
      registration order, including direct vanilla registry keys and native
      potion tag entries. `minecraft:writable_book_content` now matches decoded
      raw writable-book pages with vanilla `CollectionPredicate` `contains` /
      `count` / `size`; `minecraft:written_book_content` now matches decoded
      written-book raw title, author, `generation` `MinMaxBounds.Ints`,
      `resolved`, and plain-string plus simple literal
      `ComponentSerialization` text-object page collection predicates.
      `minecraft:villager/variant` now matches decoded 0-based
      `VillagerType` holder ids against direct registry-key or
      villager-type-tag HolderSets using the vanilla `VillagerType.bootstrap`
      registry-key order. `minecraft:attribute_modifiers` now preserves decoded
      modifier entries and matches direct plus bundle/container-nested
      `modifiers` collection predicates over direct registry-key or
      attribute-tag `attribute` HolderSets when `minecraft:attribute` registry
      keys are available, plus `id`, `amount`, `operation`, `slot`, and `size`
      / `contains` / `count`. Tool, sword, spear, humanoid armor, wolf armor,
      horse armor, nautilus armor, mace, and trident item-prototype default
      modifiers now feed the same direct plus bundle/container-nested predicate
      path unless the stack explicitly removes or overrides
      `ATTRIBUTE_MODIFIERS`.
      `minecraft:custom_data` now preserves decoded custom-data NBT compound
      summaries and matches direct plus bundle/container-nested predicates with
      vanilla `NbtUtils.compareNbt(..., true)` subset-compound and partial-list
      semantics for JSON-object and SNBT-string compound predicate values.
      `minecraft:bundle_contents`
      `items.size` constraints match vanilla `CollectionPredicate.size` against
      the decoded bundle item count, and `items.contains` / `count` now support
      vanilla `ItemPredicate` direct item-key or item-tag HolderSets,
      stack-count bounds, exact scalar/default `DataComponentMatchers`
      components, and patch-backed simple literal `minecraft:custom_name` /
      `minecraft:item_name` / `minecraft:lore` exact components plus
      `minecraft:unbreakable` Unit exact components and exact
      `minecraft:custom_data` compound components (JSON-object or SNBT-string
      expected values), plus exact `minecraft:potion_contents` components for
      direct potion keys, optional `custom_color`, ordered direct mob-effect
      `custom_effects` payloads including amplifier / duration / ambient /
      particles / icon / recursive hidden-effect details, and optional
      `custom_name`, plus exact `minecraft:writable_book_content`
      ordered `Filterable<String>` page lists with raw and optional filtered
      strings, plus exact `minecraft:written_book_content` components for raw
      / filtered title strings and ordered simple literal plus styled / extra /
      translated component page text summaries, plus
      exact `minecraft:firework_explosion` components for
      `shape`, ordered `colors`, ordered `fade_colors`, `has_trail`, and
      `has_twinkle`, plus exact `minecraft:fireworks` components for
      `flight_duration` and ordered explosion lists, plus nested partial
      `minecraft:damage`, plus exact `minecraft:jukebox_playable` components
      for direct vanilla jukebox-song keys and inline direct-song objects with
      direct or registry sound-event holders, description text summaries,
      `length_in_seconds`, and `comparator_output`, plus exact `minecraft:trim`
      components for direct trim-material registry keys, inline trim-material
      payloads with asset name / override armor assets / description text,
      direct vanilla trim-pattern keys, and inline trim-pattern payloads with
      asset id / description text / decal, plus exact `minecraft:enchantments` and
      `minecraft:stored_enchantments` components for direct enchantment
      registry-key maps, plus exact `minecraft:villager/variant` components
      for direct vanilla villager-type registry keys, plus exact
      `minecraft:lodestone_tracker` components for optional target `GlobalPos`
      and `tracked`, plus exact `minecraft:attribute_modifiers` components for
      ordered modifier lists with direct attribute registry keys, `id`,
      `amount`, `operation`, `slot`, and default / hidden / simple literal
      override display text summaries, including styled / extra component text,
      `minecraft:firework_explosion`, `minecraft:fireworks`, `minecraft:trim`,
      `minecraft:jukebox_playable`, `minecraft:potion_contents`,
      `minecraft:writable_book_content`, `minecraft:written_book_content`,
      `minecraft:villager/variant`, `minecraft:attribute_modifiers`,
      `minecraft:custom_data`, and data-component AnyValue predicates over
      decoded bundle entries.
      `minecraft:container` now decodes non-empty container entries and matches
      the same direct item-key / item-tag / stack-count / exact scalar component
      / nested partial damage, enchantments, stored-enchantments,
      firework-explosion, fireworks, trim, jukebox-playable, potion-contents,
      writable-book-content, written-book-content, villager-variant,
      attribute-modifiers, and AnyValue predicate collection subset.
      `minecraft:fireworks`
      `explosions.contains` / `count` predicates now match decoded explosion
      shape / trail / twinkle summaries. `minecraft:trim` direct vanilla
      registry-key or trim-pattern-tag constraints now match decoded
      `ArmorTrim.pattern()` holder ids through vanilla `TrimPatterns.bootstrap`
      order and native trim-pattern tag catalog.
      `minecraft:enchantments` and patch-backed
      `minecraft:stored_enchantments` match decoded enchantment levels and
      direct registry-key or enchantment-tag HolderSet predicates when the
      `minecraft:enchantment` registry keys and native enchantment tag catalog
      are available to the icon resolver; GUI/HUD, dropped `GROUND`, item-frame
      `FIXED`, and owner-backed third-person generated held-item paths now
      thread that registry context. Empty
      `minecraft:enchantments` predicate lists honor vanilla's default empty
      `ENCHANTMENTS` component unless id 13 is removed, and vanilla
      `minecraft:enchanted_book` contributes its item-specific default empty
      `STORED_ENCHANTMENTS` component unless id 42 is removed. Exact
      `minecraft:enchantments` and `minecraft:stored_enchantments` component
      maps now compare decoded holder ids and levels against direct
      enchantment registry keys when that dynamic registry is available.
      Exact `minecraft:villager/variant` components now compare decoded holder
      ids against direct vanilla villager-type registry keys. Exact
      `minecraft:lodestone_tracker` components now compare the decoded optional
      target `GlobalPos` and `tracked` flag. Exact
      `minecraft:attribute_modifiers` components now compare decoded ordered
      modifier entries against direct attribute registry keys, numeric amount,
      operation, slot, and default / hidden / simple literal plus styled /
      extra override display text summaries. Exact
      `minecraft:written_book_content` components now compare decoded raw /
      filtered title strings, author, generation, resolved, and ordered simple
      literal plus styled / extra / translated raw / filtered page text
      summaries. Remaining
      constrained predicate types (inline enchantment holder payloads / server
      datapack tag remaps, broader NBT scalar typing, remaining concrete partial
      predicates and complex exact component codecs beyond the already covered
      simple literal name / lore, Unit `unbreakable`, compound `custom_data`,
      filterable-page-list `writable_book_content`, full-field
      `firework_explosion` / `fireworks`, direct-key `jukebox_playable`, and
      direct-key `trim` exact components, potion / mob-effect datapack registry
      remaps for `potion_contents`, attribute modifier inline / datapack
      attribute holder payloads, full style-sensitive written-book page
      `ComponentSerialization` equality, datapack trim-material or trim-pattern
      registry-key remaps, datapack
      villager-type registry remaps, jukebox-song datapack registry remaps, and
      similar) remain follow-up.
    - `minecraft:charge_type` — `Charge.get` (`ROCKET` when any charged
      projectile is `minecraft:firework_rocket`, `ARROW` when charged otherwise,
      else `NONE`), using the native item registry to identify the projectile
    - `minecraft:trim_material` — `TrimMaterialProperty.get`, projecting the
      armor trim material holder id through the `minecraft:trim_material` dynamic
      registry (`bbb-world` registry keys threaded into the GUI icon resolver,
      dropped-item `GROUND` flat model path, and item-frame `FIXED` flat model
      path) to the material key (e.g. `minecraft:quartz`) matched against each case
    - `minecraft:main_hand` — `MainHand.get`, matching the owner's
      `HumanoidArm` serialized name (`left` / `right`) for third-person
      entity-owned generated item attachments and GUI/HUD item icons that can
      use the local-player owner context. This matches
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` to
      `ItemModelResolver.updateForTopItem`; null-owner/fake item consumers still
      fall back.
    - `minecraft:context_dimension` — `ContextDimension.get`, matching the
      current `ClientLevel.dimension()` resource key for GUI/HUD item icons and
      owner-backed third-person generated held-item paths from `bbb-world`'s
      `WorldLevelInfo.dimension`; no-level item consumers still fall back.
    - `minecraft:context_entity_type` — `ContextEntityType.get`, matching the
      owner entity type resource key for GUI/HUD item icons. This mirrors
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` as the owner,
      so the GUI/HUD context value is `minecraft:player`. Owner-backed generated
      held-item paths project the renderer entity kind to the vanilla entity
      type key before resolving generated item layers; tests pin player vs witch
      branch selection. Null-owner/fake item consumers still fall back.
    - `minecraft:view_entity` — `IsViewEntity.get`, for GUI/HUD local-player
      item icons in the normal camera==player path. This mirrors
      `GuiGraphicsExtractor.item`, which passes `minecraft.player` as owner;
      native threads an explicit view-entity bit rather than comparing by item
      contents or entity type. Spectator camera identity and non-GUI
      owner-backed item consumers remain follow-up.
    - `minecraft:extended_view` — `ExtendedView.get`, for GUI/HUD local-player
      item icons when either Shift key is down. Native threads
      `ClientInputState::shift_down()` into the item icon resolver and keeps
      vanilla's GUI display-context gate, so non-GUI consumers select the false
      branch even while Shift is down.
    - `minecraft:keybind_down` — `IsKeybindDown.get`, for GUI/HUD local-player
      item icons whose condition references native-tracked default key names.
      Native projects pressed movement/gameplay/inventory/multiplayer/misc/
      creative non-debug keys plus mouse attack/use/pick buttons into the item
      icon resolver, covering the vanilla `Options.keyMappings` defaults such
      as social interactions, quick actions, screenshot, perspective, fullscreen,
      GUI / spectator-shader toggles, creative toolbar activators,
      spectator-hotbar, hotbar 1-9, and the valid default-unbound
      smooth-camera / spectator-outlines key names as false under the vanilla
      default keymap. User-rebound/custom key mappings and F3/debug modifier
      combos remain follow-up.
    - `minecraft:fishing_rod/cast` — `FishingRodCast.get`, for GUI/HUD
      local-player selected hotbar main-hand fishing rods when a fishing bobber
      add-entity has owner data equal to the local player id. Non-selected
      hotbar slots and no-bobber paths select false; offhand / cursor /
      inventory identity and fishing-hook billboard / line rendering remain
      follow-up.
    - `minecraft:local_time` — `LocalTime.get`, formatting wall-clock time for
      the vanilla 26.1 chest/trapped-chest `MM-dd` selector plus a
      root/en plus selected English regional week-data ICU `SimpleDateFormat`
      subset (`y`/`u` year, supported-English `Y` week-year, `G` era text,
      `Q`/`q` quarter, root/en `M`/`L` month widths 1..=5, `d` day, `D`
      day-of-year, `g` Julian day, supported-English `w`/`W`
      week-of-year / week-of-month, `F` day-of-week-in-month,
      supported-English `E`/`e`/`c` weekdays, 24/12-hour
      `H`/`k`/`K`/`h`, `m`/`s`/`S`, `A` milliseconds-in-day, root/en `a`
      AM/PM widths 1..=5, `Z`/`X`/`x` offset fields through width 5,
      localized-GMT `O` offsets, short `z` zone abbreviations, `VV` zone IDs,
      `VVV` exemplar cities, fixed/UTC long `z` names, and quoted literals).
      Explicit `GMT`/UTC offset and IANA `time_zone` IDs use that zone; absent
      `time_zone` uses the system local timezone like vanilla.
      Tests pin GMT `12-25` selecting the Christmas branch, `12-27` selecting
      the fallback, cross-midnight `UTC+02:30` plus `Asia/Tokyo` date-time /
      weekday / AM-PM branches, UTC `X`/`x` zero-offset formatting,
      `uuuu-DDD-G` proleptic-year / day-of-year / era, `Q`/`q` quarter,
      `g` Julian day, root/en narrow `M`/`L` month symbols,
      localized-GMT `O`, `F` day-of-week-in-month, `A` milliseconds-in-day,
      root/en `a` narrow AM/PM, supported-English `Y` week-year, and
      supported-English `w`/`W`
      week plus `e`/`c` local weekday branches, a root/en `w` year-end boundary
      branch, a Sunday-first regional branch, and a Monday/minimal-days=4
      Jan 1 previous-week-year / previous-month `W` branch; an IANA-zone short
      `z` / `VV` / `VVV` branch plus fixed-offset `zzzz`; UTC and
      `UTC+02:30` offset branches now pin `Z`/`X`/`x` width 4/5 output as well.
    - `minecraft:time` — `Time.get`, for GUI/HUD item icons with a local-player
      owner and `ClientLevel` context. Native projects the `daytime` target
      from the overworld sun angle and `moon_phase` from the vanilla
      eight-phase `day_time / 24000` cycle, applies vanilla default
      `wobble=true` standard wobbler smoothing
      (`NeedleDirectionHelper.standardWobbler(0.9F)`), and then applies
      vanilla range-dispatch threshold selection. Tests pin no-level `0.0`
      fallback, overworld day-time and moon-phase texture selection, and a
      default-wobbled first-tick branch that raw non-wobbled target selection
      would miss. `source=random` uses a persistent per-property Java
      LCG-shaped random source; vanilla seeds it with a client-local unique
      seed, so native uses a deterministic local seed while preserving
      per-property advancement. Tests pin the random branch selecting a texture
      instead of falling back.
    - `minecraft:compass` — `CompassAngle.get`, for GUI/HUD item icons with a
      local-player owner and `ClientLevel` context targeting spawn, lodestone,
      or recovery. Native projects the default-spawn, `LodestoneTracker.target`,
      or local-player `lastDeathLocation` `GlobalPos`, validates it against the
      current dimension, computes vanilla's owner-position / visual-yaw
      rotation toward the block center, applies exact non-wobbled target
      rotation for `wobble=false`, applies default `wobble=true`
      `NeedleDirectionHelper` smoothing factor `0.8` for valid local-player
      targets, and then applies vanilla range-dispatch threshold selection.
      Tests pin no-pose `0.0` fallback, missing-component / missing-recovery
      threshold behavior, same-dimension spawn/lodestone/recovery texture
      selection, compass invalid-target threshold behavior, and a
      default-wobbled valid-target spawn texture-selection branch. No-target /
      invalid-target rotation now follows vanilla
      `getRandomlySpinningRotation`: `target=none` is parsed, each baked
      property has a no-target wobbler/random state, `wobble=true` updates once
      per game tick with factor `0.8`,
      `wobble=false` uses the non-wobbler random value, and the item-model
      seed hash is added before positive modulo. HUD hotbar icons pass
      vanilla-shaped `slot_index + 1` seeds. Tests pin `target=none` and
      cross-dimension spawn invalid-target branches selecting random-spin
      textures instead of the old fixed `0.0` fallback.
    - `minecraft:component` — `ComponentContents.get`, currently matching
      decoded persistent scalar / enum components with typed `when` values:
      `minecraft:max_stack_size`, `minecraft:max_damage`, `minecraft:damage`,
      `minecraft:item_model`, `minecraft:rarity`, and
      `minecraft:enchantment_glint_override`, plus `minecraft:map_id` from the
      synced `MapId` int wrapper and the RGB int wrappers
      `minecraft:dyed_color` / `minecraft:map_color`, plus simple literal
      JSON-string / `{"text": ...}` `minecraft:custom_name` components, plus
      `minecraft:item_name` simple literal patch values and vanilla item/block
      default translatable description keys from
      `Item.Properties.finalizeInitializer`. Native item icons project vanilla
      common defaults (`max_stack_size=64`, `item_model=<item id>`,
      `rarity=common`) and damageable item defaults (`damage=0`,
      `max_damage=<item default>`), and removed component ids suppress the
      selected value before case matching. Tests pin texture selection for
      string, numeric, boolean, resource-id, default, patched, explicit map-id,
      explicit color, literal custom-name string / text-object, item/block
      default / literal item-name string / text-object, and removed component
      cases.
    - `minecraft:cooldown` — `Cooldown.get`, matching the local player's
      `ItemCooldowns.getCooldownPercent(itemStack, 0.0F)` for GUI/HUD item
      icons. The item model property intentionally uses vanilla's `0.0F`
      partial tick, while the separate HUD cooldown overlay still uses render
      partial tick.
    - `minecraft:use_duration` — `UseDuration.get`, for GUI/HUD local-player
      item icons and owner-backed third-person generated held items whose stack
      is the active `LivingEntity.getUseItem()`, using the local / entity use
      tick counter as elapsed ticks (`remaining=false`, vanilla bow asset path)
      and reading vanilla `Consumable.consumeTicks()` (`consume_seconds * 20`
      truncated to int) for ordinary consumable stacks when `remaining=true`;
      tests also pin the 26.1 `EnderEyeItem.getUseDuration` override to `0`.
    - `minecraft:use_cycle` — `UseCycle.get`, for GUI/HUD local-player item
      icons using the active stack's remaining ticks modulo the declared
      positive `period` (vanilla brush asset path, 200 tick brush duration)
    - `minecraft:crossbow/pull` — `CrossbowPull.get`, for GUI/HUD local-player
      item icons and owner-backed third-person generated held-item paths using
      elapsed ticks divided by `CrossbowItem.getChargeDuration`: default 25
      ticks, or Quick Charge's vanilla `-0.25F` per level when the stack
      enchantment holder id resolves to `minecraft:quick_charge` through the
      synced `minecraft:enchantment` registry. Already charged crossbows still
      return `0.0`.
    - `minecraft:display_context` — `DisplayContext.get`, returning the
      serialized `ItemDisplayContext` currently used by the consumer (`gui`,
      `ground`, `fixed`, `thirdperson_righthand`, etc.) before case matching.
  - A value-aware `RangeDispatch` / `Select` is treated as a runtime condition so
    it is resolved per stack rather than collapsed at model-build time.
  - The trim-material registry keys are projected into the GUI icon path
    (`hud_item_icon_for_stack`), dropped-item generated model path, item-frame
    generated model path, and owner-backed third-person held generated item
    path; no-registry / null-context consumers still fall back to the untrimmed
    model.
  - `bbb-protocol` now preserves the `minecraft:bees` component occupant count
    (`DataComponents.BEES`, id 77) so bundle-fullness weight can distinguish
    beehive-like full-weight entries from ordinary stack-size weighted entries.
  - The GUI/HUD numeric `minecraft:compass` path now covers spawn, lodestone,
    recovery, and `none` targets: valid targets project owner-position / yaw
    against the current default spawn, `LodestoneTracker.target`, or
    local-player `lastDeathLocation`, default valid-target wobble is applied
    when requested by the model, and no-target / invalid-target cases use the
    vanilla random-spin branch instead of a fixed `0.0`.
    `minecraft:time` projects GUI/HUD `daytime` / `moon_phase` target values
    from world time, applies the default `wobble=true` standard wobbler, and
    advances per-property `source=random` state instead of falling back.
    `minecraft:local_time` resolves the vanilla chest/trapped-chest `MM-dd`
    selector and common root/en plus selected English regional week-data ICU
    date-time patterns from wall-clock time, including `y`/`u` year,
    supported-English `Y` week-year, `G` era, `D` day-of-year, `Q`/`q` quarter
    widths 1..=5 for root/en, `g` Julian day, `F` day-of-week-in-month,
    supported-English `w`/`W` week numbers and `e`/`c` local weekdays,
    fixed-offset / IANA
    `time_zone` IDs, `Z`/`X`/`x` offset fields, localized-GMT `O` offset
    widths, the root/en `w` year-end boundary, the Sunday-first regional
    branch, and the Monday/minimal-days=4 Jan 1 previous-week-year /
    previous-month `W` branch; full localized symbols and long-tail ICU
    pattern fields (locale-specific week data beyond the selected English
    regional groups, IANA long `z`, generic `v`, and one-/four-letter `V` widths) remain
    follow-up. Short `z` zone abbreviations, `VV` explicit zone
    IDs, `VVV` exemplar cities, and fixed/UTC long `z` names now resolve for
    explicit zones.
    GUI/HUD use-tick properties are wired for the local active stack,
    owner-backed third-person generated held-item paths use the entity render
    state's shared use tick counter, and both paths apply vanilla Quick
    Charge-modified crossbow charge duration when the enchantment registry is
    available. Ordinary and active-use first-person generated item stacks now
    render in the depth-cleared hand pass, and the active-use path passes the
    local active-use tick context into `minecraft:use_duration` /
    `minecraft:use_cycle` item-model range-dispatch properties.
    `minecraft:main_hand` and `minecraft:context_entity_type` still fall back on
    native item consumers that do not pass a `LivingEntity` owner, such as
    fake/null-owner item surfaces. `minecraft:custom_model_data` condition is
    wired for the stack-local `flags` list, and `minecraft:selected` is wired
    for HUD hotbar selected-slot icons plus recognized local and server-opened
    GUI hotbar slots.
    `minecraft:view_entity` is wired for
    GUI/HUD local-player icons in the normal camera==player path, and
    `minecraft:extended_view` is wired for Shift-held GUI/HUD local-player
    icons while retaining vanilla's GUI display-context gate. `minecraft:keybind_down`
    is wired for native-tracked default non-debug `Options.keyMappings` names,
    including vanilla default-unbound names that resolve false until user key
    rebinding exists.
    `minecraft:fishing_rod/cast` is wired for GUI/HUD selected hotbar
    main-hand fishing rods while a local-player-owned fishing bobber exists.
    `minecraft:carried` is wired as an explicit resolver context bit for the
    GUI inventory cursor-carried item path, which now renders the world cursor
    stack as a floating GUI item at vanilla's non-dragging cursor offset and
    applies the vanilla quick-craft remainder count while drag-distributing
    across multiple slots. Touchscreen split-stack and snapback animation remain
    GUI follow-up.
    `minecraft:component` is wired for the scalar / enum / simple literal
    custom-name component select subset listed above, and the condition form
    covers component-type / AnyValue,
    `minecraft:damage`, empty single-component predicates, direct-key
    enchantment HolderSet predicates when the synced enchantment registry is
    available, and direct plus nested bundle/container writable/written-book
    predicates for decoded raw string fields/pages, plus direct and
    bundle/container-nested villager variant predicates for direct registry-key
    or villager-type-tag HolderSets, plus direct and bundle/container-nested
    attribute modifier predicates for decoded direct registry-key or
    attribute-tag attribute / id / amount / operation / slot entries when the
    synced attribute registry is available, plus direct and
    bundle/container-nested custom-data NBT compound predicates for JSON-object
    and SNBT-string compound expected values, plus patch-backed simple literal
    custom-name / item-name / lore exact components and `unbreakable` Unit exact
    components, plus exact custom-data compound components; broader
    component-codec and remaining constrained `DataComponentPredicate` parity
    remains the documented follow-up.

### Native Input, Movement, Interaction, Inventory, And Command Flows

- Owner: `bbb-native` + `bbb-net` + `bbb-protocol` + `bbb-world`
- Status: `partial`
- Next action:
  - Movement: extend the native fixed-20Hz local movement / AABB collision /
    still-water-lava fluid slice toward remaining vanilla survival physics,
    voxel collision shapes, and fluid presentation nuance.
  - Block destroy: close remaining destroy-speed and destroy-profile gaps
    outside the mechanically parsed `Blocks.java` declarations.
  - Commands: continue adding focused command queue and encode tests for
    inventory, interaction, chat, command, and sign editing.
  - Inventory: implement remaining rich tooltip behavior (non-ASCII font
    providers, bidirectional text shaping, official tooltip
    background/frame sprites, and italic/complex component styles).
  - Completion requires full vanilla movement and these flows to work
    through encoded serverbound packets end to end.
- Evidence / boundary:
  - Movement, block destroy, commands, and inventory each have a native
    implementation covering the currently supported vanilla-shaped behavior:
    serverbound movement/physics projection, sprint/destroy-speed derivation
    from world-owned predicates, command-queue packet encoding, and
    container/tooltip rendering for the local and opened containers.
- Detailed per-slice history: docs/unsupported/native-input-movement-interaction-inventory-and-command-flows.md

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
- When a row's body grows past 800 lines, relocate its per-slice detail
  bullets as-is into `docs/unsupported/<kebab-case-row-name>.md` (starting
  with `# <Row Name> — detailed ledger`) and keep the row itself to owner,
  status, a concise next-action/evidence summary, and a
  `Detailed per-slice history:` link line.
