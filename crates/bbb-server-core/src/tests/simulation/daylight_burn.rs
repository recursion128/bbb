//! `Mob.burnUndead` on the real tick: a bare zombie in
//! the noon sun catches fire, a helmet takes the wear instead, and a roof
//! keeps the sky out.

use super::*;

use super::nbt as entry;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

use crate::tests::entities::restore_entity_writes;

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn spawn_zombie(world: &mut ServerWorld, x: f64) -> (i32, Uuid) {
    let position = Vec3d { x, y: 65.0, z: 4.5 };
    let (id, uuid) = helpers::spawn_living_at(world, VANILLA_ENTITY_TYPE_ZOMBIE_ID, position);
    world.load_stored_entity_record_for_test(
        DIMENSION,
        id,
        &[
            entry("NoAI", NbtSummaryValue::Byte(1)),
            entry("PersistenceRequired", NbtSummaryValue::Byte(1)),
        ],
    );
    world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .set_pos(position);
    (id, uuid)
}

fn fire_ticks(world: &ServerWorld, id: i32) -> i32 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .remaining_fire_ticks()
}

fn helmet_damage(world: &ServerWorld, id: i32) -> Option<i32> {
    let helmet = world
        .stored_entity_for_test(DIMENSION, id)?
        .living()?
        .item_by_slot(EquipmentSlot::Head)?;
    assert_eq!(helmet.item_resource_id(), Some("minecraft:iron_helmet"));
    Some(helmet.summary().component_patch.damage.unwrap_or(0))
}

/// `isSunBurnTick` passes for a zombie under open noon
/// sky, which `igniteForSeconds(8.0F)` sets alight for 160 ticks; with a
/// helmet in `sunProtectionSlot` the same tick instead adds `nextInt(2)` to
/// its damage, which the save keeps; and a roof fails `canSeeSky`.
#[test]
fn noon_sun_burns_a_bare_zombie_wears_a_helmet_and_spares_a_roofed_one() {
    let (sender, receiver, mut world) = helpers::restored_world(
        helpers::operator_datapack_config(),
        &restore_candidate(0, None, Vec::new()),
    );
    chat_command(&sender, &receiver, &mut world, 1, "weather clear");
    chat_command(&sender, &receiver, &mut world, 1, "time set noon");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fill 9 68 3 12 68 6 minecraft:stone",
    );
    let (bare, _) = spawn_zombie(&mut world, 2.5);
    let (helmeted, helmeted_uuid) = spawn_zombie(&mut world, 6.5);
    let (roofed, _) = spawn_zombie(&mut world, 10.5);
    // `Mob.burnUndead`'s `igniteForSeconds` is `LivingEntity.igniteForTicks`
    // which scales by `BURNING_TIME`.
    let (scaled, _) = spawn_zombie(&mut world, 14.5);
    world
        .stored_entity_mut_for_test(DIMENSION, scaled)
        .unwrap()
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BURNING_TIME_ID)
        .unwrap()
        .set_base_value(2.0);
    world
        .stored_entity_mut_for_test(DIMENSION, helmeted)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::Head,
            Some(crate::entity::item::plain_stack("minecraft:iron_helmet", 1).unwrap()),
        );

    let mut bare_lit = false;
    let mut scaled_lit = false;
    for _ in 0..1000 {
        world.tick(receiver.drain());
        if !bare_lit && fire_ticks(&world, bare) > 0 {
            assert_eq!(fire_ticks(&world, bare), 160);
            bare_lit = true;
        }
        if !scaled_lit && fire_ticks(&world, scaled) > 0 {
            assert_eq!(fire_ticks(&world, scaled), 320);
            scaled_lit = true;
        }
        assert!(fire_ticks(&world, helmeted) <= 0);
        assert!(fire_ticks(&world, roofed) <= 0);
        if bare_lit
            && scaled_lit
            && helmet_damage(&world, helmeted).is_some_and(|damage| damage > 0)
        {
            break;
        }
    }
    assert!(bare_lit && scaled_lit);
    let damage = helmet_damage(&world, helmeted).unwrap();
    assert!(damage > 0);

    for (dimension, chunk) in world.pending_runtime_entity_chunks() {
        world
            .complete_runtime_entity_chunk(&dimension, chunk, None)
            .unwrap();
    }
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the entity region saves");
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);
    let reloaded = restored
        .live_entity_id_for_uuid_for_test(DIMENSION, helmeted_uuid)
        .expect("the helmeted zombie reloads");
    assert_eq!(helmet_damage(&restored, reloaded), Some(damage));
}
