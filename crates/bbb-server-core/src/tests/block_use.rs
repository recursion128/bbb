//! Right-clicking a block: note blocks, bells, copper golem statues, bee nests,
//! skulls, copper bulbs, composters, cakes, pumpkins, redstone ore, sweet
//! berry bushes, cave vines, the dragon egg, fence leash binds and cauldron
//! bucket fill/empty.
//!
//! The topic modules below hold the tests; what stays here is the fixtures
//! more than one of them reaches for.

use super::*;

mod bells;
mod block_entities;
mod block_sound;
mod campfire_shape;
mod debug_stick;
mod edibles;
mod hand_use;
mod harvest;
mod interaction_source;
mod item_on_block;
mod placed_by;
mod placement;
mod placement_cells;
mod redstone_placement;
mod spawn_egg;
mod tripwire;

/// The last `ClientboundBlockUpdatePacket` one tick sent for a position.
fn block_update_sent(tick: &TickResult, pos: BlockPos) -> Option<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockUpdate(update),
                ..
            } if update.pos == pos => Some(update.block_state_id),
            _ => None,
        })
        .next_back()
}

/// The sound and block-event packets one tick sent, in event order, named by
/// kind — the two `runBlockEvents` (`ServerLevel.java:1241-1266`) can produce.
fn block_event_packet_order(tick: &TickResult) -> Vec<&'static str> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::Sound(_),
                ..
            } => Some("sound"),
            WorldEvent::Send {
                packet: ClientboundPlayPacket::BlockEvent(_),
                ..
            } => Some("block_event"),
            _ => None,
        })
        .collect()
}

fn note_block_state(instrument: &str, note: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:note_block",
        &BTreeMap::from([
            ("instrument".to_string(), instrument.to_string()),
            ("note".to_string(), note.to_string()),
            ("powered".to_string(), "false".to_string()),
        ]),
    )
    .expect("a built-in note block state")
}

fn placement_state(name: &str, properties: &[(&str, &str)]) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect(),
    )
    .unwrap_or_else(|| panic!("missing built-in placement state {name} {properties:?}"))
}

fn select_placement_item(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    name: &str,
) {
    let item_id = bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in item {name}"))
        .id;
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                ..ItemStackSummary::empty()
            },
        }),
    );
}

fn lectern_item(block_entity_type_id: i32, entries: Vec<NbtSummaryEntry>) -> ItemStackSummary {
    ItemStackSummary {
        item_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:lectern")
            .map(|item| item.id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
            ],
            block_entity_data_type_id: Some(block_entity_type_id),
            block_entity_data_tag: Some(NbtSummaryValue::Compound(entries)),
            ..DataComponentPatchSummary::default()
        }.into(),
    }
}

fn lectern_book_tag() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        nbt("id", NbtSummaryValue::string("minecraft:written_book")),
        nbt("count", NbtSummaryValue::Int(1)),
    ])
}

fn place_block_item(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    hit: BlockPos,
    direction: Direction,
    cursor_y: f32,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: hit,
                direction,
                cursor_x: 0.5,
                cursor_y,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn bell_state() -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("attachment".to_string(), "floor".to_string()),
        ("powered".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:bell",
        &properties,
    )
    .expect("minecraft:bell is a vanilla 26.1 block")
}

fn copper_golem_statue_state(name: &str, pose: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("copper_golem_pose".to_string(), pose.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .unwrap_or_else(|| panic!("{name} {pose} is a vanilla 26.1 block"))
}

fn copper_golem_pose(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get("copper_golem_pose")
        .cloned()
}

fn bee_nest_state(honey: i32) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), "north".to_string()),
        ("honey_level".to_string(), honey.to_string()),
    ]);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:bee_nest",
        &properties,
    )
    .expect("minecraft:bee_nest is a vanilla 26.1 block")
}

fn honey_level(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get("honey_level")
        .cloned()
}

fn player_head_state(powered: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:player_head",
        &BTreeMap::from([
            (
                "powered".to_string(),
                if powered { "true" } else { "false" }.to_string(),
            ),
            ("rotation".to_string(), "0".to_string()),
        ]),
    )
    .expect("a built-in player head")
}

