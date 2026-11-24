use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
};
use bbb_facts::attribute_types::vanilla_attribute_type_for_resource_id;
use bbb_facts::block_types::vanilla_block_type_for_resource_id;
use bbb_facts::builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID;
use bbb_facts::data_component_types::*;
use bbb_facts::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID;
use bbb_facts::item_component_value_types::{ItemRaritySummary, ItemUseAnimationSummary};
use bbb_facts::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_STONE_ID};
use bbb_facts::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_facts::potion_types::vanilla_potion_for_resource_id;
use bbb_facts::static_registry_elements::vanilla_static_registry_elements;
use bbb_facts::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_protocol::packets::{
    AdventureModePredicateDataSummary, AttributeModifierSummary, BannerPatternLayerSummary,
    BlockPredicateSummary, BlocksAttacksSummary, ConsumableSummary, ConsumeEffectSummary,
    DamageReductionSummary, DataComponentPatchSummary, DataComponentPredicateTypeSummary,
    EquippableSummary, GameProfilePropertySummary, HashedStack, HolderSetSummary,
    InstrumentComponentSummary, InstrumentSummary, ItemDamageFunctionSummary,
    ItemEnchantmentSummary, ItemStackSummary, JukeboxSongSummary, KineticWeaponConditionSummary,
    KineticWeaponSummary, MobEffectDetailsSummary, MobEffectInstanceSummary,
    PaintingVariantSummary, PartialDataComponentPredicateSummary,
    PersistentDataComponentValueSummary, PersistentItemPredicateSummary, PiercingWeaponSummary,
    PlayerModelTypeSummary, PlayerSkinPatchSummary, PotionContentsSummary,
    ResolvableProfileKindSummary, ResolvableProfileSummary, ResourceTextureSummary,
    SoundEventSummary, StatePropertyMatcherSummary, StatePropertyValueMatcherSummary,
    SuspiciousStewEffectSummary, TrimMaterialSummary, TrimPatternSummary, UseEffectsSummary,
};
use bbb_protocol::{ComponentStyle, JavaString, NbtSummaryEntry, NbtSummaryValue, StyledTextRun};

use super::*;
use crate::persistent_item_stack_projection::PersistentItemStackHolderResolver;

#[derive(Default)]
struct TestLookup {
    registries: BTreeMap<(&'static str, i32), String>,
    reverse: BTreeMap<(&'static str, String), i32>,
    tags: BTreeMap<(&'static str, String), Vec<String>>,
}

impl TestLookup {
    fn insert(&mut self, registry: &'static str, id: i32, resource_id: &str) {
        self.registries
            .insert((registry, id), resource_id.to_string());
        self.reverse.insert((registry, resource_id.to_string()), id);
    }

    fn insert_tag(&mut self, registry: &'static str, tag: &str, members: &[&str]) {
        self.tags.insert(
            (registry, tag.to_string()),
            members.iter().map(|s| (*s).to_string()).collect(),
        );
    }
}

impl PersistentRegistryResolver for TestLookup {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if self
            .reverse
            .keys()
            .any(|(r, id)| *r == registry_id && id == resource_id)
        {
            PersistentRegistryMembership::Present
        } else {
            UnavailableHolderResolver.resource_membership(registry_id, resource_id)
        }
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        if let Some((_, members)) = self
            .tags
            .iter()
            .find(|((r, t), _)| *r == registry_id && t == tag_id)
        {
            return PersistentRegistryTagMembers::Present(members.clone());
        }
        UnavailableHolderResolver.tag_members(registry_id, tag_id)
    }
}

impl PersistentItemStackHolderResolver for TestLookup {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.reverse
            .iter()
            .find(|((r, id), _)| *r == registry_id && id == resource_id)
            .map(|(_, id)| *id)
            .or_else(|| match registry_id {
                "minecraft:item" => vanilla_item_type_for_resource_id(resource_id).map(|e| e.id),
                "minecraft:block" => vanilla_block_type_for_resource_id(resource_id).map(|e| e.id),
                "minecraft:mob_effect" => {
                    vanilla_mob_effect_type_for_resource_id(resource_id).map(|e| e.id)
                }
                "minecraft:potion" => vanilla_potion_for_resource_id(resource_id).map(|e| e.id),
                "minecraft:attribute" => {
                    vanilla_attribute_type_for_resource_id(resource_id).map(|e| e.id)
                }
                _ => None,
            })
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.registries
            .iter()
            .find(|((r, id), _)| *r == registry_id && *id == holder_id)
            .map(|(_, s)| s.clone())
    }
}

impl ItemHashLookup for TestLookup {}

fn sound_event_id(resource_id: &str) -> Option<i32> {
    vanilla_static_registry_elements("minecraft:sound_event")?
        .iter()
        .position(|entry| *entry == resource_id)
        .map(|idx| idx as i32)
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

#[test]
fn crc32c_and_hash_ops_primitives_match_guava_26_1() {
    assert_eq!(crc32c(b"123456789"), 0xe306_9283);
    assert_eq!(hash_ops_crc32c_int(7), -1_726_626_450);

    let unpaired = JavaString::from_json_string(r#""\ud800""#).unwrap();
    assert_eq!(
        hash_java_string(&unpaired),
        crc32c(&[HASH_TAG_STRING, 1, 0, 0, 0, 0x00, 0xd8])
    );

    // Unsigned 32-bit `HashCode::padToLong` ordering matches Java Guava HashOps.
    let low_key = hash_int(0);
    let high_key = hash_string("z");
    let ordered = hash_map(vec![(high_key, hash_int(2)), (low_key, hash_int(1))]);
    let expected = if low_key < high_key {
        [low_key, hash_int(1), high_key, hash_int(2)]
    } else {
        [high_key, hash_int(2), low_key, hash_int(1)]
    };
    let mut raw = vec![HASH_TAG_MAP_START];
    for word in expected {
        raw.extend_from_slice(&word.to_le_bytes());
    }
    raw.push(HASH_TAG_MAP_END);
    assert_eq!(ordered, crc32c(&raw));
}

#[test]
fn hashes_primitive_unit_enum_and_rejects_transient_components() {
    let lookup = TestLookup::default();
    let custom_data = NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Int(7))]);
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        ],
        custom_data: Some(custom_data.clone()),
        unbreakable: true,
        use_effects: Some(UseEffectsSummary {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier: 0.2,
        }),
        item_model: Some("minecraft:diamond_sword".to_string()),
        rarity: Some(ItemRaritySummary::Rare),
        enchantment_glint_override: Some(true),
        dye_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
        ..DataComponentPatchSummary::default()
    };

    let hashed = hashed_component_patch_from_summary(&lookup, &patch).unwrap();
    assert_eq!(
        hashed.added_components,
        BTreeMap::from([
            (
                VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                hash_nbt(&custom_data, false) as i32
            ),
            (
                VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
                hash_empty_map() as i32
            ),
            (
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                hash_empty_map() as i32
            ),
            (
                VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
                hash_string("minecraft:diamond_sword") as i32
            ),
            (
                VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
                hash_string("rare") as i32
            ),
            (
                VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
                hash_boolean(true) as i32
            ),
            (
                VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                hash_string("red") as i32
            ),
        ])
    );

    for (type_id, transient) in [
        (
            VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID],
                creative_slot_lock: true,
                ..DataComponentPatchSummary::default()
            },
        ),
        (
            VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID],
                additional_trade_cost: Some(3),
                ..DataComponentPatchSummary::default()
            },
        ),
        (
            VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
            DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
        ),
    ] {
        assert_eq!(hash_component_value(&lookup, &transient, type_id), None);
        assert_eq!(
            hashed_component_patch_from_summary(&lookup, &transient),
            None
        );
    }
}

