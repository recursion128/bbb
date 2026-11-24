//! `LivingEntity.detectEquipmentUpdates` (`LivingEntity.java:2873-2951`) as
//! the tracking player sees a session's slots change.

use bbb_protocol::equipment_slots::EquipmentSlot;

use super::*;

fn equipment_updates(
    tick: &TickResult,
    viewer: u64,
    entity_id: i32,
) -> Vec<Vec<(EquipmentSlot, Option<i32>)>> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SetEquipment(update) if update.entity_id == entity_id => Some(
                update
                    .slots
                    .iter()
                    .map(|slot| (slot.slot, slot.item.item_id))
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

/// `handleEquipmentChanges` (`:2943-2951`) goes to the trackers alone, once
/// per change, in `EquipmentSlot` order.
#[test]
fn a_changed_slot_is_sent_to_the_trackers_once() {
    let mut fixture = Fixture::new(GameType::Survival);
    let alice = fixture.alice;
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:iron_sword", 1));
    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:shield", 1));

    let changed = fixture.idle();
    assert_eq!(
        equipment_updates(&changed, BOB, alice),
        [vec![
            (
                EquipmentSlot::MainHand,
                Some(item_id("minecraft:iron_sword"))
            ),
            (EquipmentSlot::OffHand, Some(item_id("minecraft:shield"))),
        ]]
    );
    assert!(equipment_updates(&changed, ALICE, alice).is_empty());
    assert!(equipment_updates(&fixture.idle(), BOB, alice).is_empty());

    // `SetCarriedItem` onto an empty hotbar slot empties the main hand.
    let selected = fixture.play(ALICE, PlayServerbound::SetCarriedItem { slot: 1 });
    assert_eq!(
        equipment_updates(&selected, BOB, alice),
        [vec![(EquipmentSlot::MainHand, None)]]
    );
}

/// `handleHandSwap` (`:2928-2941`): the two hands trading places is one
/// `SWAP_HANDS` entity event instead of an equipment packet.
#[test]
fn swapping_hands_is_an_entity_event() {
    let mut fixture = Fixture::new(GameType::Survival);
    let alice = fixture.alice;
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:iron_sword", 1));
    fixture.hold(OFFHAND_MENU_SLOT, stack("minecraft:shield", 1));
    fixture.idle();

    let swapped = fixture.action(PlayerActionKind::SwapItemWithOffhand);
    assert_eq!(entity_events(&swapped, BOB, alice), [55]);
    assert!(entity_events(&swapped, ALICE, alice).is_empty());
    assert!(equipment_updates(&swapped, BOB, alice).is_empty());
    assert_eq!(
        fixture.slot(OFFHAND_SLOT).item_id,
        Some(item_id("minecraft:iron_sword"))
    );
}
