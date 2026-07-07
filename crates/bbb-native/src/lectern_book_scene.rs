//! World -> renderer projection for the lectern's static open book.
//!
//! Vanilla renders it through the `BlockEntityRenderDispatcher` +
//! `LecternRenderer` pair: for each lectern block entity whose `HAS_BOOK`
//! property is set, a `LecternRenderState` carrying `yRot =
//! FACING.getClockWise().toYRot()` and the block-position light coords
//! (`LecternRenderer.extractRenderState`, `java:32-42`), rendered with a fixed
//! `BookModel.State.forAnimation(0, 0.1, 0.9, 1.2)` and a fixed transform. bbb
//! has no separate BER dispatch; the book instance rides the existing single
//! entity-model submission stream as `EntityModelKind::LecternBook`, like the
//! chest and the banner. The book carries no block-entity animation data — the
//! source is purely the block state.

use bbb_renderer::EntityModelInstance;
use bbb_world::{LecternBookModelSourceState, TerrainLight, WorldStore};

/// Like chests/bells/banners, book instances are projected from block states,
/// not the entity list, so they carry a sentinel id no server entity can use.
const LECTERN_BOOK_BLOCK_MODEL_ENTITY_ID: i32 = -1;

/// Vanilla `LecternRenderer.BOOK_STATE = BookModel.State.forAnimation(0.0, 0.1,
/// 0.9, 1.2)` (`LecternRenderer.java:21`): a fixed open book. `progress = 0`,
/// `openness = 1.2` (so the derived `BookModel.State.openness = 1.25·1.2 =
/// 1.5`), `pageFlip1 = 0.1`, `pageFlip2 = 0.9`.
const LECTERN_BOOK_PROGRESS: f32 = 0.0;
const LECTERN_BOOK_OPEN: f32 = 1.2;
const LECTERN_BOOK_PAGE_FLIP_1: f32 = 0.1;
const LECTERN_BOOK_PAGE_FLIP_2: f32 = 0.9;

/// Projects every lectern block with a book into an open-book instance:
/// position at the block min corner, the `FACING.getClockWise().toYRot()` yaw,
/// the fixed `BookModel.State`, and the block-position light.
pub(crate) fn lectern_book_model_instances_from_world(
    world: &WorldStore,
) -> Vec<EntityModelInstance> {
    world
        .lectern_book_model_source_states()
        .into_iter()
        .map(|source| lectern_book_model_instance(&source, world))
        .collect()
}

fn lectern_book_model_instance(
    source: &LecternBookModelSourceState,
    world: &WorldStore,
) -> EntityModelInstance {
    let mut instance = EntityModelInstance::lectern_book(
        LECTERN_BOOK_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        source.y_rot,
    )
    .with_book_progress(LECTERN_BOOK_PROGRESS)
    .with_book_open(LECTERN_BOOK_OPEN)
    .with_book_page_flip_1(LECTERN_BOOK_PAGE_FLIP_1)
    .with_book_page_flip_2(LECTERN_BOOK_PAGE_FLIP_2);
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(book_light_coords(light));
    }
    instance
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

    fn set_lectern(world: &mut WorldStore, pos: BlockPos, facing: &str, has_book: bool) {
        let properties: BTreeMap<String, String> = [
            ("facing".to_string(), facing.to_string()),
            ("powered".to_string(), "false".to_string()),
            ("has_book".to_string(), has_book.to_string()),
        ]
        .into_iter()
        .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties("minecraft:lectern", &properties)
            .expect("no registered state for minecraft:lectern");
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
    fn projects_only_lecterns_with_a_book_with_fixed_state_and_facing_yaw() {
        let mut world = world_with_air_chunk();
        let with_book = BlockPos { x: 3, y: 4, z: 5 };
        let without_book = BlockPos { x: 6, y: 4, z: 5 };
        set_lectern(&mut world, with_book, "east", true);
        set_lectern(&mut world, without_book, "north", false);

        let instances = lectern_book_model_instances_from_world(&world);
        assert_eq!(instances.len(), 1);
        let book = &instances[0];
        assert_eq!(book.kind, EntityModelKind::LecternBook);
        assert_eq!(book.entity_id, LECTERN_BOOK_BLOCK_MODEL_ENTITY_ID);
        assert_eq!(book.position, [3.0, 4.0, 5.0]);
        // EAST.getClockWise() = SOUTH; SOUTH.toYRot() = 0.
        assert_eq!(book.render_state.body_rot, 0.0);
        // The fixed BOOK_STATE fields.
        assert_eq!(book.render_state.book_progress, 0.0);
        assert_eq!(book.render_state.book_open, 1.2);
        assert_eq!(book.render_state.book_page_flip_1, 0.1);
        assert_eq!(book.render_state.book_page_flip_2, 0.9);
        // Empty light data: sky falls back to 15, block to 0 -> pack(0, 15).
        assert_eq!(book.render_state.light_coords, 15 << 20);
    }

    #[test]
    fn facing_maps_to_the_clockwise_yaw() {
        let mut world = world_with_air_chunk();
        for (facing, expected) in [
            ("north", -90.0),
            ("south", 90.0),
            ("west", 180.0),
            ("east", 0.0),
        ] {
            let pos = BlockPos { x: 1, y: 2, z: 3 };
            set_lectern(&mut world, pos, facing, true);
            let instances = lectern_book_model_instances_from_world(&world);
            assert_eq!(instances.len(), 1, "facing {facing}");
            assert_eq!(
                instances[0].render_state.body_rot, expected,
                "facing {facing}"
            );
        }
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