#[test]
fn hashes_food_consumable_and_attack_codecs_against_official_26_1_vectors() {
    let lookup = TestLookup::default();
    let speed_id = vanilla_mob_effect_type_for_resource_id("minecraft:speed")
        .unwrap()
        .id;
    let poison_id = vanilla_mob_effect_type_for_resource_id("minecraft:poison")
        .unwrap()
        .id;
    let drink_sound = sound_event_id("minecraft:entity.generic.drink").unwrap();

    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        ],
        food_nutrition: Some(7),
        food_saturation_bits: Some((-0.0_f32).to_bits()),
        food_can_always_eat: true,
        consumable: Some(ConsumableSummary {
            consume_seconds: 0.375,
            animation: ItemUseAnimationSummary::Drink,
        }),
        consumable_sound: Some(SoundEventSummary {
            registry_id: Some(drink_sound),
            sound_id: None,
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(false),
        consumable_effects: vec![
            ConsumeEffectSummary::ApplyEffects {
                effects: vec![
                    MobEffectInstanceSummary {
                        effect_id: speed_id,
                        amplifier: 200,
                        duration: 321,
                        ambient: true,
                        show_particles: false,
                        show_icon: true,
                        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                            amplifier: 7,
                            duration: 42,
                            ambient: false,
                            show_particles: true,
                            show_icon: false,
                            hidden_effect: None,
                        })),
                    },
                    MobEffectInstanceSummary {
                        effect_id: poison_id,
                        amplifier: 1,
                        duration: -1,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    },
                ],
                probability_bits: 0.375_f32.to_bits(),
            },
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![poison_id, speed_id],
                effect_tag: None,
            },
            ConsumeEffectSummary::ClearAllEffects,
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 8.5_f32.to_bits(),
            },
            ConsumeEffectSummary::PlaySound {
                sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("bbb:consume".into()),
                    fixed_range_bits: Some(24.5_f32.to_bits()),
                },
            },
        ],
        blocks_attacks: Some(BlocksAttacksSummary {
            block_delay_seconds_bits: 0.375_f32.to_bits(),
            disable_cooldown_scale_bits: 1.25_f32.to_bits(),
            damage_reductions: vec![
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 135.0_f32.to_bits(),
                    damage_types: None,
                    base_bits: (-0.0_f32).to_bits(),
                    factor_bits: 0.625_f32.to_bits(),
                },
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 45.0_f32.to_bits(),
                    damage_types: None,
                    base_bits: 2.5_f32.to_bits(),
                    factor_bits: 0.25_f32.to_bits(),
                },
            ],
            item_damage: ItemDamageFunctionSummary {
                threshold_bits: 4.0_f32.to_bits(),
                base_bits: (-0.0_f32).to_bits(),
                factor_bits: 0.75_f32.to_bits(),
            },
            bypassed_by: None,
            block_sound: None,
            disable_sound: None,
        }),
        kinetic_weapon: Some(KineticWeaponSummary {
            contact_cooldown_ticks: 17,
            delay_ticks: 2,
            dismount_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: 11,
                min_speed_bits: (-0.0_f32).to_bits(),
                min_relative_speed_bits: 0.25_f32.to_bits(),
            }),
            knockback_conditions: None,
            damage_conditions: None,
            forward_movement_bits: (-0.0_f32).to_bits(),
            damage_multiplier_bits: 1.5_f32.to_bits(),
            sound: None,
            hit_sound: None,
        }),
        piercing_weapon: Some(PiercingWeaponSummary {
            deals_knockback: true,
            dismounts: false,
            sound: None,
            hit_sound: None,
        }),
        ..DataComponentPatchSummary::default()
    };

    let hashed = hashed_component_patch_from_summary(&lookup, &patch).unwrap();
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID] as u32,
        0x9758_c10d
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID] as u32,
        0x507c_982f
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID] as u32,
        0xfea9_40b3
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID] as u32,
        0xab78_a599
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID] as u32,
        hash_empty_map()
    );
}

