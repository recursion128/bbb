//! Items that place an entity: `BoatItem.use`, `MinecartItem.useOn`,
//! `HangingEntityItem.useOn`, `EndCrystalItem.useOn` and `ArmorStandItem.useOn`
//! with its stack config.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_MINECART_ID,
    VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
};
use bbb_protocol::packets::{BlockHitResult, BlockPos, Direction, ItemStackSummary, Vec3d};

use super::*;
use crate::entity::{Entity, EntityData};
use crate::tests::entities::{console_command, console_command_answer};

/// The fixture floor's top is y = 63, so the session stands at y = 64 with its
/// eye at 65.62 and looks down +z.
const STANDING: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    (sender, receiver, world)
}

fn hold(world: &mut ServerWorld, stack: ItemStackSummary) {
    world.set_inventory_slot_for_test(ConnectionId(1), 0, stack);
}

fn stack(name: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id(name)),
        count,
        ..ItemStackSummary::empty()
    }
}

fn held_count(world: &ServerWorld) -> i32 {
    world.publication().players[0].inventory.slots[0].count
}

fn entities_of(world: &ServerWorld, entity_type_id: i32) -> Vec<Entity> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id))
        .filter(|entity| entity.entity_type_id() == entity_type_id)
        .cloned()
        .collect()
}

fn set(world: &mut ServerWorld, pos: BlockPos, name: &str) {
    world.set_block(
        Java26BuiltinDimension::Overworld,
        pos,
        block_state_id(name),
        SET_BLOCK_UPDATE_ALL,
    );
}

fn use_on(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    pos: BlockPos,
    direction: Direction,
) {
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
            sequence: 1,
        }),
    );
}

#[test]
fn a_boat_lands_where_the_look_meets_the_floor_and_takes_the_stack_name() {
    let (sender, receiver, mut world) = fixture();
    hold(
        &mut world,
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Skiff".to_owned()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..stack("minecraft:oak_boat", 2)
        },
    );

    // 45 degrees down from the 65.62 eye meets the floor top 1.62 ahead.
    throw(&sender, &receiver, &mut world, 1, 0.0, 45.0, 1);

    let boats = entities_of(&world, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    assert_eq!(boats.len(), 1);
    let position = boats[0].position();
    assert!((position.y - 64.0).abs() < 1e-6, "{position:?}");
    assert!((position.z - 2.12).abs() < 0.01, "{position:?}");
    assert_eq!(
        boats[0].custom_name().map(|name| name.text().to_owned()),
        Some("Skiff".to_owned())
    );
    assert_eq!(held_count(&world), 1);
}

#[test]
fn a_minecart_is_set_on_the_clicked_rail_and_refused_off_it() {
    let (sender, receiver, mut world) = fixture();
    hold(&mut world, stack("minecraft:minecart", 2));
    let rail = BlockPos { x: 0, y: 64, z: 2 };

    use_on(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 0, y: 63, z: 2 },
        Direction::Up,
    );
    assert!(entities_of(&world, VANILLA_ENTITY_TYPE_MINECART_ID).is_empty());

    let rail_state =
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:rail",
            &std::collections::BTreeMap::from([
                ("shape".to_owned(), "north_south".to_owned()),
                ("waterlogged".to_owned(), "false".to_owned()),
            ]),
        )
        .unwrap();
    world.set_block(
        Java26BuiltinDimension::Overworld,
        rail,
        rail_state,
        SET_BLOCK_UPDATE_ALL,
    );
    use_on(&sender, &receiver, &mut world, rail, Direction::Up);

    let carts = entities_of(&world, VANILLA_ENTITY_TYPE_MINECART_ID);
    assert_eq!(carts.len(), 1);
    assert_eq!(
        carts[0].position(),
        Vec3d {
            x: 0.5,
            y: 64.0625,
            z: 2.5,
        }
    );
    assert_eq!(held_count(&world), 1);
}

