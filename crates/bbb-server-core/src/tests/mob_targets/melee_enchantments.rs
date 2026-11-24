//! `Mob.doHurtTarget` (Mob.java:1355-1359) reads the weapon's enchantments.

use super::melee::{fixture, hits, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

/// Sharpness I adds `0.5 * level + 0.5` to the zombie's 3 attack damage.
#[test]
fn a_zombie_with_a_sharpness_weapon_hits_harder() {
    let (sender, receiver, mut world) = fixture();
    let (_, zombie) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            z: PLAYER_POSITION.z + 3.0,
            ..PLAYER_POSITION
        },
    );
    let command = format!(
        "item replace entity {zombie} weapon.mainhand with minecraft:stick[enchantments={{sharpness:1}}]"
    );
    let tick = console_command(&sender, &receiver, &mut world, 20, &command);
    assert!(
        console_command_answer(&tick).0.is_some(),
        "{:?}",
        console_command_answer(&tick)
    );
    let hits = hits(&mut world, 100);
    assert_eq!(hits.first().map(|hit| hit.1), Some(4.0), "{hits:?}");
}
