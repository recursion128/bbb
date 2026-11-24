use super::*;

#[test]
fn instrument_tooltip_context_uses_typed_session_order_and_safe_stub_fallback() {
    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    fn instrument_nbt(description: &str, use_duration: f32) -> Vec<u8> {
        let mut out = vec![10, 10];
        write_nbt_string(&mut out, "sound_event");
        out.push(8);
        write_nbt_string(&mut out, "sound_id");
        write_nbt_string(&mut out, "example:horn");
        out.push(0);
        out.push(5);
        write_nbt_string(&mut out, "use_duration");
        out.extend_from_slice(&use_duration.to_bits().to_be_bytes());
        out.push(5);
        write_nbt_string(&mut out, "range");
        out.extend_from_slice(&96.0_f32.to_bits().to_be_bytes());
        out.push(8);
        write_nbt_string(&mut out, "description");
        write_nbt_string(&mut out, description);
        out.push(0);
        out
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        bbb_client_world::INSTRUMENT_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:custom",
            instrument_nbt("Custom horn", 4.0),
        )],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.record_registry_entries(
        bbb_client_world::INSTRUMENT_REGISTRY,
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::stub("minecraft:ponder_goat_horn"),
            bbb_client_world::RegistryPacketEntry::stub("example:unknown_stub"),
            bbb_client_world::RegistryPacketEntry::with_raw_data(
                "minecraft:sing_goat_horn",
                instrument_nbt("Broken", 0.0),
            ),
        ],
    );

    let (keys, presentations) = world_instrument_tooltip_context(&world).unwrap();
    assert_eq!(
        &*keys,
        ["example:custom", "minecraft:ponder_goat_horn", "", ""]
    );
    assert_eq!(
        presentations[0]
            .as_ref()
            .map(|presentation| presentation.description.as_str()),
        Some("Custom horn")
    );
    assert!(presentations[1..].iter().all(Option::is_none));
}

#[test]
fn trim_tooltip_context_keeps_valid_payloads_and_only_known_vanilla_stubs() {
    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }
    fn trim_nbt(asset_field: &str, asset: &str, description: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, asset_field, asset);
        write_string(&mut out, "description", description);
        out.push(0);
        out
    }

    let mut world = WorldStore::new();
    world.begin_registry_configuration();
    for data in bbb_client_render::scene::synchronized_registry_test_fixture::minimal_non_empty_synchronized_registry_data() {
        world.record_registry_data(data);
    }
    world.record_registry_entries(
        bbb_client_world::TRIM_MATERIAL_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:opal",
            trim_nbt("asset_name", "opal", "Opal"),
        )],
    );
    world.record_registry_entries(
        bbb_client_world::TRIM_PATTERN_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:spiral",
            trim_nbt("asset_id", "example:spiral", "Spiral"),
        )],
    );
    world.finish_registry_configuration().unwrap();
    world.record_registry_entries(
        bbb_client_world::TRIM_MATERIAL_REGISTRY,
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::stub("minecraft:iron"),
            bbb_client_world::RegistryPacketEntry::stub("example:custom_stub"),
            bbb_client_world::RegistryPacketEntry::with_data_len("minecraft:diamond", 12),
        ],
    );
    world.record_registry_entries(
        bbb_client_world::TRIM_PATTERN_REGISTRY,
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::stub("minecraft:spire"),
            bbb_client_world::RegistryPacketEntry::stub("example:custom_stub"),
        ],
    );

    let (material_keys, material_presentations) =
        world_trim_material_tooltip_context(&world).unwrap();
    assert_eq!(&*material_keys, ["example:opal", "minecraft:iron", "", ""]);
    assert_eq!(
        material_presentations[0]
            .as_ref()
            .map(|material| material.description.as_str()),
        Some("Opal")
    );
    assert!(material_presentations[1..].iter().all(Option::is_none));

    let (pattern_keys, pattern_presentations) = world_trim_pattern_tooltip_context(&world).unwrap();
    assert_eq!(&*pattern_keys, ["example:spiral", "minecraft:spire", ""]);
    assert_eq!(
        pattern_presentations[0]
            .as_ref()
            .map(|pattern| pattern.description.as_str()),
        Some("Spiral")
    );
    assert!(pattern_presentations[1..].iter().all(Option::is_none));
}

