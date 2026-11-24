use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

mod block_data;

fn sand_scene(y: i32) -> (EntityStore, NativeLevel) {
    let mut level = NativeLevel::new();
    level.set(pos(0, y, 0), state_id("minecraft:sand"));
    (EntityStore::new(0, EntityIdCounter::new()), level)
}

#[test]
fn a_box_resting_exactly_on_another_cannot_move_further_into_it() {
    let resting = Aabb {
        min_x: 0.0,
        min_y: 1.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 2.0,
        max_z: 1.0,
    };
    let floor = Aabb {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    assert_eq!(
        collide_with_boxes(
            Vec3d {
                x: 0.0,
                y: -0.5,
                z: 0.0
            },
            resting,
            &[floor]
        ),
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0
        }
    );
}

// ---------------------------------------------------------------------------
// The `FallingBlock` producer
// ---------------------------------------------------------------------------

#[test]
fn placing_a_falling_block_schedules_its_own_tick_two_ticks_later() {
    let mut level = NativeLevel::new();
    let source = pos(0, 70, 0);
    level.set(source, state_id("minecraft:sand"));
    level.world.tick(Vec::new());
    assert_eq!(
        level.world.block_state(DIMENSION, source),
        Some(state_id("minecraft:sand"))
    );
    level.world.tick(Vec::new());
    assert_eq!(
        level.world.block_state(DIMENSION, source),
        Some(state_id("minecraft:air"))
    );
    assert!(level
        .world
        .stored_entity_ids_for_test(DIMENSION)
        .iter()
        .any(|id| matches!(
            level
                .world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .data(),
            EntityData::FallingBlock(_)
        )));
}

#[test]
fn a_block_that_does_not_fall_stays_after_scheduled_ticks() {
    let mut level = NativeLevel::new();
    let source = pos(0, 70, 0);
    level.set(source, state_id("minecraft:stone"));
    for _ in 0..3 {
        level.world.tick(Vec::new());
    }
    assert_eq!(
        level.world.block_state(DIMENSION, source),
        Some(state_id("minecraft:stone"))
    );
    assert!(!level
        .world
        .stored_entity_ids_for_test(DIMENSION)
        .iter()
        .any(|id| matches!(
            level
                .world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .data(),
            EntityData::FallingBlock(_)
        )));
}

#[test]
fn the_scheduled_tick_creates_an_entity_only_when_the_block_below_is_free() {
    let sand = state_id("minecraft:sand");
    let (mut store, mut level) = sand_scene(64);
    // Stone directly below: nothing happens and the block stays.
    assert_eq!(
        level.with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 64, 0), sand)),
        None
    );
    assert!(store.is_empty());
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
        sand
    );

    let (mut store, mut level) = sand_scene(67);
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 67, 0), sand))
        .expect("air below a sand block makes it fall");
    // `FallingBlockEntity.fall` clears the source block before adding the
    // entity, so the entity never collides with where it came from.
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 67, 0)).unwrap(),
        state_id("minecraft:air")
    );
    let entity = store.get(id).expect("entity is live");
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID
    );
    assert_eq!(
        entity.position(),
        Vec3d {
            x: 0.5,
            y: 67.0,
            z: 0.5
        }
    );
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0
        }
    );
    assert_eq!(falling_block_state(&store, id).start_pos(), pos(0, 67, 0));
}

