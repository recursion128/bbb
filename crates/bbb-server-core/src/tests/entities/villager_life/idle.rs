//! What a villager looks at and wanders to between its chores
//! (`VillagerGoalPackages.java:69-100,125-145,175-219,299-310`).

use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID as CAT;

/// Whether the villager's head points along the floor towards `to`.
fn faces(village: &Village, villager: i32, to: Vec3d) -> bool {
    village.entity(villager).is_some_and(|entity| {
        let position = entity.position();
        let yaw = (to.z - position.z).atan2(to.x - position.x).to_degrees() as f32 - 90.0;
        crate::entity::living::wrap_degrees(entity.y_head_rot() - yaw).abs() < 10.0
    })
}

/// The minimal look set of the rest package turns a bedless villager's
/// head to a player within eight blocks.
#[test]
fn a_resting_villager_looks_at_a_player_nearby() {
    let mut village = Village::new();
    village.command("time set 18000");
    village.command("gamerule advance_time false");
    let villager = village.villager(village.at(3, 0));
    // Hold the body still so only the head can turn.
    village.command(
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );
    let player = village.world.publication().players[0].position;
    assert!(village.tick_until(1_200, |village| faces(village, villager, player)));
}

/// `InteractWith.of(EntityType.CAT, 8, ...)` in the idle package walks a
/// villager up to a cat it sees.
#[test]
fn an_idle_villager_walks_up_to_a_cat() {
    let mut village = Village::new();
    village.command("time set 3000");
    village.command("gamerule advance_time false");
    // A glass pen keeps the strolling villager within sight of the cat.
    for (from, to) in [
        ((0, -2), (8, -2)),
        ((0, 9), (8, 9)),
        ((0, -2), (0, 9)),
        ((8, -2), (8, 9)),
    ] {
        let (from, to) = (village.at(from.0, from.1), village.at(to.0, to.1));
        village.command(&format!(
            "fill {} {} {} {} {} {} minecraft:glass",
            from.x,
            from.y,
            from.z,
            to.x,
            to.y + 1,
            to.z
        ));
    }
    let villager = village.villager(village.at(4, 0));
    let cat = village.mob(CAT, village.at(4, 6));
    village.merge(cat, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    let cat_at = village.center(village.at(4, 6));
    assert!(village.tick_until(2_400, |village| village.distance(villager, cat_at) < 2.5));
}

/// `JumpOnBed` runs in the idle package too, not only in play.
#[test]
fn an_idle_baby_villager_jumps_on_a_bed() {
    let mut village = Village::new();
    village.command("time set 7000");
    village.command("gamerule advance_time false");
    village.bed(village.at(3, 2));
    let baby = village.villager(village.at(3, 0));
    village.merge(baby, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
    let floor = village.feet_y;
    assert!(village.tick_until(3_000, |village| {
        village
            .entity(baby)
            .is_some_and(|entity| entity.position().y > floor + 1.0)
    }));
}

/// `StrollToPoiList.create(SECONDARY_JOB_SITE, ...)` in the work package
/// walks a farmer over to farmland near its job site.
#[test]
fn a_working_farmer_strolls_to_its_farmland() {
    let mut village = Village::new();
    village.command("gamerule random_tick_speed 0");
    village.command("time set 3000");
    village.command("gamerule advance_time false");
    let composter = village.at(6, -2);
    village.set_block(composter, "minecraft:composter");
    let field = village.at(6, 2);
    village.set_block(
        BlockPos {
            y: field.y - 1,
            ..field
        },
        "minecraft:farmland",
    );
    let farmer = village.villager(village.at(5, -2));
    employ_farmer(&mut village, farmer, composter);
    let field_at = village.center(field);
    assert!(village.tick_until(3_000, |village| village.distance(farmer, field_at) < 1.6));
}