#[test]
fn hashes_registry_holders_inline_records_and_rejects_direct_fixed_holders() {
    let mut lookup = TestLookup::default();
    lookup.insert("minecraft:enchantment", 0, "example:sharp");
    lookup.insert("minecraft:damage_type", 0, "minecraft:arrow");
    lookup.insert("minecraft:damage_type", 1, "minecraft:cactus");

    let desc = |text: &str| {
        (
            text.to_string(),
            Some(
                vec![StyledTextRun {
                    text: text.to_string(),
                    style: ComponentStyle::default(),
                }]
                .into_boxed_slice(),
            ),
            Some(Box::new(NbtSummaryValue::Compound(vec![entry(
                "text",
                NbtSummaryValue::string(text),
            )]))),
        )
    };
    let (inst_t, inst_s, inst_n) = desc("Signal horn");
    let (mat_t, mat_s, mat_n) = desc("Opal");
    let (pat_t, pat_s, pat_n) = desc("Spiral");

    let material = TrimMaterialSummary {
        asset_name: "opal".to_string(),
        override_armor_assets: BTreeMap::from([("example:steel".into(), "opal_dark".into())]),
        description: mat_t,
        description_styled: mat_s,
        description_nbt: mat_n,
    };
    let pattern = TrimPatternSummary {
        asset_id: "example:spiral".to_string(),
        description: pat_t,
        description_styled: pat_s,
        description_nbt: pat_n,
        decal: true,
    };
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
            VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        ],
        damage_type_id: Some(0),
        enchantments: vec![ItemEnchantmentSummary {
            holder_id: 0,
            level: 3,
        }],
        damage_resistant_type_registry_ids: vec![0, 1],
        repairable_item_registry_ids: vec![VANILLA_ITEM_STONE_ID],
        instrument: InstrumentComponentSummary {
            direct: Some(Box::new(InstrumentSummary {
                sound_event: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:signal".into()),
                    fixed_range_bits: Some(12.0_f32.to_bits()),
                },
                use_duration_bits: 7.0_f32.to_bits(),
                range_bits: 256.0_f32.to_bits(),
                description: inst_t,
                description_styled: inst_s,
                description_nbt: inst_n,
            })),
            ..Default::default()
        },
        armor_trim_material_direct: Some(Box::new(material)),
        armor_trim_pattern_direct: Some(Box::new(pattern)),
        banner_pattern_layers: vec![BannerPatternLayerSummary {
            registry_id: None,
            asset_id: Some("example:wave".to_string()),
            translation_key: Some("block.example.banner.wave".into()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
        }],
        ..DataComponentPatchSummary::default()
    };

    let hashed = hashed_component_patch_from_summary(&lookup, &patch).unwrap();
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
        hash_string("minecraft:arrow") as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
        hash_map(vec![(hash_string("example:sharp"), hash_int(3))]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID],
        hash_record([(
            "types",
            hash_list([
                hash_string("minecraft:arrow"),
                hash_string("minecraft:cactus")
            ])
        )]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID],
        hash_record([("items", hash_string("minecraft:stone"))]) as i32
    );
    let inst_expected = hash_record([
        (
            "sound_event",
            hash_record([
                ("sound_id", hash_string("example:signal")),
                ("range", hash_float(12.0_f32.to_bits())),
            ]),
        ),
        ("use_duration", hash_float(7.0_f32.to_bits())),
        ("range", hash_float(256.0_f32.to_bits())),
        ("description", hash_string("Signal horn")),
    ]);
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
        inst_expected as i32
    );

    let direct_jukebox = DataComponentPatchSummary {
        jukebox_direct_song: Some(Box::new(JukeboxSongSummary {
            sound_event: SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:song".into()),
                fixed_range_bits: None,
            },
            description: "Song".into(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::string("Song"))),
            length_in_seconds_bits: 10.0_f32.to_bits(),
            comparator_output: 1,
        })),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        hash_component_value(
            &lookup,
            &direct_jukebox,
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID
        ),
        None
    );
    let direct_painting = DataComponentPatchSummary {
        painting_variant_direct: Some(PaintingVariantSummary {
            width: 1,
            height: 1,
            asset_id: "example:painting".into(),
            title: None,
            title_styled: None,
            title_nbt: None,
            author: None,
            author_styled: None,
            author_nbt: None,
        }),
        ..DataComponentPatchSummary::default()
    };
    assert_eq!(
        hash_component_value(
            &lookup,
            &direct_painting,
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID
        ),
        None
    );
}

#[test]
fn hashes_rich_components_and_resolvable_profile() {
    let mut lookup = TestLookup::default();
    lookup.insert("minecraft:dialog", 0, "example:notice");

    let rich = NbtSummaryValue::Compound(vec![
        entry("translate", NbtSummaryValue::string("example.key")),
        entry(
            "with",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Byte(-1),
                NbtSummaryValue::Short(-2),
                NbtSummaryValue::Int(-3),
                NbtSummaryValue::Long(-4),
                NbtSummaryValue::Float(1.5_f32.to_bits()),
                NbtSummaryValue::Double(2.5_f64.to_bits()),
            ]),
        ),
        entry("bold", NbtSummaryValue::Byte(1)),
        entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::string("custom")),
                entry("id", NbtSummaryValue::string("example:test")),
                entry(
                    "payload",
                    NbtSummaryValue::Compound(vec![
                        entry("bytes", NbtSummaryValue::ByteArray(vec![-1, 2])),
                        entry("ints", NbtSummaryValue::IntArray(vec![-3, 4])),
                        entry("longs", NbtSummaryValue::LongArray(vec![-5, 6])),
                    ]),
                ),
            ]),
        ),
    ]);
    assert_eq!(
        hash_component(&lookup, &rich),
        Some(hash_record([
            ("translate", hash_string("example.key")),
            (
                "with",
                hash_list([
                    hash_byte(-1),
                    hash_short(-2),
                    hash_int(-3),
                    hash_long(-4),
                    hash_float(1.5_f32.to_bits()),
                    hash_double(2.5_f64.to_bits()),
                ]),
            ),
            ("bold", hash_boolean(true)),
            (
                "click_event",
                hash_record([
                    ("action", hash_string("custom")),
                    ("id", hash_string("example:test")),
                    (
                        "payload",
                        hash_record([
                            ("bytes", hash_byte_array(&[-1, 2])),
                            ("ints", hash_int_array([-3, 4])),
                            ("longs", hash_long_array(&[-5, 6])),
                        ]),
                    ),
                ]),
            ),
        ]))
    );

    let show_item = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Hover")),
        entry(
            "hover_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::string("show_item")),
                entry("id", NbtSummaryValue::string("stone")),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry("repair_cost", NbtSummaryValue::Int(7))]),
                ),
            ]),
        ),
    ]);
    assert_eq!(
        hash_component(&lookup, &show_item),
        Some(hash_record([
            ("text", hash_string("Hover")),
            (
                "hover_event",
                hash_record([
                    ("action", hash_string("show_item")),
                    ("id", hash_string("minecraft:stone")),
                    (
                        "components",
                        hash_map(vec![(hash_string("minecraft:repair_cost"), hash_int(7))]),
                    ),
                ]),
            ),
        ]))
    );

    let named_dialog = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Open")),
        entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::string("show_dialog")),
                entry("dialog", NbtSummaryValue::string("example:notice")),
            ]),
        ),
    ]);
    assert_eq!(
        hash_component(&lookup, &named_dialog),
        Some(hash_record([
            ("text", hash_string("Open")),
            (
                "click_event",
                hash_record([
                    ("action", hash_string("show_dialog")),
                    ("dialog", hash_string("example:notice")),
                ]),
            ),
        ]))
    );

    let direct_dialog = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Open")),
        entry(
            "click_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::string("show_dialog")),
                entry(
                    "dialog",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::string("notice")),
                        entry("title", NbtSummaryValue::string("Direct")),
                    ]),
                ),
            ]),
        ),
    ]);
    assert_eq!(hash_component(&lookup, &direct_dialog), None);

    let mut profile = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::Partial,
        uuid: Some(uuid::Uuid::from_u128(
            0x1234_5678_9abc_def0_8000_0000_ffff_ffff,
        )),
        name: Some("Alex".to_string()),
        properties: vec![GameProfilePropertySummary {
            name: "textures".into(),
            value: "skin-value".into(),
            signature: Some("skin-signature".into()),
        }],
        profile_textures: None,
        skin_patch: PlayerSkinPatchSummary {
            body: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/wide/alex".into(),
                texture_path: "minecraft:textures/entity/player/wide/alex.png".into(),
            }),
            cape: None,
            elytra: Some(ResourceTextureSummary {
                asset_id: "minecraft:entity/player/elytra".into(),
                texture_path: "minecraft:textures/entity/player/elytra.png".into(),
            }),
            model: Some(PlayerModelTypeSummary::Slim),
        },
    };
    let expected_profile = hash_record([
        (
            "id",
            hash_int_array([
                0x1234_5678,
                0x9abc_def0_u32 as i32,
                0x8000_0000_u32 as i32,
                -1,
            ]),
        ),
        ("name", hash_string("Alex")),
        (
            "properties",
            hash_list([hash_record([
                ("name", hash_string("textures")),
                ("value", hash_string("skin-value")),
                ("signature", hash_string("skin-signature")),
            ])]),
        ),
        ("texture", hash_string("minecraft:entity/player/wide/alex")),
        ("elytra", hash_string("minecraft:entity/player/elytra")),
        ("model", hash_string("slim")),
    ]);
    assert_eq!(hash_resolvable_profile(&profile), Some(expected_profile));
    profile.name = Some("bad name".to_string());
    assert_eq!(hash_resolvable_profile(&profile), None);
}

