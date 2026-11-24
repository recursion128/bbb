use super::*;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_MOVING_PISTON_ID, VANILLA_BLOCK_TNT_ID};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DIGGING_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_WARDEN_ID};

const POS: BlockPos = BlockPos { x: 6, y: 300, z: 6 };

fn explode_at(world: &mut ServerWorld, center: Vec3d) {
    world.explode(
        OVERWORLD,
        PendingExplosion {
            source: None,
            center,
            radius: 3.0,
            fire: false,
            interaction: ExplosionInteraction::Tnt,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut Vec::new(),
    );
}

const CENTER: Vec3d = Vec3d {
    x: 6.5,
    y: 300.5,
    z: 6.5,
};

/// `TntBlock.wasExploded` (`TntBlock.java:75-82`) primes nothing while
/// `tnt_explodes` is off; the block is still blown away.
#[test]
fn an_exploded_tnt_block_is_only_removed_while_tnt_explodes_is_off() {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    assert!(world.set_game_rule_for_test(
        "minecraft:tnt_explodes",
        bbb_protocol::game_rules::GameRuleValue::Boolean(false),
    ));
    assert!(world.set_block(
        OVERWORLD,
        POS,
        vanilla_block_default_state_id(VANILLA_BLOCK_TNT_ID).unwrap(),
        SET_BLOCK_UPDATE_ALL,
    ));
    explode_at(&mut world, CENTER);
    assert_eq!(
        world.block_state(OVERWORLD, POS),
        Some(block_state_id("minecraft:air"))
    );
    assert!(!world
        .stored_entity_ids_for_test(OVERWORLD)
        .into_iter()
        .any(|id| world
            .stored_entity_for_test(OVERWORLD, id)
            .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_TNT_ID)));
}

/// `MovingPistonBlock.getDrops` (`MovingPistonBlock.java:95-98`) drops what
/// the piston was moving.
#[test]
fn an_exploded_moving_piston_drops_the_block_it_was_moving() {
    let table = serde_json::json!({"type":"minecraft:block","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:dirt"}]}]});
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        "minecraft:blocks/dirt",
        &table,
    ));
    assert!(world.set_block(
        OVERWORLD,
        POS,
        vanilla_block_default_state_id(VANILLA_BLOCK_MOVING_PISTON_ID).unwrap(),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world
        .level_block_entity_for_test(OVERWORLD, POS)
        .expect("the moving piston has its block entity")
        .configure_piston(
            block_state_id("minecraft:dirt"),
            Direction::East,
            true,
            false
        ));
    explode_at(&mut world, CENTER);
    assert_eq!(
        world.block_state(OVERWORLD, POS),
        Some(block_state_id("minecraft:air"))
    );
    let drops = world
        .stored_items_for_test(OVERWORLD)
        .into_iter()
        .map(|(_, _, id, count, ..)| (id, count))
        .collect::<Vec<_>>();
    assert_eq!(drops, [("minecraft:dirt".to_owned(), 1)]);
}

/// `Warden.ignoreExplosion` (`Warden.java:367-369`) skips a digging or
/// emerging warden entirely, knockback included.
#[test]
fn a_digging_warden_is_not_knocked_back_by_an_explosion() {
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    let warden = |world: &mut ServerWorld, x: f64| {
        world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                VANILLA_ENTITY_TYPE_WARDEN_ID,
                Vec3d { x, ..CENTER },
                crate::entity::EntityData::GeneralLiving(
                    crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_WARDEN_ID)
                        .expect("a warden is a living entity"),
                ),
            )
            .0
    };
    let digging = warden(&mut world, 8.5);
    let standing = warden(&mut world, 4.5);
    world
        .stored_entity_mut_for_test(OVERWORLD, digging)
        .unwrap()
        .set_pose_id(VANILLA_ENTITY_POSE_DIGGING_ID);
    world.track_stored_chunk_for_test(OVERWORLD, (0, 0));
    explode_at(&mut world, CENTER);
    let pushed = |world: &ServerWorld, id| {
        world
            .stored_entity_for_test(OVERWORLD, id)
            .unwrap()
            .delta_movement()
            != Vec3d::default()
    };
    assert!(!pushed(&world, digging));
    assert!(pushed(&world, standing));
}
