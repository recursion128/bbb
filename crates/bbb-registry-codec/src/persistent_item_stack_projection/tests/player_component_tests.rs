//! Round trips and refusals for `minecraft:attribute_modifiers`,
//! `minecraft:profile`, `minecraft:food` and `minecraft:consumable`.

use super::super::nbt_support::latest_field;
use super::*;
use bbb_facts::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
};
use bbb_facts::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
};
use bbb_facts::equipment_slots::{
    VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID, VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
};
use bbb_facts::item_component_value_types::ItemUseAnimationSummary;
use bbb_facts::mob_effect_types::{VANILLA_MOB_EFFECT_BAD_OMEN_ID, VANILLA_MOB_EFFECT_POISON_ID};
use bbb_facts::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::packets::{
    AttributeModifierSummary, ConsumeEffectSummary, MobEffectDetailsSummary,
    MobEffectInstanceSummary, ResolvableProfileKindSummary, ResourceTextureSummary,
    SoundEventSummary,
};
use uuid::Uuid;

fn component_stack(item: &str, type_id: i32, value: NbtSummaryValue) -> NbtSummaryValue {
    let component = bbb_facts::data_component_types::vanilla_data_component_type(type_id)
        .expect("known component")
        .resource_id;
    saved_stack_with_components(item, vec![entry(component, value)])
}

fn uuid_ints(uuid: Uuid) -> NbtSummaryValue {
    let (most, least) = uuid.as_u64_pair();
    NbtSummaryValue::IntArray(vec![
        (most >> 32) as i32,
        most as i32,
        (least >> 32) as i32,
        least as i32,
    ])
}

fn round_trip_via<R>(saved: &NbtSummaryValue, resolver: &R) -> ItemStackSummary
where
    R: PersistentItemStackHolderResolver,
{
    let summary = crate::read_item_stack(saved, resolver).unwrap();
    let raw =
        crate::item_stack_summary_to_persistent_nbt_with_resolver(&summary, resolver).unwrap();
    assert_eq!(raw, saved.clone());
    let decoded = raw.clone();
    let again = crate::read_item_stack(&decoded, resolver).unwrap();
    assert_eq!(again, summary);
    summary
}

fn round_trip_with<R>(saved: &NbtSummaryValue, resolver: &R) -> ItemStackSummary
where
    R: PersistentItemStackHolderResolver,
{
    round_trip_via(saved, resolver)
}

fn round_trip(saved: &NbtSummaryValue) -> ItemStackSummary {
    let summary = crate::read_item_stack(saved, &crate::UnavailableHolderResolver).unwrap();
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(raw, saved.clone());
    let decoded = raw.clone();
    let again = crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(again, summary);
    summary
}

/// `minecraft:consumable` names built-in registry elements -- the sound
/// and every `apply_effects`/`remove_effects` holder
/// -- so its save form is read through a publication that
/// answers them.
fn consumable_round_trip(saved: &NbtSummaryValue) -> ItemStackSummary {
    round_trip_with(saved, &BuiltinResolver)
}

// ---------------------------------------------------------------------------
// minecraft:attribute_modifiers
// ---------------------------------------------------------------------------