#[test]
fn a_falling_block_applies_swept_effects_before_handling_its_portal() {
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = crate::entity::falling_block::spawn_fallen_block(
        &mut store,
        pos(0, 64, 0),
        state_id("minecraft:sand"),
    );
    let mut entity = store.detach(id).unwrap();
    entity.portal_cooldown = 1;
    entity.set_delta_movement(Vec3d {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
    let entry = pos(2, 64, 0);
    let mut level = NativeLevel::new();
    level
        .world
        .set_block(DIMENSION, entry, state_id("minecraft:nether_portal"), 18);

    assert!(level.with_level(|ctx| crate::entity::falling_block::tick(
        &mut entity,
        &mut store,
        ctx
    )));

    assert_eq!(entity.portal_cooldown(), 299);
    assert!(entity.portal_process().is_none());
    assert_eq!(
        match entity.data() {
            EntityData::FallingBlock(data) => data.time(),
            other => panic!("expected falling block, found {other:?}"),
        },
        1
    );
}

/// `spawn_requested_fallen_block` is `addFreshEntity` (`FallingBlockEntity.java:100`)
/// plus the writes the asking block makes on the entity it gets back:
/// `disableDrop` (`BrushableBlock.java:90`) and `setHurtsEntities`
/// (`PointedDripstoneBlock.java:301`).
#[test]
fn a_requested_fall_carries_the_asking_blocks_writes_onto_the_entity() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let plain = spawn_requested_fallen_block(
        &mut store,
        FallingBlockFallRequest {
            pos: pos(0, 4, 0),
            block_state_id: sand,
            disable_drop: false,
            hurt_entities: None,
        },
    );
    let data = falling_block_state(&store, plain);
    assert_eq!(data.start_pos(), pos(0, 4, 0));
    assert_eq!(data.block_state_id(), sand);
    assert!(!data.cancel_drop());
    assert!(!data.hurt_entities());
    assert_eq!(data.fall_damage_per_distance(), 0.0);
    assert_eq!(data.fall_damage_max(), 40);

    let brushed = spawn_requested_fallen_block(
        &mut store,
        FallingBlockFallRequest {
            pos: pos(0, 5, 0),
            block_state_id: sand,
            disable_drop: true,
            hurt_entities: None,
        },
    );
    assert!(falling_block_state(&store, brushed).cancel_drop());

    let stalactite = spawn_requested_fallen_block(
        &mut store,
        FallingBlockFallRequest {
            pos: pos(0, 6, 0),
            block_state_id: sand,
            disable_drop: false,
            hurt_entities: Some(FallingBlockHurt {
                damage_per_distance: 6.0,
                damage_max: 40,
            }),
        },
    );
    let data = falling_block_state(&store, stalactite);
    assert!(data.hurt_entities());
    assert_eq!(data.fall_damage_per_distance(), 6.0);
    assert_eq!(data.fall_damage_max(), 40);
    // The entity half alone makes no level write; the caller owns that.
    assert!(!data.cancel_drop());
}

#[test]
fn the_fire_tag_decides_whether_a_block_can_fall_into_it() {
    let sand = state_id("minecraft:sand");
    let stone = state_id("minecraft:stone");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let mut level = NativeLevel::new();
    level.world.set_block(DIMENSION, pos(0, 63, 0), stone, 18);
    level.world.set_block(DIMENSION, pos(0, 64, 0), sand, 18);

    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:block",
        "minecraft:fire",
        &[],
    )]));
    assert_eq!(
        level.with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 64, 0), sand)),
        None
    );
    level.install_tags(registry_tag_snapshot(&[(
        "minecraft:block",
        "minecraft:fire",
        &["minecraft:stone"],
    )]));
    assert!(level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 64, 0), sand))
        .is_some());
}

#[test]
fn a_falling_block_below_the_bottom_of_the_world_is_not_created() {
    let mut level = NativeLevel::new();
    let y = level.with_level(|ctx| ctx.min_y()) - 1;
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    assert_eq!(
        level.with_level(|ctx| falling_block_scheduled_tick(
            &mut store,
            ctx,
            pos(0, y, 0),
            state_id("minecraft:sand")
        )),
        None
    );
    assert!(store.is_empty());
}

