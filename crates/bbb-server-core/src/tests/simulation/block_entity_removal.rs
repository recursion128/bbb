//! `BlockEntity.preRemoveSideEffects` reached
//! through a real block write: the container slot order, the lectern book, the
//! jukebox record, and the flag that skips all three.

use super::*;

use super::helpers::{shrieker_state, state_with as named_block_state};
use super::sounds_sent as sounds;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;

fn air_state() -> i32 {
    block_state_id("minecraft:air")
}

/// The live item entities of one level as `(item resource id, count, position)`.
fn ground_items(
    world: &ServerWorld,
    dimension: Java26BuiltinDimension,
) -> Vec<(String, i32, Vec3d)> {
    world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(dimension, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((
                stack.item_resource_id()?.to_string(),
                stack.count(),
                entity.position(),
            ))
        })
        .collect()
}

fn furnace_state(lit: bool) -> i32 {
    named_block_state(
        "minecraft:furnace",
        &[
            ("facing", "north"),
            ("lit", if lit { "true" } else { "false" }),
        ],
    )
}

/// The live experience orbs of one level as `(value, position)`.
fn ground_orbs(world: &ServerWorld, dimension: Java26BuiltinDimension) -> Vec<(i32, Vec3d)> {
    world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(dimension, id)?;
            let crate::entity::EntityData::ExperienceOrb(orb) = entity.data() else {
                return None;
            };
            Some((orb.value(), entity.position()))
        })
        .collect()
}

/// `AbstractFurnaceBlockEntity.preRemoveSideEffects`
/// `super.preRemoveSideEffects`
/// drops the container contents, and `getRecipesToAwardAndPopExperience(level,
/// Vec3.atCenterOf(pos))` pops the cooked experience at the block's
/// center with no taker to award the recipes to.
#[test]
fn breaking_a_furnace_drops_its_contents_and_pops_the_cooked_experience() {
    use bbb_protocol::item_types::{VANILLA_ITEM_COAL_ID, VANILLA_ITEM_RAW_GOLD_ID};

    let (_sender, receiver, mut world) = helpers::joined_world(datapack_config());
    let at = BlockPos { x: 0, y: 64, z: 8 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        furnace_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let raw_gold = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_RAW_GOLD_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let coal = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_COAL_ID),
        count: 1,
        component_patch: Default::default(),
    };
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::furnace_mut)
        .expect("the write created the furnace's block entity")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(raw_gold),
                crate::tests::native_stack(coal),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("the fixture furnace items are accepted");
    // `minecraft:gold_ingot_from_smelting_raw_gold` takes 200 ticks and is
    // worth a whole point, so `createExperience` (`:358-364`) has no fraction
    // roll to make the assertion depend on the level's random.
    for _ in 0..200 {
        world.tick(receiver.drain());
    }
    let furnace = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.furnace())
        .expect("the furnace block entity is live")
        .clone();
    assert_eq!(
        furnace
            .recipes_used()
            .get("minecraft:gold_ingot_from_smelting_raw_gold"),
        Some(&1)
    );

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());

    // The cook spent the input and the fuel, so the container half of the
    // override has the output slot left to drop.
    assert_eq!(
        ground_items(&world, Java26BuiltinDimension::Overworld)
            .iter()
            .map(|(resource_id, count, _)| (resource_id.as_str(), *count))
            .collect::<Vec<_>>(),
        vec![("minecraft:gold_ingot", 1)]
    );
    assert_eq!(
        ground_orbs(&world, Java26BuiltinDimension::Overworld),
        vec![(
            1,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: 8.5,
            }
        )],
        "`Vec3.atCenterOf(pos)` is where `ExperienceOrb.award` places the orb"
    );
}