/// `ItemAttributeModifiers.CODEC`: the
/// entry's built-in attribute name, the flattened `AttributeModifier`
/// and the two `optionalFieldOf` defaults.
#[test]
fn attribute_modifiers_round_trip_every_slot_and_display_branch() {
    let saved = component_stack(
        "minecraft:diamond_sword",
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        NbtSummaryValue::List(vec![
            NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:attack_damage".into()),
                ),
                entry(
                    "id",
                    NbtSummaryValue::String("minecraft:base_attack_damage".into()),
                ),
                entry("amount", NbtSummaryValue::Double(1.5_f64.to_bits())),
                entry("operation", NbtSummaryValue::String("add_value".into())),
            ]),
            NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:max_health".into()),
                ),
                entry("id", NbtSummaryValue::String("example:boost".into())),
                entry("amount", NbtSummaryValue::Double(2.0_f64.to_bits())),
                entry(
                    "operation",
                    NbtSummaryValue::String("add_multiplied_base".into()),
                ),
                entry("slot", NbtSummaryValue::String("mainhand".into())),
                entry(
                    "display",
                    NbtSummaryValue::Compound(vec![entry(
                        "type",
                        NbtSummaryValue::String("hidden".into()),
                    )]),
                ),
            ]),
            NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:max_health".into()),
                ),
                entry("id", NbtSummaryValue::String("example:shown".into())),
                entry("amount", NbtSummaryValue::Double((-0.5_f64).to_bits())),
                entry("operation", NbtSummaryValue::String("add_value".into())),
                entry(
                    "display",
                    NbtSummaryValue::Compound(vec![
                        entry("type", NbtSummaryValue::String("override".into())),
                        entry("value", NbtSummaryValue::String("+2 Max Health".into())),
                    ]),
                ),
            ]),
        ]),
    );
    let summary = round_trip(&saved);
    let modifiers = &summary.component_patch.attribute_modifiers;
    assert_eq!(modifiers.len(), 3);
    assert_eq!(
        modifiers[0].attribute_id,
        VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID
    );
    assert_eq!(modifiers[0].modifier_id, "minecraft:base_attack_damage");
    assert_eq!(modifiers[0].amount_bits, 1.5_f64.to_bits());
    assert_eq!(modifiers[0].slot_id, VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID);
    assert_eq!(modifiers[1].attribute_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID);
    assert_eq!(modifiers[1].amount_bits, 2.0_f64.to_bits());
    assert_eq!(
        modifiers[1].operation_id,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
    );
    assert_eq!(
        modifiers[1].slot_id,
        VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID
    );
    assert_eq!(
        modifiers[1].display_id,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID
    );
    assert_eq!(
        modifiers[2].display_id,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID
    );
    assert_eq!(modifiers[2].display_text.as_deref(), Some("+2 Max Health"));
    assert_eq!(
        modifiers[2].display_text_nbt,
        Some(NbtSummaryValue::String("+2 Max Health".into()))
    );
}

/// The dispatch writes its type name beside the branch codec's own fields
/// and both defaults are omitted.
#[test]
fn attribute_modifier_codec_defaults_are_omitted_from_the_save_form() {
    let summary = stack(
        VANILLA_ITEM_DIAMOND_SWORD_ID,
        1,
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID],
            attribute_modifiers: vec![AttributeModifierSummary {
                attribute_id: VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                modifier_id: "minecraft:base_attack_damage".to_string(),
                amount_bits: 0.0_f64.to_bits(),
                operation_id: 0,
                slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                display_id: 0,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            }],
            ..DataComponentPatchSummary::default()
        },
    );
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:attribute_modifiers",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:attack_damage".into())
                ),
                entry(
                    "id",
                    NbtSummaryValue::String("minecraft:base_attack_damage".into())
                ),
                entry("amount", NbtSummaryValue::Double(0.0_f64.to_bits())),
                entry("operation", NbtSummaryValue::String("add_value".into())),
            ])]),
        )]))
    );
}

// ---------------------------------------------------------------------------
// minecraft:profile
// ---------------------------------------------------------------------------

