//! Buckets, glass bottles and lily pads used on the fluid in front of the
//! player: `BucketItem.use`, `MobBucketItem`, `SolidBucketItem.useOn`,
//! `BottleItem.use` and `PlaceOnWaterBlockItem.use`.

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
use bbb_protocol::packets::{BlockHitResult, BlockPos, Direction, ItemStackSummary, Vec3d};

use super::*;
use crate::entity::EntityData;

/// The fixture floor's top is y = 63, so the session stands at y = 64 with its
/// eye at 65.62 and looks down +z.
const STANDING: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

/// The cell a level look down +z first meets at eye height.
const AHEAD: BlockPos = BlockPos { x: 0, y: 65, z: 2 };
/// The cell between the session and [`AHEAD`], against its north face.
const BETWEEN: BlockPos = BlockPos { x: 0, y: 65, z: 1 };

fn fixture(config: ServerWorldConfig) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    (sender, receiver, world)
}

fn hold(world: &mut ServerWorld, name: &str) {
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id(name)),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
}

fn held(world: &ServerWorld) -> ItemStackSummary {
    world.publication().players[0].inventory.slots[0].clone()
}

fn fluid(name: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        name,
        &std::collections::BTreeMap::from([("level".to_string(), "0".to_string())]),
    )
    .expect("the built-in fluid source state")
}

fn set(world: &mut ServerWorld, dimension: Java26BuiltinDimension, pos: BlockPos, state: i32) {
    world.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
    assert_eq!(world.block_state(dimension, pos), Some(state), "{pos:?}");
}

fn state_at(world: &mut ServerWorld, pos: BlockPos) -> Option<i32> {
    world.block_state(Java26BuiltinDimension::Overworld, pos)
}

fn use_item(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    x_rot: f32,
) {
    throw(sender, receiver, world, 1, 0.0, x_rot, 1);
}

#[test]
fn an_empty_bucket_scoops_a_water_source_and_fires_filled_bucket() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::AdvancementElement;

    let mut config = datapack_config();
    let mut startup = (*config.datapacks.take().unwrap()).clone();
    let player = serde_json::json!({"type":"minecraft:player",
        "slots":{"inventory.0":{"items":"minecraft:diamond"}}});
    for (id, predicate) in [
        ("test:fill_water", player.clone()),
        (
            "test:fill_list",
            serde_json::json!([{
                "condition":"minecraft:entity_properties", "entity":"this", "predicate":player
            }]),
        ),
    ] {
        startup.advancements.advancements.push(AdvancementElement {
            id: id.to_owned(),
            source_pack: "test".to_owned(),
            advancement: parse_advancement(&serde_json::json!({
                "criteria": {"fill": {
                    "trigger": "minecraft:filled_bucket",
                    "conditions": {"item": {"items": "minecraft:water_bucket"}, "player":predicate}
                }},
                "rewards":{"experience":3}
            }))
            .unwrap(),
        });
    }
    config.datapacks = Some(std::sync::Arc::new(startup));
    let (sender, receiver, mut world) = fixture(config);
    hold(&mut world, "minecraft:bucket");
    set(
        &mut world,
        Java26BuiltinDimension::Overworld,
        AHEAD,
        fluid("minecraft:water"),
    );

    use_item(&sender, &receiver, &mut world, 0.0);

    assert_eq!(
        state_at(&mut world, AHEAD),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(
        held(&world).item_id,
        Some(item_id("minecraft:water_bucket"))
    );
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:fill_water"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:fill_list"));
    assert_eq!(world.experience_total_for_test(ConnectionId(1)), 0);
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        9,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:diamond")),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    for _ in 0..2 {
        hold(&mut world, "minecraft:bucket");
        set(
            &mut world,
            Java26BuiltinDimension::Overworld,
            AHEAD,
            fluid("minecraft:water"),
        );
        use_item(&sender, &receiver, &mut world, 0.0);
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:fill_water"));
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:fill_list"));
        assert_eq!(world.experience_total_for_test(ConnectionId(1)), 6);
    }
}

