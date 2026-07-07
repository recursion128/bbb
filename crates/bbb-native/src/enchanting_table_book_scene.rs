//! World -> renderer projection for the enchanting-table hovering book.
//!
//! Vanilla renders it through the `BlockEntityRenderDispatcher` +
//! `EnchantTableRenderer` pair: per enchanting-table block entity, an
//! `EnchantTableRenderState` carrying the partial-tick-interpolated `flip` /
//! `open` / `time` / `yRot` (`EnchantTableRenderer.extractRenderState`,
//! `java:34-56`), and the light coords sampled at the block position. bbb has
//! no separate BER dispatch; the book instance rides the existing single
//! entity-model submission stream as `EntityModelKind::EnchantingBook`, like
//! the chest and the banner. The `extractRenderState` lerp and the
//! `EnchantTableRenderer.submit` `BookModel.State` derivation run here.

use bbb_renderer::EntityModelInstance;
use bbb_world::{EnchantingTableBookSourceState, TerrainLight, WorldStore};

/// Like chests/bells/banners, book instances are projected from block states,
/// not the entity list, so they carry a sentinel id no server entity can use.
const ENCHANTING_BOOK_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Projects every enchanting-table block in the loaded chunks into a hovering
/// book instance: position at the block min corner, the
/// `EnchantTableRenderer.extractRenderState` partial-tick lerp of the raw
/// animation fields, the `EnchantTableRenderer.submit` `BookModel.State`
/// page-flip fractions + hover offset, and the block-position light.
pub(crate) fn enchanting_table_book_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .enchanting_table_book_source_states()
        .into_iter()
        .map(|source| enchanting_table_book_model_instance(&source, world, partial_tick))
        .collect()
}

fn enchanting_table_book_model_instance(
    source: &EnchantingTableBookSourceState,
    world: &WorldStore,
    partial_tick: f32,
) -> EntityModelInstance {
    // `EnchantTableRenderer.extractRenderState` (`java:42-55`).
    let flip = lerp(partial_tick, source.o_flip, source.flip);
    let open = lerp(partial_tick, source.o_open, source.open);
    let time = source.time as f32 + partial_tick;
    let y_rot_radians = source.o_rot + wrap_radians(source.rot - source.o_rot) * partial_tick;

    // `EnchantTableRenderer.submit` (`java:63-69`): the hover bob, the two
    // page-flip fractions, and the `BookModel.State` progress/openness. The
    // `BookModel.State.forAnimation` openness derivation runs renderer-side.
    let float_y = 0.1 + (time * 0.1).sin() * 0.01;
    let page_flip_1 = ((frac(flip + 0.25) * 1.6) - 0.3).clamp(0.0, 1.0);
    let page_flip_2 = ((frac(flip + 0.75) * 1.6) - 0.3).clamp(0.0, 1.0);

    let mut instance = EntityModelInstance::enchanting_book(
        ENCHANTING_BOOK_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        // Vanilla `Axis.YP.rotation(-yRot)` takes radians; `body_rot` carries
        // degrees, so the transform's `Ry(-body_rot°)` reproduces `-yRot` rad.
        y_rot_radians.to_degrees(),
    )
    .with_book_progress(time)
    .with_book_open(open)
    .with_book_page_flip_1(page_flip_1)
    .with_book_page_flip_2(page_flip_2)
    .with_book_float_y(float_y);
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(book_light_coords(light));
    }
    instance
}

/// Vanilla `Mth.lerp(delta, start, end) = start + delta·(end − start)`.
fn lerp(delta: f32, start: f32, end: f32) -> f32 {
    start + delta * (end - start)
}

/// Vanilla `Mth.frac(value) = value − floor(value)`.
fn frac(value: f32) -> f32 {
    value - value.floor()
}

/// Vanilla `EnchantTableRenderer.extractRenderState`'s `or` fold into
/// `(-π, π]` (`while (or >= π) or -= 2π; while (or < -π) or += 2π;`).
fn wrap_radians(mut value: f32) -> f32 {
    use std::f32::consts::PI;
    while value >= PI {
        value -= 2.0 * PI;
    }
    while value < -PI {
        value += 2.0 * PI;
    }
    value
}

