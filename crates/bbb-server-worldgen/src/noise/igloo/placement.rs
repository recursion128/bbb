//! `IglooPiece.postProcess` plus `TemplateStructurePiece.postProcess`.

use std::sync::Arc;

use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;

use crate::noise::ruined_portal::RuinedPortalMirror;
use crate::noise::structure_pieces::{JigsawBoundingBox, JigsawRotation};
use crate::structure_processor::StructureProcessor;
use crate::structure_template_placement::{
    place_materialized_template_with_palette, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{block_is, catalog_default_state, WorldGenLevel};
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};
use bbb_server_types::block_state_parse::parse_block_state_prefix;
use bbb_server_types::fluid::block_state_is_air;

use super::{
    template_offset, template_pivot, transform_about_pivot, IglooPiece, IglooStructureStart,
    IGLOO_CHEST_LOOT, INITIAL_TEMPLATE_Y, TEMPLATE_TOP,
};

/// `Block.UPDATE_CLIENTS` (`Block.java:92`), the `placeInWorld` update flag.
const UPDATE_CLIENTS: i32 = 2;
/// `Block.UPDATE_NEIGHBORS | UPDATE_CLIENTS` (`IglooPieces.java:113,144`).
const UPDATE_NEIGHBORS_CLIENTS: i32 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IglooChunkPlacement {
    pub attempted: bool,
    pub placed: bool,
}

impl IglooStructureStart {
    /// `IglooPieces.IglooPiece.postProcess` then
    /// `TemplateStructurePiece.postProcess` (`IglooPieces.java:122-148`,
    /// `TemplateStructurePiece.java:82-117`).
    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> IglooChunkPlacement {
        let mut report = IglooChunkPlacement::default();
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
            report.attempted = true;
            if place_piece(
                &mut self.pieces[index],
                template,
                level,
                chunk_bounds,
                random,
            ) {
                report.placed = true;
            }
        }
        report
    }
}

fn place_piece<L: WorldGenLevel>(
    piece: &mut IglooPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let (Some(pivot), Some(offset)) = (
        template_pivot(piece.template()),
        template_offset(piece.template()),
    ) else {
        return false;
    };
    let stored = piece.template_position();
    let entrance = wrapping_add(
        stored,
        transform_about_pivot(
            BlockPos {
                x: 3_i32.wrapping_sub(offset.x),
                y: 0,
                z: 0_i32.wrapping_sub(offset.z),
            },
            piece.rotation(),
            pivot,
        ),
    );
    let height = level.get_world_surface_wg_height(entrance.x, entrance.z);
    let adjusted = BlockPos {
        x: stored.x,
        y: stored
            .y
            .wrapping_add(height.wrapping_sub(INITIAL_TEMPLATE_Y).wrapping_sub(1)),
        z: stored.z,
    };
    piece.set_template_position(adjusted);
    let placed = place_template(piece, pivot, template, level, chunk_bounds, random);
    if piece.template() == TEMPLATE_TOP {
        place_trapdoor_snow(piece, pivot, level, chunk_bounds);
    }
    piece.set_template_position(stored);
    placed
}

/// `IglooPieces.java:140-145`.
fn place_trapdoor_snow<L: WorldGenLevel>(
    piece: &IglooPiece,
    pivot: BlockPos,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
) {
    let trapdoor = wrapping_add(
        piece.template_position(),
        transform_about_pivot(BlockPos { x: 3, y: 0, z: 5 }, piece.rotation(), pivot),
    );
    if !chunk_bounds.contains(trapdoor) {
        return;
    }
    let below = BlockPos {
        x: trapdoor.x,
        y: trapdoor.y.wrapping_sub(1),
        z: trapdoor.z,
    };
    let below_state = level.get_block(below);
    if block_state_is_air(below_state) || block_is(below_state, "minecraft:ladder") {
        return;
    }
    let snow = catalog_default_state("minecraft:snow_block");
    let _ = level.set_block(trapdoor, snow, UPDATE_NEIGHBORS_CLIENTS);
}

/// `TemplateStructurePiece.postProcess` (`TemplateStructurePiece.java:91-117`)
/// with `IglooPieces.makeSettings` (`IglooPieces.java:89-96`).
fn place_template<L: WorldGenLevel>(
    piece: &IglooPiece,
    pivot: BlockPos,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    chunk_bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = make_settings();
    let origin = piece.template_position();
    let Some(palette) = place_materialized_template_with_palette(
        template,
        level,
        MaterializedTemplatePlacement {
            position: origin,
            reference_pos: origin,
            rotation: piece.rotation(),
            mirror: RuinedPortalMirror::None,
            pivot,
            finalize_entities: false,
            ignore_entities: false,
            bounding_box: chunk_bounds,
            processors: &processors,
            liquid_settings: StructureLiquidSettings::IgnoreWaterlogging,
            update_mode: UPDATE_CLIENTS,
            known_shape: true,
        },
        random,
        TemplateSettingsRandom::Positional,
    ) else {
        return false;
    };
    handle_data_markers(piece, pivot, palette, chunk_bounds, level, random);
    replace_jigsaw_blocks(piece, pivot, palette, chunk_bounds, level);
    true
}

