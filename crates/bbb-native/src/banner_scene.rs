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

use bbb_protocol::{
    builtin_enum_component_values::vanilla_dye_color_name, packets::BannerPatternLayerSummary,
};
use bbb_renderer::{
    BannerPatternKind, BannerPatternLayer, BannerPatternTexture, EntityDyeColor,
    EntityDynamicTexture, EntityModelInstance,
};
use bbb_world::{
    BannerBlockForm as WorldBannerBlockForm, BannerDyeColorKind as WorldBannerDyeColorKind,
    BannerModelSourceState, BannerPatternLayerState, WorldStore,
};

use crate::entity_variant_textures::NativeEntityVariantTextureRuntime;

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
    textures: Option<&NativeEntityVariantTextureRuntime>,
) -> Vec<EntityModelInstance> {
    world
        .banner_model_source_states(partial_tick)
        .into_iter()
        .map(|source| banner_model_instance(&source, world, textures))
        .collect()
}

fn banner_model_instance(
    source: &BannerModelSourceState,
    world: &WorldStore,
    textures: Option<&NativeEntityVariantTextureRuntime>,
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
        banner_pattern_layers(world, &source.layers, |sprite_id| {
            textures.and_then(|textures| textures.ready_banner_pattern_texture(sprite_id))
        }),
    )
    .with_banner_flag_phase(source.phase);
    if let Some(light) = world.sample_render_light(source.pos) {
        instance = instance.with_light_coords(light.packed());
    }
    instance
}

