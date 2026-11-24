//! `ResultSlot.onTake` (`ResultSlot.java:82-112`) over the 2x2 player grid.

use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BLUE_DYE_ID, VANILLA_ITEM_FIREWORK_STAR_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID,
    VANILLA_ITEM_FLINT_ID, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_GUNPOWDER_ID,
    VANILLA_ITEM_HONEY_BOTTLE_ID, VANILLA_ITEM_IRON_INGOT_ID, VANILLA_ITEM_RED_DYE_ID,
    VANILLA_ITEM_STICK_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_SUGAR_ID,
    VANILLA_ITEM_WHITE_BANNER_ID,
};
use bbb_protocol::packets::BannerPatternLayerSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_protocol::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::registry_codec::RegistryReferenceLookup;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::recipe::{ItemTagIndex, RecipeElement, RecipeSet};

use super::*;

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn recipe_set(id: &str, body: serde_json::Value) -> std::sync::Arc<RecipeSet> {
    let bytes = serde_json::to_vec(&body).unwrap();
    let source = bbb_registry_codec::decode_direct_registry_json(&bytes).unwrap();
    std::sync::Arc::new(RecipeSet {
        recipes: vec![RecipeElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            recipe: bbb_server_datapack::recipe::parse_recipe_source_with_lookup(
                source.root(),
                &vanilla_lookup(),
            )
            .expect("test recipe"),
        }],
        warnings: Vec::new(),
    })
}

struct FixedLookup;

impl RegistryReferenceLookup for FixedLookup {
    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        matches!(
            bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
                registry,
                resource_id
            ),
            PersistentRegistryMembership::Present
        )
    }

    fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
        false
    }
}

impl PersistentRegistryResolver for FixedLookup {
    fn resource_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
            registry,
            resource_id,
        )
    }

    fn tag_members(&self, _registry: &str, _tag: &str) -> PersistentRegistryTagMembers {
        PersistentRegistryTagMembers::Absent
    }
}

impl PersistentItemStackHolderResolver for FixedLookup {
    fn holder_id(&self, _registry: &str, _resource_id: &str) -> Option<i32> {
        None
    }
}

/// Recipe templates decode against the actual startup prototypes and holder scopes.
fn vanilla_lookup() -> bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup<'static> {
    static STARTUP: std::sync::OnceLock<bbb_server_datapack::startup::DatapackStartup> =
        std::sync::OnceLock::new();
    bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(
        STARTUP.get_or_init(|| {
            bbb_server_datapack::startup::DatapackStartup::load(
                &bbb_server_datapack::startup::DatapackStartupOptions {
                    datapacks_dir: None,
                    saved_data_packs: None,
                    safe_mode: false,
                    limits: Default::default(),
                },
            )
            .expect("vanilla publication loads")
        }),
    )
}

#[test]
fn variant_recipes_use_the_reordered_startup_publication_through_craft_save_and_slot() {
    use bbb_protocol::data_component_types::vanilla_data_component_resource_id_for_type_id;
    use bbb_protocol::item_types::{vanilla_item_type, VANILLA_ITEM_EGG_ID};
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_registry_codec::{
        decode_direct_registry_json, item_stack_summary_to_persistent_nbt_with_resolver,
    };
    use bbb_server_datapack::recipe::parse_recipe_source_with_lookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    struct TempPack(std::path::PathBuf);
    impl Drop for TempPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let options = |datapacks_dir| DatapackStartupOptions {
        datapacks_dir,
        saved_data_packs: None,
        safe_mode: false,
        limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
    };
    let vanilla = DatapackStartup::load(&options(None)).unwrap();
    let registries = [
        (82, "minecraft:wolf_variant"),
        (97, "minecraft:chicken_variant"),
    ];
    let world =
        TempPack(std::env::temp_dir().join(format!("bbb-variant-recipe-{}", std::process::id())));
    let pack = world.0.join("datapacks/variants");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"variant test","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    let mut names = Vec::new();
    for (type_id, registry) in registries {
        let original = vanilla
            .registries
            .registry(registry)
            .unwrap()
            .elements
            .first()
            .unwrap();
        let name = original.id.clone();
        let entry_dir = pack.join(format!(
            "data/aaa/{}",
            registry.strip_prefix("minecraft:").unwrap()
        ));
        std::fs::create_dir_all(&entry_dir).unwrap();
        std::fs::write(entry_dir.join("aaa.json"), original.source_json.as_ref()).unwrap();
        names.push((type_id, registry, name));
    }
    let mut sources = vec![(81, "minecraft:plains".to_string())];
    sources.extend(names.iter().map(|(id, _, name)| (*id, name.clone())));
    for (type_id, name) in &sources {
        let component = vanilla_data_component_resource_id_for_type_id(*type_id).unwrap();
        let mut components = serde_json::Map::new();
        components.insert(
            component.to_string(),
            serde_json::Value::String(name.clone()),
        );
        let body = serde_json::json!({
            "type":"crafting_shapeless", "ingredients":["stone"],
            "result":{"id":"stick", "components":components}
        });
        let recipe_dir = pack.join("data/aaa/recipe");
        std::fs::create_dir_all(&recipe_dir).unwrap();
        std::fs::write(
            recipe_dir.join(format!("variant_{type_id}.json")),
            serde_json::to_vec(&body).unwrap(),
        )
        .unwrap();
    }
    let startup = DatapackStartup::load(&options(Some(world.0.join("datapacks")))).unwrap();
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(&startup);
    for (type_id, registry, original) in &names {
        let loaded = startup.registries.registry(registry).unwrap();
        assert_eq!(loaded.elements[0].id, "aaa:aaa");
        assert_eq!(loaded.elements[1].id, *original);
        assert_eq!(lookup.holder_id(registry, original), Some(1), "{registry}");
        assert!(startup
            .recipes
            .recipe(&format!("aaa:variant_{type_id}"))
            .is_some());
    }
    let field = |patch: &bbb_protocol::packets::DataComponentPatchSummary, type_id| match type_id {
        81 => patch.villager_variant_id,
        82 => patch.wolf_variant_id,
        83 => patch.wolf_sound_variant_id,
        93 => patch.pig_variant_id,
        94 => patch.pig_sound_variant_id,
        95 => patch.cow_variant_id,
        96 => patch.cow_sound_variant_id,
        97 => patch.chicken_variant_id,
        98 => patch.chicken_sound_variant_id,
        99 => patch.zombie_nautilus_variant_id,
        100 => patch.frog_variant_id,
        102 => patch.painting_variant_id,
        105 => patch.cat_variant_id,
        106 => patch.cat_sound_variant_id,
        _ => unreachable!(),
    };
    for (type_id, name) in &sources {
        let element = startup
            .recipes
            .recipe(&format!("aaa:variant_{type_id}"))
            .unwrap()
            .clone();
        let mut menu = PlayerInventoryMenu::empty();
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            std::sync::Arc::new(RecipeSet {
                recipes: vec![element],
                warnings: Vec::new(),
            }),
            ItemTagIndex::new(),
        )
        .unwrap();
        assert!(menu
            .set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                1,
                RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
            )
            .unwrap());
        let crafted = menu
            .slot_safe_take(
                &bbb_registry_codec::UnavailableHolderResolver,
                None,
                0,
                1,
                CARRIED_SLOT_SIZE,
                false,
            )
            .unwrap();
        assert_eq!(crafted.item_id(), Some(VANILLA_ITEM_STICK_ID));
        let expected = if *type_id == 81 {
            Some(2)
        } else {
            let registry = names.iter().find(|(id, _, _)| id == type_id).unwrap().1;
            lookup.holder_id(registry, name)
        };
        assert_eq!(
            field(&crafted.summary().component_patch, *type_id),
            expected
        );
        assert!(expected.is_some());
        assert_eq!(crafted.summary().component_patch.added_type_ids, [*type_id]);
        for (other, _) in &sources {
            if other != type_id {
                assert_eq!(
                    field(&crafted.summary().component_patch, *other),
                    None,
                    "{type_id} changed {other}"
                );
            }
        }
        assert!(crafted
            .summary()
            .component_patch
            .painting_variant_direct
            .is_none());
        let nbt =
            item_stack_summary_to_persistent_nbt_with_resolver(crafted.summary(), &lookup).unwrap();
        let saved = nbt.clone();
        let reloaded = bbb_registry_codec::read_item_stack(&saved, &lookup).unwrap();
        assert_eq!(field(&reloaded.component_patch, *type_id), expected);
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: reloaded,
                },
            )
            .unwrap();
        let PlayClientbound::ContainerSetSlot(slot) =
            decode_play_clientbound(packet_id, &bytes).unwrap()
        else {
            panic!("slot packet");
        };
        assert_eq!(field(&slot.item.component_patch, *type_id), expected);
    }
    let catalog = bbb_protocol::vanilla_item_prototype_component_catalog().unwrap();
    {
        let item_id = VANILLA_ITEM_EGG_ID;
        let item_name = vanilla_item_type(item_id).unwrap().resource_id;
        let registered = catalog
            .component_codec_values(item_id)
            .unwrap()
            .get(&97)
            .unwrap_or_else(|| panic!("egg variant default {item_name}/{item_id}"))
            .clone();
        let template = |value: serde_json::Value| {
            let mut components = serde_json::Map::new();
            components.insert("minecraft:chicken/variant".into(), value);
            serde_json::json!({"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":item_name,"components":components}})
        };
        let craft = |body: serde_json::Value| {
            let bytes = serde_json::to_vec(&body).unwrap();
            let source = decode_direct_registry_json(&bytes).unwrap();
            let recipe = parse_recipe_source_with_lookup(source.root(), &lookup).unwrap();
            recipe.assemble_crafting(
                &bbb_server_datapack::recipe::CraftingInput::of(
                    1,
                    1,
                    vec![stack(VANILLA_ITEM_STONE_ID, 1)],
                ),
                &ItemTagIndex::new(),
            )
        };
        let folded = craft(template(registered));
        assert_eq!(folded.item_id, Some(item_id));
        assert!(!folded.component_patch.added_type_ids.contains(&97));
        let changed = craft(template(serde_json::Value::String("aaa:aaa".into())));
        assert_eq!(changed.item_id, Some(item_id));
        assert_eq!(changed.component_patch.chicken_variant_id, Some(0));
        assert!(changed.component_patch.added_type_ids.contains(&97));
        let removed = craft(
            serde_json::json!({"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":item_name,"components":{"!minecraft:chicken/variant":{}}}}),
        );
        assert_eq!(removed.item_id, Some(item_id));
        assert!(removed.component_patch.removed_type_ids.contains(&97));
    }
}