#[test]
fn water_and_lava_buckets_empty_against_the_clicked_face() {
    for (bucket, liquid) in [
        ("minecraft:water_bucket", "minecraft:water"),
        ("minecraft:lava_bucket", "minecraft:lava"),
    ] {
        let (sender, receiver, mut world) = fixture(datapack_config());
        hold(&mut world, bucket);
        set(
            &mut world,
            Java26BuiltinDimension::Overworld,
            AHEAD,
            block_state_id("minecraft:stone"),
        );

        use_item(&sender, &receiver, &mut world, 0.0);

        assert_eq!(
            state_at(&mut world, BETWEEN),
            Some(fluid(liquid)),
            "{bucket}"
        );
        assert_eq!(held(&world).item_id, Some(item_id("minecraft:bucket")));
    }
}

#[test]
fn water_poured_in_the_nether_evaporates() {
    let (sender, receiver, mut world) = fixture(datapack_config());
    let standing = Vec3d {
        x: 0.5,
        y: 100.0,
        z: 0.5,
    };
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: standing,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    ));
    let nether = Java26BuiltinDimension::Nether;
    let air = block_state_id("minecraft:air");
    for pos in [
        BlockPos { x: 0, y: 100, z: 0 },
        BlockPos { x: 0, y: 101, z: 0 },
        BlockPos { x: 0, y: 101, z: 1 },
    ] {
        set(&mut world, nether, pos, air);
    }
    let target = BlockPos { x: 0, y: 101, z: 2 };
    set(
        &mut world,
        nether,
        target,
        block_state_id("minecraft:stone"),
    );
    hold(&mut world, "minecraft:water_bucket");

    use_item(&sender, &receiver, &mut world, 0.0);

    assert_eq!(
        world.block_state(nether, BlockPos { x: 0, y: 101, z: 1 }),
        Some(air)
    );
    assert_eq!(held(&world).item_id, Some(item_id("minecraft:bucket")));
}

#[test]
fn a_powder_snow_bucket_places_powder_snow_on_the_clicked_face() {
    let (sender, receiver, mut world) = fixture(datapack_config());
    hold(&mut world, "minecraft:powder_snow_bucket");
    let floor = BlockPos { x: 0, y: 63, z: 2 };

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: floor,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 1.0,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    );

    assert_eq!(
        state_at(&mut world, BlockPos { x: 0, y: 64, z: 2 }),
        Some(block_state_id("minecraft:powder_snow"))
    );
    assert_eq!(held(&world).item_id, Some(item_id("minecraft:bucket")));
}

#[test]
fn an_axolotl_bucket_pours_water_and_releases_its_axolotl() {
    use crate::entity::ai::{BrainPacked, MemoryModuleType, MemoryValue, PackedMemory};
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    };
    use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};

    let (sender, receiver, mut world) = fixture(datapack_config());
    world.tick_rate_mut().set_frozen(true);
    // `Axolotl.Variant.BLUE` (`Axolotl.java:627`), which no natural spawn picks.
    let blue = 4;
    let retained_memory = PackedMemory {
        memory_type: MemoryModuleType::PlayDeadTicks,
        value: MemoryValue::Int(321),
        time_to_live: None,
    };
    let entity_brain = BrainPacked {
        memories: vec![
            retained_memory.clone(),
            PackedMemory {
                memory_type: MemoryModuleType::HasHuntingCooldown,
                value: MemoryValue::Bool(true),
                time_to_live: Some(77),
            },
        ],
    };
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id("minecraft:axolotl_bucket")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                ],
                builtin_enum_component_ids: std::collections::BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    blue,
                )]),
                entity_data_tag: Some(Nbt::Compound(vec![
                    Entry {
                        name: "id".into(),
                        value: Nbt::String("minecraft:axolotl".into()),
                    },
                    Entry {
                        name: "Brain".into(),
                        value: entity_brain.encode(),
                    },
                ])),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    );
    set(
        &mut world,
        Java26BuiltinDimension::Overworld,
        AHEAD,
        block_state_id("minecraft:stone"),
    );

    use_item(&sender, &receiver, &mut world, 0.0);

    assert_eq!(
        state_at(&mut world, BETWEEN),
        Some(fluid("minecraft:water"))
    );
    assert_eq!(held(&world).item_id, Some(item_id("minecraft:bucket")));
    let axolotl = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .filter(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID)
        })
        .expect("the bucket's axolotl is in the level");
    let at = axolotl.position();
    assert_eq!(
        BlockPos {
            x: at.x.floor() as i32,
            y: at.y.floor() as i32,
            z: at.z.floor() as i32,
        },
        BETWEEN
    );
    let EntityData::GeneralLiving(data) = axolotl.data() else {
        panic!("an axolotl is a living entity");
    };
    assert_eq!(data.is_from_bucket(), Some(true));
    assert_eq!(data.component_simple_variant(), Some(blue));
    let Nbt::Compound(saved) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, axolotl.id())
        .unwrap()
    else {
        panic!("entity record")
    };
    assert_eq!(
        saved
            .iter()
            .find(|entry| entry.name == "Brain")
            .map(|entry| &entry.value),
        Some(
            &BrainPacked {
                memories: vec![retained_memory]
            }
            .encode()
        )
    );
}

