//! World -> renderer projection for banner block-entity models.
//!
//! Vanilla renders banners through the `BlockEntityRenderDispatcher` +
//! `BannerRenderer` pair: per banner block entity, a `BannerRenderState`
//! carrying the block's base `DyeColor`, the BE's `BannerPatternLayers`, the
//! ground/wall transformation, the flag swing phase, and the light coords
//! sampled at the block position. bbb has no separate BER dispatch; banner
//! instances ride the existing single entity-model submission stream as
//! `EntityModelKind::Banner`, like the chest and the pot. The pattern
//! registry-id -> sprite mapping happens here, mirroring
//! `Sheets.getBannerSprite` over `BannerPatterns.bootstrap`.

use bbb_renderer::{BannerPatternKind, BannerPatternLayer, EntityDyeColor, EntityModelInstance};
use bbb_world::{
    BannerBlockForm as WorldBannerBlockForm, BannerDyeColorKind as WorldBannerDyeColorKind,
    BannerModelSourceState, BannerPatternLayerState, TerrainLight, WorldStore,
};

/// Like chests/bells/pots, banner instances are projected from block states,
/// not the entity list, so they carry a sentinel id no server entity can use.
const BANNER_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every banner block in the loaded chunks into a banner model
/// instance: position at the block min corner, `-angle` yaw
/// (`BannerRenderer.modelTransformation`'s `Axis.YP.rotationDegrees(-angle)`),
/// the base color, the mapped pattern layers, the flag swing phase, and the
/// block-position light.
pub(crate) fn banner_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .banner_model_source_states(partial_tick)
        .into_iter()
        .map(|source| banner_model_instance(&source, world))
        .collect()
}

fn banner_model_instance(
    source: &BannerModelSourceState,
    world: &WorldStore,
) -> EntityModelInstance {
    let mut instance = EntityModelInstance::banner(
        BANNER_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        -source.angle_degrees,
        source.form == WorldBannerBlockForm::Wall,
        banner_dye_color(source.base_color),
        banner_pattern_layers(&source.layers),
    )
    .with_banner_flag_phase(source.phase);
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(banner_light_coords(light));
    }
    instance
}

/// The block-id base color (`AbstractBannerBlock.getColor`) as the
/// renderer's dye.
fn banner_dye_color(color: WorldBannerDyeColorKind) -> EntityDyeColor {
    match color {
        WorldBannerDyeColorKind::White => EntityDyeColor::White,
        WorldBannerDyeColorKind::Orange => EntityDyeColor::Orange,
        WorldBannerDyeColorKind::Magenta => EntityDyeColor::Magenta,
        WorldBannerDyeColorKind::LightBlue => EntityDyeColor::LightBlue,
        WorldBannerDyeColorKind::Yellow => EntityDyeColor::Yellow,
        WorldBannerDyeColorKind::Lime => EntityDyeColor::Lime,
        WorldBannerDyeColorKind::Pink => EntityDyeColor::Pink,
        WorldBannerDyeColorKind::Gray => EntityDyeColor::Gray,
        WorldBannerDyeColorKind::LightGray => EntityDyeColor::LightGray,
        WorldBannerDyeColorKind::Cyan => EntityDyeColor::Cyan,
        WorldBannerDyeColorKind::Purple => EntityDyeColor::Purple,
        WorldBannerDyeColorKind::Blue => EntityDyeColor::Blue,
        WorldBannerDyeColorKind::Brown => EntityDyeColor::Brown,
        WorldBannerDyeColorKind::Green => EntityDyeColor::Green,
        WorldBannerDyeColorKind::Red => EntityDyeColor::Red,
        WorldBannerDyeColorKind::Black => EntityDyeColor::Black,
    }
}