#[test]
fn banner_tooltip_context_keeps_typed_payloads_and_known_vanilla_stubs() {
    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_string(out, name);
        write_nbt_string(out, value);
    }
    fn pattern_nbt(asset_id: &str, translation_key: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_string(&mut out, "asset_id", asset_id);
        write_string(&mut out, "translation_key", translation_key);
        out.push(0);
        out
    }

    let mut world = WorldStore::new();
    world.record_registry_entries(
        bbb_client_world::BANNER_PATTERN_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:custom",
            pattern_nbt("example:banner/custom", "block.example.banner.custom"),
        )],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.record_registry_entries(
        bbb_client_world::BANNER_PATTERN_REGISTRY,
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::stub("minecraft:guster"),
            bbb_client_world::RegistryPacketEntry::stub("example:unknown_stub"),
            bbb_client_world::RegistryPacketEntry::with_data_len("minecraft:base", 12),
            bbb_client_world::RegistryPacketEntry::with_raw_data(
                "example:malformed",
                pattern_nbt("Invalid Asset", "block.example.banner.invalid"),
            ),
        ],
    );

    let (keys, presentations) = world_banner_pattern_tooltip_context(&world).unwrap();
    assert_eq!(&*keys, ["example:custom", "minecraft:guster", "", "", ""]);
    assert_eq!(
        presentations[0]
            .as_ref()
            .map(|presentation| &presentation.translation_key),
        Some(&bbb_protocol::JavaString::from(
            "block.example.banner.custom"
        ))
    );
    assert_eq!(
        presentations[1]
            .as_ref()
            .map(|presentation| &presentation.translation_key),
        Some(&bbb_protocol::JavaString::from(
            "block.minecraft.banner.guster"
        ))
    );
    assert!(presentations[2..].iter().all(Option::is_none));
}

#[test]
fn painting_tooltip_context_uses_session_order_and_typed_components() {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let painting = NbtSummaryValue::Compound(vec![
        entry("width", NbtSummaryValue::Int(4)),
        entry("height", NbtSummaryValue::Int(2)),
        entry("asset_id", NbtSummaryValue::string("example:gallery/wide")),
        entry("title", NbtSummaryValue::string("Custom gallery")),
    ]);
    let mut world = WorldStore::new();
    world.begin_registry_configuration();
    for data in bbb_client_render::scene::synchronized_registry_test_fixture::minimal_non_empty_synchronized_registry_data() {
        if data.registry != bbb_client_world::PAINTING_VARIANT_REGISTRY {
            world.record_registry_data(data);
        }
    }
    world.record_registry_entries(
        bbb_client_world::PAINTING_VARIANT_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::with_raw_data(
            "example:wide",
            bbb_protocol::nbt_summary::encode_nbt_summary(&painting, 4096).unwrap(),
        )],
    );
    world.finish_registry_configuration().unwrap();
    world.record_registry_entries(
        bbb_client_world::PAINTING_VARIANT_REGISTRY,
        0,
        vec![
            bbb_client_world::RegistryPacketEntry::stub("minecraft:kebab"),
            bbb_client_world::RegistryPacketEntry::stub("example:unknown"),
        ],
    );
    let presentations = world_painting_variant_tooltip_context(&world).unwrap();
    assert_eq!(presentations.len(), 3);
    assert_eq!(
        presentations[0]
            .as_ref()
            .map(|presentation| presentation.title.as_deref()),
        Some(Some("Custom gallery"))
    );
    assert_eq!(
        presentations[1]
            .as_ref()
            .and_then(|presentation| presentation.title_translation_key.as_deref()),
        Some("painting.minecraft.kebab.title")
    );
    assert_eq!(presentations[2], None);
}

#[test]
fn tooltip_registry_context_reuses_arc_cache_and_invalidates_on_registry_revision() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        bbb_client_world::INSTRUMENT_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::stub(
            "minecraft:ponder_goat_horn",
        )],
    );
    world.record_registry_entries(
        bbb_client_world::ENCHANTMENT_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::stub(
            "minecraft:sharpness",
        )],
    );

    let (keys_a, presentations_a) = world_instrument_tooltip_context(&world).unwrap();
    let (keys_b, presentations_b) = world_instrument_tooltip_context(&world).unwrap();
    assert!(Arc::ptr_eq(&keys_a, &keys_b));
    assert!(Arc::ptr_eq(&presentations_a, &presentations_b));

    let defs_a = world_enchantment_tooltip_definitions(&world).unwrap();
    let defs_b = world_enchantment_tooltip_definitions(&world).unwrap();
    assert!(Arc::ptr_eq(&defs_a, &defs_b));

    world.record_registry_entries(
        bbb_client_world::INSTRUMENT_REGISTRY,
        0,
        vec![bbb_client_world::RegistryPacketEntry::stub(
            "minecraft:sing_goat_horn",
        )],
    );
    let (keys_c, _) = world_instrument_tooltip_context(&world).unwrap();
    assert!(!Arc::ptr_eq(&keys_a, &keys_c));
    assert_eq!(
        &*keys_c,
        ["minecraft:ponder_goat_horn", "minecraft:sing_goat_horn"]
    );
}
