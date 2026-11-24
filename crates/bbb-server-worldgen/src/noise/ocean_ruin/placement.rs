//! `OceanRuinPiece.postProcess` plus `TemplateStructurePiece.postProcess`.

use std::sync::Arc;

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;

use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::structure_processor::{block_match_replace_with_loot, StructureProcessor};
use crate::structure_template_placement::{
    place_materialized_template_with_palette, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::value_providers::IntProvider;
use crate::{block_is, block_set_property, catalog_default_state, WorldGenLevel};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};
use bbb_server_types::block_state_parse::parse_block_state_prefix;
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidFamily};

use super::{
    transform_about_origin, OceanRuinBiomeTemp, OceanRuinPiece, OceanRuinStructureStart,
    OCEAN_RUIN_BIG_LOOT, OCEAN_RUIN_COLD_ARCHAEOLOGY, OCEAN_RUIN_SMALL_LOOT,
    OCEAN_RUIN_WARM_ARCHAEOLOGY,
};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`), the `placeInWorld` update flag.
const UPDATE_CLIENTS: i32 = 2;
/// `Block.UPDATE_NEIGHBORS | UPDATE_CLIENTS` (`TemplateStructurePiece.java:114`).
const UPDATE_JIGSAW_FINAL: i32 = 3;
/// `OceanRuinPieces.getHeight` sentinel (`OceanRuinPieces.java:370`).
const GET_HEIGHT_MIN_SENTINEL: i32 = 512;
/// `CappedProcessor` archaeology cap (`OceanRuinPieces.java:140`).
const ARCHY_CAP: i32 = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OceanRuinChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl OceanRuinStructureStart {
    /// `StructureStart.placeInChunk` then `OceanRuinPiece.postProcess`
    /// (`StructureStart.java:93-96`, `OceanRuinPieces.java:346-366`,
    /// `TemplateStructurePiece.java:82-117`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> OceanRuinChunkPlacement {
        let mut report = OceanRuinChunkPlacement::default();
        for index in 0..self.pieces.len() {
            let template_id = self.pieces[index].template().to_string();
            let live_template = level.structure_template(&template_id);
            let Some(template) = live_template.as_ref().or_else(|| {
                templates
                    .template(&template_id)
                    .map(|element| &element.template)
            }) else {
                continue;
            };
            let template = template.materialized();
            if !piece_template_bounds(
                self.pieces[index].template_position(),
                self.pieces[index].rotation(),
                template.size,
            )
            .intersects(chunk_bounds)
            {
                continue;
            }
            adjust_height(&mut self.pieces[index], template.size, level);
            self.cached_bounding_box.invalidate();
            report.attempted = true;
            if place_template(&self.pieces[index], template, level, chunk_bounds, random) {
                report.placed = true;
            }
        }
        report
    }
}

/// `StructureTemplate.getBoundingBox` with `Mirror.NONE` and pivot `ZERO`
/// (`StructureTemplate.java:615-629`).
fn piece_template_bounds(
    position: BlockPos,
    rotation: JigsawRotation,
    size: [i32; 3],
) -> JigsawBoundingBox {
    if size == [0, 0, 0] {
        return JigsawBoundingBox {
            min: position,
            max: position,
        };
    }
    let corner1 = transform_about_origin(BlockPos { x: 0, y: 0, z: 0 }, rotation);
    let corner2 = transform_about_origin(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        rotation,
    );
    JigsawBoundingBox {
        min: BlockPos {
            x: corner1.x.min(corner2.x) + position.x,
            y: corner1.y.min(corner2.y) + position.y,
            z: corner1.z.min(corner2.z) + position.z,
        },
        max: BlockPos {
            x: corner1.x.max(corner2.x) + position.x,
            y: corner1.y.max(corner2.y) + position.y,
            z: corner1.z.max(corner2.z) + position.z,
        },
    }
}

/// `OceanRuinPieces.java:355-364`.
fn adjust_height<L: WorldGenLevel>(piece: &mut OceanRuinPiece, size: [i32; 3], level: &L) {
    let origin = piece.template_position();
    let height = level.get_ocean_floor_wg_height(origin.x, origin.z);
    let origin = BlockPos {
        x: origin.x,
        y: height,
        z: origin.z,
    };
    piece.set_template_position(origin);
    let corner = wrapping_add(
        transform_about_origin(
            BlockPos {
                x: size[0].wrapping_sub(1),
                y: 0,
                z: size[2].wrapping_sub(1),
            },
            piece.rotation(),
        ),
        origin,
    );
    piece.set_template_position(BlockPos {
        x: origin.x,
        y: sample_settled_height(origin, level, corner),
        z: origin.z,
    });
}

/// `OceanRuinPieces.getHeight` (`OceanRuinPieces.java:368-400`).
fn sample_settled_height<L: WorldGenLevel>(pos: BlockPos, level: &L, corner: BlockPos) -> i32 {
    let mut new_y = pos.y;
    let mut min_y = GET_HEIGHT_MIN_SENTINEL;
    let top_y = pos.y.wrapping_sub(1);
    let mut area = 0;
    let min_x = pos.x.min(corner.x);
    let max_x = pos.x.max(corner.x);
    let min_z = pos.z.min(corner.z);
    let max_z = pos.z.max(corner.z);
    let mut z = min_z;
    while z != max_z.wrapping_add(1) {
        let mut x = min_x;
        while x != max_x.wrapping_add(1) {
            let floor_y = column_floor_y(level, x, z, pos.y);
            min_y = min_y.min(floor_y);
            if floor_y < top_y.wrapping_sub(2) {
                area += 1;
            }
            x = x.wrapping_add(1);
        }
        z = z.wrapping_add(1);
    }
    let width = pos.x.wrapping_sub(corner.x).abs();
    if top_y.wrapping_sub(min_y) > 2 && area > width.wrapping_sub(2) {
        new_y = min_y.wrapping_add(1);
    }
    new_y
}

fn column_floor_y<L: WorldGenLevel>(level: &L, x: i32, z: i32, template_y: i32) -> i32 {
    let mut floor_y = template_y.wrapping_sub(1);
    let mut pos = BlockPos { x, y: floor_y, z };
    let mut state = level.get_block(pos);
    while is_open_ocean_column(level, state) && floor_y > level.get_min_y().wrapping_add(1) {
        floor_y = floor_y.wrapping_sub(1);
        pos.y = floor_y;
        state = level.get_block(pos);
    }
    floor_y
}

/// `OceanRuinPieces.java:381-382`.
fn is_open_ocean_column<L: WorldGenLevel>(level: &L, state: i32) -> bool {
    if block_state_is_air(state)
        || block_state_fluid_state(state).family() == Some(FluidFamily::Water)
    {
        return true;
    }
    level.block_state_in_tag(state, "minecraft:ice")
}

/// `TemplateStructurePiece.postProcess` (`TemplateStructurePiece.java:91-117`)
/// with `OceanRuinPieces.makeSettings` (`OceanRuinPieces.java:289-298`).
fn place_template<L: WorldGenLevel>(
    piece: &OceanRuinPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = make_settings(piece);
    let origin = piece.template_position();
    // Ocean-ruin settings use Mirror.NONE and pivot ZERO (`OceanRuinPieces.java:289-298`).
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: RuinedPortalMirror::None,
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: false,
            bounding_box: chunk_bounds,
            processors: &processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: UPDATE_CLIENTS,
            known_shape: true,
        },
        random,
        TemplateSettingsRandom::Positional,
    ) else {
        return false;
    };
    handle_data_markers(piece, palette, chunk_bounds, level, random);
    replace_jigsaw_blocks(piece, palette, chunk_bounds, level);
    true
}

/// `OceanRuinPieces.makeSettings` (`OceanRuinPieces.java:289-298`).
fn make_settings(piece: &OceanRuinPiece) -> Vec<StructureProcessor> {
    let (candidate, replacement, loot) = match piece.biome_temp() {
        OceanRuinBiomeTemp::Warm => (
            "minecraft:sand",
            catalog_default_state("minecraft:suspicious_sand"),
            OCEAN_RUIN_WARM_ARCHAEOLOGY,
        ),
        OceanRuinBiomeTemp::Cold => (
            "minecraft:gravel",
            catalog_default_state("minecraft:suspicious_gravel"),
            OCEAN_RUIN_COLD_ARCHAEOLOGY,
        ),
    };
    vec![
        StructureProcessor::BlockRot {
            rottable_blocks: None,
            integrity: piece.integrity().get(),
        },
        StructureProcessor::BlockIgnore {
            blocks: vec![
                catalog_default_state("minecraft:air"),
                catalog_default_state("minecraft:structure_block"),
            ],
        },
        StructureProcessor::Capped {
            delegate: Box::new(StructureProcessor::Rule {
                rules: vec![block_match_replace_with_loot(candidate, replacement, loot)],
            }),
            limit: IntProvider::Constant(ARCHY_CAP),
        },
    ]
}

/// `OceanRuinPieces.handleDataMarker` (`OceanRuinPieces.java:319-342`).
fn handle_data_markers<L: WorldGenLevel>(
    piece: &OceanRuinPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        chunk_bounds,
        "minecraft:structure_block",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        if !compound_string(nbt, "mode").is_some_and(|mode| mode.eq_ignore_ascii_case("data")) {
            continue;
        }
        match compound_string(nbt, "metadata")
            .unwrap_or_default()
            .as_str()
        {
            "chest" => place_chest(piece, pos, level, random),
            "drowned" => place_drowned(pos, level),
            _ => {}
        }
    }
}

fn place_chest<L: WorldGenLevel>(
    piece: &OceanRuinPiece,
    pos: BlockPos,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    let mut state = catalog_default_state("minecraft:chest");
    if block_state_fluid_state(level.get_block(pos)).family() == Some(FluidFamily::Water) {
        state = block_set_property(state, "waterlogged", "true");
    }
    let _ = level.set_block(pos, state, UPDATE_CLIENTS);
    let loot = if piece.is_large() {
        OCEAN_RUIN_BIG_LOOT
    } else {
        OCEAN_RUIN_SMALL_LOOT
    };
    let seed = random.next_long();
    level.set_block_entity_loot_table(pos, loot, seed);
}

/// `OceanRuinPieces.handleDataMarker` drowned arm (`OceanRuinPieces.java:329-341`).
fn place_drowned<L: WorldGenLevel>(pos: BlockPos, level: &mut L) {
    // `Entity.snapTo(BlockPos)` (`Entity.java:1705-1706`) = `getBottomCenter`.
    let position = [
        f64::from(pos.x) + 0.5,
        f64::from(pos.y),
        f64::from(pos.z) + 0.5,
    ];
    // create == null (`:331`): leave the marker cell; Vanilla mutates only when drowned != null.
    level.add_structure_mob("minecraft:drowned", position);
    let filler = if pos.y > level.get_sea_level() {
        "minecraft:air"
    } else {
        "minecraft:water"
    };
    let state = catalog_default_state(filler);
    let _ = level.set_block(pos, state, UPDATE_CLIENTS);
}

fn replace_jigsaw_blocks<L: WorldGenLevel>(
    piece: &OceanRuinPiece,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        chunk_bounds,
        "minecraft:jigsaw",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        let state_string =
            compound_string(nbt, "final_state").unwrap_or_else(|| "minecraft:air".into());
        // An unparseable `final_state` logs and places air (`TemplateStructurePiece.java:103-114`).
        let state = parse_block_state_prefix(&state_string)
            .unwrap_or_else(|| catalog_default_state("minecraft:air"));
        let _ = level.set_block(pos, state, UPDATE_JIGSAW_FINAL);
    }
}

/// `StructureTemplate.filterBlocks` (`StructureTemplate.java:220-236`).
fn filter_blocks(
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    origin: BlockPos,
    rotation: JigsawRotation,
    chunk_bounds: JigsawBoundingBox,
    block_name: &str,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, block_name))
        .filter_map(|block| {
            let pos = wrapping_add(transform_about_origin(block.position, rotation), origin);
            chunk_bounds
                .contains(pos)
                .then_some((pos, block.nbt.clone()))
        })
        .collect()
}

fn wrapping_add(left: BlockPos, right: BlockPos) -> BlockPos {
    BlockPos {
        x: left.x.wrapping_add(right.x),
        y: left.y.wrapping_add(right.y),
        z: left.z.wrapping_add(right.z),
    }
}

fn compound_string(value: &NbtSummaryValue, name: &str) -> Option<String> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    fields.iter().rev().find_map(|entry| {
        (entry.name == name)
            .then_some(&entry.value)
            .and_then(|value| match value {
                NbtSummaryValue::String(text) => text.try_to_string().ok(),
                _ => None,
            })
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use bbb_protocol::block_state_types::vanilla_block_state_identity;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::random::WorldgenRandom;
    use bbb_server_datapack::structure_template::{
        StructureTemplate, StructureTemplateBlock, StructureTemplateElement,
        StructureTemplateEntity, StructureTemplatePalette, StructureTemplateSet,
    };

    use super::super::{OceanRuinFloat, OceanRuinGenerationStub, OceanRuinPiece};
    use super::*;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        max_y: i32,
        min_y: i32,
        ocean: BTreeMap<(i32, i32), i32>,
        writes: Vec<(BlockPos, i32, i32)>,
        loot: Vec<(BlockPos, &'static str, i64)>,
        brushable: Vec<(BlockPos, &'static str, i64)>,
        loot_available: bool,
        seed: i64,
        structure_mobs: Vec<(String, [f64; 3])>,
        template_entities: Vec<(NbtSummaryValue, [f64; 3], [f32; 2])>,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                max_y: 319,
                min_y: -64,
                ocean: BTreeMap::new(),
                writes: Vec::new(),
                loot: Vec::new(),
                brushable: Vec::new(),
                loot_available: true,
                seed: 0,
                structure_mobs: Vec::new(),
                template_entities: Vec::new(),
            }
        }

        fn set_ocean(&mut self, x: i32, z: i32, height: i32) {
            self.ocean.insert((x, z), height);
        }

        fn fill_column_solid(&mut self, x: i32, z: i32, top: i32, state: i32) {
            for y in self.min_y..=top {
                self.cells.insert((x, y, z), state);
            }
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn registry_tag_contains(&self, registry_id: &str, tag_id: &str, element_id: &str) -> bool {
            crate::tests::builtin_registry_tag_contains(registry_id, tag_id, element_id)
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells
                .get(&(pos.x, pos.y, pos.z))
                .copied()
                .unwrap_or(self.air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
            self.writes.push((pos, state, update_flags));
            if state == self.air {
                self.cells.remove(&(pos.x, pos.y, pos.z));
            } else {
                self.cells.insert((pos.x, pos.y, pos.z), state);
            }
            true
        }

        fn get_min_y(&self) -> i32 {
            self.min_y
        }

        fn get_max_y(&self) -> i32 {
            self.max_y
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn get_ocean_floor_wg_height(&self, x: i32, z: i32) -> i32 {
            self.ocean.get(&(x, z)).copied().unwrap_or(self.max_y)
        }

        fn get_seed(&self) -> i64 {
            self.seed
        }

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
        }

        fn set_brushable_block_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            loot_table_seed: i64,
        ) {
            self.brushable.push((pos, loot_table, loot_table_seed));
        }

        fn add_structure_mob(&mut self, resource_id: &str, position: [f64; 3]) {
            self.structure_mobs
                .push((resource_id.to_string(), position));
        }

        fn add_template_entity(
            &mut self,
            nbt: &NbtSummaryValue,
            position: [f64; 3],
            rotation: [f32; 2],
        ) {
            self.template_entities
                .push((nbt.clone(), position, rotation));
        }
    }

    fn at(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(name: &'static str) -> i32 {
        catalog_default_state(name)
    }

    fn block(x: i32, y: i32, z: i32, state: i32) -> StructureTemplateBlock {
        StructureTemplateBlock {
            position: at(x, y, z),
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

    fn templates(
        id: &str,
        size: [i32; 3],
        blocks: Vec<StructureTemplateBlock>,
    ) -> StructureTemplateSet {
        templates_with_entities(id, size, blocks, Vec::new())
    }

    fn templates_with_entities(
        id: &str,
        size: [i32; 3],
        blocks: Vec<StructureTemplateBlock>,
        entities: Vec<StructureTemplateEntity>,
    ) -> StructureTemplateSet {
        StructureTemplateSet {
            templates: vec![StructureTemplateElement {
                id: id.to_string(),
                source_pack: "test".to_string(),
                template: StructureTemplate::from_materialized(MaterializedStructureTemplate {
                    size,
                    palettes: vec![palette(blocks)],
                    entities,
                })
                .unwrap(),
            }],
            warnings: Vec::new(),
        }
    }

    fn start(
        template: &str,
        origin: BlockPos,
        biome_temp: OceanRuinBiomeTemp,
        integrity: f32,
        is_large: bool,
    ) -> OceanRuinStructureStart {
        start_with_rotation(
            template,
            origin,
            biome_temp,
            integrity,
            is_large,
            JigsawRotation::None,
        )
    }

    fn start_with_rotation(
        template: &str,
        origin: BlockPos,
        biome_temp: OceanRuinBiomeTemp,
        integrity: f32,
        is_large: bool,
        rotation: JigsawRotation,
    ) -> OceanRuinStructureStart {
        OceanRuinStructureStart::from_generation_stub(
            OceanRuinGenerationStub::new(
                [0, 0],
                origin,
                if biome_temp == OceanRuinBiomeTemp::Warm {
                    super::super::OCEAN_RUIN_WARM_ID
                } else {
                    super::super::OCEAN_RUIN_COLD_ID
                },
                vec![OceanRuinPiece::new(
                    template,
                    origin,
                    rotation,
                    biome_temp,
                    OceanRuinFloat::from_f32(integrity),
                    is_large,
                )],
            ),
            1,
        )
    }

    fn wide_bounds() -> JigsawBoundingBox {
        JigsawBoundingBox {
            min: at(-1000, -1000, -1000),
            max: at(1000, 1000, 1000),
        }
    }

    fn place(
        start: &mut OceanRuinStructureStart,
        templates: &StructureTemplateSet,
        level: &mut PlacementLevel,
        random: &mut dyn RandomSource,
    ) -> OceanRuinChunkPlacement {
        start.place_in_chunk(templates, level, wide_bounds(), random)
    }

    #[test]
    fn ocean_floor_height_stays_when_the_footprint_is_solid() {
        let id = "minecraft:underwater_ruin/warm_1";
        let stone = state("minecraft:stone");
        let templates = templates(
            id,
            [2, 1, 2],
            vec![block(0, 0, 0, state("minecraft:prismarine"))],
        );
        let origin = at(4, 90, 8);
        let mut level = PlacementLevel::new();
        level.set_ocean(4, 8, 12);
        for (x, z) in [(4, 8), (5, 8), (4, 9), (5, 9)] {
            level.fill_column_solid(x, z, 11, stone);
        }
        let mut start = start(id, origin, OceanRuinBiomeTemp::Warm, 1.0, false);
        let report = place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(start.pieces()[0].template_position().y, 12);
    }

    #[test]
    fn place_in_chunk_skips_pieces_that_miss_the_writable_area() {
        let near_id = "minecraft:underwater_ruin/warm_1";
        let far_id = "minecraft:underwater_ruin/warm_2";
        let stone = state("minecraft:stone");
        let templates = StructureTemplateSet {
            templates: vec![
                templates(
                    near_id,
                    [2, 1, 2],
                    vec![block(0, 0, 0, state("minecraft:prismarine"))],
                )
                .templates
                .remove(0),
                templates(
                    far_id,
                    [2, 1, 2],
                    vec![block(0, 0, 0, state("minecraft:prismarine"))],
                )
                .templates
                .remove(0),
            ],
            warnings: Vec::new(),
        };
        let mut start = OceanRuinStructureStart::from_generation_stub(
            OceanRuinGenerationStub::new(
                [0, 0],
                at(4, 90, 8),
                super::super::OCEAN_RUIN_WARM_ID,
                vec![
                    OceanRuinPiece::new(
                        near_id,
                        at(4, 90, 8),
                        JigsawRotation::None,
                        OceanRuinBiomeTemp::Warm,
                        OceanRuinFloat::from_f32(1.0),
                        false,
                    ),
                    OceanRuinPiece::new(
                        far_id,
                        at(80, 90, 80),
                        JigsawRotation::None,
                        OceanRuinBiomeTemp::Warm,
                        OceanRuinFloat::from_f32(1.0),
                        false,
                    ),
                ],
            ),
            1,
        );
        let mut level = PlacementLevel::new();
        level.set_ocean(4, 8, 12);
        for (x, z) in [(4, 8), (5, 8), (4, 9), (5, 9)] {
            level.fill_column_solid(x, z, 11, stone);
        }
        let report = start.place_in_chunk(
            &templates,
            &mut level,
            JigsawBoundingBox {
                min: at(0, -64, 0),
                max: at(15, 320, 15),
            },
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(start.pieces()[0].template_position().y, 12);
        assert_eq!(start.pieces()[1].template_position().y, 90);
    }

    #[test]
    fn get_height_sinks_when_enough_columns_are_deep() {
        let id = "minecraft:underwater_ruin/warm_2";
        let stone = state("minecraft:stone");
        let templates = templates(
            id,
            [3, 1, 1],
            vec![block(0, 0, 0, state("minecraft:prismarine"))],
        );
        let origin = at(0, 90, 0);
        let mut level = PlacementLevel::new();
        level.set_ocean(0, 0, 10);
        for x in 0..3 {
            level.fill_column_solid(x, 0, 5, stone);
        }
        let mut start = start(id, origin, OceanRuinBiomeTemp::Warm, 1.0, false);
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(start.pieces()[0].template_position().y, 6);
    }

    #[test]
    fn chest_marker_places_waterlogged_loot_on_the_marker() {
        let id = "minecraft:underwater_ruin/markers";
        let sand = state("minecraft:sand");
        let water = state("minecraft:water");
        let structure = state("minecraft:structure_block");
        let templates = templates(
            id,
            [1, 2, 1],
            vec![
                block(0, 0, 0, sand),
                StructureTemplateBlock {
                    position: at(0, 1, 0),
                    state: structure,
                    nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "mode".into(),
                            value: NbtSummaryValue::string("DATA"),
                        },
                        NbtSummaryEntry {
                            name: "metadata".into(),
                            value: NbtSummaryValue::string("chest"),
                        },
                    ]))),
                },
            ],
        );
        let origin = at(2, 8, 3);
        let mut start = start(id, origin, OceanRuinBiomeTemp::Warm, 1.0, true);
        let mut level = PlacementLevel::new();
        level.set_ocean(2, 3, 8);
        level.fill_column_solid(2, 3, 7, state("minecraft:stone"));
        level.cells.insert((2, 9, 3), water);
        let mut random = WorldgenRandom::with_seed(9);
        place(&mut start, &templates, &mut level, &mut random);
        let mut expected = WorldgenRandom::with_seed(9);
        let seed = expected.next_long();
        assert!(
            level
                .writes
                .iter()
                .any(|(pos, state, flags)| *pos == at(2, 9, 3)
                    && *flags == UPDATE_CLIENTS
                    && block_is(*state, "minecraft:chest")),
            "{:?}",
            level.writes
        );
        let chest = level.get_block(at(2, 9, 3));
        assert!(block_is(chest, "minecraft:chest"));
        assert_eq!(
            vanilla_block_state_identity(chest)
                .and_then(|identity| identity.properties.get("waterlogged").cloned()),
            Some("true".to_string())
        );
        assert_eq!(level.loot, vec![(at(2, 9, 3), OCEAN_RUIN_BIG_LOOT, seed)]);
    }

    fn drowned_data_marker() -> StructureTemplateBlock {
        StructureTemplateBlock {
            position: at(0, 0, 0),
            state: state("minecraft:structure_block"),
            nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "mode".into(),
                    value: NbtSummaryValue::string("DATA"),
                },
                NbtSummaryEntry {
                    name: "metadata".into(),
                    value: NbtSummaryValue::string("drowned"),
                },
            ]))),
        }
    }

    fn place_drowned_marker(origin: BlockPos) -> PlacementLevel {
        let id = "minecraft:underwater_ruin/drowned";
        let templates = templates(id, [1, 1, 1], vec![drowned_data_marker()]);
        let mut start = start(id, origin, OceanRuinBiomeTemp::Cold, 1.0, false);
        let mut level = PlacementLevel::new();
        level.set_ocean(origin.x, origin.z, origin.y);
        level.fill_column_solid(origin.x, origin.z, origin.y - 1, state("minecraft:stone"));
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        level
    }

    fn bottom_center(pos: BlockPos) -> [f64; 3] {
        [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y),
            f64::from(pos.z) + 0.5,
        ]
    }

    #[test]
    fn drowned_marker_places_at_bottom_center_and_replaces_the_cell() {
        let below = at(0, 4, 0);
        let below_level = place_drowned_marker(below);
        assert_eq!(
            below_level.structure_mobs,
            vec![("minecraft:drowned".to_string(), bottom_center(below))]
        );
        assert!(
            below_level
                .writes
                .iter()
                .any(|(pos, written, flags)| *pos == below
                    && *flags == UPDATE_CLIENTS
                    && block_is(*written, "minecraft:water")),
            "{:?}",
            below_level.writes
        );
        assert!(block_is(below_level.get_block(below), "minecraft:water"));

        let above = at(0, 64, 0);
        let above_level = place_drowned_marker(above);
        assert_eq!(
            above_level.structure_mobs,
            vec![("minecraft:drowned".to_string(), bottom_center(above))]
        );
        assert!(
            above_level
                .writes
                .iter()
                .any(|(pos, written, flags)| *pos == above
                    && *flags == UPDATE_CLIENTS
                    && block_is(*written, "minecraft:air")),
            "{:?}",
            above_level.writes
        );
        assert!(block_is(above_level.get_block(above), "minecraft:air"));
    }

    #[test]
    fn clockwise90_transforms_the_settled_corner() {
        let id = "minecraft:underwater_ruin/rotate";
        let stone = state("minecraft:stone");
        let templates = templates(
            id,
            [3, 1, 1],
            vec![block(0, 0, 0, state("minecraft:prismarine"))],
        );
        let origin = at(10, 90, 20);
        let mut level = PlacementLevel::new();
        level.set_ocean(10, 20, 10);
        for z in 20..=22 {
            level.fill_column_solid(10, z, 5, stone);
        }
        let mut start = start_with_rotation(
            id,
            origin,
            OceanRuinBiomeTemp::Warm,
            1.0,
            false,
            JigsawRotation::Clockwise90,
        );
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(start.pieces()[0].template_position().y, 6);
    }

    #[test]
    fn builtin_ocean_ruin_template_writes_inside_the_chunk_bb() {
        use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
        use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

        let Ok(startup) = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        }) else {
            return;
        };
        let Some(template) = startup
            .structures
            .templates
            .iter()
            .find(|template| template.id.starts_with("minecraft:underwater_ruin/"))
        else {
            return;
        };
        if !template.template.materialized().entities.is_empty() {
            return;
        }
        let origin = at(0, 64, 0);
        let mut start = start(&template.id, origin, OceanRuinBiomeTemp::Warm, 1.0, false);
        let mut level = PlacementLevel::new();
        level.set_ocean(0, 0, 64);
        level.fill_column_solid(0, 0, 63, state("minecraft:stone"));
        let report = start.place_in_chunk(
            &startup.structures,
            &mut level,
            wide_bounds(),
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted);
        assert!(report.placed);
        assert!(
            level.writes.iter().any(|(_, block, flags)| {
                *flags == UPDATE_CLIENTS && !block_is(*block, "minecraft:air")
            }),
            "{}",
            template.id
        );
    }
}
