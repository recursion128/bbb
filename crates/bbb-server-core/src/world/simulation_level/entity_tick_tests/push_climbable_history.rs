use super::*;
use crate::entity::{living, EntityTickOutcome};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Vec3Ops;
use std::sync::Arc;

const OLD: BlockPos = BlockPos { x: 6, y: 68, z: 8 };
const FIRST: BlockPos = BlockPos { x: 8, y: 65, z: 9 };
const SOURCE: Vec3d = Vec3d {
    x: 8.9,
    y: 65.25,
    z: 8.9,
};

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn write(level: &mut LevelCtx<'_, '_>, pos: BlockPos, value: i32) {
    assert!(level.write_block(pos, value).is_some());
    assert_eq!(level.block_state(pos), value);
}

fn arena(level: &mut LevelCtx<'_, '_>) {
    level.chunks.try_ensure_served_full_column(0, 0).unwrap();
    for x in 5..=11 {
        for z in 6..=11 {
            for y in 63..=72 {
                write(
                    level,
                    BlockPos { x, y, z },
                    state(if y == 63 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    }),
                );
            }
        }
    }
    write(level, OLD, state("minecraft:ladder"));
}

fn no_ai_no_gravity(store: &mut EntityStore, id: i32) {
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    entries.retain(|entry| entry.name != "NoAI" && entry.name != "NoGravity");
    entries.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(1),
        },
        NbtSummaryEntry {
            name: "NoGravity".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]);
    store.load_command_data(id, &entries).unwrap();
}

fn neighbor(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, kind: i32, pos: Vec3d) -> i32 {
    let id = store
        .spawn_general_living(
            kind,
            Vec3d {
                x: 6.5,
                y: 68.25,
                z: 8.5,
            },
        )
        .unwrap();
    no_ai_no_gravity(store, id);
    let mut body = store.detach(id).unwrap();
    living::travel_in_air(&mut body, store, level, Vec3d::default()).unwrap();
    assert_eq!(body.living().unwrap().last_climbable_pos(), Some(OLD));
    body.set_pos(pos);
    body.set_delta_movement(Vec3d::default());
    body.set_needs_sync(false);
    store.attach(body);
    id
}

fn root_fixture(level: &mut LevelCtx<'_, '_>, last: bool) -> (EntityStore, i32, i32, Option<i32>) {
    let mut store = batch_store();
    let source = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, SOURCE)
        .unwrap();
    no_ai_no_gravity(&mut store, source);
    let first = neighbor(
        &mut store,
        level,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 8.8,
            z: 9.1,
            ..SOURCE
        },
    );
    let last = last.then(|| {
        neighbor(
            &mut store,
            level,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 9.1,
                z: 8.8,
                ..SOURCE
            },
        )
    });
    store.drain_lifecycle_events();
    (store, source, first, last)
}

fn history(store: &EntityStore, id: i32) -> Option<BlockPos> {
    store
        .get(id)
        .unwrap()
        .living()
        .unwrap()
        .last_climbable_pos()
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        EntityTickOutcome::Complete
    ));
}

fn install_climbable_tag(level: &mut LevelCtx<'_, '_>, members: &[&str]) {
    let config = crate::tests::datapack_config();
    let registries = &config.datapacks.as_ref().unwrap().tags.registries;
    let mut replaced = false;
    let mut tags = Vec::new();
    for registry in registries {
        for (tag, existing) in &registry.tags {
            let values =
                if registry.registry_id == "minecraft:block" && tag == "minecraft:climbable" {
                    replaced = true;
                    members.iter().map(|member| (*member).to_string()).collect()
                } else {
                    existing.clone()
                };
            tags.push((registry.registry_id.clone(), tag.clone(), values));
        }
    }
    assert!(replaced);
    level
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

#[test]
fn push_climbable_history_world_live_tag_change_rejects_then_pushes_and_keeps_negative_history() {
    with_config(crate::tests::datapack_config(), |level| {
        arena(level);
        write(level, FIRST, state("minecraft:structure_void"));
        let (mut store, source, candidate, _) = root_fixture(level, false);
        install_climbable_tag(level, &["minecraft:ladder", "minecraft:structure_void"]);
        tick(&mut store, level, source);
        assert_eq!(store.get(source).unwrap().tick_count(), 1);
        assert_eq!(history(&store, candidate), Some(FIRST));
        assert_eq!(
            store.get(candidate).unwrap().delta_movement(),
            Vec3d::default()
        );

        let negative = BlockPos { x: 9, y: 65, z: 9 };
        write(level, negative, state("minecraft:structure_void"));
        store.get_mut(candidate).unwrap().set_pos(Vec3d {
            x: 9.1,
            z: 9.1,
            ..SOURCE
        });
        install_climbable_tag(level, &["minecraft:ladder"]);
        tick(&mut store, level, source);
        assert_eq!(store.get(source).unwrap().tick_count(), 2);
        assert_eq!(history(&store, candidate), Some(FIRST));
        assert_eq!(store.get(candidate).unwrap().tick_count(), 0);
        assert!(store.get(candidate).unwrap().delta_movement().length_sqr() > 0.0);
        assert!(store.get(candidate).unwrap().needs_sync());
        assert!(store.get(source).unwrap().delta_movement().length_sqr() > 0.0);

        for id in [source, candidate] {
            store
                .get_mut(id)
                .unwrap()
                .set_delta_movement(Vec3d::default());
            store.get_mut(id).unwrap().set_needs_sync(false);
        }
        install_climbable_tag(level, &["minecraft:ladder", "minecraft:structure_void"]);
        tick(&mut store, level, source);
        assert_eq!(store.get(source).unwrap().tick_count(), 3);
        assert_eq!(history(&store, candidate), Some(negative));
        assert_eq!(
            store.get(candidate).unwrap().delta_movement(),
            Vec3d::default()
        );
        assert!(!store.get(candidate).unwrap().needs_sync());
    });
}