#[test]
fn native_record_and_holder_values_craft_through_frozen_save_and_slot() {
    use bbb_protocol::packets::data_components::fields::component_values_equal;
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_registry_codec::{
        decode_direct_registry_json, item_stack_summary_to_persistent_nbt_with_resolver,
    };
    use bbb_server_datapack::recipe::parse_recipe_source_with_lookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    let vanilla = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
    })
    .unwrap();
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(&vanilla);
    let description = r#"{"text":"owned","hover_event":{"action":"show_item","id":"minecraft:stone","count":2},"click_event":{"action":"show_dialog","dialog":"minecraft:server_links"}}"#;
    let holders = format!(
        r#"{{"trim":{{"material":{{"asset_name":"copper","description":{description}}},"pattern":{{"asset_id":"test:wave","description":{description},"decal":true}}}},"provides_trim_material":{{"asset_name":"quartz","override_armor_assets":{{"minecraft:iron":"quartz_darker"}},"description":{description}}},"instrument":{{"sound_event":{{"sound_id":"test:horn","range":32.0}},"use_duration":7.0,"range":256.0,"description":{description}}},"jukebox_playable":"minecraft:cat","enchantments":{{"minecraft:sharpness":2,"minecraft:unbreaking":3}},"stored_enchantments":{{"minecraft:mending":1}}}}"#
    );
    let combat = r##"{"tool":{"rules":[{"blocks":"#minecraft:mineable/pickaxe","speed":8.0,"correct_for_drops":true},{"blocks":["stone","dirt"],"speed":0.5}],"default_mining_speed":2.0,"damage_per_block":0},"attribute_modifiers":[{"type":"attack_damage","id":"test:bonus","amount":2.5,"operation":"add_value","slot":"mainhand","display":{"type":"override","value":{"text":"bonus","italic":false}}},{"type":"armor","id":"test:hidden","amount":-1.0,"operation":"add_multiplied_total","display":{"type":"hidden"}}],"equippable":{"slot":"head","allowed_entities":["zombie","husk"],"equip_sound":{"sound_id":"test:equip","range":8.0},"camera_overlay":"minecraft:misc/pumpkinblur"},"blocks_attacks":{"damage_reductions":[{"type":"#minecraft:is_fire","base":1.0,"factor":0.5},{"horizontal_blocking_angle":45.0,"base":0.0,"factor":1.0}],"bypassed_by":"#minecraft:bypasses_shield","block_sound":"minecraft:item.shield.block"},"kinetic_weapon":{"delay_ticks":5,"damage_conditions":{"max_duration_ticks":20,"min_speed":1.5},"damage_multiplier":0.7},"piercing_weapon":{"dismounts":true,"hit_sound":"minecraft:item.spear_wood.hit"}}"##;
    let records = r#"{"profile":{"name":"Steve","properties":[{"name":"textures","value":"e30="},{"name":"extra","value":"v","signature":"signed"}],"texture":"entity/player/wide/steve","model":"slim"},"bees":[{"entity_data":{"id":"minecraft:bee","Health":5.0,"CannotEnterHiveTicks":40},"ticks_in_hive":10,"min_ticks_in_hive":600}],"container_loot":{"loot_table":"chests/simple_dungeon","seed":42}}"#;
    let mut crafted_values = Vec::new();
    for (components, type_ids) in [
        (holders.as_str(), &[56, 62, 61, 64, 13, 42][..]),
        (combat, &[28, 16, 32, 37, 39, 38][..]),
        (records, &[70, 77, 79][..]),
    ] {
        let body = format!(
            r#"{{"type":"crafting_shapeless","ingredients":["stone"],"result":{{"id":"stick","components":{components}}}}}"#
        );
        let decoded = decode_direct_registry_json(body.as_bytes()).unwrap();
        let recipe = parse_recipe_source_with_lookup(decoded.root(), &lookup).unwrap();
        let mut menu = PlayerInventoryMenu::empty();
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            std::sync::Arc::new(RecipeSet {
                recipes: vec![RecipeElement {
                    id: "test:records".to_string(),
                    source_pack: "test".to_string(),
                    recipe,
                }],
                warnings: Vec::new(),
            }),
            ItemTagIndex::new(),
        )
        .unwrap();
        assert!(menu
            .set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                1,
                RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
            )
            .unwrap());
        let crafted = menu
            .slot_safe_take(
                &bbb_registry_codec::UnavailableHolderResolver,
                None,
                0,
                1,
                CARRIED_SLOT_SIZE,
                false,
            )
            .unwrap();
        assert_eq!(crafted.item_id(), Some(VANILLA_ITEM_STICK_ID));
        let raw =
            item_stack_summary_to_persistent_nbt_with_resolver(crafted.summary(), &lookup).unwrap();
        let saved = raw.clone();
        let loaded = bbb_registry_codec::read_item_stack(&saved, &lookup).unwrap();
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: loaded.clone(),
                },
            )
            .unwrap();
        let PlayClientbound::ContainerSetSlot(slot) =
            decode_play_clientbound(packet_id, &bytes).unwrap()
        else {
            panic!("slot packet");
        };
        for item in [&loaded, &slot.item] {
            for &type_id in type_ids {
                assert!(
                    item.component_patch.added_type_ids.contains(&type_id),
                    "{type_id}"
                );
                assert!(
                    component_values_equal(
                        type_id,
                        &crafted.summary().component_patch,
                        &item.component_patch
                    )
                    .unwrap(),
                    "{type_id}"
                );
            }
        }
        assert_eq!(
            item_stack_summary_to_persistent_nbt_with_resolver(&slot.item, &lookup).unwrap(),
            raw
        );
        crafted_values.push(crafted.summary().component_patch.clone());
    }

    let holders = &crafted_values[0];
    let material = holders.armor_trim_material_direct.as_deref().unwrap();
    assert_eq!(material.asset_name, "copper");
    assert!(holders.armor_trim_pattern_direct.as_deref().unwrap().decal);
    let provided = holders.provides_trim_material_direct.as_deref().unwrap();
    assert_eq!(
        provided.override_armor_assets["minecraft:iron"],
        "quartz_darker"
    );
    let owned_description = |value: &NbtSummaryValue| {
        let NbtSummaryValue::Compound(fields) = value else {
            return false;
        };
        ["hover_event", "click_event"]
            .iter()
            .all(|name| fields.iter().any(|field| field.name == *name))
    };
    for description in [
        material.description_nbt.as_deref(),
        provided.description_nbt.as_deref(),
        holders
            .instrument
            .direct
            .as_deref()
            .unwrap()
            .description_nbt
            .as_deref(),
    ] {
        assert!(owned_description(description.unwrap()));
    }
    assert_eq!(
        holders.instrument.direct.as_deref().unwrap().description,
        "owned"
    );
    assert!(holders.jukebox_song_id.is_some());
    assert_eq!(holders.enchantments.len(), 2);
    assert_eq!(holders.stored_enchantments.len(), 1);

    let combat = &crafted_values[1];
    assert_eq!(
        combat.tool.as_ref().unwrap().rules[1].block_registry_ids,
        [
            bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
            bbb_protocol::block_types::VANILLA_BLOCK_DIRT_ID,
        ]
    );
    assert_eq!(combat.attribute_modifiers[1].display_id, 1);
    assert_eq!(
        combat.attribute_modifiers[0].display_text.as_deref(),
        Some("bonus")
    );
    let blocks = combat.blocks_attacks.as_ref().unwrap();
    assert_eq!(
        blocks.bypassed_by.as_ref().unwrap().tag.as_deref(),
        Some("minecraft:bypasses_shield")
    );
    assert_eq!(
        combat
            .equippable
            .as_ref()
            .unwrap()
            .equip_sound
            .fixed_range_bits,
        Some(8.0_f32.to_bits())
    );

    let records = &crafted_values[2];
    let profile = records.profile.as_ref().unwrap();
    assert_eq!(profile.properties.len(), 2);
    assert_eq!(
        profile.properties[1].signature.as_ref().unwrap().units(),
        "signed".encode_utf16().collect::<Vec<_>>()
    );
    assert_eq!(
        profile.skin_patch.body.as_ref().unwrap().asset_id,
        "minecraft:entity/player/wide/steve"
    );
    assert!(records.bees_present);
    assert_eq!(records.bees.len(), 1);
    assert_eq!(
        records.container_loot_table.as_deref(),
        Some("minecraft:chests/simple_dungeon")
    );
    assert_eq!(records.container_loot_seed, Some(42));
}

