# Particle Runtime Vanilla Parity — detailed ledger

- Next action:
  - Implement remaining renderer slices for:
    - provider-specific behavior
    - remaining non-particle-atlas terrain/item particle layer sorting:
      - native preserves submission commands and `raw_options_len` for
        definition-less block/item atlas particle types
      - renderer records `OPAQUE_TERRAIN` / `OPAQUE_ITEMS` layer metadata
      - sprite-transparency-driven `TRANSLUCENT_TERRAIN` /
        `TRANSLUCENT_ITEMS` selection is covered for uploaded terrain/item
        sprite metadata
    - collision/player-coupled physics:
      - particle tick now accepts a world collision callback, clips vanilla
        0.2x0.2 particle AABBs against known block collision shapes on all
        three axes with vanilla Y-first / largest-horizontal axis order, applies
        `Particle.onGround` X/Z damping, and resets
        `FallingDustParticle` roll on the tick after ground contact
      - `SpellParticle` alpha now receives a native local-player scoping
        context and mirrors vanilla close-to-first-person-spyglass behavior
      - `PlayerCloudParticle.Provider` / `SneezeProvider` now receive native
        local-player position / delta-movement context and mirror the vanilla
        post-`super.tick()` 2-block Y / Y-velocity pull when the cloud is above
        the local player's feet
      - entity event `35` now feeds vanilla totem
        `TrackingEmitter` particles through native, using the entity's current
        AABB width/height, 16 unit-sphere samples per tick, 30 ticks, and
        `minecraft:totem_of_undying` delayed spawn commands
      - `ClientboundAnimate` actions `4`/`5` now feed vanilla crit /
        enchanted-hit `TrackingEmitter` particles through native with the
        default 3-tick lifetime and the entity's current AABB width/height
      - `ClientboundGameEvent` elder-guardian effect now feeds the vanilla
        `minecraft:elder_guardian` particle at the local player's feet; the
        same GameEvent side-effect path emits arrow-hit, puffer-fish sting, and
        elder-guardian curse sounds at the vanilla local-player positions
      - entity event `35` now also records and dispatches vanilla
        `minecraft:item.totem.use` as a positioned local sound at the entity's
        current position, with `Entity.getSoundSource()`-shaped source mapping
      - Ravager and IronGolem entity event `4` now dispatch vanilla fixed-pitch
        attack sounds at the current entity position:
        `minecraft:entity.ravager.attack` / `minecraft:entity.iron_golem.attack`,
        volume `1.0`, pitch `1.0`, silent gate, and vanilla-shaped hostile /
        neutral source mapping
      - `ClientboundTakeItemEntity` now records and dispatches vanilla item /
        experience-orb pickup sounds at the picked entity position before the
        world removes or shrinks the entity; native also submits the vanilla
        `ItemPickupParticle` runtime command before that mutation, and the
        renderer tracks its `ITEM_PICKUP` group order, 3-tick lifetime, target
        midpoint following, and quadratic extract interpolation. Ordinary
        item-stack carried models now also submit through the item-pickup
        particle group using frozen source age/light and dropped-item GROUND
        item-cluster baking. Experience-orb carried models now capture
        vanilla `ExperienceOrbRenderState.icon`, the orb renderer's `+7`
        block-light rule, frozen age, and draw the 16x16 icon from
        `textures/entity/experience/experience_orb.png` through the same
        `ITEM_PICKUP` group. Component-rich item stacks now bake through the
        pickup channel too: native serializes the picked-up
        `DataComponentPatchSummary` (already decoded by
        `ClientboundTakeItemEntity`, no second wire decode) as an opaque blob on
        `option_item_pickup_component_patch`, the renderer round-trips it
        command -> instance -> `ItemPickupParticleRenderState` without inspecting
        it, and the native bake rebuilds the component-rich stack to reuse the
        exact dropped-item GROUND projection (`item_display_transform_for_stack`
        / `generated_item_layers_for_stack_with_registry_context`), so the pickup
        carried bake is byte-identical to the dropped-item bake for the same
        stack (ITEM_MODEL / custom_model_data / damage / block_state_properties
        driven models). The arrow/trident carried-model branch (the last
        picked-up entity family) is done: world projects
        `TakeItemEntityPickupProjectileModel` — normal / tipped
        (`TippableArrowRenderer.isTipped = getColor() > 0`) / spectral arrow and
        trident with `ThrownTrident.ID_FOIL` — plus the extracted `yRot`/`xRot`
        (vanilla `extractEntity(entity, 1.0F)` render-state rotations); native
        carries it on `option_item_pickup_projectile_model`; the renderer bakes
        `ArrowModel` / `TridentModel` (foil glint pass included) with the
        vanilla `Ry(yRot-90) * Rz(xRot [+90 trident])` root transform at the
        quadratic-interpolated pickup position and draws it inside the
        `ITEM_PICKUP` group between the orb-icon and elder-guardian draws
        through the entity translucent-cull pipeline. With item stacks,
        experience orbs, and arrow/trident projectiles all submitting carried
        models, the consumer surface of vanilla's generic `EntityRenderState`
        submit (`ItemPickupParticleGroup.State.submit` ->
        `EntityRenderDispatcher.submit`) is fully covered — no picked-up entity
        kind renders through any other path.
      - Ravager entity event `69` now emits vanilla roar `minecraft:poof`
        particles: 40 commands at the ravager AABB center with gaussian
        velocity scaled by `0.2`, preserving the particle type's
        `overrideLimiter=true`. The same event now also applies
        `Ravager.applyRoarKnockbackClient` for the native local-authoritative
        alive living target modeled here as the local player: targets inside the
        ravager AABB inflated by `4.0` receive vanilla `strongKnockback`
        `(xd/dd*4.0, +0.2, zd/dd*4.0)` with `dd=max(xd^2+zd^2,0.001)`.
      - Witch entity event `15` now emits vanilla `minecraft:witch`
        particles: `nextInt(35)+10` commands at the witch's current `x/z` and
        `boundingBox.maxY + 0.5`, each with gaussian `*0.13F` positional jitter
        on all three axes and zero velocity.
      - LivingEntity entity event `60` now emits vanilla `minecraft:poof`
        particles: 20 commands using gaussian `*0.02` velocity and
        `getRandomX(1.0) - vx*10`, `getRandomY() - vy*10`,
        `getRandomZ(1.0) - vz*10` position sampling from the current living
        entity AABB width/height.
      - LivingEntity entity event `67` now emits vanilla `minecraft:bubble`
        particles: 8 commands at entity position plus three
        `random.triangle(0.0, 1.0)` offsets, using the entity's current
        `deltaMovement` as command velocity.
      - LivingEntity entity event `46` now emits vanilla `minecraft:portal`
        particles: 128 commands interpolated from `xo/yo/zo` to the current
        position, with width/height random offsets and
        `(nextFloat()-0.5)*0.2` velocity.
      - Snowball entity event `3` now emits vanilla hit particles: 8 commands
        at the snowball's current position with zero velocity. The normal
        synced item stack branch uses `minecraft:item`; an explicit empty stack
        uses `minecraft:item_snowball`.
      - ThrownEgg entity event `3` now emits vanilla hit particles: non-empty
        item stacks spawn 8 `minecraft:item` commands at the egg's current
        position with `(nextFloat()-0.5)*0.08` velocity on each axis. Missing
        metadata uses `Items.EGG`; explicit empty stacks emit no particles.
      - Arrow entity event `0` now emits vanilla tipped-arrow effect-clear
        particles: when `Arrow.ID_EFFECT_COLOR` is not `-1`, native submits 20
        `minecraft:entity_effect` commands with zero velocity, option RGB
        decoded from the synced color, and positions sampled from
        `getRandomX(0.5)` / `getRandomY()` / `getRandomZ(0.5)` over the current
        arrow AABB. Color `-1` emits no particles; color `0` emits black.
      - Animal entity event `18` now emits vanilla love `minecraft:heart`
        particles from `Animal.handleEntityEvent`: 7 commands with gaussian
        `*0.02` velocity and `getRandomX(1.0)`, `getRandomY()+0.5`,
        `getRandomZ(1.0)` position sampling from the current animal AABB. This
        is gated to classes inheriting `Animal`.
      - Allay entity event `18` now emits the vanilla duplication
        `minecraft:heart` burst: 3 commands from `Allay.handleEntityEvent`, using
        the same `spawnHeartParticle` gaussian `*0.02` velocity and
        `getRandomX(1.0)`, `getRandomY()+0.5`, `getRandomZ(1.0)` current-AABB
        sampling as vanilla.
      - TamableAnimal and AbstractHorse entity events `6`/`7` now emit vanilla
        taming feedback particles: event `7` success uses 7
        `minecraft:heart` commands, event `6` failure uses 7
        `minecraft:smoke` commands, both with gaussian `*0.02` velocity and
        `getRandomX(1.0)`, `getRandomY()+0.5`, `getRandomZ(1.0)` current-AABB
        sampling from `spawnTamingParticles`.
      - Villager entity events `12`/`13`/`14`/`42` now emit vanilla
        `addParticlesAroundSelf` bursts: 5 commands with gaussian `*0.02`
        velocity, `getRandomX(1.0)`, `getRandomY()+1.0`, and
        `getRandomZ(1.0)` current-AABB sampling; ids map to
        `minecraft:heart`, `minecraft:angry_villager`,
        `minecraft:happy_villager`, and `minecraft:splash`.
      - Dolphin entity event `38` now emits vanilla
        `minecraft:happy_villager` particles from `Dolphin.handleEntityEvent`:
        7 commands with gaussian `*0.01` velocity and `getRandomX(1.0)`,
        `getRandomY()+0.2`, `getRandomZ(1.0)` current-AABB sampling.
      - Fox entity event `45` now emits vanilla `minecraft:item` particles from
        `Fox.handleEntityEvent`: 8 commands when the fox main hand is non-empty,
        using the main-hand item stack, mouth anchor `position + getLookAngle()/2`
        on x/z, and the local random velocity fan rotated by `-xRot` then
        `-yRot` with `+0.05` Y.
      - HoneyBlock entity events `53`/`54` now emit vanilla `minecraft:block`
        particles: event `53` base Entity slide emits 5 commands and event
        `54` LivingEntity jump emits 10 commands, both using
        `Blocks.HONEY_BLOCK.defaultBlockState()`, the entity position, and zero
        velocity.
      - Ravager stun client ticks now mirror vanilla `Ravager.aiStep` /
        `stunEffect`: entity event `39` arms the 40-tick stun timer, each stunned
        tick consumes a deterministic Java-LCG client RNG for `nextInt(6)`, and
        successful ticks enqueue a grey `minecraft:entity_effect` particle at
        the head anchor `position - width*sin(yBodyRot)`,
        `position.y + height - 0.3`, `position + width*cos(yBodyRot)` with the
        vanilla `±0.3` x/z jitter. Evoker fangs attack ticks now mirror vanilla
        `EvokerFangs.tick`: entity event `4` starts `lifeTicks`, and the tick
        where it reaches `14` enqueues the 12 `minecraft:crit` particles with
        vanilla `width*0.5`, `1.05 + random`, and `0.3..0.6` Y-velocity ranges
        before the renderer particle engine advances. The same event also emits
        vanilla `minecraft:entity.evoker_fangs.attack` positioned audio at the
        fang position with neutral source, volume `1.0`, silent gate, and pitch
        `random.nextFloat() * 0.2 + 0.85`.
      - `WakeParticle.Provider` (`minecraft:fishing`) now uses vanilla
        `setSize(0.01F, 0.01F)` collision bounds and the collision-backed
        `Particle.move` path before `0.98` friction and wake sprite cycling.
      - remaining deferred work is broader collision clipping parity for
        special contexts beyond the covered WakeParticle case, player-coupled
        particle emitters beyond the currently covered scoped cases, and broader
        entity-event particle/audio parity outside the currently covered fixed
        attack sounds, Fox, and other families.
    - terrain/item particle option metadata / atlas rendering:
      - native preserves commands and raw option length for definition-less
        block/item atlas particle types
      - native now decodes `BlockParticleOption` block-state ids for block
        atlas particles plus `falling_dust`, and decodes `ItemParticleOption`
        item id / count plus raw component patch byte length for `item`
      - renderer now records vanilla TerrainParticle / BreakingItemParticle /
        BlockMarker provider shape plus random 4x4 UV sub-rect offsets for the
        TerrainParticle / BreakingItemParticle paths
      - billboard vertex emission converts recorded sub-rect offsets into
        vanilla-shaped atlas UVs when a concrete sprite UV is available
      - renderer now records explicit texture-atlas ownership metadata for
        particle / terrain / item atlas layers
      - renderer maps `FallingDustParticle.Provider` as an ordinary particle
        atlas `OPAQUE` provider with vanilla lifetime, age-sprite, size curve,
        roll-speed, clamped falling motion metadata, and native spawn rejection
        for non-air block states whose vanilla render shape is `INVISIBLE`
      - native spawn resolution mirrors
        `TerrainParticle.createTerrainParticle` for `block`, `dust_pillar`,
        and `block_crumble`: air, `moving_piston`, and
        `shouldSpawnTerrainParticles=false` block states are rejected after
        packet sample RNG is consumed; `block_marker` does not use this filter
      - remaining deferred work is broader firework presentation outside the
        current Starter particle/audio path and FireworkRocketEntity client
        tick trail
    - LevelEvent particle and audio side effects are now covered for the
      vanilla 26.1 `LevelEventHandler` switch cases that emit particles,
      positioned sounds, local ambience, global sounds, or jukebox start/stop.
      Remaining related work belongs to deferred owners such as terrain/item
      atlas rendering, block-entity client-effect presentation, and broader
      audio-device/runtime parity rather than a P1-5 LevelEvent mapping gap.
  - Preserve missing definition/sprite diagnostics.
