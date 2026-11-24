//! Runtime facts from datapack `DimensionType.DIRECT_CODEC` elements.
//!
//! The exact source decoder validates the complete record
//! (`DimensionType.java:83-107`) before this owner projects gameplay and shape.

use std::collections::BTreeMap;
use std::fmt;

use bbb_protocol::{content_registry::ContentRegistry, NbtSummaryEntry};
use bbb_registry_codec::registry_codec::decode_direct_dimension_type_facts_with_content_registry;
use bbb_registry_codec::{decode_direct_registry_json, IntProvider, RegistryElementData};
pub use bbb_server_types::dimension_type::DimensionTypeShape;
use bbb_server_types::save::{
    parse_level_stem, Java26DimensionGeometries, Java26LevelKey, Java26LevelStem,
    Java26WorldGenSettings,
};

use super::dynamic_registry::DatapackGeneration;
use super::vanilla_snapshot::VanillaDatapackSnapshot;

/// The dimension type registry a `LevelStem` points into
/// (`Registries.DIMENSION_TYPE`).
pub const DIMENSION_TYPE_REGISTRY: &str = "minecraft:dimension_type";

/// A dimension type element this server refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionTypeError {
    pub element_id: String,
    pub message: String,
}

impl fmt::Display for DimensionTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "dimension type {}: {}", self.element_id, self.message)
    }
}

impl std::error::Error for DimensionTypeError {}

fn refuse(element_id: &str, message: impl Into<String>) -> DimensionTypeError {
    DimensionTypeError {
        element_id: element_id.to_string(),
        message: message.into(),
    }
}

/// Runtime facts retained for one decoded dimension-type registry element.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionTypeRuntimeFacts {
    pub element_id: String,
    pub shape: DimensionTypeShape,
    /// `default_clock` (`DimensionType.java:102`), the clock
    /// `TimeCommand.getDefaultClock` (`TimeCommand.java:207-210`) resolves the
    /// bare `/time` arms against.
    pub default_clock: Option<String>,
    /// `ambient_light` (`DimensionType.java:40`), the floor
    /// `LevelReader.getLightLevelDependentMagicValue` lerps towards
    /// (`LevelReader.java:113-117`).
    pub ambient_light: f32,
    pub infiniburn_tag: String,
    pub monster_spawn_light_level: IntProvider,
    pub monster_spawn_block_light_limit: i32,
}

fn decode_runtime_facts(
    element_id: &str,
    source: &RegistryElementData,
) -> Result<DimensionTypeRuntimeFacts, DimensionTypeError> {
    decode_runtime_facts_with_content_registry(element_id, source, ContentRegistry::vanilla())
}

fn decode_runtime_facts_with_content_registry(
    element_id: &str,
    source: &RegistryElementData,
    content_registry: &ContentRegistry,
) -> Result<DimensionTypeRuntimeFacts, DimensionTypeError> {
    let decoded =
        decode_direct_dimension_type_facts_with_content_registry(source, content_registry)
            .map_err(|error| refuse(element_id, error.reason))?;
    Ok(DimensionTypeRuntimeFacts {
        element_id: element_id.to_string(),
        default_clock: decoded.default_clock,
        ambient_light: decoded.scalars.ambient_light,
        shape: DimensionTypeShape {
            has_fixed_time: decoded.scalars.has_fixed_time,
            has_skylight: decoded.scalars.has_skylight,
            has_ceiling: decoded.scalars.has_ceiling,
            has_ender_dragon_fight: decoded.scalars.has_ender_dragon_fight,
            coordinate_scale: decoded.scalars.coordinate_scale,
            min_y: decoded.scalars.min_y,
            height: decoded.scalars.height,
            logical_height: decoded.scalars.logical_height,
        },
        infiniburn_tag: decoded.infiniburn_tag,
        monster_spawn_light_level: decoded.monster_spawn_light_level,
        monster_spawn_block_light_limit: decoded.scalars.monster_spawn_block_light_limit,
    })
}

/// Runtime facts for `element_id` from the generation's exact winning source.
pub fn dimension_type_runtime_facts(
    generation: &DatapackGeneration,
    element_id: &str,
) -> Result<DimensionTypeRuntimeFacts, DimensionTypeError> {
    dimension_type_runtime_facts_with_content_registry(
        generation,
        element_id,
        ContentRegistry::vanilla(),
    )
}