#[test]
fn collection_components_craft_save_reload_and_slot() {
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_registry_codec::item_stack_summary_to_persistent_nbt_with_resolver;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    struct TempPack(std::path::PathBuf);
    impl Drop for TempPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let world = TempPack(
        std::env::temp_dir().join(format!("bbb-collection-recipe-{}", std::process::id())),
    );
    let pack = world.0.join("datapacks/collections");
    let recipe_dir = pack.join("data/aaa/recipe");
    std::fs::create_dir_all(&recipe_dir).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"collection test","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(recipe_dir.join("all.json"), br#"{"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":"stick","components":{"debug_stick_state":{"minecraft:oak_log":123,"minecraft:oak_log":"axis"},"recipes":["test:first","test:second","test:first"],"pot_decorations":["minecraft:angler_pottery_sherd","minecraft:blade_pottery_sherd"],"block_state":{"a":"old","?":"literal","plain":"plain","a":"new"}}}}"#).unwrap();
    std::fs::write(recipe_dir.join("empty_pot.json"), br#"{"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":"stick","components":{"pot_decorations":[]}}}"#).unwrap();

    let startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: Some(world.0.join("datapacks")),
        saved_data_packs: None,
        safe_mode: false,
        limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
    })
    .unwrap();
    let lookup = bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::new(
        &startup.registries,
        &startup.tags,
    );
    let craft = |name: &str| {
        let element = startup.recipes.recipe(name).unwrap().clone();
        let mut menu = PlayerInventoryMenu::empty();
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            std::sync::Arc::new(RecipeSet {
                recipes: vec![element],
                warnings: Vec::new(),
            }),
            ItemTagIndex::new(),
        )
        .unwrap();
        assert!(menu
            .set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                1,
                RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
            )
            .unwrap());
        let crafted = menu
            .slot_safe_take(
                &bbb_registry_codec::UnavailableHolderResolver,
                None,
                0,
                1,
                CARRIED_SLOT_SIZE,
                false,
            )
            .unwrap();
        assert_eq!(crafted.item_id(), Some(VANILLA_ITEM_STICK_ID));
        crafted
    };
    let round_trip = |crafted: ItemStackSummary| {
        let raw = item_stack_summary_to_persistent_nbt_with_resolver(&crafted, &lookup).unwrap();
        let saved = raw.clone();
        let loaded = bbb_registry_codec::read_item_stack(&saved, &lookup).unwrap();
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: loaded.clone(),
                },
            )
            .unwrap();
        let PlayClientbound::ContainerSetSlot(slot) =
            decode_play_clientbound(packet_id, &bytes).unwrap()
        else {
            panic!("slot packet");
        };
        (crafted, saved, loaded, slot.item)
    };

    let (crafted, saved, loaded, slot) = round_trip(craft("aaa:all").summary().clone());
    let expected_recipes = vec![
        "test:first".to_string(),
        "test:second".to_string(),
        "test:first".to_string(),
    ];
    let expected_pot = vec![
        bbb_protocol::item_types::VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID,
        bbb_protocol::item_types::VANILLA_ITEM_BLADE_POTTERY_SHERD_ID,
        bbb_protocol::item_types::VANILLA_ITEM_BRICK_ID,
        bbb_protocol::item_types::VANILLA_ITEM_BRICK_ID,
    ];
    for item in [&crafted, &loaded, &slot] {
        let patch = &item.component_patch;
        for type_id in [57, 66, 74, 76] {
            assert!(patch.added_type_ids.contains(&type_id), "{type_id}");
        }
        assert_eq!(patch.debug_stick_properties.len(), 1);
        assert_eq!(
            patch.debug_stick_properties[0].block_registry_id,
            bbb_protocol::block_types::VANILLA_BLOCK_OAK_LOG_ID
        );
        assert_eq!(patch.debug_stick_properties[0].property, "axis");
        assert_eq!(patch.recipe_keys.as_ref(), Some(&expected_recipes));
        assert_eq!(patch.pot_decorations_item_ids, expected_pot);
    }
    for item in [&crafted, &loaded, &slot] {
        let properties = &item.component_patch.block_state_properties;
        assert_eq!(properties.len(), 3);
        assert_eq!(properties.get_str("a").unwrap(), "new");
        assert_eq!(properties.get_str("?").unwrap(), "literal");
        assert_eq!(properties.get_str("plain").unwrap(), "plain");
    }

    let saved_components =
        bbb_protocol::nbt_summary::value_last_field(&saved, "components").unwrap();
    let saved_value = |type_id| {
        let component =
            bbb_protocol::data_component_types::vanilla_data_component_type(type_id).unwrap();
        bbb_protocol::nbt_summary::value_last_field(saved_components, component.resource_id)
            .unwrap()
    };
    assert_eq!(
        saved_value(66),
        &NbtSummaryValue::List(
            expected_recipes
                .iter()
                .cloned()
                .map(NbtSummaryValue::string)
                .collect()
        )
    );
    assert_eq!(
        saved_value(74),
        &NbtSummaryValue::List(vec![
            NbtSummaryValue::string("minecraft:angler_pottery_sherd"),
            NbtSummaryValue::string("minecraft:blade_pottery_sherd"),
            NbtSummaryValue::string("minecraft:brick"),
            NbtSummaryValue::string("minecraft:brick"),
        ])
    );
    let NbtSummaryValue::Compound(saved_properties) = saved_value(76) else {
        panic!("block state NBT map");
    };
    assert_eq!(saved_properties.len(), 3);
    assert!(saved_properties
        .iter()
        .any(|entry| entry.name == "a" && entry.value == NbtSummaryValue::String("new".into())));

    let (empty, empty_saved, empty_loaded, empty_slot) =
        round_trip(craft("aaa:empty_pot").summary().clone());
    let bricks = vec![bbb_protocol::item_types::VANILLA_ITEM_BRICK_ID; 4];
    for item in [&empty, &empty_loaded, &empty_slot] {
        assert!(item.component_patch.added_type_ids.contains(&74));
        assert_eq!(item.component_patch.pot_decorations_item_ids, bricks);
    }
    assert_eq!(
        bbb_protocol::nbt_summary::value_last_field(
            bbb_protocol::nbt_summary::value_last_field(&empty_saved, "components").unwrap(),
            "minecraft:pot_decorations",
        )
        .unwrap(),
        &NbtSummaryValue::List(vec![NbtSummaryValue::string("minecraft:brick"); 4])
    );
    assert!(!stack(VANILLA_ITEM_STICK_ID, 1)
        .component_patch
        .added_type_ids
        .contains(&74));
}

