use super::*;

#[test]
fn merchant_component_enchantable_roundtrip_checks_positive_codec_integer() {
    for value in [
        NbtSummaryValue::Int(15),
        NbtSummaryValue::Double(15.75f64.to_bits()),
        NbtSummaryValue::Int(i32::MAX),
    ] {
        let saved = saved_stack_with_components(
            "minecraft:enchanted_book",
            vec![entry(
                "minecraft:enchantable",
                NbtSummaryValue::Compound(vec![entry("value", value.clone())]),
            )],
        );
        let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
        assert_eq!(
            summary.component_patch.enchantable_value,
            codec_int_value(&value)
        );
        let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
        let decoded = raw.clone();
        assert_eq!(
            crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap(),
            summary
        );
    }
    for value in [
        NbtSummaryValue::Int(0),
        NbtSummaryValue::Int(-1),
        NbtSummaryValue::String("15".into()),
    ] {
        let saved = saved_stack_with_components(
            "minecraft:enchanted_book",
            vec![entry(
                "minecraft:enchantable",
                NbtSummaryValue::Compound(vec![entry("value", value)]),
            )],
        );
        assert!(crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err());
    }
    for value in [None, Some(0), Some(-1)] {
        let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID]);
        patch.enchantable_value = value;
        assert!(
            item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 1, patch)).is_err()
        );
    }
}

fn potion(value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components(
        "minecraft:potion",
        vec![entry("minecraft:potion_contents", value)],
    )
}

#[test]
fn merchant_component_potion_roundtrip_preserves_hidden_effects() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([((MOB_EFFECT_REGISTRY, "minecraft:speed"), 1)]),
    };
    let effect = NbtSummaryValue::Compound(vec![
        entry("id", NbtSummaryValue::String("speed".into())),
        entry("amplifier", NbtSummaryValue::Int(2)),
        entry("duration", NbtSummaryValue::Int(-1)),
        entry("ambient", NbtSummaryValue::Byte(1)),
        entry("show_particles", NbtSummaryValue::Byte(0)),
        entry("show_icon", NbtSummaryValue::Byte(1)),
        entry(
            "hidden_effect",
            NbtSummaryValue::Compound(vec![
                entry("duration", NbtSummaryValue::Int(40)),
                entry(
                    "hidden_effect",
                    NbtSummaryValue::Compound(vec![entry("amplifier", NbtSummaryValue::Int(1))]),
                ),
            ]),
        ),
    ]);
    let saved = potion(NbtSummaryValue::Compound(vec![
        entry("potion", NbtSummaryValue::String("water".into())),
        entry("custom_color", NbtSummaryValue::Int(-123)),
        entry(
            "custom_name",
            NbtSummaryValue::String("merchant's brew".into()),
        ),
        entry("custom_effects", NbtSummaryValue::List(vec![effect])),
    ]));
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    let patch = &summary.component_patch;
    assert_eq!(patch.potion_contents.custom_color, Some(-123));
    assert_eq!(
        patch.potion_contents.custom_name,
        Some("merchant's brew".into())
    );
    assert_eq!(patch.potion_contents.custom_effects.len(), 1);
    let effect = &patch.potion_contents.custom_effects[0];
    assert_eq!((effect.amplifier, effect.duration), (2, -1));
    assert!(effect.ambient && effect.show_icon && !effect.show_particles);
    let hidden = effect.hidden_effect.as_ref().unwrap();
    assert_eq!((hidden.amplifier, hidden.duration), (0, 40));
    assert!(!hidden.ambient && hidden.show_particles && hidden.show_icon);
    assert_eq!(
        patch.potion_contents.custom_effects[0]
            .hidden_effect
            .as_ref()
            .unwrap()
            .hidden_effect
            .as_ref()
            .unwrap()
            .amplifier,
        1
    );
    assert!(patch.codec_backed_nbt.is_empty());
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let potion = bbb_protocol::nbt_summary::value_last_field(
        projected_components(&raw).unwrap(),
        "minecraft:potion_contents",
    )
    .unwrap();
    let NbtSummaryValue::Compound(fields) = potion else {
        panic!("saved potion");
    };
    let NbtSummaryValue::List(effects) = &fields
        .iter()
        .find(|field| field.name == "custom_effects")
        .unwrap()
        .value
    else {
        panic!("saved effects");
    };
    let NbtSummaryValue::Compound(effect) = &effects[0] else {
        panic!("saved effect");
    };
    assert_eq!(
        effect
            .iter()
            .find(|field| field.name == "amplifier")
            .unwrap()
            .value,
        NbtSummaryValue::Byte(2)
    );
    let loaded = crate::read_item_stack(&raw, &resolver).unwrap();
    assert_eq!(loaded, summary);
}