#[test]
fn hashes_adventure_mode_lock_and_component_predicates() {
    let mut lookup = TestLookup::default();
    let stone_block_id = vanilla_block_type_for_resource_id("minecraft:stone")
        .unwrap()
        .id;
    lookup.insert_tag(
        "minecraft:block",
        "minecraft:mineable/pickaxe",
        &["minecraft:stone"],
    );

    let partial_lock = PartialDataComponentPredicateSummary {
        predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        ),
        wire_value: None,
    };
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        ],
        can_place_on_persistent: Some(AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                blocks: Some(HolderSetSummary {
                    registry_ids: vec![stone_block_id],
                    tag: None,
                }),
                state: Some(vec![
                    StatePropertyMatcherSummary {
                        name: "facing".into(),
                        value: StatePropertyValueMatcherSummary::Exact("north".into()),
                    },
                    StatePropertyMatcherSummary {
                        name: "age".into(),
                        value: StatePropertyValueMatcherSummary::Range {
                            min: Some("1".into()),
                            max: Some("3".into()),
                        },
                    },
                ]),
                nbt: Some(NbtSummaryValue::Compound(vec![
                    entry("name", NbtSummaryValue::string("a\"b")),
                    entry("z", NbtSummaryValue::Int(2)),
                ])),
                exact_components: Some(Box::new(DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                    max_stack_size: Some(16),
                    ..DataComponentPatchSummary::default()
                })),
                partial_components: vec![partial_lock.clone()],
            }],
        }),
        can_break_persistent: Some(AdventureModePredicateDataSummary {
            predicates: vec![
                BlockPredicateSummary::default(),
                BlockPredicateSummary {
                    blocks: Some(HolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:mineable/pickaxe".to_string()),
                    }),
                    ..BlockPredicateSummary::default()
                },
            ],
        }),
        lock_predicate: Some(PersistentItemPredicateSummary {
            items: Some(HolderSetSummary {
                registry_ids: vec![VANILLA_ITEM_STONE_ID],
                tag: None,
            }),
            count_min: Some(1),
            count_max: Some(3),
            exact_components: Vec::new(),
            partial_components: vec![partial_lock],
        }),
        ..DataComponentPatchSummary::default()
    };

    let partial_hash = hash_map(vec![(hash_string("minecraft:lock"), hash_empty_map())]);
    let hashed = hashed_component_patch_from_summary(&lookup, &patch).unwrap();
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
        hash_record([
            ("blocks", hash_string("minecraft:stone")),
            (
                "state",
                hash_map(vec![
                    (hash_string("facing"), hash_string("north")),
                    (
                        hash_string("age"),
                        hash_record([("min", hash_string("1")), ("max", hash_string("3"))]),
                    ),
                ]),
            ),
            (
                "components",
                hash_map(vec![(
                    hash_string("minecraft:max_stack_size"),
                    hash_int(16)
                )]),
            ),
            ("nbt", hash_string("{name:'a\"b',z:2}")),
            ("predicates", partial_hash),
        ]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID],
        hash_list([
            hash_empty_map(),
            hash_record([("blocks", hash_string("#minecraft:mineable/pickaxe"))]),
        ]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_LOCK_TYPE_ID],
        hash_record([
            ("items", hash_string("minecraft:stone")),
            (
                "count",
                hash_record([("min", hash_int(1)), ("max", hash_int(3))]),
            ),
            ("predicates", partial_hash),
        ]) as i32
    );

    let invalid_lock = PersistentItemPredicateSummary {
        count_min: Some(2),
        count_max: Some(2),
        exact_components: vec![PersistentDataComponentValueSummary {
            component_type_id: VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
            value: NbtSummaryValue::Int(0),
        }],
        ..PersistentItemPredicateSummary::default()
    };
    assert_eq!(hash_persistent_item_predicate(&lookup, &invalid_lock), None);

    let fireworks_pred = NbtSummaryValue::Compound(vec![
        entry(
            "explosions",
            NbtSummaryValue::Compound(vec![
                entry(
                    "contains",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                        "shape",
                        NbtSummaryValue::string("star"),
                    )])]),
                ),
                entry(
                    "count",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry(
                            "test",
                            NbtSummaryValue::Compound(vec![entry(
                                "has_trail",
                                NbtSummaryValue::Byte(1),
                            )]),
                        ),
                        entry("count", NbtSummaryValue::Int(1)),
                    ])]),
                ),
            ]),
        ),
        entry("flight_duration", NbtSummaryValue::Int(2)),
    ]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_FIREWORKS_TYPE_ID,
            &fireworks_pred
        ),
        Some(hash_record([
            (
                "explosions",
                hash_record([
                    (
                        "contains",
                        hash_list([hash_record([("shape", hash_string("star"))])]),
                    ),
                    (
                        "count",
                        hash_list([hash_record([
                            ("test", hash_record([("has_trail", hash_boolean(true))])),
                            ("count", hash_int(1)),
                        ])]),
                    ),
                ]),
            ),
            ("flight_duration", hash_int(2)),
        ]))
    );
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID,
            &NbtSummaryValue::Compound(vec![entry("unknown", NbtSummaryValue::Int(1))]),
        ),
        None
    );

    let written_pred = NbtSummaryValue::Compound(vec![
        entry("author", NbtSummaryValue::string("Alex")),
        entry("resolved", NbtSummaryValue::Byte(1)),
    ]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_WRITTEN_BOOK_TYPE_ID,
            &written_pred,
        ),
        Some(hash_record([
            ("author", hash_string("Alex")),
            ("resolved", hash_boolean(true)),
        ]))
    );

    let mut invalid_attr_lookup = TestLookup::default();
    invalid_attr_lookup.insert("minecraft:attribute", 0, "minecraft:attack_damage");
    let attr_id = vanilla_attribute_type_for_resource_id("minecraft:attack_damage")
        .unwrap()
        .id;
    let attr_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID],
        attribute_modifiers: vec![
            AttributeModifierSummary {
                attribute_id: attr_id,
                modifier_id: "example:hidden".into(),
                amount_bits: 1.5_f64.to_bits(),
                operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id: attr_id,
                modifier_id: "example:override".into(),
                amount_bits: 2.0_f64.to_bits(),
                operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                display_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some("Override".into()),
                display_text_styled: None,
                display_text_nbt: Some(NbtSummaryValue::string("Override")),
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    assert!(hashed_component_patch_from_summary(&invalid_attr_lookup, &attr_patch).is_some());
}

#[test]
fn hashed_stack_create_and_matches_enforces_exact_item_count_and_patch() {
    let mut lookup = TestLookup::default();
    let mod_item_id = 5000;
    lookup.insert("minecraft:item", mod_item_id, "modded:crystal_shard");

    let empty = ItemStackSummary::empty();
    assert_eq!(
        hashed_stack_from_summary(&lookup, &empty),
        Some(HashedStack::Empty)
    );
    assert!(hashed_stack_matches(&lookup, &HashedStack::Empty, &empty));

    let stack = ItemStackSummary {
        item_id: Some(mod_item_id),
        count: 3,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID],
            repair_cost: Some(4),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let hashed = hashed_stack_from_summary(&lookup, &stack).unwrap();
    assert!(hashed_stack_matches(&lookup, &hashed, &stack));
    assert!(!hashed_stack_matches(&lookup, &HashedStack::Empty, &stack));
    assert!(!hashed_stack_matches(&lookup, &hashed, &empty));

    let mut wrong_count = stack.clone();
    wrong_count.count = 2;
    assert!(!hashed_stack_matches(&lookup, &hashed, &wrong_count));

    let mut wrong_item = stack.clone();
    wrong_item.item_id = Some(VANILLA_ITEM_STONE_ID);
    assert!(!hashed_stack_matches(&lookup, &hashed, &wrong_item));

    let mut wrong_removed = stack.clone();
    wrong_removed.component_patch.removed_type_ids.clear();
    assert!(!hashed_stack_matches(&lookup, &hashed, &wrong_removed));

    let mut wrong_added_val = stack.clone();
    wrong_added_val.component_patch.repair_cost = Some(5);
    assert!(!hashed_stack_matches(&lookup, &hashed, &wrong_added_val));

    let mut extra_added = stack.clone();
    extra_added
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    extra_added.component_patch.max_stack_size = Some(16);
    assert!(!hashed_stack_matches(&lookup, &hashed, &extra_added));

    let unknown_mod_stack = ItemStackSummary {
        item_id: Some(mod_item_id),
        count: 1,
        component_patch: Default::default(),
    };
    assert_eq!(
        hashed_stack_from_summary(&VanillaHashLookup, &unknown_mod_stack),
        None
    );

    if let HashedStack::Item(mut mutated_hash) = hashed.clone() {
        mutated_hash.components.removed_components = BTreeSet::new();
        assert!(!hashed_stack_matches(
            &lookup,
            &HashedStack::Item(mutated_hash),
            &stack
        ));
    }
    let _ = EquippableSummary {
        slot_id: 6,
        equip_sound: SoundEventSummary {
            registry_id: sound_event_id("minecraft:item.armor.equip_generic"),
            sound_id: None,
            fixed_range_bits: None,
        },
        asset_id: None,
        camera_overlay: None,
        allowed_entities: None,
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: SoundEventSummary {
            registry_id: sound_event_id("minecraft:item.shears.snip"),
            sound_id: None,
            fixed_range_bits: None,
        },
    };
    let _ = PotionContentsSummary::default();
    let _ = SuspiciousStewEffectSummary {
        effect_id: 0,
        duration: 160,
    };
}

#[test]
fn component_hashing_matches_vanilla_canonical_codec_and_bounds_show_item_depth() {
    let lookup = TestLookup::default();

    // 1. List-form text component `["a", "b"]` encodes identically to `{"text":"a","extra":["b"]}`.
    let list_form = NbtSummaryValue::List(vec![
        NbtSummaryValue::string("a"),
        NbtSummaryValue::string("b"),
    ]);
    let record_form = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("a")),
        entry(
            "extra",
            NbtSummaryValue::List(vec![NbtSummaryValue::string("b")]),
        ),
    ]);
    assert_eq!(
        hash_component(&lookup, &list_form),
        hash_component(&lookup, &record_form)
    );

    // 2. Default `interpret: false`, `plain: false`, and empty `with: []` are omitted.
    let nbt_default = NbtSummaryValue::Compound(vec![
        entry("nbt", NbtSummaryValue::string("foo")),
        entry("storage", NbtSummaryValue::string("test:bag")),
        entry("interpret", NbtSummaryValue::Byte(0)),
        entry("plain", NbtSummaryValue::Byte(0)),
    ]);
    let nbt_minimal = NbtSummaryValue::Compound(vec![
        entry("nbt", NbtSummaryValue::string("foo")),
        entry("storage", NbtSummaryValue::string("test:bag")),
    ]);
    assert_eq!(
        hash_component(&lookup, &nbt_default),
        hash_component(&lookup, &nbt_minimal)
    );

    let translate_empty_with = NbtSummaryValue::Compound(vec![
        entry("translate", NbtSummaryValue::string("key.example")),
        entry("with", NbtSummaryValue::List(Vec::new())),
    ]);
    let translate_minimal = NbtSummaryValue::Compound(vec![entry(
        "translate",
        NbtSummaryValue::string("key.example"),
    )]);
    assert_eq!(
        hash_component(&lookup, &translate_empty_with),
        hash_component(&lookup, &translate_minimal)
    );

    // 3. 4-float list `shadow_color` hashes identically to its packed ARGB int.
    let shadow_floats = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Glow")),
        entry(
            "shadow_color",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(1.0f32.to_bits()),
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::Float(0.0f32.to_bits()),
                NbtSummaryValue::Float(1.0f32.to_bits()),
            ]),
        ),
    ]);
    let shadow_int = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Glow")),
        entry("shadow_color", NbtSummaryValue::Int(0xFFFF_0000_u32 as i32)),
    ]);
    assert_eq!(
        hash_component(&lookup, &shadow_floats),
        hash_component(&lookup, &shadow_int)
    );

    // 4. Duplicate compound keys follow last-wins semantics.
    let dup_color = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Hi")),
        entry("color", NbtSummaryValue::string("red")),
        entry("color", NbtSummaryValue::string("blue")),
    ]);
    let final_color = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Hi")),
        entry("color", NbtSummaryValue::string("blue")),
    ]);
    assert_eq!(
        hash_component(&lookup, &dup_color),
        hash_component(&lookup, &final_color)
    );

    // 5. Nested `show_item` components increment depth and reject past MAX_ITEM_STACK_HASH_DEPTH.
    let wrap_show_item = |inner: NbtSummaryValue| {
        NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("wrap")),
            entry(
                "hover_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("show_item")),
                    entry("id", NbtSummaryValue::string("stone")),
                    entry(
                        "components",
                        NbtSummaryValue::Compound(vec![entry("custom_name", inner)]),
                    ),
                ]),
            ),
        ])
    };
    let mut nested = NbtSummaryValue::string("leaf");
    for _ in 0..2 {
        nested = wrap_show_item(nested);
    }
    assert!(hash_component(&lookup, &nested).is_some());
    for _ in 2..=bbb_protocol::packets::data_components::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        nested = wrap_show_item(nested);
    }
    assert_eq!(hash_component(&lookup, &nested), None);
}

