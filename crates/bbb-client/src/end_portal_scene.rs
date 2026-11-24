//! World -> renderer projection for end portal/gateway block-entity models.
//!
//! Vanilla submits both invisible portal blocks through block-entity renderers:
//! the end portal/gateway cube faces use `AbstractEndPortalRenderer`, while an
//! active gateway also submits the beacon-style spawn/cooldown beam. bbb keeps
//! both in the shared entity-model stream as block-sentinel instances.

use crate::block_entity_visibility::{
    DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE, END_GATEWAY_VIEW_DISTANCE,
};

use bbb_client_render::{
    EndGatewayBeamRenderState, EndPortalModelFace, EndPortalModelFaces, EndPortalModelKind,
    EntityModelInstance, EntityModelKind,
};
use bbb_client_world::{
    EndGatewayBeamSourceState, EndPortalBlockKind, EndPortalFace, EndPortalFaces,
    EndPortalModelSourceState, WorldStore,
};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const END_PORTAL_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) fn end_portal_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
    source_is_visible: impl Fn(bbb_client_world::BlockPos, f64) -> bool,
) -> Vec<EntityModelInstance> {
    world
        .end_portal_model_source_states_matching(partial_tick, |pos, kind| {
            source_is_visible(
                pos,
                match kind {
                    EndPortalBlockKind::EndPortal => DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE,
                    EndPortalBlockKind::EndGateway => END_GATEWAY_VIEW_DISTANCE,
                },
            )
        })
        .into_iter()
        .map(end_portal_model_instance)
        .collect()
}

fn end_portal_model_instance(source: EndPortalModelSourceState) -> EntityModelInstance {
    let mut instance = EntityModelInstance::new(
        END_PORTAL_BLOCK_MODEL_ENTITY_ID,
        EntityModelKind::EndPortalBlock {
            kind: end_portal_kind(source.kind),
            faces: end_portal_faces(source.faces),
        },
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        0.0,
    );
    if let Some(beam) = source.gateway_beam {
        instance =
            instance.with_block_entity(|s| s.end_gateway_beam = Some(end_gateway_beam(beam)));
    }
    instance
}

fn end_portal_kind(kind: EndPortalBlockKind) -> EndPortalModelKind {
    match kind {
        EndPortalBlockKind::EndPortal => EndPortalModelKind::EndPortal,
        EndPortalBlockKind::EndGateway => EndPortalModelKind::EndGateway,
    }
}

fn end_portal_faces(faces: EndPortalFaces) -> EndPortalModelFaces {
    let mut out = EndPortalModelFaces::EMPTY;
    for face in faces.iter() {
        out = out.with(match face {
            EndPortalFace::Down => EndPortalModelFace::Down,
            EndPortalFace::Up => EndPortalModelFace::Up,
            EndPortalFace::North => EndPortalModelFace::North,
            EndPortalFace::South => EndPortalModelFace::South,
            EndPortalFace::West => EndPortalModelFace::West,
            EndPortalFace::East => EndPortalModelFace::East,
        });
    }
    out
}

fn end_gateway_beam(beam: EndGatewayBeamSourceState) -> EndGatewayBeamRenderState {
    EndGatewayBeamRenderState {
        scale: beam.scale,
        height: beam.height,
        color_argb: beam.color_argb,
        animation_time: beam.animation_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate, PlayTime};
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    bbb_client_world::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str) {
        let properties = BTreeMap::new();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name}"));
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
    fn end_portal_and_gateway_project_to_renderer_block_instances() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:end_portal",
        );
        set_block(
            &mut world,
            BlockPos { x: 5, y: 3, z: 4 },
            "minecraft:end_gateway",
        );

        let instances =
            end_portal_model_instances_from_world_at_partial_tick(&world, 0.5, |_, _| true);

        assert_eq!(instances.len(), 2);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndPortal,
                faces: EndPortalModelFaces::Y_AXIS,
            }
        );
        assert_eq!(
            instances[1].kind,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                faces: EndPortalModelFaces::ALL,
            }
        );
    }

    #[test]
    fn active_gateway_projects_beam_render_state() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(&mut world, pos, "minecraft:end_gateway");
        world.apply_world_time(PlayTime {
            game_time: 43,
            clock_updates: Vec::new(),
        });
        world.advance_end_gateway_ticks(20);

        let instances =
            end_portal_model_instances_from_world_at_partial_tick(&world, 0.5, |_, _| true);

        let beam = instances[0].block_entity.end_gateway_beam.unwrap();
        assert_eq!(beam.color_argb, 0xFFC74EBD);
        assert_eq!(beam.animation_time, 3.5);
        assert!(beam.height > 0);
        assert!(beam.scale > 0.0);
    }
}