/// `FULL_CODEC` picks its either branch from
/// the record itself: a full game profile needs both `id` and `name`, a partial
/// keeps whichever it has, and the skin patch is appended either way.
#[test]
fn profile_round_trips_both_either_branches_and_the_skin_patch() {
    let uuid = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
    let saved = component_stack(
        "minecraft:player_head",
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("id", uuid_ints(uuid)),
            entry("name", NbtSummaryValue::String("Player".into())),
            entry(
                "properties",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("name", NbtSummaryValue::String("textures".into())),
                    entry("value", NbtSummaryValue::String("encoded".into())),
                    entry("signature", NbtSummaryValue::String("signed".into())),
                ])]),
            ),
            entry("texture", NbtSummaryValue::String("example:skin".into())),
            entry("model", NbtSummaryValue::String("slim".into())),
        ]),
    );
    let summary = round_trip(&saved);
    let profile = summary.component_patch.profile.as_ref().unwrap();
    assert_eq!(profile.kind, ResolvableProfileKindSummary::GameProfile);
    assert_eq!(profile.uuid, Some(uuid));
    assert_eq!(profile.name.as_deref(), Some("Player"));
    assert_eq!(profile.properties.len(), 1);
    assert_eq!(profile.properties[0].name, "textures");
    assert_eq!(profile.properties[0].signature.as_ref().unwrap(), "signed");
    assert_eq!(
        profile.skin_patch.body,
        Some(ResourceTextureSummary {
            asset_id: "example:skin".to_string(),
            texture_path: "example:textures/skin.png".to_string(),
        })
    );
    assert_eq!(
        profile.skin_patch.model,
        Some(bbb_protocol::packets::PlayerModelTypeSummary::Slim)
    );

    // The partial branch: the codec's `PLAYER_NAME` alternative is a decode
    // branch only, so a name-only record is still written as the record.
    let saved = component_stack(
        "minecraft:player_head",
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        NbtSummaryValue::Compound(vec![entry(
            "name",
            NbtSummaryValue::String("Player".into()),
        )]),
    );
    let summary = round_trip(&saved);
    let profile = summary.component_patch.profile.as_ref().unwrap();
    assert_eq!(profile.kind, ResolvableProfileKindSummary::Partial);
    assert_eq!(profile.uuid, None);

    let saved = component_stack(
        "minecraft:player_head",
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        NbtSummaryValue::Compound(vec![entry("id", uuid_ints(uuid))]),
    );
    let summary = round_trip(&saved);
    let profile = summary.component_patch.profile.as_ref().unwrap();
    assert_eq!(profile.kind, ResolvableProfileKindSummary::Partial);
    assert_eq!(profile.name, None);
    assert_eq!(profile.uuid, Some(uuid));
}

#[test]
fn profile_inputs_preserve_compact_names_and_raw_property_maps() {
    let compact = read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &NbtSummaryValue::String("Player_1".into()),
        &UnavailableHolderResolver,
    )
    .unwrap()
    .profile
    .unwrap();
    assert_eq!(compact.name.as_deref(), Some("Player_1"));
    assert_eq!(compact.kind, ResolvableProfileKindSummary::Partial);

    let raw = bbb_protocol::JavaString::from(vec![0xd800, 65, 0xdc00]);
    let max_name = bbb_protocol::JavaString::from("k".repeat(64));
    let max_value = bbb_protocol::JavaString::from("v".repeat(32_767));
    let mut values = vec![max_value.to_nbt()];
    values.extend((0..14).map(|index| NbtSummaryValue::String(index.to_string().into())));
    let properties = vec![entry(
        "properties",
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: raw.clone(),
                value: NbtSummaryValue::List(vec![raw.to_nbt()]),
            },
            NbtSummaryEntry {
                name: max_name.clone(),
                value: NbtSummaryValue::List(values),
            },
        ]),
    )];
    let source = NbtSummaryValue::Compound(properties);
    let patch = read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &source,
        &UnavailableHolderResolver,
    )
    .unwrap();
    let encoded =
        data_component_patch_to_persistent_nbt_with_resolver(&patch, &UnavailableHolderResolver)
            .unwrap()
            .unwrap();
    assert_eq!(
        read_component_patch(&encoded, &UnavailableHolderResolver).unwrap(),
        patch
    );
    let saved = component_stack(
        "minecraft:player_head",
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        source.clone(),
    );
    let stack = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let encoded_stack = item_stack_summary_to_persistent_nbt(&stack).unwrap();
    let reread = encoded_stack.clone();
    assert_eq!(
        crate::read_item_stack(&reread, &crate::UnavailableHolderResolver).unwrap(),
        stack
    );
    let nested = saved_stack_with_components(
        "minecraft:shulker_box",
        vec![entry(
            "minecraft:container",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry("slot", NbtSummaryValue::Int(0)),
                entry(
                    "item",
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "id",
                            NbtSummaryValue::String("minecraft:player_head".into()),
                        ),
                        entry(
                            "components",
                            NbtSummaryValue::Compound(vec![entry("minecraft:profile", source)]),
                        ),
                    ]),
                ),
            ])]),
        )],
    );
    let nested_stack = crate::read_item_stack(&nested, &crate::UnavailableHolderResolver).unwrap();
    let nested_encoded = item_stack_summary_to_persistent_nbt(&nested_stack).unwrap();
    let nested_reread = nested_encoded.clone();
    assert_eq!(
        crate::read_item_stack(&nested_reread, &crate::UnavailableHolderResolver).unwrap(),
        nested_stack
    );
    bbb_protocol::packets::data_components::validate_data_component_patch_summary(&patch).unwrap();

    // Map-form properties exceeding wire limits (total count > 16, name > 64, value > 32767)
    // must be rejected so unencodable profiles cannot cause a persistent Chunk/Join Ban.
    let too_many = NbtSummaryValue::Compound(vec![entry(
        "properties",
        NbtSummaryValue::Compound(vec![entry(
            "k",
            NbtSummaryValue::List(
                (0..17)
                    .map(|index| NbtSummaryValue::String(index.to_string().into()))
                    .collect(),
            ),
        )]),
    )]);
    assert!(read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &too_many,
        &UnavailableHolderResolver,
    )
    .is_err());

    let too_long_name = NbtSummaryValue::Compound(vec![entry(
        "properties",
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: bbb_protocol::JavaString::from("k".repeat(65)),
            value: NbtSummaryValue::List(vec![NbtSummaryValue::String("short".into())]),
        }]),
    )]);
    assert!(read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &too_long_name,
        &UnavailableHolderResolver,
    )
    .is_err());

    let too_long_value = NbtSummaryValue::Compound(vec![entry(
        "properties",
        NbtSummaryValue::Compound(vec![entry(
            "k",
            NbtSummaryValue::List(vec![NbtSummaryValue::String("v".repeat(32_768).into())]),
        )]),
    )]);
    assert!(read_item_component(
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        &too_long_value,
        &UnavailableHolderResolver,
    )
    .is_err());

    let profile = patch.profile.unwrap();
    assert_eq!(profile.properties.len(), 16);
    assert_eq!(profile.properties[0].name, raw);
    assert_eq!(profile.properties[0].value, raw);
    assert_eq!(profile.properties[0].signature, None);
    assert_eq!(profile.properties[1].name.units(), max_name.units());
    assert_eq!(profile.properties[1].value.units(), max_value.units());
    assert!(profile
        .properties
        .iter()
        .all(|property| property.signature.is_none()));
}