#[test]
fn potion_contents_drops_hidden_effect_and_inline_trim_pattern_always_encodes_decal() {
    let lookup = TestLookup::default();

    let haste_id = vanilla_mob_effect_type_for_resource_id("minecraft:haste")
        .unwrap()
        .id;
    let effect = MobEffectInstanceSummary {
        effect_id: haste_id,
        amplifier: 1,
        duration: 200,
        ambient: false,
        show_particles: true,
        show_icon: true,
        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
            amplifier: 0,
            duration: 40,
            ambient: false,
            show_particles: true,
            show_icon: false,
            hidden_effect: None,
        })),
    };
    let patch_with = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            custom_effects: vec![effect.clone()],
            ..Default::default()
        },
        ..Default::default()
    };
    let patch_without = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            custom_effects: vec![MobEffectInstanceSummary {
                hidden_effect: None,
                ..effect.clone()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let hashed = hash_component_value(
        &lookup,
        &patch_with,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    )
    .unwrap();
    // Official Vanilla 26.1 `PotionContents.FULL_CODEC` hash (`PotionContents.java:51,150-152`).
    assert_eq!(hashed, 0xd688_8158);
    assert_eq!(
        hash_component_value(
            &lookup,
            &patch_without,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID
        ),
        Some(hashed)
    );

    // Inline `TrimPattern` with `decal: false` still emits `"decal": false` (`TrimPattern.java:20`).
    let trim_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
        armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
            asset_name: "opal".to_string(),
            override_armor_assets: BTreeMap::new(),
            description: "Opal".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::string("Opal"))),
        })),
        armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
            asset_id: "example:spiral".to_string(),
            description: "Spiral".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::string("Spiral"))),
            decal: false,
        })),
        ..Default::default()
    };
    let expected_material = hash_record([
        ("asset_name", hash_string("opal")),
        ("description", hash_string("Opal")),
    ]);
    let expected_pattern = hash_record([
        ("asset_id", hash_string("example:spiral")),
        ("description", hash_string("Spiral")),
        ("decal", hash_boolean(false)),
    ]);
    assert_eq!(
        hash_component_value(&lookup, &trim_patch, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID),
        Some(hash_record([
            ("material", expected_material),
            ("pattern", expected_pattern),
        ]))
    );
}

