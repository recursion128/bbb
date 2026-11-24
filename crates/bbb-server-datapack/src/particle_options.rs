//! Particle content projection shared by datapack loading and commands.

mod codec;

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_values,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{vanilla_block_type_for_resource_id, BlockTypeCatalogEntry};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id;
use bbb_protocol::item_types::{vanilla_item_type_for_resource_id, VANILLA_ITEM_AIR_ID};
use bbb_protocol::packets::{
    data_components::validate_persistent_data_component_patch_summary_with_registry, BlockPos,
    DataComponentPatchSummary, ItemStackTemplateSummary, ParticleOptions, ParticlePayload, Vec3d,
};
use bbb_protocol::particle_types::{
    vanilla_particle_type_for_resource_id, VanillaParticleCodecFamily,
};
use bbb_protocol::{JavaString, NbtSummaryValue};
use bbb_registry_codec::registry_codec::RegistryReferenceLookup;
use bbb_registry_codec::{
    read_item_component_into_with_content_registry, read_item_component_with_content_registry,
    validate_persistent_component_nbt, ItemStackPersistentProjectionError,
    PersistentItemStackHolderResolver, PersistentNbtDecodeContext,
};
use bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY;

use self::codec::{
    either, fixed_ints, fixed_list, float, int, map, registry_element, string,
    try_parse_identifier, Decoded,
};

