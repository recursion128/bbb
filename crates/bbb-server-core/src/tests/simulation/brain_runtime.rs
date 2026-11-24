//! The Brain runtime on the real Mob tick path: `minecraft:frog`, the first
//! type whose brain this server builds.

use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{EntityData, GeneralLivingData};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROAKING_ID, VANILLA_ENTITY_POSE_USING_TONGUE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_SLIME_ID,
};
use bbb_protocol::packets::{BlockPos, DebugBrainDump, Vec3d};
use uuid::Uuid;

use super::helpers::{
    admin_command as command, brain_dumps, creaking_memory_line as memory_line,
    restore_saved_entities,
};
use super::*;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const FROG_POSITION: Vec3d = Vec3d {
    x: 2.5,
    y: 65.0,
    z: 4.5,
};
const FROG_BLOCK: BlockPos = BlockPos { x: 2, y: 65, z: 4 };
/// The long-jump cooldown `FrogAi.initMemories` writes at spawn
/// (`FrogAi.java:63-65`), which no activity may run the long jump under
/// (`FrogAi.java:191`).
const SPAWN_COOLDOWN: i32 = 100;

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    helpers::restored_world(
        operator_config(&[("alice", 1)]),
        &restore_candidate(0, None, Vec::new()),
    )
}

fn subscribe(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID],
        },
    )
}

/// Ticks `count` times and returns the last tick's result, which is where the
/// dump of the final state is published.
fn tick_for(receiver: &WorldCommandReceiver, world: &mut ServerWorld, count: usize) -> TickResult {
    let mut last = world.tick(receiver.drain());
    for _ in 1..count {
        last = world.tick(receiver.drain());
    }
    last
}

/// Ticks `count` times and returns the last dump the mob published, which is
/// the last tick its value changed on: a tick that changes nothing publishes
/// nothing (`DebugSubscriptions.publish`).
fn tick_and_read(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    entity_id: i32,
    count: usize,
) -> DebugBrainDump {
    let mut last = None;
    for _ in 0..count {
        let result = world.tick(receiver.drain());
        if let Some(dump) = brain_dumps(&result).remove(&entity_id) {
            last = Some(dump);
        }
    }
    last.unwrap_or_else(|| panic!("the mob published no brain in {count} ticks"))
}

fn spawn_frog(world: &mut ServerWorld) -> (i32, Uuid) {
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_FROG_ID,
        FROG_POSITION,
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_FROG_ID).expect("the frog is living"),
        ),
    );
    set_cooldown(world, id, SPAWN_COOLDOWN);
    (id, uuid)
}

/// The brain memory `FrogAi.initMemories` writes at spawn, through the same
/// production writer `finalizeSpawn` uses.
fn set_cooldown(world: &mut ServerWorld, entity_id: i32, cooldown: i32) {
    let entity = world
        .stored_entity_mut_for_test(DIMENSION, entity_id)
        .expect("the frog is live");
    assert!(crate::entity::set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::LongJumpCooldownTicks,
            value: MemoryValue::Int(cooldown),
            time_to_live: None,
        },
    ));
}

/// `minecraft:water` at `level=0`, which is the source state a placed bucket
/// leaves (`FlowingFluidBlock`; `minecraft:water`'s default already is level 0).
fn source_water() -> i32 {
    helpers::state_with("minecraft:water", &[("level", "0")])
}

/// The pool the water fixtures hold the frog in: three water layers over the
/// generated floor, `POOL_RADIUS` blocks out in every direction.
///
/// A puddle next to dry land cannot hold a frog. `TryFindLand` triggers on the
/// fluid at the body's own block and not on the `is_in_water` memory
/// and walks it to the first cell in reach that is
/// not water and has a sturdy block under it -- the dry shore,
/// whether or not the water sensor has scanned yet. `RandomStroll`'s land
/// sample searches the same way out to ten blocks and
/// refuses only water, so the shore has to lie
/// past that reach. The floor stays generated: water over a sturdy block is
/// never a landing spot, and an air cell over water fails `GoalUtils.isNotStable`
/// which is what keeps the cells the sample draws
/// above and below the frog out of the search too.
fn fill_water(world: &mut ServerWorld, filled: bool) {
    const POOL_RADIUS: i32 = 12;
    let water = source_water();
    let air = block_state_id("minecraft:air");
    for x in (FROG_BLOCK.x - POOL_RADIUS)..=(FROG_BLOCK.x + POOL_RADIUS) {
        for z in (FROG_BLOCK.z - POOL_RADIUS)..=(FROG_BLOCK.z + POOL_RADIUS) {
            for y in [FROG_BLOCK.y - 1, FROG_BLOCK.y, FROG_BLOCK.y + 1] {
                let _ = world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    if filled { water } else { air },
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    }
}

/// Any frog UUID; every sensor scans on the brain's first tick.
fn uuid_scanning_food_after_landing() -> Uuid {
    Uuid::from_u128(0xf206)
}

/// The entity sounds one tick published to the nearby player, as
/// `(registry id, volume, pitch)`.
fn entity_sounds(tick: &TickResult) -> Vec<(String, f32, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SoundEntity(sound),
                ..
            } => Some(sound),
            _ => None,
        })
        .map(|sound| {
            let SoundEventHolder::Reference { registry_id } = sound.sound else {
                panic!("a frog sound is a registry reference")
            };
            (
                bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                    .expect("a built-in sound event")
                    .to_string(),
                sound.volume,
                sound.pitch,
            )
        })
        .collect()
}

