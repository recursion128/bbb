use std::{collections::BTreeMap, path::Path};

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{resources::ResourceLocation, PackRoots};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestroyProfile {
    pub destroy_time_tenths: Option<u32>,
    pub requires_correct_tool: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestroyProfileCatalog {
    profiles: BTreeMap<String, BlockDestroyProfile>,
}

impl BlockDestroyProfileCatalog {
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
        Self::load_from_java_sources(blocks_java, block_ids_java)
    }

    pub fn load_from_java_sources(
        blocks_java: impl AsRef<Path>,
        block_ids_java: impl AsRef<Path>,
    ) -> Result<Self> {
        let blocks_java = blocks_java.as_ref();
        let block_ids_java = block_ids_java.as_ref();
        let blocks_source = std::fs::read_to_string(blocks_java)
            .with_context(|| format!("read block registry source {}", blocks_java.display()))?;
        let block_id_constants = if block_ids_java.is_file() {
            let source = std::fs::read_to_string(block_ids_java)
                .with_context(|| format!("read block id source {}", block_ids_java.display()))?;
            parse_block_id_constants(&source)?
        } else {
            BTreeMap::new()
        };
        Self::from_blocks_java_source(&blocks_source, &block_id_constants)
    }

    fn from_blocks_java_source(
        source: &str,
        block_id_constants: &BTreeMap<String, String>,
    ) -> Result<Self> {
        let declaration =
            Regex::new(r#"(?s)public\s+static\s+final\s+Block\s+([A-Z0-9_]+)\s*=\s*(.*?);"#)?;
        let mut fields = BTreeMap::new();
        let mut profiles = BTreeMap::new();
        for capture in declaration.captures_iter(source) {
            let field = capture.get(1).unwrap().as_str();
            let expression = capture.get(2).unwrap().as_str();
            let resource_id = resource_id_for_declaration(field, expression, block_id_constants)?;
            if let Some(profile) = profile_for_declaration(expression, &fields)? {
                profiles.insert(resource_id.clone(), profile.clone());
                fields.insert(field.to_string(), profile);
            }
        }

        if profiles.is_empty() {
            bail!("Blocks.java did not contain block destroy profiles");
        }

        Ok(Self { profiles })
    }

    pub fn profile(&self, resource_id: &str) -> Option<&BlockDestroyProfile> {
        let resource_id = ResourceLocation::parse(resource_id).ok()?.id();
        self.profiles.get(&resource_id)
    }

    pub fn profiles(&self) -> &BTreeMap<String, BlockDestroyProfile> {
        &self.profiles
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
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

fn profile_for_declaration(
    expression: &str,
    fields: &BTreeMap<String, BlockDestroyProfile>,
) -> Result<Option<BlockDestroyProfile>> {
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
        profile(2, false)
    } else if expression.trim_start().starts_with("registerStainedGlass(") {
        profile(3, false)
    } else if let Some(helper_profile) = profile_from_known_helper(expression) {
        helper_profile
    } else if expression.contains("BlockBehaviour.Properties.of()")
        || expression.contains("wallVariant(")
    {
        default_profile()
    } else {
        return Ok(None);
    };

    apply_property_chain(expression, &mut profile)?;
    if let Some(host) = infested_host_from_constructor(expression)? {
        let Some(host_profile) = fields.get(&host) else {
            return Ok(None);
        };
        profile.destroy_time_tenths =
            infested_destroy_time_tenths(host_profile.destroy_time_tenths);
    }
    Ok(Some(profile))
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

fn profile_from_known_helper(expression: &str) -> Option<BlockDestroyProfile> {
    if expression.contains("logProperties(") || expression.contains("netherStemProperties(") {
        return Some(profile(20, false));
    }
    if expression.contains("leavesProperties(") {
        return Some(profile(2, false));
    }
    if expression.contains("buttonProperties(") {
        return Some(profile(5, false));
    }
    if expression.contains("flowerPotProperties(") {
        return Some(profile(0, false));
    }
    if expression.contains("candleProperties(") {
        return Some(profile(1, false));
    }
    if expression.contains("shulkerBoxProperties(") {
        return Some(profile(20, false));
    }
    if expression.contains("pistonProperties(") {
        return Some(profile(15, false));
    }
    None
}

fn infested_host_from_constructor(expression: &str) -> Result<Option<String>> {
    optional_capture(
        r#"new\s+Infested(?:RotatedPillar)?Block\(\s*([A-Z0-9_]+)\s*,\s*p\s*\)"#,
        expression,
    )
}

fn infested_destroy_time_tenths(host_destroy_time_tenths: Option<u32>) -> Option<u32> {
    host_destroy_time_tenths.map(|value| (f64::from(value) / 2.0).round() as u32)
}

fn apply_property_chain(expression: &str, profile: &mut BlockDestroyProfile) -> Result<()> {
    let operation = Regex::new(
        r#"\.(strength|destroyTime)\(\s*(-?[0-9]+(?:\.[0-9]+)?)[Ff]?(?:\s*,[^)]*)?\)|\.(instabreak|requiresCorrectToolForDrops)\(\s*\)"#,
    )?;
    for capture in operation.captures_iter(expression) {
        if let Some(method) = capture.get(1) {
            let value = capture.get(2).unwrap().as_str();
            match method.as_str() {
                "strength" | "destroyTime" => {
                    profile.destroy_time_tenths = destroy_time_tenths(value)?;
                }
                _ => {}
            }
            continue;
        }

        match capture.get(3).unwrap().as_str() {
            "instabreak" => profile.destroy_time_tenths = Some(0),
            "requiresCorrectToolForDrops" => profile.requires_correct_tool = true,
            _ => {}
        }
    }
    Ok(())
}

fn destroy_time_tenths(value: &str) -> Result<Option<u32>> {
    let value: f64 = value.parse()?;
    if value < 0.0 {
        return Ok(None);
    }
    Ok(Some((value * 10.0).round() as u32))
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

fn default_profile() -> BlockDestroyProfile {
    profile(0, false)
}

fn profile(destroy_time_tenths: u32, requires_correct_tool: bool) -> BlockDestroyProfile {
    BlockDestroyProfile {
        destroy_time_tenths: Some(destroy_time_tenths),
        requires_correct_tool,
    }
}

fn minecraft_id(path: &str) -> Result<String> {
    ResourceLocation::new("minecraft", path).map(|location| location.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_destroy_profiles_parse_direct_copy_and_helper_declarations() {
        let source = r#"
            public class Blocks {
               public static final Block AIR = register("air", AirBlock::new, BlockBehaviour.Properties.of().replaceable().air());
               public static final Block STONE = register(
                  "stone",
                  BlockBehaviour.Properties.of().mapColor(MapColor.STONE).requiresCorrectToolForDrops().strength(1.5F, 6.0F)
               );
               public static final Block INFESTED_STONE = register(
                  "infested_stone", p -> new InfestedBlock(STONE, p), BlockBehaviour.Properties.of().mapColor(MapColor.CLAY)
               );
               public static final Block DIRT = register(BlockIds.DIRT, BlockBehaviour.Properties.of().strength(0.5F));
               public static final Block BEDROCK = register("bedrock", BlockBehaviour.Properties.of().strength(-1.0F, 3600000.0F).noLootTable());
               public static final Block DEEPSLATE_DIAMOND_ORE = register(
                  "deepslate_diamond_ore",
                  p -> new DropExperienceBlock(UniformInt.of(3, 7), p),
                  BlockBehaviour.Properties.ofLegacyCopy(STONE).strength(4.5F, 3.0F)
               );
               public static final Block OAK_LOG = register("oak_log", RotatedPillarBlock::new, logProperties(MapColor.WOOD, MapColor.PODZOL, SoundType.WOOD));
               public static final Block OAK_STAIRS = registerLegacyStair("oak_stairs", OAK_LOG);
               public static final Block WHITE_BED = registerBed("white_bed", DyeColor.WHITE);
               public static final Block WHITE_STAINED_GLASS = registerStainedGlass("white_stained_glass", DyeColor.WHITE);
               public static final Block AMETHYST_BLOCK = register("amethyst_block", BlockBehaviour.Properties.of().strength(1.5F).sound(SoundType.AMETHYST).requiresCorrectToolForDrops());
            }
        "#;
        let constants = BTreeMap::from([("DIRT".to_string(), "dirt".to_string())]);

        let catalog =
            BlockDestroyProfileCatalog::from_blocks_java_source(source, &constants).unwrap();

        assert_eq!(
            catalog
                .profile("minecraft:air")
                .unwrap()
                .destroy_time_tenths,
            Some(0)
        );
        assert_eq!(
            catalog.profile("minecraft:stone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(15),
                requires_correct_tool: true,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:infested_stone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(8),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:dirt")
                .unwrap()
                .destroy_time_tenths,
            Some(5)
        );
        assert_eq!(
            catalog
                .profile("minecraft:bedrock")
                .unwrap()
                .destroy_time_tenths,
            None
        );
        assert_eq!(
            catalog.profile("minecraft:deepslate_diamond_ore").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(45),
                requires_correct_tool: true,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:oak_stairs").unwrap(),
            catalog.profile("minecraft:oak_log").unwrap()
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_bed")
                .unwrap()
                .destroy_time_tenths,
            Some(2)
        );
        assert_eq!(
            catalog
                .profile("minecraft:white_stained_glass")
                .unwrap()
                .destroy_time_tenths,
            Some(3)
        );
        assert!(
            catalog
                .profile("minecraft:amethyst_block")
                .unwrap()
                .requires_correct_tool
        );
    }

    #[test]
    fn block_destroy_profiles_load_local_vanilla_sources_when_available() {
        let Ok(roots) = PackRoots::discover() else {
            return;
        };

        let catalog = roots.load_block_destroy_profile_catalog().unwrap();

        assert!(
            catalog.len() >= 600,
            "expected broad vanilla coverage, got {} profiles",
            catalog.len()
        );
        assert_eq!(
            catalog.profile("minecraft:diamond_ore").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(30),
                requires_correct_tool: true,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:deepslate_diamond_ore")
                .unwrap()
                .destroy_time_tenths,
            Some(45)
        );
        assert_eq!(
            catalog.profile("minecraft:infested_stone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(8),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:infested_cobblestone").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(10),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog.profile("minecraft:infested_deepslate").unwrap(),
            &BlockDestroyProfile {
                destroy_time_tenths: Some(15),
                requires_correct_tool: false,
            }
        );
        assert_eq!(
            catalog
                .profile("minecraft:bedrock")
                .unwrap()
                .destroy_time_tenths,
            None
        );
        assert_eq!(
            catalog
                .profile("minecraft:flower_pot")
                .unwrap()
                .destroy_time_tenths,
            Some(0)
        );
    }
}