/// Vanilla `LightCoordsUtil.pack(block, sky)` (`block << 4 | sky << 20`) over
/// the raw stored light sample — the book render state's `lightCoords`.
fn book_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
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

    fn set_enchanting_table(world: &mut WorldStore, pos: BlockPos) {
        let properties: BTreeMap<String, String> = BTreeMap::new();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:enchanting_table", &properties)
            .expect("no registered state for minecraft:enchanting_table");
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    #[test]
    fn projects_a_closed_book_for_an_untouched_table() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_enchanting_table(&mut world, pos);
        let instances =
            enchanting_table_book_model_instances_from_world_at_partial_tick(&world, 0.0);
        assert_eq!(instances.len(), 1);
        let book = &instances[0];
        assert_eq!(book.kind, EntityModelKind::EnchantingBook);
        assert_eq!(book.entity_id, ENCHANTING_BOOK_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(book.position, [3.0, 4.0, 5.0]);
        // A fresh table: time/open/flip all 0, so progress/open/flips are 0 and
        // the yaw is 0.
        assert_eq!(book.render_state.book_progress, 0.0);
        assert_eq!(book.render_state.book_open, 0.0);
        assert_eq!(book.render_state.body_rot, 0.0);
        // float_y = 0.1 + sin(0)*0.01 = 0.1.
        assert!((book.render_state.book_float_y - 0.1).abs() < 1e-6);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(book.render_state.light_coords, 15 << 20);
    }

    #[test]
    fn extracts_partial_tick_lerp_and_book_state_fields() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        set_enchanting_table(&mut world, pos);
        // With no player the book stays closed but `time` and `rot` still
        // advance (tRot drifts 0.02/tick and rot chases it), exercising the
        // time / yaw projection math without needing to place a player.
        for _ in 0..6 {
            world.advance_enchanting_table_book_ticks(1);
        }
        let source = world.enchanting_table_book_source_states()[0];
        assert_eq!(source.time, 6);
        // rot has moved off zero as it chases the drifting tRot.
        assert!(source.rot.abs() > 0.0);
        let pt = 0.5;
        let instances =
            enchanting_table_book_model_instances_from_world_at_partial_tick(&world, pt);
        let book = &instances[0];
        // book_progress = time + partialTick.
        assert!((book.render_state.book_progress - (source.time as f32 + pt)).abs() < 1e-5);
        // book_open = lerp(pt, oOpen, open) — still 0 for a closed book.
        let expected_open = source.o_open + pt * (source.open - source.o_open);
        assert!((book.render_state.book_open - expected_open).abs() < 1e-6);
        // The yaw is the wrapped `oRot + or*pt`, in degrees.
        let expected_yaw =
            (source.o_rot + wrap_radians(source.rot - source.o_rot) * pt).to_degrees();
        assert!((book.render_state.body_rot - expected_yaw).abs() < 1e-4);
    }

    #[test]
    fn closed_book_page_flip_fractions_are_the_fixed_frac_of_zero_flip() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_enchanting_table(&mut world, pos);
        let instances =
            enchanting_table_book_model_instances_from_world_at_partial_tick(&world, 0.0);
        let book = &instances[0];
        // flip = 0 -> ff1 = clamp(frac(0.25)·1.6 − 0.3) = 0.4 − 0.3 = 0.1.
        assert!((book.render_state.book_page_flip_1 - 0.1).abs() < 1e-6);
        // ff2 = clamp(frac(0.75)·1.6 − 0.3) = 1.2 − 0.3 = 0.9.
        assert!((book.render_state.book_page_flip_2 - 0.9).abs() < 1e-6);
    }

    #[test]
    fn lerp_frac_and_wrap_match_vanilla_math() {
        // Mth.lerp(0.25, 4, 8) = 4 + 0.25·4 = 5.
        assert_eq!(lerp(0.25, 4.0, 8.0), 5.0);
        // Mth.frac(2.75) = 0.75; Mth.frac(-0.3) = 0.7.
        assert!((frac(2.75) - 0.75).abs() < 1e-6);
        assert!((frac(-0.3) - 0.7).abs() < 1e-6);
        // wrap folds (-π, π]: 1.5π -> -0.5π, -1.5π -> 0.5π.
        use std::f32::consts::PI;
        assert!((wrap_radians(1.5 * PI) - (-0.5 * PI)).abs() < 1e-5);
        assert!((wrap_radians(-1.5 * PI) - (0.5 * PI)).abs() < 1e-5);
    }

    #[test]
    fn packs_book_light_coords_like_vanilla() {
        assert_eq!(
            book_light_coords(TerrainLight { sky: 15, block: 0 }),
            15 << 20
        );
        assert_eq!(
            book_light_coords(TerrainLight { sky: 7, block: 9 }),
            9 << 4 | 7 << 20
        );
    }
}