/// Slime: saved Size is one less than the live size.
fn spawn_food_slime(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let (slime_id, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        position,
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_SLIME_ID).expect("the slime is living"),
        ),
    );
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(DIMENSION, slime_id)
        .expect("the slime has a record")
    else {
        panic!("an entity record is a compound")
    };
    let size = entries
        .iter_mut()
        .find(|entry| entry.name == "Size")
        .expect("a slime record carries its size");
    size.value = bbb_protocol::NbtSummaryValue::Int(0);
    // A slime with its AI hops off the two-block floor before the tongue lands.
    entries.retain(|entry| entry.name != "NoAI");
    entries.push(bbb_protocol::NbtSummaryEntry {
        name: "NoAI".into(),
        value: bbb_protocol::NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(DIMENSION, slime_id, &entries);
    slime_id
}

/// The stone the frog and its food stand on.
fn lay_floor(world: &mut ServerWorld, xs: std::ops::RangeInclusive<i32>) {
    let floor = block_state_id("minecraft:stone");
    for x in xs {
        let _ = world.set_block(
            DIMENSION,
            BlockPos { x, y: 64, z: 4 },
            floor,
            SET_BLOCK_UPDATE_ALL,
        );
    }
}

fn assert_frog_passenger(world: &ServerWorld, frog: i32, passenger: i32, scale: f32) {
    assert_eq!(
        world.stored_passengers_for_test(DIMENSION, frog),
        &[passenger]
    );
    let frog = world.stored_entity_for_test(DIMENSION, frog).unwrap();
    assert_eq!(
        frog.living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_SCALE_ID) as f32,
        scale
    );
    let attachment = bbb_protocol::entity_types::vanilla_entity_type_passenger_attachments(
        VANILLA_ENTITY_TYPE_FROG_ID,
    )
    .unwrap()[0];
    let attachment = attachment.map(|coordinate| coordinate * scale);
    let (sin, cos) = (-f64::from(frog.y_rot()).to_radians()).sin_cos();
    let origin = frog.position();
    let expected = Vec3d {
        x: origin.x + f64::from(attachment[0]) * cos + f64::from(attachment[2]) * sin,
        y: origin.y + f64::from(attachment[1]),
        z: origin.z + f64::from(attachment[2]) * cos - f64::from(attachment[0]) * sin,
    };
    let actual = world
        .stored_entity_for_test(DIMENSION, passenger)
        .unwrap()
        .position();
    assert!(
        (actual.x - expected.x).abs() < 1e-6
            && (actual.y - expected.y).abs() < 1e-6
            && (actual.z - expected.z).abs() < 1e-6,
        "passenger={actual:?}, attachment={expected:?}, pose={}",
        frog.pose_id()
    );
}

#[test]
fn frog_brain_walk_target_reads_dynamic_water_path_tags() {
    fn walk_target(
        water_is_lava_tagged: bool,
        flowing_water_is_water_tagged: bool,
    ) -> (Vec<String>, Option<String>) {
        let (sender, receiver, mut world) = fixture();
        let _ = subscribe(&sender, &receiver, &mut world);
        let (frog_id, _) = spawn_frog(&mut world);
        fill_water(&mut world, true);
        let mut lava = vec![
            "minecraft:lava".to_owned(),
            "minecraft:flowing_lava".to_owned(),
        ];
        if water_is_lava_tagged {
            lava.push("minecraft:water".to_owned());
        }
        let mut water = vec!["minecraft:water".to_owned()];
        if flowing_water_is_water_tagged {
            water.push("minecraft:flowing_water".to_owned());
        }
        let tags = crate::registry_tags::RegistryTagSnapshot::new([
            (
                "minecraft:fluid".to_owned(),
                "minecraft:water".to_owned(),
                water,
            ),
            (
                "minecraft:fluid".to_owned(),
                "minecraft:lava".to_owned(),
                lava,
            ),
        ]);
        world.install_level_registry_tags(std::sync::Arc::new(tags));
        let tick = tick_for(&receiver, &mut world, 25);
        let dump = brain_dumps(&tick);
        let frog = dump.get(&frog_id).expect("the frog publishes its brain");
        (
            frog.activities.clone(),
            memory_line(frog, "walk_target").map(str::to_owned),
        )
    }

    let builtin = walk_target(false, true);
    let still_swimmable = walk_target(false, false);
    let changed = walk_target(true, true);
    assert_eq!(builtin.0, ["core", "swim"]);
    assert_eq!(still_swimmable.0, ["core", "swim"]);
    assert_eq!(changed.0, ["core", "swim"]);
    assert!(
        builtin
            .1
            .as_deref()
            .is_some_and(|line| line != "walk_target: -"),
        "{builtin:?}"
    );
    assert_eq!(still_swimmable.1, builtin.1);
    assert_ne!(
        changed.1, builtin.1,
        "the Frog Brain WALK_TARGET must consume the dynamic path rule"
    );
}

