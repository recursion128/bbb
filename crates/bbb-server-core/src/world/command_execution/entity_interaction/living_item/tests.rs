use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::parse_advancement;
use crate::world::{JoinContext, ServerWorldConfig, WorldCommand};
use bbb_protocol::data_component_types::*;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_SHEEP_ID};
use bbb_protocol::packets::{
    DataComponentPatchSummary, GameProfile, InteractEntity, NbtSummaryEntry, NbtSummaryValue,
    PlayServerbound,
};
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::json;
use std::sync::OnceLock;

const PLAYER: ConnectionId = ConnectionId(1);

fn world(creative: bool) -> ServerWorld {
    static STARTUP: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    let mut startup = (**STARTUP.get_or_init(|| {
        Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap(),
        )
    }))
    .clone();
    startup.advancements = AdvancementSet {
        advancements: [
            ("test:sheep", "minecraft:sheep"),
            ("test:other", "minecraft:zombie"),
        ]
        .into_iter()
        .map(|(id, target)| {
            AdvancementElement {
        id: id.into(), source_pack: "test".into(), advancement: parse_advancement(&json!({
            "criteria":{"done":{"trigger":"minecraft:player_interacted_with_entity", "conditions":{
                "player":{"type":"minecraft:player"}, "entity":{"type":target},
                "item":{"items":"minecraft:red_dye","count":1}
            }}}
        })).unwrap(),
    }
        })
        .collect(),
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: if creative {
            GameType::Creative
        } else {
            GameType::Survival
        },
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    world.set_test_respawn_radius(0);
    world.join(
        PLAYER,
        GameProfile {
            uuid: Uuid::from_u128(1),
            name: "dyer".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    world.levels[PLAYER].mark_client_loaded();
    world.levels[PLAYER]
        .inventory
        .clear_all_items(None)
        .unwrap();
    world.tick(Vec::new());
    world
}

fn sheep(world: &mut ServerWorld, baby: bool) -> i32 {
    let pos = world.levels[PLAYER].position();
    let (id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        Vec3d {
            z: pos.z + 1.0,
            ..pos
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_SHEEP_ID).unwrap()),
    );
    let entity = world
        .level_mut(Java26BuiltinDimension::Overworld)
        .entities
        .get_mut(id)
        .unwrap();
    if baby {
        crate::entity::general_living::set_spawner_age(entity, -24000);
    }
    id
}

fn stack(item: &str, count: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
                .unwrap()
                .id,
        ),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    })
}

fn interact(world: &mut ServerWorld, id: i32, hand: InteractionHand) -> Vec<WorldEvent> {
    world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id: PLAYER,
                packet: PlayServerbound::Interact(InteractEntity {
                    entity_id: id,
                    hand,
                    location: Vec3d::default(),
                    using_secondary_action: false,
                }),
            },
        )])
        .events
}

fn entity_color(entity: &crate::entity::Entity) -> i8 {
    crate::entity::tracking::synched_values(entity)
        .into_iter()
        .find_map(|row| {
            if row.value.data_id == bbb_protocol::entity_synched_data::VANILLA_SHEEP_WOOL_DATA_ID {
                if let bbb_protocol::packets::EntityDataValueKind::Byte(value) = row.value.value {
                    return Some(value & 15);
                }
            }
            None
        })
        .unwrap()
}

fn color(world: &ServerWorld, id: i32) -> i8 {
    entity_color(
        world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .get(id)
            .unwrap(),
    )
}