- Evidence / boundary:
  - Current runtime:
    - Drains level-particle spawn batches.
    - Records vanilla particle render-plan metadata for covered single-quad
      providers: `ParticleRenderType.SINGLE_QUADS`,
      `SingleQuadParticle.Layer.OPAQUE` / `TRANSLUCENT`, vanilla
      `ParticleEngine` group order (`SINGLE_QUADS`, `ITEM_PICKUP`,
      `ELDER_GUARDIANS`), and stable solid-before-translucent vertex
      collection within the atlas-backed single-quad path. Atlas-backed
      billboards split opaque and translucent vertex batches and draw them
      through vanilla-shaped `RenderPipelines.OPAQUE_PARTICLE` (no blend) and
      `RenderPipelines.TRANSLUCENT_PARTICLE` (`BlendFunction.TRANSLUCENT`)
      GPU pipelines. Definition-less block/item atlas particle types now keep
      submission commands, record terrain/item layer metadata, preserve decoded
      block-state / item-template option metadata, and record vanilla provider
      shape plus TerrainParticle / BreakingItemParticle random 4x4 sub-rect
      offsets. Billboard vertex emission converts those offsets into
      vanilla-shaped atlas sub-rect UVs when a sprite UV is available, including
      the TerrainParticle / BreakingItemParticle horizontal `u0`/`u1` flip.
      `ParticleInstance` now also carries explicit particle / terrain / item
      texture-atlas ownership metadata. `FallingDustParticle.Provider` is
      represented as an ordinary particle-atlas `OPAQUE` provider with zero
      constructor velocity, age sprite selection, vanilla lifetime, grow-to-base
      size curve, roll / rotSpeed runtime state, and Y velocity clamped to
      `-0.14`; native spawn resolution now projects the provider's
      non-air `RenderShape.INVISIBLE` rejection for water/lava, bubble column,
      barrier, structure void, end portal/gateway, light, and moving piston,
      and the `FallingBlock#getDustColor` branch for sand/red_sand/gravel, anvils,
      dragon_egg, and concrete_powder into the renderer visual tint, and now
      applies vanilla static mapColor fallback for foundational non-tinted
      stone/dirt/planks, wood/log/bamboo axis states, crimson/warped stem/hyphae
      static colors, wooden stairs/slabs/pressure plates/doors/trapdoors/fences/
      fence gates/signs/hanging signs/shelves, banner/wall banner `WOOD`, button,
      glass/glass pane/iron bars/iron chain/copper bars/copper chains,
      redstone/slime/bone/frosted-ice/dirt-path/petrified-slab misc static
      blocks, ladder/torch/end rod, rail/redstone fixture, skull/head, non-tinted
      potted, cake, air / cave_air / void_air, and test_instance_block default
      `MapColor.NONE` groups, DyeColor and colored
      terracotta families, decorative
      bed/candle/shulker families, cave/emissive
      amethyst/tuff/calcite/sculk/froglight families, copper weathering
      families, nether flora / blackstone
      static families, quartz/prismarine/End static families, construction
      stone/brick static families, deepslate construction variants, infested
      stone CLAY variants, resin/pale garden static families,
      plant/dripstone/moss/root/mud natural static families, non-tinted foliage
      static families, crop/succulent static families, utility/mechanical static
      families including stone/weighted pressure plates, utility fixtures,
      functional blocks, and redstone utility/control blocks,
      aquatic/coral static families, bamboo/honey/campfire utility static
      families, water plant/egg static families, flower/tall flower static
      families, fire/cocoa/creaking heart static families, glowstone/enchanting/
      beacon static families, produce/fungus static families, plus ore/deepslate/nether,
      snow/ice/clay/sandstone/suspicious block, and resource block
      mineral/natural static colors, plus mycelium, packed mud, nether brick
      fence, nether portal default `MapColor.NONE`, stripped pale oak wood, and
      all copper lantern weathering/waxed variants. The
      `falling_dust_colors_cover_all_accepted_vanilla_block_states` test now
      enumerates every vanilla 26.1 block state accepted by the provider and
      asserts a resolved tint or mapColor.
      Renderer
      particle draw batches now keep per-atlas draw ranges and bind the
      particle / terrain / item atlas texture selected by
      `SingleQuadParticle.Layer`; native terrain texture upload also supplies
      block atlas sprite UVs to the particle renderer, and native item atlas
      upload supplies item atlas sprite UVs to the same path. Terrain/item
      particle sprite uploads now also carry atlas `hasTranslucent`, and
      renderer vertex batching mirrors vanilla
      `SingleQuadParticle.Layer.bySprite` by routing current translucent
      terrain/item sprites to `TRANSLUCENT_TERRAIN` / `TRANSLUCENT_ITEMS`.
      Fixed `BreakingItemParticle` providers now resolve `minecraft:item_slime`,
      `minecraft:item_cobweb`, and `minecraft:item_snowball` to their vanilla
      item atlas sprite ids. `TerrainParticle` providers now resolve
      `minecraft:block`, `minecraft:block_marker`, `minecraft:dust_pillar`,
      and `minecraft:block_crumble` block-state particle material sprite ids
      through the terrain block-model catalog and upload those ids in spawn
      commands. Generic `minecraft:item` particles now decode the
      `ItemStackTemplate` `DataComponentPatch` into the protocol component
      summary and resolve the concrete GROUND item particle material
      active-layer sprite ids through the native item runtime, including
      component-driven root item-model changes. Particle ticking now gets a
      world collision callback from native, clips vanilla 0.2x0.2 particle AABBs
      against known block collision shapes on all three axes with vanilla
      Y-first / largest-horizontal axis order, applies vanilla
      `Particle.onGround` X/Z damping, and resets `FallingDustParticle` roll on
      the tick after ground contact. Native spawn resolution also mirrors
      `TerrainParticle.createTerrainParticle` for
      definition-less `minecraft:block`, `minecraft:dust_pillar`, and
      `minecraft:block_crumble` submissions by rejecting air, moving-piston, and
      `shouldSpawnTerrainParticles=false` block states after packet sample RNG
      is consumed; `minecraft:block_marker` remains unfiltered like vanilla
      `BlockMarker.Provider`. The native coverage test
      `level_event_particle_resolver_covers_vanilla_26_1_particle_events`
      enumerates every vanilla 26.1 `LevelEventHandler` event id that emits
      particles and verifies a representative mapped batch with no missing
      definition / sprite diagnostics.
      Renderer tests enumerate every id registered by vanilla 26.1
      `ParticleResources.registerProviders()` and assert it maps to an explicit
      vanilla provider descriptor rather than the generic `Particle` fallback.
      Native entity event handling now expands vanilla totem event `35` into a
      `TrackingEmitter` batch: 30 delayed ticks, 16 unit-sphere samples per
      tick, entity AABB width/height position sampling, and
      `minecraft:totem_of_undying` child particles.
      Native animate handling now also expands vanilla actions `4`/`5` into
      crit / enchanted-hit `TrackingEmitter` batches with the default 3 delayed
      ticks and the same entity AABB width/height sampling.
      Native GameEvent handling now expands vanilla elder-guardian effect into
      a `minecraft:elder_guardian` command at the local player's feet, and emits
      `entity.arrow.hit_player`, `entity.puffer_fish.sting`, and
      `entity.elder_guardian.curse` positioned sounds at the vanilla local
      player eye/feet positions.
      The same entity event now emits `minecraft:item.totem.use` as a
      positioned local sound at the current entity position, with source
      mapping derived from vanilla `Entity.getSoundSource()` (`Player` /
      `Monster` / default `Entity` branches covered).
      Native Ravager / IronGolem entity event `4` now emits the vanilla
      fixed-pitch attack positioned sounds: `minecraft:entity.ravager.attack`
      and `minecraft:entity.iron_golem.attack` at the current entity position,
      with volume/pitch `1.0`, silent gate, and hostile/neutral source mapping.
      Native EvokerFangs entity event `4` now emits the vanilla randomized
      positioned attack sound `minecraft:entity.evoker_fangs.attack` at the
      current fang position, with neutral source, volume `1.0`, silent gate,
      pitch `random.nextFloat() * 0.2 + 0.85`, and deterministic native/audio
      tests.
      Native ZombieVillager entity event `16` now emits the vanilla randomized
      cure sound `minecraft:entity.zombie_villager.cure` at `getX()` /
      `getEyeY()` / `getZ()`, with hostile source, silent gate, volume
      `1.0 + random.nextFloat()`, and pitch `random.nextFloat() * 0.7 + 0.3`.
      Native Armadillo entity event `64` now mirrors vanilla
      `Armadillo.handleEntityEvent` by playing `minecraft:entity.armadillo.peek`
      at the current `getX()` / `getY()` / `getZ()` position with neutral source
      and fixed volume/pitch `1.0`, independent of the generic silent flag.
      Native ArmorStand entity event `32` now mirrors the vanilla hit branch by
      playing `minecraft:entity.armor_stand.hit` at the current entity position
      with neutral source, volume `0.3`, pitch `1.0`, and no generic silent
      gate, alongside the existing hit-wiggle state update.
      Native ArmorStand LivingEntity death event `3` now maps the vanilla
      `getDeathSound()` branch to `minecraft:entity.armor_stand.break` at the
      current entity position, with neutral source, generic silent gate, volume
      `1.0`, and pitch `(random.nextFloat() - random.nextFloat()) * 0.2 + 1.0`.
      Native Zombie LivingEntity death event `3` now maps
      `Zombie.getDeathSound()` to `minecraft:entity.zombie.death`, with hostile
      source, the generic silent gate, volume `1.0`, and the same death-event
      pitch randomization.
      Native ZombieVillager LivingEntity death event `3` now maps
      `ZombieVillager.getDeathSound()` to
      `minecraft:entity.zombie_villager.death`, with the same hostile source,
      silent gate, volume, and death-event pitch randomization.
      Native Ravager and IronGolem LivingEntity death event `3` now map
      `getDeathSound()` to `minecraft:entity.ravager.death` and
      `minecraft:entity.iron_golem.death`, with hostile / neutral source
      mapping, generic silent gate, volume `1.0`, and the same death-event
      pitch randomization.
      Native Witch and Villager LivingEntity death event `3` now map
      `Witch.getDeathSound()` / `Villager.getDeathSound()` to
      `minecraft:entity.witch.death` / `minecraft:entity.villager.death`,
      with hostile / neutral source mapping, generic silent gate, volume
      `1.0`, and the same death-event pitch randomization.
      Native Skeleton, Stray, and Bogged LivingEntity death event `3` now map
      vanilla `getDeathSound()` to `minecraft:entity.skeleton.death`,
      `minecraft:entity.stray.death`, and `minecraft:entity.bogged.death`,
      with hostile source, generic silent gate, volume `1.0`, and the same
      death-event pitch randomization.
      Native TakeItemEntity handling now also emits vanilla item /
      experience-orb pickup positioned sounds at the picked entity position
      before world mutation removes or shrinks the entity. The same packet now
      emits a synthetic `minecraft:item_pickup` particle command with the
      source entity position / delta movement, target midpoint source, and
      pre-shrink item stack summary; renderer runtime keeps it in the vanilla
      `ITEM_PICKUP` group and advances the 3-tick target-following lifetime.
      Ordinary item-stack carried models now draw in the particle target after
      single-quad particles and before elder-guardian special-group rendering,
      reusing the captured source entity id, frozen `extractEntity(..., 1.0F)`
      age/light, quadratic target interpolation, and dropped-item GROUND
      cluster bake. Experience-orb carried models now draw in the same
      `ITEM_PICKUP` group with captured icon, frozen age, the orb renderer's
      `+7` block-light rule, and the vanilla 64x64 experience-orb texture
      atlas subdivisions. Component-rich item stacks now also draw in the pickup
      target: the picked-up stack's `DataComponentPatchSummary` (already decoded
      by `ClientboundTakeItemEntity`) rides the pickup channel as an opaque blob
      that the renderer round-trips without inspecting, and the native bake
      rebuilds the stack to reuse the dropped-item GROUND projection, making the
      pickup carried bake byte-identical to the dropped-item bake for the same
      component-rich stack. Arrow/trident carried models (the last picked-up
      entity family) now also draw in the same `ITEM_PICKUP` group: world
      projects the projectile kind (normal/tipped/spectral arrow, foiled
      trident) with the extracted render-state rotations, and the renderer bakes
      `ArrowModel`/`TridentModel` with the vanilla renderer root transform at
      the quadratic-interpolated pickup position, so vanilla's generic
      `EntityRenderState` submit surface is fully covered by its three actual
      consumers.
      Native ravager event `69` now emits the vanilla roar `minecraft:poof`
      burst from the ravager AABB center with gaussian `0.2` velocity and
      applies the vanilla local-authoritative roar knockback to the local player
      when its current pose AABB intersects the ravager AABB inflated by `4.0`.
      Native LivingEntity event `60` now emits the vanilla
      `minecraft:poof` burst from `makePoofParticles`: 20 commands with
      gaussian `0.02` velocity and `getRandomX` / `getRandomY` /
      `getRandomZ` position sampling from the current living entity AABB.
      Native LivingEntity event `67` now emits vanilla drown
      `minecraft:bubble` particles from `makeDrownParticles`: 8 commands at
      entity position plus `random.triangle(0.0, 1.0)` offsets, carrying the
      entity's current `deltaMovement` as velocity.
      Native LivingEntity event `46` now emits vanilla `minecraft:portal`
      particles from `handleEntityEvent`: 128 commands interpolated from
      `xo/yo/zo` to current position with width/height random offsets and
      `(nextFloat()-0.5)*0.2` velocity.
      Native Snowball event `3` now emits vanilla hit particles from
      `Snowball.handleEntityEvent`: 8 commands at the snowball position with
      zero velocity, using `minecraft:item` for the normal synced item stack
      branch and `minecraft:item_snowball` for explicit empty stacks.
      Native ThrownEgg event `3` now emits vanilla hit particles from
      `ThrownEgg.handleEntityEvent`: 8 `minecraft:item` commands for non-empty
      item stacks at the egg position, with `(nextFloat()-0.5)*0.08` velocity
      on each axis; explicit empty stacks emit no particles.
      Native Arrow event `0` now emits vanilla tipped-arrow effect-clear
      particles from `Arrow.handleEntityEvent`: 20 `minecraft:entity_effect`
      commands when synced `ID_EFFECT_COLOR != -1`, with RGB option color,
      zero velocity, and current-arrow AABB position sampling via
      `getRandomX(0.5)`, `getRandomY()`, and `getRandomZ(0.5)`. Color `-1`
      emits no particles; color `0` emits black.
      Native Animal event `18` now emits vanilla love `minecraft:heart`
      particles from `Animal.handleEntityEvent`: 7 commands with gaussian
      `0.02` velocity and current-animal AABB position sampling via
      `getRandomX(1.0)`, `getRandomY()+0.5`, and `getRandomZ(1.0)`.
      Native Allay event `18` now emits the vanilla duplication
      `minecraft:heart` burst from `Allay.handleEntityEvent`: 3 commands with
      the same gaussian `0.02` velocity and current-allay AABB position
      sampling.
      Native TamableAnimal and AbstractHorse events `6`/`7` now emit vanilla
      taming feedback particles from `spawnTamingParticles`: event `7` success
      uses 7 `minecraft:heart` commands and event `6` failure uses 7
      `minecraft:smoke` commands with gaussian `0.02` velocity and current-AABB
      position sampling.
      Native Villager events `12`/`13`/`14`/`42` now emit vanilla
      `addParticlesAroundSelf` bursts: 5 commands with gaussian `0.02`
      velocity and current-villager AABB position sampling with `+1.0` Y
      offset, mapped to `minecraft:heart`, `minecraft:angry_villager`,
      `minecraft:happy_villager`, and `minecraft:splash`.
      Native Dolphin event `38` now emits vanilla `minecraft:happy_villager`
      particles from `Dolphin.handleEntityEvent`: 7 commands with gaussian
      `0.01` velocity and current-dolphin AABB position sampling with `+0.2`
      Y offset.
      Native Fox event `45` now emits vanilla item-eat particles from
      `Fox.handleEntityEvent`: 8 `minecraft:item` commands for a non-empty main
      hand, main-hand item stack option, mouth anchor, and the rotated random
      velocity fan.
      Native HoneyBlock entity events `53`/`54` now emit vanilla
      `minecraft:block` particles from `showParticles`: 5 slide commands for
      base Entity event `53` and 10 jump commands for LivingEntity event `54`,
      both using the honey block default state, entity position, and zero
      velocity.
      Renderer light-descriptor tests also enumerate the vanilla 26.1
      `getLightCoords` override families: full-bright particles, forced
      block-light particles, age-based smooth block emission, portal/enchant
      quartic emission, the `FireflyParticle` light fade, and world-sampled
      counterexamples.
      Renderer lifetime-descriptor tests cover the vanilla 26.1 constructor /
      provider formula families, including base particle lifetime, rising
      particles, cloud scaling, ash smoke scaling/division, crit/random divisor
      formulas, command-option lifetimes, portal/reverse-portal ranges, falling
      dust, dust scale, and inclusive tick seed particles.
      Renderer quad-size curve tests cover every modeled vanilla shape:
      constant, grow-to-base, flame half-shrink, lava full-shrink, flash
      overlay sine size, portal grow curve, reverse-portal shrink curve, and
      shriek growth.
      Renderer alpha/color coverage now keeps each vanilla-shaped curve on an
      explicit descriptor: `SimpleAnimatedParticle` half-lifetime fade,
      firework spark's initial `0.99` alpha plus half-lifetime fade, firework
      flash `OverlayParticle.extract` alpha, shriek extract-time fade,
      vault-connection `LifetimeAlpha`, firefly `getFadeAmount`, EndRod
      half-lifetime fade-color blending, dust color-transition lerp, and
      decoded option / random / fixed provider tints. Native terrain particle
      colors now install vanilla `BlockColors.createDefault()` layer-0 tint for
      terrain particle providers (`0.6 * colorAsTerrainParticle`) and
      non-FallingBlock `falling_dust` (`colorAsTerrainParticle`) for constant,
      default-colormap, redstone power, stem age, and lily pad world-color
      sources; the `falling_dust` mapColor fallback now covers foundational
      static block colors for stone/dirt/planks, wood/log/bamboo axis states,
      wooden stairs/slabs/pressure plates/doors/trapdoors/fences/fence gates/
      signs/hanging signs/shelves, banner/wall banner `WOOD`, button,
      glass/glass pane/iron bars/iron chain/copper bars/copper chains,
      redstone/slime/bone/frosted-ice/dirt-path/petrified-slab misc static
      blocks, ladder/torch/end rod, rail/redstone fixture, skull/head, non-tinted
      potted, cake, air / cave_air / void_air, and test_instance_block default
      `MapColor.NONE` groups, crimson/warped
      stem/hyphae colors, DyeColor / colored terracotta families,
      bed/candle/shulker decorative
      families,
      cave/emissive block families,
      copper weathering families, nether flora / blackstone static families,
      quartz/prismarine/End static families, construction stone/brick static
      families, deepslate construction variants, infested stone CLAY variants,
      resin/pale garden static families, plant/dripstone/moss/root/mud natural
      static families, non-tinted foliage static families, crop/succulent static
      families, produce/fungus static families, utility/mechanical static
      families including stone/weighted pressure plates, utility fixtures,
      functional blocks, and redstone utility/control blocks, aquatic/coral
      static families, bamboo/honey/campfire utility static families, and
      water plant/egg static families, flower/tall flower static families,
      fire/cocoa/creaking heart static families, glowstone/enchanting/beacon
      static families, and
      ore/deepslate/nether plus
      mineral/natural static block families, plus the final accepted vanilla
      static states covered by the registry-wide falling-dust color test.
      Terrain particles and non-FallingBlock `falling_dust` now also sample
      biome-aware `BlockColors` at each actual spawn position using vanilla's
      `BlockPos.containing(x, y, z)` owner: packet-count random spawns query
      their generated position through the world probe before native emits
      `ParticleSpawnCommand.option_color`. Firework non-empty explosion spark
      fade-colors are now threaded as per-command fade targets, and
      trail/twinkle child behavior is handled by the renderer runtime. Native
      firework batches now also queue the vanilla life-0 local ambient blast /
      large_blast sound, including far variants, volume, pitch range, seed, and
      distance-delay metadata, plus delayed twinkle / twinkle_far sounds after
      the vanilla Starter lifetime using release-time camera distance for the
      far variant. FireworkRocketEntity client tick trail now submits one
      sprite-backed `minecraft:firework` particle per advanced client tick at
      the rocket's current world transform, with vanilla gaussian X/Z speed
      `*0.05` and Y speed `-deltaMovement.y*0.5`. OminousItemSpawner client
      tick now mirrors vanilla `level.gameTime % 5 == 0` by queuing
      `random.nextIntBetweenInclusive(1,3)` sprite-backed
      `minecraft:ominous_spawning` particles at the spawner position, with
      velocity `0.4*(gaussian-gaussian)` on each axis and the particle type's
      override-limiter behavior.
    - Advances age-selected particle sprites with vanilla
      `SpriteSet.get(index, max)` shape (`index * (sprites.size() - 1) / max`),
      keeps random-selected sprites stable after intake, and preserves missing
      definition / missing sprite / unknown particle diagnostics through native
      spawn resolution and renderer counters.
    - Applies vanilla `ClientLevel.doAddParticle` thinning for
      `ClientboundLevelParticlesPacket` spawns: non-override particles beyond
      camera distance squared `1024.0` are dropped, override-limiter particles
      bypass distance and particle-status filtering, and CLI `--client-particles`
      drives `ParticleStatus.ALL` / `DECREASED` / `MINIMAL` with vanilla-shaped
      `nextInt(3)`, `alwaysShow && MINIMAL` `nextInt(10)` promotion, and the
      second decreased `nextInt(3)` drop. The camera position comes from the
      same eye-position projection used by global level-event audio.
    - Enforces vanilla `ParticleLimit.SPORE_BLOSSOM` (`1000`) for
      `SuspendedParticle.SporeBlossomAirProvider`: over-limit
      `minecraft:spore_blossom_air` spawns are rejected without evicting the
      accepted particles, expiration releases the count, and diagnostics keep
      limited drops separate from the 16384 active-particle queue. Its renderer
      descriptor now also mirrors the vanilla random sprite, `y - 0.125`
      initial position, `(0, -0.8, 0)` initial velocity, `0.6..1.2`
      quad-size multiplier, `500..1000` overridden lifetime after consuming the
      constructor lifetime draw, `[0.32, 0.5, 0.22]` color, `0.01` gravity,
      `1.0` friction, no-physics metadata, and opaque particle layer.
    - Emits renderer spawn batches for simple vanilla `LevelEventHandler`
      side effects:
      - event `1500`: ten `minecraft:composter` particles using vanilla
        `ComposterBlock.handleFill` position/velocity formulas, including the
        loaded block shape's center-column max-Y for composter fill height and a
        vanilla full-block/unknown fallback
      - event `1501`: eight `minecraft:large_smoke` particles above lava
        extinguish
      - event `1502`: five `minecraft:smoke` particles inside redstone torch
        burnout
      - event `1503`: sixteen `minecraft:smoke` particles above end portal
        frame fill
      - event `1504`: pointed-dripstone drip particles, using the loaded
        client block state to validate a downward, unwaterlogged `tip`, find
        the root within vanilla's 11-block search, sample the root-above fluid,
        apply mud-as-water outside water-evaporating dimensions, fall back to
        the built-in dimension default (`water` outside the Nether, `lava` in
        the Nether), and submit the vanilla XZ-offset drip position
      - event `1505`: plant-growth `minecraft:happy_villager` particles for
        vanilla `BoneMealItem.addGrowthParticles` branches, including
        BonemealableBlock grower/in-block emission, rooted-dirt and
        mangrove-leaves below-position grower particles using loaded
        block-state shape max-Y spread height, water and neighbor-spreader wide
        spread (`count * 3`, `spreadWidth=3.0`, `spreadHeight=1.0`), and the
        `allowFloatingParticles=false` 7x7 support-layer non-air filter,
        followed by the vanilla `minecraft:item.bone_meal.use` sound after the
        growth particle RNG sequence
      - event `2000`: ten directionally emitted `minecraft:smoke` particles
      - event `2010`: ten directionally emitted `minecraft:white_smoke`
        particles
      - events `2001` / `3008`: vanilla `ClientLevel.addDestroyBlockEffect`
        density grid submissions for event-data block-state ids, using native
        block-outline boxes when the state shape is known and full-cube fallback
        otherwise, with definition-less `minecraft:block` commands carrying
        `BlockParticleOption` metadata and skipping air plus
        `shouldSpawnTerrainParticles=false` block states; unlike
        `TerrainParticle.Provider`, vanilla `addDestroyBlockEffect` does not
        reject `moving_piston`. Event `3008` also records/plays the vanilla
        brushable completion sound for suspicious sand/gravel using
        `SoundSource.PLAYERS`, volume `1.0`, and pitch `1.0`; non-brushable
        event-data states do not emit a sound. These block particles now sample
        vanilla terrain tint at the event block position; any remaining
        terrain/block particle GPU presentation refinements stay with broader
        terrain/block particle work
      - events `2002` / `2007`: eight `minecraft:item` splash-potion break
        particles with vanilla `ItemParticleOption(Items.SPLASH_POTION)`,
        center position, and gaussian/upward velocity, followed by 100
        vanilla-positioned `minecraft:effect` / `minecraft:instant_effect`
        spell particles with event-data RGB, random brightness, and
        `SpellParticleOption` power, then
        `minecraft:entity.splash_potion.break` after the particle RNG sequence
      - events `2011` (`PARTICLES_BEE_GROWTH`) and `2012`
        (`PARTICLES_TURTLE_EGG_PLACEMENT`): vanilla
        `ParticleUtils.spawnParticleInBlock`-shaped `minecraft:happy_villager`
        particles, using loaded block-state shape max-Y for spread height, event
        `data` as count, and gaussian `0.02` velocity
      - event `2013`: vanilla
        `ParticleUtils.spawnSmashAttackParticles` dust-pillar submissions using
        event `data` for the two float-bounded loop counts, event-position
        block state as `BlockParticleOption(ParticleTypes.DUST_PILLAR, state)`,
        and `TerrainParticle.DustPillarProvider` rejection for air,
        `moving_piston`, and `shouldSpawnTerrainParticles=false` states after
        the position/velocity random draws are consumed
      - event `2003`: eight `minecraft:item` ender-eye break particles with
        vanilla `ItemParticleOption(Items.ENDER_EYE)`, center position, and
        gaussian/upward velocity, followed by the vanilla portal ring
      - event `2004`: twenty paired `minecraft:smoke` and `minecraft:flame`
        particles around the block center
      - event `2006`: 200 vanilla-positioned `minecraft:dragon_breath`
        particles with `PowerParticleOption`-shaped velocities, followed by
        `minecraft:entity.dragon_fireball.explode` after the particle RNG
        sequence when `data == 1`
      - event `2008`: one centered `minecraft:explosion` particle
      - event `2009`: eight `minecraft:cloud` particles above the block
      - event `3000`: one always-visible centered
        `minecraft:explosion_emitter` particle
      - event `3002`: vanilla axis or block-face
        `minecraft:electric_spark` particles
      - event `3003`: vanilla block-face `minecraft:wax_on` particles,
        followed by `minecraft:item.honeycomb.wax_on` after the six-face
        `UniformInt.of(3,5)` particle RNG sequence
      - event `3004`: vanilla block-face `minecraft:wax_off` particles
      - event `3005`: vanilla block-face `minecraft:scrape` particles
      - event `3006`: the charged `count = data >> 6`, `count > 0`
        branch now emits vanilla block-face `minecraft:sculk_charge`
        particles with `UniformInt.of(0, count)` repetition, full-block
        six-face vs `MultifaceBlock.unpack(data & 63)` face selection,
        `0.65` / `0.57` / `0.35` step factors, `+-0.005` speed supplier,
        and `SculkChargeParticleOptions` roll; the `count == 0` branch now
        emits vanilla `minecraft:sculk_charge_pop` submissions using the target
        block's full-shape context for `40` particles / `0.45` spread or
        non-full/unknown context for `20` particles / `0.25` spread, with
        `0.07` velocity scale
      - event `3009`: vanilla block-face `minecraft:egg_crack` particles
      - event `3011`: trial spawner smoke plus normal/ominous flame spawn
        particles
      - event `3012`: trial spawner mob-spawn sound-side smoke plus
        normal/ominous flame spawn particles
      - event `3013`: normal trial spawner detected-player particles
      - event `3014`: trial spawner eject-item sound-side
        `minecraft:small_flame` and `minecraft:smoke` particles
      - event `3015`: vault activation `minecraft:smoke` plus
        `minecraft:small_flame` / `minecraft:soul_fire_flame` particles from
        `randomPosInsideCage`, gated on a loaded vault block entity at the event
        position. Loaded vaults now also parse `shared_data.connected_players`
        and `connected_particles_range`, resolve in-range loaded player entities
        by UUID, and emit vanilla `minecraft:vault_connection` particles from
        the facing keyhole position before the cage particles using
        `Mth.nextInt(random, 2, 5)` and `Vec3.offsetRandom(random, 1.0)`.
        The event is followed by the vanilla distance-delayed
        `minecraft:block.vault.activate` sound using the post-particle random
        pitch sequence; broader idle/tick block-entity client effects remain
        deferred.
      - event `3016`: vault deactivation `minecraft:small_flame` /
        `minecraft:soul_fire_flame` particles from `randomPosCenterOfCage`,
        with gaussian `0.02` velocity, followed by the vanilla
        distance-delayed `minecraft:block.vault.deactivate` sound using the
        post-particle random pitch sequence
      - event `3017`: trial spawner eject-item `minecraft:small_flame` and
        `minecraft:smoke` particles
      - event `3018`: cobweb-place `minecraft:poof` particles
      - event `3019`: ominous trial spawner detected-player particles
      - event `3020`: ominous trial spawner activation detected-player,
        `minecraft:trial_omen`, and `minecraft:soul_fire_flame` particles
      - event `3021`: trial spawner item-spawn sound-side smoke plus
        normal/ominous flame spawn particles
    - LevelEvent item-particle branches now submit definition-less
      `minecraft:item` commands with item-template option metadata and resolve
      empty-component splash-potion / ender-eye break particles through the
      installed default GROUND item material sprite ids. Native item atlas
      upload supplies the item sprite UV catalog the renderer draw path needs.
    - Advances CPU-side common particles.
    - Samples vanilla-shaped curves for common particle providers:
      - size
      - color
      - age-size
      - `DragonBreathParticle.Provider` purple color range, 0.75 size scale,
        grow-to-base size curve, lifetime, friction, no-physics metadata, and
        the vanilla special tick motion: unchanged Y position speeds up X/Z by
        `1.1` before friction, while Y velocity is only damped after `onGround`
        sets the persistent `hasHitGround` branch and adds `0.002` upward drift
      - `SuspendedTownParticle.HappyVillagerProvider`,
        `ComposterFillProvider`, plain `Provider` for `mycelium`, and
        `EggCrackProvider`: random sprite selection, provider tint,
        suspended-town size scaling, provider initial-speed transform,
        lifetime, friction metadata, and the vanilla collision-free
        `SuspendedTownParticle.move` override
      - `SuspendedTownParticle.DolphinSpeedProvider` blue tint, random alpha,
        suspended-town size scaling, provider initial-speed transform, half
        lifetime, friction metadata, and the vanilla collision-free
        `SuspendedTownParticle.move` override
      - `HeartParticle.Provider` random sprite selection, fixed lifetime,
        grow-to-base size curve, 1.5 quad-size scale, initial y-speed offset,
        friction, no-physics, and blocked-y speed-up metadata
      - `HeartParticle.AngryVillagerProvider` inherits heart behavior and
        applies the vanilla initial `y + 0.5` provider position offset
      - `PortalParticle.Provider` random sprite selection,
        `0.1 * (random * 0.2 + 0.5)` quad size, brightness-derived
        `[0.9, 0.3, 1.0]` RGB scaling, `40..49` lifetime,
        `1 - (1-progress)^2` render-size curve, start-position tick path, and
        `(age / lifetime)^4` smooth block-light emission, while preserving
        vanilla `hasPhysics=true` metadata plus the collision-free `move`
        override
      - `ReversePortalParticle.ReversePortalProvider` inherits portal random
        sprite/color setup, applies the vanilla `1.5` quad-size multiplier,
        consumes the parent portal lifetime draw before overriding lifetime to
        `60..61`, uses the `1 - progress / 1.5` render-size curve, moves by
        incremental age-scaled velocity, preserves vanilla `hasPhysics=true`
        metadata plus the collision-free `move` override, and inherits quartic
        smooth block emission
      - `NoteParticle.Provider` command-x hue color formula, fixed lifetime,
        grow-to-base size curve, 1.5 quad-size scale, initial y-speed offset,
        friction, and blocked-y speed-up metadata
      - `LavaParticle.Provider` random sprite selection, constructor-random
        horizontal velocity damped by `0.8`, random upward velocity
        `0.05..0.45`, `0.2..2.2` quad-size scaling, vanilla
        `1 - progress^2` shrinking size curve, `16 / (random * 0.8 + 0.2)`
        lifetime, `0.999` friction, `0.75` gravity, physics metadata,
        full-bright block light, and child smoke emission. Lava spawn commands
        carry the pack-backed smoke child
        SpriteSet, and runtime emits smoke after lava ticks when
        `random.nextFloat() > age / lifetime`
      - `CampfireSmokeParticle.CosyProvider` and `SignalProvider` use random
        sprite selection, constructor `scale(3.0)`, fixed alpha `0.9` / `0.95`,
        `80..129` / `280..329` lifetime, command x/z velocity plus
        `yAux + random.nextFloat() / 500.0`, gravity `3.0E-6`, physics metadata,
        translucent particle layer, vanilla `0.25` x `0.25` collision size,
        vanilla random x/z drift, collision-backed `move`, alpha-`<= 0`
        pre-motion removal, and alpha fade during the final 60 ticks.
      - `SnowflakeParticle.Provider` age sprite selection, fixed pale-blue tint,
        `0.1 * (random * random + 1.0)` quad size, command velocity plus
        random `+-0.05` per axis, `16 / (random * 0.8 + 0.2) + 2` lifetime,
        `1.0` friction, `0.225` gravity, physics metadata, opaque particle
        layer, and vanilla post-tick damping (`xd *= 0.95`, `yd *= 0.9`,
        `zd *= 0.95`)
      - `DripParticle.NectarFallProvider` and
        `SporeBlossomFallProvider` use random sprite selection, zero initial
        velocity, DripParticle opaque layer, physics metadata, direct gravity
        motion with `0.98` friction, fixed tints `[0.92, 0.782, 0.72]` /
        `[0.32, 0.5, 0.22]`, gravity `0.007` / `0.005`, and lifetimes
        `16 / (random * 0.8 + 0.2)` / `64 / randomBetween(0.1, 0.9)`.
        Renderer ticks now use the world collision callback for their vanilla
        `move` path and remove the particle when `onGround` becomes true.
      - `DripParticle.HoneyHangProvider`, `HoneyFallProvider`, and
        `HoneyLandProvider` use random sprite selection, zero initial velocity,
        DripParticle opaque layer, physics metadata, fixed honey tints,
        `0.98` friction, direct gravity motion, hang-particle `0.02`
        post-move damping, lifetimes `100`,
        `64 / (random * 0.8 + 0.2)`, and
        `128 / (random * 0.8 + 0.2)`, with gravity `0.000012`, `0.01`,
        and `0.06`. The falling provider now removes on `onGround` through the
        collision-backed `move` path. The landing provider now uses the
        collision-backed `DripParticle` move/friction path without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates. Falling honey ground hits
        now emit renderer particle sound events for
        `minecraft:block.beehive.drip` with `SoundSource.BLOCKS`, pitch `1.0`,
        and vanilla `0.3..1.0` volume range; native drains those events after
        particle tick and submits positioned audio commands.
      - `DripParticle.ObsidianTearHangProvider`,
        `ObsidianTearFallProvider`, and `ObsidianTearLandProvider` use random
        sprite selection, zero initial velocity, DripParticle opaque layer,
        physics metadata, fixed purple tint, `0.98` friction, direct gravity
        motion, hang-particle `0.02` post-move damping, glowing block-light
        override, lifetimes `100`, `64 / (random * 0.8 + 0.2)`, and
        `28 / (random * 0.8 + 0.2)`, with gravity `0.000012`, `0.01`, and
        `0.06`. The falling provider now removes on `onGround` through the
        collision-backed `move` path. The landing provider now uses the
        collision-backed `DripParticle` move/friction path without
        `WaterDropParticle`'s random on-ground removal branch. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates.
      - `DripParticle.LavaHangProvider`, `LavaFallProvider`, and
        `LavaLandProvider` use random sprite selection, zero initial velocity,
        DripParticle opaque layer, physics metadata, `0.98` friction, direct
        gravity motion, non-glowing world light, and lifetimes `40`,
        `64 / (random * 0.8 + 0.2)`, and
        `16 / (random * 0.8 + 0.2)`, with gravity `0.0012`, `0.06`, and
        `0.06`. The hang provider starts at the vanilla default white and
        updates RGB during runtime with `CoolingDripHangParticle.preMoveUpdate`
        before applying hang-particle `0.02` post-move damping. Hang-to-fall
        and fall-to-land child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates. The falling provider now removes on
        `onGround` through the collision-backed `move` path, the landing
        provider now uses collision-backed `DripParticle` move/friction without
        `WaterDropParticle`'s random on-ground removal branch, and lava
        providers now remove when their containing block's matching lava fluid
        surface contains the particle.
      - `DripParticle.WaterHangProvider` and `WaterFallProvider` use random
        sprite selection, zero initial velocity, DripParticle opaque layer,
        physics metadata, fixed blue tint, non-glowing world light, `0.98`
        friction, direct gravity motion, hang-particle `0.02` post-move
        damping, and lifetimes `40` and
        `64 / (random * 0.8 + 0.2)`, with gravity `0.0012` and `0.06`.
        Hang-to-fall and fall-to-splash child spawning now use vanilla
        lifetime/on-ground triggers through renderer child templates. The falling provider
        now removes on `onGround` through the collision-backed `move` path, and
        water providers now remove when their containing block's matching water
        fluid surface contains the particle.
      - `DripParticle.DripstoneLavaHangProvider`,
        `DripstoneLavaFallProvider`, `DripstoneWaterHangProvider`, and
        `DripstoneWaterFallProvider` use random sprite selection, zero initial
        velocity, DripParticle opaque layer, physics metadata, non-glowing
        world light, `0.98` friction, direct gravity motion, hang-particle
        `0.02` post-move damping, lava cooling hang RGB runtime formula, water
        fixed blue tint, lava falling tint, hang lifetime `40`, falling
        lifetime `64 / (random * 0.8 + 0.2)`, hang gravity `0.0012`, and
        falling gravity `0.06`. The falling providers now remove on `onGround`
        through the collision-backed `move` path and remove when matching
        water/lava fluid surfaces contain the particle. Hang-to-fall and
        fall-to-land/splash child spawning now use vanilla lifetime/on-ground
        triggers through renderer child templates. Dripstone fall-and-land
        ground hits now emit renderer particle sound events for
        `minecraft:block.pointed_dripstone.drip_lava` /
        `minecraft:block.pointed_dripstone.drip_water` with
        `SoundSource.BLOCKS`, pitch `1.0`, and vanilla `0.3..1.0` volume range;
        native drains those events after particle tick and submits positioned
        audio commands.
      - `SuspendedParticle.CrimsonSporeProvider` and `WarpedSporeProvider`
        use random sprite selection, `y - 0.125` initial position,
        `0.6..1.2` quad-size multiplier, `16 / (random * 0.8 + 0.2)`
        lifetime, no physics, `1.0` friction, zero gravity, and opaque
        particle layer. Crimson spores use gaussian micro-drift and
        `[0.9, 0.4, 0.5]` tint; warped spores use downward random drift and
        `[0.1, 0.1, 0.3]` tint.
      - `SquidInkParticle.Provider` and `GlowInkProvider` age sprite selection,
        fixed `0.5` quad size, black / glow-ink tint, command velocity,
        `6 / (random * 0.8 + 0.2)` lifetime, `0.92` friction, zero gravity, and
        no-physics metadata plus full-bright light coords, translucent particle
        layer, and `SimpleAnimatedParticle` half-lifetime alpha fade updated on
        runtime ticks and reused during vertex emission; runtime ticks now
        apply the vanilla post-`super.tick()` `yd -= 0.0074F` downward drift
        when the post-move world block sample is air.
      - `SimpleVerticalParticle.PauseMobGrowthProvider` and
        `ResetMobGrowthProvider` random sprite selection, random `0.5..1.1`
        quad-size scaling, fixed lifetime `8`, command velocity with
        `-0.03` / `+0.03` y offset, default `0.98` friction, zero gravity, and
        physics metadata plus vanilla opaque particle layer
      - `PlayerCloudParticle.Provider` vanilla constructor-random initial
        speed plus command velocity, post-tick local-player Y / Y-velocity
        pull within 2 blocks when above the player's feet, and `SneezeProvider`
        fixed green tint / alpha override on the player-cloud curve
      - `SmokeParticle.Provider`, `LargeSmokeParticle.Provider`, and
        `WhiteSmokeParticle.Provider` share the vanilla `BaseAshSmokeParticle`
        constructor-random initial speed scaled by `0.1` plus command velocity
        (the same velocity shape as the player-cloud providers, from
        `super(level, x, y, z, 0, 0, 0, ...)` then `xd *= 0.1` and `xd += xa`),
        age sprite selection, `0.75 * scale` quad size with the vanilla `x32`
        grow-to-base curve, random gray (`smoke` / `large_smoke`) or fixed
        `0xBAB1C2` (`white_smoke`) tint, `8 / (random * 0.8 + 0.2) * scale`
        lifetime (`scale` `1.0` for `smoke` / `white_smoke`, `2.5` for
        `large_smoke`), `0.96` friction, `-0.1` gravity, physics metadata, and
        the vanilla `speedUpWhenYMotionIsBlocked` X/Z speed-up when Y movement
        is blocked by the world collision callback.
      - `CritParticle.Provider`, `DamageIndicatorProvider`, and
        `MagicProvider` constructor-random initial speed scaled by `0.1` plus
        `0.4` command velocity, the damage-indicator `yAux + 1.0` offset,
        crit lifetime or fixed damage lifetime, grow-to-base size curve,
        initial random gray / magic color multipliers, `0.7` friction, `0.5`
        gravity, no-physics metadata, and the vanilla constructor-time `tick()`
        that advances age/position and damps velocity before the particle
        enters the runtime list
      - `FlameParticle.Provider` and `SmallFlameProvider` use random sprites,
        rising lifetime, flame render-size curve, smooth block-light emission,
        vanilla `hasPhysics=true` metadata, and the collision-free `move`
        override
      - `BubbleParticle.Provider` command velocity scaled by `0.2` plus
        random `+-0.02` velocity, random `0.2..0.8` quad-size scaling,
        `8 / (random * 0.8 + 0.2)` lifetime, `0.85` friction, and upward
        `+0.002` y-velocity tick behavior represented as negative gravity.
        Runtime ticks now remove the particle when the containing block's fluid
        state is not water.
      - `BubbleColumnUpParticle.Provider` shares the bubble velocity and
        quad-size formulas while using `40 / (random * 0.8 + 0.2)` lifetime,
        `-0.125` gravity, and `0.85` friction. Runtime ticks now remove the
        particle when the containing block's fluid state is not water.
      - `WaterCurrentDownParticle.Provider` uses random sprite selection,
        fixed initial velocity `(0, -0.05, 0)`, `30 + random.nextFloat() * 60`
        lifetime, random `0.2..0.8` quad-size scaling, opaque particle layer,
        no-physics metadata, gravity `0.002`, and the vanilla swirl tick
        formula (`xd += 0.6*cos(angle)`, `zd += 0.6*sin(angle)`, horizontal
        damping `0.07`, `angle += 0.08`). Runtime ticks now use the vanilla
        no-physics `move` path and remove the particle when the containing
        block's fluid state is not water.
      - `FlyTowardsPositionParticle.EnchantProvider` and `NautilusProvider`
        use random sprite selection, command velocity, initial render position
        at `spawn + velocity` with the original spawn position retained as the
        curve start, `0.1 * (random * 0.5 + 0.2)` quad size, brightness-derived
        `[0.9, 0.9, 1.0]` RGB scaling, `30..39` lifetime, opaque particle
        layer, no-physics metadata, the vanilla fly-towards position curve
        (`pos = 1 - age/lifetime`, `y -= (age/lifetime)^4 * 1.2`), and
        quartic smooth block-light emission
      - `FlyTowardsPositionParticle.VaultConnectionProvider` reuses the
        fly-towards position curve with vanilla `scale(1.5)`, translucent
        layer, full-block glowing light, and `LifetimeAlpha(0.0, 0.6, 0.25,
        1.0)` applied at runtime tick and partial-tick vertex emission
      - `TotemParticle.Provider` uses age sprite selection, command velocity,
        `0.75` quad-size multiplier, `60 + random.nextInt(12)` lifetime,
        translucent particle layer, `0.6` friction, `1.25` gravity, full-bright
        light coords, both vanilla random color branches, and the
        `SimpleAnimatedParticle` half-lifetime alpha fade
      - Simple `SpellParticle.Provider` particles (`infested`, `raid_omen`,
        `trial_omen`) use the vanilla random horizontal constructor velocity,
        y-velocity scaling, still-horizontal x/z dampening, age sprite
        selection, `0.75` quad-size scale, `8 / (random * 0.8 + 0.2)`
        lifetime, `0.96` friction, `-0.1` gravity, no physics, and
        blocked-y speed-up metadata. Option-colored spell particles now carry
        decoded command metadata: `effect` / `instant_effect` use
        `SpellParticle.InstantProvider` RGB color plus `setPower(power)`, and
        `entity_effect` uses `SpellParticle.MobEffectProvider` ARGB color /
        alpha. Spell particles now also track vanilla `originalAlpha` and
        sample native local-player scope context so particles within distance
        squared `9.0` of a first-person spyglass user render with alpha `0.0`,
        then lerp back toward the provider alpha by `0.05` when the scope gate
        clears. `flash` now maps to `FireworkParticles.FlashProvider` with
        decoded ARGB color, fixed lifetime `4`, translucent layer, and the
        vanilla overlay size / render-alpha formulas. `firework` now maps to
        `FireworkParticles.SparkProvider` with age sprites, vanilla
        `SimpleAnimatedParticle` friction `0.91`, gravity `0.1`, full-bright
        light, translucent layer, command velocity, `0.75` quad-size scale,
        fixed initial alpha `0.99`, `48 + random.nextInt(12)` lifetime, and
        the half-lifetime alpha fade formula. Firework rocket entity event `17`
        with empty/no explosions now emits vanilla `minecraft:poof` particles;
        non-empty explosions now project `FireworkParticles.Starter` small /
        large ball, star, creeper, and burst base spark shapes, center `flash`,
        per-spark fade-colors, trail child spark duplication, and twinkle
        visibility gating, plus the life-0 blast / large_blast local ambient
        sound and delayed twinkle / twinkle_far local ambient sound.
        `trail` now maps to `TrailParticle.Provider` with decoded target / RGB
        color / duration, vanilla random color scaling, target interpolation,
        full-bright light, and opaque layer. `vibration` now maps block
        `PositionSource` options to `VibrationSignalParticle.Provider` with
        decoded target center / arrival ticks, random sprite selection, fixed
        `0.3` quad size,
        translucent layer, full-block light, target interpolation, vanilla
        yaw/pitch/sway state, and the two rotated quads from
        `VibrationSignalParticle.extract`. Entity `PositionSource` options now
        preserve entity id / `y_offset`, and native level-particle command
        resolution queries the current world entity transform to seed
        `option_target=entity.position + (0, y_offset, 0)` when the entity is
        loaded; renderer particle ticks now receive native entity target
        contexts, refresh the vibration target each tick, and remove the
        particle when the source entity is missing.
        `sculk_charge` now carries decoded `SculkChargeParticleOptions.roll`
        through to billboard roll rotation.
        `trial_spawner_detection` and `_ominous` now map to
        `TrialSpawnerDetectionParticle.Provider` with age sprites, the
        `BaseAshSmokeParticle` initial velocity (`super(..., 0, 0, 0, ...)`
        normalized base spread, then `xd *= 0.0; yd *= 0.9; zd *= 0.0` and the
        command velocity added straight through with no offset — the x/z base
        spread is dropped while the upward y drift is scaled by `0.9`),
        `scale(1.5)` over the vanilla `0.75` single-quad scale,
        `12 / (0.5 + random * 0.5)` lifetime, opaque layer, full-block light,
        grow-to-base size curve, physics metadata, and
        `SingleQuadParticle.FacingCameraMode.LOOKAT_Y` represented as a
        per-instance facing mode whose vertex transform keeps world-Y up.
        `dust_plume` now maps to `DustPlumeParticle.Provider` with age
        sprites, `BaseAshSmokeParticle` `0.75` quad-size scale,
        `7 / (random * 0.8 + 0.2)` lifetime, the `BaseAshSmokeParticle`
        initial velocity (`super(..., 0.7F, 0.6F, 0.7F, xa, ya + 0.15F, za,
        ...)`): the `Particle` 7-arg normalized base spread scaled per axis
        by `(0.7, 0.6, 0.7)` plus the command velocity with a `0.15` y
        offset, opaque layer, no-physics metadata, `0.5` initial gravity,
        `0.96` friction, `ARGB(0xBAB1C2) - random * 0.2` tint,
        grow-to-base size curve, and the provider tick override that decays
        gravity by `0.88` and friction by `0.92` before default motion.
        `rain` and `splash` now map to `WaterDropParticle.Provider` /
        `SplashParticle.Provider` with random sprites, vanilla single-quad
        size, `8 / (random * 0.8 + 0.2)` lifetime, opaque layer, physics
        metadata, `0.98` friction, direct gravity motion (`yd -= gravity`),
        and water-drop damping. `rain` preserves the constructor random x/z
        velocity damped by `0.3`, `0.1..0.3` y velocity, and `0.06`
        gravity; `splash` uses `0.04` gravity and the horizontal command
        branch `(xa, 0.1, za)`. Runtime ticks now use the collision-backed
        `move` path and apply vanilla `onGround` 50% random removal plus X/Z
        ground damping. Block/fluid in-block removal now queries the world
        surface height as `max(collisionShape.max(Y, localX, localZ),
        fluidState.height)` and removes `rain` / `splash` below that surface.
        `fishing` now maps to `WakeParticle.Provider` with first sprite
        initialization, vanilla single-quad size, `8 / (random * 0.8 + 0.2)`
        lifetime, command velocity, opaque layer, physics metadata, `0.98`
        friction, zero gravity, `setSize(0.01F, 0.01F)` collision bounds,
        collision-backed `move`, damping, and the vanilla wake sprite cycle using
        `SpriteSet.get((60 - lifetime) % 4, 4)` during ticks.
        `ominous_spawning` now maps to
        `FlyStraightTowardsParticle.OminousSpawnProvider` with random sprites,
        command velocity, initial position at `spawn + velocity` while keeping
        `spawn` as the interpolation start, vanilla
        `0.1 * (random * 0.5 + 0.2)` quad size followed by
        `scale(randomBetween(3, 5))`, `25 + random * 5` lifetime, opaque
        layer, no-physics metadata, full-block light, and the straight-toward
        tick path plus `ARGB.srgbLerp` from `0xFF45AEFE` to white.
        `firefly` now maps to `FireflyParticle.FireflyProvider` with random
        sprites, vanilla `200..300` inclusive lifetime, initial alpha `0`,
        translucent layer, `speedUpWhenYMotionIsBlocked`, `0.96` friction,
        provider aux velocity (`0.5 - random.nextDouble()` x/z and signed
        `yAux`) through the vanilla `Particle` constructor followed by `*0.8`,
        `0.75 * scale(1.5)` quad-size path, first-tick / 5% random speed
        reroll, alpha fade (`0.3` / `0.5`) and direct smooth block-light fade
        (`0.1` / `0.3`). Runtime ticks now use the collision-backed vanilla
        `super.tick()` move path and remove when the post-move world block
        sample is not air.
        `cherry_leaves`, `pale_oak_leaves`, and `tinted_leaves` now map to
        `FallingLeavesParticle.CherryProvider` / `PaleOakProvider` /
        `TintedLeavesProvider` with random sprites, fixed `300` lifetime,
        opaque layer, `1.0` friction, physics metadata,
        `scale * (0.05 | 0.075)` quad-size choice, cherry flow-away
        parameters `(fall=0.25, side=2.0, startVelocity=0.0)`, pale/tinted
        swirl parameters `(fall=0.07, side=10.0, startVelocity=0.021)`,
        tinted `ColorParticleOption` ARGB command decode with renderer RGB
        tinting, default particle alpha preservation, gravity
        `fallAcceleration * 1.2 * 0.0025`, flow/swirl acceleration, and roll
        spin acceleration. Runtime ticking now routes the leaf move through the
        world collision callback, removes on `onGround`, removes on horizontal
        axis blocking after the first tick, and preserves the vanilla first-tick
        horizontal-block grace.
        `dust` / `dust_color_transition` now map to their vanilla providers
        with decoded RGB colors, transition target color, clamped scale,
        scale-shaped quad size / lifetime, random color variation, age sprites,
        opaque layer, and transition partial-tick color lerp. Other non-spell
        option-driven atlas rendering and provider-specific option effects
        remain separate follow-up work.
      - `SpellParticle.WitchProvider` reuses the simple spell motion/lifetime
        metadata and applies the vanilla shared random magenta brightness
        (`0.35..0.85` for red and blue, zero green)
      - `GlowParticle.WaxOnProvider`, `WaxOffProvider`, `ScrapeProvider`, and
        `ElectricSparkProvider` use vanilla command-scaled velocity,
        fixed or random-choice tint, age sprite selection, `0.75` quad-size
        scale, provider lifetime ranges, `0.96` friction, no physics, and
        blocked-y speed-up metadata; `GlowParticle.getLightCoords` smooth
        block emission is represented in particle light descriptors
      - `GlowParticle.GlowSquidProvider` uses vanilla random horizontal plus
        `yAux` constructor velocity, still-horizontal command x/z dampening,
        random-choice cyan/green tint, age sprite selection, `0.75`
        quad-size scale, `8 / (random * 0.8 + 0.2)` lifetime, `0.96`
        friction, no physics, blocked-y speed-up metadata, and the same smooth
        glow emission curve
      - `SoulParticle.Provider` and `EmissiveProvider` use vanilla
        `RisingParticle` position jitter, `constructor * 0.01 + aux` velocity,
        `alpha=1`, `1.5` quad-size scale, age sprite selection, rising lifetime,
        `0.96` friction, gravity `0`, physics metadata, and `EmissiveProvider`
        full-bright block override
      - `HugeExplosionParticle.Provider` (`minecraft:explosion`) uses vanilla
        xAux-derived quad size, random gray tint, age sprite selection,
        `6 + random.nextInt(4)` lifetime, static zero velocity in the current
        CPU model, base physics metadata, and full-bright light coords
      - `HugeExplosionSeedParticle.Provider` (`minecraft:explosion_emitter`)
        is represented as a definition-less no-render emitter with fixed
        lifetime `8`, six child `minecraft:explosion` submissions each tick,
        `nextDouble() - nextDouble()` offsets scaled by `4.0`, and vanilla
        child xAux `age / lifetime` feeding the explosion quad-size formula.
      - `SonicBoomParticle.Provider` uses vanilla fixed `1.5` quad size,
        random gray tint inherited from `HugeExplosionParticle`, age sprite
        selection, fixed lifetime `16`, static zero velocity in the current CPU
        model, base physics metadata, and inherited full-bright light coords
      - `GustParticle.Provider` and `SmallProvider` use vanilla fixed quad
        sizes (`1.0` and scaled `0.15`), age sprite selection,
        `12 + random.nextInt(4)` lifetime, static zero velocity in the current
        CPU model, base physics metadata, and full-bright light coords
      - `GustSeedParticle.Provider` for `gust_emitter_large` /
        `gust_emitter_small` is represented as a definition-less no-render
        emitter with vanilla constructor parameters `(3.0, 7, 0)` /
        `(1.0, 3, 2)`, inclusive tick lifetime, three child `minecraft:gust`
        submissions on `age % (delay + 1) == 0`, `nextDouble() -
        nextDouble()` offsets scaled by the provider scale, and vanilla child
        xAux `age / lifetime`; `GustParticle.Provider` ignores that xAux just
        like vanilla.
      - `ElderGuardianParticle.Provider` for `minecraft:elder_guardian` is
        accepted as a definition-less special particle, records fixed lifetime
        `30`, zero aux/motion/gravity provider metadata, the translucent
        `entityTranslucent(textures/entity/guardian/guardian_elder.png)`
        render-type intent, and `ParticleRenderType.ELDER_GUARDIANS`; the
        atlas billboard submission path explicitly skips non-`SINGLE_QUADS`
        groups, while the particle target now renders the bind-pose elder
        guardian model after single-quads through the entity translucent
        pipeline with vanilla alpha, camera-relative transform, full-bright
        light, and no overlay.
      - `SculkChargePopParticle.Provider` uses vanilla command velocity,
        `alpha=1`, base quad size, age sprite selection,
        `6 + random.nextInt(4)` lifetime, `0.96` friction, and no-physics
        metadata plus full-bright block override and the translucent particle
        layer
      - `SculkChargeParticle.Provider` carries decoded
        `SculkChargeParticleOptions.roll` into initial `oRoll` / `roll`, and
        billboard vertex emission applies the vanilla roll transform.
    - Uploads a stitched official particle atlas when assets are available.
      Animated particle texture frames are restitched and uploaded on the same
      50 ms cadence as terrain texture animation; local vanilla 26.1 has
      `assets/minecraft/textures/particle/vibration.png.mcmeta` with
      `frametime: 1`, and `particles/vibration.json` references
      `minecraft:vibration`.
    - Draws active particles as camera-facing textured billboards.
    - 2026-07-05 target-ownership alignment: opaque
      (`translucent == false`) single-quad particles now draw into the main
      color+depth target during the new `opaque_particle_main_pass` step,
      before `copy_main_depth_to_feature_targets` copies the main depth into the
      translucent / item-entity / particle feature targets; only translucent
      particles keep rendering into the dedicated particles target in
      `particle_target_pass`. This mirrors vanilla
      `ParticleFeatureRenderer.render`
      (`net/minecraft/client/renderer/feature/ParticleFeatureRenderer.java`
      26.1 lines 46-57,
      `useParticleTarget = particleTarget != null && translucent`) and
      `LevelRenderer.addMainPass` ordering
      (`net/minecraft/client/renderer/LevelRenderer.java` 26.1:
      `renderSolidFeatures` at line 675 runs before the three `copyDepthFrom`
      calls at lines 680-689, translucent particles trail at
      `renderTranslucentParticles` line 714). Both particle pipelines share
      vanilla `DepthStencilState.DEFAULT` (`LESS_THAN_OR_EQUAL`, depth write on),
      so the opaque pipeline writes main depth exactly like vanilla
      `OPAQUE_PARTICLE`, and that depth now propagates into every feature target.
  - Follow-up work in the plan:
    - full vanilla provider behavior
    - presentation parity

## Per-provider tracking table (established 2026-07-05)

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