fn skull_powered(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get("powered")
        .cloned()
}

fn copper_bulb_state(name: &str, lit: bool, powered: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &BTreeMap::from([
            (
                "lit".to_string(),
                if lit { "true" } else { "false" }.to_string(),
            ),
            (
                "powered".to_string(),
                if powered { "true" } else { "false" }.to_string(),
            ),
        ]),
    )
    .unwrap_or_else(|| panic!("{name} is a vanilla 26.1 copper bulb"))
}

fn copper_bulb_prop(world: &mut ServerWorld, pos: BlockPos, name: &str) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get(name)
        .cloned()
}

fn composter_state(level: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:composter",
        &BTreeMap::from([("level".to_string(), level.to_string())]),
    )
    .expect("a built-in composter")
}

fn composter_level(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get("level")
        .cloned()
}

fn cake_state(bites: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:cake",
        &BTreeMap::from([("bites".to_string(), bites.to_string())]),
    )
    .expect("a built-in cake")
}

fn cake_bites(world: &mut ServerWorld, pos: BlockPos) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(live_block(world, pos)?)?
        .properties
        .get("bites")
        .cloned()
}

fn armor_stand_item(count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_ARMOR_STAND_ID),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn armor_stand_use(pos: BlockPos, direction: Direction, sequence: i32) -> PlayServerbound {
    PlayServerbound::UseItemOn(UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos,
            direction,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
        sequence,
    })
}

fn armor_stand_adds(tick: &TickResult) -> Vec<&bbb_protocol::packets::AddEntity> {
    tick.events
        .iter()
        .flat_map(WorldEvent::deliveries)
        .filter_map(|(_, packet)| match packet {
            ClientboundPlayPacket::AddEntity(add)
                if add.entity_type_id
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID =>
            {
                Some(add)
            }
            _ => None,
        })
        .collect()
}

fn fence_gate_state(facing: &str, open: bool) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_fence_gate",
        &BTreeMap::from([
            ("facing".to_string(), facing.to_string()),
            ("in_wall".to_string(), "false".to_string()),
            ("open".to_string(), open.to_string()),
            ("powered".to_string(), "false".to_string()),
        ]),
    )
    .expect("a built-in fence gate state")
}

fn candle_state(lit: bool, waterlogged: bool) -> i32 {
    placement_state(
        "minecraft:candle",
        &[
            ("candles", "1"),
            ("lit", if lit { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn candle_cake_state(lit: bool) -> i32 {
    placement_state(
        "minecraft:candle_cake",
        &[("lit", if lit { "true" } else { "false" })],
    )
}

fn campfire_state(lit: bool) -> i32 {
    placement_state(
        "minecraft:campfire",
        &[
            ("facing", "north"),
            ("lit", if lit { "true" } else { "false" }),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    )
}

fn click_use(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    cursor_y: f32,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

fn give_named(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    name: &str,
) {
    select_placement_item(sender, receiver, world, name);
}

/// The fixture inventory starts with stone (`PlayerInventoryMenu::fixture`).
fn clear_hand(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary::empty(),
        }),
    );
}

fn pumpkin_state() -> i32 {
    block_state_id("minecraft:pumpkin")
}

fn carved_pumpkin_state(facing: &str) -> i32 {
    placement_state("minecraft:carved_pumpkin", &[("facing", facing)])
}

fn redstone_ore_state(name: &str, lit: bool) -> i32 {
    placement_state(name, &[("lit", if lit { "true" } else { "false" })])
}

fn sweet_berry_state(age: i32) -> i32 {
    placement_state("minecraft:sweet_berry_bush", &[("age", &age.to_string())])
}

fn click_face(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    direction: Direction,
    sequence: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    )
}

/// `SweetBerryBushBlock.useWithoutItem` (`SweetBerryBushBlock.java:116-141`)
/// harvests age `> 1` through `minecraft:harvest/sweet_berry_bush`.
fn empty_hand_picks_sweet_berries(age: i32, held: Option<&str>) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        sweet_berry_state(age),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    match held {
        Some(name) => give_named(&sender, &receiver, &mut world, name),
        None => clear_hand(&sender, &receiver, &mut world),
    }
    let picked = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(sweet_berry_state(1)));
    assert_eq!(
        block_update_sent(&picked, at),
        Some(sweet_berry_state(1)),
        "{picked:?}"
    );
    assert!(
        sounds_sent(&picked).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.sweet_berry_bush.pick_berries"
                )
                .expect("a registered sound"),
            }),
        "{picked:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
                && event.pos == at
                && event.source_entity_id == Some(entity_id_of(&world, 1))
                && event.affected_state == Some(sweet_berry_state(1))
        }),
        "{events:?}"
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert!(!entities.is_empty(), "{entities:?}");
    assert!(
        entities.iter().any(|entity| {
            let NbtSummaryValue::Compound(stack) = field(entity, "Item") else {
                return false;
            };
            field(stack, "id") == &NbtSummaryValue::string("minecraft:sweet_berries")
        }),
        "{entities:?}"
    );
}

