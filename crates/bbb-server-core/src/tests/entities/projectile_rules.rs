//! Projectile rules shared across projectile types in a live world.

use super::*;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SHOOTER: u128 = 0x5e7_0a1;

fn joined_world() -> (WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "shooter", SHOOTER);
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 0.5,
            y: 70.0,
            z: -20.5,
        },
    );
    (receiver, world)
}

#[test]
fn an_arrow_aims_a_fireball_it_hits_along_its_shooters_look() {
    let (receiver, mut world) = joined_world();
    let fireball = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID,
            Vec3d {
                x: 0.5,
                y: 70.0,
                z: 10.5,
            },
            EntityData::LargeFireball(LargeFireballData::new(
                HurtingProjectileState::new(None, 0.0),
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                1,
            )),
        )
        .0;
    let mut data = crate::entity::ArrowData::new(plain_stack("minecraft:arrow", 1).unwrap());
    data.projectile.set_owner(Some(Uuid::from_u128(SHOOTER)));
    let arrow = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID,
            Vec3d {
                x: 0.5,
                y: 70.0,
                z: 8.5,
            },
            EntityData::Arrow(data),
        )
        .0;
    world
        .stored_entity_mut_for_test(OVERWORLD, arrow)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 2.5,
        });
    world.tick(receiver.drain());

    let fireball = world
        .stored_entity_for_test(OVERWORLD, fireball)
        .expect("the arrow does not hurt a fireball");
    let EntityData::LargeFireball(fireball_data) = fireball.data() else {
        unreachable!()
    };
    assert_eq!(
        fireball_data.projectile().owner(),
        Some(Uuid::from_u128(SHOOTER))
    );
    assert!(
        fireball.delta_movement().z > 0.9,
        "{:?}",
        fireball.delta_movement()
    );
    let arrow = world.stored_entity_for_test(OVERWORLD, arrow).unwrap();
    assert!(
        arrow.delta_movement().z < 0.0,
        "the refused hit bounces the arrow back: {:?}",
        arrow.delta_movement()
    );
}

#[test]
fn chicks_hatched_against_a_wall_are_moved_out_of_it() {
    let (receiver, mut world) = joined_world();
    for x in 8..16 {
        for y in 64..67 {
            assert!(world.set_block(
                OVERWORLD,
                BlockPos { x, y, z: 20 },
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    let eggs: Vec<i32> = (0..64)
        .map(|index| {
            let id = world
                .spawn_stored_entity_for_test(
                    OVERWORLD,
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EGG_ID,
                    Vec3d {
                        x: 8.5 + f64::from(index % 8),
                        y: 65.0 + f64::from(index / 8) * 0.2,
                        z: 18.5,
                    },
                    EntityData::Egg(crate::entity::EggData::new(
                        plain_stack("minecraft:egg", 1).unwrap(),
                    )),
                )
                .0;
            world
                .stored_entity_mut_for_test(OVERWORLD, id)
                .unwrap()
                .set_delta_movement(Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                });
            id
        })
        .collect();
    for _ in 0..10 {
        world.tick(receiver.drain());
    }
    assert!(eggs
        .iter()
        .all(|id| world.stored_entity_for_test(OVERWORLD, *id).is_none()));

    let chicks: Vec<_> = world
        .stored_entity_ids_for_test(OVERWORLD)
        .into_iter()
        .filter_map(|id| world.stored_entity_for_test(OVERWORLD, id))
        .filter(|entity| {
            entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID
        })
        .collect();
    assert!(!chicks.is_empty(), "64 eggs at 1/8 hatch at least once");
    for chick in chicks {
        assert!(
            chick.bounding_box().max_z <= 20.0 + 1.0e-5,
            "{:?}",
            chick.bounding_box()
        );
    }
}

#[test]
fn llama_spit_into_thorns_hurts_the_llama() {
    let (receiver, mut world) = joined_world();
    let spawn = |world: &mut ServerWorld, type_id: i32, z: f64| {
        world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                type_id,
                Vec3d {
                    x: -10.5,
                    y: 64.0,
                    z,
                },
                EntityData::GeneralLiving(crate::entity::GeneralLivingData::new(type_id).unwrap()),
            )
            .0
    };
    let llama = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID,
        10.5,
    );
    let zombie = spawn(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 15.5);
    let registry: Vec<String> = world
        .datapacks()
        .unwrap()
        .registries
        .registry("minecraft:enchantment")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    let mut chestplate = plain_stack("minecraft:diamond_chestplate", 1)
        .unwrap()
        .summary()
        .clone();
    chestplate.component_patch.enchantments = vec![bbb_protocol::packets::ItemEnchantmentSummary {
        holder_id: registry
            .iter()
            .position(|id| id == "minecraft:thorns")
            .unwrap() as i32,
        level: 10,
    }];
    chestplate
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
    world
        .stored_entity_mut_for_test(OVERWORLD, zombie)
        .and_then(|zombie| zombie.living_mut())
        .unwrap()
        .set_item_slot(
            bbb_protocol::equipment_slots::EquipmentSlot::Chest,
            Some(RuntimeItemStack::from_summary(chestplate)),
        );
    world.tick(receiver.drain());
    let llama_health = |world: &ServerWorld| stored_living(world, OVERWORLD, llama).health();
    let before = llama_health(&world);
    let mut spit = crate::entity::LlamaSpitData::default();
    spit.set_owner(Some(stored_uuid(&world, OVERWORLD, llama)));
    let spit = world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
            Vec3d {
                x: -10.5,
                y: 64.8,
                z: 12.5,
            },
            EntityData::LlamaSpit(spit),
        )
        .0;
    world
        .stored_entity_mut_for_test(OVERWORLD, spit)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
    for _ in 0..10 {
        world.tick(receiver.drain());
    }
    assert!(world.stored_entity_for_test(OVERWORLD, spit).is_none());
    assert!(llama_health(&world) < before);
}