/// The block-id base color (`AbstractBannerBlock.getColor`) as the
/// renderer's dye.
pub(crate) fn banner_dye_color(color: WorldBannerDyeColorKind) -> EntityDyeColor {
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
/// invalid pattern value or dye name folds the whole stack to empty, like the vanilla
/// `BannerPatternLayers.CODEC` failing the registry/dye lookup and `loadAdditional` falling back to
/// `BannerPatternLayers.EMPTY`. Data-driven patterns use `Sheets.BANNER_MAPPER`'s runtime atlas;
/// known bootstrap patterns retain the static sheet only when that atlas is unavailable.
fn banner_pattern_layers(
    world: &WorldStore,
    layers: &[BannerPatternLayerState],
    mut ready_texture: impl FnMut(&str) -> Option<EntityDynamicTexture>,
) -> [Option<BannerPatternLayer>; 16] {
    let mut out = [None; 16];
    for (index, layer) in layers.iter().enumerate() {
        let Ok(Some(presentation)) = world.banner_pattern_presentation(&layer.pattern) else {
            return [None; 16];
        };
        let Some(color) = banner_layer_dye_color(&layer.color) else {
            return [None; 16];
        };
        let texture = banner_pattern_texture_with(&presentation.asset_id, |sprite_id| {
            ready_texture(sprite_id)
        });
        let Some(texture) = texture else {
            return [None; 16];
        };
        if index < out.len() {
            out[index] = Some(BannerPatternLayer { texture, color });
        }
    }
    out
}

/// Resolves one banner pattern through the shared `Sheets.BANNER_MAPPER`
/// atlas owner. A ready resource-pack atlas wins over the checked-in
/// bootstrap sheet; otherwise only an exact builtin asset may use the static
/// atlas. Callers must first resolve synchronized holder identity through
/// [`WorldStore::synchronized_banner_pattern_presentation`].
pub(crate) fn banner_pattern_texture(
    asset_id: &str,
    textures: Option<&NativeEntityVariantTextureRuntime>,
) -> Option<BannerPatternTexture> {
    banner_pattern_texture_with(asset_id, |sprite_id| {
        textures.and_then(|textures| textures.ready_banner_pattern_texture(sprite_id))
    })
}

fn banner_pattern_texture_with(
    asset_id: &str,
    mut ready_texture: impl FnMut(&str) -> Option<EntityDynamicTexture>,
) -> Option<BannerPatternTexture> {
    let sprite_id = banner_pattern_sprite_id(asset_id)?;
    ready_texture(&sprite_id)
        .map(BannerPatternTexture::Runtime)
        .or_else(|| BannerPatternKind::for_resource_id(asset_id).map(BannerPatternTexture::Builtin))
}

/// Resolves one item-component layer's asset identity without letting a
/// bootstrap ordinal override a synchronized registry. Direct values retain
/// their packet asset id; numeric values use Vanilla's fixed order only when
/// no `WorldStore` registry context exists.
pub(crate) fn banner_component_pattern_asset_id(
    world: Option<&WorldStore>,
    layer: &BannerPatternLayerSummary,
) -> Option<String> {
    if let Some(asset_id) = &layer.asset_id {
        banner_pattern_sprite_id(asset_id)?;
        return Some(asset_id.clone());
    }
    let holder_id = layer.registry_id?;
    match world {
        Some(world) => world
            .synchronized_banner_pattern_presentation(holder_id)
            .ok()
            .flatten()
            .map(|presentation| presentation.asset_id),
        None => BannerPatternKind::for_vanilla_registry_id(holder_id)
            .map(|pattern| pattern.resource_id().to_string()),
    }
}

/// Projects one effective `BANNER_PATTERNS` component into the renderer's
/// fixed 16-layer banner stack. Invalid holder identity folds the component
/// to empty like the Vanilla codec. A temporarily unavailable custom texture
/// leaves only that layer absent; later ready atlas publication restores it
/// without changing canonical item state.
pub(crate) fn banner_component_pattern_layers(
    world: &WorldStore,
    layers: &[BannerPatternLayerSummary],
    textures: Option<&NativeEntityVariantTextureRuntime>,
) -> [Option<BannerPatternLayer>; 16] {
    let mut out = [None; 16];
    for (index, layer) in layers.iter().take(out.len()).enumerate() {
        let Some(asset_id) = banner_component_pattern_asset_id(Some(world), layer) else {
            return [None; 16];
        };
        let Some(color) = banner_layer_dye_color(vanilla_dye_color_name(layer.color_id)) else {
            return [None; 16];
        };
        out[index] = banner_pattern_texture(&asset_id, textures)
            .map(|texture| BannerPatternLayer { texture, color });
    }
    out
}

/// Vanilla `Sheets.BANNER_MAPPER = new MaterialMapper(BLOCK_SHEET, "entity/banner")` prefixes the
/// direct banner-pattern `asset_id` path while preserving its namespace.
fn banner_pattern_sprite_id(asset_id: &str) -> Option<String> {
    let (namespace, path) = asset_id.split_once(':')?;
    if namespace.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{namespace}:entity/banner/{path}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockEntityData, BlockPos as ProtocolBlockPos};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, RegistryPacketEntry, WorldDimension,
    };
    use std::collections::BTreeMap;

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
                    bbb_world::vanilla_air_block_state_id(),
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
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID,
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

        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.5, None);
        assert_eq!(instances.len(), 2);
        let standing = &instances[0];
        let wall = &instances[1];
        let mut layers = [None; 16];
        layers[0] = Some(BannerPatternLayer {
            texture: BannerPatternTexture::Builtin(BannerPatternKind::StripeTop),
            color: EntityDyeColor::Purple,
        });
        layers[1] = Some(BannerPatternLayer {
            texture: BannerPatternTexture::Builtin(BannerPatternKind::Creeper),
            color: EntityDyeColor::White,
        });
        assert_eq!(
            standing.kind,
            EntityModelKind::Banner {
                wall: false,
                flag_only: false,
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
                flag_only: false,
                base_color: EntityDyeColor::Red,
                layers: [None; 16],
            }
        );
        // NORTH.toYRot() = 180 -> body_rot -180.
        assert_eq!(wall.render_state.body_rot, -180.0);
    }

    #[test]
    fn named_block_entity_layer_uses_synchronized_pattern_value_not_static_resource_name() {
        let mut world = world_with_air_chunk();
        world.record_registry_entries(
            bbb_world::BANNER_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "minecraft:stripe_top",
                banner_pattern_registry_nbt("minecraft:creeper", "block.minecraft.banner.creeper"),
            )],
        );
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
            banner_nbt(&[("minecraft:stripe_top", "red")]),
        );

        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0, None);
        let EntityModelKind::Banner { layers, .. } = instances[0].kind else {
            panic!("expected banner instance");
        };
        assert_eq!(
            layers[0],
            Some(BannerPatternLayer {
                texture: BannerPatternTexture::Builtin(BannerPatternKind::Creeper),
                color: EntityDyeColor::Red,
            })
        );
    }

    #[test]
    fn data_driven_pattern_uses_mapper_sprite_and_runtime_texture() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            bbb_world::BANNER_PATTERN_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:custom",
                banner_pattern_registry_nbt("example:custom_asset", "block.example.banner.custom"),
            )],
        );
        let texture = EntityDynamicTexture { handle: 91_001 };
        let layers = banner_pattern_layers(
            &world,
            &[BannerPatternLayerState {
                pattern: bbb_world::BannerPatternHolderState::Named("example:custom".to_string()),
                color: "cyan".to_string(),
            }],
            |sprite_id| {
                assert_eq!(sprite_id, "example:entity/banner/custom_asset");
                Some(texture)
            },
        );

        assert_eq!(
            layers[0],
            Some(BannerPatternLayer {
                texture: BannerPatternTexture::Runtime(texture),
                color: EntityDyeColor::Cyan,
            })
        );
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
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0, None);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Banner {
                wall: false,
                flag_only: false,
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
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0, None);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::Banner {
                wall: false,
                flag_only: false,
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
        let instances = banner_model_instances_from_world_at_partial_tick(&world, 0.0, None);
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
                BannerPatternKind::for_resource_id(&format!("minecraft:{name}")).is_some(),
                "pattern {name}"
            );
        }
        assert_eq!(
            BannerPatternKind::for_resource_id("minecraft:ominous"),
            None
        );
    }

    fn banner_pattern_registry_nbt(asset_id: &str, translation_key: &str) -> Vec<u8> {
        let mut payload = vec![10, 8];
        write_mutf8(&mut payload, "asset_id");
        write_mutf8(&mut payload, asset_id);
        payload.push(8);
        write_mutf8(&mut payload, "translation_key");
        write_mutf8(&mut payload, translation_key);
        payload.push(0);
        payload
    }
}