#[test]
fn a_real_frog_swim_path_drives_its_smooth_control() {
    let (sender, receiver, mut world) = fixture();
    let _ = subscribe(&sender, &receiver, &mut world);
    let (frog_id, _) = spawn_frog(&mut world);
    fill_water(&mut world, true);
    let _ = world.tick(receiver.drain());
    let frog = world.stored_entity_for_test(DIMENSION, frog_id).unwrap();
    // Smooth control adds buoyancy before Frog's water travel moves and damps it.
    assert_eq!(frog.position().y, FROG_POSITION.y + 0.005);
    assert_eq!(frog.delta_movement().y, 0.005 * 0.9);

    let mut driven = None;
    let mut sent_rotation = false;
    for _ in 2..=200 {
        let result = world.tick(receiver.drain());
        sent_rotation |= result.events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::MoveEntity(update), .. }
                    if update.id == frog_id && update.y_rot.is_some() && update.x_rot.is_some()
            )
        });
        let frog = world.stored_entity_for_test(DIMENSION, frog_id).unwrap();
        let living = frog.living().unwrap();
        if driven.is_none() && living.speed() > 0.0 {
            driven = Some((frog.y_rot(), living.y_body_rot, living.speed(), living.zza));
        }
        if driven.is_some() && sent_rotation {
            break;
        }
    }
    let (yaw, body_yaw, speed, forward) = driven.expect("the frog swims to a target");
    // The body turns with the head while swimming.
    assert_eq!(yaw, body_yaw);
    assert_eq!(speed, 0.02);
    assert!(forward > 0.0);
    assert!(sent_rotation);

    fill_water(&mut world, false);
    let _ = world.tick(receiver.drain());
    let frog = world.stored_entity_for_test(DIMENSION, frog_id).unwrap();
    assert!(!frog.is_in_water());
    assert!(frog.delta_movement().y < 0.0);
}

#[test]
fn a_no_ai_frog_keeps_the_brain_its_save_gave_it_and_never_steps_it() {
    let (sender, receiver, mut world) = fixture();
    let _ = subscribe(&sender, &receiver, &mut world);
    let (frog_id, uuid) = spawn_frog(&mut world);
    fill_water(&mut world, true);
    // `Mob.isNoAi` through `/data merge`, which is how a save and a command
    // both hand the flag in (`Mob`, `Mob.isEffectiveAi`)
    let merged = command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    let mut published: Vec<DebugBrainDump> =
        brain_dumps(&merged).remove(&frog_id).into_iter().collect();
    for _ in 0..25 {
        let result = world.tick(receiver.drain());
        published.extend(brain_dumps(&result).remove(&frog_id));
    }
    assert!(
        world
            .stored_entity_for_test(DIMENSION, frog_id)
            .expect("the frog is live")
            .is_in_water(),
        "the fixture put the frog in water"
    );
    // No tick reached the brain, so every dump is the brain the save holds: the
    // spawn cooldown untouched and no sensor memory.
    assert!(
        !published.is_empty(),
        "a no-AI frog still publishes the brain its save holds"
    );
    for dump in &published {
        assert_eq!(memory_line(dump, "is_in_water"), Some("is_in_water: -"));
        assert_eq!(
            memory_line(dump, "long_jump_cooling_down"),
            Some("long_jump_cooling_down: Int(100)")
        );
        assert_eq!(dump.activities, ["core", "idle"]);
        assert!(dump.behaviors.is_empty());
    }
}