#[test]
fn is_free_accepts_air_fire_liquid_and_replaceable_and_nothing_else() {
    // `isAir` (`FallingBlock.java:64`), which is the `#minecraft:air` block
    // tag: air, cave air, void air.
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:air")
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:cave_air")
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:void_air")
    ));
    // `is(BlockTags.FIRE)`.
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:fire")
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:soul_fire")
    ));
    // `liquid()`, the cached block flag, which is on for every state of the
    // two `LiquidBlock`s regardless of level or falling.
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with("minecraft:water", &[("level", "0")])
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with("minecraft:water", &[("level", "5")])
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with("minecraft:water", &[("level", "8")])
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with("minecraft:lava", &[("level", "0")])
    ));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with("minecraft:lava", &[("level", "7")])
    ));
    // `canBeReplaced()`, the cached flag with no place context.
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:short_grass")
    ));
    assert!(!is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:stone")
    ));
    assert!(!is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:sand")
    ));
    assert!(!is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_id("minecraft:torch")
    ));
    // A waterlogged block holds a water `FluidState` but is not itself a
    // liquid, so none of the four branches accepts it.
    assert!(!is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        state_with(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "true")],
        )
    ));
}

#[test]
fn the_falling_block_family_is_read_off_the_generated_dust_colour_facts() {
    let falling: Vec<&str> = (0..VANILLA_BLOCK_STATE_COUNT)
        .filter(|id| is_falling_block(*id))
        .filter_map(|id| vanilla_block_state_identity(id).map(|identity| identity.name.as_str()))
        .collect();
    let mut distinct: Vec<&str> = falling.clone();
    distinct.sort_unstable();
    distinct.dedup();
    // The 26.1 `FallingBlock` subclasses are `ColoredFallingBlock` (sand, red
    // sand, gravel), `DragonEggBlock`, `AnvilBlock` (three anvils) and
    // `ConcretePowderBlock` (sixteen powders).
    assert_eq!(distinct.len(), 23);
    for expected in [
        "minecraft:sand",
        "minecraft:red_sand",
        "minecraft:gravel",
        "minecraft:dragon_egg",
        "minecraft:anvil",
        "minecraft:chipped_anvil",
        "minecraft:damaged_anvil",
        "minecraft:white_concrete_powder",
        "minecraft:black_concrete_powder",
    ] {
        assert!(distinct.contains(&expected), "{expected} should fall");
    }
    for rejected in [
        "minecraft:stone",
        "minecraft:suspicious_sand",
        "minecraft:suspicious_gravel",
        "minecraft:pointed_dripstone",
    ] {
        assert!(!distinct.contains(&rejected), "{rejected} should not fall");
    }
}