#[test]
fn packet_dyes_baby_updates_metadata_and_restores_saved_color_with_original_item_criterion() {
    let mut world = world(false);
    let observer = ConnectionId(2);
    world.join(
        observer,
        GameProfile {
            uuid: Uuid::from_u128(2),
            name: "observer".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    let id = sheep(&mut world, true);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(InteractionHand::MainHand, stack("minecraft:red_dye", 1));
    let events = interact(&mut world, id, InteractionHand::MainHand);
    assert_eq!(color(&world, id), 14);
    let sounds = events.iter().filter_map(|event| {
        match event {
            WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::Sound(sound) }
                if matches!(sound.sound, bbb_protocol::packets::SoundEventHolder::Reference { registry_id }
                    if bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(registry_id) == Some("minecraft:item.dye.use")) => Some((*connection_id, sound)),
            _ => None,
        }
    }).collect::<Vec<_>>();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].0, observer);
    assert_eq!(
        sounds[0].1.source,
        bbb_protocol::packets::SoundSource::Players
    );
    assert_eq!((sounds[0].1.volume, sounds[0].1.pitch), (1.0, 1.0));
    assert_eq!(
        world
            .game_events(Java26BuiltinDimension::Overworld)
            .0
            .iter()
            .filter(
                |event| event.game_event == crate::level_event::VanillaGameEvent::ENTITY_INTERACT
            )
            .count(),
        1
    );

    assert!(world.levels[PLAYER].inventory.selected_stack().is_empty());
    assert!(world.player_advancement_is_done(PLAYER, "test:sheep"));
    assert!(!world.player_advancement_is_done(PLAYER, "test:other"));
    assert!(events.iter().any(|event| matches!(event, WorldEvent::Send {
        packet: ClientboundPlayPacket::SetEntityData(data), ..
    } if data.id == id && data.values.iter().any(|value|
        value.data_id == bbb_protocol::entity_synched_data::VANILLA_SHEEP_WOOL_DATA_ID
            && value.value == bbb_protocol::packets::EntityDataValueKind::Byte(14)))));
    assert!(!events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::EntityAnimation(_),
            ..
        }
    )));
    let level = world.level(Java26BuiltinDimension::Overworld);
    let entity = level.entities.get(id).unwrap();
    let chunk = (
        entity.block_position().x.div_euclid(16),
        entity.block_position().z.div_euclid(16),
    );
    let saved = crate::entity::write_entity_chunk(&level.entities, chunk).unwrap();
    let mut restored = crate::entity::EntityStore::new(0, crate::entity::EntityIdCounter::new());
    let ids = crate::entity::read_entity_chunk(&mut restored, &saved).unwrap();
    assert!(ids
        .into_iter()
        .any(|id| entity_color(restored.get(id).unwrap()) == 14));
}

#[test]
fn packet_dyed_adult_shears_new_wool_and_pass_cases_preserve_items() {
    let mut world = world(false);
    let id = sheep(&mut world, false);
    let hand = InteractionHand::OffHand;
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, stack("minecraft:red_dye", 2));
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 14);
    assert_eq!(world.levels[PLAYER].inventory.off_hand_stack().count(), 1);
    interact(&mut world, id, hand);
    assert_eq!(world.levels[PLAYER].inventory.off_hand_stack().count(), 1);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, stack("minecraft:shears", 1));
    interact(&mut world, id, hand);
    let wool = world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter();
    assert!(wool
        .filter_map(|id| world
            .level(Java26BuiltinDimension::Overworld)
            .entities
            .get(id))
        .any(|entity| matches!(entity.data(), EntityData::Item(item)
            if item.item().item_id() == stack("minecraft:red_wool", 1).item_id())));
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, stack("minecraft:blue_dye", 2));
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 14);
    assert_eq!(world.levels[PLAYER].inventory.off_hand_stack().count(), 2);
}

