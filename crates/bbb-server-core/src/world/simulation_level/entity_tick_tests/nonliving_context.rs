use super::*;
use crate::entity::{EntityData, ExperienceOrbData, ItemEntityData, TntData};
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_TNT_ID,
};
use bbb_protocol::NbtSummaryValue;
use std::sync::Arc;

const PLATFORM: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const WALKABLE: &str = "minecraft:powder_snow_walkable_mobs";

#[derive(Clone, Copy, Debug)]
enum Kind {
    Item,
    Orb,
    Tnt,
}

const KINDS: [Kind; 3] = [Kind::Item, Kind::Orb, Kind::Tnt];

impl Kind {
    fn resource_id(self) -> &'static str {
        match self {
            Self::Item => "minecraft:item",
            Self::Orb => "minecraft:experience_orb",
            Self::Tnt => "minecraft:tnt",
        }
    }
}

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn install(level: &mut LevelCtx<'_, '_>, name: &str) {
    for x in 7..=9 {
        for z in 7..=9 {
            for y in 62..=70 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level
                .write_block(BlockPos { x, y: 63, z }, state("minecraft:stone"))
                .unwrap();
        }
    }
    let mut platform = state(name);
    if name == "minecraft:scaffolding" {
        platform = crate::fluid::with_property(platform, "distance", "0").unwrap();
        platform = crate::fluid::with_property(platform, "bottom", "false").unwrap();
    }
    level.write_block(PLATFORM, platform).unwrap();
}

fn body(store: &mut EntityStore, kind: Kind, y: f64, delta_y: f64, shift: bool) -> i32 {
    let (type_id, data) = match kind {
        Kind::Item => (
            VANILLA_ENTITY_TYPE_ITEM_ID,
            EntityData::Item(
                ItemEntityData::new(
                    crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
                    0.0,
                )
                .with_saved_fields(5, 41, 17, None, None),
            ),
        ),
        Kind::Orb => (
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            EntityData::ExperienceOrb(ExperienceOrbData::from_saved(5, 73, 7, 1)),
        ),
        Kind::Tnt => (
            VANILLA_ENTITY_TYPE_TNT_ID,
            EntityData::Tnt(TntData::with_saved_fields(
                50,
                state("minecraft:tnt"),
                None,
                4.0,
            )),
        ),
    };
    let id = store.spawn(type_id, Vec3d { x: 8.5, y, z: 8.5 }, data);
    let body = store.get_mut(id).unwrap();
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: delta_y,
        z: 0.0,
    });
    body.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
    assert!(body.living().is_none());
    id
}

fn distance(store: &mut EntityStore, id: i32, value: f64) {
    let NbtSummaryValue::Compound(mut record) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    record
        .iter_mut()
        .find(|entry| entry.name == "fall_distance")
        .unwrap()
        .value = NbtSummaryValue::Double(value.to_bits());
    store.load_command_data(id, &record).unwrap();
    assert_eq!(store.get(id).unwrap().fall_distance(), value);
}

fn tags(omit_entity_registry: bool, members: &[Kind]) -> Arc<RegistryTagSnapshot> {
    let config = crate::tests::datapack_config();
    let registries: Vec<_> = config
        .datapacks
        .as_ref()
        .unwrap()
        .tags
        .registries
        .iter()
        .filter(|registry| !omit_entity_registry || registry.registry_id != "minecraft:entity_type")
        .collect();
    let mut tags: Vec<_> = registries
        .iter()
        .flat_map(|registry| {
            registry
                .tags
                .iter()
                .filter(|(tag, _)| {
                    registry.registry_id != "minecraft:entity_type" || tag.as_str() != WALKABLE
                })
                .map(|(tag, members)| (registry.registry_id.clone(), tag.clone(), members.clone()))
        })
        .collect();
    if !omit_entity_registry {
        tags.push((
            "minecraft:entity_type".into(),
            WALKABLE.into(),
            members
                .iter()
                .map(|kind| kind.resource_id().to_owned())
                .collect(),
        ));
    }
    Arc::new(RegistryTagSnapshot::new(tags))
}