#[test]
fn may_replace_follows_the_class_override_not_the_cached_replaceable_flag() {
    // `FallingBlockEntity.tick:184-186` asks
    // `canBeReplaced(DirectionalPlaceContext)`, not `canBeReplaced()`, and the
    // two disagree wherever a class overrides the context form with something
    // that is not gated on the held stack. `is_free` reads the flag
    // (`FallingBlock.java:64`), so the contrast between the two calls below is
    // the divergence itself.

    // `SnowLayerBlock:112-120` with a stack that is not a snow layer answers
    // `layers == 1`, while `Blocks.java:1997-2010` registers snow
    // `.replaceable()` at every depth.
    assert!(can_be_replaced_by_empty_handed_place(state_with(
        "minecraft:snow",
        &[("layers", "1")]
    )));
    for layers in 2..=8 {
        let snow = state_with("minecraft:snow", &[("layers", &layers.to_string())]);
        assert!(
            !can_be_replaced_by_empty_handed_place(snow),
            "snow[layers={layers}] may not be replaced"
        );
        assert!(
            is_free(crate::registry_tags::RegistryTagSnapshot::builtin(), snow),
            "snow[layers={layers}] still carries the flag"
        );
    }

    // `VineBlock:282-286`: the clicked position is the tested position, so the
    // answer is "some face is still vacant".
    let full_vine = state_with(
        "minecraft:vine",
        &[
            ("up", "true"),
            ("north", "true"),
            ("east", "true"),
            ("south", "true"),
            ("west", "true"),
        ],
    );
    let partial_vine = state_with(
        "minecraft:vine",
        &[
            ("up", "true"),
            ("north", "true"),
            ("east", "true"),
            ("south", "true"),
            ("west", "false"),
        ],
    );
    assert!(!can_be_replaced_by_empty_handed_place(full_vine));
    assert!(can_be_replaced_by_empty_handed_place(partial_vine));
    assert!(is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        full_vine
    ));

    // `MultifaceBlock:174-177` answers true for an empty stack, and sculk vein
    // is the one multiface block vanilla does not register `.replaceable()`
    // (`Blocks.java:6079-6088`).
    let sculk_vein = state_id("minecraft:sculk_vein");
    assert!(can_be_replaced_by_empty_handed_place(sculk_vein));
    assert!(!is_free(
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        sculk_vein
    ));
    assert!(can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:glow_lichen"
    )));
    assert!(can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:resin_clump"
    )));

    // The three that refuse outright for a stack that is not their own item:
    // `ScaffoldingBlock:71-74`, `SlabBlock:83-97`, `PitcherCropBlock:128-131`.
    assert!(!can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:scaffolding"
    )));
    assert!(!can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:oak_slab"
    )));
    assert!(!can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:pitcher_crop"
    )));

    // The six classes whose override *is* gated on the held stack fall back to
    // `BlockBehaviour:270-272`, which an empty stack reduces to the cached
    // flag, so for them the answer is the flag whatever the flag happens to be.
    for gated in [
        "minecraft:sea_pickle",
        "minecraft:candle",
        "minecraft:turtle_egg",
        "minecraft:pink_petals",
        "minecraft:leaf_litter",
        "minecraft:kelp_plant",
    ] {
        let state = state_id(gated);
        let flag = vanilla_block_state_support_facts(state)
            .expect("a built-in state has support facts")
            .replaceable();
        assert_eq!(
            can_be_replaced_by_empty_handed_place(state),
            flag,
            "{gated}"
        );
    }

    // And the classes with no override at all.
    assert!(can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:short_grass"
    )));
    assert!(can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:water"
    )));
    assert!(!can_be_replaced_by_empty_handed_place(state_id(
        "minecraft:stone"
    )));
}

#[test]
fn the_vine_state_definition_is_exactly_the_five_face_properties() {
    // `vine_has_a_vacant_face` counts "some property is not true" instead of
    // naming `VineBlock.PROPERTY_BY_DIRECTION`; that is the same predicate only
    // because `VineBlock.createBlockStateDefinition` (`VineBlock.java:308-310`)
    // adds those five properties and nothing else.
    let identity = vanilla_block_state_identity(state_id("minecraft:vine"))
        .expect("vine is a built-in 26.1 block");
    let mut names: Vec<&str> = identity
        .properties
        .keys()
        .map(std::string::String::as_str)
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["east", "north", "south", "up", "west"]);
    assert!(identity.properties.values().all(|value| value == "false"));
}

/// FallingBlockEntity.java:188 asks the landing state's own `canSurvive`;
/// scaffolding resting on the floor is within distance, so it lands
/// (ScaffoldingBlock.java:119-122,132-134).
#[test]
fn falling_scaffolding_lands_as_scaffolding() {
    let scaffolding = state_with(
        "minecraft:scaffolding",
        &[
            ("bottom", "true"),
            ("distance", "7"),
            ("waterlogged", "false"),
        ],
    );
    let mut level = NativeLevel::new();
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 67.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(scaffolding, pos(0, 67, 0))),
    );
    run_to_completion(&mut store, &mut level, id);
    let landed = level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap();
    assert!(crate::block_behavior::scaffolding::is_scaffolding(landed));
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

// ---------------------------------------------------------------------------
// The `FallingBlockEntity` consumer
// ---------------------------------------------------------------------------

#[test]
fn a_falling_block_moves_down_under_gravity() {
    let (mut store, mut level) = sand_scene(66);
    let sand = state_id("minecraft:sand");
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 66, 0), sand))
        .unwrap();
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    let entity = store.get(id).unwrap();
    assert_eq!(entity.position().y, 66.0 - DEFAULT_GRAVITY);
    assert_eq!(entity.delta_movement().y, -DEFAULT_GRAVITY * 0.98);
    assert!(entity.fall_distance() > 0.0);
}