#[test]
fn a_glass_bottle_fills_with_water_from_a_source() {
    let (sender, receiver, mut world) = fixture(datapack_config());
    hold(&mut world, "minecraft:glass_bottle");
    set(
        &mut world,
        Java26BuiltinDimension::Overworld,
        AHEAD,
        fluid("minecraft:water"),
    );

    use_item(&sender, &receiver, &mut world, 0.0);

    let potion = held(&world);
    assert_eq!(potion.item_id, Some(item_id("minecraft:potion")));
    assert_eq!(
        potion.component_patch.potion_contents.potion_id,
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water").map(|p| p.id)
    );
    assert_eq!(state_at(&mut world, AHEAD), Some(fluid("minecraft:water")));
}

#[test]
fn a_lily_pad_lands_on_the_water_surface_the_player_looks_at() {
    let (sender, receiver, mut world) = fixture(datapack_config());
    hold(&mut world, "minecraft:lily_pad");
    let pool = BlockPos { x: 0, y: 63, z: 2 };
    set(
        &mut world,
        Java26BuiltinDimension::Overworld,
        pool,
        fluid("minecraft:water"),
    );

    // Pitched 40 degrees down, the eye ray first meets the pool's surface.
    use_item(&sender, &receiver, &mut world, 40.0);

    assert_eq!(
        state_at(&mut world, BlockPos { x: 0, y: 64, z: 2 }),
        Some(block_state_id("minecraft:lily_pad"))
    );
    assert!(held(&world).item_id.is_none() || held(&world).count == 0);
}

/// `BucketItem.use` (BucketItem.java:57) and `BottleItem.use`
/// (BottleItem.java:53) ask `Level.mayInteract`, so spawn protection refuses
/// a non-operator.
#[test]
fn spawn_protection_refuses_buckets_and_bottles_for_a_non_operator() {
    for (held_item, unchanged) in [
        ("minecraft:water_bucket", "minecraft:water_bucket"),
        ("minecraft:glass_bottle", "minecraft:glass_bottle"),
    ] {
        let (sender, receiver, mut world) = fixture(ServerWorldConfig {
            admin_lists: operator_config(&[("operator", 99)]).admin_lists,
            ..datapack_config()
        });
        hold(&mut world, held_item);
        set(
            &mut world,
            Java26BuiltinDimension::Overworld,
            AHEAD,
            if held_item == "minecraft:glass_bottle" {
                fluid("minecraft:water")
            } else {
                block_state_id("minecraft:stone")
            },
        );

        use_item(&sender, &receiver, &mut world, 0.0);

        assert_eq!(
            state_at(&mut world, BETWEEN),
            Some(block_state_id("minecraft:air")),
            "{held_item}"
        );
        assert_eq!(
            held(&world).item_id,
            Some(item_id(unchanged)),
            "{held_item}"
        );
    }
}
