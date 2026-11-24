//! Whole-mesh draw lanes: each resident entity mesh drawn with one pipeline and its bind groups.

use crate::entity_models::EntityModelMeshGpu;

/// A lane draws only when its mesh and its group-0 bind group are both resident; group 1 is
/// left as bound when `None`.
pub(super) struct MeshLane<'a> {
    pose: Option<&'a wgpu::BindGroup>,
    mesh: Option<(&'a wgpu::Buffer, &'a wgpu::Buffer, u32)>,
    pipeline: &'a wgpu::RenderPipeline,
    group0: Option<&'a wgpu::BindGroup>,
    group1: Option<&'a wgpu::BindGroup>,
}

pub(super) fn lane<'a>(
    mesh: &'a Option<EntityModelMeshGpu>,
    pipeline: &'a wgpu::RenderPipeline,
    group0: Option<&'a wgpu::BindGroup>,
    group1: Option<&'a wgpu::BindGroup>,
) -> MeshLane<'a> {
    MeshLane {
        pose: None,
        mesh: mesh
            .as_ref()
            .map(|mesh| (&mesh.vertex_buffer, &mesh.index_buffer, mesh.index_count)),
        pipeline,
        group0,
        group1,
    }
}

pub(super) fn entity_surface_lane<'a>(
    mesh: &'a Option<EntityModelMeshGpu>,
    pipeline: &'a wgpu::RenderPipeline,
    posed_pipeline: Option<&'a wgpu::RenderPipeline>,
    atlas: Option<&'a wgpu::BindGroup>,
    lightmap: Option<&'a wgpu::BindGroup>,
) -> MeshLane<'a> {
    let mut lane = lane(mesh, pipeline, atlas, lightmap);
    if let Some(pose) = mesh.as_ref().and_then(|m| m.vertex_pose.as_ref()) {
        lane.pipeline = posed_pipeline.unwrap();
        lane.pose = Some(&pose.group);
    }
    lane
}

pub(super) fn draw_mesh_lanes(
    pass: &mut wgpu::RenderPass<'_>,
    lanes: &[MeshLane<'_>],
    pipeline_switches: &mut u64,
    draw_calls: &mut u64,
) {
    for lane in lanes {
        let (Some((vertices, indices, index_count)), Some(group0)) = (lane.mesh, lane.group0)
        else {
            continue;
        };
        pass.set_pipeline(lane.pipeline);
        *pipeline_switches += 1;
        pass.set_bind_group(0, group0, &[]);
        if let Some(group1) = lane.group1 {
            pass.set_bind_group(1, group1, &[]);
        }
        if let Some(pose) = lane.pose {
            pass.set_bind_group(2, pose, &[]);
        }
        pass.set_vertex_buffer(0, vertices.slice(..));
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);
        *draw_calls += 1;
    }
}