#[test]
fn raw_consume_effect_recipe_take_keeps_generic_hidden_through_save_and_wire() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID, VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
    };
    use bbb_protocol::packets::{
        play_clientbound::{decode_play_clientbound, PlayClientbound},
        ConsumeEffectSummary,
    };
    use bbb_registry_codec::{decode_direct_registry_json, item_stack_summary_to_persistent_nbt};
    use bbb_server_datapack::recipe::parse_recipe_source_with_lookup;

    let source = decode_direct_registry_json(br#"{"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":"stick","components":{"consumable":{"has_consume_particles":false,"sound":{"sound_id":"example:drink"},"on_consume_effects":[{"type":"apply_effects","effects":[{"id":"haste","amplifier":511,"duration":200,"hidden_effect":{"duration":40}}]}]},"death_protection":{"death_effects":[{"type":"apply_effects","effects":[{"id":"haste","duration":100,"hidden_effect":{"duration":20}}]}]}}}}"#).unwrap();
    let recipe = parse_recipe_source_with_lookup(source.root(), &vanilla_lookup()).unwrap();
    let set = RecipeSet {
        recipes: vec![RecipeElement {
            id: "test:consume".into(),
            source_pack: "test".into(),
            recipe,
        }],
        warnings: Vec::new(),
    };
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        std::sync::Arc::new(set),
        ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
        )
        .unwrap());
    let crafted = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(crafted.item_id(), Some(VANILLA_ITEM_STICK_ID));
    assert_eq!(
        crafted
            .summary()
            .component_patch
            .consumable_has_consume_particles,
        Some(false)
    );
    assert_eq!(
        crafted
            .summary()
            .component_patch
            .consumable_sound
            .as_ref()
            .unwrap()
            .fixed_range_bits,
        None
    );
    assert!(crafted
        .summary()
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID));
    assert!(crafted
        .summary()
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID));
    let verify_hidden = |item: &ItemStackSummary| {
        let ConsumeEffectSummary::ApplyEffects { effects, .. } =
            &item.component_patch.consumable_effects[0]
        else {
            panic!("consumable apply");
        };
        assert_eq!(effects[0].amplifier, 255);
        assert_eq!(effects[0].hidden_effect.as_ref().unwrap().duration, 40);
        let ConsumeEffectSummary::ApplyEffects { effects, .. } =
            &item.component_patch.death_protection_effects[0]
        else {
            panic!("death apply");
        };
        assert_eq!(effects[0].hidden_effect.as_ref().unwrap().duration, 20);
    };
    verify_hidden(crafted.summary());
    let raw = item_stack_summary_to_persistent_nbt(crafted.summary()).unwrap();
    let components = bbb_protocol::nbt_summary::value_last_field(&raw, "components").unwrap();
    let consumable =
        bbb_protocol::nbt_summary::value_last_field(components, "minecraft:consumable").unwrap();
    let NbtSummaryValue::Compound(fields) = consumable else {
        panic!("consumable compound");
    };
    let NbtSummaryValue::List(on_consume) = &fields
        .iter()
        .find(|field| field.name == "on_consume_effects")
        .unwrap()
        .value
    else {
        panic!("effect list");
    };
    let NbtSummaryValue::Compound(apply) = &on_consume[0] else {
        panic!("apply");
    };
    let NbtSummaryValue::List(effects) = &apply
        .iter()
        .find(|field| field.name == "effects")
        .unwrap()
        .value
    else {
        panic!("effects");
    };
    let NbtSummaryValue::Compound(effect) = &effects[0] else {
        panic!("effect");
    };
    assert_eq!(
        effect
            .iter()
            .find(|field| field.name == "amplifier")
            .unwrap()
            .value,
        NbtSummaryValue::Byte(-1)
    );
    assert!(effect.iter().any(|field| field.name == "hidden_effect"));
    let loaded = bbb_registry_codec::read_item_stack(&raw, &FixedLookup).unwrap();
    verify_hidden(&loaded);
    let (packet_id, bytes) =
        bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
            &bbb_protocol::packets::inventory::ContainerSetSlot {
                container_id: 0,
                state_id: 1,
                slot: 0,
                item: loaded,
            },
        )
        .unwrap();
    let PlayClientbound::ContainerSetSlot(slot) =
        decode_play_clientbound(packet_id, &bytes).unwrap()
    else {
        panic!("slot packet");
    };
    verify_hidden(&slot.item);
}