/// `ExtraCodecs.PROPERTY_MAP`'s xmap encodes `propertyMap.values()` over an
/// `ImmutableMultimap`, so a save file's list is
/// grouped by first-seen key before it reaches the wire.
#[test]
fn profile_properties_are_grouped_by_first_seen_key() {
    let property = |name: &str, value: &str| {
        NbtSummaryValue::Compound(vec![
            entry("name", NbtSummaryValue::String(name.into())),
            entry("value", NbtSummaryValue::String(value.into())),
        ])
    };
    let saved = component_stack(
        "minecraft:player_head",
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("name", NbtSummaryValue::String("Player".into())),
            entry(
                "properties",
                NbtSummaryValue::List(vec![
                    property("a", "1"),
                    property("b", "1"),
                    property("a", "2"),
                ]),
            ),
        ]),
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let names: Vec<String> = summary
        .component_patch
        .profile
        .as_ref()
        .unwrap()
        .properties
        .iter()
        .map(|property| {
            property
                .value
                .to_jdk_utf8()
                .into_iter()
                .map(char::from)
                .collect()
        })
        .collect();
    assert_eq!(names, ["1", "2", "1"]);

    // The grouped order is the codec's fixpoint.
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        raw,
        component_stack(
            "minecraft:player_head",
            VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
            NbtSummaryValue::Compound(vec![
                entry("name", NbtSummaryValue::String("Player".into())),
                entry(
                    "properties",
                    NbtSummaryValue::List(vec![
                        property("a", "1"),
                        property("a", "2"),
                        property("b", "1"),
                    ]),
                ),
            ]),
        )
    );
}

// ---------------------------------------------------------------------------
// minecraft:food
// ---------------------------------------------------------------------------

