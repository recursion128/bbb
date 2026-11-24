use super::*;
use crate::connection::ClientboundPlayPacket;
use crate::world::WorldEvent;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{SoundEventHolder, SoundSource};
use bbb_server_types::save::Java26BuiltinDimension;

#[test]
fn a_dispenser_shears_copper_golem_antennae_using_the_live_tag() {
    for (item, slot, dead, custom, expected) in [
        ("poppy", "saddle", false, false, true),
        ("poppy", "head", false, false, false),
        ("stick", "saddle", false, false, false),
        ("poppy", "saddle", true, false, false),
        ("poppy", "saddle", false, true, false),
        ("stick", "saddle", false, true, true),
    ] {
        let mut world = NativeWorld::new();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "viewer", 1);
        crate::tests::console_command(&sender, &receiver, &mut world.world, 1, "forceload add 0 0");
        world.set(UNDER_FRONT, stone());
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut world.world,
            2,
            "summon minecraft:copper_golem 0.5 64 1.5",
        );
        let golem = world
            .entities()
            .into_iter()
            .find(|entity| {
                entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID
            })
            .unwrap();
        let id = golem.id();
        let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
            .world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
        else {
            unreachable!()
        };
        let bbb_protocol::NbtSummaryValue::Compound(extra) = bbb_protocol::parse_snbt_compound(&format!(
            "{{NoAI:1b,Health:{}f,equipment:{{{slot}:{{id:\"minecraft:{item}\",count:7,components:{{\"minecraft:custom_data\":{{kept:17}}}}}}}}}}",
            if dead { 0 } else { 10 },
        )).unwrap() else { unreachable!() };
        for entry in extra {
            entries.retain(|field| field.name != entry.name);
            entries.push(entry);
        }
        world.world.load_stored_entity_record_for_test(
            Java26BuiltinDimension::Overworld,
            id,
            &entries,
        );
        let original = world
            .world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            .unwrap()
            .living()
            .unwrap()
            .equipment()
            .get(if slot == "saddle" {
                EquipmentSlot::Saddle
            } else {
                EquipmentSlot::Head
            })
            .unwrap()
            .clone();
        if custom {
            world.world.install_level_registry_tags(std::sync::Arc::new(
                crate::registry_tags::RegistryTagSnapshot::new([(
                    "minecraft:item".into(),
                    "minecraft:shearable_from_copper_golem".into(),
                    vec!["minecraft:stick".into()],
                )]),
            ));
        }
        world.set(FLOOR, stone());
        world.set(DROPPER, dispenser("south", false));
        let mut slots = empty_nine();
        slots[0] = stack("minecraft:shears", 1);
        set_items(&mut world, DROPPER, slots);
        let redstone = super::super::default_state("minecraft:redstone_block");
        world.set(SIDE, redstone);
        world.neighbor_changed(DROPPER, block_type_of(redstone).unwrap());
        let mut heard = false;
        for _ in 0..5 {
            let tick = world.world.tick(receiver.drain());
            heard |= tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if sound.source == SoundSource::Blocks && matches!(sound.sound,
                    SoundEventHolder::Reference { registry_id }
                    if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id)
                        == Some("minecraft:entity.copper_golem.shear"))
            ));
        }
        assert_eq!(heard, expected, "{item}/{slot}/dead={dead}/custom={custom}");
        assert_eq!(
            held(&mut world).component_patch.damage,
            expected.then_some(1)
        );
        let drops: Vec<_> = world
            .entities()
            .into_iter()
            .filter_map(|entity| {
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some(data.item().clone())
            })
            .collect();
        assert_eq!(drops, if expected { vec![original] } else { vec![] });
        if expected {
            assert!(world
                .world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
                .unwrap()
                .living()
                .unwrap()
                .equipment()
                .get(EquipmentSlot::Saddle)
                .is_none());
            assert!(world
                .world
                .game_events(Java26BuiltinDimension::Overworld)
                .0
                .iter()
                .any(|event| event.game_event == crate::level_event::VanillaGameEvent::SHEAR));
        }
    }
}
