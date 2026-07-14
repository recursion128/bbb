use bbb_protocol::packets::inventory::FireworkExplosionSummary;
use bbb_protocol::{
    canonical_f32_json_utf16_len, canonical_f64_json_utf16_len, canonical_json_string_utf16_len,
    canonical_resolvable_profile_json_utf16_len,
};

use super::super::DialogDataComponentPatchState;
use super::holder_set::DialogRegistryHolderSetState;
use super::sound_event::DialogSoundEventHolderState;
use super::DialogDataComponentValueState;

/// Returns an exact JsonOps length when every patch value has a supported semantic encoder.
/// Complex holder and recursive values remain unknown rather than falling back to raw NBT size.
pub(in crate::client::dialog::body) fn canonical_data_component_patch_json_utf16_len(
    patch: &DialogDataComponentPatchState,
) -> Result<Option<usize>, String> {
    let mut length = 2usize;
    for (index, entry) in patch.entries.iter().enumerate() {
        let value_len = if entry.removed {
            // DataComponentPatch.PatchKey selects Codec.EMPTY.codec() for removals.
            2
        } else {
            let Some(value) = entry.value_data.as_ref() else {
                return Ok(None);
            };
            validate_supported_json_floats(value)?;
            let Some(length) = canonical_component_value_json_utf16_len(value) else {
                return Ok(None);
            };
            length
        };
        let key_len = entry
            .resource_id
            .len()
            .saturating_add(2)
            .saturating_add(usize::from(entry.removed));
        length = length
            .saturating_add(usize::from(index != 0))
            .saturating_add(key_len)
            .saturating_add(1)
            .saturating_add(value_len);
    }
    Ok(Some(length))
}

fn validate_supported_json_floats(value: &DialogDataComponentValueState) -> Result<(), String> {
    let mut values = Vec::new();
    let mut double_values = Vec::new();
    match value {
        DialogDataComponentValueState::Float { bits } => values.push(*bits),
        DialogDataComponentValueState::UseEffects {
            speed_multiplier_bits,
            ..
        } => values.push(*speed_multiplier_bits),
        DialogDataComponentValueState::Food {
            saturation_bits, ..
        } => values.push(*saturation_bits),
        DialogDataComponentValueState::UseCooldown { seconds_bits, .. } => {
            values.push(*seconds_bits);
        }
        DialogDataComponentValueState::Weapon {
            disable_blocking_for_seconds_bits,
            ..
        } => values.push(*disable_blocking_for_seconds_bits),
        DialogDataComponentValueState::AttackRange {
            min_reach_bits,
            max_reach_bits,
            min_creative_reach_bits,
            max_creative_reach_bits,
            hitbox_margin_bits,
            mob_factor_bits,
        } => values.extend([
            *min_reach_bits,
            *max_reach_bits,
            *min_creative_reach_bits,
            *max_creative_reach_bits,
            *hitbox_margin_bits,
            *mob_factor_bits,
        ]),
        DialogDataComponentValueState::CustomModelData { floats_bits, .. } => {
            values.extend(floats_bits);
        }
        DialogDataComponentValueState::MapDecorations { decorations } => {
            for decoration in decorations {
                double_values.extend([decoration.x_bits, decoration.z_bits]);
                values.push(decoration.rotation_bits);
            }
        }
        DialogDataComponentValueState::BreakSound {
            sound:
                DialogSoundEventHolderState::Direct {
                    fixed_range_bits: Some(bits),
                    ..
                },
        } => values.push(*bits),
        _ => {}
    }
    if values
        .into_iter()
        .any(|bits| canonical_f32_json_utf16_len(bits).is_none())
        || double_values
            .into_iter()
            .any(|bits| canonical_f64_json_utf16_len(bits).is_none())
    {
        return Err(
            "canonical item component patch contains a non-finite Float/Double value".to_string(),
        );
    }
    Ok(())
}