#[test]
fn predicates_accept_empty_compound_any_component_and_normalize_bounds_and_holders() {
    let mut lookup = TestLookup::default();
    lookup.insert("minecraft:enchantment", 0, "minecraft:sharpness");
    lookup.insert("minecraft:damage_type", 0, "minecraft:on_fire");
    lookup.insert_tag(
        "minecraft:damage_type",
        "minecraft:is_fire",
        &["minecraft:on_fire"],
    );

    // Wire and persistent decoders produce `Some(Compound([]))` for `AnyComponent`.
    let any_empty_compound = PartialDataComponentPredicateSummary {
        predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        ),
        wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
    };
    assert_eq!(
        hash_partial_component_predicates(&lookup, &[any_empty_compound]),
        Some(hash_map(vec![(
            hash_string("minecraft:lock"),
            hash_empty_map()
        )]))
    );
    let any_non_empty = PartialDataComponentPredicateSummary {
        predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        ),
        wire_value: Some(NbtSummaryValue::Compound(vec![entry(
            "x",
            NbtSummaryValue::Int(1),
        )])),
    };
    assert_eq!(
        hash_partial_component_predicates(&lookup, &[any_non_empty]),
        None
    );

    // Named holder set tags inside `exact_components` stay `#tag` without failing validation.
    let tagged_exact = PersistentItemPredicateSummary {
        exact_components: vec![PersistentDataComponentValueSummary {
            component_type_id: VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
            value: NbtSummaryValue::Compound(vec![entry(
                "types",
                NbtSummaryValue::string("#minecraft:is_fire"),
            )]),
        }],
        ..Default::default()
    };
    assert_eq!(
        hash_persistent_item_predicate(&lookup, &tagged_exact),
        Some(hash_record([(
            "components",
            hash_map(vec![(
                hash_string("minecraft:damage_resistant"),
                hash_record([("types", hash_string("#minecraft:is_fire"))]),
            )]),
        )]))
    );

    // `MinMaxBounds.Ints`: exact `{min:2,max:2}` -> `2`, `Byte(3)` -> `Int(3)`, `{}` omitted, `min>max` rejected.
    let damage_pred = NbtSummaryValue::Compound(vec![
        entry(
            "durability",
            NbtSummaryValue::Compound(vec![
                entry("min", NbtSummaryValue::Byte(2)),
                entry("max", NbtSummaryValue::Short(2)),
            ]),
        ),
        entry("damage", NbtSummaryValue::Compound(Vec::new())),
    ]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID,
            &damage_pred,
        ),
        Some(hash_record([("durability", hash_int(2))]))
    );
    let invalid_bounds = NbtSummaryValue::Compound(vec![entry(
        "damage",
        NbtSummaryValue::Compound(vec![
            entry("min", NbtSummaryValue::Int(5)),
            entry("max", NbtSummaryValue::Int(1)),
        ]),
    )]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_DAMAGE_TYPE_ID,
            &invalid_bounds,
        ),
        None
    );

    // Single-element holder list collapses to scalar string; duplicate compound keys rejected.
    let ench_pred = NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
        entry(
            "enchantments",
            NbtSummaryValue::List(vec![NbtSummaryValue::string("sharpness")]),
        ),
        entry("levels", NbtSummaryValue::Compound(Vec::new())),
    ])]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_ENCHANTMENTS_TYPE_ID,
            &ench_pred,
        ),
        Some(hash_list([hash_record([(
            "enchantments",
            hash_string("minecraft:sharpness"),
        )])]))
    );
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_FIREWORK_EXPLOSION_TYPE_ID,
            &NbtSummaryValue::Compound(vec![entry("colors", NbtSummaryValue::List(Vec::new()))]),
        ),
        None
    );

    // Nested `ItemPredicate` supports `"predicates"` and `AttributeModifiersPredicate` normalizes `Float` -> `Double`.
    let container_pred = NbtSummaryValue::Compound(vec![entry(
        "items",
        NbtSummaryValue::Compound(vec![entry(
            "contains",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("items", NbtSummaryValue::string("stone")),
                entry(
                    "predicates",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:lock",
                        NbtSummaryValue::Compound(Vec::new()),
                    )]),
                ),
            ])]),
        )]),
    )]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID,
            &container_pred,
        ),
        Some(hash_record([(
            "items",
            hash_record([(
                "contains",
                hash_list([hash_record([
                    ("items", hash_string("minecraft:stone")),
                    (
                        "predicates",
                        hash_map(vec![(hash_string("minecraft:lock"), hash_empty_map())]),
                    ),
                ])]),
            )]),
        )]))
    );
    let attr_pred = NbtSummaryValue::Compound(vec![entry(
        "modifiers",
        NbtSummaryValue::Compound(vec![entry(
            "contains",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("attribute", NbtSummaryValue::string("attack_damage")),
                entry("id", NbtSummaryValue::string("bonus")),
                entry("amount", NbtSummaryValue::Float(1.5_f32.to_bits())),
                entry("operation", NbtSummaryValue::string("add_value")),
                entry("slot", NbtSummaryValue::string("mainhand")),
            ])]),
        )]),
    )]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID,
            &attr_pred,
        ),
        Some(hash_record([(
            "modifiers",
            hash_record([(
                "contains",
                hash_list([hash_record([
                    ("attribute", hash_string("minecraft:attack_damage")),
                    ("id", hash_string("minecraft:bonus")),
                    ("amount", hash_double(1.5_f64.to_bits())),
                    ("operation", hash_string("add_value")),
                    ("slot", hash_string("mainhand")),
                ])]),
            )]),
        )]))
    );

    // Nested `ItemPredicate` omits empty `components`/`predicates` (`DataComponentMatchers.java:18-24`).
    let empty_matchers_container_pred = NbtSummaryValue::Compound(vec![entry(
        "items",
        NbtSummaryValue::Compound(vec![entry(
            "contains",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("items", NbtSummaryValue::string("stone")),
                entry("components", NbtSummaryValue::Compound(Vec::new())),
                entry("predicates", NbtSummaryValue::Compound(Vec::new())),
            ])]),
        )]),
    )]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_CONTAINER_TYPE_ID,
            &empty_matchers_container_pred,
        ),
        Some(hash_record([(
            "items",
            hash_record([(
                "contains",
                hash_list([hash_record([("items", hash_string("minecraft:stone"))])]),
            )]),
        )]))
    );

    // `AttributeModifiersPredicate` accepts integer NBT tags for `MinMaxBounds.Doubles` (`NbtOps.java:65-67`).
    let int_amount_attr_pred = NbtSummaryValue::Compound(vec![entry(
        "modifiers",
        NbtSummaryValue::Compound(vec![entry(
            "contains",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![entry("amount", NbtSummaryValue::Int(2))]),
                NbtSummaryValue::Compound(vec![entry(
                    "amount",
                    NbtSummaryValue::Compound(vec![
                        entry("min", NbtSummaryValue::Byte(1)),
                        entry("max", NbtSummaryValue::Long(5)),
                    ]),
                )]),
            ]),
        )]),
    )]);
    assert_eq!(
        hash_concrete_component_predicate(
            &lookup,
            VANILLA_DATA_COMPONENT_PREDICATE_ATTRIBUTE_MODIFIERS_TYPE_ID,
            &int_amount_attr_pred,
        ),
        Some(hash_record([(
            "modifiers",
            hash_record([(
                "contains",
                hash_list([
                    hash_record([("amount", hash_double(2.0_f64.to_bits()))]),
                    hash_record([(
                        "amount",
                        hash_record([
                            ("min", hash_double(1.0_f64.to_bits())),
                            ("max", hash_double(5.0_f64.to_bits())),
                        ]),
                    )]),
                ]),
            )]),
        )]))
    );

    // `GameProfile` requires both `uuid` and `name`; non-compound `custom_data` is rejected.
    let incomplete_profile = ResolvableProfileSummary {
        kind: ResolvableProfileKindSummary::GameProfile,
        uuid: None,
        name: Some("Alex".to_string()),
        properties: Vec::new(),
        profile_textures: None,
        skin_patch: PlayerSkinPatchSummary::default(),
    };
    assert_eq!(hash_resolvable_profile(&incomplete_profile), None);
    let bad_custom_data = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID],
        custom_data: Some(NbtSummaryValue::Int(42)),
        ..Default::default()
    };
    assert_eq!(
        hash_component_value(
            &lookup,
            &bad_custom_data,
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
        ),
        None
    );
}

