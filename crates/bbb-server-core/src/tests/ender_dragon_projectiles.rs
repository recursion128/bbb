//! Real projectile ticks against the dragon's independently hittable parts.

use super::*;
use crate::entity::item::plain_stack;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
    VANILLA_ENTITY_TYPE_TRIDENT_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};
use bbb_server_types::geometry::{Aabb, Vec3Ops};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn dragon_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    dragon_world_with_config(datapack_config())
}

fn dragon_world_with_config(
    config: ServerWorldConfig,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "archer", 1);
    let dragon = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            Vec3d {
                x: 15.5,
                y: 100.0,
                z: 15.5,
            },
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID).unwrap(),
            ),
        )
        .0;
    world.tick(receiver.drain());
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(DIMENSION, dragon)
        .unwrap()
    else {
        panic!("a dragon has a saved record");
    };
    entries.retain(|entry| entry.name != "NoAI");
    entries.push(NbtSummaryEntry {
        name: "NoAI".into(),
        value: NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(DIMENSION, dragon, &entries);
    (sender, receiver, world, dragon)
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .health()
}

fn part(world: &ServerWorld, dragon: i32, offset: i32) -> Aabb {
    world
        .dragon_part_box_for_test(DIMENSION, dragon + offset)
        .unwrap()
}

fn launch(
    world: &mut ServerWorld,
    position: Vec3d,
    motion: Vec3d,
    wind: bool,
    piercing: i8,
    power: bool,
) -> i32 {
    let data = if wind {
        EntityData::WindCharge(crate::entity::WindChargeData::new(
            crate::entity::HurtingProjectileState::new(Some(Uuid::from_u128(1)), 0.0),
            5,
        ))
    } else {
        let mut data = crate::entity::ArrowData::new(plain_stack("minecraft:arrow", 1).unwrap());
        data.projectile.set_owner(Some(Uuid::from_u128(1)));
        data.set_pierce_level(piercing);
        if power {
            let mut bow = plain_stack("minecraft:bow", 1).unwrap();
            enchant(world, &mut bow, "minecraft:power", 5);
            data.set_weapon(Some(bow));
        }
        EntityData::Arrow(data)
    };
    let type_id = if wind {
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID
    } else {
        VANILLA_ENTITY_TYPE_ARROW_ID
    };
    let id = world
        .spawn_stored_entity_for_test(DIMENSION, type_id, position, data)
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_delta_movement(motion);
    id
}

fn enchant(
    world: &ServerWorld,
    stack: &mut bbb_server_types::item_stack::RuntimeItemStack,
    resource_id: &str,
    level: i32,
) {
    let holder_id = world
        .datapacks()
        .unwrap()
        .registries
        .registry("minecraft:enchantment")
        .unwrap()
        .elements
        .iter()
        .position(|entry| entry.id == resource_id)
        .unwrap() as i32;
    let patch = &mut stack.summary_mut().component_patch;
    patch
        .enchantments
        .push(bbb_protocol::packets::ItemEnchantmentSummary { holder_id, level });
    patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
}

#[test]
fn dragon_projectiles_damage_the_head_and_body_once_and_miss_the_parent_hole() {
    for (offset, expected, power) in [(1, 8.0, false), (3, 3.0, false), (1, 20.0, true)] {
        let (_sender, receiver, mut world, dragon) = dragon_world();
        let bounds = part(&world, dragon, offset);
        let (start, movement) = if offset == 1 {
            (
                Vec3d {
                    z: bounds.min_z - 2.0,
                    ..bounds.center()
                },
                Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 4.0,
                },
            )
        } else {
            (
                Vec3d {
                    y: bounds.max_y + 2.0,
                    ..bounds.center()
                },
                Vec3d {
                    x: 0.0,
                    y: -4.0,
                    z: 0.0,
                },
            )
        };
        let before = health(&world, dragon);
        let arrow = launch(&mut world, start, movement, false, 0, power);
        world.tick(receiver.drain());
        assert_eq!(health(&world, dragon), before - expected, "part {offset}");
        assert!(
            world.stored_entity_for_test(DIMENSION, arrow).is_none(),
            "the hit consumes the arrow"
        );
    }

    let config = datapack_config();
    let power: serde_json::Value = serde_json::from_slice(
        &config
            .datapacks
            .as_ref()
            .unwrap()
            .registries
            .element("minecraft:enchantment", "minecraft:power")
            .unwrap()
            .source_json,
    )
    .unwrap();
    for (nbt, expected) in [("{Invulnerable:0b}", 20.0), ("{NoAI:1b}", 8.0)] {
        let mut effects = power["effects"].clone();
        let original = effects["minecraft:damage"][0]["requirements"].take();
        effects["minecraft:damage"][0]["requirements"] = serde_json::json!({
            "condition":"minecraft:all_of",
            "terms":[original, {
                "condition":"minecraft:entity_properties",
                "entity":"this",
                "predicate":{"type":"minecraft:ender_dragon", "nbt":nbt}
            }]
        });
        let (_sender, receiver, mut world, dragon) = dragon_world_with_config(
            datapack_config_with_enchantment_effects("minecraft:power", effects),
        );
        let NbtSummaryValue::Compound(saved) = world
            .stored_entity_command_data_for_test(DIMENSION, dragon)
            .unwrap()
        else {
            panic!("the parent dragon has canonical saved data");
        };
        assert!(saved
            .iter()
            .any(|entry| entry.name == "NoAI" && entry.value == NbtSummaryValue::Byte(1)));
        let bounds = part(&world, dragon, 1);
        let before = health(&world, dragon);
        let arrow = launch(
            &mut world,
            Vec3d {
                z: bounds.min_z - 2.0,
                ..bounds.center()
            },
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 4.0,
            },
            false,
            0,
            true,
        );
        world.tick(receiver.drain());
        assert_eq!(health(&world, dragon), before - expected, "{nbt}");
        assert!(
            world.stored_entity_for_test(DIMENSION, arrow).is_none(),
            "the NBT-gated head hit consumes the arrow: {nbt}"
        );
    }

    let (_sender, receiver, mut world, dragon) = dragon_world();
    let position = world
        .stored_entity_for_test(DIMENSION, dragon)
        .unwrap()
        .position();
    let before = health(&world, dragon);
    let arrow = launch(
        &mut world,
        position.plus(Vec3d {
            x: -2.0,
            y: 6.0,
            z: 0.0,
        }),
        Vec3d {
            x: 4.0,
            y: 0.0,
            z: 0.0,
        },
        false,
        0,
        false,
    );
    world.tick(receiver.drain());
    assert_eq!(health(&world, dragon), before);
    assert!(
        world.stored_entity_for_test(DIMENSION, arrow).is_some(),
        "the parent box alone cannot stop an arrow"
    );
}

