use super::*;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

fn stack(name: &str, resistance: Option<(&str, NbtSummaryValue)>) -> RuntimeItemStack {
    let mut raw = vec![
        NbtSummaryEntry {
            name: "id".into(),
            value: NbtSummaryValue::String(name.into()),
        },
        NbtSummaryEntry {
            name: "count".into(),
            value: NbtSummaryValue::Int(1),
        },
    ];
    if let Some((component, value)) = resistance {
        raw.push(NbtSummaryEntry {
            name: "components".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: component.into(),
                value,
            }]),
        });
    }
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(raw),
        &crate::tests::LenientTags(crate::world::registry_runtime::item_stack_resolver_for_test()),
    )
    .unwrap()
}

fn resistant_tag(tag: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "types".into(),
        value: NbtSummaryValue::string(format!("#{tag}")),
    }])
}

fn item(store: &mut EntityStore, stack: RuntimeItemStack) -> i32 {
    crate::entity::spawn_item_entity(
        store,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        stack,
    )
    .unwrap()
}

fn config_with_damage_tag(tag: &str, members: &[&str]) -> crate::world::ServerWorldConfig {
    let mut config = crate::tests::datapack_config();
    let startup = std::sync::Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .tags
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:damage_type")
        .unwrap();
    registry.tags.insert(
        tag.into(),
        members.iter().map(|member| (*member).into()).collect(),
    );
    config
}

#[test]
fn item_fire_live_prototype_removal_and_overridden_tag_reach_base_tick() {
    for (overridden, removed, immune) in [
        (false, false, true),
        (false, true, false),
        (true, false, false),
    ] {
        let config = if overridden {
            config_with_damage_tag("minecraft:is_fire", &[])
        } else {
            crate::tests::datapack_config()
        };
        with_config(config, |level| {
            let mut store = batch_store();
            let removal = removed.then_some((
                "!minecraft:damage_resistant",
                NbtSummaryValue::Compound(vec![]),
            ));
            let id = item(&mut store, stack("minecraft:netherite_sword", removal));
            let mut body = store.detach(id).unwrap();
            assert_eq!(body.fire_immune_in(level).unwrap(), immune);
            body.set_remaining_fire_ticks(40);
            assert_eq!(body.is_on_fire_in(level).unwrap(), !immune);
            let mut hurts = 0;
            assert!(crate::entity::base_tick_hurting_on_fire(
                &mut body,
                &mut store,
                level,
                |_, _, _| hurts += 1
            ));
            assert_eq!(hurts, usize::from(!immune));
            assert_eq!(body.remaining_fire_ticks(), if immune { 0 } else { 39 });
            store.attach(body);
        });
    }
}

#[test]
fn item_fire_uses_loaded_custom_resistance_tag() {
    for members in [vec!["minecraft:in_fire"], vec!["minecraft:lava"], vec![]] {
        with_config(
            config_with_damage_tag("test:item_fire", &members),
            |level| {
                let mut store = batch_store();
                let id = item(
                    &mut store,
                    stack(
                        "minecraft:stone",
                        Some((
                            "minecraft:damage_resistant",
                            resistant_tag("test:item_fire"),
                        )),
                    ),
                );
                assert_eq!(
                    store.get(id).unwrap().fire_immune_in(level).unwrap(),
                    members.contains(&"minecraft:in_fire")
                );
            },
        );
    }
}

#[test]
fn item_fire_immune_powder_contact_preserves_block_without_extinguish_sound() {
    with_config(crate::tests::datapack_config(), |level| {
        let mut store = batch_store();
        let id = item(&mut store, stack("minecraft:netherite_sword", None));
        store.get_mut(id).unwrap().set_remaining_fire_ticks(40);
        let pos = BlockPos { x: 0, y: 64, z: 0 };
        let snow = vanilla_block_state_id_by_name_and_properties(
            "minecraft:powder_snow",
            &Default::default(),
        )
        .unwrap();
        level.write_block(pos, snow).unwrap();
        store.drain_lifecycle_events();
        let mut body = store.detach(id).unwrap();
        let resolved = crate::entity::movement::move_entity_by_piston(
            &mut body,
            &mut store,
            level,
            Vec3d::default(),
            Direction::East,
        )
        .then_some(());
        store.attach(body);
        assert_eq!(resolved, Some(()));
        assert_eq!(level.block_state(pos), snow);
        assert!(!store.get(id).unwrap().is_on_fire_in(level).unwrap());
        assert!(!store
            .drain_lifecycle_events()
            .iter()
            .any(|event| matches!(event, crate::entity::EntityLifecycleEvent::Sound { .. })));
        assert_eq!(level.event_sink.drain_block_sounds().count(), 0);
    });
}