#[test]
fn sand_becomes_a_block_again_where_it_lands() {
    let sand = state_id("minecraft:sand");
    let (mut store, mut level) = sand_scene(66);
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 66, 0), sand))
        .unwrap();
    run_to_completion(&mut store, &mut level, id);
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 64, 0)),
        Some(sand)
    );
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 66, 0)),
        Some(state_id("minecraft:air"))
    );
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
    assert!(store.is_empty());
}

#[test]
fn landing_where_the_block_cannot_be_replaced_drops_the_item_instead() {
    let sand = state_id("minecraft:sand");
    // A torch has no collision shape, so the sand falls through its cell and
    // lands on the stone below it; the torch is not replaceable, so the block
    // cannot be placed there.
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:torch"), 18);
    level.world.set_block(DIMENSION, pos(0, 67, 0), sand, 18);
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 67, 0), sand))
        .expect("the sand falls");

    for _ in 0..40 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(
        store.get(id).is_none(),
        "the entity resolves within 40 ticks"
    );
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
        state_id("minecraft:torch")
    );
    assert_eq!(level.world.stored_items_for_test(DIMENSION).len(), 1);
    let items = level.world.stored_items_for_test(DIMENSION);
    assert_eq!(items[0].2, "minecraft:sand");
    assert_eq!(items[0].3, 1);
    assert_eq!(items[0].1.y, 64.0);
}

#[test]
fn the_entity_drops_game_rule_withholds_the_item_but_not_the_removal() {
    let sand = state_id("minecraft:sand");
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level
        .world
        .set_block(DIMENSION, pos(0, 64, 0), state_id("minecraft:torch"), 18);
    level.world.set_block(DIMENSION, pos(0, 67, 0), sand, 18);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule minecraft:entity_drops false",
    );
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 67, 0), sand))
        .expect("the sand falls");

    for _ in 0..40 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(store.get(id).is_none());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

#[test]
fn an_aged_falling_block_outside_the_world_drops_its_item() {
    let mut level = NativeLevel::new();
    let y = level.with_level(|ctx| ctx.min_y()) - 1;
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: f64::from(y),
            z: 0.5,
        },
        EntityData::FallingBlock(
            FallingBlockData::new(state_id("minecraft:sand"), pos(0, y, 0))
                .with_saved_fields(100, true, false, false, 0.0, 40),
        ),
    );
    level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    assert!(store.get(id).is_none());
    assert!(level
        .world
        .stored_items_for_test(DIMENSION)
        .iter()
        .any(|item| item.2 == "minecraft:sand" && item.3 == 1));
}

/// Ticks until the entity resolves, or fails.
fn run_to_completion(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..40 {
        level.with_level(|ctx| tick_entities(store, ctx, true));
        if store.get(id).is_none() {
            return;
        }
    }
    panic!("the entity did not resolve within 40 ticks");
}

#[test]
fn a_falling_anvil_carries_the_damage_fields_the_falling_hook_sets() {
    // `FallingBlock.tick:52` calls `this.falling(entity)` on the block that
    // scheduled the tick, and `AnvilBlock.falling` (`AnvilBlock.java:82-84`) is
    // `setHurtsEntities(2.0F, 40)`. All three fields are saved
    // (`FallingBlockEntity.java:285-287`), so a world with a falling anvil in
    // it differs on disk without them.
    for anvil in [
        "minecraft:anvil",
        "minecraft:chipped_anvil",
        "minecraft:damaged_anvil",
    ] {
        let state = state_id(anvil);
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        level.world.set_block(DIMENSION, pos(0, 67, 0), state, 18);
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = level
            .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 67, 0), state))
            .expect("the anvil falls");
        let data = falling_block_state(&store, id);
        assert!(data.hurt_entities(), "{anvil}");
        assert_eq!(data.fall_damage_per_distance(), 2.0, "{anvil}");
        assert_eq!(data.fall_damage_max(), 40, "{anvil}");

        // And the save form carries them, which is where the divergence shows.
        let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
        let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
        let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
        let reloaded_data = falling_block_state(&reloaded, ids[0]);
        assert!(reloaded_data.hurt_entities(), "{anvil}");
        assert_eq!(reloaded_data.fall_damage_per_distance(), 2.0, "{anvil}");
    }
}