#[test]
fn packet_resolves_dye_patch_removal_and_creative_components_and_equips_first() {
    let mut world = world(true);
    let id = sheep(&mut world, false);
    let hand = InteractionHand::MainHand;
    let mut patched = stack("minecraft:red_dye", 3);
    let patch = &mut patched.summary_mut().component_patch;
    patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_DYE_TYPE_ID];
    patch.dye_color_id = Some(11);
    patch.custom_data = Some(bbb_protocol::NbtSummaryValue::Compound(vec![
        bbb_protocol::NbtSummaryEntry {
            name: "kept".into(),
            value: bbb_protocol::NbtSummaryValue::Int(7),
        },
    ]));
    patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID);
    let resolver = world.item_stack_resolver();
    patched = RuntimeItemStack::try_read(
        &patched.write(resolver.as_dyn()).unwrap().unwrap(),
        resolver.as_dyn(),
    )
    .unwrap();
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, patched.clone());
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 11);
    assert_eq!(world.levels[PLAYER].inventory.selected_stack(), &patched);
    let mut removed = stack("minecraft:red_dye", 2);
    removed
        .summary_mut()
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_DYE_TYPE_ID);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, removed.clone());
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 11);
    assert_eq!(world.levels[PLAYER].inventory.selected_stack(), &removed);
    let mut other = stack("minecraft:stick", 2);
    other.summary_mut().component_patch = patched.summary().component_patch.clone();
    other.summary_mut().component_patch.dye_color_id = Some(14);

    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, other.clone());
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 11);
    assert_eq!(world.levels[PLAYER].inventory.selected_stack(), &other);
    use bbb_protocol::equipment_slots::EquipmentSlot;
    let mut equip = stack("minecraft:red_dye", 1);
    let mut eq = stack("minecraft:carved_pumpkin", 1)
        .effective_component(
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            world.item_stack_resolver().as_dyn(),
        )
        .unwrap()
        .unwrap()
        .into_owned();
    eq.equippable.as_mut().unwrap().equip_on_interact = true;
    equip.summary_mut().component_patch.equippable = eq.equippable;
    equip
        .summary_mut()
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    let resolver = world.item_stack_resolver();
    equip = RuntimeItemStack::try_read(
        &equip.write(resolver.as_dyn()).unwrap().unwrap(),
        resolver.as_dyn(),
    )
    .unwrap();
    let dead = sheep(&mut world, false);
    world
        .level_mut(Java26BuiltinDimension::Overworld)
        .entities
        .get_mut(dead)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_health(0.0);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, stack("minecraft:red_dye", 1));
    interact(&mut world, dead, hand);
    assert_eq!(color(&world, dead), 0);
    assert_eq!(world.levels[PLAYER].inventory.selected_stack().count(), 1);
    world.levels[PLAYER]
        .inventory
        .set_item_in_hand(hand, equip.clone());
    interact(&mut world, id, hand);
    assert_eq!(color(&world, id), 11);
    let entity = world
        .level(Java26BuiltinDimension::Overworld)
        .entities
        .get(id)
        .unwrap();
    assert_eq!(
        entity.living().unwrap().item_by_slot(EquipmentSlot::Head),
        Some(&equip)
    );
    assert_eq!(world.levels[PLAYER].inventory.selected_stack(), &equip);
}

#[test]
fn creative_milking_deduplicates_effective_components_and_preserves_distinct_buckets() {
    for (component, value, expected_milk) in [
        (
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
            1,
        ),
        (
            "minecraft:rarity",
            NbtSummaryValue::String("uncommon".into()),
            2,
        ),
        ("!minecraft:rarity", NbtSummaryValue::Compound(vec![]), 2),
    ] {
        let mut world = world(true);
        let resolver = world.item_stack_resolver();
        let existing = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:milk_bucket".into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(1),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: component.into(),
                        value,
                    }]),
                },
            ]),
            &resolver,
        )
        .unwrap();
        let pos = world.levels[PLAYER].position();
        let (id, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                z: pos.z + 1.0,
                ..pos
            },
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        );
        let empty_bucket = stack("minecraft:bucket", 1);
        world.levels[PLAYER]
            .inventory
            .set_item_in_hand(InteractionHand::MainHand, empty_bucket.clone());
        world.levels[PLAYER]
            .inventory
            .set_item_in_hand(InteractionHand::OffHand, existing.clone());

        interact(&mut world, id, InteractionHand::MainHand);

        let inventory = &world.levels[PLAYER].inventory;
        assert_eq!(inventory.selected_stack(), &empty_bucket);
        assert_eq!(inventory.off_hand_stack(), &existing);
        assert_eq!(
            inventory
                .inventory_slots()
                .iter()
                .filter(|item| item.item_id() == existing.item_id())
                .map(RuntimeItemStack::count)
                .sum::<i32>(),
            expected_milk,
            "{component}",
        );
        assert_eq!(
            inventory
                .inventory_slots()
                .iter()
                .map(RuntimeItemStack::count)
                .sum::<i32>(),
            expected_milk + 1,
        );
        if expected_milk == 2 {
            let plain = stack("minecraft:milk_bucket", 1);
            assert!(inventory
                .inventory_slots()
                .iter()
                .any(|item| item == &plain));
        }
    }
}
