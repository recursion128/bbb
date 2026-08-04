//! Immutable render extraction for `TestInstanceRenderer.submitErrorMarker`.
//!
//! Beside the beacon beam and the bounding box it delegates to, the test
//! instance renderer draws its own error markers: a translucent red cube filling
//! the offending block plus a white always-on-top label above it.

use bbb_renderer::{DebugFilledBox, DebugTextBillboard};
use bbb_world::WorldStore;

/// `TestInstanceRenderer.ERROR_PADDING`.
const ERROR_PADDING: f32 = 0.02;
/// `ARGB.colorFromFloat(0.375F, 1.0F, 0.0F, 0.0F)`.
const ERROR_COLOR: [u8; 4] = [255, 0, 0, 96];
const LABEL_COLOR: [u8; 4] = [255; 4];
/// `TextGizmo.Style.whiteAndCentered().withScale(0.16F)`.
const LABEL_SCALE: f32 = 0.16;
/// `Vec3.atLowerCornerWithOffset(pos, 0.5, 1.2, 0.5)`.
const LABEL_HEIGHT: f32 = 1.2;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct TestInstanceErrorGizmos {
    pub(crate) filled_boxes: Vec<DebugFilledBox>,
    pub(crate) text_billboards: Vec<DebugTextBillboard>,
}

pub(crate) fn test_instance_error_gizmos(world: &WorldStore) -> TestInstanceErrorGizmos {
    let mut output = TestInstanceErrorGizmos::default();
    for (_, marker) in world.test_instance_error_marker_source_states() {
        let pos = [
            marker.pos.x as f32,
            marker.pos.y as f32,
            marker.pos.z as f32,
        ];
        output.filled_boxes.push(DebugFilledBox {
            min: pos.map(|coordinate| coordinate - ERROR_PADDING),
            max: pos.map(|coordinate| coordinate + 1.0 + ERROR_PADDING),
            color: ERROR_COLOR,
        });
        let mut label = DebugTextBillboard::centered(
            [pos[0] + 0.5, pos[1] + LABEL_HEIGHT, pos[2] + 0.5],
            marker.text.text(),
            LABEL_COLOR,
        );
        label.scale = LABEL_SCALE;
        label.always_on_top = true;
        output.text_billboards.push(label);
    }
    output
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
        packets::BlockEntityData,
    };
    use bbb_world::{
        BlockEntityRecord, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };

    use super::*;

    #[test]
    fn error_markers_match_vanilla_geometry_color_and_label() {
        let mut world = world_with_test_instance_block();
        // `{errors: [{pos: [I; 12, 65, -4], text: "boom"}]}`.
        let mut nbt = vec![10, 9];
        push_nbt_string(&mut nbt, "errors");
        nbt.push(10);
        nbt.extend_from_slice(&1_i32.to_be_bytes());
        nbt.push(11);
        push_nbt_string(&mut nbt, "pos");
        nbt.extend_from_slice(&3_i32.to_be_bytes());
        for value in [12_i32, 65, -4] {
            nbt.extend_from_slice(&value.to_be_bytes());
        }
        nbt.push(8);
        push_nbt_string(&mut nbt, "text");
        push_nbt_string(&mut nbt, "boom");
        nbt.push(0);
        nbt.push(0);
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
                raw_nbt: nbt,
            })
            .expect("test instance block entity data"));

        let gizmos = test_instance_error_gizmos(&world);
        assert_eq!(
            gizmos.filled_boxes,
            vec![DebugFilledBox {
                min: [11.98, 64.98, -4.02],
                max: [13.02, 66.02, -2.98],
                color: [255, 0, 0, 96],
            }]
        );
        assert_eq!(gizmos.text_billboards.len(), 1);
        let label = &gizmos.text_billboards[0];
        assert_eq!(label.position, [12.5, 66.2, -3.5]);
        assert_eq!(label.text, "boom");
        assert_eq!(label.scale, LABEL_SCALE);
        assert!(label.always_on_top);
    }

    #[test]
    fn blocks_without_error_markers_emit_nothing() {
        assert_eq!(
            test_instance_error_gizmos(&world_with_test_instance_block()),
            TestInstanceErrorGizmos::default()
        );
    }

    fn world_with_test_instance_block() -> WorldStore {
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
            block_entities: vec![BlockEntityRecord::empty(
                0,
                0,
                0,
                VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
            )],
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

    fn push_nbt_string(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