#[test]
fn tooltip_and_lodestone_recipe_take_keeps_save_coordinates_and_wraps_wire() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
    };
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_registry_codec::item_stack_summary_to_persistent_nbt;

    let set = recipe_set(
        "test:tooltip_lodestone",
        serde_json::json!({
            "type": "minecraft:crafting_shapeless",
            "ingredients": ["minecraft:stone"],
            "result": {
                "id": "minecraft:compass",
                "components": {
                    "minecraft:tooltip_display": {
                        "hide_tooltip": true,
                        "hidden_components": ["minecraft:damage", "minecraft:max_damage"]
                    },
                    "minecraft:lodestone_tracker": {
                        "target": {
                            "dimension": "minecraft:overworld",
                            "pos": [33554432, 2048, -33554433]
                        },
                        "tracked": false
                    }
                }
            }
        }),
    );
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        set,
        ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
        )
        .unwrap());
    let crafted = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        crafted.item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_COMPASS_ID)
    );
    assert_eq!(crafted.count(), 1);
    let assert_full = |stack: &ItemStackSummary| {
        assert!(stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID));
        assert!(stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
        assert!(stack.component_patch.tooltip_hide_tooltip);
        assert_eq!(
            stack.component_patch.tooltip_hidden_component_type_ids,
            [
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
            ]
        );
        assert_eq!(stack.component_patch.lodestone_tracked, Some(false));
        assert_eq!(
            stack.component_patch.lodestone_target.as_ref().unwrap().pos,
            bbb_protocol::packets::chunks::BlockPos {
                x: 33_554_432,
                y: 2_048,
                z: -33_554_433,
            }
        );
    };
    assert_full(crafted.summary());

    let nbt = item_stack_summary_to_persistent_nbt(crafted.summary()).unwrap();
    let components = bbb_protocol::nbt_summary::value_last_field(&nbt, "components").unwrap();
    let lodestone =
        bbb_protocol::nbt_summary::value_last_field(components, "minecraft:lodestone_tracker")
            .unwrap();
    let NbtSummaryValue::Compound(fields) = lodestone else {
        panic!("saved lodestone record");
    };
    let NbtSummaryValue::Compound(target) = &fields
        .iter()
        .find(|field| field.name == "target")
        .unwrap()
        .value
    else {
        panic!("saved target record");
    };
    assert_eq!(
        target
            .iter()
            .find(|field| field.name == "pos")
            .unwrap()
            .value,
        NbtSummaryValue::IntArray(vec![33_554_432, 2_048, -33_554_433])
    );
    let loaded =
        bbb_registry_codec::read_item_stack(&nbt, &bbb_registry_codec::UnavailableHolderResolver)
            .unwrap();
    assert_full(&loaded);

    let (packet_id, bytes) =
        bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
            &bbb_protocol::packets::inventory::ContainerSetSlot {
                container_id: 0,
                state_id: 1,
                slot: 0,
                item: crafted.summary().clone(),
            },
        )
        .unwrap();
    let PlayClientbound::ContainerSetSlot(slot) =
        decode_play_clientbound(packet_id, &bytes).unwrap()
    else {
        panic!("slot packet");
    };
    assert_eq!(
        slot.item.component_patch.tooltip_hidden_component_type_ids,
        [
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID
        ]
    );
    assert_eq!(slot.item.component_patch.lodestone_tracked, Some(false));
    assert_eq!(
        slot.item
            .component_patch
            .lodestone_target
            .as_ref()
            .unwrap()
            .pos,
        bbb_protocol::packets::chunks::BlockPos {
            x: -33_554_432,
            y: -2_048,
            z: 33_554_431,
        }
    );
}

/// `minecraft:flint_and_steel`, laid out so the match sits in the grid's
/// bottom row and `CraftingInput.ofPositioned` (`CraftingInput.java:36-84`)
/// reports a non-zero top.
fn flint_and_steel_menu() -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        recipe_set(
            "minecraft:flint_and_steel",
            serde_json::json!({
                "type": "minecraft:crafting_shaped",
                "key": { "I": "minecraft:iron_ingot", "F": "minecraft:flint" },
                "pattern": ["IF"],
                "result": { "id": "minecraft:flint_and_steel" }
            }),
        ),
        ItemTagIndex::new(),
    )
    .unwrap();
    menu
}

/// `minecraft:sugar_from_honey_bottle`: one ingredient with a crafting
/// remainder (`Items.java:2105`) and a result of more than one.
fn honey_bottle_menu() -> PlayerInventoryMenu {
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        recipe_set(
            "minecraft:sugar_from_honey_bottle",
            serde_json::json!({
                "type": "minecraft:crafting_shapeless",
                "ingredients": ["minecraft:honey_bottle"],
                "result": { "id": "minecraft:sugar", "count": 3 }
            }),
        ),
        ItemTagIndex::new(),
    )
    .unwrap();
    menu
}

#[test]
fn firework_star_menu_take_preserves_explosion_through_save_and_slot_packet() {
    use bbb_protocol::packets::inventory::ContainerSetSlot;
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot;
    use bbb_registry_codec::item_stack_summary_to_persistent_nbt;

    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(crate::inventory::test_item_resolver(), None, recipe_set("test:firework_star", serde_json::json!({
        "type": "minecraft:crafting_special_firework_star",
        "shapes": { "star": "minecraft:stick" },
        "trail": "minecraft:diamond",
        "twinkle": "minecraft:feather",
        "fuel": "minecraft:gunpowder",
        "dye": ["minecraft:red_dye", "minecraft:blue_dye"],
        "result": { "id": "minecraft:firework_star", "components": {
            "minecraft:dyed_color": 123456,
            "minecraft:firework_explosion": { "shape": "burst", "colors": [1], "fade_colors": [2] }
        }}
    })), ItemTagIndex::new()).unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_RED_DYE_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STICK_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            3,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_BLUE_DYE_ID, 1))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            4,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GUNPOWDER_ID, 1))
        )
        .unwrap());
    let result = menu.slot_item(0).into_owned();
    let explosion = result
        .summary()
        .component_patch
        .firework_explosion
        .as_ref()
        .unwrap();
    assert_eq!(explosion.shape, FireworkExplosionShapeSummary::Star);
    assert_eq!(explosion.colors, vec![11_743_532, 2_437_522]);
    assert!(explosion.fade_colors.is_empty());
    assert_eq!(result.summary().component_patch.dyed_color, Some(123456));

    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(taken.item_id(), Some(VANILLA_ITEM_FIREWORK_STAR_ID));
    assert_eq!(
        taken.summary().component_patch.firework_explosion.as_ref(),
        Some(explosion)
    );
    assert!(menu.slot_item(0).is_empty());

    let nbt = item_stack_summary_to_persistent_nbt(taken.summary()).unwrap();
    let saved =
        bbb_registry_codec::read_item_stack(&nbt, &bbb_registry_codec::UnavailableHolderResolver)
            .unwrap();
    assert_eq!(
        saved.component_patch.firework_explosion.as_ref(),
        Some(explosion)
    );
    assert_eq!(saved.component_patch.dyed_color, Some(123456));

    let (id, bytes) = encode_play_clientbound_container_set_slot(&ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 0,
        item: taken.summary().clone(),
    })
    .unwrap();
    let PlayClientbound::ContainerSetSlot(decoded) = decode_play_clientbound(id, &bytes).unwrap()
    else {
        panic!("slot packet");
    };
    assert_eq!(
        decoded.item.component_patch.firework_explosion.as_ref(),
        Some(explosion)
    );
    assert_eq!(decoded.item.component_patch.dyed_color, Some(123456));
}