fn make_settings() -> Vec<StructureProcessor> {
    vec![StructureProcessor::BlockIgnore {
        blocks: vec![catalog_default_state("minecraft:structure_block")],
    }]
}

/// `IglooPieces.handleDataMarker` (`IglooPieces.java:109-118`).
fn handle_data_markers<L: WorldGenLevel>(
    piece: &IglooPiece,
    pivot: BlockPos,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
    random: &mut dyn RandomSource,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        pivot,
        chunk_bounds,
        "minecraft:structure_block",
    ) {
        let Some(nbt) = nbt.as_deref() else {
            continue;
        };
        if !compound_string(nbt, "mode").is_some_and(|mode| mode.eq_ignore_ascii_case("data")) {
            continue;
        }
        if compound_string(nbt, "metadata").unwrap_or_default() != "chest" {
            continue;
        }
        let air = catalog_default_state("minecraft:air");
        let _ = level.set_block(pos, air, UPDATE_NEIGHBORS_CLIENTS);
        let below = BlockPos {
            x: pos.x,
            y: pos.y.wrapping_sub(1),
            z: pos.z,
        };
        let seed = random.next_long();
        level.set_block_entity_loot_table(below, IGLOO_CHEST_LOOT, seed);
    }
}

fn replace_jigsaw_blocks<L: WorldGenLevel>(
    piece: &IglooPiece,
    pivot: BlockPos,
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    chunk_bounds: JigsawBoundingBox,
    level: &mut L,
) {
    for (pos, nbt) in filter_blocks(
        palette,
        piece.template_position(),
        piece.rotation(),
        pivot,
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
        let _ = level.set_block(pos, state, UPDATE_NEIGHBORS_CLIENTS);
    }
}