/// `LevelChunk.setBlockState`'s removal arm reaches
/// `BlockEntity.preRemoveSideEffects`, so a broken
/// chest's contents leave as the item entities `Containers.dropContents` makes
/// rather than with the record.
#[test]
fn breaking_a_chest_drops_every_slot_in_slot_order() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let chest = named_block_state(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let stone = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 9,
        component_patch: Default::default(),
    };
    let diamond_patch = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        ],
        damage: Some(7),
        ..DataComponentPatchSummary::default()
    };
    let diamond = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID),
        count: 3,
        component_patch: diamond_patch.into(),
    };
    let mut items = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
    items[2] = crate::tests::native_stack(stone.clone());
    items[5] = crate::tests::native_stack(diamond.clone());
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest_mut)
        .expect("the write created the chest's block entity")
        .set_items(items));

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let drops = ground_items(&world, Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 2, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:stone");
    assert_eq!(drops[0].1, 9);
    assert_eq!(drops[1].0, "minecraft:diamond");
    assert_eq!(drops[1].1, 3);
    for (_, _, position) in &drops {
        // `Math.floor(pos) + random * 0.75 + 0.125` stays inside the block
        // column.
        assert!((0.0..1.0).contains(&position.x), "{position:?}");
        assert!((64.0..65.0).contains(&position.y), "{position:?}");
        assert!((1.0..2.0).contains(&position.z), "{position:?}");
    }
    // The component patch travels with the stack into the entity.
    let diamond_components = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            (stack.item_resource_id()? == "minecraft:diamond").then(|| stack.clone())
        })
        .expect("the diamond drop");
    assert_eq!(diamond_components.summary().component_patch.damage, Some(7));

    // The drops are ordinary item entities, so the save carries them.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let saved_items = saved_entities(&request)
        .iter()
        .filter(|entity| {
            entity.iter().any(|entry| {
                entry.name == "id" && entry.value == NbtSummaryValue::string("minecraft:item")
            })
        })
        .count();
    assert_eq!(saved_items, 2);

    for id in world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld) {
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .expect("a live entity");
        let crate::entity::EntityData::Item(item) = entity.data() else {
            continue;
        };
        // `Containers.dropItemStack` never calls `setDefaultPickUpDelay`
        // unlike `Entity.spawnAtLocation`
        // so a container drop is pickable at once.
        assert_eq!(item.pickup_delay(), 0);
    }
}

/// `LevelChunk`: flag 256 skips `preRemoveSideEffects`, so the
/// record goes and the contents do not.
#[test]
fn skipping_block_entity_side_effects_keeps_the_contents_off_the_ground() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let chest = named_block_state(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        chest,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let mut items = vec![bbb_server_types::item_stack::RuntimeItemStack::empty(); 27];
    items[0] = crate::tests::native_stack(ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: Default::default(),
    });
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::chest_mut)
        .expect("the write created the chest's block entity")
        .set_items(items));

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL
            | crate::block_entity::SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS,
    ));
    assert!(ground_items(&world, Java26BuiltinDimension::Overworld).is_empty());
    assert!(world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .is_none());
}

/// `LecternBlockEntity.preRemoveSideEffects`
/// drops the book one block above the lectern, a quarter block toward its
/// facing, with the default pickup delay and no motion of its own.
#[test]
fn breaking_a_lectern_drops_its_book_toward_its_facing() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let lectern = named_block_state(
        "minecraft:lectern",
        &[
            ("facing", "east"),
            ("has_book", "true"),
            ("powered", "false"),
        ],
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        lectern,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let book = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::lectern_mut)
        .expect("the write created the lectern's block entity")
        .set_book(crate::tests::native_stack(book))
        .expect("the fixture item is accepted");

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let drops = ground_items(&world, Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 1, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:written_book");
    assert_eq!(drops[0].1, 1);
    // `LecternBlockEntity.preRemoveSideEffects` does call
    // `setDefaultPickUpDelay` (`:234`).
    let entity = world
        .stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)[0],
        )
        .expect("a live entity");
    let crate::entity::EntityData::Item(item) = entity.data() else {
        panic!("the book became an item entity");
    };
    assert_eq!(
        item.pickup_delay(),
        crate::entity::item::DEFAULT_PICKUP_DELAY
    );
    // East is `getStepX() == 1`, so `0.5 + 0.25` (`:231-232`).
    assert!((drops[0].2.x - 0.75).abs() < 1.0e-9, "{:?}", drops[0].2);
    assert!((drops[0].2.y - 65.0).abs() < 1.0e-9, "{:?}", drops[0].2);
    assert!((drops[0].2.z - 1.5).abs() < 1.0e-9, "{:?}", drops[0].2);
}