/// Content-registry-aware form of [`dimension_type_runtime_facts`].
pub fn dimension_type_runtime_facts_with_content_registry(
    generation: &DatapackGeneration,
    element_id: &str,
    content_registry: &ContentRegistry,
) -> Result<DimensionTypeRuntimeFacts, DimensionTypeError> {
    let registry = generation
        .registry(DIMENSION_TYPE_REGISTRY)
        .ok_or_else(|| {
            refuse(
                element_id,
                "the generation loaded no dimension type registry",
            )
        })?;
    let element = registry
        .elements
        .iter()
        .find(|element| element.id == element_id)
        .ok_or_else(|| {
            refuse(
                element_id,
                "the dimension type registry has no such element",
            )
        })?;
    let source = element
        .decoded_direct_source()
        .expect("loaded direct registry source");
    decode_runtime_facts_with_content_registry(element_id, &source, content_registry)
}

/// The shape of the dimension type `element_id` in `generation`.
///
/// The generation is the one the datapack lifecycle already built, so a stem
/// pointing at a type a datapack replaced reads the replacement, exactly as
/// `RegistryDataLoader` would.
pub fn dimension_type_shape(
    generation: &DatapackGeneration,
    element_id: &str,
) -> Result<DimensionTypeShape, DimensionTypeError> {
    dimension_type_runtime_facts(generation, element_id).map(|facts| facts.shape)
}

/// Content-registry-aware form of [`dimension_type_shape`].
pub fn dimension_type_shape_with_content_registry(
    generation: &DatapackGeneration,
    element_id: &str,
    content_registry: &ContentRegistry,
) -> Result<DimensionTypeShape, DimensionTypeError> {
    dimension_type_runtime_facts_with_content_registry(generation, element_id, content_registry)
        .map(|facts| facts.shape)
}

/// Runtime facts for a dimension type from the embedded Vanilla pack.
pub fn builtin_dimension_type_runtime_facts(
    element_id: &str,
) -> Result<DimensionTypeRuntimeFacts, DimensionTypeError> {
    let path = builtin_dimension_type_path(element_id)?;
    let snapshot = VanillaDatapackSnapshot::embedded_shared()
        .map_err(|error| refuse(element_id, format!("built-in datapack snapshot: {error}")))?;
    let bytes = snapshot
        .read_path(&format!("dimension_type/{path}.json"))
        .ok_or_else(|| refuse(element_id, "the built-in pack has no such dimension type"))?;
    let source = decode_direct_registry_json(bytes)
        .map_err(|error| refuse(element_id, error.to_string()))?;
    decode_runtime_facts(element_id, &source)
}

/// The shape of the dimension type `element_id` as the built-in
/// `data/minecraft/` pack declares it.
///
/// This is the same decode [`dimension_type_shape`] runs, over the other place
/// the element can come from: the embedded snapshot of the vanilla pack. A
/// caller that holds a loaded [`DatapackGeneration`] reads it from there,
/// because a datapack may have replaced the element; a caller that has none --
/// the created save's own documents and the save loader -- reads the built-in
/// element.
///
/// Refused for anything outside the `minecraft` namespace: the snapshot holds
/// `data/minecraft/` and nothing else, so a stem naming another namespace has
/// no element here to be sized from.
pub fn builtin_dimension_type_shape(
    element_id: &str,
) -> Result<DimensionTypeShape, DimensionTypeError> {
    builtin_dimension_type_runtime_facts(element_id).map(|facts| facts.shape)
}

/// `DimensionType.defaultClock` of the built-in element, which is what a
/// projection with no loaded generation behind it resolves `/time` against.
pub fn builtin_dimension_type_default_clock(
    element_id: &str,
) -> Result<Option<String>, DimensionTypeError> {
    builtin_dimension_type_runtime_facts(element_id).map(|facts| facts.default_clock)
}

fn builtin_dimension_type_path(element_id: &str) -> Result<&str, DimensionTypeError> {
    let path = element_id.strip_prefix("minecraft:").ok_or_else(|| {
        refuse(
            element_id,
            "only the minecraft namespace is in the built-in pack",
        )
    })?;
    if path.contains('/') || path.is_empty() {
        return Err(refuse(element_id, "is not a built-in dimension type path"));
    }
    Ok(path)
}

/// Applies the live LEVEL_STEM registry over the saved stem map (`WorldDimensions.java:153-168`).
pub fn bake_java_26_1_level_stems(
    settings: &Java26WorldGenSettings,
    generation: &DatapackGeneration,
) -> BTreeMap<Java26LevelKey, Java26LevelStem> {
    let Some(registry) = generation.registry("minecraft:dimension") else {
        return settings.dimensions.clone();
    };
    let mut dimensions = settings.dimensions.clone();
    for element in &registry.elements {
        let Some(key) = Java26LevelKey::parse(element.id.as_str()).ok() else {
            continue;
        };
        let Some(value) = decode_direct_registry_json(element.source_json.as_ref())
            .ok()
            .and_then(|source| source.source_order_nbt_projection().ok())
        else {
            continue;
        };
        let stem_entry = NbtSummaryEntry {
            name: key.as_str().into(),
            value,
        };
        if let Some(stem) = parse_level_stem(&stem_entry) {
            dimensions.insert(key, stem);
        }
    }
    dimensions
}

