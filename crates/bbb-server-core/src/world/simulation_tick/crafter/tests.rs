use super::*;
use crate::entity::EntityData;
use crate::world::TickResult;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::item_types::{VANILLA_ITEM_OAK_LOG_ID, VANILLA_ITEM_OAK_PLANKS_ID};
use bbb_protocol::packets::ItemStackSummary;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use std::sync::{Arc, OnceLock};

const POS: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const POWER: BlockPos = BlockPos { x: 7, y: 64, z: 8 };
const PLAYER: ConnectionId = ConnectionId(1);

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn fixture() -> (ServerWorld, crate::mailbox::WorldCommandReceiver) {
    let (world, _, receiver) = fixture_with_config(|_| {});
    (world, receiver)
}

pub(super) fn fixture_with_config(
    configure: impl FnOnce(&mut DatapackStartup),
) -> (
    ServerWorld,
    crate::mailbox::WorldCommandSender,
    crate::mailbox::WorldCommandReceiver,
) {
    static STARTUP: OnceLock<DatapackStartup> = OnceLock::new();
    let mut startup = STARTUP
        .get_or_init(|| {
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap()
        })
        .clone();
    startup.advancements.advancements.push(bbb_server_datapack::advancement::AdvancementElement {
        id: "test:crafter".into(), source_pack: "test".into(),
        advancement: crate::tests::parse_advancement(&serde_json::json!({
            "criteria":{"crafted":{"trigger":"minecraft:crafter_recipe_crafted","conditions":{
                "recipe_id":"minecraft:oak_planks", "ingredients":[{"items":"minecraft:oak_log","count":2}]
            }}}, "rewards":{"experience":7}
        })).unwrap(),
    });
    configure(&mut startup);
    let mut world = ServerWorld::new(crate::world::ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    world.set_test_respawn_radius(0);
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "crafter", 1);
    world.set_player_position_for_test(
        PLAYER,
        Vec3d {
            x: 11.5,
            y: 64.0,
            z: 8.5,
        },
    );
    world.set_block(
        Java26BuiltinDimension::Overworld,
        POS,
        state("minecraft:crafter"),
        SET_BLOCK_UPDATE_ALL,
    );
    let item = bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_OAK_LOG_ID),
        count: 2,
        component_patch: Default::default(),
    });
    world
        .level_mut(Java26BuiltinDimension::Overworld)
        .chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .crafter_mut()
        .unwrap()
        .items_mut()[0] = item;
    (world, sender, receiver)
}

fn events(tick: &TickResult) -> Vec<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::LevelEvent(packet),
            } if *connection_id == PLAYER && packet.pos == POS => Some(packet.event_type),
            _ => None,
        })
        .collect()
}