/// `FoodProperties.DIRECT_CODEC`.
#[test]
fn food_round_trips_and_omits_the_optional_flag() {
    let saved = component_stack(
        "minecraft:apple",
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("nutrition", NbtSummaryValue::Int(4)),
            entry("saturation", NbtSummaryValue::Float(0.75_f32.to_bits())),
            entry("can_always_eat", NbtSummaryValue::Byte(1)),
        ]),
    );
    let summary = round_trip(&saved);
    let patch = &summary.component_patch;
    assert_eq!(patch.food_nutrition, Some(4));
    assert_eq!(patch.food_saturation_bits, Some(0.75_f32.to_bits()));
    assert!(patch.food_can_always_eat);

    // `can_always_eat` defaults to `false`, so it is dropped while encoding.
    let saved = component_stack(
        "minecraft:apple",
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("nutrition", NbtSummaryValue::Int(4)),
            entry("saturation", NbtSummaryValue::Float(0.75_f32.to_bits())),
        ]),
    );
    let summary = round_trip(&saved);
    assert!(!summary.component_patch.food_can_always_eat);
}

// ---------------------------------------------------------------------------
// minecraft:consumable
// ---------------------------------------------------------------------------

/// `Consumable.CODEC` with all five defaults present,
/// including an `apply_effects` entry whose mob effect is a built-in holder.
#[test]
fn consumable_round_trips_every_effect_and_omits_every_default() {
    let saved = component_stack(
        "minecraft:ominous_bottle",
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("consume_seconds", NbtSummaryValue::Float(2.5_f32.to_bits())),
            entry("animation", NbtSummaryValue::String("drink".into())),
            entry(
                "sound",
                NbtSummaryValue::String("minecraft:entity.generic.drink".into()),
            ),
            entry("has_consume_particles", NbtSummaryValue::Byte(0)),
            entry(
                "on_consume_effects",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:apply_effects".into()),
                    ),
                    entry(
                        "effects",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("id", NbtSummaryValue::String("minecraft:bad_omen".into())),
                            entry("amplifier", NbtSummaryValue::Byte(1)),
                            entry("duration", NbtSummaryValue::Int(6000)),
                            entry("ambient", NbtSummaryValue::Byte(1)),
                            entry("show_particles", NbtSummaryValue::Byte(0)),
                            entry("show_icon", NbtSummaryValue::Byte(1)),
                            entry(
                                "hidden_effect",
                                NbtSummaryValue::Compound(vec![
                                    entry("duration", NbtSummaryValue::Int(20)),
                                    entry("show_icon", NbtSummaryValue::Byte(0)),
                                ]),
                            ),
                        ])]),
                    ),
                    entry("probability", NbtSummaryValue::Float(0.5_f32.to_bits())),
                ])]),
            ),
        ]),
    );
    let summary = consumable_round_trip(&saved);
    let patch = &summary.component_patch;
    assert_eq!(patch.consumable.unwrap().consume_seconds, 2.5);
    assert_eq!(
        patch.consumable.unwrap().animation,
        ItemUseAnimationSummary::Drink
    );
    assert_eq!(
        patch.consumable_sound.as_ref().unwrap().registry_id,
        vanilla_sound_event_registry_id("minecraft:entity.generic.drink")
    );
    assert_eq!(patch.consumable_has_consume_particles, Some(false));
    assert_eq!(
        patch.consumable_effects,
        [ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                effect_id: VANILLA_MOB_EFFECT_BAD_OMEN_ID,
                amplifier: 1,
                duration: 6000,
                ambient: true,
                show_particles: false,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 20,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            probability_bits: 0.5_f32.to_bits(),
        }]
    );

    // An empty compound is every default at once: `EAT`, the generic eat sound,
    // particles on and no effects.
    let saved = component_stack(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        NbtSummaryValue::Compound(Vec::new()),
    );
    let summary = consumable_round_trip(&saved);
    let patch = &summary.component_patch;
    assert_eq!(patch.consumable.unwrap().consume_seconds, 1.6);
    assert_eq!(
        patch.consumable.unwrap().animation,
        ItemUseAnimationSummary::Eat
    );
    assert_eq!(
        patch.consumable_sound.as_ref().unwrap().registry_id,
        vanilla_sound_event_registry_id("minecraft:entity.generic.eat")
    );
    assert_eq!(patch.consumable_has_consume_particles, Some(true));
    assert!(patch.consumable_effects.is_empty());
}