/// `ResultSlot.onTake` (`:90-98`): one item leaves every occupied cell of the
/// positioned box, whatever it was holding, and the cells outside the box are
/// not walked at all.
#[test]
fn result_take_removes_one_item_from_every_occupied_cell_of_the_positioned_box() {
    let mut menu = flint_and_steel_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            3,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 3))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            4,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 2))
        )
        .unwrap());
    assert_eq!(
        menu.slot_item(0).item_id(),
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );

    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(taken.item_id(), Some(VANILLA_ITEM_FLINT_AND_STEEL_ID));
    assert_eq!(
        *menu.slot_item(3),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 2))
    );
    assert_eq!(
        *menu.slot_item(4),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
    );
    // The empty top row is inside the grid but outside the positioned box.
    assert!(menu.slot_item(1).is_empty());
    assert!(menu.slot_item(2).is_empty());
    // `InventoryMenu.slotsChanged` re-runs over what is left.
    assert_eq!(
        menu.slot_item(0).item_id(),
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );

    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(taken.item_id(), Some(VANILLA_ITEM_FLINT_AND_STEEL_ID));
    assert_eq!(
        *menu.slot_item(3),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_IRON_INGOT_ID, 1))
    );
    assert!(menu.slot_item(4).is_empty());
    assert!(menu.slot_item(0).is_empty());
}

/// `:100-102`: a remainder whose cell the take emptied is written straight
/// back into that cell.
#[test]
fn result_take_writes_a_remainder_into_the_cell_the_take_emptied() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        taken,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
    assert!(menu
        .inventory_slots()
        .iter()
        .all(RuntimeItemStack::is_empty));
    assert!(menu.dropped_from_craft().is_empty());
}

/// `:106`: a remainder the cell will not hold goes to `Inventory.add`
/// (`Inventory.java:254-301`), which fills the first free slot.
#[test]
fn result_take_adds_a_remainder_the_cell_still_holds_an_item_for_to_the_inventory() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 2))
        )
        .unwrap());
    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        taken,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
    );
    assert_eq!(
        menu.inventory_slots()[0],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
    assert!(menu.dropped_from_craft().is_empty());
}

/// `:107`: what `Inventory.add` refuses goes to `Player.drop`
/// (`Player.java:582-584`) instead of back into the grid.
#[test]
fn result_take_drops_a_remainder_a_full_inventory_refuses() {
    let mut menu = honey_bottle_menu();
    for slot in 0..SELECTION_SIZE * 4 {
        menu.set_inventory_slot(
            slot,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 64)),
        );
    }
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 2))
        )
        .unwrap());
    let _ = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
    );
    assert_eq!(
        menu.dropped_from_craft(),
        [DroppedMenuStack {
            stack: RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1)),
            thrown_from_hand: false
        }]
    );
    assert!(menu
        .inventory_slots()
        .iter()
        .all(|item| item.is_empty() || item.item_id() == Some(VANILLA_ITEM_STONE_ID)));
}

/// `ResultSlot.remove` (`ResultSlot.java:30-36`) hands back the whole result
/// however little the click asked for, because `ResultContainer.removeItem`
/// (`ResultContainer.java:37-39`) is `ContainerHelper.takeItem`
/// (`ContainerHelper.java:17-19`) and ignores the amount -- while
/// `checkTakeAchievements` (`:50-53`) awards the stat for `removeCount`, which
/// is only what was asked for.
#[test]
fn a_half_take_of_the_result_hands_back_all_of_it_and_counts_only_the_half() {
    let mut whole = honey_bottle_menu();
    assert!(whole
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    let taken = whole
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        taken,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert_eq!(whole.crafted_count(VANILLA_ITEM_SUGAR_ID), 3);

    // `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:445-450`) asks a secondary click for
    // `(count + 1) / 2`.
    let mut half = honey_bottle_menu();
    assert!(half
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    let taken = half
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            2,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        taken,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert!(half.slot_item(0).is_empty());
    assert_eq!(half.crafted_count(VANILLA_ITEM_SUGAR_ID), 2);
}

/// A repeated take accumulates the stat rather than replacing it, and every
/// craft awards `Stats.ITEM_CRAFTED` for the result's own item
/// (`ItemStack.java:723-726`).
#[test]
fn crafted_counts_accumulate_per_result_item() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 2))
        )
        .unwrap());
    let _ = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    let _ = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(menu.crafted_count(VANILLA_ITEM_SUGAR_ID), 6);
    assert_eq!(menu.crafted_count(VANILLA_ITEM_HONEY_BOTTLE_ID), 0);
}

/// `InventoryMenu.quickMoveStack` (`InventoryMenu.java:109-114`) counts the
/// move through `ResultSlot.onQuickCraft` (`ResultSlot.java:39-42`), so a
/// shift-click awards the whole result.
#[test]
fn a_quick_move_off_the_result_counts_everything_it_moved() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    let mut dropped = Vec::new();
    let moved = menu
        .quick_move_stack(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            false,
            &mut dropped,
        )
        .expect("the fixture carries no catalog lookups");
    assert_eq!(
        moved,
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert_eq!(menu.crafted_count(VANILLA_ITEM_SUGAR_ID), 3);
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
}

/// `AbstractContainerMenu.doClick`'s empty-hotbar swap
/// (`AbstractContainerMenu.java:477-481`) counts the
/// result through `ResultSlot.onSwapCraft` (`ResultSlot.java:45-47`).
#[test]
fn a_swap_off_the_result_counts_everything_it_took() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    let mut dropped = Vec::new();
    menu.do_swap(
        &bbb_registry_codec::UnavailableHolderResolver,
        None,
        0,
        4,
        false,
        &mut dropped,
    )
    .expect("the fixture carries no catalog lookups");
    assert_eq!(
        menu.inventory_slots()[4],
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_SUGAR_ID, 3))
    );
    assert_eq!(menu.crafted_count(VANILLA_ITEM_SUGAR_ID), 3);
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(stack(VANILLA_ITEM_GLASS_BOTTLE_ID, 1))
    );
}

/// `RecipeCraftingHolder.awardUsedRecipes` hands the used recipe to
/// `ServerPlayer.awardRecipes`, which the world runs after the click.
#[test]
fn a_take_queues_the_used_recipe_for_the_book() {
    let mut menu = honey_bottle_menu();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_HONEY_BOTTLE_ID, 1))
        )
        .unwrap());
    assert!(menu.recipe_book().known().next().is_none());
    let _ = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            3,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(
        menu.take_crafted_recipes()
            .into_iter()
            .filter(|craft| craft.unlock)
            .map(|craft| craft.recipe_id)
            .collect::<Vec<_>>(),
        ["minecraft:sugar_from_honey_bottle"]
    );
}