#[test]
fn a_painting_and_an_item_frame_hang_on_the_clicked_wall_face() {
    let (sender, receiver, mut world) = fixture();
    let wall = BlockPos { x: 0, y: 64, z: 3 };
    set(&mut world, wall, "minecraft:stone");
    hold(&mut world, stack("minecraft:painting", 1));

    use_on(&sender, &receiver, &mut world, wall, Direction::North);

    let paintings = entities_of(&world, VANILLA_ENTITY_TYPE_PAINTING_ID);
    assert_eq!(paintings.len(), 1);
    let EntityData::Painting(painting) = paintings[0].data() else {
        unreachable!()
    };
    // Only one stone backs the cell, so the largest fitting size is 1x1.
    let variant = bbb_protocol::painting_variant_types::vanilla_painting_variant_for_resource_id(
        painting.variant().resource_id(),
    )
    .unwrap();
    assert_eq!((variant.width, variant.height), (1, 1));
    assert_eq!(painting.pose().attached(), BlockPos { x: 0, y: 64, z: 2 });
    assert_eq!(held_count(&world), 0);

    let painting_id = paintings[0].id();
    let painting_uuid = paintings[0].uuid();
    let reset = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {painting_uuid} {{variant:'minecraft:kebab'}}"),
    );
    assert_eq!(
        console_command_answer(&reset).0,
        (painting.variant().resource_id() != "minecraft:kebab").then_some(1)
    );
    assert!(matches!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, painting_id)
            .unwrap()
            .data(),
        EntityData::Painting(data) if data.variant().resource_id() == "minecraft:kebab"
    ));

    let changed = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("data merge entity {painting_uuid} {{variant:'aztec'}}"),
    );
    assert_eq!(console_command_answer(&changed).0, Some(1));
    let actual = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, painting_id)
        .unwrap();
    assert_eq!(actual.uuid(), painting_uuid);
    let EntityData::Painting(aztec) = actual.data() else {
        unreachable!()
    };
    assert_eq!(aztec.variant().resource_id(), "minecraft:aztec");
    let aztec = aztec.variant().holder_id();
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, painting_id)
        .unwrap()
    else {
        panic!("painting command data is a compound");
    };
    assert_eq!(
        saved
            .iter()
            .find(|entry| entry.name == "variant")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string("minecraft:aztec"))
    );
    assert!(changed.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::SetEntityData(packet),
        } if packet.id == painting_id && packet.values.iter().any(|value|
            value.data_id == bbb_protocol::entity_synched_data::VANILLA_PAINTING_VARIANT_DATA_ID
                && matches!(&value.value, EntityDataValueKind::PaintingVariant(variant)
                    if variant.registry.as_ref().is_some_and(|reference| reference.holder_id() == aztec)))
    )));

    // The frame cannot share the painting's face, so it goes one block up.
    let upper = BlockPos { x: 0, y: 65, z: 3 };
    set(&mut world, upper, "minecraft:stone");
    hold(&mut world, stack("minecraft:item_frame", 1));
    use_on(&sender, &receiver, &mut world, upper, Direction::North);

    let frames = entities_of(&world, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
    assert_eq!(frames.len(), 1);
    let EntityData::ItemFrame(frame) = frames[0].data() else {
        unreachable!()
    };
    assert_eq!(frame.pose().attached(), BlockPos { x: 0, y: 65, z: 2 });
    assert_eq!(frame.pose().direction(), Direction::North);
    assert_eq!(held_count(&world), 0);
}

#[test]
fn an_armor_stand_takes_its_stack_name_and_entity_data() {
    let (sender, receiver, mut world) = fixture();
    hold(
        &mut world,
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                custom_name: Some("Butler".to_owned()),
                entity_data_tag: Some(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "id".into(),
                        value: NbtSummaryValue::string("minecraft:armor_stand"),
                    },
                    NbtSummaryEntry {
                        name: "ShowArms".into(),
                        value: NbtSummaryValue::Byte(1),
                    },
                ])),
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..stack("minecraft:armor_stand", 1)
        },
    );

    use_on(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 0, y: 63, z: 2 },
        Direction::Up,
    );

    let stands = entities_of(&world, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    assert_eq!(stands.len(), 1);
    assert_eq!(
        stands[0].custom_name().map(|name| name.text().to_owned()),
        Some("Butler".to_owned())
    );
    assert!(crate::entity::armor_stand::armor_stand(&stands[0])
        .unwrap()
        .show_arms());
    assert_eq!(held_count(&world), 0);
}