#[test]
fn merchant_component_potion_projection_narrows_effect_bytes() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([((MOB_EFFECT_REGISTRY, "minecraft:speed"), 1)]),
    };
    let saved = potion(NbtSummaryValue::Compound(vec![entry(
        "custom_effects",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::String("speed".into())),
            entry("amplifier", NbtSummaryValue::Int(511)),
            entry("ambient", NbtSummaryValue::Int(256)),
            entry("show_particles", NbtSummaryValue::Int(256)),
            entry("show_icon", NbtSummaryValue::Int(257)),
        ])]),
    )]));
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    let effect = &summary.component_patch.potion_contents.custom_effects[0];
    assert_eq!(effect.amplifier, 255);
    assert!(!effect.ambient && !effect.show_particles && effect.show_icon);
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let decoded = raw.clone();
    let loaded = crate::read_item_stack(&decoded, &resolver).unwrap();
    assert_eq!(
        loaded.component_patch.potion_contents.custom_effects[0].amplifier,
        255
    );
}

#[test]
fn merchant_component_potion_short_form_defaults_and_invalid_holders() {
    let saved = potion(NbtSummaryValue::String("water".into()));
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert!(summary.component_patch.potion_contents.potion_id.is_some());
    assert_eq!(
        summary.component_patch.potion_contents.custom_effects.len(),
        0
    );
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        crate::read_item_stack(&raw, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
    for value in [
        NbtSummaryValue::String("missing".into()),
        NbtSummaryValue::Int(3),
        NbtSummaryValue::Compound(vec![entry(
            "custom_effects",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                "id",
                NbtSummaryValue::String("missing".into()),
            )])]),
        )]),
    ] {
        assert!(crate::read_item_stack(&potion(value), &crate::UnavailableHolderResolver).is_err());
    }
    let mut bad = summary;
    bad.component_patch.potion_contents.potion_id = Some(i32::MAX);
    assert!(item_stack_summary_to_persistent_nbt(&bad).is_err());
}

#[test]
fn merchant_component_stored_enchantments_roundtrip_uses_published_holders() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([((ENCHANTMENT_REGISTRY, "example:merchant"), 19)]),
    };
    let saved = saved_stack_with_components(
        "minecraft:enchanted_book",
        vec![entry(
            "minecraft:stored_enchantments",
            NbtSummaryValue::Compound(vec![entry("example:merchant", NbtSummaryValue::Int(3))]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    assert_eq!(
        summary.component_patch.stored_enchantments,
        vec![ItemEnchantmentSummary {
            holder_id: 19,
            level: 3
        }]
    );
    let raw = item_stack_summary_to_persistent_nbt_with_resolver(&summary, &resolver).unwrap();
    assert_eq!(
        crate::read_item_stack(&raw.clone(), &resolver).unwrap(),
        summary
    );
    let missing = HolderResolver {
        ids: BTreeMap::new(),
    };
    assert!(crate::read_item_stack(&saved, &missing).is_err());
    assert!(item_stack_summary_to_persistent_nbt_with_resolver(&summary, &missing).is_err());
    let mut malformed = summary;
    malformed.component_patch.stored_enchantments[0].level = 0;
    assert!(item_stack_summary_to_persistent_nbt_with_resolver(&malformed, &resolver).is_err());
}

#[test]
fn merchant_component_additional_trade_cost_remains_transient() {
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID]);
    patch.additional_trade_cost = Some(7);
    let summary = stack(VANILLA_ITEM_STONE_ID, 1, patch);
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&raw),
        Some(&NbtSummaryValue::Compound(vec![]))
    );
}