/// Maps the stored raw pattern layers onto the renderer's fixed-size layer
/// array (the vanilla `BannerRenderer.MAX_PATTERNS = 16` render cap). Any
/// unknown pattern id or dye name folds the whole stack to empty, like the
/// vanilla `BannerPatternLayers.CODEC` failing the registry/dye lookup and
/// `loadAdditional` falling back to `BannerPatternLayers.EMPTY` (a
/// data-driven pattern bbb has no texture for lands in that fold too).
fn banner_pattern_layers(layers: &[BannerPatternLayerState]) -> [Option<BannerPatternLayer>; 16] {
    let mut out = [None; 16];
    for (index, layer) in layers.iter().enumerate() {
        let Some(pattern) = banner_pattern_kind(&layer.pattern) else {
            return [None; 16];
        };
        let Some(color) = banner_layer_dye_color(&layer.color) else {
            return [None; 16];
        };
        if index < out.len() {
            out[index] = Some(BannerPatternLayer { pattern, color });
        }
    }
    out
}

/// The `DyeColor.CODEC` name mapping, strict: an unknown name fails the
/// vanilla codec (no black fallback here, unlike the sign text color).
fn banner_layer_dye_color(name: &str) -> Option<EntityDyeColor> {
    match name {
        "white" => Some(EntityDyeColor::White),
        "orange" => Some(EntityDyeColor::Orange),
        "magenta" => Some(EntityDyeColor::Magenta),
        "light_blue" => Some(EntityDyeColor::LightBlue),
        "yellow" => Some(EntityDyeColor::Yellow),
        "lime" => Some(EntityDyeColor::Lime),
        "pink" => Some(EntityDyeColor::Pink),
        "gray" => Some(EntityDyeColor::Gray),
        "light_gray" => Some(EntityDyeColor::LightGray),
        "cyan" => Some(EntityDyeColor::Cyan),
        "purple" => Some(EntityDyeColor::Purple),
        "blue" => Some(EntityDyeColor::Blue),
        "brown" => Some(EntityDyeColor::Brown),
        "green" => Some(EntityDyeColor::Green),
        "red" => Some(EntityDyeColor::Red),
        "black" => Some(EntityDyeColor::Black),
        _ => None,
    }
}

