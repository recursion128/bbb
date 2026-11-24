use super::*;
use bbb_client_world::WorldStore;
use bbb_protocol::{ComponentStyle, NbtSummaryEntry};

fn compound(entries: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn string(text: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(text.into())
}

fn shadow_text() -> NbtSummaryValue {
    compound(vec![
        ("text", string("A")),
        ("shadow_color", NbtSummaryValue::Int(0)),
        (
            "extra",
            NbtSummaryValue::List(vec![compound(vec![
                ("text", string("B")),
                ("shadow_color", NbtSummaryValue::Int(-65536)),
            ])]),
        ),
    ])
}

fn item(components: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    compound(vec![
        ("id", string("minecraft:stone")),
        ("count", NbtSummaryValue::Int(1)),
        ("components", compound(components)),
    ])
}

fn canonicalize(stack: ItemStackSummary, world: &WorldStore) -> ItemStackSummary {
    let nbt = world.persistent_item_stack_to_nbt(&stack).unwrap();
    world.persistent_item_stack_from_nbt(&nbt).unwrap()
}

fn old_runs() -> Vec<StyledTextRun> {
    vec![StyledTextRun {
        text: "AB".into(),
        style: ComponentStyle::default(),
    }]
}

fn import(stack: ItemStackSummary, world: &WorldStore) -> ItemStackSummary {
    import_value(serde_json::to_value(stack).unwrap(), world)
}

fn import_value(stack: serde_json::Value, world: &WorldStore) -> ItemStackSummary {
    let neighbor = ItemStackSummary {
        item_id: Some(1),
        count: 2,
        ..ItemStackSummary::empty()
    };
    let raw = serde_json::to_vec(&serde_json::json!({ "hotbars": [[stack, neighbor]] })).unwrap();
    let file = super::super::decode_legacy_json(&raw, world).unwrap();
    assert_eq!(
        world
            .persistent_item_stack_from_nbt(&file.hotbars[0][1])
            .unwrap(),
        neighbor
    );
    world
        .persistent_item_stack_from_nbt(&file.hotbars[0][0])
        .unwrap()
}

#[test]
fn imports_old_merged_runs_and_preserves_exact_shadow_nbt() {
    let world = WorldStore::new();
    let canonical = world
        .persistent_item_stack_from_nbt(&item(vec![
            ("minecraft:custom_name", shadow_text()),
            ("minecraft:item_name", shadow_text()),
            ("minecraft:lore", NbtSummaryValue::List(vec![shadow_text()])),
        ]))
        .unwrap();
    assert_eq!(
        canonical
            .component_patch
            .custom_name_styled
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    let mut exact = canonical.clone();
    exact.component_patch.custom_name_styled = Some(old_runs());
    upgrade(&mut exact).unwrap();
    assert_eq!(exact, canonical);
    let canonical = canonicalize(canonical, &world);
    let mut old = canonical.clone();
    old.component_patch.custom_name_styled = Some(old_runs());
    old.component_patch.item_name_styled = Some(old_runs());
    old.component_patch.lore_styled = vec![old_runs()];
    assert!(world.persistent_item_stack_to_nbt(&old).is_err());
    assert_eq!(import(old, &world), canonical);
    assert_eq!(import(canonical.clone(), &world), canonical);
}

#[test]
fn rejects_forged_projection_shadow_plain_and_missing_raw_per_cell() {
    let world = WorldStore::new();
    let canonical = world
        .persistent_item_stack_from_nbt(&item(vec![("minecraft:custom_name", shadow_text())]))
        .unwrap();
    for mutation in 0..5 {
        let mut bad = canonical.clone();
        bad.component_patch.custom_name_styled = Some(old_runs());
        match mutation {
            0 => bad.component_patch.custom_name_styled.as_mut().unwrap()[0].text = "forged".into(),
            1 => {
                bad.component_patch.custom_name_styled.as_mut().unwrap()[0]
                    .style
                    .shadow_color = Some(123)
            }
            2 => bad.component_patch.custom_name = Some("forged".into()),
            3 => bad.component_patch.custom_name_nbt = None,
            4 => {
                bad.component_patch.custom_name_styled.as_mut().unwrap()[0]
                    .style
                    .bold = Some(true)
            }
            _ => unreachable!(),
        }
        assert_eq!(
            import(bad, &world),
            ItemStackSummary::empty(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn imports_nested_hover_caches_but_rejects_forged_hover_source() {
    let world = WorldStore::new();
    for entity in [false, true] {
        let hover = if entity {
            compound(vec![
                ("action", string("show_entity")),
                ("id", string("minecraft:pig")),
                ("uuid", string("00000000-0000-0000-0000-000000000001")),
                ("name", shadow_text()),
            ])
        } else {
            compound(vec![
                ("action", string("show_text")),
                ("value", shadow_text()),
            ])
        };
        let raw = compound(vec![("text", string("Hover")), ("hover_event", hover)]);
        let canonical = world
            .persistent_item_stack_from_nbt(&item(vec![("minecraft:custom_name", raw)]))
            .unwrap();
        let canonical = canonicalize(canonical, &world);
        let mut old = canonical.clone();
        let hover = &mut old.component_patch.custom_name_styled.as_mut().unwrap()[0]
            .style
            .hover_event;
        match hover.as_mut().unwrap() {
            ComponentHoverEvent::ShowText { value, .. } => *value = old_runs(),
            ComponentHoverEvent::ShowEntity { name, .. } => *name = Some(old_runs()),
            _ => unreachable!(),
        }
        assert_eq!(import(old.clone(), &world), canonical);
        let hover = &mut old.component_patch.custom_name_styled.as_mut().unwrap()[0]
            .style
            .hover_event;
        match hover.as_mut().unwrap() {
            ComponentHoverEvent::ShowText { component, .. } => *component = Some(string("forged")),
            ComponentHoverEvent::ShowEntity { name_component, .. } => {
                *name_component = Some(string("forged"))
            }
            _ => unreachable!(),
        }
        assert_eq!(import(old, &world), ItemStackSummary::empty());
    }
}

#[test]
fn imports_nested_items_from_canonical_container_slots() {
    let world = WorldStore::new();
    let nested = item(vec![("minecraft:custom_name", shadow_text())]);
    for key in [
        "minecraft:use_remainder",
        "minecraft:charged_projectiles",
        "minecraft:bundle_contents",
        "minecraft:container",
    ] {
        let value = match key {
            "minecraft:use_remainder" => nested.clone(),
            "minecraft:container" => NbtSummaryValue::List(vec![compound(vec![
                ("slot", NbtSummaryValue::Int(2)),
                ("item", nested.clone()),
            ])]),
            _ => NbtSummaryValue::List(vec![nested.clone()]),
        };
        let canonical = world
            .persistent_item_stack_from_nbt(&item(vec![(key, value)]))
            .unwrap();
        let canonical = canonicalize(canonical, &world);
        let mut old = canonical.clone();
        let patch = &mut *old.component_patch;
        for child in patch
            .use_remainder
            .iter_mut()
            .map(|item| item.as_mut())
            .chain(&mut patch.charged_projectiles_items)
            .chain(&mut patch.bundle_contents_items)
            .chain(patch.container.slots.iter_mut().map(|slot| &mut slot.item))
        {
            child.component_patch.custom_name_styled = Some(old_runs());
        }
        assert_eq!(import(old.clone(), &world), canonical, "{key}");
        if key == "minecraft:container" {
            let mut mixed = serde_json::to_value(&old).unwrap();
            mixed["component_patch"]["container_items"][0]["component_patch"]
                ["custom_name_styled"] = serde_json::to_value(
                &canonical.component_patch.container.slots[0]
                    .item
                    .component_patch
                    .custom_name_styled,
            )
            .unwrap();
            assert_eq!(import_value(mixed, &world), canonical);
            let mut conflicting_count = serde_json::to_value(&old).unwrap();
            conflicting_count["component_patch"]["container_items"][0]["count"] = 2.into();
            assert_eq!(import_value(conflicting_count, &world), canonical);
        }
    }
}

#[test]
fn imports_direct_description_holders_from_canonical_instrument() {
    let world = WorldStore::new();
    let material = compound(vec![
        ("asset_name", string("opal")),
        ("description", shadow_text()),
    ]);
    let pattern = compound(vec![
        ("asset_id", string("example:spiral")),
        ("description", shadow_text()),
        ("decal", NbtSummaryValue::Byte(1)),
    ]);
    let instrument = compound(vec![
        (
            "sound_event",
            compound(vec![("sound_id", string("example:signal"))]),
        ),
        ("use_duration", NbtSummaryValue::Int(7)),
        ("range", NbtSummaryValue::Int(256)),
        ("description", shadow_text()),
    ]);
    let canonical = world
        .persistent_item_stack_from_nbt(&item(vec![
            (
                "minecraft:trim",
                compound(vec![("material", material.clone()), ("pattern", pattern)]),
            ),
            ("minecraft:provides_trim_material", material),
            ("minecraft:instrument", instrument),
        ]))
        .unwrap();
    let canonical = canonicalize(canonical, &world);
    let mut old = canonical.clone();
    let patch = &mut *old.component_patch;
    for material in [
        &mut patch.armor_trim_material_direct,
        &mut patch.provides_trim_material_direct,
    ]
    .into_iter()
    .flatten()
    {
        material.description_styled = Some(old_runs().into_boxed_slice());
    }
    patch
        .armor_trim_pattern_direct
        .as_mut()
        .unwrap()
        .description_styled = Some(old_runs().into_boxed_slice());
    patch.instrument.direct.as_mut().unwrap().description_styled =
        Some(old_runs().into_boxed_slice());
    assert_eq!(import(old.clone(), &world), canonical);
    let mut projected = serde_json::to_value(&old).unwrap();
    let mut stale = old_runs();
    stale[0].text = "ignored projection".into();
    projected["component_patch"]["instrument_description_styled"] =
        serde_json::to_value(stale).unwrap();
    assert_eq!(import_value(projected, &world), canonical);
    let mut bad = old;
    bad.component_patch
        .instrument
        .direct
        .as_mut()
        .unwrap()
        .description_styled
        .as_mut()
        .unwrap()[0]
        .text = "forged".into();
    assert_eq!(import(bad, &world), ItemStackSummary::empty());
}

#[test]
fn imports_adventure_predicate_exact_components() {
    let world = WorldStore::new();
    for key in ["minecraft:can_break", "minecraft:can_place_on"] {
        let predicate = compound(vec![(
            "components",
            compound(vec![("minecraft:custom_name", shadow_text())]),
        )]);
        let canonical = world
            .persistent_item_stack_from_nbt(&item(vec![(key, predicate)]))
            .unwrap();
        let canonical = canonicalize(canonical, &world);
        let mut old = canonical.clone();
        let patch = &mut *old.component_patch;
        for predicate in [
            &mut patch.can_break_persistent,
            &mut patch.can_place_on_persistent,
        ]
        .into_iter()
        .flatten()
        {
            predicate.predicates[0]
                .exact_components
                .as_mut()
                .unwrap()
                .custom_name_styled = Some(old_runs());
        }
        assert_eq!(import(old, &world), canonical, "{key}");
    }
}

#[test]
fn imports_attribute_display_and_missing_shadow_field() {
    let world = WorldStore::new();
    let canonical = world
        .persistent_item_stack_from_nbt(&item(vec![(
            "minecraft:attribute_modifiers",
            NbtSummaryValue::List(vec![compound(vec![
                ("type", string("minecraft:armor")),
                ("id", string("example:bonus")),
                ("amount", NbtSummaryValue::Int(1)),
                ("operation", string("add_value")),
                (
                    "display",
                    compound(vec![("type", string("override")), ("value", shadow_text())]),
                ),
            ])]),
        )]))
        .unwrap();
    let canonical = canonicalize(canonical, &world);
    let mut old = canonical.clone();
    old.component_patch.attribute_modifiers[0].display_text_styled = Some(old_runs());
    let mut json = serde_json::json!({"hotbars": [[old]]});
    json["hotbars"][0][0]["component_patch"]["attribute_modifiers"][0]["display_text_styled"][0]
        ["style"]
        .as_object_mut()
        .unwrap()
        .remove("shadow_color");
    let file =
        super::super::decode_legacy_json(&serde_json::to_vec(&json).unwrap(), &world).unwrap();
    assert_eq!(
        world
            .persistent_item_stack_from_nbt(&file.hotbars[0][0])
            .unwrap(),
        canonical
    );
}

#[test]
fn migration_enforces_existing_item_nesting_limit() {
    use bbb_protocol::packets::ItemStackTemplateSummary;
    let mut stack = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        ..ItemStackSummary::empty()
    };
    let mut patch = &mut stack.component_patch;
    for _ in 0..=MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        patch.use_remainder = Some(Box::new(ItemStackTemplateSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }));
        patch = &mut patch.use_remainder.as_mut().unwrap().component_patch;
    }
    assert!(upgrade(&mut stack).is_err());
}

fn with_predicate(exact: DataComponentPatchSummary) -> DataComponentPatchSummary {
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        packets::{AdventureModePredicateDataSummary, BlockPredicateSummary},
    };
    DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
        can_place_on_persistent: Some(AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                exact_components: Some(Box::new(exact)),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}

fn with_remainder(patch: DataComponentPatchSummary) -> DataComponentPatchSummary {
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        packets::ItemStackTemplateSummary,
    };
    DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID],
        use_remainder: Some(Box::new(ItemStackTemplateSummary {
            item_id: 1,
            count: 1,
            component_patch: patch.into(),
        })),
        ..Default::default()
    }
}

