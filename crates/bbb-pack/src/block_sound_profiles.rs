use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, PackRoots};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSoundProfile {
    pub break_sound: String,
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
    hit_sound: String,
    volume: f32,
    pitch: f32,
}

impl BlockSoundProfileCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let blocks_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("world")
            .join("level")
            .join("block")
            .join("Blocks.java");
        let block_ids_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("references")
            .join("BlockIds.java");
        let sound_type_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("world")
            .join("level")
            .join("block")
            .join("SoundType.java");
        let sound_events_java = roots
            .sources_dir
            .join("net")
            .join("minecraft")
            .join("sounds")
            .join("SoundEvents.java");
        Self::load_from_java_sources(
            blocks_java,
            block_ids_java,
            sound_type_java,
            sound_events_java,
        )
    }

    pub fn load_from_java_sources(
        blocks_java: impl AsRef<Path>,
        block_ids_java: impl AsRef<Path>,
        sound_type_java: impl AsRef<Path>,
        sound_events_java: impl AsRef<Path>,
    ) -> Result<Self> {
        let blocks_java = blocks_java.as_ref();
        let block_ids_java = block_ids_java.as_ref();
        let sound_type_java = sound_type_java.as_ref();
        let sound_events_java = sound_events_java.as_ref();
        let blocks_source = std::fs::read_to_string(blocks_java)
            .with_context(|| format!("read block registry source {}", blocks_java.display()))?;
        let sound_type_source = std::fs::read_to_string(sound_type_java)
            .with_context(|| format!("read sound type source {}", sound_type_java.display()))?;
        let sound_events_source = std::fs::read_to_string(sound_events_java)
            .with_context(|| format!("read sound events source {}", sound_events_java.display()))?;
        let block_id_constants = if block_ids_java.is_file() {
            let source = std::fs::read_to_string(block_ids_java)
                .with_context(|| format!("read block id source {}", block_ids_java.display()))?;
            parse_block_id_constants(&source)?
        } else {
            BTreeMap::new()
        };
        Self::from_java_sources(
            &blocks_source,
            &block_id_constants,
            &sound_type_source,
            &sound_events_source,
        )
    }

    fn from_java_sources(
        blocks_source: &str,
        block_id_constants: &BTreeMap<String, String>,
        sound_type_source: &str,
        sound_events_source: &str,
    ) -> Result<Self> {
        let sound_events = parse_sound_event_constants(sound_events_source)?;
        let sound_types = parse_sound_types(sound_type_source, &sound_events)?;
        let declaration =
            Regex::new(r#"(?s)public\s+static\s+final\s+Block\s+([A-Z0-9_]+)\s*=\s*(.*?);"#)?;
        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();
        for capture in declaration.captures_iter(blocks_source) {
            let field = capture.get(1).unwrap().as_str();
            let expression = capture.get(2).unwrap().as_str();
            let resource_id = resource_id_for_declaration(field, expression, block_id_constants)?;
            if let Some(profile) = profile_for_declaration(expression, &fields, &sound_types)? {
                profiles.insert(resource_id.clone(), profile.clone());
                fields.insert(field.to_string(), profile);
            }
        }

        if profiles.is_empty() {
            bail!("Blocks.java did not contain block sound profiles");
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

fn parse_sound_event_constants(source: &str) -> Result<BTreeMap<String, String>> {
    let declaration = Regex::new(
        r#"public\s+static\s+final\s+(?:SoundEvent|Holder\.Reference<SoundEvent>)\s+([A-Z0-9_]+)\s*=\s*register(?:ForHolder)?\("([^"]+)""#,
    )?;
    let mut events = BTreeMap::new();
    for capture in declaration.captures_iter(source) {
        events.insert(
            capture.get(1).unwrap().as_str().to_string(),
            minecraft_id(capture.get(2).unwrap().as_str())?,
        );
    }
    Ok(events)
}

fn parse_sound_types(
    source: &str,
    sound_events: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, SoundTypeProfile>> {
    let declaration = Regex::new(
        r#"(?s)public\s+static\s+final\s+SoundType\s+([A-Z0-9_]+)\s*=\s*new\s+SoundType\((.*?)\);"#,
    )?;
    let mut sound_types = BTreeMap::new();
    for capture in declaration.captures_iter(source) {
        let name = capture.get(1).unwrap().as_str();
        let args = split_top_level_args(capture.get(2).unwrap().as_str());
        if args.len() != 7 {
            bail!(
                "unsupported SoundType {name}: expected 7 args, got {}",
                args.len()
            );
        }
        let break_constant = sound_event_constant(&args[2])?;
        let hit_constant = sound_event_constant(&args[5])?;
        let Some(break_sound) = sound_events.get(&break_constant).cloned() else {
            bail!("unknown break SoundEvents constant {break_constant} for SoundType {name}");
        };
        let Some(hit_sound) = sound_events.get(&hit_constant).cloned() else {
            bail!("unknown hit SoundEvents constant {hit_constant} for SoundType {name}");
        };
        sound_types.insert(
            name.to_string(),
            SoundTypeProfile {
                break_sound,
                hit_sound,
                volume: parse_java_float(&args[0])?,
                pitch: parse_java_float(&args[1])?,
            },
        );
    }
    Ok(sound_types)
}

fn profile_for_declaration(
    expression: &str,
    fields: &BTreeMap<String, BlockSoundProfile>,
    sound_types: &BTreeMap<String, SoundTypeProfile>,
) -> Result<Option<BlockSoundProfile>> {
    let mut profile = if let Some(base) = copy_base_from_register_helper(expression)? {
        let Some(profile) = fields.get(&base).cloned() else {
            return Ok(None);
        };
        profile
    } else if let Some(base) = copy_base_from_properties(expression)? {
        let Some(profile) = fields.get(&base).cloned() else {
            return Ok(None);
        };
        profile
    } else if expression.trim_start().starts_with("registerBed(") {
        block_profile_for_sound_type("WOOD", sound_types)?
    } else if expression.trim_start().starts_with("registerStainedGlass(") {
        block_profile_for_sound_type("GLASS", sound_types)?
    } else if let Some(sound_type) = sound_type_from_known_helper(expression)? {
        block_profile_for_sound_type(&sound_type, sound_types)?
    } else if expression.contains("BlockBehaviour.Properties.of()")
        || expression.contains("wallVariant(")
    {
        block_profile_for_sound_type("STONE", sound_types)?
    } else {
        return Ok(None);
    };

    if let Some(sound_type) = explicit_sound_type(expression)? {
        profile = block_profile_for_sound_type(&sound_type, sound_types)?;
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
        hit_sound: profile.hit_sound.clone(),
        volume: profile.volume,
        pitch: profile.pitch,
    })
}

fn sound_type_from_known_helper(expression: &str) -> Result<Option<String>> {
    for helper in ["logProperties", "leavesProperties"] {
        if let Some(sound_type) = optional_capture(
            &format!(r#"{helper}\([^)]*SoundType\.([A-Z0-9_]+)"#),
            expression,
        )? {
            return Ok(Some(sound_type));
        }
    }
    if expression.contains("netherStemProperties(") {
        return Ok(Some("STEM".to_string()));
    }
    if expression.contains("candleProperties(") {
        return Ok(Some("CANDLE".to_string()));
    }
    if expression.contains("shulkerBoxProperties(")
        || expression.contains("pistonProperties(")
        || expression.contains("buttonProperties(")
        || expression.contains("flowerPotProperties(")
    {
        return Ok(Some("STONE".to_string()));
    }
    Ok(None)
}

fn explicit_sound_type(expression: &str) -> Result<Option<String>> {
    optional_capture(r#"\.sound\(\s*SoundType\.([A-Z0-9_]+)\s*\)"#, expression)
}

fn sound_event_constant(expression: &str) -> Result<String> {
    optional_capture(r#"SoundEvents\.([A-Z0-9_]+)"#, expression)?
        .ok_or_else(|| anyhow::anyhow!("unsupported sound event expression {expression:?}"))
}

fn split_top_level_args(args: &str) -> Vec<String> {
    let mut split = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    for (index, ch) in args.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                split.push(args[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    split.push(args[start..].trim().to_string());
    split
}

fn parse_java_float(value: &str) -> Result<f32> {
    Ok(value.trim().trim_end_matches(['F', 'f']).parse()?)
}

fn resource_id_for_declaration(
    field: &str,
    expression: &str,
    block_id_constants: &BTreeMap<String, String>,
) -> Result<String> {
    for helper in [
        "register",
        "registerBed",
        "registerLegacyStair",
        "registerStair",
        "registerStainedGlass",
    ] {
        if let Some(name) = optional_capture(&format!(r#"{helper}\(\s*"([^"]+)""#), expression)? {
            return minecraft_id(&name);
        }
    }

    if let Some(name) = optional_capture(r#"vanillaBlockId\(\s*"([^"]+)""#, expression)? {
        return minecraft_id(&name);
    }

    if let Some(constant) = optional_capture(r#"register\(\s*BlockIds\.([A-Z0-9_]+)"#, expression)?
    {
        let name = block_id_constants
            .get(&constant)
            .cloned()
            .unwrap_or_else(|| constant.to_ascii_lowercase());
        return minecraft_id(&name);
    }

    bail!("unsupported block registry declaration {field}: {expression:?}")
}

fn copy_base_from_register_helper(expression: &str) -> Result<Option<String>> {
    for helper in ["registerLegacyStair", "registerStair"] {
        if let Some(base) = optional_capture(
            &format!(r#"{helper}\(\s*"[^"]+"\s*,\s*([A-Z0-9_]+)"#),
            expression,
        )? {
            return Ok(Some(base));
        }
    }
    Ok(None)
}

fn copy_base_from_properties(expression: &str) -> Result<Option<String>> {
    optional_capture(
        r#"BlockBehaviour\.Properties\.of(?:Legacy|Full)Copy\(\s*([A-Z0-9_]+)"#,
        expression,
    )
}

fn parse_block_id_constants(source: &str) -> Result<BTreeMap<String, String>> {
    let declaration = Regex::new(
        r#"public\s+static\s+final\s+ResourceKey<Block>\s+([A-Z0-9_]+)\s*=\s*createKey\("([^"]+)"\)"#,
    )?;
    let mut constants = BTreeMap::new();
    for capture in declaration.captures_iter(source) {
        constants.insert(
            capture.get(1).unwrap().as_str().to_string(),
            capture.get(2).unwrap().as_str().to_string(),
        );
    }
    Ok(constants)
}

fn optional_capture(pattern: &str, expression: &str) -> Result<Option<String>> {
    let regex = Regex::new(pattern)?;
    Ok(regex
        .captures(expression)
        .and_then(|capture| capture.get(1))
        .map(|capture| capture.as_str().to_string()))
}

fn minecraft_id(path: &str) -> Result<String> {
    ResourceLocation::new("minecraft", path).map(|location| location.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_sound_profiles_parse_sound_types_copy_and_helpers() {
        let sound_events = r#"
            public class SoundEvents {
                public static final SoundEvent STONE_HIT = register("block.stone.hit");
                public static final SoundEvent GRASS_HIT = register("block.grass.hit");
                public static final SoundEvent WOOD_HIT = register("block.wood.hit");
                public static final SoundEvent GLASS_HIT = register("block.glass.hit");
                public static final SoundEvent CANDLE_HIT = register("block.candle.hit");
                public static final SoundEvent STONE_BREAK = register("block.stone.break");
                public static final SoundEvent GRASS_BREAK = register("block.grass.break");
                public static final SoundEvent STONE_STEP = register("block.stone.step");
                public static final SoundEvent STONE_PLACE = register("block.stone.place");
                public static final SoundEvent STONE_FALL = register("block.stone.fall");
            }
        "#;
        let sound_types = r#"
            public class SoundType {
                public static final SoundType STONE = new SoundType(
                    1.0F, 1.0F, SoundEvents.STONE_BREAK, SoundEvents.STONE_STEP,
                    SoundEvents.STONE_PLACE, SoundEvents.STONE_HIT, SoundEvents.STONE_FALL
                );
                public static final SoundType GRASS = new SoundType(
                    0.8F, 1.2F, SoundEvents.GRASS_BREAK, SoundEvents.STONE_STEP,
                    SoundEvents.STONE_PLACE, SoundEvents.GRASS_HIT, SoundEvents.STONE_FALL
                );
                public static final SoundType WOOD = new SoundType(
                    1.0F, 1.0F, SoundEvents.STONE_BREAK, SoundEvents.STONE_STEP,
                    SoundEvents.STONE_PLACE, SoundEvents.WOOD_HIT, SoundEvents.STONE_FALL
                );
                public static final SoundType GLASS = new SoundType(
                    1.0F, 1.0F, SoundEvents.STONE_BREAK, SoundEvents.STONE_STEP,
                    SoundEvents.STONE_PLACE, SoundEvents.GLASS_HIT, SoundEvents.STONE_FALL
                );
                public static final SoundType CANDLE = new SoundType(
                    0.5F, 0.7F, SoundEvents.STONE_BREAK, SoundEvents.STONE_STEP,
                    SoundEvents.STONE_PLACE, SoundEvents.CANDLE_HIT, SoundEvents.STONE_FALL
                );
            }
        "#;
        let blocks = r#"
            public class Blocks {
               public static final Block STONE = register("stone", BlockBehaviour.Properties.of().strength(1.5F));
               public static final Block GRASS_BLOCK = register("grass_block", BlockBehaviour.Properties.of().strength(0.6F).sound(SoundType.GRASS));
               public static final Block DEEPSLATE_GRASS = register("deepslate_grass", BlockBehaviour.Properties.ofLegacyCopy(GRASS_BLOCK).strength(4.5F));
               public static final Block OAK_LOG = register("oak_log", RotatedPillarBlock::new, logProperties(MapColor.WOOD, MapColor.PODZOL, SoundType.WOOD));
               public static final Block OAK_STAIRS = registerLegacyStair("oak_stairs", OAK_LOG);
               public static final Block WHITE_BED = registerBed("white_bed", DyeColor.WHITE);
               public static final Block WHITE_STAINED_GLASS = registerStainedGlass("white_stained_glass", DyeColor.WHITE);
               public static final Block CANDLE = register("candle", CandleBlock::new, candleProperties(MapColor.SAND));
            }
        "#;

        let catalog = BlockSoundProfileCatalog::from_java_sources(
            blocks,
            &BTreeMap::new(),
            sound_types,
            sound_events,
        )
        .unwrap();

        assert_eq!(
            catalog.profile("minecraft:stone").unwrap().hit_sound,
            "minecraft:block.stone.hit"
        );
        assert_eq!(
            catalog.profile("minecraft:grass_block").unwrap(),
            &BlockSoundProfile {
                break_sound: "minecraft:block.grass.break".to_string(),
                hit_sound: "minecraft:block.grass.hit".to_string(),
                volume: 0.8,
                pitch: 1.2,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:deepslate_grass").unwrap(),
            catalog.profile("minecraft:grass_block").unwrap()
        );
        assert_eq!(
            catalog.profile("minecraft:oak_stairs").unwrap(),
            catalog.profile("minecraft:oak_log").unwrap()
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
    }

    #[test]
    fn block_sound_profiles_load_local_vanilla_sources_when_available() {
        let Ok(roots) = PackRoots::discover() else {
            return;
        };

        let catalog = roots.load_block_sound_profile_catalog().unwrap();

        assert!(
            catalog.len() >= 600,
            "expected broad vanilla coverage, got {} profiles",
            catalog.len()
        );
        assert_eq!(
            catalog.profile("minecraft:grass_block").unwrap().hit_sound,
            "minecraft:block.grass.hit"
        );
        assert_eq!(
            catalog
                .profile("minecraft:grass_block")
                .unwrap()
                .break_sound,
            "minecraft:block.grass.break"
        );
        assert_eq!(
            catalog.profile("minecraft:dirt").unwrap().hit_sound,
            "minecraft:block.gravel.hit"
        );
        assert_eq!(
            catalog.profile("minecraft:oak_stairs").unwrap(),
            catalog.profile("minecraft:oak_log").unwrap()
        );
    }
}