fn membership(level: &LevelCtx<'_, '_>, kind: Kind) -> bool {
    LevelCtx::registry_tag_membership(level, "minecraft:entity_type", WALKABLE, kind.resource_id())
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

fn clocks(body: &Entity, ticks: i32) {
    assert_eq!(body.tick_count(), ticks);
    match body.data() {
        EntityData::Item(item) => {
            assert_eq!(item.age(), 41 + ticks);
            assert_eq!(item.pickup_delay(), 17 - ticks);
        }
        EntityData::ExperienceOrb(orb) => assert_eq!(orb.age(), 73 + ticks),
        EntityData::Tnt(tnt) => assert_eq!(tnt.fuse(), 50 - ticks),
        _ => unreachable!(),
    }
}

fn height(body: &Entity, expected: f64) {
    assert!(
        (body.position().y - expected).abs() < 1.0e-12,
        "{}: {:?}, expected Y={expected}",
        body.entity_type_id(),
        body.position()
    );
    assert_eq!(body.position().x, 8.5);
    assert_eq!(body.position().z, 8.5);
}

#[test]
fn nonliving_context_actual_stone_tick_controls_validate_all_three_clocks() {
    for kind in KINDS {
        with_config(crate::tests::datapack_config(), |level| {
            install(level, "minecraft:stone");
            let mut store = batch_store();
            let id = body(&mut store, kind, 66.5, -1.0, false);
            tick(&mut store, level, id);
            let after = store.get(id).unwrap();
            height(after, 66.0);
            assert!(after.on_ground());
            clocks(after, 1);
        });
    }
}

#[test]
fn nonliving_context_actual_scaffolding_above_support_and_shift_pass_through() {
    // ScaffoldingBlock.java:137-146.
    for kind in KINDS {
        for shift in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                install(level, "minecraft:scaffolding");
                let mut store = batch_store();
                let id = body(&mut store, kind, 66.5, -1.0, shift);
                let falling_y = 65.5 - store.get(id).unwrap().gravity();
                tick(&mut store, level, id);
                let after = store.get(id).unwrap();
                height(after, if shift { falling_y } else { 66.0 });
                assert_eq!(after.on_ground(), !shift, "{kind:?}, shift={shift}");
                clocks(after, 1);
            });
        }
    }
}

#[test]
fn nonliving_context_actual_powder_snow_nonmember_is_empty_for_all_three_ticks() {
    // PowderSnowBlock.java:116-131; PowderSnowBlock.java:139-145.
    with_config(crate::tests::datapack_config(), |level| {
        for kind in KINDS {
            for shift in [false, true] {
                install(level, "minecraft:powder_snow");
                level.chunks.install_registry_tags(tags(false, &[]));
                assert!(!membership(level, kind));
                let mut store = batch_store();
                let id = body(&mut store, kind, 66.5, -1.0, shift);
                let falling_y = 65.5 - store.get(id).unwrap().gravity();
                tick(&mut store, level, id);
                let after = store.get(id).unwrap();
                height(after, falling_y);
                assert!(!after.on_ground());
                clocks(after, 1);
            }
        }
    });
}

#[test]
fn nonliving_context_actual_powder_snow_custom_live_tag_supports_each_body() {
    with_config(crate::tests::datapack_config(), |level| {
        for kind in KINDS {
            install(level, "minecraft:powder_snow");
            level.chunks.install_registry_tags(tags(false, &[kind]));
            assert!(membership(level, kind));
            let mut store = batch_store();
            let id = body(&mut store, kind, 66.5, -1.0, false);
            tick(&mut store, level, id);
            let after = store.get(id).unwrap();
            height(after, 66.0);
            assert!(after.on_ground());
            clocks(after, 1);
        }
    });
}

#[test]
fn nonliving_context_actual_prior_distance_uses_float_height_before_shift_and_nonmember() {
    for kind in KINDS {
        with_config(crate::tests::datapack_config(), |level| {
            install(level, "minecraft:powder_snow");
            level.chunks.install_registry_tags(tags(false, &[]));
            assert!(!membership(level, kind));
            let mut store = batch_store();
            let id = body(&mut store, kind, 66.5, -1.0, true);
            distance(&mut store, id, 2.5_f64.next_up());
            tick(&mut store, level, id);
            let after = store.get(id).unwrap();
            height(after, 65.0 + f64::from(0.9_f32));
            assert!(after.on_ground());
            clocks(after, 1);
        });
    }
}

#[test]
fn nonliving_context_actual_orb_empty_prefix_inside_powder_snow_applies_gravity() {
    with_config(crate::tests::datapack_config(), |level| {
        install(level, "minecraft:powder_snow");
        level.chunks.install_registry_tags(tags(false, &[]));
        let mut store = batch_store();
        let id = body(&mut store, Kind::Orb, 65.5, 0.0, false);
        tick(&mut store, level, id);
        let after = store.get(id).unwrap();
        assert!(
            after.position().y < 65.5,
            "Empty-context noCollision permits gravity: {:?}",
            after.position()
        );
        assert!(!after.on_ground());
        assert!(after.delta_movement().y < 0.0);
        clocks(after, 1);
    });
}