#[test]
fn recipe_book_awards_only_unknown_recipes() {
    let mut book = RecipeBook::default();
    assert!(book.award("test:first"));
    assert!(book.contains("test:first"));
    assert_eq!(book.to_be_displayed().collect::<Vec<_>>(), ["test:first"]);

    assert!(book.remove_highlight("test:first"));
    assert!(!book.award("test:first"));
    assert!(book.to_be_displayed().next().is_none());
}

#[test]
fn recipe_book_removal_clears_known_and_highlight_idempotently() {
    let mut book = RecipeBook::default();
    assert!(book.award("test:first"));
    assert!(book.remove("test:first"));
    assert!(!book.contains("test:first"));
    assert!(book.to_be_displayed().next().is_none());
    assert!(!book.remove("test:first"));

    assert!(book.award("test:seen"));
    assert!(book.remove_highlight("test:seen"));
    assert!(!book.remove_highlight("test:seen"));
    assert!(book.contains("test:seen"));
}

#[test]
fn recipe_book_retain_known_prunes_catalog_stale_entries() {
    let mut book = RecipeBook::default();
    assert!(book.award("test:kept"));
    assert!(book.award("test:stale"));
    assert!(book.remove_highlight("test:kept"));

    book.retain_known(|id| id == "test:kept");

    assert_eq!(book.known().collect::<Vec<_>>(), ["test:kept"]);
    assert!(book.to_be_displayed().next().is_none());
}

/// A banner whose one pattern layer makes it `BannerDuplicateRecipe`'s source
/// (`BannerDuplicateRecipe.java:55-65`).
fn patterned_banner(count: i32) -> ItemStackSummary {
    let mut stack = stack(VANILLA_ITEM_WHITE_BANNER_ID, count);
    stack.component_patch.banner_pattern_layers = vec![BannerPatternLayerSummary {
        registry_id: None,
        asset_id: Some("minecraft:stripe_bottom".to_string()),
        translation_key: None,
        color_id: 15,
    }];
    stack
}

/// `ResultSlot.onTake`'s middle arm (`ResultSlot.java:103-105`): when the
/// remainder is `isSameItemSameComponents` with what the cell kept, vanilla
/// grows the remainder by the cell's count and writes it back, so the stack
/// never leaves the grid.
///
/// `BannerDuplicateRecipe.getRemainingItems` (`BannerDuplicateRecipe.java:104`)
/// is one of the only two crafting recipes whose remainder is a copy of an
/// input rather than a fresh `ItemStackTemplate`, so it is the only shape that
/// can reach this arm.
#[test]
fn result_take_grows_a_remainder_that_matches_what_the_cell_kept() {
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_catalog(
        crate::inventory::test_item_resolver(),
        None,
        recipe_set(
            "minecraft:white_banner_duplicate",
            serde_json::json!({
                "type": "minecraft:crafting_special_bannerduplicate",
                "banner": "minecraft:white_banner",
                "result": { "id": "minecraft:white_banner" }
            }),
        ),
        ItemTagIndex::new(),
    )
    .unwrap();
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            RuntimeItemStack::from_summary(patterned_banner(2))
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_WHITE_BANNER_ID, 1))
        )
        .unwrap());
    assert_eq!(
        menu.slot_item(0).item_id(),
        Some(VANILLA_ITEM_WHITE_BANNER_ID)
    );

    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(taken.item_id(), Some(VANILLA_ITEM_WHITE_BANNER_ID));
    // One banner left the source cell and the remainder grew it straight back.
    assert_eq!(
        *menu.slot_item(1),
        RuntimeItemStack::from_summary(patterned_banner(2))
    );
    // The plain target has no remainder, so its cell simply empties.
    assert!(menu.slot_item(2).is_empty());
    assert!(menu
        .inventory_slots()
        .iter()
        .all(RuntimeItemStack::is_empty));
    assert!(menu.dropped_from_craft().is_empty());
}

/// Nested templates and predicate components survive crafting, save/load and slot packets.
#[test]
fn nested_and_predicate_values_craft_through_frozen_save_and_slot() {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    };
    use bbb_protocol::packets::data_components::fields::component_values_equal;
    use bbb_protocol::packets::play_clientbound::{decode_play_clientbound, PlayClientbound};
    use bbb_registry_codec::{
        decode_direct_registry_json, item_stack_summary_to_persistent_nbt_with_resolver,
    };
    use bbb_server_datapack::recipe::parse_recipe_source_with_lookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    let vanilla = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: bbb_server_datapack::registry_data_loader::DatapackLoadLimits::default(),
    })
    .unwrap();
    let lookup =
        bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(&vanilla);
    let (lock, can_break, can_place_on, container, bundle, charged, remainder) = (
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    );
    let records = r##"{"lock":{"items":"minecraft:stone","count":{"min":1,"max":3},"predicates":{"minecraft:damage":{"durability":{"min":1}}},"components":{"minecraft:damage":2,"minecraft:max_damage":5}},"can_break":[{"blocks":"minecraft:stone","state":{"lit":"true","facing":"north"}},{"blocks":"#minecraft:logs","components":{"minecraft:max_damage":4}}],"can_place_on":{"blocks":["minecraft:dirt","minecraft:grass_block"],"nbt":"{answer:42}"},"container":[{"slot":2,"item":{"id":"minecraft:dirt","count":3,"components":{"minecraft:repair_cost":1}}},{"slot":0,"item":"minecraft:stone"}],"use_remainder":{"id":"minecraft:bowl","components":{"!minecraft:max_stack_size":{}}}}"##;
    let stacks = r#"{"bundle_contents":[{"id":"minecraft:arrow","count":2},"minecraft:stick"],"charged_projectiles":[{"id":"minecraft:arrow","components":{"minecraft:repair_cost":2}}]}"#;
    let constructed = r#"{"lock":{"predicates":{"minecraft:max_damage":{}}},"can_place_on":{"predicates":{"minecraft:enchantments":[{"levels":1}]}}}"#;
    for (result, components, type_ids, reload_equal) in [
        (
            "chest",
            records,
            &[lock, can_break, can_place_on, container, remainder][..],
            true,
        ),
        ("bundle", stacks, &[bundle, charged][..], true),
        ("chest", constructed, &[lock, can_place_on][..], true),
    ] {
        let body = format!(
            r#"{{"type":"crafting_shapeless","ingredients":["stone"],"result":{{"id":"{result}","components":{components}}}}}"#
        );
        let decoded = decode_direct_registry_json(body.as_bytes()).unwrap();
        let recipe = parse_recipe_source_with_lookup(decoded.root(), &lookup).unwrap();
        let mut menu = PlayerInventoryMenu::empty();
        menu.install_crafting_catalog(
            crate::inventory::test_item_resolver(),
            None,
            std::sync::Arc::new(RecipeSet {
                recipes: vec![RecipeElement {
                    id: "test:nested".to_string(),
                    source_pack: "test".to_string(),
                    recipe,
                }],
                warnings: Vec::new(),
            }),
            ItemTagIndex::new(),
        )
        .unwrap();
        let mut take = || {
            assert!(menu
                .set_by_player(
                    crate::inventory::test_item_resolver(),
                    None,
                    1,
                    RuntimeItemStack::from_summary(stack(VANILLA_ITEM_STONE_ID, 1))
                )
                .unwrap());
            menu.slot_safe_take(
                &bbb_registry_codec::UnavailableHolderResolver,
                None,
                0,
                1,
                CARRIED_SLOT_SIZE,
                false,
            )
            .unwrap()
        };
        let crafted = take();
        let again = take();
        assert!(crafted.item_id().is_some());
        assert_eq!(
            crafted.summary().component_patch.added_type_ids.len(),
            type_ids.len()
        );
        // Both takes copy the one template value.
        assert!(crafted.summary().component_patch == again.summary().component_patch);
        let raw =
            item_stack_summary_to_persistent_nbt_with_resolver(crafted.summary(), &lookup).unwrap();
        let saved = raw.clone();
        let loaded = bbb_registry_codec::read_item_stack(&saved, &lookup).unwrap();
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: loaded.clone(),
                },
            )
            .unwrap();
        let PlayClientbound::ContainerSetSlot(slot) =
            decode_play_clientbound(packet_id, &bytes).unwrap()
        else {
            panic!("slot packet");
        };
        let reloaded =
            item_stack_summary_to_persistent_nbt_with_resolver(&loaded, &lookup).unwrap();
        assert_eq!(
            item_stack_summary_to_persistent_nbt_with_resolver(&slot.item, &lookup).unwrap(),
            reloaded
        );
        for item in [&loaded, &slot.item] {
            for &type_id in type_ids {
                assert!(item.component_patch.added_type_ids.contains(&type_id));
                assert_eq!(
                    component_values_equal(
                        type_id,
                        &crafted.summary().component_patch,
                        &item.component_patch
                    )
                    .unwrap(),
                    reload_equal,
                    "{result} {type_id}"
                );
            }
        }
        if reload_equal {
            assert_eq!(reloaded, raw);
            assert!(loaded.component_patch == slot.item.component_patch);
        }
    }
}