#[test]
fn consumable_persistent_numeric_codec_wraps_amplifier_and_bool() {
    let saved = component_stack(
        "minecraft:stone",
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        NbtSummaryValue::Compound(vec![
            entry("has_consume_particles", NbtSummaryValue::Int(256)),
            entry(
                "on_consume_effects",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:apply_effects".into()),
                    ),
                    entry(
                        "effects",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("id", NbtSummaryValue::String("minecraft:poison".into())),
                            entry("amplifier", NbtSummaryValue::Int(511)),
                            entry("show_particles", NbtSummaryValue::Int(256)),
                            entry(
                                "hidden_effect",
                                NbtSummaryValue::Compound(vec![entry(
                                    "duration",
                                    NbtSummaryValue::Int(40),
                                )]),
                            ),
                        ])]),
                    ),
                ])]),
            ),
        ]),
    );
    let summary = crate::read_item_stack(&saved, &BuiltinResolver).unwrap();
    assert_eq!(
        summary.component_patch.consumable_has_consume_particles,
        Some(false)
    );
    let ConsumeEffectSummary::ApplyEffects { effects, .. } =
        &summary.component_patch.consumable_effects[0]
    else {
        panic!("apply effects");
    };
    assert_eq!(effects[0].amplifier, 255);
    assert!(!effects[0].show_particles);
    assert_eq!(effects[0].hidden_effect.as_ref().unwrap().duration, 40);

    let raw = crate::item_stack_summary_to_persistent_nbt_with_resolver(&summary, &BuiltinResolver)
        .unwrap();
    let component = bbb_protocol::nbt_summary::value_last_field(
        projected_components(&raw).unwrap(),
        "minecraft:consumable",
    )
    .unwrap();
    let NbtSummaryValue::Compound(fields) = component else {
        panic!("consumable compound");
    };
    assert_eq!(
        latest_field(fields, "has_consume_particles"),
        Some(&NbtSummaryValue::Byte(0))
    );
    let NbtSummaryValue::List(on_consume) = latest_field(fields, "on_consume_effects").unwrap()
    else {
        panic!("effects");
    };
    let NbtSummaryValue::Compound(apply) = &on_consume[0] else {
        panic!("apply");
    };
    let NbtSummaryValue::List(effects) = latest_field(apply, "effects").unwrap() else {
        panic!("effect list");
    };
    let NbtSummaryValue::Compound(effect) = &effects[0] else {
        panic!("effect");
    };
    assert_eq!(
        latest_field(effect, "amplifier"),
        Some(&NbtSummaryValue::Byte(-1))
    );
    assert_eq!(
        latest_field(effect, "show_particles"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert!(latest_field(effect, "hidden_effect").is_some());
}

#[test]
fn consumable_direct_sound_lenient_range_keeps_optional_value_through_save_and_wire() {
    let sound = |range| {
        let mut fields = vec![entry(
            "sound_id",
            NbtSummaryValue::String("example:drink".into()),
        )];
        if let Some(value) = range {
            fields.push(entry("range", value));
        }
        component_stack(
            "minecraft:stone",
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            NbtSummaryValue::Compound(vec![entry("sound", NbtSummaryValue::Compound(fields))]),
        )
    };
    let missing = consumable_round_trip(&sound(None));
    let malformed = crate::read_item_stack(
        &sound(Some(NbtSummaryValue::String("not-a-float".into()))),
        &BuiltinResolver,
    )
    .unwrap();
    let present = consumable_round_trip(&sound(Some(NbtSummaryValue::Float(16.0f32.to_bits()))));
    for summary in [&missing, &malformed] {
        assert_eq!(
            summary
                .component_patch
                .consumable_sound
                .as_ref()
                .unwrap()
                .fixed_range_bits,
            None
        );
        let raw =
            crate::item_stack_summary_to_persistent_nbt_with_resolver(summary, &BuiltinResolver)
                .unwrap();
        let decoded = raw.clone();
        let reloaded = crate::read_item_stack(&decoded, &BuiltinResolver).unwrap();
        assert_eq!(
            reloaded
                .component_patch
                .consumable_sound
                .as_ref()
                .unwrap()
                .fixed_range_bits,
            None
        );
        let (packet_id, bytes) =
            bbb_protocol::packets::server_play::encode_play_clientbound_container_set_slot(
                &bbb_protocol::packets::inventory::ContainerSetSlot {
                    container_id: 0,
                    state_id: 1,
                    slot: 0,
                    item: summary.clone(),
                },
            )
            .unwrap();
        let bbb_protocol::packets::play_clientbound::PlayClientbound::ContainerSetSlot(slot) =
            bbb_protocol::packets::play_clientbound::decode_play_clientbound(packet_id, &bytes)
                .unwrap()
        else {
            panic!("slot packet");
        };
        assert_eq!(
            slot.item
                .component_patch
                .consumable_sound
                .as_ref()
                .unwrap()
                .fixed_range_bits,
            None
        );
    }
    assert_eq!(
        present
            .component_patch
            .consumable_sound
            .as_ref()
            .unwrap()
            .fixed_range_bits,
        Some(16.0f32.to_bits())
    );
    assert_ne!(
        missing.component_patch.consumable_sound,
        present.component_patch.consumable_sound
    );
}

#[test]
fn consumable_round_trips_the_other_four_effect_types() {
    let saved = component_stack(
        "minecraft:milk_bucket",
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        NbtSummaryValue::Compound(vec![entry(
            "on_consume_effects",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![entry(
                    "type",
                    NbtSummaryValue::String("minecraft:clear_all_effects".into()),
                )]),
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:remove_effects".into()),
                    ),
                    entry(
                        "effects",
                        NbtSummaryValue::String("minecraft:poison".into()),
                    ),
                ]),
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:teleport_randomly".into()),
                    ),
                    entry("diameter", NbtSummaryValue::Float(8.0_f32.to_bits())),
                ]),
                NbtSummaryValue::Compound(vec![
                    entry(
                        "type",
                        NbtSummaryValue::String("minecraft:play_sound".into()),
                    ),
                    entry(
                        "sound",
                        NbtSummaryValue::Compound(vec![
                            entry("sound_id", NbtSummaryValue::String("example:burp".into())),
                            entry("range", NbtSummaryValue::Float(4.0_f32.to_bits())),
                        ]),
                    ),
                ]),
                NbtSummaryValue::Compound(vec![entry(
                    "type",
                    NbtSummaryValue::String("minecraft:teleport_randomly".into()),
                )]),
            ]),
        )]),
    );
    let summary = consumable_round_trip(&saved);
    assert_eq!(
        summary.component_patch.consumable_effects,
        vec![
            ConsumeEffectSummary::ClearAllEffects,
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![VANILLA_MOB_EFFECT_POISON_ID],
                effect_tag: None,
            },
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 8.0_f32.to_bits(),
            },
            ConsumeEffectSummary::PlaySound {
                sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:burp".to_string()),
                    fixed_range_bits: Some(4.0_f32.to_bits()),
                },
            },
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 16.0_f32.to_bits(),
            },
        ]
    );
}

/// A `#tag` holder set has no built-in expansion: the 26.1 facts answer
/// `Absent` for every `minecraft:mob_effect` tag
/// (`persistent_registry_facts_26_1.rs`), so the membership comes from the
/// publication the projection was handed. A publication that claims the tag
/// projects it, and the wire decode's `effect_tag` is what carries it.
#[test]
fn consumable_effect_tags_take_their_membership_from_the_publication() {
    let saved = component_stack(
        "minecraft:milk_bucket",
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        NbtSummaryValue::Compound(vec![entry(
            "on_consume_effects",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                entry(
                    "type",
                    NbtSummaryValue::String("minecraft:remove_effects".into()),
                ),
                entry("effects", NbtSummaryValue::String("#example:bad".into())),
            ])]),
        )]),
    );
    let summary = round_trip_via(&saved, &PermissiveResolver);
    assert_eq!(
        summary.component_patch.consumable_effects,
        [ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: Vec::new(),
            effect_tag: Some("example:bad".to_string()),
        }]
    );
}