/// Vanilla `BannerPatterns.bootstrap` (`BannerPatterns.java:60-105`),
/// transcribed as a constant table: the forty-three registered pattern ids
/// (every vanilla entry's `asset_id` equals its registry id,
/// `BannerPatterns.register`). An unknown id returns `None` (vanilla's
/// registry holder lookup fails the whole `patterns` codec).
fn banner_pattern_kind(pattern_id: &str) -> Option<BannerPatternKind> {
    match pattern_id {
        "minecraft:base" => Some(BannerPatternKind::Base),
        "minecraft:square_bottom_left" => Some(BannerPatternKind::SquareBottomLeft),
        "minecraft:square_bottom_right" => Some(BannerPatternKind::SquareBottomRight),
        "minecraft:square_top_left" => Some(BannerPatternKind::SquareTopLeft),
        "minecraft:square_top_right" => Some(BannerPatternKind::SquareTopRight),
        "minecraft:stripe_bottom" => Some(BannerPatternKind::StripeBottom),
        "minecraft:stripe_top" => Some(BannerPatternKind::StripeTop),
        "minecraft:stripe_left" => Some(BannerPatternKind::StripeLeft),
        "minecraft:stripe_right" => Some(BannerPatternKind::StripeRight),
        "minecraft:stripe_center" => Some(BannerPatternKind::StripeCenter),
        "minecraft:stripe_middle" => Some(BannerPatternKind::StripeMiddle),
        "minecraft:stripe_downright" => Some(BannerPatternKind::StripeDownright),
        "minecraft:stripe_downleft" => Some(BannerPatternKind::StripeDownleft),
        "minecraft:small_stripes" => Some(BannerPatternKind::SmallStripes),
        "minecraft:cross" => Some(BannerPatternKind::Cross),
        "minecraft:straight_cross" => Some(BannerPatternKind::StraightCross),
        "minecraft:triangle_bottom" => Some(BannerPatternKind::TriangleBottom),
        "minecraft:triangle_top" => Some(BannerPatternKind::TriangleTop),
        "minecraft:triangles_bottom" => Some(BannerPatternKind::TrianglesBottom),
        "minecraft:triangles_top" => Some(BannerPatternKind::TrianglesTop),
        "minecraft:diagonal_left" => Some(BannerPatternKind::DiagonalLeft),
        "minecraft:diagonal_up_right" => Some(BannerPatternKind::DiagonalUpRight),
        "minecraft:diagonal_up_left" => Some(BannerPatternKind::DiagonalUpLeft),
        "minecraft:diagonal_right" => Some(BannerPatternKind::DiagonalRight),
        "minecraft:circle" => Some(BannerPatternKind::Circle),
        "minecraft:rhombus" => Some(BannerPatternKind::Rhombus),
        "minecraft:half_vertical" => Some(BannerPatternKind::HalfVertical),
        "minecraft:half_horizontal" => Some(BannerPatternKind::HalfHorizontal),
        "minecraft:half_vertical_right" => Some(BannerPatternKind::HalfVerticalRight),
        "minecraft:half_horizontal_bottom" => Some(BannerPatternKind::HalfHorizontalBottom),
        "minecraft:border" => Some(BannerPatternKind::Border),
        "minecraft:curly_border" => Some(BannerPatternKind::CurlyBorder),
        "minecraft:gradient" => Some(BannerPatternKind::Gradient),
        "minecraft:gradient_up" => Some(BannerPatternKind::GradientUp),
        "minecraft:bricks" => Some(BannerPatternKind::Bricks),
        "minecraft:globe" => Some(BannerPatternKind::Globe),
        "minecraft:creeper" => Some(BannerPatternKind::Creeper),
        "minecraft:skull" => Some(BannerPatternKind::Skull),
        "minecraft:flower" => Some(BannerPatternKind::Flower),
        "minecraft:mojang" => Some(BannerPatternKind::Mojang),
        "minecraft:piglin" => Some(BannerPatternKind::Piglin),
        "minecraft:flow" => Some(BannerPatternKind::Flow),
        "minecraft:guster" => Some(BannerPatternKind::Guster),
        _ => None,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the banner render state's `lightCoords`.
fn banner_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_state_id: state_id,
            })
        );
    }

    fn banner_nbt(layers: &[(&str, &str)]) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(9);
        write_mutf8(&mut payload, "patterns");
        payload.push(10);
        payload.extend_from_slice(&(layers.len() as i32).to_be_bytes());
        for (pattern, color) in layers {
            payload.push(8);
            write_mutf8(&mut payload, "pattern");
            write_mutf8(&mut payload, pattern);
            payload.push(8);
            write_mutf8(&mut payload, "color");
            write_mutf8(&mut payload, color);
            payload.push(0);
        }
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn apply_banner_patterns(world: &mut WorldStore, pos: BlockPos, nbt: Vec<u8>) {
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id: 0,
                raw_nbt: nbt,
            })
            .unwrap());
    }

    #[test]
    fn projects_banner_instances_with_color_patterns_yaw_phase_and_light() {
        let mut world = world_with_air_chunk();
        let standing_pos = BlockPos { x: 3, y: 4, z: 5 };
        let wall_pos = BlockPos { x: 6, y: 4, z: 5 };
        // ROTATION 4 = east: convertToDegrees(4) = 90 -> body_rot -90.
        set_block(
            &mut world,
            standing_pos,
            "minecraft:lime_banner",
            &[("rotation", "4")],
        );
        set_block(
            &mut world,
            wall_pos,
            "minecraft:red_wall_banner",
            &[("facing", "north")],
        );
        apply_banner_patterns(
            &mut world,
            standing_pos,
            banner_nbt(&[
                ("minecraft:stripe_top", "purple"),
                ("minecraft:creeper", "white"),
            ]),
        );

        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.5);
        assert_eq!(instances.len(), 2);
        let standing = &instances[0];
        let wall = &instances[1];
        let mut layers = [None; 16];
        layers[0] = Some(BannerPatternLayer {
            pattern: BannerPatternKind::StripeTop,
            color: EntityDyeColor::Purple,
        });
        layers[1] = Some(BannerPatternLayer {
            pattern: BannerPatternKind::Creeper,
            color: EntityDyeColor::White,
        });
        assert_eq!(
            standing.kind,
            EntityModelKind::Banner {
                wall: false,
                base_color: EntityDyeColor::Lime,
                layers,
            }
        );
        assert_eq!(standing.entity_id, BANNER_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(standing.position, [3.0, 4.0, 5.0]);
        assert_eq!(standing.render_state.body_rot, -90.0);
        // phase = (floorMod(3*7 + 4*9 + 5*13 + 0, 100) + 0.5) / 100 = (22 + 0.5) / 100.
        assert!((standing.render_state.banner_flag_phase - 0.225).abs() < 1e-6);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(standing.render_state.light_coords, 15 << 20);
        // A banner without a BE record renders the plain base.
        assert_eq!(
            wall.kind,
            EntityModelKind::Banner {
                wall: true,
                base_color: EntityDyeColor::Red,
                layers: [None; 16],
            }
        );
        // NORTH.toYRot() = 180 -> body_rot -180.
        assert_eq!(wall.render_state.body_rot, -180.0);
    }

    #[test]
    fn unknown_pattern_or_color_folds_the_stack_like_the_vanilla_codec() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:white_banner",
            &[("rotation", "0")],
        );
        apply_banner_patterns(
            &mut world,
            pos,
            banner_nbt(&[
                ("minecraft:creeper", "black"),
                ("minecraft:not_a_pattern", "red"),
            ]),
        );
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Banner {
                wall: false,
                base_color: EntityDyeColor::White,
                layers: [None; 16],
            }
        );
        // An unknown dye name folds too (no black fallback for banner layers).
        apply_banner_patterns(
            &mut world,
            pos,
            banner_nbt(&[("minecraft:creeper", "not_a_color")]),
        );
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Banner {
                wall: false,
                base_color: EntityDyeColor::White,
                layers: [None; 16],
            }
        );
    }

    #[test]
    fn pattern_stack_clamps_at_the_vanilla_sixteen_layer_render_cap() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:white_banner",
            &[("rotation", "0")],
        );
        let stack: Vec<(&str, &str)> = (0..18).map(|_| ("minecraft:creeper", "black")).collect();
        apply_banner_patterns(&mut world, pos, banner_nbt(&stack));
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0);
        let EntityModelKind::Banner { layers, .. } = instances[0].kind else {
            panic!("expected a banner kind");
        };
        assert!(layers.iter().all(|layer| layer.is_some()));
    }

    #[test]
    fn pattern_id_table_matches_the_vanilla_bootstrap_registrations() {
        // BannerPatterns.java:60-105: every entry's registry id doubles as its
        // asset id; the table reconstructs from the vanilla naming rule, so a
        // typo in either column breaks the round trip below.
        let patterns = [
            "base",
            "square_bottom_left",
            "square_bottom_right",
            "square_top_left",
            "square_top_right",
            "stripe_bottom",
            "stripe_top",
            "stripe_left",
            "stripe_right",
            "stripe_center",
            "stripe_middle",
            "stripe_downright",
            "stripe_downleft",
            "small_stripes",
            "cross",
            "straight_cross",
            "triangle_bottom",
            "triangle_top",
            "triangles_bottom",
            "triangles_top",
            "diagonal_left",
            "diagonal_up_right",
            "diagonal_up_left",
            "diagonal_right",
            "circle",
            "rhombus",
            "half_vertical",
            "half_horizontal",
            "half_vertical_right",
            "half_horizontal_bottom",
            "border",
            "curly_border",
            "gradient",
            "gradient_up",
            "bricks",
            "globe",
            "creeper",
            "skull",
            "flower",
            "mojang",
            "piglin",
            "flow",
            "guster",
        ];
        assert_eq!(patterns.len(), 43);
        for name in patterns {
            assert!(
                banner_pattern_kind(&format!("minecraft:{name}")).is_some(),
                "pattern {name}"
            );
        }
        assert_eq!(banner_pattern_kind("minecraft:ominous"), None);
    }

    #[test]
    fn packs_banner_light_coords_like_vanilla() {
        assert_eq!(
            banner_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            banner_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
    }
}