#[test]
fn the_water_memory_and_the_live_cooldown_survive_a_save_and_a_reload() {
    let (sender, receiver, mut world) = fixture();
    let _ = subscribe(&sender, &receiver, &mut world);
    let (frog_id, uuid) = spawn_frog(&mut world);
    fill_water(&mut world, true);
    let wet = tick_for(&receiver, &mut world, 25);
    let dump = brain_dumps(&wet);
    let frog = dump.get(&frog_id).expect("the frog publishes its brain");
    assert_eq!(memory_line(frog, "is_in_water"), Some("is_in_water: Unit"));

    // What the save writes is the live brain's own pack: the sensor's memory
    // is there and the cooldown is the counted-down value, not the 100 the
    // mob spawned with.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    let write = request
        .entity_writes
        .iter()
        .find(|write| (write.chunk_x, write.chunk_z) == (0, 0))
        .expect("the frog's chunk is written");
    let chunk = Java26EntityChunkData::parse(write.document.clone(), write.chunk_x, write.chunk_z)
        .expect("the entity region parses");
    let record = chunk
        .entities()
        .find(|record| record.uuid() == uuid)
        .expect("the frog is in the region")
        .raw()
        .to_vec();
    assert_eq!(
        record
            .iter()
            .find_map(
                |entry| match (entry.name.to_jdk_string().as_str(), &entry.value) {
                    ("Brain", bbb_protocol::NbtSummaryValue::Compound(brain)) => Some(brain),
                    _ => None,
                }
            )
            .and_then(|brain| {
                brain.iter().find_map(|entry| {
                    match (entry.name.to_jdk_string().as_str(), &entry.value) {
                        ("memories", bbb_protocol::NbtSummaryValue::Compound(memories)) => {
                            Some(memories)
                        }
                        _ => None,
                    }
                })
            })
            .expect("the record carries a Brain with memories"),
        &[
            bbb_protocol::NbtSummaryEntry {
                name: "minecraft:is_in_water".into(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "value".into(),
                        value: bbb_protocol::NbtSummaryValue::Compound(Vec::new()),
                    },
                ]),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "minecraft:long_jump_cooling_down".into(),
                value: bbb_protocol::NbtSummaryValue::Compound(vec![
                    bbb_protocol::NbtSummaryEntry {
                        name: "value".into(),
                        value: bbb_protocol::NbtSummaryValue::Int(75),
                    },
                ]),
            },
        ]
    );
    drop(world);

    // A restart: the entity region is parsed back and the world restored.
    let mut candidate = restore_candidate(0, None, Vec::new());
    restore_saved_entities(&mut candidate, &request.entity_writes);
    let (sender, receiver) = mailbox();
    let mut restored = spawn_pinned_restore(operator_config(&[("alice", 1)]), &candidate);
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    // The session has to track the frog for the channel to publish it.
    stand_at(&sender, &receiver, &mut restored, 1, FROG_POSITION);
    let frog_id = restored
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            restored
                .stored_entity_for_test(DIMENSION, *id)
                .is_some_and(|entity| entity.uuid() == uuid)
        })
        .expect("the frog reloads");
    let _ = subscribe(&sender, &receiver, &mut restored);

    // The brain the reload builds starts from the saved pack: the cooldown is
    // the counted-down value the save wrote rather than the 100 the mob
    // spawned with.
    let first = tick_and_read(&receiver, &mut restored, frog_id, 1);
    let first_cooldown = memory_line(&first, "long_jump_cooling_down")
        .expect("the cooldown is published")
        .to_owned();
    assert!(
        first_cooldown != "long_jump_cooling_down: Int(100)"
            && first_cooldown != "long_jump_cooling_down: Int(75)",
        "the reloaded brain starts where the save left it: {first_cooldown}"
    );

    // And it counts down a tick at a time from there.
    let later = tick_and_read(&receiver, &mut restored, frog_id, 3);
    let later_value: i32 = memory_line(&later, "long_jump_cooling_down")
        .expect("the cooldown is published")
        .rsplit(' ')
        .next()
        .and_then(|value| value.strip_prefix("Int(")?.strip_suffix(')')?.parse().ok())
        .expect("the cooldown prints a number");
    let first_value: i32 = first_cooldown
        .rsplit(' ')
        .next()
        .and_then(|value| value.strip_prefix("Int(")?.strip_suffix(')')?.parse().ok())
        .expect("the cooldown prints a number");
    assert_eq!(later_value, first_value - 3);
}