#[test]
fn crafter_short_redstone_pulse_crafts_after_four_ticks_and_publishes_before_consumption() {
    let (mut world, receiver) = fixture();
    let dim = Java26BuiltinDimension::Overworld;
    world.set_block(
        dim,
        POWER,
        state("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    );
    assert_eq!(
        world
            .get_block_entity(dim, POS)
            .unwrap()
            .crafter()
            .unwrap()
            .triggered(),
        1
    );
    world.set_block(dim, POWER, state("minecraft:air"), SET_BLOCK_UPDATE_ALL);
    assert_eq!(
        world
            .get_block_entity(dim, POS)
            .unwrap()
            .crafter()
            .unwrap()
            .triggered(),
        0
    );
    for _ in 0..3 {
        let tick = world.tick(receiver.drain());
        assert!(!events(&tick).contains(&1049));
    }
    let crafted = world.tick(receiver.drain());
    assert_eq!(events(&crafted), vec![1049, 2010]);
    let crafter = world.get_block_entity(dim, POS).unwrap().crafter().unwrap();
    assert_eq!(crafter.items()[0].count(), 1);
    assert_eq!(crafter.crafting_ticks_remaining(), 5);
    let saved = world.block_entity_save_for_test(dim, POS).unwrap();
    let restored = BlockEntity::load_static_with_resolver(
        POS,
        world.block_state(dim, POS).unwrap(),
        &saved,
        &world.item_stack_resolver(),
    )
    .unwrap()
    .unwrap();
    let body = restored.crafter().unwrap();
    assert_eq!(body.crafting_ticks_remaining(), 5);
    assert_eq!(body.items()[0].count(), 1);
    *world.level_block_entity_for_test(dim, POS).unwrap() = restored;
    assert!(world.player_advancement_is_done(PLAYER, "test:crafter"));
    assert_eq!(world.levels[PLAYER].experience.total(), 7);
    let item = world
        .level(dim)
        .entities
        .ids()
        .into_iter()
        .find_map(|id| {
            let entity = world.level(dim).entities.get(id)?;
            let EntityData::Item(item) = entity.data() else {
                return None;
            };
            (item.item().item_id().unwrap() == VANILLA_ITEM_OAK_PLANKS_ID).then_some((entity, item))
        })
        .unwrap();
    assert_eq!(item.1.item().count(), 4);
    assert_eq!(item.1.pickup_delay(), 0);
    assert!(item.0.delta_movement().z < 0.0);
    for _ in 0..5 {
        world.tick(receiver.drain());
    }
    assert_eq!(
        world
            .get_block_entity(dim, POS)
            .unwrap()
            .crafter()
            .unwrap()
            .crafting_ticks_remaining(),
        0
    );
    assert!(
        vanilla_block_state_identity(world.block_state(dim, POS).unwrap())
            .unwrap()
            .properties
            .get("crafting")
            .is_some_and(|value| value == "false")
    );
    world.set_block(
        dim,
        POWER,
        state("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    );
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    assert!(world
        .get_block_entity(dim, POS)
        .unwrap()
        .crafter()
        .unwrap()
        .items()[0]
        .is_empty());
}

#[test]
fn crafter_outputs_to_front_container_without_ejection_events_or_criterion() {
    let (mut world, receiver) = fixture();
    let dim = Java26BuiltinDimension::Overworld;
    let target = BlockPos { z: 7, ..POS };
    world.set_block(dim, target, state("minecraft:barrel"), SET_BLOCK_UPDATE_ALL);
    world.set_block(
        dim,
        POWER,
        state("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    );
    let mut seen = Vec::new();
    for _ in 0..4 {
        seen.extend(events(&world.tick(receiver.drain())));
    }
    assert!(!seen.contains(&1049) && !seen.contains(&2010));
    assert!(!world.player_advancement_is_done(PLAYER, "test:crafter"));
    let container = world
        .container_items_at_by_key(&dim.into(), target)
        .unwrap();
    assert_eq!(container[0].item_id(), Some(VANILLA_ITEM_OAK_PLANKS_ID));
    assert_eq!(container[0].count(), 4);
    assert_eq!(
        world
            .get_block_entity(dim, POS)
            .unwrap()
            .crafter()
            .unwrap()
            .items()[0]
            .count(),
        1
    );
}

#[test]
fn crafter_map_post_process_allocates_saved_map() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID as MAP_ID,
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID as POST,
    };
    use bbb_protocol::packets::{DataComponentPatchSummary, MapPostProcessingSummary};
    let (mut world, _) = fixture();
    let original = world
        .create_map(Java26BuiltinDimension::Overworld, 150, 130, 0, true, false)
        .unwrap();
    let old_id = original.map_id().unwrap();
    let resolver = world.item_stack_resolver();
    for (operation, expected_scale, expected_locked) in [
        (MapPostProcessingSummary::Scale, 1, false),
        (MapPostProcessingSummary::Lock, 0, true),
    ] {
        let input =
            bbb_server_types::item_stack::RuntimeItemStack::from_summary(ItemStackSummary {
                item_id: original.item_id(),
                count: 3,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![MAP_ID, POST],
                    map_id: Some(old_id),
                    map_post_processing: Some(operation),
                    ..Default::default()
                }
                .into(),
            });
        let result = world
            .maps
            .post_process_crafted_map(input, &resolver)
            .unwrap();
        assert_eq!(result.count(), 3);
        assert_ne!(result.map_id().unwrap(), old_id);
        assert_eq!(result.summary().component_patch.map_post_processing, None);
        let saved = world.maps.dirty_persistent_snapshots();
        let map = &saved
            .iter()
            .find(|map| map.map_id == result.map_id().unwrap())
            .unwrap()
            .data;
        assert_eq!(map.scale, expected_scale);
        assert_eq!(map.locked, expected_locked);
    }
    let old = world
        .maps
        .dirty_persistent_snapshots()
        .into_iter()
        .find(|map| map.map_id == old_id)
        .unwrap();
    assert_eq!(old.data.scale, 0);
    assert!(!old.data.locked);
}

#[test]
fn crafter_map_extending_recipe_scales_its_output_and_consumes_the_grid() {
    use bbb_protocol::item_types::{VANILLA_ITEM_FILLED_MAP_ID, VANILLA_ITEM_PAPER_ID};
    let (mut world, receiver) = fixture();
    let dim = Java26BuiltinDimension::Overworld;
    let source = world.create_map(dim, 150, 130, 0, true, false).unwrap();
    let old_id = source.map_id().unwrap();
    let paper = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_PAPER_ID),
        count: 1,
        component_patch: Default::default(),
    });
    let crafter = world
        .level_mut(dim)
        .chunks
        .block_entities_mut()
        .get_mut(POS)
        .unwrap()
        .crafter_mut()
        .unwrap();
    crafter.items_mut().fill(paper);
    crafter.items_mut()[4] = source;
    assert!(world.open_crafter_menu(PLAYER, dim, POS, &mut Vec::new()));
    let preview = world.levels[PLAYER].inventory.slot_item(45).into_owned();
    assert_eq!(preview.map_id(), Some(old_id));
    assert_eq!(
        preview.summary().component_patch.map_post_processing,
        Some(bbb_protocol::packets::MapPostProcessingSummary::Scale)
    );
    assert_eq!(world.maps.dirty_persistent_snapshots().len(), 1);
    world.set_block(
        dim,
        POWER,
        state("minecraft:redstone_block"),
        SET_BLOCK_UPDATE_ALL,
    );
    for _ in 0..4 {
        world.tick(receiver.drain());
    }
    let result = world
        .level(dim)
        .entities
        .ids()
        .into_iter()
        .find_map(|id| {
            let entity = world.level(dim).entities.get(id)?;
            let EntityData::Item(item) = entity.data() else {
                return None;
            };
            (item.item().item_id() == Some(VANILLA_ITEM_FILLED_MAP_ID)).then(|| item.item().clone())
        })
        .unwrap();
    assert_ne!(result.map_id(), Some(old_id));
    assert_eq!(result.summary().component_patch.map_post_processing, None);
    let saved = world.maps.dirty_persistent_snapshots();
    assert_eq!(
        saved
            .iter()
            .find(|map| map.map_id == result.map_id().unwrap())
            .unwrap()
            .data
            .scale,
        1
    );
    assert_eq!(
        saved
            .iter()
            .find(|map| map.map_id == old_id)
            .unwrap()
            .data
            .scale,
        0
    );
    assert!(world
        .get_block_entity(dim, POS)
        .unwrap()
        .crafter()
        .unwrap()
        .items()
        .iter()
        .all(RuntimeItemStack::is_empty));
}