#[test]
fn native_publication_keeps_template_and_banner_remainder_first_through_player_and_crafter() {
    use bbb_registry_codec::decode_direct_registry_json;
    use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
    use bbb_server_datapack::recipe::parse_recipe_source_with_lookup;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
    let recipes = [
        ("test:template", r#"{"type":"crafting_shapeless","ingredients":["stone"],"result":{"id":"chest","components":{"lock":{"predicates":{"minecraft:max_damage":{}}}}}}"#),
        ("test:banner", r#"{"type":"crafting_special_bannerduplicate","banner":"white_banner","result":{"id":"white_banner"}}"#),
    ].into_iter().map(|(id, source)| {
        let json = decode_direct_registry_json(source.as_bytes()).unwrap();
        RecipeElement {
            id: id.into(),
            source_pack: "test".into(),
            recipe: parse_recipe_source_with_lookup(json.root(), &lookup).unwrap(),
        }
    }).collect();
    startup.recipes = std::sync::Arc::new(RecipeSet {
        recipes,
        warnings: Vec::new(),
    });
    let startup = std::sync::Arc::new(startup);
    let tags = ItemTagIndex::from_loaded(startup.tags.registry("minecraft:item").unwrap());
    let fresh = |summary| RuntimeItemStack::from_summary(summary);
    let mut menu = PlayerInventoryMenu::empty();
    menu.install_crafting_publication(
        crate::inventory::test_item_resolver(),
        None,
        std::sync::Arc::clone(&startup),
        tags,
    )
    .unwrap();
    let stone = fresh(stack(VANILLA_ITEM_STONE_ID, 1));
    let mut preview_grid = vec![RuntimeItemStack::empty(); 9];
    preview_grid[4] = stone.clone();
    let preview = menu
        .crafting_preview(
            crate::inventory::test_item_resolver(),
            None,
            preview_grid.clone(),
        )
        .unwrap();
    let preview_again = menu
        .crafting_preview(crate::inventory::test_item_resolver(), None, preview_grid)
        .unwrap();
    assert!(preview.same_item_same_components(&preview_again));
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            stone.clone()
        )
        .unwrap());
    let first = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert!(menu
        .set_by_player(crate::inventory::test_item_resolver(), None, 1, stone)
        .unwrap());
    let second = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert!(first.same_item_same_components(&second));
    assert!(first.same_item_same_components(&preview));

    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            fresh(stack(VANILLA_ITEM_STONE_ID, 2))
        )
        .unwrap());
    let held_result = menu.slot_item(0).into_owned();
    let result_before = menu.slot_item(0).into_owned();
    let mut reloaded = (*startup).clone();
    std::sync::Arc::make_mut(&mut reloaded.recipes)
        .recipes
        .clear();
    menu.reload_crafting_publication(std::sync::Arc::new(reloaded), ItemTagIndex::new());
    assert!(menu.slot_item(0).same_item_same_components(&held_result));
    let taken = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert_eq!(taken, result_before);
    assert_eq!(menu.slot_item(1).count(), 2);
    assert!(menu.slot_item(0).is_empty());
    menu.reload_crafting_publication(std::sync::Arc::clone(&startup), ItemTagIndex::new());

    let mut banner = patterned_banner(2);
    banner
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID);
    banner.component_patch.banner_pattern_layers[0].translation_key = Some("test.pattern".into());
    let source = fresh(banner);
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            1,
            source.clone()
        )
        .unwrap());
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            2,
            fresh(stack(VANILLA_ITEM_WHITE_BANNER_ID, 1))
        )
        .unwrap());
    let duplicated = menu
        .slot_safe_take(
            &bbb_registry_codec::UnavailableHolderResolver,
            None,
            0,
            1,
            CARRIED_SLOT_SIZE,
            false,
        )
        .unwrap();
    assert!(duplicated.same_item_same_components(&source));
    assert!(menu.slot_item(1).same_item_same_components(&source));
    assert_eq!(menu.slot_item(1).count(), 2);
    assert!(menu.slot_item(2).is_empty());
    assert!(menu.dropped_from_craft().is_empty());
}

/// `RecipeCraftingHolder.setRecipeUsed` (`RecipeCraftingHolder.java:28-35`):
/// under `limited_crafting` only a recipe the book knows yields a result.
#[test]
fn limited_crafting_needs_the_recipe_in_the_book() {
    let mut menu = flint_and_steel_menu();
    menu.set_limited_crafting(true);
    for (slot, item) in [(3, VANILLA_ITEM_IRON_INGOT_ID), (4, VANILLA_ITEM_FLINT_ID)] {
        assert!(menu
            .set_by_player(
                crate::inventory::test_item_resolver(),
                None,
                slot,
                RuntimeItemStack::from_summary(stack(item, 1))
            )
            .unwrap());
    }
    assert!(menu.slot_item(0).is_empty());

    menu.recipe_book_mut().award("minecraft:flint_and_steel");
    assert!(menu
        .set_by_player(
            crate::inventory::test_item_resolver(),
            None,
            4,
            RuntimeItemStack::from_summary(stack(VANILLA_ITEM_FLINT_ID, 1))
        )
        .unwrap());
    assert_eq!(
        menu.slot_item(0).item_id(),
        Some(VANILLA_ITEM_FLINT_AND_STEEL_ID)
    );
}