/// `JukeboxBlockEntity.popOutTheItem` pops the
/// record at `0.5, 1.01, 0.5` with a `±0.35` horizontal spread; `setRemoved`
/// follows.
#[test]
fn breaking_a_jukebox_pops_its_record_above_the_block() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded_stocked(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let jukebox = named_block_state("minecraft:jukebox", &[("has_record", "true")]);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        jukebox,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let record = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_MUSIC_DISC_CAT_ID),
        count: 1,
        component_patch: Default::default(),
    };
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::jukebox_mut)
        .expect("the write created the jukebox's block entity")
        .set_the_item(
            crate::tests::native_stack(record),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .expect("the fixture item is accepted");
    let stops = |world: &ServerWorld| {
        world
            .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
            .iter()
            .filter(|event| {
                event.recorded.game_event == crate::level_event::VanillaGameEvent::JUKEBOX_STOP_PLAY
            })
            .count()
    };
    let before = stops(&world);

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    // The playing song stops as the record pops, then `setRemoved` stops again.
    assert_eq!(stops(&world) - before, 2);
    let drops = ground_items(&world, Java26BuiltinDimension::Overworld);
    assert_eq!(drops.len(), 1, "{drops:?}");
    assert_eq!(drops[0].0, "minecraft:music_disc_cat");
    assert_eq!(drops[0].1, 1);
    let entity = world
        .stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            world.stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)[0],
        )
        .expect("a live entity");
    let crate::entity::EntityData::Item(item) = entity.data() else {
        panic!("the record became an item entity");
    };
    assert_eq!(
        item.pickup_delay(),
        crate::entity::item::DEFAULT_PICKUP_DELAY
    );
    assert!(
        (drops[0].2.x - 0.5).abs() <= 0.35 + 1.0e-9,
        "{:?}",
        drops[0].2
    );
    assert!((drops[0].2.y - 65.01).abs() < 1.0e-9, "{:?}", drops[0].2);
    assert!(
        (drops[0].2.z - 1.5).abs() <= 0.35 + 1.0e-9,
        "{:?}",
        drops[0].2
    );
}