/// Projects particle content into its protocol payload against the frozen registry publication.
pub fn particle_payload_from_nbt(
    resource_id: &str,
    options: &NbtSummaryValue,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Result<ParticlePayload, String> {
    let content_registry = lookup.content_registry();
    let (particle_id, codec_family) =
        if let Some(entry) = vanilla_particle_type_for_resource_id(resource_id) {
            (entry.id, entry.codec_family)
        } else if let Some(spec) = content_registry.particle_by_resource_id(resource_id) {
            (spec.id, VanillaParticleCodecFamily::Simple)
        } else {
            return Err(format!("unknown particle: {resource_id}"));
        };
    decode_options(codec_family, options, lookup)?
        .payload_with_registry(particle_id, content_registry)
        .map_err(|error| error.to_string())
}

/// `type.codec().codec().parse(ops, extraData)` (`ParticleArgument.java:81`),
/// each option class's `MapCodec` as `ParticleTypes` registers it
/// (`ParticleTypes.java:14-154`).
fn decode_options(
    family: VanillaParticleCodecFamily,
    value: &NbtSummaryValue,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Decoded<ParticleOptions> {
    let content_registry = lookup.content_registry();
    let input = map(value)?;
    Ok(match family {
        VanillaParticleCodecFamily::Simple => ParticleOptions::Simple,
        // `BlockParticleOption.java:19-21`.
        VanillaParticleCodecFamily::Block => ParticleOptions::Block {
            state_id: input.field("block_state", |value| block_state(value, content_registry))?,
        },
        // `PowerParticleOption.java:13-15`.
        VanillaParticleCodecFamily::Power => ParticleOptions::Power {
            power: input.optional_field("power", 1.0, float)?,
        },
        // `DustParticleOptions.java:15-20`.
        VanillaParticleCodecFamily::Dust => ParticleOptions::Dust {
            color: input.field("color", rgb_color)?,
            scale: input.field("scale", scale)?,
        },
        // `DustColorTransitionOptions.java:15-22`.
        VanillaParticleCodecFamily::DustColorTransition => ParticleOptions::DustColorTransition {
            from_color: input.field("from_color", rgb_color)?,
            to_color: input.field("to_color", rgb_color)?,
            scale: input.field("scale", scale)?,
        },
        // `SpellParticleOption.java:17-25`.
        VanillaParticleCodecFamily::Spell => ParticleOptions::Spell {
            color: input.optional_field("color", -1, rgb_color)?,
            power: input.optional_field("power", 1.0, float)?,
        },
        // `ColorParticleOption.java:14-16`.
        VanillaParticleCodecFamily::Color => ParticleOptions::Color {
            color: input.field("color", argb_color)?,
        },
        // `SculkChargeParticleOptions.java:11-13`.
        VanillaParticleCodecFamily::SculkCharge => ParticleOptions::SculkCharge {
            roll: input.field("roll", float)?,
        },
        // `ItemParticleOption.java:13-15`.
        VanillaParticleCodecFamily::Item => {
            let declared = input.field("item", |value| item_stack_template(value, lookup))?;
            project_template(declared, lookup)?
        }
        // `VibrationParticleOption.java:16-22`.
        VanillaParticleCodecFamily::Vibration => ParticleOptions::Vibration {
            destination: input.field("destination", vibration_destination)?,
            arrival_in_ticks: input.field("arrival_in_ticks", int)?,
        },
        // `TrailParticleOption.java:12-19`.
        VanillaParticleCodecFamily::Trail => ParticleOptions::Trail {
            target: input.field("target", vec3)?,
            color: input.field("color", rgb_color)?,
            duration: input.field("duration", |value| int_range(value, 1, i32::MAX))?,
        },
        // `ShriekParticleOption.java:11-13`.
        VanillaParticleCodecFamily::Shriek => ParticleOptions::Shriek {
            delay: input.field("delay", int)?,
        },
    })
}

/// `ExtraCodecs.RGB_COLOR_CODEC` (ExtraCodecs.java:139) over `VECTOR3F`
/// (`:94-98`).
fn rgb_color(value: &NbtSummaryValue) -> Decoded<i32> {
    either(int(value), || {
        let v = fixed_list(value, 3, float)?;
        Ok(color_from_float(1.0, v[0], v[1], v[2]))
    })
}

/// `ExtraCodecs.ARGB_COLOR_CODEC` (ExtraCodecs.java:140) over `VECTOR4F`
/// (`:104-109`), alpha last.
fn argb_color(value: &NbtSummaryValue) -> Decoded<i32> {
    either(int(value), || {
        let v = fixed_list(value, 4, float)?;
        Ok(color_from_float(v[3], v[0], v[1], v[2]))
    })
}

/// `ARGB.colorFromFloat` (ARGB.java:209-211): each channel is
/// `Mth.floor(value * 255.0F)` (`:225-227`, Mth.java:61-63), masked by `color`
/// (ARGB.java:68-70).
fn color_from_float(alpha: f32, red: f32, green: f32, blue: f32) -> i32 {
    let channel = |value: f32| ((value * 255.0).floor() as i32) & 0xff;
    channel(alpha) << 24 | channel(red) << 16 | channel(green) << 8 | channel(blue)
}

/// `ScalableParticleOptionsBase.SCALE` (ScalableParticleOptionsBase.java:10-11).
fn scale(value: &NbtSummaryValue) -> Decoded<f32> {
    let scale = float(value)?;
    if (0.01..=4.0).contains(&scale) {
        Ok(scale)
    } else {
        Err("scale out of range".into())
    }
}

/// `ExtraCodecs.intRange` (ExtraCodecs.java:429-431); `POSITIVE_INT` (`:149`)
/// is the range from one.
fn int_range(value: &NbtSummaryValue, min: i32, max: i32) -> Decoded<i32> {
    let n = int(value)?;
    if (min..=max).contains(&n) {
        Ok(n)
    } else {
        Err(format!("{n} out of range"))
    }
}

/// `Vec3.CODEC` (Vec3.java:19-24).
fn vec3(value: &NbtSummaryValue) -> Decoded<Vec3d> {
    let v = fixed_list(value, 3, codec::double)?;
    Ok(Vec3d {
        x: v[0],
        y: v[1],
        z: v[2],
    })
}

/// `BlockParticleOption.BLOCK_STATE_CODEC` (BlockParticleOption.java:13-15):
/// a `BlockState.CODEC` compound, or a block name for its default state.
fn block_state(value: &NbtSummaryValue, content_registry: &ContentRegistry) -> Decoded<i32> {
    either(block_state_compound(value, content_registry), || {
        block_default_state_id(value, content_registry)
    })
}

enum ResolvedBlock<'a> {
    Vanilla(&'static BlockTypeCatalogEntry),
    Mod(&'a bbb_protocol::content_registry::ModBlockSpec),
}

fn block<'a>(
    value: &NbtSummaryValue,
    content_registry: &'a ContentRegistry,
) -> Decoded<ResolvedBlock<'a>> {
    registry_element(value, |id| {
        vanilla_block_type_for_resource_id(id)
            .map(ResolvedBlock::Vanilla)
            .or_else(|| {
                content_registry
                    .block_by_resource_id(id)
                    .map(ResolvedBlock::Mod)
            })
    })
}

fn block_default_state_id(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Decoded<i32> {
    Ok(match block(value, content_registry)? {
        ResolvedBlock::Vanilla(entry) => default_state_id(entry),
        ResolvedBlock::Mod(spec) => spec.first_state_id,
    })
}

fn default_state_id(block: &BlockTypeCatalogEntry) -> i32 {
    vanilla_block_default_state_id(block.id).expect("every built-in block has a default state")
}

/// `BlockState.CODEC` (BlockState.java:9) over `StateHolder.codec`
/// (StateHolder.java:148-162): `Name` picks the block, and `Properties` is
/// read leniently, each property keeping the default state's value unless the
/// compound names one of its own (`StateDefinition.java:133-138`).
fn block_state_compound(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Decoded<i32> {
    let input = map(value)?;
    match input.field("Name", |name| block(name, content_registry))? {
        ResolvedBlock::Vanilla(entry) => {
            let default = default_state_id(entry);
            let Some(NbtSummaryValue::Compound(declared)) = input.get("Properties") else {
                return Ok(default);
            };
            let identity =
                vanilla_block_state_identity(default).expect("every default state has an identity");
            let properties: BTreeMap<String, String> = identity
                .properties
                .iter()
                .map(|(name, default_value)| {
                    let declared = declared
                        .iter()
                        .find(|entry| entry.name == name.as_str())
                        .and_then(|entry| string(&entry.value).ok())
                        .filter(|value| {
                            vanilla_block_property_values(&identity.name, name)
                                .is_some_and(|values| values.contains(&value.as_str()))
                        });
                    (
                        name.clone(),
                        declared.unwrap_or_else(|| default_value.clone()),
                    )
                })
                .collect();
            Ok(
                vanilla_block_state_id_by_name_and_properties(&identity.name, &properties)
                    .unwrap_or(default),
            )
        }
        ResolvedBlock::Mod(spec) => {
            let mut offset = 0u16;
            if let Some(NbtSummaryValue::Compound(declared)) = input.get("Properties") {
                if let Some(parsed) = declared
                    .iter()
                    .rev()
                    .find(|entry| entry.name == MOD_STATE_OFFSET_PROPERTY)
                    .and_then(|entry| string(&entry.value).ok())
                    .and_then(|value| value.parse::<u16>().ok())
                    .filter(|&parsed| parsed < spec.state_count)
                {
                    offset = parsed;
                }
            }
            Ok(spec.first_state_id + i32::from(offset))
        }
    }
}

/// `VibrationParticleOption.SAFE_POSITION_SOURCE_CODEC`
/// (VibrationParticleOption.java:14-15) over `PositionSource.CODEC`
/// (PositionSource.java:14), dispatched on `type`.
fn vibration_destination(value: &NbtSummaryValue) -> Decoded<BlockPos> {
    let input = map(value)?;
    let block = input.field("type", |kind| {
        registry_element(kind, |id| match id {
            "minecraft:block" => Some(true),
            "minecraft:entity" => Some(false),
            _ => None,
        })
    })?;
    if !block {
        // `EntityPositionSource.java:20-26` still decodes before the safe
        // codec refuses it, but either way the input is refused.
        return Err("entity position sources are not allowed".into());
    }
    // `BlockPositionSource.java:13-15` over `BlockPos.CODEC` (BlockPos.java:34-38).
    let v = input.field("pos", |pos| fixed_ints(pos, 3))?;
    Ok(BlockPos {
        x: v[0],
        y: v[1],
        z: v[2],
    })
}

/// What `ItemStackTemplate.CODEC` (ItemStackTemplate.java:21-29) read, before
/// its component values are projected.
struct DeclaredTemplate<'a> {
    item_id: i32,
    count: i32,
    patch: Vec<PatchEntry<'a>>,
}

/// One `DataComponentPatch` entry: a value to set, or a removal.
struct PatchEntry<'a> {
    type_id: i32,
    value: Option<&'a NbtSummaryValue>,
}

fn item_stack_template<'a>(
    value: &'a NbtSummaryValue,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Decoded<DeclaredTemplate<'a>> {
    let content_registry = lookup.content_registry();
    let full = || {
        let input = map(value)?;
        Ok(DeclaredTemplate {
            item_id: input.field("id", |value| item(value, content_registry))?,
            count: input.optional_field("count", 1, |count| int_range(count, 1, 99))?,
            patch: input.optional_field("components", Vec::new(), |patch| {
                component_patch(patch, lookup)
            })?,
        })
    };
    either(full(), || {
        Ok(DeclaredTemplate {
            item_id: item(value, content_registry)?,
            count: 1,
            patch: Vec::new(),
        })
    })
}

/// `Item.CODEC` (Item.java:99-101).
fn item(value: &NbtSummaryValue, content_registry: &ContentRegistry) -> Decoded<i32> {
    let item_id = registry_element(value, |id| {
        vanilla_item_type_for_resource_id(id)
            .map(|entry| entry.id)
            .or_else(|| content_registry.item_by_resource_id(id).map(|spec| spec.id))
    })?;
    if item_id == VANILLA_ITEM_AIR_ID {
        Err("item must not be air".into())
    } else {
        Ok(item_id)
    }
}

/// `DataComponentPatch.PatchKey` (DataComponentPatch.java:289-317).
#[derive(Clone, Copy, PartialEq)]
struct PatchKey {
    type_id: i32,
    resource_id: &'static str,
    removed: bool,
}

fn patch_key(name: &JavaString) -> Decoded<PatchKey> {
    let name = name
        .try_to_string()
        .map_err(|_| "not a component identifier")?;
    let (removed, name) = match name.strip_prefix('!') {
        Some(rest) => (true, rest),
        None => (false, name.as_str()),
    };
    let component = try_parse_identifier(name)
        .and_then(|id| vanilla_data_component_type_for_resource_id(&id))
        .filter(|component| component.persistent)
        .ok_or_else(|| format!("unknown persistent component: {name}"))?;
    Ok(PatchKey {
        type_id: component.type_id,
        resource_id: component.resource_id,
        removed,
    })
}

/// `DataComponentPatch.CODEC` (DataComponentPatch.java:23-40): every entry
/// must decode, a repeated key is refused, and a later entry for the same
/// component type replaces an earlier one.
fn component_patch<'a>(
    value: &'a NbtSummaryValue,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Decoded<Vec<PatchEntry<'a>>> {
    let content_registry = lookup.content_registry();
    let context = PersistentNbtDecodeContext::with_resolver(lookup);
    let mut seen: Vec<PatchKey> = Vec::new();
    let mut patch: Vec<PatchEntry<'a>> = Vec::new();
    for entry in map(value)?.entries() {
        let key = patch_key(&entry.name)?;
        let value = if key.removed {
            // `Codec.EMPTY.codec()` still asks for a map (`:316`).
            map(&entry.value)?;
            None
        } else {
            if content_registry.is_vanilla_only() {
                validate_persistent_component_nbt(
                    key.type_id,
                    key.resource_id,
                    &entry.value,
                    context,
                )
                .map_err(|error| error.to_string())?;
            } else {
                read_item_component_with_content_registry(
                    key.type_id,
                    &entry.value,
                    lookup,
                    content_registry,
                )
                .map_err(|error| error.to_string())?;
            }
            Some(&entry.value)
        };
        if seen.contains(&key) {
            return Err(format!("duplicate component: {}", key.resource_id));
        }
        seen.push(key);
        match patch.iter_mut().find(|entry| entry.type_id == key.type_id) {
            Some(entry) => entry.value = value,
            None => patch.push(PatchEntry {
                type_id: key.type_id,
                value,
            }),
        }
    }
    Ok(patch)
}

/// The decoded template as the datapack template runtime builds it, with the
/// patch in the order `DataComponentPatch.STREAM_CODEC` writes it: set entries
/// then removals, each in map order (DataComponentPatch.java:102-143).
fn project_template(
    declared: DeclaredTemplate<'_>,
    lookup: &(impl RegistryReferenceLookup + PersistentItemStackHolderResolver),
) -> Decoded<ParticleOptions> {
    let content_registry = lookup.content_registry();
    // `/particle` exposes the template projection's error context to the player.
    let invalid = |error: ItemStackPersistentProjectionError| {
        ItemStackPersistentProjectionError::InvalidPersistentComponent {
            type_id: -1,
            resource_id: "minecraft:recipe",
            detail: error.to_string(),
        }
        .to_string()
    };
    let mut patch = DataComponentPatchSummary::default();
    for entry in &declared.patch {
        match entry.value {
            Some(value) => {
                read_item_component_into_with_content_registry(
                    entry.type_id,
                    value,
                    lookup,
                    content_registry,
                    &mut patch,
                )
                .map_err(&invalid)?;
            }
            None => patch.removed_type_ids.push(entry.type_id),
        }
    }
    patch.added_type_ids.sort_unstable();
    patch.removed_type_ids.sort_unstable();
    validate_persistent_data_component_patch_summary_with_registry(&patch, content_registry)
        .map_err(|error| {
            invalid(ItemStackPersistentProjectionError::InvalidWireProjection {
                detail: error.to_string(),
            })
        })?;
    patch.added_type_ids.clear();
    patch.removed_type_ids.clear();
    for entry in &declared.patch {
        if entry.value.is_some() {
            patch.added_type_ids.push(entry.type_id);
        } else {
            patch.removed_type_ids.push(entry.type_id);
        }
    }
    Ok(ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
        item_id: declared.item_id,
        count: declared.count,
        component_patch: patch.into(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frozen_lookup::FrozenDatapackRegistryLookup;
    use crate::registry_data_loader::DatapackLoadLimits;
    use crate::startup::{DatapackStartup, DatapackStartupOptions};
    use crate::vanilla_snapshot::VanillaDatapackSnapshot;
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    };
    use bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_SWORD_ID;
    use bbb_protocol::packets::{
        DataComponentPatchSummary, ItemEnchantmentSummary, ItemStackTemplateSummary,
    };

    #[test]
    fn item_particle_projects_custom_enchantment_from_frozen_startup() {
        let root = std::env::temp_dir().join(format!("bbb-particle-holder-{}", std::process::id()));
        let pack = root.join("particle");
        std::fs::create_dir_all(pack.join("data/example/enchantment")).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{"pack":{"description":"test","pack_format":1000,"supported_formats":[0,10000]}}"#,
        )
        .unwrap();
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        std::fs::write(
            pack.join("data/example/enchantment/particle.json"),
            snapshot.read_path("enchantment/sharpness.json").unwrap(),
        )
        .unwrap();
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        });
        std::fs::remove_dir_all(root).unwrap();
        let startup = startup.unwrap();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
        let holder_id = lookup
            .holder_id("minecraft:enchantment", "example:particle")
            .unwrap();
        let (_, options) = bbb_protocol::parse_command_snbt_value_prefix(
            r#"{type:"minecraft:item",item:{id:"minecraft:diamond_sword",components:{"minecraft:enchantments":{"example:particle":3},"minecraft:damage":7}}}"#,
        ).unwrap();
        let payload = particle_payload_from_nbt("minecraft:item", &options, &lookup).unwrap();
        let expected = ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
            item_id: VANILLA_ITEM_DIAMOND_SWORD_ID,
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                ],
                damage: Some(7),
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id,
                    level: 3,
                }],
                ..Default::default()
            }
            .into(),
        }))
        .payload(bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ITEM_ID)
        .unwrap();
        assert_eq!(payload, expected);
        let (_, missing) = bbb_protocol::parse_command_snbt_value_prefix(
            r#"{item:{id:"stone",components:{enchantments:{"bbb:missing":1}}}}"#,
        )
        .unwrap();
        assert_eq!(
            particle_payload_from_nbt("minecraft:item", &missing, &lookup).unwrap_err(),
            "not a string",
        );
    }

    #[test]
    fn oversized_nbt_byte_array_in_fixed_list_is_rejected_without_materializing() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);
        let options = NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::ByteArray(vec![1; 4 * 1024 * 1024]),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "scale".into(),
                value: NbtSummaryValue::Float(1.0f32.to_bits()),
            },
        ]);
        let err = particle_payload_from_nbt("minecraft:dust", &options, &lookup).unwrap_err();
        assert_eq!(err, "expected 3 elements");
    }

    #[test]
    fn particle_payload_supports_custom_mod_items_blocks_and_particles() {
        use bbb_protocol::content_registry::{
            ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration,
            ModParticleRegistration,
        };

        let mut builder = ContentRegistryBuilder::new();
        builder.register_item(ModItemRegistration {
            resource_id: "example:ruby_ingot".to_string(),
            max_stack_size: 16,
            is_block_item: false,
        });
        builder.register_block(ModBlockRegistration {
            resource_id: "example:obsidian_brick".to_string(),
            state_count: 2,
            light_emission: 0,
            destroy_speed: 3.0,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        builder.register_particle(ModParticleRegistration {
            resource_id: "example:ember".to_string(),
            override_limiter: false,
        });
        let content_registry = builder.freeze_for_session().unwrap();
        let ruby_id = content_registry
            .item_by_resource_id("example:ruby_ingot")
            .unwrap()
            .id;
        let brick = content_registry
            .block_by_resource_id("example:obsidian_brick")
            .unwrap();
        let ember_id = content_registry
            .particle_by_resource_id("example:ember")
            .unwrap()
            .id;

        let startup = DatapackStartup::load_with_content_registry(
            &DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
            &content_registry,
        )
        .unwrap();
        let lookup = FrozenDatapackRegistryLookup::from_startup(&startup);

        let (_, item_opts) =
            bbb_protocol::parse_command_snbt_value_prefix(r#"{item:{id:"example:ruby_ingot"}}"#)
                .unwrap();
        let item_payload =
            particle_payload_from_nbt("minecraft:item", &item_opts, &lookup).unwrap();
        assert_eq!(
            item_payload,
            ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
                item_id: ruby_id,
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }))
            .payload_with_registry(
                bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ITEM_ID,
                &content_registry,
            )
            .unwrap()
        );

        let (_, block_opts) = bbb_protocol::parse_command_snbt_value_prefix(
            r#"{block_state:{Name:"example:obsidian_brick",Properties:{bbb_state_offset:"1"}}}"#,
        )
        .unwrap();
        let block_payload =
            particle_payload_from_nbt("minecraft:block", &block_opts, &lookup).unwrap();
        assert_eq!(
            block_payload,
            ParticleOptions::Block {
                state_id: brick.first_state_id + 1,
            }
            .payload_with_registry(
                bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID,
                &content_registry,
            )
            .unwrap()
        );

        let (_, simple_opts) = bbb_protocol::parse_command_snbt_value_prefix(r#"{}"#).unwrap();
        let simple_payload =
            particle_payload_from_nbt("example:ember", &simple_opts, &lookup).unwrap();
        assert_eq!(
            simple_payload,
            ParticleOptions::Simple
                .payload_with_registry(ember_id, &content_registry)
                .unwrap()
        );
    }
}