#[test]
fn a_real_frog_targets_catches_and_eats_the_small_slime_next_to_it() {
    use crate::tests::entities::console_command_answer;

    let target_condition = serde_json::json!({
        "condition":"minecraft:entity_properties", "entity":"this",
        "predicate":{"type":"minecraft:frog","targeted_entity":{"type":"minecraft:slime"}}
    });
    let mut wrong_condition = target_condition.clone();
    wrong_condition["predicate"]["targeted_entity"]["type"] = serde_json::json!("minecraft:cow");
    let table = serde_json::json!({"type":"minecraft:entity","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[target_condition.clone()],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]},
        {"rolls":1,"conditions":[wrong_condition],"entries":[{"type":"minecraft:item","name":"minecraft:coal"}]},
        {"rolls":1,"conditions":[
            {"condition":"minecraft:entity_properties","entity":"this","predicate":{"type":"minecraft:frog"}},
            {"condition":"minecraft:inverted","term":target_condition}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]}
    ]});
    for (saved_size, dies) in [(0, false), (1, false), (0, true)] {
        let (sender, receiver) = mailbox();
        let mut config = operator_config(&[("alice", 1)]);
        config.datapacks =
            datapack_config_with_loot_table("minecraft:entities/frog", &table).datapacks;
        let mut world = spawn_pinned_restore(config, &restore_candidate(0, None, Vec::new()));
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        lay_floor(&mut world, 2..=3);
        let frog_id = world
            .restore_stored_entity_for_test(
                DIMENSION,
                uuid_scanning_food_after_landing(),
                VANILLA_ENTITY_TYPE_FROG_ID,
                FROG_POSITION,
                EntityData::GeneralLiving(
                    GeneralLivingData::new(VANILLA_ENTITY_TYPE_FROG_ID)
                        .expect("the frog is living"),
                ),
            )
            .expect("the frog restores");
        set_cooldown(&mut world, frog_id, SPAWN_COOLDOWN);
        let slime_id = spawn_food_slime(
            &mut world,
            Vec3d {
                x: 3.5,
                y: 65.0,
                z: 4.5,
            },
        );
        if saved_size == 1 {
            let NbtSummaryValue::Compound(mut entries) = world
                .stored_entity_command_data_for_test(DIMENSION, slime_id)
                .expect("the slime has a record")
            else {
                panic!("an entity record is a compound")
            };
            let size = entries
                .iter_mut()
                .find(|entry| entry.name == "Size")
                .unwrap();
            size.value = NbtSummaryValue::Int(saved_size);
            world.load_stored_entity_record_for_test(DIMENSION, slime_id, &entries);
        }
        assert_eq!(record_int(&world, slime_id, "Size"), saved_size);
        let EntityData::GeneralLiving(slime) = world
            .stored_entity_for_test(DIMENSION, slime_id)
            .unwrap()
            .data()
        else {
            panic!("the slime is living")
        };
        assert_eq!(slime.slime_retained().unwrap().size(), saved_size + 1);

        let mut caught = false;
        for _ in 0..80 {
            let tick = world.tick(receiver.drain());
            let sounds = entity_sounds(&tick);
            assert!(sounds
                .iter()
                .all(|(name, _, _)| name != "minecraft:entity.frog.eat"));
            if saved_size == 1 {
                assert!(sounds
                    .iter()
                    .all(|(name, _, _)| name != "minecraft:entity.frog.tongue"));
            } else if let Some((_, volume, pitch)) = sounds
                .iter()
                .find(|(name, _, _)| name == "minecraft:entity.frog.tongue")
            {
                assert_eq!((*volume, *pitch), (2.0, 1.0));
                caught = true;
                break;
            }
        }
        if saved_size == 1 {
            assert!(world
                .stored_entity_for_test(DIMENSION, slime_id)
                .is_some_and(|slime| slime.is_alive()));
            continue;
        }
        assert!(caught, "the frog catches the size-one slime");
        assert_eq!(
            world
                .stored_entity_for_test(DIMENSION, frog_id)
                .unwrap()
                .pose_id(),
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_USING_TONGUE_ID
        );
        assert!(world
            .stored_entity_for_test(DIMENSION, slime_id)
            .is_some_and(|slime| slime.is_alive()));

        let frozen = command(&sender, &receiver, &mut world, 100, "tick freeze");
        assert_eq!(console_command_answer(&frozen).0, Some(1));
        let frog_uuid = world
            .stored_entity_for_test(DIMENSION, frog_id)
            .unwrap()
            .uuid();
        let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:frog',targeted_entity:{type:'minecraft:slime'}}}";
        let positive = format!("execute as {frog_uuid} if predicate {predicate}");
        let wrong = positive.replace("minecraft:slime", "minecraft:cow");
        let inverted = format!("execute as {frog_uuid} if predicate {{condition:'minecraft:inverted',term:{predicate}}}");
        let relation =
            format!("execute as {frog_uuid} on target if entity @s[type=minecraft:slime]");
        for (operation, text, expected) in [
            (101, &positive, Some(1)),
            (102, &wrong, None),
            (103, &inverted, None),
            (104, &relation, Some(1)),
        ] {
            let result = command(&sender, &receiver, &mut world, operation, text);
            assert_eq!(console_command_answer(&result).0, expected, "{text}");
        }
        assert!(world
            .stored_entity_for_test(DIMENSION, slime_id)
            .is_some_and(|slime| slime.is_alive()));
        if dies {
            let killed = command(
                &sender,
                &receiver,
                &mut world,
                105,
                &format!("kill {frog_uuid}"),
            );
            assert_eq!(console_command_answer(&killed).0, Some(1));
            assert_eq!(
                world
                    .stored_entity_for_test(DIMENSION, frog_id)
                    .unwrap()
                    .living()
                    .unwrap()
                    .health(),
                0.0
            );
            assert!(world
                .stored_entity_for_test(DIMENSION, slime_id)
                .is_some_and(|slime| slime.is_alive()));
            let mut drops = world
                .stored_items_for_test(DIMENSION)
                .into_iter()
                .map(|(_, _, id, count, ..)| (id, count))
                .collect::<Vec<_>>();
            drops.sort();
            assert_eq!(
                drops,
                vec![
                    ("minecraft:apple".to_owned(), 1),
                    ("minecraft:diamond".to_owned(), 1)
                ]
            );
            continue;
        }
        let mut eaten = command(&sender, &receiver, &mut world, 105, "tick unfreeze");
        assert_eq!(console_command_answer(&eaten).0, Some(0));
        for _ in 0..80 {
            if world.stored_entity_for_test(DIMENSION, slime_id).is_none() {
                break;
            }
            eaten.events.extend(world.tick(receiver.drain()).events);
        }
        assert_eq!(
            entity_sounds(&eaten)
                .into_iter()
                .filter(|(name, _, _)| name == "minecraft:entity.frog.eat")
                .collect::<Vec<_>>(),
            vec![("minecraft:entity.frog.eat".to_string(), 2.0, 1.0)]
        );
        assert!(
            world.stored_entity_for_test(DIMENSION, slime_id).is_none(),
            "the size-one slime the frog bit is gone"
        );
        let children = eaten
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        packet: ClientboundPlayPacket::AddEntity(add),
                        ..
                    } if add.entity_type_id == VANILLA_ENTITY_TYPE_SLIME_ID
                )
            })
            .count();
        assert_eq!(children, 0, "a size-one slime has no split children");
        let frozen = command(&sender, &receiver, &mut world, 106, "tick freeze");
        assert_eq!(console_command_answer(&frozen).0, Some(1));
        for (operation, text, expected) in [
            (107, &positive, None),
            (108, &inverted, Some(1)),
            (109, &relation, None),
        ] {
            let result = command(&sender, &receiver, &mut world, operation, text);
            assert_eq!(console_command_answer(&result).0, expected, "{text}");
        }
    }
}