#[test]
fn fireworks_flight_duration_hashes_as_unsigned_byte() {
    let lookup = VanillaHashLookup;
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID],
        fireworks_flight_duration: Some(200),
        ..Default::default()
    };
    assert_eq!(
        hash_component_value(&lookup, &patch, VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID),
        Some(hash_record([("flight_duration", hash_byte(200_u8 as i8))]))
    );
}

#[test]
fn tropical_fish_pattern_hashes_sparse_packed_ids_and_rejects_unknown() {
    let lookup = VanillaHashLookup;
    for (packed_id, expected_name) in [(0, "kob"), (257, "stripey"), (1281, "clayfish")] {
        let patch = DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID],
            tropical_fish_pattern_id: Some(packed_id),
            ..Default::default()
        };
        assert_eq!(
            hash_component_value(
                &lookup,
                &patch,
                VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID
            ),
            Some(hash_string(expected_name))
        );
    }
    let unknown = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID],
        tropical_fish_pattern_id: Some(9999),
        ..Default::default()
    };
    assert_eq!(
        hash_component_value(
            &lookup,
            &unknown,
            VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID
        ),
        None
    );
}

#[test]
fn block_predicate_hash_omits_empty_exact_components() {
    let lookup = VanillaHashLookup;
    let none_exact = BlockPredicateSummary::default();
    let some_empty_exact = BlockPredicateSummary {
        exact_components: Some(Box::new(DataComponentPatchSummary::default())),
        ..Default::default()
    };
    assert_eq!(
        hash_block_predicate(&lookup, &some_empty_exact),
        hash_block_predicate(&lookup, &none_exact)
    );
    assert_eq!(
        hash_block_predicate(&lookup, &some_empty_exact),
        Some(hash_empty_map())
    );
}