#[test]
fn hopper_balances_crafter_slots_using_effective_components() {
    use bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    for (component, value, expected) in [
        (
            "minecraft:rarity",
            NbtSummaryValue::String("common".into()),
            [4, 2],
        ),
        (
            "minecraft:rarity",
            NbtSummaryValue::String("uncommon".into()),
            [5, 1],
        ),
        (
            "!minecraft:rarity",
            NbtSummaryValue::Compound(vec![]),
            [5, 1],
        ),
    ] {
        let (mut world, receiver) = fixture();
        let dim = Java26BuiltinDimension::Overworld;
        let plain = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        });
        let other = RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String("minecraft:stone".into()),
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
            &world.item_stack_resolver(),
        )
        .unwrap();
        let crafter = world
            .level_block_entity_for_test(dim, POS)
            .unwrap()
            .crafter_mut()
            .unwrap();
        crafter.items_mut().fill(RuntimeItemStack::empty());
        for slot in 2..9 {
            assert!(crafter.set_slot_disabled(slot, true));
        }
        crafter.items_mut()[0] = plain.with_count(4);
        crafter.items_mut()[1] = other;
        let hopper_pos = BlockPos {
            y: POS.y + 1,
            ..POS
        };
        world.set_block(
            dim,
            hopper_pos,
            state("minecraft:hopper"),
            SET_BLOCK_UPDATE_ALL,
        );
        world
            .level_block_entity_for_test(dim, hopper_pos)
            .unwrap()
            .hopper_mut()
            .unwrap()
            .items_mut()[0] = plain;

        world.tick(receiver.drain());

        let items = world
            .get_block_entity(dim, POS)
            .unwrap()
            .crafter()
            .unwrap()
            .items();
        assert_eq!(
            [items[0].count(), items[1].count()],
            expected,
            "{component}"
        );
        assert!(world
            .get_block_entity(dim, hopper_pos)
            .unwrap()
            .hopper()
            .unwrap()
            .items()[0]
            .is_empty());
    }
}