#[test]
fn template_and_predicate_depth_keep_their_independent_codec_boundaries() {
    use bbb_protocol::packets::data_components::validate_data_component_patch_summary;
    let world = WorldStore::new();
    let name = world
        .persistent_item_stack_from_nbt(&item(vec![("minecraft:custom_name", shadow_text())]))
        .unwrap()
        .component_patch;
    for predicates_outside in [false, true] {
        let mut patch = if predicates_outside {
            with_remainder(name.clone().into_inner())
        } else {
            with_predicate(name.clone().into_inner())
        };
        for _ in 0..32 {
            patch = if predicates_outside {
                with_predicate(patch)
            } else {
                with_remainder(patch)
            };
        }
        validate_data_component_patch_summary(&patch).unwrap();
        let mut old = ItemStackSummary {
            item_id: Some(1),
            count: 1,
            component_patch: patch.clone().into(),
        };
        let mut leaf: &mut DataComponentPatchSummary = &mut old.component_patch;
        loop {
            if leaf.use_remainder.is_some() {
                leaf = &mut leaf.use_remainder.as_mut().unwrap().component_patch;
            } else if leaf.can_place_on_persistent.is_some() {
                leaf = leaf.can_place_on_persistent.as_mut().unwrap().predicates[0]
                    .exact_components
                    .as_mut()
                    .unwrap();
            } else {
                leaf.custom_name_styled = Some(old_runs());
                break;
            }
        }
        upgrade(&mut old).unwrap();
        assert_eq!(old.component_patch, patch);
        validate_data_component_patch_summary(&old.component_patch).unwrap();
        if predicates_outside {
            old.component_patch = with_predicate(old.component_patch.into_inner()).into();
            upgrade(&mut old).unwrap();
            assert!(validate_data_component_patch_summary(&old.component_patch).is_err());
        }
    }
}

