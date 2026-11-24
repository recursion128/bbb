//! Built-in Nether fossil start, template placement, and save codec.

use crate::random::large_feature_random;
use crate::random::WorldgenRandom;
use bbb_protocol::block_state_types::vanilla_block_state_rotate_y;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::structure_pool::StructureLiquidSettings;
use bbb_server_datapack::structure_template::{
    MaterializedStructureTemplate, StructureTemplateSet,
};

use super::ruined_portal::RuinedPortalMirror;
use super::structure_nbt::{
    bounding_box_to_nbt, compound, entry, int, rotation, rotation_name, start_to_nbt, string,
    NbtFields, NbtRead,
};
use super::structure_pieces::{CachedStartBoundingBox, JigsawBoundingBox, JigsawRotation};
use super::structures::{NoiseStructurePlan, StructureStartSetOwner, StructureTerrainAdjustment};
use crate::height_provider::sample_height_provider_with_bounds;
use crate::structure_processor::StructureProcessor;
use crate::structure_template_placement::{
    place_materialized_template, MaterializedTemplatePlacement, TemplateSettingsRandom,
};
use crate::{catalog_default_state, WorldGenLevel};

pub const NETHER_FOSSIL_ID: &str = "minecraft:nether_fossil";
pub const NETHER_FOSSIL_TYPE_ID: &str = "minecraft:nether_fossil";
pub const NETHER_FOSSILS_SET_ID: &str = "minecraft:nether_fossils";
pub const NETHER_FOSSIL_PIECE_ID: &str = "minecraft:nefos";

pub const FOSSIL_TEMPLATES: [&str; 14] = [
    "minecraft:nether_fossils/fossil_1",
    "minecraft:nether_fossils/fossil_2",
    "minecraft:nether_fossils/fossil_3",
    "minecraft:nether_fossils/fossil_4",
    "minecraft:nether_fossils/fossil_5",
    "minecraft:nether_fossils/fossil_6",
    "minecraft:nether_fossils/fossil_7",
    "minecraft:nether_fossils/fossil_8",
    "minecraft:nether_fossils/fossil_9",
    "minecraft:nether_fossils/fossil_10",
    "minecraft:nether_fossils/fossil_11",
    "minecraft:nether_fossils/fossil_12",
    "minecraft:nether_fossils/fossil_13",
    "minecraft:nether_fossils/fossil_14",
];