/// Bakes live `LEVEL_STEM` winners and resolves each dimension's section geometry.
pub fn dimension_geometries_from_generation(
    settings: &Java26WorldGenSettings,
    generation: &DatapackGeneration,
) -> Java26DimensionGeometries {
    dimension_geometries_from_generation_with_content_registry(
        settings,
        generation,
        ContentRegistry::vanilla(),
    )
}

/// Content-registry-aware form of [`dimension_geometries_from_generation`].
pub fn dimension_geometries_from_generation_with_content_registry(
    settings: &Java26WorldGenSettings,
    generation: &DatapackGeneration,
    content_registry: &ContentRegistry,
) -> Java26DimensionGeometries {
    let stems = bake_java_26_1_level_stems(settings, generation);
    Java26DimensionGeometries::from_stems(&stems, |stem| {
        dimension_type_runtime_facts_with_content_registry(
            generation,
            &stem.dimension_type,
            content_registry,
        )
        .ok()
        .map(|facts| facts.shape)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One built-in dimension type, read straight out of the embedded snapshot
    /// of `data/minecraft/`.
    fn builtin(path: &str) -> RegistryElementData {
        let snapshot = VanillaDatapackSnapshot::embedded().unwrap();
        let bytes = snapshot
            .read_path(&format!("dimension_type/{path}.json"))
            .unwrap_or_else(|| panic!("the snapshot carries dimension_type/{path}.json"));
        decode_direct_registry_json(bytes).unwrap()
    }

    fn shape(path: &str) -> DimensionTypeShape {
        decode_runtime_facts(&format!("minecraft:{path}"), &builtin(path))
            .unwrap()
            .shape
    }

    #[test]
    fn the_four_built_in_dimension_types_decode_to_the_shapes_they_declare() {
        let overworld = shape("overworld");
        assert_eq!((overworld.min_y, overworld.height), (-64, 384));
        assert_eq!(overworld.logical_height, 384);
        assert_eq!(overworld.coordinate_scale, 1.0);
        assert!(overworld.has_skylight);
        assert!(!overworld.has_ceiling);
        assert!(!overworld.has_fixed_time);
        assert_eq!(overworld.section_count(), 24);
        assert_eq!(overworld.max_y_exclusive(), 320);

        // `overworld_caves` differs from `overworld` only in its ceiling, which
        // is what makes it the pair that proves the flags are read per element.
        let caves = shape("overworld_caves");
        assert!(caves.has_ceiling);
        assert_eq!((caves.min_y, caves.height), (-64, 384));

        // The nether is the one built-in whose logical height is below its
        // height (`DimensionType.java:70-72` permits it, portals read it).
        let nether = shape("the_nether");
        assert_eq!((nether.min_y, nether.height), (0, 256));
        assert_eq!(nether.logical_height, 128);
        assert_eq!(nether.coordinate_scale, 8.0);
        assert!(!nether.has_skylight);
        assert!(nether.has_ceiling);
        assert!(nether.has_fixed_time);

        let end = shape("the_end");
        assert_eq!((end.min_y, end.height), (0, 256));
        assert_eq!(end.logical_height, 256);
        assert!(end.has_ender_dragon_fight);
        assert!(end.has_skylight);
    }

    /// The built-in lookup is the same decode over the same bytes: a caller
    /// with no loaded generation must read exactly what one holding the vanilla
    /// pack would.
    #[test]
    fn the_built_in_lookup_answers_what_the_snapshot_element_decodes_to() {
        for (path, ambient_light) in [
            ("overworld", 0.0),
            ("overworld_caves", 0.0),
            ("the_nether", 0.1),
            ("the_end", 0.25),
        ] {
            let expected = shape(path);
            assert_eq!(
                builtin_dimension_type_shape(&format!("minecraft:{path}")).unwrap(),
                expected,
                "built-in {path}"
            );
            assert_eq!(
                bbb_server_types::dimension_type::builtin_dimension_type_shape_facts(&format!(
                    "minecraft:{path}"
                )),
                Some(expected),
                "built-in facts {path}"
            );
            assert_eq!(
                builtin_dimension_type_runtime_facts(&format!("minecraft:{path}"))
                    .unwrap()
                    .ambient_light,
                ambient_light,
                "built-in {path} ambient light"
            );
        }
        let overworld = builtin_dimension_type_runtime_facts("minecraft:overworld").unwrap();
        assert_eq!(overworld.element_id, "minecraft:overworld");
        assert_eq!(overworld.infiniburn_tag, "minecraft:infiniburn_overworld");
        assert_eq!(
            overworld.shape,
            builtin_dimension_type_shape("minecraft:overworld").unwrap()
        );
    }
}