#[test]
fn a_scaled_frogs_passenger_follows_croaking_and_tongue_then_reloads() {
    let prepare_floor = |world: &mut ServerWorld| {
        let stone = block_state_id("minecraft:stone");
        for x in 0..16 {
            for z in 0..16 {
                world.set_block(
                    DIMENSION,
                    BlockPos { x, y: 64, z },
                    stone,
                    SET_BLOCK_UPDATE_ALL,
                );
            }
        }
    };
    let (sender, receiver, mut world) = fixture();
    world.tick_rate_mut().set_frozen(true);
    prepare_floor(&mut world);
    let (frog, uuid) = spawn_frog(&mut world);
    for (id, text) in [
        (201, format!("attribute {uuid} minecraft:scale base set 2")),
        (
            202,
            "summon minecraft:armor_stand 3.5 66 4.5 {Small:1b,NoGravity:1b,Invulnerable:1b}"
                .to_string(),
        ),
    ] {
        let _ = command(&sender, &receiver, &mut world, id, &text);
    }
    let passenger = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        })
        .unwrap();
    let passenger_uuid = world
        .stored_entity_for_test(DIMENSION, passenger)
        .unwrap()
        .uuid();
    let _ = command(
        &sender,
        &receiver,
        &mut world,
        203,
        &format!("ride {passenger_uuid} mount {uuid}"),
    );
    world.tick_rate_mut().set_frozen(false);
    let mut croaked = false;
    for _ in 0..600 {
        world.tick(receiver.drain());
        if world
            .stored_entity_for_test(DIMENSION, frog)
            .unwrap()
            .pose_id()
            == VANILLA_ENTITY_POSE_CROAKING_ID
        {
            assert_frog_passenger(&world, frog, passenger, 2.0);
            croaked = true;
            break;
        }
    }
    assert!(croaked, "the mounted frog naturally croaks");
    world.tick_rate_mut().set_frozen(true);
    let position = world
        .stored_entity_for_test(DIMENSION, frog)
        .unwrap()
        .position();
    let food = spawn_food_slime(
        &mut world,
        Vec3d {
            x: position.x + if position.x < 8.0 { 1.0 } else { -1.0 },
            ..position
        },
    );
    let food_uuid = world
        .stored_entity_for_test(DIMENSION, food)
        .unwrap()
        .uuid();
    world.tick_rate_mut().set_frozen(false);
    let mut caught = false;
    for _ in 0..120 {
        let tick = world.tick(receiver.drain());
        assert_frog_passenger(&world, frog, passenger, 2.0);
        if world
            .stored_entity_for_test(DIMENSION, frog)
            .unwrap()
            .pose_id()
            == VANILLA_ENTITY_POSE_USING_TONGUE_ID
        {
            assert!(entity_sounds(&tick)
                .iter()
                .any(|(name, _, _)| name == "minecraft:entity.frog.tongue"));
            caught = true;
            break;
        }
    }
    assert!(caught, "the mounted frog catches its real food");
    let request = world
        .save_request(1_786_000_014_003, 217, RegionCompression::Zlib)
        .unwrap();
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_saved_entities(&mut candidate, &request.entity_writes);
    let (sender, receiver) = mailbox();
    let mut restored = spawn_pinned_restore(operator_config(&[("alice", 1)]), &candidate);
    let frog = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .unwrap();
    let passenger = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, passenger_uuid)
        .unwrap();
    assert_eq!(
        restored.stored_passengers_for_test(DIMENSION, frog),
        &[passenger]
    );
    assert!(restored
        .live_entity_id_for_uuid_for_test(DIMENSION, food_uuid)
        .is_some());
    restored.tick_rate_mut().set_frozen(true);
    prepare_floor(&mut restored);
    join_loaded(&sender, &receiver, &mut restored, 1, "alice", 1);
    restored.tick_rate_mut().set_frozen(false);
    let mut resumed = false;
    for _ in 0..120 {
        let tick = restored.tick(receiver.drain());
        assert_frog_passenger(&restored, frog, passenger, 2.0);
        if entity_sounds(&tick)
            .iter()
            .any(|(name, _, _)| name == "minecraft:entity.frog.eat")
        {
            assert!(restored
                .live_entity_id_for_uuid_for_test(DIMENSION, food_uuid)
                .is_none());
            resumed = true;
            break;
        }
    }
    assert!(resumed, "the restored mounted frog resumes eating");
}