#[test]
fn an_end_crystal_sits_on_obsidian_once_and_nowhere_else() {
    let (sender, receiver, mut world) = fixture();
    hold(&mut world, stack("minecraft:end_crystal", 3));
    let obsidian = BlockPos { x: 0, y: 63, z: 2 };
    set(&mut world, obsidian, "minecraft:obsidian");

    use_on(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 0, y: 63, z: 3 },
        Direction::Up,
    );
    use_on(&sender, &receiver, &mut world, obsidian, Direction::Up);
    // The first crystal now fills the space above.
    use_on(&sender, &receiver, &mut world, obsidian, Direction::Up);

    let crystals = entities_of(&world, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID);
    assert_eq!(crystals.len(), 1);
    assert_eq!(
        crystals[0].position(),
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 2.5,
        }
    );
    assert_eq!(held_count(&world), 2);
}

/// `Painting.create` draws from the live `#placeable` tag and
/// `Painting.calculateBoundingBox` reads the held variant's size
/// (`Painting.java:92-119`, `:150-160`); the save writes and reads the
/// element's identifier (`VariantUtils.java:26-32`).
#[test]
fn a_datapack_painting_variant_is_placed_sized_and_saved_by_its_identifier() {
    let root = std::env::temp_dir().join(format!("bbb-painting-variant-{}", Uuid::new_v4()));
    let pack = root.join("test");
    let variants = pack.join("data/test/painting_variant");
    let tags = pack.join("data/minecraft/tags/painting_variant");
    std::fs::create_dir_all(&variants).unwrap();
    std::fs::create_dir_all(&tags).unwrap();
    std::fs::write(
        pack.join("pack.mcmeta"),
        r#"{"pack":{"description":"painting","pack_format":1000,"supported_formats":[0,10000]}}"#,
    )
    .unwrap();
    std::fs::write(
        variants.join("tall.json"),
        r#"{"asset_id":"minecraft:kebab","width":1,"height":3}"#,
    )
    .unwrap();
    std::fs::write(tags.join("placeable.json"), r#"{"values":["test:tall"]}"#).unwrap();
    let mut config = datapack_config();
    config.datapacks = Some(std::sync::Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    std::fs::remove_dir_all(root).unwrap();
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    // A one-wide, five-tall column: no two-wide variant fits, and the 1x3
    // datapack variant is the largest that does.
    for y in 64..=68 {
        set(&mut world, BlockPos { x: 0, y, z: 3 }, "minecraft:stone");
    }
    hold(&mut world, stack("minecraft:painting", 1));

    use_on(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 0, y: 66, z: 3 },
        Direction::North,
    );

    let paintings = entities_of(&world, VANILLA_ENTITY_TYPE_PAINTING_ID);
    assert_eq!(paintings.len(), 1);
    let EntityData::Painting(painting) = paintings[0].data() else {
        unreachable!()
    };
    assert_eq!(painting.variant().resource_id(), "test:tall");
    let bounds = paintings[0].bounding_box();
    assert_eq!(bounds.max_y - bounds.min_y, 3.0);
    assert_eq!(bounds.max_x - bounds.min_x, 1.0);

    // A merge reads the whole compound back through the save codec.
    let painting_id = paintings[0].id();
    let uuid = paintings[0].uuid();
    console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {uuid} {{Silent:1b}}"),
    );
    let EntityData::Painting(reloaded) = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, painting_id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(reloaded.variant(), painting.variant());
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, painting_id)
        .unwrap()
    else {
        panic!("painting command data is a compound");
    };
    assert_eq!(
        saved
            .iter()
            .find(|entry| entry.name == "variant")
            .map(|entry| &entry.value),
        Some(&NbtSummaryValue::string("test:tall"))
    );
}

/// `HangingEntityItem.useOn` (`HangingEntityItem.java:63-64`) applies the
/// stack's `entity_data` before `survives`: a 4x4 variant on a lone stone is
/// not hung and the painting stays in hand.
#[test]
fn a_painting_whose_entity_data_does_not_fit_is_not_hung() {
    let (sender, receiver, mut world) = fixture();
    let wall = BlockPos { x: 0, y: 64, z: 3 };
    set(&mut world, wall, "minecraft:stone");
    hold(
        &mut world,
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                entity_data_tag: Some(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "id".into(),
                        value: NbtSummaryValue::string("minecraft:painting"),
                    },
                    NbtSummaryEntry {
                        name: "variant".into(),
                        value: NbtSummaryValue::string("minecraft:pigscene"),
                    },
                ])),
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..stack("minecraft:painting", 1)
        },
    );

    use_on(&sender, &receiver, &mut world, wall, Direction::North);

    assert!(entities_of(&world, VANILLA_ENTITY_TYPE_PAINTING_ID).is_empty());
    assert_eq!(held_count(&world), 1);
}