/// `SculkShriekerBlockEntity.preRemoveSideEffects` (`:133-138`) over
/// `tryRespond` (`:140-148`): a shrieking shrieker whose warning level reached
/// four summons a warden as it goes.
#[test]
fn breaking_a_fully_warned_shrieker_summons_a_warden() {
    let (sender, receiver, mut world) = helpers::joined_stand_world(
        datapack_config(),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
    );
    // `MobEffectUtil.addEffectToPlayersAround` filters on survival
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shrieker_state(true, true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sculk_shrieker_mut)
        .expect("the write created the shrieker's block entity")
        .set_warning_level_for_test(4);

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let published = world.tick(receiver.drain());

    let warden = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID)
        })
        .expect("the shrieker summoned a warden");
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, warden)
        .expect("the warden is stored");
    // `SpawnUtil.trySpawnMob`'s `ON_TOP_OF_COLLIDER` walk starts six above the
    // shrieker and descends.
    assert!(
        (entity.position().x - 0.5).abs() <= 5.5 && (entity.position().z - 1.5).abs() <= 5.5,
        "the warden lands in the five-block ring: {:?}",
        entity.position()
    );
    // `Warden.applyDarknessAround` reached the session.
    assert!(
        world
            .player_active_effect_for_test(ConnectionId(1), "minecraft:darkness")
            .is_some(),
        "the summon still darkens the player"
    );
    assert!(
        published.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::UpdateMobEffect(_),
                ..
            }
        )),
        "the darkness reaches the client"
    );
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };
    let entity = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, warden)
        .unwrap();
    assert_eq!(entity.pose_id(), VANILLA_ENTITY_POSE_EMERGING_ID);
    let uuid = entity.uuid();
    fn memory<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
        let NbtSummaryValue::Compound(brain) = field(fields, "Brain") else {
            panic!("Brain compound")
        };
        let NbtSummaryValue::Compound(memories) = field(brain, "memories") else {
            panic!("memories compound")
        };
        field(memories, name)
    }
    let request = world
        .save_request(1_786_000_003_000, 203, RegionCompression::Zlib)
        .unwrap();
    let saved = saved_entities(&request)
        .into_iter()
        .find(|fields| field(fields, "UUID") == &nbt_uuid(uuid))
        .unwrap();
    for (name, limit) in [
        ("minecraft:is_emerging", 134),
        ("minecraft:dig_cooldown", 1200),
    ] {
        let NbtSummaryValue::Compound(memory) = memory(&saved, name) else {
            panic!("unit memory compound")
        };
        assert_eq!(
            field(memory, "value"),
            &NbtSummaryValue::Compound(Vec::new())
        );
        assert!(
            matches!(field(memory, "ttl"), NbtSummaryValue::Long(ttl) if (1..=limit).contains(ttl))
        );
    }
    let switched = crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        211,
        "gamemode creative alice",
    );
    let sound = |tick: &TickResult, name: &str| {
        tick.events.iter().any(|event| {
        let (sound, volume, pitch) = match event {
            WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            WorldEvent::Send { packet: ClientboundPlayPacket::SoundEntity(sound), .. } => (&sound.sound, sound.volume, sound.pitch),
            _ => return false,
        };
        if !matches!(sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
            if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(*registry_id) == Some(name)) { return false; }
        assert_eq!((volume, pitch), (5.0, 1.0));
        true
    })
    };
    assert!(
        sound(&published, "minecraft:entity.warden.agitated"),
        "triggered spawning publishes its agitated sound"
    );
    let mut standing = false;
    let mut emerged = sound(&published, "minecraft:entity.warden.emerge")
        || sound(&switched, "minecraft:entity.warden.emerge");
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        emerged |= sound(&tick, "minecraft:entity.warden.emerge");
        if world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, warden)
            .unwrap()
            .pose_id()
            == VANILLA_ENTITY_POSE_STANDING_ID
        {
            standing = true;
            break;
        }
    }
    assert!(
        standing,
        "the triggered warden finishes emerging and stands"
    );
    assert!(emerged, "emergence reaches the client");
}

/// The same removal with the warning level short of four plays the reply sound
/// instead of summoning.
#[test]
fn a_partially_warned_shrieker_answers_with_a_sound() {
    let (sender, receiver, mut world) = helpers::joined_world(datapack_config());
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
    );
    // `MobEffectUtil.addEffectToPlayersAround` filters on survival
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        shrieker_state(true, true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::sculk_shrieker_mut)
        .expect("the write created the shrieker's block entity")
        .set_warning_level_for_test(2);

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let published = world.tick(receiver.drain());

    assert!(
        world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .all(|id| world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .is_none_or(|entity| { entity.entity_type_id() != VANILLA_ENTITY_TYPE_WARDEN_ID })),
        "a level-two warning does not summon"
    );
    let sound = sounds(&published)
        .into_iter()
        .find(|sound| {
            matches!(
                &sound.sound,
                SoundEventHolder::Reference { registry_id }
                    if *registry_id
                        == vanilla_sound_event_registry_id("minecraft:entity.warden.nearby_closer")
                            .expect("the reply sound is in the catalog")
            )
        })
        .expect("the warning level names its reply sound");
    assert_eq!(sound.source, SoundSource::Hostile);
    assert_eq!(sound.volume, 5.0);
    // `playWardenReplySound` offsets by `Mth.randomBetweenInclusive(-10, 10)`.
    assert!(
        (sound.position.x - 0.0).abs() <= 10.5
            && (sound.position.y - 64.0).abs() <= 10.5
            && (sound.position.z - 1.0).abs() <= 10.5,
        "{:?}",
        sound.position
    );
}