#[test]
fn dragon_projectiles_share_part_hits_with_wind_charges() {
    let (_sender, receiver, mut world, dragon) = dragon_world();
    let bounds = part(&world, dragon, 7);
    let before = health(&world, dragon);
    let projectile = launch(
        &mut world,
        Vec3d {
            x: bounds.max_x + 2.0,
            ..bounds.center()
        },
        Vec3d {
            x: -4.0,
            y: 0.0,
            z: 0.0,
        },
        true,
        0,
        false,
    );
    world.tick(receiver.drain());
    assert_eq!(health(&world, dragon), before - 1.25);
    assert!(world
        .stored_entity_for_test(DIMENSION, projectile)
        .is_none());
}

/// `AbstractDragonSittingPhase.onHurt` (`AbstractDragonSittingPhase.java:19-26`):
/// a perched dragon takes nothing from an arrow and sets it alight.
#[test]
fn a_perched_dragon_ignites_the_arrow_it_shrugs_off() {
    let (_sender, receiver, mut world, dragon) = dragon_world();
    let NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(DIMENSION, dragon)
        .unwrap()
    else {
        unreachable!()
    };
    entries.retain(|entry| entry.name != "DragonPhase");
    entries.push(NbtSummaryEntry {
        name: "DragonPhase".into(),
        value: NbtSummaryValue::Int(6),
    });
    world.load_stored_entity_record_for_test(DIMENSION, dragon, &entries);
    let bounds = part(&world, dragon, 3);
    let before = health(&world, dragon);
    let arrow = launch(
        &mut world,
        Vec3d {
            x: bounds.max_x + 2.0,
            ..bounds.center()
        },
        Vec3d {
            x: -4.0,
            y: 0.0,
            z: 0.0,
        },
        false,
        0,
        false,
    );
    world.tick(receiver.drain());
    assert_eq!(health(&world, dragon), before);
    assert!(
        world
            .stored_entity_for_test(DIMENSION, arrow)
            .unwrap()
            .remaining_fire_ticks()
            > 0
    );
}

