//! Block placement for the bounded jigsaw piece graph.

use bbb_protocol::packets::BlockPos;
use bbb_registry_codec::random_source::RandomSource;

use crate::placed_feature::place_placed_feature;
use crate::structure_processor::{Heightmap, StructureProcessor};
use crate::structure_template_placement::{
    place_materialized_template, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{catalog_default_state, WorldGenLevel};
use bbb_server_datapack::dynamic_registry::DatapackGeneration;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::structure_pool::{
    PoolFeatureHolder, ProcessorListHolder, StructureLiquidSettings, StructurePoolElement,
    StructurePoolProjection,
};
use bbb_server_datapack::structure_template::StructureTemplateSet;

use super::structure_pieces::{JigsawBoundingBox, JigsawPiece, JigsawPieceGraph, JigsawRotation};

const UPDATE_KNOWN_SHAPE: i32 = 18;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JigsawChunkPlacement {
    pub attempted_pieces: usize,
    pub successful_pieces: usize,
}

impl JigsawPieceGraph {
    /// `JigsawPlacement.generateJigsaw` places every piece using the command position
    /// as reference, without chunk clipping (`JigsawPlacement.java:251-258`).
    pub fn place_jigsaw<L: WorldGenLevel>(
        &self,
        startup: &DatapackStartup,
        level: &mut L,
        reference_pos: BlockPos,
        random: &mut dyn RandomSource,
    ) {
        self.place_jigsaw_with_keep_jigsaws(startup, level, reference_pos, random, false)
    }

    // JigsawBlockEntity.java:139-144; SinglePoolElement.java:164-181.
    pub fn place_jigsaw_with_keep_jigsaws<L: WorldGenLevel>(
        &self,
        startup: &DatapackStartup,
        level: &mut L,
        reference_pos: BlockPos,
        random: &mut dyn RandomSource,
        keep_jigsaws: bool,
    ) {
        let bounds = JigsawBoundingBox {
            min: BlockPos {
                x: i32::MIN,
                y: i32::MIN,
                z: i32::MIN,
            },
            max: BlockPos {
                x: i32::MAX,
                y: i32::MAX,
                z: i32::MAX,
            },
        };
        for piece in &self.pieces {
            place_pool_element(
                &piece.element,
                &startup.structures,
                &startup.registries,
                level,
                piece.position,
                reference_pos,
                piece.rotation,
                bounds,
                random,
                self.liquid_settings,
                keep_jigsaws,
            );
        }
    }

    /// `StructureStart.placeInChunk` followed by
    /// `PoolElementStructurePiece.postProcess` (`StructureStart.java:79-99`,
    /// `PoolElementStructurePiece.java:91-120`). The later StructureStart owner
    /// supplies the feature-seeded stream and publishes the persisted pieces.
    pub fn place_blocks_in_chunk<L: WorldGenLevel>(
        &self,
        startup: &DatapackStartup,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> JigsawChunkPlacement {
        place_jigsaw_pieces_in_chunk(
            &self.pieces,
            self.liquid_settings,
            startup,
            level,
            chunk_bounds,
            random,
        )
    }
}

pub(crate) fn place_jigsaw_pieces_in_chunk<L: WorldGenLevel>(
    pieces: &[JigsawPiece],
    liquid_settings: StructureLiquidSettings,
    startup: &DatapackStartup,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> JigsawChunkPlacement {
    let mut report = JigsawChunkPlacement::default();
    let Some(reference_pos) = structure_reference_pos(pieces) else {
        return report;
    };
    for piece in pieces {
        if !piece.bounding_box.intersects(chunk_bounds) {
            continue;
        }
        report.attempted_pieces += 1;
        if place_pool_element(
            &piece.element,
            &startup.structures,
            &startup.registries,
            level,
            piece.position,
            reference_pos,
            piece.rotation,
            chunk_bounds,
            random,
            liquid_settings,
            false,
        ) {
            report.successful_pieces += 1;
        }
    }
    report
}

#[allow(clippy::too_many_arguments)]
fn place_pool_element<L: WorldGenLevel>(
    element: &StructurePoolElement,
    templates: &StructureTemplateSet,
    generation: &DatapackGeneration,
    level: &mut L,
    position: BlockPos,
    reference_pos: BlockPos,
    rotation: JigsawRotation,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
    default_liquid_settings: StructureLiquidSettings,
    keep_jigsaws: bool,
) -> bool {
    match element {
        StructurePoolElement::Empty => true,
        // FeaturePoolElement.java:81-94: the placed feature at the piece origin,
        // unclipped by the chunk box.
        StructurePoolElement::Feature { feature, .. } => match feature {
            PoolFeatureHolder::Named(id) => level
                .placed_feature(id)
                .is_some_and(|placed| place_placed_feature(&placed, level, random, position)),
            PoolFeatureHolder::Inline { feature, .. } => {
                place_placed_feature(feature, level, random, position)
            }
        },
        StructurePoolElement::Single {
            legacy,
            location,
            processors,
            projection,
            liquid_settings,
        } => {
            let Some(template) = level.structure_template(location).or_else(|| {
                templates
                    .template(location)
                    .map(|entry| entry.template.clone())
            }) else {
                // `StructureTemplateManager.getOrCreate` returns an empty
                // template for an unknown id (`StructureTemplateManager.java:83-92`).
                return false;
            };
            let processors =
                placement_processors(*legacy, processors, *projection, generation, keep_jigsaws);
            place_materialized_template(
                template.materialized(),
                level,
                MaterializedTemplatePlacement {
                    position,
                    reference_pos,
                    rotation,
                    mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                    pivot: BlockPos { x: 0, y: 0, z: 0 },
                    finalize_entities: true,
                    ignore_entities: false,
                    bounding_box: chunk_bounds,
                    processors: &processors,
                    liquid_settings: liquid_settings.unwrap_or(default_liquid_settings),
                    update_mode: UPDATE_KNOWN_SHAPE,
                    known_shape: true,
                },
                random,
                TemplateSettingsRandom::Positional,
            )
        }
        // `ListPoolElement.place` is intentionally not transactional: an
        // earlier child remains placed when a later child returns false
        // (`ListPoolElement.java:71-93`).
        StructurePoolElement::List { elements, .. } => {
            for child in elements {
                if !place_pool_element(
                    child,
                    templates,
                    generation,
                    level,
                    position,
                    reference_pos,
                    rotation,
                    chunk_bounds,
                    random,
                    default_liquid_settings,
                    keep_jigsaws,
                ) {
                    return false;
                }
            }
            true
        }
    }
}

fn placement_processors(
    legacy: bool,
    holder: &ProcessorListHolder,
    projection: StructurePoolProjection,
    generation: &DatapackGeneration,
    keep_jigsaws: bool,
) -> Vec<StructureProcessor> {
    let structure_block = catalog_default_state("minecraft:structure_block");
    let air = catalog_default_state("minecraft:air");
    let mut processors = Vec::new();
    if !legacy {
        processors.push(StructureProcessor::BlockIgnore {
            blocks: vec![structure_block],
        });
    }
    // `SinglePoolElement.getSettings` adds replacement before the named and
    // projection processors (`SinglePoolElement.java:164-181`).
    if !keep_jigsaws {
        processors.push(StructureProcessor::JigsawReplacement);
    }
    match holder {
        ProcessorListHolder::InlineEmpty => {}
        ProcessorListHolder::Inline { list, .. } => {
            processors.extend(list.0.iter().cloned());
        }
        ProcessorListHolder::Named(resource_id) => {
            if let Some(list) = generation.worldgen.processor_lists.get(resource_id) {
                processors.extend(list.0.iter().cloned());
            }
        }
    }
    if projection == StructurePoolProjection::TerrainMatching {
        processors.push(StructureProcessor::Gravity {
            heightmap: Heightmap::WorldSurfaceWg,
            offset: -1,
        });
    }
    // `LegacySinglePoolElement.getSettings` removes the first ignore processor
    // and appends STRUCTURE_AND_AIR after every other processor (`:28-34`).
    if legacy {
        processors.push(StructureProcessor::BlockIgnore {
            blocks: vec![structure_block, air],
        });
    }
    processors
}

fn structure_reference_pos(pieces: &[JigsawPiece]) -> Option<BlockPos> {
    let first = pieces.first()?;
    let center = |minimum: i32, maximum: i32| {
        minimum.wrapping_add(maximum.wrapping_sub(minimum).wrapping_add(1) / 2)
    };
    Some(BlockPos {
        x: center(first.bounding_box.min.x, first.bounding_box.max.x),
        y: first.bounding_box.min.y,
        z: center(first.bounding_box.min.z, first.bounding_box.max.z),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    use crate::random::WorldgenRandom;
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::block_is;
    use crate::test_support::SeamLevel;
    use bbb_server_datapack::dynamic_registry::DatapackGeneration;
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    use bbb_server_datapack::structure_template::{
        MaterializedStructureTemplate, StructureTemplate, StructureTemplateBlock,
        StructureTemplateElement, StructureTemplateEntity, StructureTemplatePalette,
    };

    use super::super::position_seed;
    use super::super::structure_pieces::JigsawPiece;

    use super::*;

    fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        vanilla_block_state_id_by_name_and_properties(name, &properties).unwrap()
    }

    fn block(x: i32, y: i32, z: i32, state: i32) -> StructureTemplateBlock {
        StructureTemplateBlock {
            position: BlockPos { x, y, z },
            state,
            nbt: None,
        }
    }

    fn palette(blocks: Vec<StructureTemplateBlock>) -> StructureTemplatePalette {
        StructureTemplatePalette {
            blocks,
            jigsaws: Vec::new(),
        }
    }

    fn template(
        size: [i32; 3],
        palettes: Vec<StructureTemplatePalette>,
    ) -> MaterializedStructureTemplate {
        MaterializedStructureTemplate {
            size,
            palettes,
            entities: Vec::new(),
        }
    }

    fn bounds() -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: BlockPos {
                x: -100,
                y: -100,
                z: -100,
            },
            max: BlockPos {
                x: 100,
                y: 100,
                z: 100,
            },
        }
    }

    fn place(
        template: &MaterializedStructureTemplate,
        level: &mut SeamLevel,
        position: BlockPos,
        rotation: JigsawRotation,
        processors: &[StructureProcessor],
        liquids: StructureLiquidSettings,
    ) -> bool {
        let mut random = WorldgenRandom::with_seed(0);
        place_materialized_template(
            template,
            level,
            MaterializedTemplatePlacement {
                position,
                reference_pos: position,
                rotation,
                mirror: crate::noise::ruined_portal::RuinedPortalMirror::None,
                pivot: BlockPos { x: 0, y: 0, z: 0 },
                finalize_entities: true,
                ignore_entities: false,
                bounding_box: bounds(),
                processors,
                liquid_settings: liquids,
                update_mode: UPDATE_KNOWN_SHAPE,
                known_shape: true,
            },
            &mut random,
            TemplateSettingsRandom::Positional,
        )
    }

    #[test]
    fn palette_is_selected_from_the_piece_position_stream() {
        let stone = state("minecraft:stone", &[]);
        let dirt = state("minecraft:dirt", &[]);
        let template = template(
            [1, 1, 1],
            vec![
                palette(vec![block(0, 0, 0, stone)]),
                palette(vec![block(0, 0, 0, dirt)]),
            ],
        );
        let mut seen = BTreeSet::new();
        for x in 0..32 {
            let position = BlockPos { x, y: 5, z: -7 };
            let mut random = WorldgenRandom::with_seed(position_seed(x, 5, -7));
            let expected = [stone, dirt][random.next_int(2) as usize];
            let mut level = SeamLevel::admitting();
            assert!(place(
                &template,
                &mut level,
                position,
                JigsawRotation::None,
                &[],
                StructureLiquidSettings::IgnoreWaterlogging,
            ));
            assert_eq!(level.inner.get_block(position), expected);
            seen.insert(expected);
        }
        assert_eq!(seen, BTreeSet::from([stone, dirt]));
    }

    #[test]
    fn position_and_concrete_block_state_rotate_together() {
        let north = state(
            "minecraft:oak_stairs",
            &[
                ("facing", "north"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        );
        let east = state(
            "minecraft:oak_stairs",
            &[
                ("facing", "east"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        );
        let template = template([2, 1, 1], vec![palette(vec![block(1, 0, 0, north)])]);
        let mut level = SeamLevel::admitting();
        let origin = BlockPos { x: 10, y: 4, z: 20 };
        assert!(place(
            &template,
            &mut level,
            origin,
            JigsawRotation::Clockwise90,
            &[],
            StructureLiquidSettings::IgnoreWaterlogging,
        ));
        assert_eq!(level.inner.get_block(BlockPos { x: 10, y: 4, z: 21 }), east);
        assert_eq!(
            level.inner.sets.borrow().as_slice(),
            &[(BlockPos { x: 10, y: 4, z: 21 }, east, UPDATE_KNOWN_SHAPE)]
        );
    }

    #[test]
    fn jigsaw_replacement_consumes_nbt_before_the_block_entity_preflight() {
        let jigsaw = state("minecraft:jigsaw", &[("orientation", "down_north")]);
        let stone = state("minecraft:stone", &[]);
        let mut record = block(0, 0, 0, jigsaw);
        record.nbt = Some(Arc::new(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "final_state".into(),
            value: NbtSummaryValue::string("minecraft:stone"),
        }])));
        let template = template([1, 1, 1], vec![palette(vec![record])]);
        let processors = [StructureProcessor::JigsawReplacement];
        let mut level = SeamLevel::admitting();
        let origin = BlockPos { x: 2, y: 3, z: 4 };
        assert!(place(
            &template,
            &mut level,
            origin,
            JigsawRotation::None,
            &processors,
            StructureLiquidSettings::IgnoreWaterlogging,
        ));
        assert_eq!(level.inner.get_block(origin), stone);

        struct BlockEntityLevel {
            inner: SeamLevel,
            loaded: Vec<Vec<NbtSummaryEntry>>,
        }
        impl WorldGenLevel for BlockEntityLevel {
            fn ensure_can_write(&self, pos: BlockPos) -> bool {
                self.inner.ensure_can_write(pos)
            }
            fn get_block(&self, pos: BlockPos) -> i32 {
                self.inner.get_block(pos)
            }
            fn set_block(&mut self, pos: BlockPos, state: i32, flags: i32) -> bool {
                self.inner.set_block(pos, state, flags)
            }
            fn get_min_y(&self) -> i32 {
                self.inner.get_min_y()
            }
            fn get_max_y(&self) -> i32 {
                self.inner.get_max_y()
            }
            fn get_sea_level(&self) -> i32 {
                self.inner.get_sea_level()
            }
            fn schedule_fluid_tick(&mut self, pos: BlockPos, delay: i32) {
                self.inner.schedule_fluid_tick(pos, delay);
            }
            fn schedule_block_tick(&mut self, pos: BlockPos, delay: i32) {
                self.inner.schedule_block_tick(pos, delay);
            }
            fn mark_pos_for_postprocessing(&mut self, pos: BlockPos) {
                self.inner.mark_pos_for_postprocessing(pos);
            }
            fn load_structure_block_entity(
                &mut self,
                _pos: BlockPos,
                _state: i32,
                record: &[NbtSummaryEntry],
            ) {
                self.loaded.push(record.to_vec());
            }
        }
        let mut startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        startup.structures.templates.push(StructureTemplateElement {
            id: "minecraft:test/keep_jigsaw".to_owned(),
            source_pack: "test".to_owned(),
            template: StructureTemplate::from_materialized(template).unwrap(),
        });
        startup.structures.templates.sort_by(|a, b| a.id.cmp(&b.id));
        let graph = JigsawPieceGraph {
            structure_id: "minecraft:test".to_owned(),
            start_chunk: [0, 0],
            generation_anchor: origin,
            center_bounding_box: bounds(),
            liquid_settings: StructureLiquidSettings::IgnoreWaterlogging,
            pieces: vec![JigsawPiece {
                element: StructurePoolElement::List {
                    elements: vec![StructurePoolElement::Single {
                        legacy: true,
                        location: "minecraft:test/keep_jigsaw".to_owned(),
                        processors: ProcessorListHolder::InlineEmpty,
                        projection: StructurePoolProjection::Rigid,
                        liquid_settings: None,
                    }],
                    projection: StructurePoolProjection::Rigid,
                },
                position: origin,
                ground_level_delta: 0,
                rotation: JigsawRotation::None,
                bounding_box: bounds(),
            }],
            junctions: Vec::new(),
        };
        let mut kept = BlockEntityLevel {
            inner: SeamLevel::admitting(),
            loaded: Vec::new(),
        };
        graph.place_jigsaw_with_keep_jigsaws(
            &startup,
            &mut kept,
            origin,
            &mut WorldgenRandom::with_seed(0),
            true,
        );
        assert_eq!(kept.inner.get_block(origin), jigsaw);
        assert_eq!(kept.loaded.len(), 1);
        assert!(kept.loaded[0]
            .iter()
            .any(|entry| entry.name == "final_state"
                && entry.value == NbtSummaryValue::string("minecraft:stone")));
    }

    #[test]
    fn positional_processor_random_restarts_at_each_processed_block() {
        let stone = state("minecraft:stone", &[]);
        let blocks = (0..16).map(|x| block(x, 0, 0, stone)).collect();
        let template = template([16, 1, 1], vec![palette(blocks)]);
        let processors = [StructureProcessor::BlockRot {
            rottable_blocks: None,
            integrity: 0.5,
        }];
        let origin = BlockPos { x: 20, y: 8, z: 30 };
        let mut expected = Vec::new();
        for x in 0..16 {
            let position = BlockPos {
                x: origin.x + x,
                y: origin.y,
                z: origin.z,
            };
            let mut random =
                WorldgenRandom::with_seed(position_seed(position.x, position.y, position.z));
            if random.next_float() <= 0.5 {
                expected.push((position, stone, UPDATE_KNOWN_SHAPE));
            }
        }
        let mut level = SeamLevel::admitting();
        assert!(place(
            &template,
            &mut level,
            origin,
            JigsawRotation::None,
            &processors,
            StructureLiquidSettings::IgnoreWaterlogging,
        ));
        assert_eq!(*level.inner.sets.borrow(), expected);
    }

    #[test]
    fn owned_template_entities_place_after_blocks() {
        let stone = state("minecraft:stone", &[]);
        let mut entity_template = template([1, 1, 1], vec![palette(vec![block(0, 0, 0, stone)])]);
        entity_template.entities.push(StructureTemplateEntity {
            position_bits: [0.5_f64.to_bits(), 1.0_f64.to_bits(), 0.5_f64.to_bits()],
            block_position: BlockPos { x: 0, y: 1, z: 0 },
            nbt: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:pig"),
            }]),
        });
        let mut level = SeamLevel::admitting();
        let origin = BlockPos {
            x: 10,
            y: 64,
            z: 20,
        };
        assert!(place(
            &entity_template,
            &mut level,
            origin,
            JigsawRotation::None,
            &[],
            StructureLiquidSettings::IgnoreWaterlogging,
        ));
        assert_eq!(
            *level.inner.sets.borrow(),
            [(origin, stone, UPDATE_KNOWN_SHAPE)]
        );
        let added = level.template_entities.borrow();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].1, [10.5, 65.0, 20.5]);
    }

    #[test]
    fn source_water_is_retained_and_flowing_water_no_longer_refuses_placement() {
        let fence = state(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "false")],
        );
        let wet_fence = state(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "true")],
        );
        let source_water = state("minecraft:water", &[("level", "0")]);
        let flowing_water = state("minecraft:water", &[("level", "1")]);
        let template = template([1, 1, 1], vec![palette(vec![block(0, 0, 0, fence)])]);
        let origin = BlockPos { x: 4, y: 5, z: 6 };
        let mut source = SeamLevel::filled(source_water);
        assert!(place(
            &template,
            &mut source,
            origin,
            JigsawRotation::None,
            &[],
            StructureLiquidSettings::ApplyWaterlogging,
        ));
        assert_eq!(source.inner.get_block(origin), wet_fence);

        let mut flowing = SeamLevel::filled(flowing_water);
        assert!(place(
            &template,
            &mut flowing,
            origin,
            JigsawRotation::None,
            &[],
            StructureLiquidSettings::ApplyWaterlogging,
        ));
        assert_eq!(flowing.inner.get_block(origin), fence);
    }

    #[test]
    fn list_children_keep_prior_writes_when_a_later_template_is_empty() {
        let stone = state("minecraft:stone", &[]);
        let materialized = template([1, 1, 1], vec![palette(vec![block(0, 0, 0, stone)])]);
        let templates = StructureTemplateSet {
            templates: vec![StructureTemplateElement {
                id: "minecraft:test/first".to_string(),
                source_pack: "test".to_string(),
                template: StructureTemplate::from_materialized(materialized).unwrap(),
            }],
            warnings: Vec::new(),
        };
        let single = |location: &str| StructurePoolElement::Single {
            legacy: false,
            location: location.to_string(),
            processors: ProcessorListHolder::InlineEmpty,
            projection: StructurePoolProjection::Rigid,
            liquid_settings: Some(StructureLiquidSettings::IgnoreWaterlogging),
        };
        let element = StructurePoolElement::List {
            elements: vec![
                single("minecraft:test/first"),
                single("minecraft:test/missing"),
            ],
            projection: StructurePoolProjection::Rigid,
        };
        let mut level = SeamLevel::admitting();
        let origin = BlockPos { x: 7, y: 8, z: 9 };
        assert!(!place_pool_element(
            &element,
            &templates,
            &DatapackGeneration {
                registries: Vec::new(),
                worldgen: Default::default(),
            },
            &mut level,
            origin,
            origin,
            JigsawRotation::None,
            bounds(),
            &mut WorldgenRandom::with_seed(0),
            StructureLiquidSettings::ApplyWaterlogging,
            false,
        ));
        assert_eq!(level.inner.get_block(origin), stone);
        assert_eq!(level.inner.sets.borrow().len(), 1);
    }

    #[test]
    fn feature_element_places_its_placed_feature_at_the_piece_origin() {
        let mut level = SeamLevel::admitting();
        level.insert_registry_json(
            crate::placed_feature::PLACED_FEATURE_REGISTRY,
            "minecraft:test_platform",
            r#"{"feature": {"type": "minecraft:void_start_platform", "config": {}},
               "placement": []}"#,
        );
        let element = StructurePoolElement::Feature {
            feature: PoolFeatureHolder::Named("minecraft:test_platform".to_string()),
            projection: StructurePoolProjection::Rigid,
        };
        let origin = BlockPos { x: 0, y: 64, z: 0 };
        assert!(place_pool_element(
            &element,
            &StructureTemplateSet {
                templates: Vec::new(),
                warnings: Vec::new(),
            },
            &DatapackGeneration {
                registries: Vec::new(),
                worldgen: Default::default(),
            },
            &mut level,
            origin,
            origin,
            JigsawRotation::None,
            bounds(),
            &mut WorldgenRandom::with_seed(0),
            StructureLiquidSettings::ApplyWaterlogging,
            false,
        ));
        assert_eq!(
            level.inner.get_block(BlockPos { x: 8, y: 67, z: 8 }),
            state("minecraft:cobblestone", &[])
        );
    }

    #[test]
    fn single_and_legacy_processor_order_matches_vanilla_settings_mutation() {
        let generation = DatapackGeneration {
            registries: Vec::new(),
            worldgen: Default::default(),
        };
        let single = placement_processors(
            false,
            &ProcessorListHolder::InlineEmpty,
            StructurePoolProjection::Rigid,
            &generation,
            false,
        );
        assert!(matches!(
            single.as_slice(),
            [StructureProcessor::BlockIgnore { blocks }, StructureProcessor::JigsawReplacement]
                if blocks.len() == 1
        ));

        let legacy = placement_processors(
            true,
            &ProcessorListHolder::InlineEmpty,
            StructurePoolProjection::TerrainMatching,
            &generation,
            false,
        );
        assert!(matches!(
            legacy.as_slice(),
            [
                StructureProcessor::JigsawReplacement,
                StructureProcessor::Gravity { heightmap: Heightmap::WorldSurfaceWg, offset: -1 },
                StructureProcessor::BlockIgnore { blocks }
            ] if blocks.len() == 2
        ));
    }

    #[test]
    fn graph_chunk_owner_filters_pieces_and_reports_empty_elements() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let point = JigsawBoundingBox {
            min: BlockPos { x: 1, y: 2, z: 3 },
            max: BlockPos { x: 1, y: 2, z: 3 },
        };
        let graph = JigsawPieceGraph {
            structure_id: "minecraft:test".to_string(),
            start_chunk: [0, 0],
            generation_anchor: point.min,
            center_bounding_box: point,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            pieces: vec![JigsawPiece {
                element: StructurePoolElement::Empty,
                position: point.min,
                ground_level_delta: 1,
                rotation: JigsawRotation::None,
                bounding_box: point,
            }],
            junctions: Vec::new(),
        };
        let mut level = SeamLevel::admitting();
        let report = graph.place_blocks_in_chunk(
            &startup,
            &mut level,
            point,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(
            report,
            JigsawChunkPlacement {
                attempted_pieces: 1,
                successful_pieces: 1,
            }
        );
        let elsewhere = JigsawBoundingBox {
            min: BlockPos { x: 9, y: 9, z: 9 },
            max: BlockPos { x: 9, y: 9, z: 9 },
        };
        assert_eq!(
            graph.place_blocks_in_chunk(
                &startup,
                &mut level,
                elsewhere,
                &mut WorldgenRandom::with_seed(0),
            ),
            JigsawChunkPlacement::default()
        );
    }

    #[test]
    fn builtin_materialized_template_crosses_the_piece_placement_path() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let template = startup
            .structures
            .templates
            .iter()
            .find(|template| {
                let materialized = template.template.materialized();
                materialized.size.iter().all(|axis| *axis > 0)
                    && materialized.entities.is_empty()
                    && materialized.palettes.iter().all(|palette| {
                        !palette.blocks.is_empty()
                            && palette.blocks.iter().all(|block| {
                                block.nbt.is_none()
                                    || block_is(block.state, "minecraft:jigsaw")
                                    || block_is(block.state, "minecraft:structure_block")
                            })
                    })
            })
            .expect("the built-in template set has a block-only placement fixture");
        let element = StructurePoolElement::Single {
            legacy: false,
            location: template.id.clone(),
            processors: ProcessorListHolder::InlineEmpty,
            projection: StructurePoolProjection::Rigid,
            liquid_settings: Some(StructureLiquidSettings::IgnoreWaterlogging),
        };
        let wide_bounds = JigsawBoundingBox {
            min: BlockPos {
                x: -1_000,
                y: -1_000,
                z: -1_000,
            },
            max: BlockPos {
                x: 1_000,
                y: 1_000,
                z: 1_000,
            },
        };
        let origin = BlockPos { x: 0, y: 0, z: 0 };
        let mut level = SeamLevel::admitting();
        assert!(place_pool_element(
            &element,
            &startup.structures,
            &startup.registries,
            &mut level,
            origin,
            origin,
            JigsawRotation::Clockwise90,
            wide_bounds,
            &mut WorldgenRandom::with_seed(0),
            StructureLiquidSettings::ApplyWaterlogging,
            false,
        ));
        let sets = level.inner.sets.borrow();
        assert!(!sets.is_empty(), "{}", template.id);
        assert!(sets.iter().all(|(_, state, flags)| {
            *flags == UPDATE_KNOWN_SHAPE
                && !block_is(*state, "minecraft:jigsaw")
                && !block_is(*state, "minecraft:structure_block")
        }));
    }
}