#[test]
fn missing_hurt_entities_uses_the_current_anvil_tag_generation() {
    let mut source = EntityStore::new(0x000A_1171, EntityIdCounter::new());
    source.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 4.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(
            state_id("minecraft:sand"),
            pos(0, 4, 0),
        )),
    );
    let mut raw = write_entity_chunk(&source, (0, 0)).unwrap().into_raw();
    let entities = raw
        .entries_mut()
        .iter_mut()
        .find(|entry| entry.name == "Entities")
        .unwrap();
    let NbtSummaryValue::List(entities) = &mut entities.value else {
        unreachable!()
    };
    let NbtSummaryValue::Compound(record) = &mut entities[0] else {
        unreachable!()
    };
    record.retain(|entry| entry.name != "HurtEntities");
    let document = Java26EntityChunkData::parse(raw, 0, 0).unwrap();
    let tagged_tags =
        registry_tag_snapshot(&[("minecraft:block", "minecraft:anvil", &["minecraft:sand"])]);
    let mut tagged = EntityStore::new(1, EntityIdCounter::new());
    let tagged_id =
        read_entity_chunk_with_registry_tags(&mut tagged, &document, &tagged_tags).unwrap()[0];
    assert!(falling_block_state(&tagged, tagged_id).hurt_entities());

    let untagged_tags = registry_tag_snapshot(&[("minecraft:block", "minecraft:anvil", &[])]);
    let mut untagged = EntityStore::new(2, EntityIdCounter::new());
    let untagged_id =
        read_entity_chunk_with_registry_tags(&mut untagged, &document, &untagged_tags).unwrap()[0];
    assert!(!falling_block_state(&untagged, untagged_id).hurt_entities());
}

#[test]
fn a_falling_block_outside_the_anvil_family_keeps_the_constructor_damage_fields() {
    for block in [
        "minecraft:sand",
        "minecraft:gravel",
        "minecraft:dragon_egg",
        "minecraft:white_concrete_powder",
    ] {
        let state = state_id(block);
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        level.world.set_block(DIMENSION, pos(0, 67, 0), state, 18);
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = level
            .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 67, 0), state))
            .expect("the block falls");
        let data = falling_block_state(&store, id);
        assert!(!data.hurt_entities(), "{block}");
        assert_eq!(data.fall_damage_per_distance(), 0.0, "{block}");
        assert_eq!(data.fall_damage_max(), 40, "{block}");
    }
}

#[test]
fn mth_equal_compares_against_the_float_tolerance_widened_to_a_double() {
    // `Mth.equal(double, double)` (`Mth.java:161-163`) writes its bound as
    // `1.0E-5F`, so the comparison happens against the widened `float`
    // 9.999999747378752E-6 rather than against 1.0E-5.
    const VANILLA_TOLERANCE: f64 = 9.999_999_747_378_752e-6;
    assert_eq!(VANILLA_TOLERANCE, f64::from(1.0e-5_f32));
    assert!(mth_equal(0.0, VANILLA_TOLERANCE / 2.0));
    assert!(!mth_equal(0.0, VANILLA_TOLERANCE));
    assert!(!mth_equal(0.0, 9.999_999_9e-6));
    assert!(!mth_equal(0.0, 1.0e-5));
}