/// `StructureTemplate.filterBlocks` (`StructureTemplate.java:220-236`).
fn filter_blocks(
    palette: &bbb_server_datapack::structure_template::StructureTemplatePalette,
    origin: BlockPos,
    rotation: JigsawRotation,
    pivot: BlockPos,
    chunk_bounds: JigsawBoundingBox,
    block_name: &str,
) -> Vec<(BlockPos, Option<Arc<NbtSummaryValue>>)> {
    palette
        .blocks
        .iter()
        .filter(|block| block_is(block.state, block_name))
        .filter_map(|block| {
            let pos = wrapping_add(
                transform_about_pivot(block.position, rotation, pivot),
                origin,
            );
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

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::random::WorldgenRandom;
    use bbb_server_datapack::structure_template::{
        StructureTemplate, StructureTemplateBlock, StructureTemplateElement,
        StructureTemplateEntity, StructureTemplatePalette, StructureTemplateSet,
    };

    use super::super::{IglooGenerationStub, OFFSET_TOP, PIVOT_TOP, TEMPLATE_BOTTOM};
    use super::*;

    struct PlacementLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        air: i32,
        max_y: i32,
        surface: BTreeMap<(i32, i32), i32>,
        writes: Vec<(BlockPos, i32, i32)>,
        loot: Vec<(BlockPos, &'static str, i64)>,
        loot_available: bool,
        template_entities: Vec<(NbtSummaryValue, [f64; 3], [f32; 2])>,
    }

    impl PlacementLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                air: catalog_default_state("minecraft:air"),
                max_y: 319,
                surface: BTreeMap::new(),
                writes: Vec::new(),
                loot: Vec::new(),
                loot_available: true,
                template_entities: Vec::new(),
            }
        }

        fn set_surface(&mut self, x: i32, z: i32, height: i32) {
            self.surface.insert((x, z), height);
        }
    }

    impl WorldGenLevel for PlacementLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
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
            -64
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

        fn get_world_surface_wg_height(&self, x: i32, z: i32) -> i32 {
            self.surface.get(&(x, z)).copied().unwrap_or(self.max_y)
        }

        fn set_block_entity_loot_table(
            &mut self,
            pos: BlockPos,
            loot_table: &'static str,
            seed: i64,
        ) {
            self.loot.push((pos, loot_table, seed));
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

    fn start(template: &str, origin: BlockPos) -> IglooStructureStart {
        IglooStructureStart::from_generation_stub(
            IglooGenerationStub::new(
                [0, 0],
                origin,
                super::super::IGLOO_ID,
                vec![IglooPiece::new(template, origin, JigsawRotation::None)],
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

    fn height_adjusted_origin(stored: BlockPos, surface: i32) -> BlockPos {
        BlockPos {
            x: stored.x,
            y: stored
                .y
                .wrapping_add(surface.wrapping_sub(INITIAL_TEMPLATE_Y).wrapping_sub(1)),
            z: stored.z,
        }
    }

    fn set_top_surface(level: &mut PlacementLevel, origin: BlockPos, height: i32) {
        let entrance = wrapping_add(
            origin,
            transform_about_pivot(
                at(3 - OFFSET_TOP.x, 0, -OFFSET_TOP.z),
                JigsawRotation::None,
                PIVOT_TOP,
            ),
        );
        level.set_surface(entrance.x, entrance.z, height);
    }

    fn nbt_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn villager_nbt() -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            nbt_entry("id", NbtSummaryValue::string("minecraft:villager")),
            nbt_entry("Health", NbtSummaryValue::Float(20.0_f32.to_bits())),
            nbt_entry("PersistenceRequired", NbtSummaryValue::Byte(1)),
            nbt_entry("CanPickUpLoot", NbtSummaryValue::Byte(1)),
            nbt_entry("Age", NbtSummaryValue::Int(0)),
            nbt_entry("ForcedAge", NbtSummaryValue::Int(0)),
            nbt_entry("Xp", NbtSummaryValue::Int(0)),
            nbt_entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("type", NbtSummaryValue::string("minecraft:plains")),
                    nbt_entry("profession", NbtSummaryValue::string("minecraft:cleric")),
                    nbt_entry("level", NbtSummaryValue::Int(1)),
                ]),
            ),
            nbt_entry(
                "Offers",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "Recipes",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        nbt_entry("maxUses", NbtSummaryValue::Int(7)),
                        nbt_entry("uses", NbtSummaryValue::Int(0)),
                        nbt_entry("rewardExp", NbtSummaryValue::Byte(1)),
                        nbt_entry(
                            "buy",
                            NbtSummaryValue::Compound(vec![
                                nbt_entry("id", NbtSummaryValue::string("minecraft:rotten_flesh")),
                                nbt_entry("count", NbtSummaryValue::Int(36)),
                            ]),
                        ),
                        nbt_entry(
                            "sell",
                            NbtSummaryValue::Compound(vec![
                                nbt_entry("id", NbtSummaryValue::string("minecraft:emerald")),
                                nbt_entry("count", NbtSummaryValue::Int(1)),
                            ]),
                        ),
                    ])]),
                )]),
            ),
        ])
    }

    fn place(
        start: &mut IglooStructureStart,
        templates: &StructureTemplateSet,
        level: &mut PlacementLevel,
        random: &mut dyn RandomSource,
    ) -> IglooChunkPlacement {
        start.place_in_chunk(templates, level, wide_bounds(), random)
    }

    #[test]
    fn surface_height_shifts_then_restores_the_stored_origin() {
        let id = TEMPLATE_TOP;
        let snow = state("minecraft:snow_block");
        let templates = templates(id, [1, 1, 1], vec![block(0, 0, 0, snow)]);
        let origin = at(0, INITIAL_TEMPLATE_Y, 0);
        let mut level = PlacementLevel::new();
        let entrance = wrapping_add(
            origin,
            transform_about_pivot(
                at(3 - OFFSET_TOP.x, 0, -OFFSET_TOP.z),
                JigsawRotation::None,
                PIVOT_TOP,
            ),
        );
        level.set_surface(entrance.x, entrance.z, 64);
        let mut start = start(id, origin);
        let report = place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        assert_eq!(start.pieces()[0].template_position(), origin);
        let placed_y = origin.y.wrapping_add(64 - INITIAL_TEMPLATE_Y - 1);
        assert_eq!(level.get_block(at(origin.x, placed_y, origin.z)), snow);
    }

    #[test]
    fn top_piece_writes_snow_when_the_trapdoor_floor_is_solid() {
        let id = TEMPLATE_TOP;
        let stone = state("minecraft:stone");
        let snow = state("minecraft:snow_block");
        let templates = templates(id, [1, 1, 1], vec![block(0, 0, 0, stone)]);
        let origin = at(0, INITIAL_TEMPLATE_Y, 0);
        let mut level = PlacementLevel::new();
        let entrance = wrapping_add(
            origin,
            transform_about_pivot(
                at(3 - OFFSET_TOP.x, 0, -OFFSET_TOP.z),
                JigsawRotation::None,
                PIVOT_TOP,
            ),
        );
        level.set_surface(entrance.x, entrance.z, 64);
        let adjusted_y = origin.y.wrapping_add(64 - INITIAL_TEMPLATE_Y - 1);
        let trapdoor = wrapping_add(
            at(origin.x, adjusted_y, origin.z),
            transform_about_pivot(at(3, 0, 5), JigsawRotation::None, PIVOT_TOP),
        );
        level
            .cells
            .insert((trapdoor.x, trapdoor.y - 1, trapdoor.z), stone);
        let mut start = start(id, origin);
        place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert_eq!(level.get_block(trapdoor), snow);
        assert!(level.writes.iter().any(|(pos, state, flags)| {
            *pos == trapdoor && *state == snow && *flags == UPDATE_NEIGHBORS_CLIENTS
        }));
    }

    #[test]
    fn chest_marker_clears_the_marker_and_loots_the_block_below() {
        let id = TEMPLATE_BOTTOM;
        let stone = state("minecraft:stone");
        let structure = state("minecraft:structure_block");
        let templates = templates(
            id,
            [1, 2, 1],
            vec![
                block(0, 0, 0, stone),
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
        let origin = at(4, INITIAL_TEMPLATE_Y, 8);
        let mut level = PlacementLevel::new();
        let offset = template_offset(id).unwrap();
        let pivot = template_pivot(id).unwrap();
        let entrance = wrapping_add(
            origin,
            transform_about_pivot(at(3 - offset.x, 0, -offset.z), JigsawRotation::None, pivot),
        );
        level.set_surface(entrance.x, entrance.z, 64);
        let mut start = start(id, origin);
        let mut random = WorldgenRandom::with_seed(11);
        place(&mut start, &templates, &mut level, &mut random);
        let mut expected = WorldgenRandom::with_seed(11);
        let seed = expected.next_long();
        let adjusted_y = origin.y.wrapping_add(64 - INITIAL_TEMPLATE_Y - 1);
        let marker = wrapping_add(
            at(origin.x, adjusted_y, origin.z),
            transform_about_pivot(at(0, 1, 0), JigsawRotation::None, pivot),
        );
        let below = at(marker.x, marker.y - 1, marker.z);
        assert_eq!(level.get_block(marker), level.air);
        assert!(level.writes.iter().any(|(pos, state, flags)| {
            *pos == marker && *state == level.air && *flags == UPDATE_NEIGHBORS_CLIENTS
        }));
        assert_eq!(level.loot, vec![(below, IGLOO_CHEST_LOOT, seed)]);
    }

    #[test]
    fn places_owned_template_villager_after_height_adjust() {
        let snow = state("minecraft:snow_block");
        let nbt = villager_nbt();
        let local = [2.5, 1.0, 1.5];
        let templates = templates_with_entities(
            TEMPLATE_TOP,
            [1, 1, 1],
            vec![block(0, 0, 0, snow)],
            vec![StructureTemplateEntity {
                position_bits: local.map(f64::to_bits),
                block_position: at(2, 1, 1),
                nbt: nbt.clone(),
            }],
        );
        let origin = at(0, INITIAL_TEMPLATE_Y, 0);
        let mut level = PlacementLevel::new();
        set_top_surface(&mut level, origin, 64);
        let mut start = start(TEMPLATE_TOP, origin);
        let report = place(
            &mut start,
            &templates,
            &mut level,
            &mut WorldgenRandom::with_seed(0),
        );
        assert!(report.attempted && report.placed);
        let placed = height_adjusted_origin(origin, 64);
        assert_eq!(level.get_block(placed), snow);
        assert_eq!(level.template_entities.len(), 1);
        assert_eq!(level.template_entities[0].0, nbt);
        assert_eq!(
            level.template_entities[0].1,
            [
                local[0] + f64::from(placed.x),
                local[1] + f64::from(placed.y),
                local[2] + f64::from(placed.z),
            ]
        );
    }

    #[test]
    fn builtin_igloo_template_writes_inside_the_chunk_bb() {
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
            .find(|template| template.id == TEMPLATE_TOP)
        else {
            return;
        };
        if !template.template.materialized().entities.is_empty() {
            return;
        }
        let origin = at(0, INITIAL_TEMPLATE_Y, 0);
        let mut start = start(&template.id, origin);
        let mut level = PlacementLevel::new();
        level.set_surface(3, 0, 64);
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
        assert_eq!(start.pieces()[0].template_position(), origin);
    }
}