#[test]
fn dragon_projectiles_pierce_one_dragon_then_hit_an_ordinary_mob() {
    let (_sender, receiver, mut world, dragon) = dragon_world();
    let position = world
        .stored_entity_for_test(DIMENSION, dragon)
        .unwrap()
        .position();
    let cow = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_COW_ID,
            position.plus(Vec3d {
                x: 0.0,
                y: 0.0,
                z: 12.0,
            }),
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap(),
            ),
        )
        .0;
    let before = health(&world, dragon);
    launch(
        &mut world,
        position.plus(Vec3d {
            x: 0.0,
            y: 0.5,
            z: -10.0,
        }),
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 26.0,
        },
        false,
        1,
        false,
    );
    world.tick(receiver.drain());
    assert_eq!(health(&world, dragon), before - 14.0);
    assert!(
        health(&world, cow) <= 0.0,
        "one dragon leaves a piercing hit for the cow"
    );
}

#[test]
fn dragon_projectiles_hook_parts_and_reel_in_without_pulling_the_dragon() {
    let (sender, receiver, mut world, dragon) = dragon_world();
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let bounds = part(&world, dragon, 1);
    world.set_player_position_for_test(
        ConnectionId(1),
        bounds.center().plus(Vec3d {
            x: 0.0,
            y: 0.0,
            z: -5.0,
        }),
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        plain_stack("minecraft:fishing_rod", 1)
            .unwrap()
            .into_summary(),
    );
    let hook = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            Vec3d {
                z: bounds.min_z - 2.0,
                ..bounds.center()
            },
            EntityData::FishingBobber(crate::entity::FishingBobberData::new(
                Uuid::from_u128(1),
                0,
                0,
            )),
        )
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, hook)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 4.0,
        });
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    let hook_entity = world.stored_entity_for_test(DIMENSION, hook).unwrap();
    let EntityData::FishingBobber(data) = hook_entity.data() else {
        unreachable!();
    };
    assert_eq!(data.hooked_entity_id(), Some(dragon + 1));
    assert_eq!(
        hook_entity.position(),
        Vec3d {
            y: bounds.min_y + (bounds.max_y - bounds.min_y) * 0.8,
            ..bounds.center()
        }
    );
    let before = world.stored_entity_for_test(DIMENSION, dragon).unwrap();
    let (position, motion) = (before.position(), before.delta_movement());
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItem(UseItem {
            hand: InteractionHand::MainHand,
            sequence: 1,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    assert!(world.stored_entity_for_test(DIMENSION, hook).is_none());
    let after = world.stored_entity_for_test(DIMENSION, dragon).unwrap();
    assert_eq!(after.position(), position);
    assert_eq!(after.delta_movement(), motion);
    assert_eq!(
        world.publication().players[0].inventory.slots[0]
            .component_patch
            .damage,
        Some(5)
    );
}

#[test]
fn dragon_projectiles_channel_lightning_at_the_exposed_part_under_open_sky() {
    let (sender, receiver, mut world, dragon) = dragon_world();
    crate::tests::entities::console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "weather thunder 40000t",
    );
    for _ in 0..200 {
        if world.level_weather(DIMENSION).is_thundering() {
            break;
        }
        world.tick(receiver.drain());
    }
    assert!(world.level_weather(DIMENSION).is_thundering());
    let position = world
        .stored_entity_for_test(DIMENSION, dragon)
        .unwrap()
        .position();
    let bounds = part(&world, dragon, 1);
    assert!(
        bounds.min_y < position.y,
        "the head extends below the parent box"
    );
    let roof = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32 + 10,
        z: position.z.floor() as i32,
    };
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    assert!(world.set_block(DIMENSION, roof, stone, SET_BLOCK_UPDATE_CLIENTS));
    let bolts_before = world.stored_entity_ids_for_test(DIMENSION);
    let mut item = plain_stack("minecraft:trident", 1).unwrap();
    enchant(&world, &mut item, "minecraft:channeling", 1);
    let mut data = crate::entity::TridentData::new(item);
    data.projectile.set_owner(Some(Uuid::from_u128(1)));
    let trident = world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            Vec3d {
                z: bounds.min_z - 2.0,
                ..bounds.center()
            },
            EntityData::Trident(data),
        )
        .0;
    world
        .stored_entity_mut_for_test(DIMENSION, trident)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 4.0,
        });
    let before = health(&world, dragon);
    world.tick(receiver.drain());
    assert_eq!(health(&world, dragon), before - 8.0);
    let expected = Vec3d {
        y: bounds.min_y,
        ..bounds.center()
    };
    assert_ne!(expected, position);
    let bolts = world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| !bolts_before.contains(id))
        .filter_map(|id| world.stored_entity_for_test(DIMENSION, id))
        .filter(|entity| {
            entity.entity_type_id() == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
                && entity.position() == expected
        })
        .count();
    assert_eq!(
        bolts, 1,
        "Channeling reads the exposed part's sky visibility and position"
    );
}