const ROTATIONS: [JigsawRotation; 4] = [
    JigsawRotation::None,
    JigsawRotation::Clockwise90,
    JigsawRotation::Clockwise180,
    JigsawRotation::CounterClockwise90,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetherFossilPiece {
    template: String,
    position: BlockPos,
    rotation: JigsawRotation,
    template_size: [i32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetherFossilStructureStart {
    structure_id: String,
    start_chunk: [i32; 2],
    references: i32,
    terrain_adjustment: StructureTerrainAdjustment,
    generation_anchor: BlockPos,
    piece: NetherFossilPiece,
    cached_bounding_box: CachedStartBoundingBox,
}

impl NetherFossilPiece {
    pub fn new(
        template: impl Into<String>,
        position: BlockPos,
        rotation: JigsawRotation,
        template_size: [i32; 3],
    ) -> Self {
        Self {
            template: template.into(),
            position,
            rotation,
            template_size,
        }
    }

    pub fn template(&self) -> &str {
        &self.template
    }

    pub const fn position(&self) -> BlockPos {
        self.position
    }

    pub const fn rotation(&self) -> JigsawRotation {
        self.rotation
    }

    pub const fn template_size(&self) -> [i32; 3] {
        self.template_size
    }

    pub fn set_template_size(&mut self, size: [i32; 3]) {
        self.template_size = size;
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        template_box(self.position, self.rotation, self.template_size)
    }
}

impl NetherFossilStructureStart {
    pub fn structure_id(&self) -> &str {
        &self.structure_id
    }

    pub const fn start_chunk(&self) -> [i32; 2] {
        self.start_chunk
    }

    pub const fn references(&self) -> i32 {
        self.references
    }

    pub(super) fn try_add_reference(&mut self) -> bool {
        if self.references >= 1 {
            return false;
        }
        self.references = self.references.wrapping_add(1);
        true
    }

    pub const fn terrain_adjustment(&self) -> StructureTerrainAdjustment {
        self.terrain_adjustment
    }

    pub const fn generation_anchor(&self) -> BlockPos {
        self.generation_anchor
    }

    pub fn piece(&self) -> &NetherFossilPiece {
        &self.piece
    }

    pub fn piece_mut(&mut self) -> &mut NetherFossilPiece {
        self.cached_bounding_box.invalidate();
        &mut self.piece
    }

    pub fn bounding_box(&self) -> JigsawBoundingBox {
        self.cached_bounding_box
            .get_or_init(|| {
                Some(
                    self.piece
                        .bounding_box()
                        .adjusted_for_terrain(self.terrain_adjustment),
                )
            })
            .expect("nether fossil start owns one piece")
    }

    pub fn intersects_chunk(&self, x: i32, z: i32) -> bool {
        self.bounding_box().intersects_chunk(x, z)
    }

    pub fn from_parts(
        start_chunk: [i32; 2],
        references: i32,
        terrain_adjustment: StructureTerrainAdjustment,
        piece: NetherFossilPiece,
    ) -> Self {
        Self::from_parts_for(
            NETHER_FOSSIL_ID,
            start_chunk,
            references,
            terrain_adjustment,
            piece,
        )
    }

    pub(crate) fn from_parts_for(
        structure_id: &str,
        start_chunk: [i32; 2],
        references: i32,
        terrain_adjustment: StructureTerrainAdjustment,
        piece: NetherFossilPiece,
    ) -> Self {
        Self {
            structure_id: structure_id.to_string(),
            start_chunk,
            references,
            terrain_adjustment,
            generation_anchor: piece.position,
            piece,
            cached_bounding_box: CachedStartBoundingBox::new(),
        }
    }

    pub fn place_in_chunk<L: WorldGenLevel>(
        &mut self,
        templates: &StructureTemplateSet,
        level: &mut L,
        chunk_bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    ) -> bool {
        let live_template = level.structure_template(self.piece.template());
        let Some(template) = live_template.as_ref().or_else(|| {
            templates
                .template(self.piece.template())
                .map(|element| &element.template)
        }) else {
            return false;
        };
        let template = template.materialized();
        let fossil_bounds = template_box(self.piece.position, self.piece.rotation, template.size);
        let placement_bounds = union(chunk_bounds, fossil_bounds);
        let placed = place_template(&self.piece, template, level, placement_bounds, random);
        let center = BlockPos {
            x: fossil_bounds.min.x + (fossil_bounds.max.x - fossil_bounds.min.x + 1) / 2,
            y: fossil_bounds.min.y + (fossil_bounds.max.y - fossil_bounds.min.y + 1) / 2,
            z: fossil_bounds.min.z + (fossil_bounds.max.z - fossil_bounds.min.z + 1) / 2,
        };
        let world_seed = level.get_seed();
        // `NetherFossilPieces.NetherFossilPiece.postProcess` expands the write
        // box and uses a fossil-center positional RNG (`NetherFossilPieces.java:89-109`).
        let mut positional_random = WorldgenRandom::at(world_seed, center.x, center.y, center.z);
        if positional_random.next_float() < 0.5 {
            let x = fossil_bounds.min.x.wrapping_add(
                positional_random.next_int(
                    fossil_bounds
                        .max
                        .x
                        .wrapping_sub(fossil_bounds.min.x)
                        .wrapping_add(1),
                ),
            );
            let z = fossil_bounds.min.z.wrapping_add(
                positional_random.next_int(
                    fossil_bounds
                        .max
                        .z
                        .wrapping_sub(fossil_bounds.min.z)
                        .wrapping_add(1),
                ),
            );
            let pos = BlockPos {
                x,
                y: fossil_bounds.min.y,
                z,
            };
            if placement_bounds.contains(pos)
                && bbb_server_types::fluid::block_state_is_air(level.get_block(pos))
            {
                let dried = catalog_default_state("minecraft:dried_ghast");
                let turns = positional_random.next_int(4) as u8;
                let dried = vanilla_block_state_rotate_y(dried, turns).unwrap_or(dried);
                level.set_block(pos, dried, 2);
            }
        }
        placed
    }

    pub fn to_nbt(&self) -> NbtSummaryValue {
        let piece = &self.piece;
        start_to_nbt(
            self.structure_id(),
            self.start_chunk,
            self.references,
            vec![compound(vec![
                string("id", NETHER_FOSSIL_PIECE_ID),
                entry("BB", bounding_box_to_nbt(piece.bounding_box())),
                int("O", 2),
                int("GD", 0),
                int("TPX", piece.position.x),
                int("TPY", piece.position.y),
                int("TPZ", piece.position.z),
                string("Template", &piece.template),
                string("Rot", rotation_name(piece.rotation)),
            ])],
        )
    }

    pub fn from_nbt(
        outer_id: &str,
        value: &NbtSummaryValue,
        plan: &NoiseStructurePlan,
    ) -> Option<Self> {
        let fields = value.compound()?;
        let [child] = fields.field("Children")?.list()? else {
            return None;
        };
        let child = child.compound()?;
        let piece = NetherFossilPiece {
            template: child.field("Template")?.resource_id()?,
            position: BlockPos {
                x: child.field("TPX")?.int()?,
                y: child.field("TPY")?.int()?,
                z: child.field("TPZ")?.int()?,
            },
            rotation: rotation(child.field("Rot")?)?,
            template_size: [0, 0, 0],
        };
        Some(Self::from_parts_for(
            outer_id,
            [
                fields.field("ChunkX")?.int()?,
                fields.field("ChunkZ")?.int()?,
            ],
            fields.field("references")?.int()?,
            plan.structures().get(outer_id)?.terrain_adjustment,
            piece,
        ))
    }
}

impl NoiseStructurePlan {
    /// `NetherFossilStructure.findGenerationPoint` samples a column then scans
    /// for air over soul sand or a sturdy top (`NetherFossilStructure.java:30-56`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_missing_nether_fossil_starts(
        &self,
        seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: i32,
        sea_level: i32,
        existing: &[NetherFossilStructureStart],
        mut block_at: impl FnMut(i32, i32, i32) -> Option<i32>,
        mut sturdy_up: impl FnMut(i32) -> Option<bool>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Vec<NetherFossilStructureStart> {
        let mut generated = Vec::new();
        for set in self.sets() {
            if self.structure_start_owner(set) != StructureStartSetOwner::NetherFossil
                || existing.iter().any(|start| {
                    set.structures
                        .iter()
                        .any(|selection| selection.structure == start.structure_id())
                })
            {
                continue;
            }
            let Some(attempts) = self.random_spread_attempt_order(&set.id, seed, chunk_x, chunk_z)
            else {
                continue;
            };
            for id in attempts {
                let Some(start) = self.nether_fossil_generation_start(
                    &id,
                    seed,
                    chunk_x,
                    chunk_z,
                    min_y,
                    height,
                    sea_level,
                    &mut block_at,
                    &mut sturdy_up,
                    &mut biome_at,
                    &mut template_size,
                    true,
                ) else {
                    continue;
                };
                generated.push(start);
                break;
            }
        }
        generated
    }

    /// `PlaceCommand.placeStructure` (`PlaceCommand.java:277-294`).
    #[allow(clippy::too_many_arguments)]
    pub fn generate_forced_nether_fossil_start(
        &self,
        structure_id: &str,
        seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: i32,
        sea_level: i32,
        mut block_at: impl FnMut(i32, i32, i32) -> Option<i32>,
        mut sturdy_up: impl FnMut(i32) -> Option<bool>,
        mut biome_at: impl FnMut(BlockPos) -> Option<String>,
        mut template_size: impl FnMut(&str) -> Option<[i32; 3]>,
    ) -> Option<NetherFossilStructureStart> {
        self.nether_fossil_generation_start(
            structure_id,
            seed,
            chunk_x,
            chunk_z,
            min_y,
            height,
            sea_level,
            &mut block_at,
            &mut sturdy_up,
            &mut biome_at,
            &mut template_size,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn nether_fossil_generation_start(
        &self,
        id: &str,
        seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        min_y: i32,
        height: i32,
        sea_level: i32,
        block_at: &mut impl FnMut(i32, i32, i32) -> Option<i32>,
        sturdy_up: &mut impl FnMut(i32) -> Option<bool>,
        biome_at: &mut impl FnMut(BlockPos) -> Option<String>,
        template_size: &mut impl FnMut(&str) -> Option<[i32; 3]>,
        require_allowed_biome: bool,
    ) -> Option<NetherFossilStructureStart> {
        let definition = self.structures().get(id)?;
        let provider = definition.nether_fossil_height.as_ref()?;
        let mut random = large_feature_random(seed, chunk_x, chunk_z);
        let x = chunk_x * 16 + random.next_int(16);
        let z = chunk_z * 16 + random.next_int(16);
        let mut y =
            sample_height_provider_with_bounds(provider, min_y, height, &mut random).ok()?;
        let soul_sand = catalog_default_state("minecraft:soul_sand");
        while y > sea_level {
            let current = block_at(x, y, z)?;
            y -= 1;
            let below = block_at(x, y, z)?;
            if bbb_server_types::fluid::block_state_is_air(current)
                && (below == soul_sand || sturdy_up(below)?)
            {
                break;
            }
        }
        if y <= sea_level {
            return None;
        }
        let anchor = BlockPos { x, y, z };
        let biome = bbb_protocol::normalize_resource_location(biome_at(anchor)?).ok()?;
        if require_allowed_biome && !definition.biomes.contains(&biome) {
            return None;
        }
        let rotation = ROTATIONS[random.next_int(4) as usize];
        let template =
            FOSSIL_TEMPLATES[random.next_int(FOSSIL_TEMPLATES.len() as i32) as usize].to_string();
        let piece = NetherFossilPiece {
            template_size: template_size(&template)?,
            template,
            position: anchor,
            rotation,
        };
        Some(NetherFossilStructureStart::from_parts_for(
            id,
            [chunk_x, chunk_z],
            0,
            definition.terrain_adjustment,
            piece,
        ))
    }
}

fn place_template<L: WorldGenLevel>(
    piece: &NetherFossilPiece,
    template: &MaterializedStructureTemplate,
    level: &mut L,
    bounds: JigsawBoundingBox,
    random: &mut dyn RandomSource,
) -> bool {
    let processors = [StructureProcessor::BlockIgnore {
        blocks: vec![
            catalog_default_state("minecraft:air"),
            catalog_default_state("minecraft:structure_block"),
        ],
    }];
    place_materialized_template(
        template,
        level,
        MaterializedTemplatePlacement {
            position: piece.position,
            reference_pos: piece.position,
            rotation: piece.rotation,
            mirror: RuinedPortalMirror::None,
            pivot: BlockPos { x: 0, y: 0, z: 0 },
            finalize_entities: false,
            ignore_entities: false,
            bounding_box: bounds,
            processors: &processors,
            liquid_settings: StructureLiquidSettings::ApplyWaterlogging,
            update_mode: 2,
            known_shape: true,
        },
        random,
        TemplateSettingsRandom::Positional,
    )
}

fn template_box(origin: BlockPos, rotation: JigsawRotation, size: [i32; 3]) -> JigsawBoundingBox {
    if size.iter().any(|value| *value <= 0) {
        return JigsawBoundingBox {
            min: origin,
            max: origin,
        };
    }
    let first = transform(BlockPos { x: 0, y: 0, z: 0 }, rotation);
    let last = transform(
        BlockPos {
            x: size[0] - 1,
            y: size[1] - 1,
            z: size[2] - 1,
        },
        rotation,
    );
    JigsawBoundingBox {
        min: BlockPos {
            x: origin.x + first.x.min(last.x),
            y: origin.y,
            z: origin.z + first.z.min(last.z),
        },
        max: BlockPos {
            x: origin.x + first.x.max(last.x),
            y: origin.y + size[1] - 1,
            z: origin.z + first.z.max(last.z),
        },
    }
}

fn transform(position: BlockPos, rotation: JigsawRotation) -> BlockPos {
    match rotation {
        JigsawRotation::None => position,
        JigsawRotation::Clockwise90 => BlockPos {
            x: -position.z,
            y: position.y,
            z: position.x,
        },
        JigsawRotation::Clockwise180 => BlockPos {
            x: -position.x,
            y: position.y,
            z: -position.z,
        },
        JigsawRotation::CounterClockwise90 => BlockPos {
            x: position.z,
            y: position.y,
            z: -position.x,
        },
    }
}

fn union(left: JigsawBoundingBox, right: JigsawBoundingBox) -> JigsawBoundingBox {
    JigsawBoundingBox {
        min: BlockPos {
            x: left.min.x.min(right.min.x),
            y: left.min.y.min(right.min.y),
            z: left.min.z.min(right.min.z),
        },
        max: BlockPos {
            x: left.max.x.max(right.max.x),
            y: left.max.y.max(right.max.y),
            z: left.max.z.max(right.max.z),
        },
    }
}