#[test]
fn a_real_frog_ignores_food_outside_the_reach_of_its_tongue_sensor() {
    let (sender, receiver, mut world) = fixture();
    let _ = subscribe(&sender, &receiver, &mut world);
    lay_floor(&mut world, 2..=3);
    let uuid = uuid_scanning_food_after_landing();
    let frog_id = world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid,
            VANILLA_ENTITY_TYPE_FROG_ID,
            FROG_POSITION,
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_FROG_ID).expect("the frog is living"),
            ),
        )
        .expect("the frog restores");
    set_cooldown(&mut world, frog_id, SPAWN_COOLDOWN);
    // Eleven blocks east: outside `FrogAttackablesSensor`'s ten-block reach
    // so the frog never takes a target.
    let slime_id = spawn_food_slime(
        &mut world,
        Vec3d {
            x: 13.5,
            y: 65.0,
            z: 4.5,
        },
    );
    let _ = tick_for(&receiver, &mut world, 2);

    let mut sounds = Vec::new();
    let mut dumps = Vec::new();
    for _ in 0..25 {
        let result = world.tick(receiver.drain());
        sounds.extend(entity_sounds(&result));
        if let Some(frog) = brain_dumps(&result).remove(&frog_id) {
            dumps.push(frog);
        }
    }
    assert!(sounds.is_empty(), "{sounds:?}");
    let frog = dumps.last().expect("the frog publishes its brain");
    assert_eq!(memory_line(frog, "attack_target"), Some("attack_target: -"));
    assert_eq!(
        memory_line(frog, "nearest_attackable"),
        Some("nearest_attackable: -")
    );
    assert_eq!(frog.activities, ["core", "idle"]);
    assert!(world
        .stored_entity_for_test(DIMENSION, slime_id)
        .is_some_and(|slime| slime.is_alive()));
}

/// One integer field of a stored entity's own saved record.
fn record_int(world: &ServerWorld, entity_id: i32, name: &str) -> i32 {
    let bbb_protocol::NbtSummaryValue::Compound(entries) = world
        .stored_entity_command_data_for_test(DIMENSION, entity_id)
        .expect("the entity has a record")
    else {
        panic!("an entity record is a compound")
    };
    match entries.iter().find(|entry| entry.name == name) {
        Some(bbb_protocol::NbtSummaryEntry {
            value: bbb_protocol::NbtSummaryValue::Int(value),
            ..
        }) => *value,
        other => panic!("{name} is an int field: {other:?}"),
    }
}

/// Writes `InLove` into one stored animal's own record, which is the save path
/// `Animal.readAdditionalSaveData` reads. The
/// player-feeding `Animal.mobInteract` branch is the production
/// writer of the same field (`animal_feeding.rs`), so this is the fixture's
/// stand-in for a frog that was already in love before the tick under test.
fn set_in_love(world: &mut ServerWorld, entity_id: i32, value: i32) {
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(DIMENSION, entity_id)
        .expect("the frog has a record")
    else {
        panic!("an entity record is a compound")
    };
    let entry = entries
        .iter_mut()
        .find(|entry| entry.name == "InLove")
        .expect("an animal record carries InLove");
    entry.value = bbb_protocol::NbtSummaryValue::Int(value);
    world.load_stored_entity_record_for_test(DIMENSION, entity_id, &entries);
}