/// `canRespond`: the state, the
/// difficulty and the rule each close the response.
#[test]
fn a_shrieker_that_cannot_respond_stays_silent() {
    let (sender, receiver) = mailbox();
    // `/gamerule` needs the operator level the rule command asks for.
    let mut world = spawn_pinned_world(ServerWorldConfig {
        admin_lists: operator_config(&[("alice", 1)]).admin_lists,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
    );
    // `MobEffectUtil.addEffectToPlayersAround` filters on survival
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    // The third arm closes the rule instead of the state; the command path is
    // the one that refreshes the simulation's own projection.
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule spawn_wardens false",
    );
    for (index, (shrieking, can_summon)) in [(false, true), (true, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        let at = BlockPos {
            x: index as i32,
            y: 64,
            z: 1,
        };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            at,
            shrieker_state(shrieking, can_summon),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
            .and_then(BlockEntity::sculk_shrieker_mut)
            .expect("the write created the shrieker's block entity")
            .set_warning_level_for_test(4);
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            at,
            air_state(),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let published = world.tick(receiver.drain());

    assert!(sounds(&published).is_empty(), "{:?}", sounds(&published));
    assert!(world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .all(|id| world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .is_none_or(|entity| entity.entity_type_id() != VANILLA_ENTITY_TYPE_WARDEN_ID)));
    assert!(world
        .player_active_effect_for_test(ConnectionId(1), "minecraft:darkness")
        .is_none());
}

/// `CreakingHeartBlockEntity.preRemoveSideEffects` (`:306-309`) over
/// `removeProtector(null)` (`:311-321`) and `Creaking.tearDown`
/// the bound protector leaves with the heart.
#[test]
fn breaking_a_bound_creaking_heart_tears_its_protector_down() {
    let (sender, receiver, mut world) = helpers::joined_world(datapack_config());
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        named_block_state(
            "minecraft:creaking_heart",
            &[
                ("axis", "y"),
                ("creaking_heart_state", "dormant"),
                ("natural", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let protector_uuid = Uuid::from_u128(0x1234_5678_9abc_def0_1234_5678_9abc_def0);
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(BlockEntity::creaking_heart_mut)
        .expect("the write created the heart's block entity")
        .set_protector_for_test(protector_uuid);
    let protector = world
        .restore_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            protector_uuid,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: 1.5,
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
                )
                .expect("the creaking has catalog data"),
            ),
        )
        .expect("the protector is stored");

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let published = world.tick(receiver.drain());

    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, protector)
            .is_none(),
        "the protector was removed"
    );
    // `Creaking.tearDown` sends two block-crumble bursts and the death sound.
    let particles = published
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::LevelParticles(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(particles, 2, "one burst per block state");
    assert!(
        sounds(&published).iter().any(|sound| matches!(
            &sound.sound,
            SoundEventHolder::Reference { registry_id }
                if *registry_id
                    == vanilla_sound_event_registry_id("minecraft:entity.creaking.death")
                        .expect("the death sound is in the catalog")
        )),
        "{:?}",
        sounds(&published)
    );
}

/// An unbound heart answers `None` (`removeProtector`'s guard,
#[test]
fn breaking_an_unbound_creaking_heart_leaves_the_world_alone() {
    let (sender, receiver, mut world) = helpers::joined_world(datapack_config());
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 2.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        named_block_state(
            "minecraft:creaking_heart",
            &[
                ("axis", "y"),
                ("creaking_heart_state", "dormant"),
                ("natural", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let bystander = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
            Vec3d {
                x: 1.5,
                y: 64.0,
                z: 1.5,
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CREAKING_ID,
                )
                .expect("the creaking has catalog data"),
            ),
        )
        .0;

    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        air_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let published = world.tick(receiver.drain());

    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, bystander)
            .is_some(),
        "an unbound heart tears nothing down"
    );
    assert!(sounds(&published).is_empty(), "{:?}", sounds(&published));
}