#[test]
fn item_hash_and_hover_show_item_support_content_registry_custom_content() {
    use bbb_protocol::content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration,
    };

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "example:ruby_sword".to_string(),
        max_stack_size: 1,
        is_block_item: false,
    });
    builder.register_block(ModBlockRegistration {
        resource_id: "example:ruby_ore".to_string(),
        state_count: 1,
        light_emission: 0,
        destroy_speed: 3.0,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    let registry = builder.freeze_for_session().unwrap();
    let custom_item_id = registry
        .item_by_resource_id("example:ruby_sword")
        .unwrap()
        .id;
    let custom_block_id = registry
        .block_by_resource_id("example:ruby_ore")
        .unwrap()
        .block_id;

    struct CustomContentLookup {
        registry: ContentRegistry,
    }
    impl PersistentRegistryResolver for CustomContentLookup {
        fn resource_membership(
            &self,
            registry_id: &str,
            resource_id: &str,
        ) -> PersistentRegistryMembership {
            UnavailableHolderResolver.resource_membership(registry_id, resource_id)
        }
        fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
            UnavailableHolderResolver.tag_members(registry_id, tag_id)
        }
    }
    impl PersistentItemStackHolderResolver for CustomContentLookup {
        fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
            None
        }
        fn content_registry(&self) -> &ContentRegistry {
            &self.registry
        }
    }
    impl ItemHashLookup for CustomContentLookup {}

    let lookup = CustomContentLookup { registry };

    // Custom item stack hashes with custom ContentRegistry, fails with vanilla-only lookup.
    let custom_stack = ItemStackSummary {
        item_id: Some(custom_item_id),
        count: 1,
        component_patch: Default::default(),
    };
    assert!(hashed_stack_from_summary(&lookup, &custom_stack).is_some());
    assert!(hashed_stack_from_summary(&VanillaHashLookup, &custom_stack).is_none());

    // Custom block inside BlockPredicate hashes via `resolve_holder_resource_id`.
    let custom_block_pred = BlockPredicateSummary {
        blocks: Some(HolderSetSummary {
            registry_ids: vec![custom_block_id],
            tag: None,
        }),
        ..Default::default()
    };
    assert_eq!(
        hash_block_predicate(&lookup, &custom_block_pred),
        Some(hash_record([("blocks", hash_string("example:ruby_ore"))]))
    );
    assert_eq!(
        hash_block_predicate(&VanillaHashLookup, &custom_block_pred),
        None
    );

    // `HoverEvent.ShowItem` `components` patch containing custom item (`components_rich.rs:129`).
    let hover_component = NbtSummaryValue::Compound(vec![
        entry("text", NbtSummaryValue::string("Hover")),
        entry(
            "hover_event",
            NbtSummaryValue::Compound(vec![
                entry("action", NbtSummaryValue::string("show_item")),
                entry("id", NbtSummaryValue::string("minecraft:stone")),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry(
                        "minecraft:use_remainder",
                        NbtSummaryValue::string("example:ruby_sword"),
                    )]),
                ),
            ]),
        ),
    ]);
    assert!(hash_component(&lookup, &hover_component).is_some());
    assert!(hash_component(&VanillaHashLookup, &hover_component).is_none());
}