#[test]
fn sand_landing_on_a_deep_snow_layer_breaks_instead_of_replacing_it() {
    let sand = state_id("minecraft:sand");
    let snow = state_with("minecraft:snow", &[("layers", "3")]);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level.world.set_block(DIMENSION, pos(0, 64, 0), snow, 18);
    level.world.set_block(DIMENSION, pos(0, 68, 0), sand, 18);
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 68, 0), sand))
        .expect("the sand falls");
    run_to_completion(&mut store, &mut level, id);

    // `SnowLayerBlock.canBeReplaced` refuses at three layers, so this is the
    // `:225-231` branch: the snow survives and the sand becomes an item.
    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
        snow
    );
    assert_eq!(level.world.stored_items_for_test(DIMENSION).len(), 1);
    assert_eq!(
        level.world.stored_items_for_test(DIMENSION)[0].2.as_str(),
        "minecraft:sand"
    );
}

#[test]
fn sand_landing_on_a_single_snow_layer_replaces_it() {
    let sand = state_id("minecraft:sand");
    let snow = state_with("minecraft:snow", &[("layers", "1")]);
    let mut level = NativeLevel::new();
    for x in -3..=3 {
        for z in -3..=3 {
            level.world.set_block(
                DIMENSION,
                BlockPos { x, y: 63, z },
                state_id("minecraft:stone"),
                18,
            );
        }
    }
    level.world.set_block(DIMENSION, pos(0, 64, 0), snow, 18);
    level.world.set_block(DIMENSION, pos(0, 68, 0), sand, 18);
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 68, 0), sand))
        .expect("the sand falls");
    run_to_completion(&mut store, &mut level, id);

    assert_eq!(
        level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
        sand
    );
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

#[test]
fn a_waterloggable_block_lands_wet_only_in_source_water() {
    // `FallingBlockEntity.java:190` tests `is(Fluids.WATER)`, the source
    // singleton, so a block landing in flowing water stays dry. No block in the
    // `FallingBlock` family is waterloggable, but a save can carry any state,
    // so the entity is built directly.
    let dry_slab = state_with(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    let wet_slab = state_with(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "true")],
    );
    for (water, expected) in [
        (state_with("minecraft:water", &[("level", "0")]), wet_slab),
        (state_with("minecraft:water", &[("level", "1")]), dry_slab),
    ] {
        let mut level = NativeLevel::new();
        for x in -3..=3 {
            for z in -3..=3 {
                level.world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 63, z },
                    state_id("minecraft:stone"),
                    18,
                );
            }
        }
        level.world.set_block(DIMENSION, pos(0, 64, 0), water, 18);
        let mut store = EntityStore::new(0, EntityIdCounter::new());
        let id = store.spawn(
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            Vec3d {
                x: 0.5,
                y: 67.0,
                z: 0.5,
            },
            EntityData::FallingBlock(FallingBlockData::new(dry_slab, pos(0, 67, 0))),
        );
        run_to_completion(&mut store, &mut level, id);
        assert_eq!(
            level.world.block_state(DIMENSION, pos(0, 64, 0)).unwrap(),
            expected
        );
        assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
    }
}

#[test]
fn an_entity_more_than_sixty_four_blocks_below_the_world_is_discarded_by_the_base_tick() {
    let mut level = NativeLevel::new();
    let y = level.with_level(|ctx| ctx.min_y()) - 70;
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: f64::from(y),
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(
            state_id("minecraft:sand"),
            pos(0, y, 0),
        )),
    );
    let mut entity = store.detach(id).unwrap();
    assert!(level.with_level(|ctx| crate::entity::base_tick(&mut entity, &mut store, ctx)));
    assert_eq!(entity.removal_reason(), Some(RemovalReason::Discarded));
    store.attach(entity);
    assert!(store.is_empty());
    assert!(level.world.stored_items_for_test(DIMENSION).is_empty());
}

// ---------------------------------------------------------------------------
// The save seam
// ---------------------------------------------------------------------------