/// Age `<= 1` is `super.useWithoutItem` (`:139-140`), which PASSes.
fn empty_hand_leaves_unripe_sweet_berries(age: i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    let before = sweet_berry_state(age);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        before,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let missed = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(before));
    assert!(sounds_sent(&missed).is_empty(), "{missed:?}");
}

fn cave_vines_head(age: i32, berries: bool) -> i32 {
    placement_state(
        "minecraft:cave_vines",
        &[
            ("age", &age.to_string()),
            ("berries", if berries { "true" } else { "false" }),
        ],
    )
}

fn cave_vines_plant(berries: bool) -> i32 {
    placement_state(
        "minecraft:cave_vines_plant",
        &[("berries", if berries { "true" } else { "false" })],
    )
}

/// Hang a cave vine from a sturdy cell above so `updateShape` does not destroy it.
fn hang_cave_vines(world: &mut ServerWorld, vine: i32) -> BlockPos {
    let support = BlockPos { x: 0, y: 65, z: 1 };
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        vine,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    at
}

/// `CaveVines.use` (`CaveVines.java:23-46`) harvests through `minecraft:harvest/cave_vine`.
fn empty_hand_picks_cave_vines(before: i32, after: i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = hang_cave_vines(&mut world, before);
    clear_hand(&sender, &receiver, &mut world);
    let picked = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(after));
    assert_eq!(block_update_sent(&picked, at), Some(after), "{picked:?}");
    assert!(
        sounds_sent(&picked).iter().any(|sound| sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(
                    "minecraft:block.cave_vines.pick_berries"
                )
                .expect("a registered sound"),
            }),
        "{picked:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
                && event.pos == at
                && event.source_entity_id == Some(entity_id_of(&world, 1))
                && event.affected_state == Some(after)
        }),
        "{events:?}"
    );
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .unwrap();
    let entities = saved_entities(&request);
    assert!(!entities.is_empty(), "{entities:?}");
    assert!(
        entities.iter().any(|entity| {
            let NbtSummaryValue::Compound(stack) = field(entity, "Item") else {
                return false;
            };
            field(stack, "id") == &NbtSummaryValue::string("minecraft:glow_berries")
        }),
        "{entities:?}"
    );
}

fn empty_hand_leaves_cave_vines(state: i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = hang_cave_vines(&mut world, state);
    clear_hand(&sender, &receiver, &mut world);
    let missed = click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    assert_eq!(live_block(&mut world, at), Some(state));
    assert!(sounds_sent(&missed).is_empty(), "{missed:?}");
}

fn dragon_egg_state() -> i32 {
    block_state_id("minecraft:dragon_egg")
}

/// `DragonEggBlock.teleport` (`DragonEggBlock.java:51-79`) against this world's
/// live cells, border and overworld height.
fn dragon_egg_teleport_dest(
    world: &mut ServerWorld,
    random: &crate::random::GameRandom,
    pos: BlockPos,
    border: bbb_server_types::interaction::WorldBorder,
) -> Option<BlockPos> {
    for _ in 0..1000 {
        let dest = BlockPos {
            x: pos.x + random.next_int(16) - random.next_int(16),
            y: pos.y + random.next_int(8) - random.next_int(8),
            z: pos.z + random.next_int(16) - random.next_int(16),
        };
        let dest_state = live_block(world, dest);
        let below = live_block(
            world,
            BlockPos {
                x: dest.x,
                y: dest.y - 1,
                z: dest.z,
            },
        );
        if dest_state.is_some_and(bbb_server_types::fluid::block_state_is_air)
            && !below.is_none_or(bbb_server_types::fluid::block_state_is_air)
            && border.contains_block(dest)
            && dest.y >= -64
            && dest.y <= 319
        {
            return Some(dest);
        }
    }
    None
}

