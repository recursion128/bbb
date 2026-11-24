use super::*;

fn nbt_round_trip(raw: NamedCompound) -> NamedCompound {
    let bytes = bbb_protocol::encode_named_nbt_summary_java_string(
        raw.root_name(),
        &raw.to_summary(),
        1 << 20,
    )
    .unwrap();
    let (name, value) = bbb_protocol::decode_named_saved_nbt_summary(&bytes)
        .unwrap()
        .unwrap();
    NamedCompound::try_from_summary(name, value).unwrap()
}

#[test]
fn native_ender_pearls_survive_player_save_and_are_cleared_when_gone() {
    let uuid = Uuid::from_u128(7);
    let mut pearls = Vec::new();
    for (index, dimension) in ["minecraft:the_nether", "example:sky/moon"]
        .into_iter()
        .enumerate()
    {
        let mut entries =
            embedded_entity_entries(Uuid::from_u128(8 + index as u128), "minecraft:ender_pearl");
        super::super::field_patches::set_field(&mut entries, "Motion", vec3([0.25, 0.5, -0.75]));
        entries.extend([
            e(
                "ender_pearl_dimension",
                NbtSummaryValue::String(dimension.into()),
            ),
            e("Owner", uuid_array(uuid)),
            e(
                "Item",
                NbtSummaryValue::Compound(vec![
                    e(
                        "id",
                        NbtSummaryValue::String("minecraft:ender_pearl".into()),
                    ),
                    e("count", NbtSummaryValue::Int(1)),
                ]),
            ),
            e("future_pearl", NbtSummaryValue::Long(42)),
            e(
                "Passengers",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(embedded_entity_entries(
                    Uuid::from_u128(20 + index as u128),
                    "minecraft:pig",
                ))]),
            ),
        ]);
        pearls.push(NbtSummaryValue::Compound(entries));
    }
    let mut raw = canonical(uuid);
    raw.entries_mut()
        .push(e("ender_pearls", NbtSummaryValue::List(pearls.clone())));
    let parsed = Java26PlayerData::parse(nbt_round_trip(raw), uuid).unwrap();
    assert_eq!(parsed.ender_pearls().len(), 2);
    assert_eq!(
        parsed.ender_pearls()[0].dimension.as_str(),
        "minecraft:the_nether"
    );
    assert_eq!(
        parsed.ender_pearls()[1].dimension.as_str(),
        "example:sky/moon"
    );
    assert_eq!(
        parsed.ender_pearls()[0]
            .entity
            .as_ref()
            .passengers()
            .next()
            .unwrap()
            .uuid(),
        Uuid::from_u128(20)
    );
    let mut patch = created_patch(uuid);
    patch.ender_pearls = parsed.ender_pearls().to_vec();
    let written = parsed
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let restored = Java26PlayerData::parse(nbt_round_trip(written), uuid).unwrap();
    assert_eq!(restored.ender_pearls(), parsed.ender_pearls());
    assert_eq!(
        field_of(restored.raw(), "ender_pearls"),
        &NbtSummaryValue::List(pearls)
    );
    assert_eq!(
        field_of(restored.raw(), "unknown_future"),
        &NbtSummaryValue::Long(42)
    );
    patch.ender_pearls.clear();
    let written = restored
        .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
        .unwrap();
    let cleared = Java26PlayerData::parse(nbt_round_trip(written), uuid).unwrap();
    assert!(cleared.ender_pearls().is_empty());
    assert!(!cleared
        .raw()
        .entries()
        .iter()
        .any(|entry| entry.name == "ender_pearls"));
}