#[test]
fn cached_direct_song_and_painting_upgrade_preserves_persistence_rejection() {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        },
        packets::{JukeboxSongSummary, PaintingVariantSummary, SoundEventSummary},
    };
    let world = WorldStore::new();
    let text = RichTextComponent::try_from_component_nbt(shadow_text()).unwrap();
    for painting in [false, true] {
        let mut old = ItemStackSummary {
            item_id: Some(1),
            count: 1,
            ..ItemStackSummary::empty()
        };
        let patch = &mut old.component_patch;
        if painting {
            patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID];
            patch.painting_variant_direct = Some(PaintingVariantSummary {
                width: 1,
                height: 1,
                asset_id: "example:art".into(),
                title: Some(text.text().into()),
                title_styled: Some(old_runs()),
                title_nbt: Some(Box::new(shadow_text())),
                author: Some(text.text().into()),
                author_styled: Some(old_runs()),
                author_nbt: Some(Box::new(shadow_text())),
            });
        } else {
            patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID];
            patch.jukebox_direct_song = Some(Box::new(JukeboxSongSummary {
                sound_event: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:song".into()),
                    fixed_range_bits: None,
                },
                description: text.text().into(),
                description_styled: Some(old_runs().into_boxed_slice()),
                description_nbt: Some(Box::new(shadow_text())),
                length_in_seconds_bits: 7.0f32.to_bits(),
                comparator_output: 1,
            }));
        }
        let mut upgraded = old.clone();
        upgrade(&mut upgraded).unwrap();
        if painting {
            let value = upgraded
                .component_patch
                .painting_variant_direct
                .as_ref()
                .unwrap();
            assert_eq!(value.title_styled.as_deref(), Some(text.styled()));
            assert_eq!(value.author_styled.as_deref(), Some(text.styled()));
            assert_eq!(value.title_nbt.as_deref(), Some(text.component()));
            assert_eq!(value.author_nbt.as_deref(), Some(text.component()));
        } else {
            let value = upgraded
                .component_patch
                .jukebox_direct_song
                .as_ref()
                .unwrap();
            assert_eq!(value.description_styled.as_deref(), Some(text.styled()));
            assert_eq!(value.description_nbt.as_deref(), Some(text.component()));
        }
        assert_eq!(import(old, &world), ItemStackSummary::empty());
    }
}
