//! What a session sees of a ranged mob's synched state: the crossbow
//! charging pose and loaded crossbow, and the illager casting pose.

use super::*;
use bbb_protocol::entity_synched_data::{
    VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID, VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID,
};
use bbb_protocol::packets::EntityDataValueKind;

/// The values of `data_id` the session was sent for `entity_id`, in order.
fn data_rows(events: &[WorldEvent], entity_id: i32, data_id: u8) -> Vec<EntityDataValueKind> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::SetEntityData(update),
            } if update.id == entity_id => Some(update.values.clone()),
            _ => None,
        })
        .flatten()
        .filter(|value| value.data_id == data_id)
        .map(|value| value.value)
        .collect()
}

/// Whether the session was sent the mob's main hand with a loaded crossbow.
fn loaded_crossbow_sent(event: &WorldEvent, entity_id: i32) -> Option<bool> {
    match event {
        WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::SetEquipment(equipment),
        } if equipment.entity_id == entity_id => equipment
            .slots
            .iter()
            .find(|slot| slot.slot == bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            .map(|slot| {
                !slot
                    .item
                    .component_patch
                    .charged_projectiles_items
                    .is_empty()
            }),
        _ => None,
    }
}

/// `setChargingCrossbow` brackets the charge (`RangedCrossbowAttackGoal.java:104`,
/// `:117`), the full charge loads the crossbow the session sees, and the shot
/// empties it again (`CrossbowItem.java:222`, `:181`).
#[test]
fn a_pillager_shows_its_charge_and_loaded_crossbow() {
    let (sender, receiver, mut world) = fixture();
    let (pillager, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_PILLAGER_ID, east(6.0));
    give(
        &sender,
        &receiver,
        &mut world,
        uuid,
        "weapon.mainhand",
        "minecraft:crossbow",
    );
    let mut events = Vec::new();
    let mut shot = false;
    for _ in 0..300 {
        let tick = world.tick(Vec::new());
        shot = tick.events.iter().any(|event| {
            matches!(event, WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.entity_type_id == VANILLA_ENTITY_TYPE_ARROW_ID && add.data == pillager)
        });
        events.extend(tick.events);
        if shot {
            break;
        }
    }
    assert!(shot, "the pillager shoots");
    events.extend(world.tick(Vec::new()).events);
    assert_eq!(
        data_rows(
            &events,
            pillager,
            VANILLA_PILLAGER_CHARGING_CROSSBOW_DATA_ID
        )[..2],
        [
            EntityDataValueKind::Boolean(true),
            EntityDataValueKind::Boolean(false)
        ]
    );
    let loads: Vec<bool> = events
        .iter()
        .filter_map(|event| loaded_crossbow_sent(event, pillager))
        .collect();
    assert_eq!(loads[loads.len() - 2..], [true, false], "{loads:?}");
}

/// `setIsCastingSpell` publishes the spell as the use goal starts and
/// `NONE` once the casting goal stops (`SpellcasterIllager.java:63-66`,
/// `:161-164`, `:205`).
#[test]
fn an_evoker_raises_and_lowers_its_casting_pose() {
    let (_sender, _receiver, mut world) = fixture();
    let (evoker, _) = spawn(&mut world, VANILLA_ENTITY_TYPE_EVOKER_ID, east(6.0));
    let mut events = Vec::new();
    for _ in 0..200 {
        events.extend(world.tick(Vec::new()).events);
        let rows = data_rows(
            &events,
            evoker,
            VANILLA_SPELLCASTER_ILLAGER_SPELL_CASTING_DATA_ID,
        );
        if rows.contains(&EntityDataValueKind::Byte(0)) {
            assert!(
                matches!(rows[0], EntityDataValueKind::Byte(spell) if spell > 0),
                "{rows:?}"
            );
            return;
        }
    }
    panic!("the evoker never finished a cast");
}