/// One entity event the world sent this tick, by event id.
fn entity_events(tick: &TickResult, event_id: i8) -> usize {
    tick.events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityEvent(packet),
                    ..
                } if packet.event_id == event_id
            )
        })
        .count()
}

/// How many experience orbs this tick told the session about.
fn orb_spawns(tick: &TickResult) -> usize {
    tick.events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::AddEntity(add),
                    ..
                } if add.entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
            )
        })
        .count()
}

#[test]
fn two_in_love_frogs_mate_once_on_the_real_tick() {
    let (sender, receiver, mut world) = fixture();
    let _ = subscribe(&sender, &receiver, &mut world);
    // A floor wide enough for either frog's own stroll to find a stand on
    // (`LandRandomPos.getPos`), so neither falls out of
    // the world before the pair meets.
    lay_floor(&mut world, -8..=12);
    let (first_id, _) = spawn_frog(&mut world);
    let (second_id, _) = spawn_frog(&mut world);
    // Both frogs are already in love, which is the state `canMate` needs on
    // both sides.
    set_in_love(&mut world, first_id, 600);
    set_in_love(&mut world, second_id, 600);

    let mut hearts = 0;
    let mut orbs = 0;
    let mut dumps: std::collections::BTreeMap<i32, DebugBrainDump> =
        std::collections::BTreeMap::new();
    for _ in 0..200 {
        let result = world.tick(receiver.drain());
        hearts += entity_events(&result, 18);
        orbs += orb_spawns(&result);
        for (entity_id, dump) in brain_dumps(&result) {
            dumps.insert(entity_id, dump);
        }
        if orbs > 0 {
            break;
        }
    }

    // Exactly one mating: `AnimalMakeLove` writes `BREED_TARGET` on both halves
    // and the entry condition asks for an absent
    // one, so only the frog that started first ever reaches
    // `spawnChildFromBreeding`. The heart event and the orb are the
    // two things `finalizeSpawnChildFromBreeding` broadcasts
    assert_eq!(orbs, 1, "the pair mated {orbs} times");
    assert_eq!(hearts, 1, "the heart event went out {hearts} times");
    // `this.setAge(6000)` and `partner.setAge(6000)` (`:224-225`), each counted
    // down by its own `aiStep` since (`:70-72`, `:198-204`).
    assert!(record_int(&world, first_id, "Age") > 0);
    assert!(record_int(&world, second_id, "Age") > 0);
    // `this.resetLove()` and `partner.resetLove()` (`:226-227`).
    assert_eq!(record_int(&world, first_id, "InLove"), 0);
    assert_eq!(record_int(&world, second_id, "InLove"), 0);

    // `Frog.spawnChildFromBreeding` never builds an
    // offspring: the frog that bred is pregnant, which `FrogAi.updateActivity`
    // reads as the LAY_SPAWN activity.
    let pregnant: Vec<i32> = [first_id, second_id]
        .into_iter()
        .filter(|id| {
            dumps
                .get(id)
                .is_some_and(|dump| memory_line(dump, "is_pregnant") == Some("is_pregnant: Unit"))
        })
        .collect();
    assert_eq!(pregnant.len(), 1, "pregnant frogs: {pregnant:?}");
    let bred = pregnant[0];
    // LAY_SPAWN is chosen ahead of LONG_JUMP, but only once the frog is out of
    // the jump it may have been in when it bred: that activity's own condition
    // asks for an absent `LONG_JUMP_MID_JUMP`, which a jump
    // in flight holds.
    let mut settled = dumps[&bred].clone();
    for _ in 0..80 {
        if settled.activities.iter().any(|name| name == "lay_spawn") {
            break;
        }
        let result = world.tick(receiver.drain());
        if let Some(dump) = brain_dumps(&result).remove(&bred) {
            settled = dump;
        }
    }
    assert_eq!(settled.activities, ["core", "lay_spawn"]);
    assert_eq!(
        memory_line(&settled, "is_pregnant"),
        Some("is_pregnant: Unit")
    );
    // `AnimalMakeLove.tick` erased the pair's `BREED_TARGET` (`:80-81`), so the
    // LONG_JUMP activity is open again.
    for id in [first_id, second_id] {
        assert_eq!(
            memory_line(&dumps[&id], "breed_target"),
            Some("breed_target: -"),
            "frog {id} still holds a breed target"
        );
    }
    assert!(
        world.stored_entity_for_test(DIMENSION, first_id).is_some(),
        "the first frog is still live"
    );
    // The pair is still two frogs: `Frog.spawnChildFromBreeding` added no
    // child, and the orb the mating paid is the only entity that came out of
    // it.
    assert!(
        world.stored_entity_for_test(DIMENSION, second_id).is_some(),
        "the second frog is still live"
    );
}