#[test]
fn a_falling_block_round_trips_through_the_entity_chunk_save_form() {
    let sand = state_id("minecraft:sand");
    let (mut store, mut level) = sand_scene(69);
    let id = level
        .with_level(|ctx| falling_block_scheduled_tick(&mut store, ctx, pos(0, 69, 0), sand))
        .expect("the sand falls");
    for _ in 0..3 {
        level.with_level(|ctx| tick_entities(&mut store, ctx, true));
    }
    {
        let entity = store.get_mut(id).expect("entity is live");
        entity.set_rot(81.5, -22.25);
        entity.set_delta_movement(Vec3d {
            x: 0.25,
            y: -0.5,
            z: 0.75,
        });
        entity.fall_distance = 12.5;
        entity.remaining_fire_ticks = 37;
        entity.air_supply = 111;
        entity.on_ground = true;
        entity.invulnerable = true;
        entity.portal_cooldown = 42;
        entity.no_gravity = true;
    }
    let before = store.get(id).expect("entity is live").clone();

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.chunk_x(), 0);
    assert_eq!(document.chunk_z(), 0);
    assert_eq!(document.entities().len(), 1);
    assert_eq!(
        document.entities().next().unwrap().id(),
        "minecraft:falling_block"
    );

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    assert_eq!(ids.len(), 1);
    let after = reloaded.get(ids[0]).expect("entity is live");

    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.entity_type_id(), before.entity_type_id());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert_eq!(
        (after.y_rot(), after.x_rot()),
        (before.y_rot(), before.x_rot())
    );
    assert_eq!(after.fall_distance(), before.fall_distance());
    assert_eq!(after.remaining_fire_ticks(), before.remaining_fire_ticks());
    assert_eq!(after.on_ground(), before.on_ground());
    assert_eq!(after.air_supply(), before.air_supply());
    assert_eq!(after.invulnerable(), before.invulnerable());
    assert_eq!(after.portal_cooldown(), before.portal_cooldown());
    assert_eq!(after.no_gravity(), before.no_gravity());
    assert!(after.needs_sync());

    let (EntityData::FallingBlock(after_data), EntityData::FallingBlock(before_data)) =
        (after.data(), before.data())
    else {
        panic!("a falling block round trips as a falling block");
    };
    assert_eq!(after_data.block_state_id(), before_data.block_state_id());
    assert_eq!(after_data.time(), before_data.time());
    assert_eq!(after_data.drop_item(), before_data.drop_item());
    assert_eq!(after_data.cancel_drop(), before_data.cancel_drop());
    assert_eq!(after_data.hurt_entities(), before_data.hurt_entities());
    assert_eq!(after_data.fall_damage_max(), before_data.fall_damage_max());
    assert_eq!(
        after_data.fall_damage_per_distance(),
        before_data.fall_damage_per_distance()
    );
    // The start position is synched state rather than saved state, so it does
    // not survive; vanilla loses it the same way.
    assert_eq!(after_data.start_pos(), pos(0, 0, 0));
    assert_single_added_matches_store(&mut reloaded, ids[0]);
}

#[test]
fn an_empty_chunk_writes_an_empty_entity_list() {
    let store = EntityStore::new(0, EntityIdCounter::new());
    let document = write_entity_chunk(&store, (3, -4)).expect("an empty chunk is well formed");
    assert!(document.is_empty());
    assert_eq!((document.chunk_x(), document.chunk_z()), (3, -4));
}

#[test]
fn only_the_entities_of_the_requested_chunk_are_written() {
    let sand = state_id("minecraft:sand");
    let mut store = EntityStore::new(0, EntityIdCounter::new());
    store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 10.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(0, 10, 0))),
    );
    store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 20.5,
            y: 10.0,
            z: 0.5,
        },
        EntityData::FallingBlock(FallingBlockData::new(sand, pos(20, 10, 0))),
    );
    assert_eq!(
        write_entity_chunk(&store, (0, 0))
            .expect("the save form is well formed")
            .entities()
            .len(),
        1
    );
    assert_eq!(
        write_entity_chunk(&store, (1, 0))
            .expect("the save form is well formed")
            .entities()
            .len(),
        1
    );
}

mod contact;
