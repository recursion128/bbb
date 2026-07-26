use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    block_registry_declarations::{
        copy_base_from_properties, copy_base_from_register_helper, has_properties_of,
        load_generated_block_declarations,
    },
    generated_java_expression::GeneratedJavaExpression,
    resources::ResourceLocation,
    PackRoots,
};

const GENERATED_SOUND_TYPE_DECLARATIONS: &str =
    include_str!("../data/sound_type_declarations_26_1.json");

#[derive(Debug, Deserialize)]
struct SoundTypeSourceCatalog {
    vanilla_version: String,
    sources: Vec<String>,
    declarations: Vec<SoundTypeSourceDeclaration>,
}

#[derive(Debug, Deserialize)]
struct SoundTypeSourceDeclaration {
    constant: String,
    declaration_order: usize,
    volume_expression: String,
    pitch_expression: String,
    break_sound: String,
    place_sound: String,
    hit_sound: String,
    source_path: String,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSoundProfile {
    pub break_sound: String,
    pub place_sound: String,
    pub hit_sound: String,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BlockSoundProfileCatalog {
    profiles: BTreeMap<String, BlockSoundProfile>,
}

#[derive(Debug, Clone)]
struct SoundTypeProfile {
    break_sound: String,
    place_sound: String,
    hit_sound: String,
    volume: f32,
    pitch: f32,
}

impl BlockSoundProfileCatalog {
    pub fn load(_roots: &PackRoots) -> Result<Self> {
        Self::from_generated_source_catalog()
    }

    fn from_generated_source_catalog() -> Result<Self> {
        let sound_types = load_sound_types()?;
        let source = load_generated_block_declarations()?;
        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();
        for declaration in &source.declarations {
            let source_context = || {
                format!(
                    "{}:{}..{}: resolve Blocks.{} sound profile",
                    declaration.source_path,
                    declaration.start_byte,
                    declaration.end_byte,
                    declaration.field
                )
            };
            if let Some(profile) =
                profile_for_declaration(&declaration.initializer, &fields, &sound_types)
                    .with_context(source_context)?
            {
                if profiles
                    .insert(declaration.resource_id.clone(), profile.clone())
                    .is_some()
                {
                    bail!("duplicate block sound profile {}", declaration.resource_id);
                }
                fields.insert(declaration.field.clone(), profile);
            }
        }

        if profiles.is_empty() {
            bail!("generated Blocks catalog did not contain block sound profiles");
        }
        Ok(Self { profiles })
    }

    pub fn profile(&self, resource_id: &str) -> Option<&BlockSoundProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.profiles.get(&resource_id)
    }

    pub fn profiles(&self) -> &BTreeMap<String, BlockSoundProfile> {
        &self.profiles
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

fn load_sound_types() -> Result<BTreeMap<String, SoundTypeProfile>> {
    let source: SoundTypeSourceCatalog = serde_json::from_str(GENERATED_SOUND_TYPE_DECLARATIONS)
        .context("parse generated SoundType declaration catalog")?;
    if source.vanilla_version != crate::MC_VERSION
        || source.sources
            != [
                "net/minecraft/sounds/SoundEvents.java",
                "net/minecraft/world/level/block/SoundType.java",
            ]
    {
        bail!("unsupported generated SoundType source catalog manifest");
    }
    let mut sound_types = BTreeMap::new();
    for (order, declaration) in source.declarations.iter().enumerate() {
        if declaration.declaration_order != order
            || declaration.source_path != "net/minecraft/world/level/block/SoundType.java"
            || declaration.start_byte >= declaration.end_byte
        {
            bail!(
                "invalid generated SoundType declaration order/span for {}",
                declaration.constant
            );
        }
        ResourceLocation::parse(&declaration.break_sound)
            .context("invalid generated SoundType break sound")?;
        ResourceLocation::parse(&declaration.place_sound)
            .context("invalid generated SoundType place sound")?;
        ResourceLocation::parse(&declaration.hit_sound)
            .context("invalid generated SoundType hit sound")?;
        let profile = SoundTypeProfile {
            break_sound: declaration.break_sound.clone(),
            place_sound: declaration.place_sound.clone(),
            hit_sound: declaration.hit_sound.clone(),
            volume: parse_java_float(&declaration.volume_expression)?,
            pitch: parse_java_float(&declaration.pitch_expression)?,
        };
        if sound_types
            .insert(declaration.constant.clone(), profile)
            .is_some()
        {
            bail!("duplicate generated SoundType {}", declaration.constant);
        }
    }
    if sound_types.is_empty() {
        bail!("generated SoundType catalog is empty");
    }
    Ok(sound_types)
}

fn profile_for_declaration(
    expression: &GeneratedJavaExpression,
    fields: &BTreeMap<String, BlockSoundProfile>,
    sound_types: &BTreeMap<String, SoundTypeProfile>,
) -> Result<Option<BlockSoundProfile>> {
    let root_name = expression.name.as_deref();
    let mut profile = if let Some(base) = copy_base_from_register_helper(expression)? {
        let Some(profile) = fields.get(base).cloned() else {
            return Ok(None);
        };
        profile
    } else if let Some(base) = copy_base_from_properties(expression)? {
        let Some(profile) = fields.get(base).cloned() else {
            return Ok(None);
        };
        profile
    } else if root_name == Some("registerBed") {
        block_profile_for_sound_type("WOOD", sound_types)?
    } else if root_name == Some("registerStainedGlass") {
        block_profile_for_sound_type("GLASS", sound_types)?
    } else if let Some(sound_type) = sound_type_from_known_helper(expression)? {
        block_profile_for_sound_type(sound_type, sound_types)?
    } else if has_properties_of(expression) || expression.call("wallVariant").is_some() {
        block_profile_for_sound_type("STONE", sound_types)?
    } else {
        return Ok(None);
    };

    if let Some(sound_type) = explicit_sound_type(expression)? {
        profile = block_profile_for_sound_type(sound_type, sound_types)?;
    }
    Ok(Some(profile))
}

fn block_profile_for_sound_type(
    sound_type: &str,
    sound_types: &BTreeMap<String, SoundTypeProfile>,
) -> Result<BlockSoundProfile> {
    let Some(profile) = sound_types.get(sound_type) else {
        bail!("unknown SoundType {sound_type}");
    };
    Ok(BlockSoundProfile {
        break_sound: profile.break_sound.clone(),
        place_sound: profile.place_sound.clone(),
        hit_sound: profile.hit_sound.clone(),
        volume: profile.volume,
        pitch: profile.pitch,
    })
}

fn sound_type_from_known_helper(expression: &GeneratedJavaExpression) -> Result<Option<&str>> {
    for helper in ["logProperties", "leavesProperties"] {
        if let Some(call) = expression.call(helper) {
            let sound_type = call
                .arguments
                .iter()
                .flat_map(GeneratedJavaExpression::field_accesses)
                .find_map(|argument| argument.field_constant("SoundType"));
            if sound_type.is_none() {
                bail!("{helper} lacks SoundType argument");
            }
            return Ok(sound_type);
        }
    }
    if expression.call("netherStemProperties").is_some() {
        return Ok(Some("STEM"));
    }
    if expression.call("candleProperties").is_some() {
        return Ok(Some("CANDLE"));
    }
    if [
        "shulkerBoxProperties",
        "pistonProperties",
        "buttonProperties",
        "flowerPotProperties",
    ]
    .iter()
    .any(|name| expression.call(name).is_some())
    {
        return Ok(Some("STONE"));
    }
    Ok(None)
}

fn explicit_sound_type(expression: &GeneratedJavaExpression) -> Result<Option<&str>> {
    let Some(call) = expression.call("sound") else {
        return Ok(None);
    };
    let sound_type = call
        .arguments
        .first()
        .and_then(|argument| argument.field_constant("SoundType"))
        .context("block sound property is not a SoundType constant")?;
    Ok(Some(sound_type))
}

fn parse_java_float(value: &str) -> Result<f32> {
    Ok(value
        .replace(' ', "")
        .trim_end_matches(['F', 'f'])
        .parse()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_block_sound_profiles_cover_sound_types_copy_and_helpers() {
        let catalog = BlockSoundProfileCatalog::from_generated_source_catalog().unwrap();

        assert_eq!(catalog.len(), 1168);
        assert_eq!(
            catalog.profile("minecraft:stone").unwrap().hit_sound,
            "minecraft:block.stone.hit"
        );
        assert_eq!(
            catalog.profile("minecraft:grass_block").unwrap(),
            &BlockSoundProfile {
                break_sound: "minecraft:block.grass.break".to_string(),
                place_sound: "minecraft:block.grass.place".to_string(),
                hit_sound: "minecraft:block.grass.hit".to_string(),
                volume: 1.0,
                pitch: 1.0,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:dirt").unwrap().hit_sound,
            "minecraft:block.gravel.hit"
        );
        assert_eq!(
            catalog.profile("minecraft:oak_stairs").unwrap(),
            catalog.profile("minecraft:oak_planks").unwrap()
        );
        assert_eq!(
            catalog.profile("minecraft:white_bed").unwrap().hit_sound,
            "minecraft:block.wood.hit"
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_stained_glass")
                .unwrap()
                .hit_sound,
            "minecraft:block.glass.hit"
        );
        assert_eq!(
            catalog.profile("minecraft:candle").unwrap().hit_sound,
            "minecraft:block.candle.hit"
        );
        assert_eq!(
            catalog
                .profile("minecraft:waxed_weathered_copper_chain")
                .unwrap()
                .hit_sound,
            "minecraft:block.chain.hit"
        );
    }

    #[test]
    fn generated_sound_types_are_complete_contiguous_and_unique() {
        let sound_types = load_sound_types().unwrap();
        assert_eq!(sound_types.len(), 123);
        assert!(sound_types.contains_key("STONE"));
        assert!(sound_types.contains_key("WOOD"));
        assert!(sound_types.contains_key("GLASS"));
    }

    #[test]
    fn block_sound_profile_load_does_not_require_local_java_sources() {
        let root = std::env::temp_dir().join(format!(
            "bbb-pack-block-sound-no-java-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(root.join("sources").join(crate::MC_VERSION)).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();

        let catalog = BlockSoundProfileCatalog::load(&roots).unwrap();

        assert_eq!(catalog.len(), 1168);
        std::fs::remove_dir_all(root).unwrap();
    }
}