fn oak_fence() -> i32 {
    placement_state(
        "minecraft:oak_fence",
        &[
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ],
    )
}

fn spawn_oak_boat(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let boat = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
        )
        .0;
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    boat
}

fn fence_entity_links(tick: &TickResult) -> Vec<(i32, i32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityLink(update),
                ..
            } => Some((update.source_id, update.dest_id)),
            _ => None,
        })
        .collect()
}

fn fence_swung_main_hand(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(update),
                ..
            } if update.action == 0
        )
    })
}

fn fence_awarded_lead_used(tick: &TickResult) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AwardStats(update),
                ..
            } if update.stats.iter().any(|entry|
                entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                    && entry.value_id == bbb_protocol::item_types::VANILLA_ITEM_LEAD_ID
                    && entry.amount == 1)
        )
    })
}

fn fence_live_boat(world: &ServerWorld, boat: i32) -> &crate::entity::Entity {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .expect("the boat is live")
}

fn fence_live_knots(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter(|&id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .is_some_and(|entity| {
                    matches!(entity.data(), crate::entity::EntityData::LeashKnot(_))
                        && entity.removal_reason().is_none()
                })
        })
        .collect()
}

fn attach_boat_then_clear_hand(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> i32 {
    let player_position = world.publication().players[0].position;
    let boat = spawn_oak_boat(
        world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    give_named(sender, receiver, world, "minecraft:lead");
    let attached = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert_eq!(
        crate::entity::leash::leash_holder(fence_live_boat(world, boat)),
        Some(entity_id_of(world, 1))
    );
    assert!(
        fence_entity_links(&attached).contains(&(boat, entity_id_of(world, 1))),
        "attach links: {:?}",
        fence_entity_links(&attached)
    );
    // `LivingEntity.getCurrentSwingDuration` is six ticks here.
    for _ in 0..6 {
        world.tick(receiver.drain());
    }
    clear_hand(sender, receiver, world);
    boat
}

fn cauldron_item(name: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
                .unwrap_or_else(|| panic!("{name} is a built-in item"))
                .id,
        ),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn held_cauldron_item(world: &ServerWorld, connection_id: u64) -> (Option<i32>, i32) {
    let snapshot = world.publication();
    let player = snapshot
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(connection_id))
        .expect("the connection is a player");
    let held = &player.inventory.slots[usize::from(player.inventory.selected_slot)];
    (held.item_id, held.count)
}

fn cauldron_sound_sent(tick: &TickResult, resource_id: &str) -> bool {
    sounds_sent(tick).iter().any(|sound| {
        sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id(resource_id)
                    .expect("a registered sound"),
            }
    })
}

fn respawn_anchor_state(charges: i32) -> i32 {
    placement_state(
        "minecraft:respawn_anchor",
        &[("charges", &charges.to_string())],
    )
}

fn glowstone_held() -> (Option<i32>, i32) {
    (
        Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:glowstone")
                .expect("glowstone")
                .id,
        ),
        1,
    )
}

fn moving_piston_cell() -> i32 {
    crate::block_behavior::piston::moving_piston_state(Direction::North, false)
}

fn jukebox_state(has_record: bool) -> i32 {
    placement_state(
        "minecraft:jukebox",
        &[("has_record", if has_record { "true" } else { "false" })],
    )
}

