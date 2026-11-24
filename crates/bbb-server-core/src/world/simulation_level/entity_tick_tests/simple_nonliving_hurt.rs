use super::*;
use crate::entity::{EntityData, EntityLifecycleEvent};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::*;

const FEET: Vec3d = Vec3d {
    x: 8.5,
    y: 65.0,
    z: 8.5,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Arrow,
    Snowball,
    Spit,
    FallingBlock,
    Tnt,
}

fn body(store: &mut EntityStore, kind: Kind) -> i32 {
    use crate::entity::*;
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &bbb_protocol::NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "id".into(),
                value: bbb_protocol::NbtSummaryValue::String("minecraft:stone".into()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "count".into(),
                value: bbb_protocol::NbtSummaryValue::Int(1),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap();
    let (ty, data) = match kind {
        Kind::Arrow => (
            VANILLA_ENTITY_TYPE_ARROW_ID,
            EntityData::Arrow(ArrowData::new(stack)),
        ),
        Kind::Snowball => (
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            // Already shot, so the first tick raises no `PROJECTILE_SHOOT`.
            EntityData::Snowball(SnowballData::from_saved(
                stack,
                crate::entity::hurting_projectile::ProjectileState::from_saved(None, true, true),
            )),
        ),
        Kind::Spit => (
            VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
            EntityData::LlamaSpit(LlamaSpitData::default()),
        ),
        Kind::FallingBlock => (
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            EntityData::FallingBlock(FallingBlockData::new(
                state("minecraft:sand"),
                BlockPos { x: 8, y: 65, z: 8 },
            )),
        ),
        Kind::Tnt => (VANILLA_ENTITY_TYPE_TNT_ID, EntityData::Tnt(TntData::new())),
    };
    store.spawn(ty, FEET, data)
}

fn state(name: &str) -> i32 {
    let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap()
}

fn block(level: &mut LevelCtx<'_, '_>, name: &str) {
    level
        .write_block(BlockPos { x: 8, y: 65, z: 8 }, state(name))
        .unwrap();
}

fn contact(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) -> bool {
    let mut body = store.detach(id).unwrap();
    let ok = crate::entity::movement::move_entity_by_piston(
        &mut body,
        store,
        level,
        Vec3d {
            x: 0.1,
            y: 0.0,
            z: 0.0,
        },
        Direction::East,
    );
    store.attach(body);
    ok
}

#[test]
fn simple_nonliving_base_fire_marks_without_rejecting_tick_and_preserves_no_base_types() {
    with_config(crate::tests::datapack_config(), |level| {
        for kind in [
            Kind::Arrow,
            Kind::Snowball,
            Kind::Spit,
            Kind::FallingBlock,
            Kind::Tnt,
        ] {
            for invulnerable in [false, true] {
                let mut store = batch_store();
                let id = body(&mut store, kind);
                store.get_mut(id).unwrap().set_remaining_fire_ticks(20);
                store.get_mut(id).unwrap().set_invulnerable(invulnerable);
                crate::entity::tick_one_entity(&mut store, level, id);
                let entity = store.get(id).unwrap();
                let has_base = !matches!(kind, Kind::FallingBlock | Kind::Tnt);
                assert_eq!(entity.hurt_marked(), has_base && !invulnerable, "{kind:?}");
                assert_eq!(
                    entity.remaining_fire_ticks(),
                    match kind {
                        Kind::Tnt => 0,
                        Kind::FallingBlock => 20,
                        _ => 19,
                    },
                    "{kind:?}"
                );
                assert_eq!(entity.is_first_tick(), !has_base, "{kind:?}");
                if has_base {
                    assert_eq!(entity.tick_count(), 1);
                }
                // A projectile's first tick is its `PROJECTILE_SHOOT` (`Projectile.java:99-103`).
                assert!(level
                    .event_sink
                    .drain_game_events()
                    .iter()
                    .all(|event| matches!(kind, Kind::Arrow | Kind::Spit)
                        && event.game_event == VanillaGameEvent::PROJECTILE_SHOOT));
            }
        }
    });
}

#[test]
fn simple_nonliving_ordinary_lava_contact_commits_mark_or_refusal_without_burn_sound() {
    for kind in [Kind::Snowball, Kind::FallingBlock, Kind::Tnt] {
        with_config(crate::tests::datapack_config(), |level| {
            block(level, "minecraft:lava");
            let mut store = batch_store();
            let id = body(&mut store, kind);
            store.drain_lifecycle_events();
            crate::entity::tick_one_entity(&mut store, level, id);
            let entity = store.get(id).unwrap();
            assert_eq!(entity.hurt_marked(), !matches!(kind, Kind::Tnt), "{kind:?}");
            assert_eq!(
                entity.remaining_fire_ticks() > 0,
                !matches!(kind, Kind::Tnt),
                "{kind:?}"
            );
            if let EntityData::Tnt(data) = entity.data() {
                assert_eq!(data.fuse(), 79);
            }
            if let EntityData::FallingBlock(data) = entity.data() {
                assert_eq!(data.time(), 1);
            }
            assert!(!store
                .drain_lifecycle_events()
                .iter()
                .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
            assert!(level.event_sink.drain_game_events().is_empty());
        });
    }
}

#[test]
fn simple_nonliving_piston_damage_and_lava_commit_legal_false_with_immunity() {
    let kinds = [
        Kind::Arrow,
        Kind::Snowball,
        Kind::Spit,
        Kind::FallingBlock,
        Kind::Tnt,
    ];
    let cases = kinds
        .map(|kind| ("minecraft:cactus", kind, false))
        .into_iter()
        .chain([
            ("minecraft:lava", Kind::Arrow, false),
            ("minecraft:cactus", Kind::Arrow, true),
        ]);
    with_config(crate::tests::datapack_config(), |level| {
        for (name, kind, invulnerable) in cases {
            block(level, name);
            let mut store = batch_store();
            let id = body(&mut store, kind);
            store.get_mut(id).unwrap().set_invulnerable(invulnerable);
            store.drain_lifecycle_events();
            let result = contact(&mut store, level, id).then_some(());
            assert_eq!(result, Some(()), "{kind:?} {name}");
            assert_eq!(
                store.get(id).unwrap().hurt_marked(),
                !matches!(kind, Kind::Tnt) && !invulnerable,
                "{kind:?} {name}"
            );
            assert!(!store
                .drain_lifecycle_events()
                .iter()
                .any(|event| matches!(event, EntityLifecycleEvent::Sound { .. })));
            assert!(level.event_sink.drain_game_events().is_empty());
        }
    });
}