fn canonical_component_value_json_utf16_len(
    value: &DialogDataComponentValueState,
) -> Option<usize> {
    Some(match value {
        DialogDataComponentValueState::Integer { value }
        | DialogDataComponentValueState::RgbColor { value } => value.to_string().len(),
        DialogDataComponentValueState::Float { bits } => canonical_f32_json_utf16_len(*bits)?,
        DialogDataComponentValueState::Boolean { value } => bool_len(*value),
        DialogDataComponentValueState::Unit => 2,
        DialogDataComponentValueState::ResourceId { resource_id } => string_len(resource_id),
        DialogDataComponentValueState::Rarity { value } => string_len(value.as_str()),
        DialogDataComponentValueState::StringEnum {
            serialized_name, ..
        } => string_len(serialized_name),
        DialogDataComponentValueState::UseEffects {
            can_sprint,
            interact_vibrations,
            speed_multiplier_bits,
        } => {
            let mut fields = Vec::new();
            if *can_sprint {
                fields.push(("can_sprint", bool_len(true)));
            }
            if !*interact_vibrations {
                fields.push(("interact_vibrations", bool_len(false)));
            }
            if *speed_multiplier_bits != 0.2f32.to_bits() {
                fields.push((
                    "speed_multiplier",
                    canonical_f32_json_utf16_len(*speed_multiplier_bits)?,
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::SwingAnimation {
            animation_type,
            duration,
        } => {
            let mut fields = Vec::new();
            if animation_type.as_str() != "whack" {
                fields.push(("type", string_len(animation_type.as_str())));
            }
            if *duration != 6 {
                fields.push(("duration", duration.to_string().len()));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::BlockState { properties } => {
            collection_len(properties.iter().map(|entry| {
                string_len(&entry.key)
                    .saturating_add(1)
                    .saturating_add(string_len(&entry.value))
            }))
        }
        DialogDataComponentValueState::Food {
            nutrition,
            saturation_bits,
            can_always_eat,
        } => {
            let mut fields = vec![
                ("nutrition", nutrition.to_string().len()),
                (
                    "saturation",
                    canonical_f32_json_utf16_len(*saturation_bits)?,
                ),
            ];
            if *can_always_eat {
                fields.push(("can_always_eat", bool_len(true)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::UseCooldown {
            seconds_bits,
            cooldown_group,
        } => {
            let mut fields = vec![("seconds", canonical_f32_json_utf16_len(*seconds_bits)?)];
            if let Some(cooldown_group) = cooldown_group {
                fields.push(("cooldown_group", string_len(cooldown_group)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Weapon {
            item_damage_per_attack,
            disable_blocking_for_seconds_bits,
        } => {
            let mut fields = Vec::new();
            if *item_damage_per_attack != 1 {
                fields.push((
                    "item_damage_per_attack",
                    item_damage_per_attack.to_string().len(),
                ));
            }
            if *disable_blocking_for_seconds_bits != 0.0f32.to_bits() {
                fields.push((
                    "disable_blocking_for_seconds",
                    canonical_f32_json_utf16_len(*disable_blocking_for_seconds_bits)?,
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::AttackRange {
            min_reach_bits,
            max_reach_bits,
            min_creative_reach_bits,
            max_creative_reach_bits,
            hitbox_margin_bits,
            mob_factor_bits,
        } => {
            let mut fields = Vec::new();
            append_non_default_float(&mut fields, "min_reach", *min_reach_bits, 0.0)?;
            append_non_default_float(&mut fields, "max_reach", *max_reach_bits, 3.0)?;
            append_non_default_float(
                &mut fields,
                "min_creative_reach",
                *min_creative_reach_bits,
                0.0,
            )?;
            append_non_default_float(
                &mut fields,
                "max_creative_reach",
                *max_creative_reach_bits,
                5.0,
            )?;
            append_non_default_float(&mut fields, "hitbox_margin", *hitbox_margin_bits, 0.3)?;
            append_non_default_float(&mut fields, "mob_factor", *mob_factor_bits, 1.0)?;
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Enchantable { value } => {
            object_len([("value", value.to_string().len())].into_iter())
        }
        DialogDataComponentValueState::Recipes { resource_ids } => array_len(
            resource_ids
                .iter()
                .map(|resource_id| string_len(resource_id)),
        ),
        DialogDataComponentValueState::TooltipDisplay {
            hide_tooltip,
            hidden_components,
        } => {
            let mut fields = Vec::new();
            if *hide_tooltip {
                fields.push(("hide_tooltip", bool_len(true)));
            }
            if !hidden_components.is_empty() {
                fields.push((
                    "hidden_components",
                    array_len(
                        hidden_components
                            .iter()
                            .map(|component| string_len(&component.resource_id)),
                    ),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::ContainerLoot { loot_table, seed } => {
            let mut fields = vec![("loot_table", string_len(loot_table))];
            if *seed != 0 {
                fields.push(("seed", seed.to_string().len()));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::CustomModelData {
            floats_bits,
            flags,
            strings,
            colors,
        } => {
            let mut fields = Vec::new();
            if !floats_bits.is_empty() {
                fields.push((
                    "floats",
                    array_len(
                        floats_bits
                            .iter()
                            .map(|bits| canonical_f32_json_utf16_len(*bits))
                            .collect::<Option<Vec<_>>>()?
                            .into_iter(),
                    ),
                ));
            }
            if !flags.is_empty() {
                fields.push((
                    "flags",
                    array_len(flags.iter().map(|value| bool_len(*value))),
                ));
            }
            if !strings.is_empty() {
                fields.push((
                    "strings",
                    array_len(strings.iter().map(|value| string_len(value))),
                ));
            }
            if !colors.is_empty() {
                fields.push((
                    "colors",
                    array_len(colors.iter().map(|value| value.to_string().len())),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::FireworkExplosion { explosion } => {
            firework_explosion_len(explosion)
        }
        DialogDataComponentValueState::Fireworks {
            flight_duration,
            explosions,
        } => {
            let mut fields = Vec::new();
            if *flight_duration != 0 {
                fields.push(("flight_duration", flight_duration.to_string().len()));
            }
            if !explosions.is_empty() {
                fields.push((
                    "explosions",
                    array_len(explosions.iter().map(firework_explosion_len)),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::LodestoneTracker { target, tracked } => {
            let mut fields = Vec::new();
            if let Some(target) = target {
                fields.push((
                    "target",
                    object_len(
                        [
                            ("dimension", string_len(&target.dimension)),
                            (
                                "pos",
                                array_len(
                                    [target.pos.x, target.pos.y, target.pos.z]
                                        .into_iter()
                                        .map(|value| value.to_string().len()),
                                ),
                            ),
                        ]
                        .into_iter(),
                    ),
                ));
            }
            if !*tracked {
                fields.push(("tracked", bool_len(false)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Profile { profile } => {
            canonical_resolvable_profile_json_utf16_len(profile)?
        }
        DialogDataComponentValueState::MapDecorations { decorations } => collection_len(
            decorations
                .iter()
                .map(|decoration| {
                    Some(
                        string_len(&decoration.id)
                            .saturating_add(1)
                            .saturating_add(object_len(
                                [
                                    ("type", string_len(&decoration.decoration_type.resource_id)),
                                    ("x", canonical_f64_json_utf16_len(decoration.x_bits)?),
                                    ("z", canonical_f64_json_utf16_len(decoration.z_bits)?),
                                    (
                                        "rotation",
                                        canonical_f32_json_utf16_len(decoration.rotation_bits)?,
                                    ),
                                ]
                                .into_iter(),
                            )),
                    )
                })
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::SuspiciousStewEffects { effects } => {
            array_len(effects.iter().map(|effect| {
                let mut fields = vec![("id", string_len(&effect.effect.resource_id))];
                if effect.duration != 160 {
                    fields.push(("duration", effect.duration.to_string().len()));
                }
                object_len(fields.into_iter())
            }))
        }
        DialogDataComponentValueState::PotDecorations { decorations } => array_len(
            [
                decorations.back.as_ref(),
                decorations.left.as_ref(),
                decorations.right.as_ref(),
                decorations.front.as_ref(),
            ]
            .into_iter()
            .map(|decoration| {
                string_len(
                    decoration
                        .map(|decoration| decoration.resource_id.as_str())
                        .unwrap_or("minecraft:brick"),
                )
            }),
        ),
        DialogDataComponentValueState::RegistryVariant { holder, .. } => {
            string_len(&holder.resource_id)
        }
        DialogDataComponentValueState::JukeboxPlayable { song } => string_len(&song.resource_id),
        DialogDataComponentValueState::PaintingVariant { variant } => {
            string_len(&variant.resource_id)
        }
        DialogDataComponentValueState::DamageType { damage_type } => {
            string_len(&damage_type.resource_id)
        }
        DialogDataComponentValueState::Repairable { items } => {
            object_len([("items", registry_holder_set_len(items)?)].into_iter())
        }
        DialogDataComponentValueState::ProvidesBannerPatterns { patterns } => {
            registry_holder_set_len(patterns)?
        }
        DialogDataComponentValueState::DamageResistant { types } => {
            object_len([("types", registry_holder_set_len(types)?)].into_iter())
        }
        DialogDataComponentValueState::BreakSound { sound } => sound_event_len(sound)?,
        _ => return None,
    })
}

fn sound_event_len(sound: &DialogSoundEventHolderState) -> Option<usize> {
    match sound {
        DialogSoundEventHolderState::Named { resource_id, .. } => Some(string_len(resource_id)),
        DialogSoundEventHolderState::Direct {
            sound_id,
            fixed_range_bits,
        } => {
            let mut fields = vec![("sound_id", string_len(sound_id))];
            if let Some(bits) = fixed_range_bits {
                fields.push(("range", canonical_f32_json_utf16_len(*bits)?));
            }
            Some(object_len(fields.into_iter()))
        }
    }
}

fn registry_holder_set_len(holder_set: &DialogRegistryHolderSetState) -> Option<usize> {
    match holder_set.holder_set_kind.as_str() {
        "named" => holder_set
            .tag
            .as_ref()
            .map(|tag| string_len(&format!("#{tag}"))),
        "direct" => Some(if let [entry] = holder_set.entries.as_slice() {
            string_len(&entry.resource_id)
        } else {
            array_len(
                holder_set
                    .entries
                    .iter()
                    .map(|entry| string_len(&entry.resource_id)),
            )
        }),
        _ => None,
    }
}

fn firework_explosion_len(explosion: &FireworkExplosionSummary) -> usize {
    let mut fields = vec![("shape", string_len(explosion.shape.as_str()))];
    if !explosion.colors.is_empty() {
        fields.push((
            "colors",
            array_len(explosion.colors.iter().map(|value| value.to_string().len())),
        ));
    }
    if !explosion.fade_colors.is_empty() {
        fields.push((
            "fade_colors",
            array_len(
                explosion
                    .fade_colors
                    .iter()
                    .map(|value| value.to_string().len()),
            ),
        ));
    }
    if explosion.has_trail {
        fields.push(("has_trail", bool_len(true)));
    }
    if explosion.has_twinkle {
        fields.push(("has_twinkle", bool_len(true)));
    }
    object_len(fields.into_iter())
}

fn append_non_default_float(
    fields: &mut Vec<(&'static str, usize)>,
    name: &'static str,
    bits: u32,
    default: f32,
) -> Option<()> {
    if bits != default.to_bits() {
        fields.push((name, canonical_f32_json_utf16_len(bits)?));
    }
    Some(())
}

fn bool_len(value: bool) -> usize {
    if value {
        4
    } else {
        5
    }
}

fn string_len(value: &str) -> usize {
    canonical_json_string_utf16_len(value)
}

fn object_len<'a>(fields: impl Iterator<Item = (&'a str, usize)>) -> usize {
    collection_len(
        fields
            .map(|(name, value_len)| string_len(name).saturating_add(1).saturating_add(value_len)),
    )
}

fn array_len(values: impl Iterator<Item = usize>) -> usize {
    collection_len(values)
}

fn collection_len(values: impl Iterator<Item = usize>) -> usize {
    values.enumerate().fold(2usize, |length, (index, value)| {
        length
            .saturating_add(usize::from(index != 0))
            .saturating_add(value)
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::item_component_value_types::{
        FireworkExplosionShapeSummary, SwingAnimationTypeSummary,
    };
    use bbb_protocol::packets::chunks::BlockPos;
    use bbb_protocol::packets::inventory::{
        LodestoneTargetSummary, PlayerSkinPatchSummary, ResolvableProfileKindSummary,
        ResolvableProfileSummary,
    };

    use super::super::holder_set::DialogRegistryHolderReferenceState;
    use super::super::map_decorations::DialogMapDecorationState;
    use super::*;

    #[test]
    fn canonicalizes_flat_record_defaults_and_required_fields() {
        assert_exact(
            DialogDataComponentValueState::UseEffects {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier_bits: 0.2f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::UseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier_bits: 0.5f32.to_bits(),
            },
            r#"{"can_sprint":true,"interact_vibrations":false,"speed_multiplier":0.5}"#,
        );
        assert_exact(
            DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Whack,
                duration: 6,
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 9,
            },
            r#"{"type":"stab","duration":9}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Food {
                nutrition: 4,
                saturation_bits: 0.5f32.to_bits(),
                can_always_eat: true,
            },
            r#"{"nutrition":4,"saturation":0.5,"can_always_eat":true}"#,
        );
        assert_exact(
            DialogDataComponentValueState::UseCooldown {
                seconds_bits: 1.25f32.to_bits(),
                cooldown_group: Some("minecraft:test".to_string()),
            },
            r#"{"seconds":1.25,"cooldown_group":"minecraft:test"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Weapon {
                item_damage_per_attack: 1,
                disable_blocking_for_seconds_bits: 0.0f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::AttackRange {
                min_reach_bits: 0.0f32.to_bits(),
                max_reach_bits: 3.0f32.to_bits(),
                min_creative_reach_bits: 0.0f32.to_bits(),
                max_creative_reach_bits: 5.0f32.to_bits(),
                hitbox_margin_bits: 0.3f32.to_bits(),
                mob_factor_bits: 1.0f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::Enchantable { value: 12 },
            r#"{"value":12}"#,
        );
    }

    #[test]
    fn canonicalizes_flat_maps_lists_metadata_and_escaped_strings() {
        assert_exact(
            DialogDataComponentValueState::BlockState {
                properties: vec![super::super::DialogStringMapEntryState {
                    key: "a\"".to_string(),
                    value: "line\n".to_string(),
                }],
            },
            r#"{"a\"":"line\n"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Recipes {
                resource_ids: vec!["minecraft:first".to_string(), "example:second".to_string()],
            },
            r#"["minecraft:first","example:second"]"#,
        );
        assert_exact(
            DialogDataComponentValueState::TooltipDisplay {
                hide_tooltip: true,
                hidden_components: vec![super::super::DialogDataComponentReferenceState {
                    type_id: 1,
                    resource_id: "minecraft:max_stack_size".to_string(),
                }],
            },
            r#"{"hide_tooltip":true,"hidden_components":["minecraft:max_stack_size"]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::ContainerLoot {
                loot_table: "minecraft:chests/test".to_string(),
                seed: 0,
            },
            r#"{"loot_table":"minecraft:chests/test"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::CustomModelData {
                floats_bits: vec![0.5f32.to_bits()],
                flags: vec![true, false],
                strings: vec!["quote\"\u{2028}😀".to_string()],
                colors: vec![0x12_34_56],
            },
            r#"{"floats":[0.5],"flags":[true,false],"strings":["quote\"\u2028😀"],"colors":[1193046]}"#,
        );
        let non_finite_patch = DialogDataComponentPatchState {
            entries: vec![super::super::super::DialogDataComponentPatchEntryState {
                type_id: 0,
                resource_id: "minecraft:custom_model_data".to_string(),
                removed: false,
                value: None,
                value_data: Some(DialogDataComponentValueState::CustomModelData {
                    floats_bits: vec![f32::NAN.to_bits()],
                    flags: Vec::new(),
                    strings: Vec::new(),
                    colors: Vec::new(),
                }),
            }],
        };
        assert!(
            canonical_data_component_patch_json_utf16_len(&non_finite_patch)
                .unwrap_err()
                .contains("non-finite")
        );
    }

    #[test]
    fn canonicalizes_firework_and_lodestone_nested_records() {
        let explosion = FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x12_34_56],
            fade_colors: Vec::new(),
            has_trail: true,
            has_twinkle: false,
        };
        assert_exact(
            DialogDataComponentValueState::FireworkExplosion {
                explosion: explosion.clone(),
            },
            r#"{"shape":"star","colors":[1193046],"has_trail":true}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Fireworks {
                flight_duration: 0,
                explosions: Vec::new(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::Fireworks {
                flight_duration: 2,
                explosions: vec![explosion],
            },
            r#"{"flight_duration":2,"explosions":[{"shape":"star","colors":[1193046],"has_trail":true}]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::LodestoneTracker {
                target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: BlockPos { x: 1, y: -2, z: 3 },
                }),
                tracked: false,
            },
            r#"{"target":{"dimension":"minecraft:overworld","pos":[1,-2,3]},"tracked":false}"#,
        );
    }

    #[test]
    fn canonicalizes_profile_through_the_protocol_owner() {
        assert_exact(
            DialogDataComponentValueState::Profile {
                profile: ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::Partial,
                    uuid: None,
                    name: Some("Alex".to_string()),
                    properties: Vec::new(),
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary::default(),
                },
            },
            r#"{"name":"Alex"}"#,
        );
    }

    #[test]
    fn canonicalizes_map_decoration_maps_and_java_numbers() {
        assert_exact(
            DialogDataComponentValueState::MapDecorations {
                decorations: vec![DialogMapDecorationState {
                    id: "home\"".to_string(),
                    decoration_type: DialogRegistryHolderReferenceState {
                        holder_id: 0,
                        resource_id: "minecraft:mansion".to_string(),
                    },
                    x_bits: (-120.5f64).to_bits(),
                    z_bits: 64.0f64.to_bits(),
                    rotation_bits: 180.0f32.to_bits(),
                }],
            },
            r#"{"home\"":{"type":"minecraft:mansion","x":-120.5,"z":64.0,"rotation":180.0}}"#,
        );
    }

    #[test]
    fn canonicalizes_stew_and_pot_holder_collections() {
        let holder = |holder_id, resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id,
            resource_id: resource_id.to_string(),
        };
        assert_exact(
            DialogDataComponentValueState::SuspiciousStewEffects {
                effects: vec![
                    super::super::holder_set::DialogSuspiciousStewEffectState {
                        effect: holder(0, "minecraft:speed"),
                        duration: 160,
                    },
                    super::super::holder_set::DialogSuspiciousStewEffectState {
                        effect: holder(18, "minecraft:poison"),
                        duration: -20,
                    },
                ],
            },
            r#"[{"id":"minecraft:speed"},{"id":"minecraft:poison","duration":-20}]"#,
        );
        assert_exact(
            DialogDataComponentValueState::PotDecorations {
                decorations: super::super::holder_set::DialogPotDecorationsState {
                    back: None,
                    left: Some(holder(0, "minecraft:angler_pottery_sherd")),
                    right: None,
                    front: None,
                },
            },
            r#"["minecraft:brick","minecraft:angler_pottery_sherd","minecraft:brick","minecraft:brick"]"#,
        );
    }

    #[test]
    fn canonicalizes_registry_fixed_named_holders() {
        let holder = |resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id: 1,
            resource_id: resource_id.to_string(),
        };
        for (value, expected) in [
            (
                DialogDataComponentValueState::RegistryVariant {
                    registry: "minecraft:wolf_variant".to_string(),
                    holder: holder("example:selected"),
                },
                r#""example:selected""#,
            ),
            (
                DialogDataComponentValueState::JukeboxPlayable {
                    song: holder("minecraft:cat"),
                },
                r#""minecraft:cat""#,
            ),
            (
                DialogDataComponentValueState::PaintingVariant {
                    variant: holder("minecraft:kebab"),
                },
                r#""minecraft:kebab""#,
            ),
            (
                DialogDataComponentValueState::DamageType {
                    damage_type: holder("minecraft:generic_kill"),
                },
                r#""minecraft:generic_kill""#,
            ),
        ] {
            assert_exact(value, expected);
        }
    }

    #[test]
    fn canonicalizes_registry_holder_sets_in_compact_and_tag_forms() {
        let entry = |resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id: 0,
            resource_id: resource_id.to_string(),
        };
        let direct =
            |entries: Vec<DialogRegistryHolderReferenceState>| DialogRegistryHolderSetState {
                holder_set_kind: "direct".to_string(),
                tag: None,
                entries,
            };
        assert_exact(
            DialogDataComponentValueState::Repairable {
                items: DialogRegistryHolderSetState {
                    holder_set_kind: "named".to_string(),
                    tag: Some("minecraft:planks".to_string()),
                    entries: Vec::new(),
                },
            },
            r##"{"items":"#minecraft:planks"}"##,
        );
        assert_exact(
            DialogDataComponentValueState::ProvidesBannerPatterns {
                patterns: direct(vec![entry("minecraft:base")]),
            },
            r#""minecraft:base""#,
        );
        assert_exact(
            DialogDataComponentValueState::DamageResistant {
                types: direct(vec![entry("minecraft:generic"), entry("minecraft:fall")]),
            },
            r#"{"types":["minecraft:generic","minecraft:fall"]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::ProvidesBannerPatterns {
                patterns: direct(Vec::new()),
            },
            "[]",
        );
    }

    #[test]
    fn canonicalizes_registry_file_sound_event_holders() {
        for (sound, expected) in [
            (
                DialogSoundEventHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:entity.item.break".to_string(),
                },
                r#""minecraft:entity.item.break""#,
            ),
            (
                DialogSoundEventHolderState::Direct {
                    sound_id: "example:glass".to_string(),
                    fixed_range_bits: None,
                },
                r#"{"sound_id":"example:glass"}"#,
            ),
            (
                DialogSoundEventHolderState::Direct {
                    sound_id: "example:glass".to_string(),
                    fixed_range_bits: Some(12.5f32.to_bits()),
                },
                r#"{"sound_id":"example:glass","range":12.5}"#,
            ),
        ] {
            assert_exact(
                DialogDataComponentValueState::BreakSound { sound },
                expected,
            );
        }
    }

    fn assert_exact(value: DialogDataComponentValueState, expected: &str) {
        assert_eq!(
            canonical_component_value_json_utf16_len(&value),
            Some(expected.encode_utf16().count())
        );
    }
}