fn light_state(level: i32, waterlogged: bool) -> i32 {
    placement_state(
        "minecraft:light",
        &[
            ("level", &level.to_string()),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn daylight_detector_state(inverted: bool, power: i32) -> i32 {
    placement_state(
        "minecraft:daylight_detector",
        &[
            ("inverted", if inverted { "true" } else { "false" }),
            ("power", &power.to_string()),
        ],
    )
}

/// Live invert re-reads the sky at once: at noon the sun angle is 0
/// (`Timelines.java:53`), so the day arm writes full sky brightness after the
/// invert-off click.
#[test]
fn invert_on_the_live_level_rewrites_power_from_the_noon_sky() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set noon");
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        daylight_detector_state(true, 0),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    clear_hand(&sender, &receiver, &mut world);
    let clicked = right_click(&sender, &receiver, &mut world, 1, at, 1);
    let powered = daylight_detector_state(false, 15);
    assert_eq!(block_update_sent(&clicked, at), Some(powered));
    assert_eq!(live_block(&mut world, at), Some(powered));
}

fn power_at(world: &mut ServerWorld, pos: BlockPos) -> Option<i32> {
    bbb_server_types::block_queries::int_property(live_block(world, pos)?, "power")
}

/// `DaylightDetectorBlock.tickEntity` (`DaylightDetectorBlock.java:112-116`):
/// every 20 game ticks both detectors re-read the sky, so their output and the
/// redstone wire beside each follow noon, midnight and rain.
#[test]
fn daylight_detectors_follow_the_sky_and_drive_redstone() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(
        ServerWorldConfig {
            datapacks: datapack_config().datapacks,
            ..operator_config(&[("alice", 1)])
        },
        &restore_candidate(0, None, Vec::new()),
    );
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let day = BlockPos { x: 0, y: 64, z: 1 };
    let night = BlockPos { x: 4, y: 64, z: 1 };
    let day_wire = BlockPos { x: 1, y: 64, z: 1 };
    let night_wire = BlockPos { x: 5, y: 64, z: 1 };
    for (pos, state) in [
        (day, daylight_detector_state(false, 0)),
        (night, daylight_detector_state(true, 0)),
        (
            day_wire,
            crate::block_behavior::default_block_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID,
            ),
        ),
        (
            night_wire,
            crate::block_behavior::default_block_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID,
            ),
        ),
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let settle = |world: &mut ServerWorld, command: &str, ticks: usize| {
        chat_command(&sender, &receiver, world, 1, command);
        for _ in 0..ticks {
            world.tick(receiver.drain());
        }
    };
    settle(&mut world, "weather clear", 0);
    settle(&mut world, "time set noon", 40);
    assert_eq!(
        [power_at(&mut world, day), power_at(&mut world, day_wire)],
        [Some(15), Some(15)]
    );
    assert_eq!(
        [
            power_at(&mut world, night),
            power_at(&mut world, night_wire)
        ],
        [Some(0), Some(0)]
    );

    // Midnight darkens the sky by 11: the day arm's sun is below the horizon,
    // the inverted arm reads 15 - 4.
    settle(&mut world, "time set midnight", 40);
    assert_eq!(
        [power_at(&mut world, day), power_at(&mut world, day_wire)],
        [Some(0), Some(0)]
    );
    assert_eq!(
        [
            power_at(&mut world, night),
            power_at(&mut world, night_wire)
        ],
        [Some(11), Some(11)]
    );

    // Full rain at noon darkens the sky by 3.
    settle(&mut world, "time set noon", 0);
    settle(&mut world, "weather rain 100000t", 140);
    assert_eq!(power_at(&mut world, day), Some(12));
}

/// The live item entities of one level as `(item resource id, count)`.
fn campfire_ground_items(world: &ServerWorld) -> Vec<(String, i32)> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            let crate::entity::EntityData::Item(item) = entity.data() else {
                return None;
            };
            let stack = item.item();
            Some((stack.item_resource_id()?.to_string(), stack.count()))
        })
        .collect()
}

fn campfire_progress(world: &mut ServerWorld, at: BlockPos) -> Vec<i32> {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.campfire())
        .expect("the campfire block entity is live")
        .cooking_progress()
        .to_vec()
}

fn campfire_slots(world: &mut ServerWorld, at: BlockPos) -> Vec<ItemStackSummary> {
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.campfire())
        .expect("the campfire block entity is live")
        .items()
        .iter()
        .map(|stack| stack.summary().clone())
        .collect()
}

/// `CampfireBlock.getTicker` (`CampfireBlock.java:308-321`) over
/// `CampfireBlockEntity.cookTick` (`CampfireBlockEntity.java:53-83`): a lit
/// campfire advances one step per tick and drops the recipe's result at the
/// block position once the slot reaches its cooking time.
#[test]
fn a_lit_campfire_cooks_its_food_and_drops_the_recipe_result() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // The block update only reaches a session watching the column.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 1.5,
            y: 64.0,
            z: 1.5,
        },
    );
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:porkchop");
    click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    // `placeFood` copies the recipe's own 600-tick cooking time, and the click's
    // own tick already ran `cookTick` once.
    assert_eq!(campfire_progress(&mut world, at), vec![1, 0, 0, 0]);
    assert_eq!(
        campfire_slots(&mut world, at)[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_PORKCHOP_ID)
    );

    world.tick(receiver.drain());
    assert_eq!(campfire_progress(&mut world, at), vec![2, 0, 0, 0]);

    // One step short of the end, so the next tick completes the slot.
    world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.campfire_mut())
        .expect("the campfire block entity is live")
        .set_cooking(0, 599, 600);
    let cooked = world.tick(receiver.drain());
    // `Containers.dropItemStack` leaves the drop with no pickup delay, so it is
    // read before the next tick can hand it to the player standing beside it.
    let drops = campfire_ground_items(&world);
    assert_eq!(
        drops,
        vec![("minecraft:cooked_porkchop".to_string(), 1)],
        "{drops:?}"
    );
    assert!(bbb_registry_codec::item_stack_summary_is_empty(
        &campfire_slots(&mut world, at)[0]
    ));
    // The block update is drained on the tick after the one that queued it.
    let after = world.tick(receiver.drain());
    assert!(
        cooked
            .events
            .iter()
            .chain(after.events.iter())
            .any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::BlockUpdate(update),
                    ..
                } if update.pos == at
            )),
        "no block update for {at:?}"
    );
    let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        events.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::BLOCK_CHANGE
                && event.pos == at
        }),
        "{events:?}"
    );
}

/// A slot no campfire recipe claims still completes, and the stack itself is
/// what falls out (`CampfireBlockEntity.java:67-69`).
#[test]
fn a_campfire_slot_without_a_recipe_drops_the_original_stack() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let campfire = world
        .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at)
        .and_then(|entity| entity.campfire_mut())
        .expect("the campfire block entity is live");
    assert!(campfire.set_slot(
        0,
        crate::tests::native_stack(ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 2,
            component_patch: Default::default(),
        })
    ));
    assert!(campfire.set_cooking(0, 599, 600));

    world.tick(receiver.drain());
    assert!(bbb_registry_codec::item_stack_summary_is_empty(
        &campfire_slots(&mut world, at)[0]
    ));
    let drops = campfire_ground_items(&world);
    assert_eq!(drops, vec![("minecraft:stone".to_string(), 2)], "{drops:?}");
}

/// `CampfireBlockEntity.cooldownTick` (`CampfireBlockEntity.java:85-98`): an
/// unlit campfire loses two progress steps per tick and stops at zero.
#[test]
fn an_unlit_campfire_cools_its_progress_by_two() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let at = BlockPos { x: 0, y: 64, z: 1 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(true),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    give_named(&sender, &receiver, &mut world, "minecraft:porkchop");
    click_use(&sender, &receiver, &mut world, at, 0.5, 1);
    world.tick(receiver.drain());
    assert_eq!(campfire_progress(&mut world, at), vec![2, 0, 0, 0]);

    // `CampfireBlock.getTicker` picks `cooldownTick` the moment the state is
    // unlit, without the block entity changing.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        campfire_state(false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    assert_eq!(campfire_progress(&mut world, at), vec![0, 0, 0, 0]);
    // Clamped at zero: further ticks change nothing and drop nothing.
    world.tick(receiver.drain());
    assert_eq!(campfire_progress(&mut world, at), vec![0, 0, 0, 0]);
    assert!(campfire_ground_items(&world).is_empty());
    assert_eq!(
        campfire_slots(&mut world, at)[0].item_id,
        Some(bbb_protocol::item_types::VANILLA_ITEM_PORKCHOP_ID)
    );
}
