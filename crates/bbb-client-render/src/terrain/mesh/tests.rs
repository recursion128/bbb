use super::emitter::quad_lighting;
use super::geometry::FACES;
use super::{cell_index, culls_face_between_cells, TerrainChunkLookup};
use crate::gpu::TerrainGpuVertex;
use crate::terrain::{
    fixture::{TerrainCell, TerrainChunkFixture, TerrainFixtureWorld},
    TerrainBox, TerrainCardinalLighting, TerrainCross, TerrainFace, TerrainFluid, TerrainFluidKind,
    TerrainLight, TerrainMaterialClass, TerrainMesh, TerrainQuad, TerrainRenderShape,
    TerrainSectionMeshLayers, TerrainSkipRendering, TerrainTextureAtlas, TerrainTint,
    TerrainTransparency, TerrainUvRect,
};

mod exact_output;

#[test]
fn world_mesh_applies_plant_position_offsets() {
    use crate::scene::terrain_textures::{TerrainResourceSource, TerrainTextureState};
    use crate::terrain::{
        build_terrain_section_meshes, TerrainBlockRenderTable, TerrainChunkSnapshot,
    };
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;

    let textures = TerrainTextureState::default();
    let table = TerrainBlockRenderTable::from_resources(TerrainResourceSource::new(
        &textures,
        bbb_render_types::TerrainBlockRenderContext::new(Vec::new(), |state_id| {
            Some((
                if state_id == 0 {
                    bbb_render_types::TerrainMaterialClass::Empty
                } else {
                    bbb_render_types::TerrainMaterialClass::Opaque
                },
                None,
            ))
        }),
    ));
    for (name, x, expected) in [
        ("minecraft:dandelion", -26_i32, [0.25, 0.0, -0.11666666]),
        (
            "minecraft:short_grass",
            -24,
            [0.08333334, -0.18666667, -0.049999997],
        ),
    ] {
        let id =
            vanilla_block_state_id_by_name_and_properties(name, &std::collections::BTreeMap::new())
                .unwrap();
        let mut cells = vec![(0_u32, TerrainLight::FULL_BRIGHT); 16 * 16 * 16];
        cells[super::cell_index(x.rem_euclid(16) as usize, 0, 15, 16)] =
            (id as u32, TerrainLight::FULL_BRIGHT);
        let snapshot = TerrainChunkSnapshot::from_cells(-2, -3, 64, 16, cells);
        let layers =
            build_terrain_section_meshes(&snapshot, std::iter::empty(), &table, [0.0; 3], &[4]);
        let vertices: Vec<_> = [&layers.opaque, &layers.cutout, &layers.translucent]
            .into_iter()
            .flatten()
            .flat_map(|section| &section.mesh.vertices)
            .collect();
        for (axis, base) in [x as f32, 64.0, -33.0].into_iter().enumerate() {
            let min = vertices
                .iter()
                .map(|v| v.world_position([-32, 64, -48])[axis])
                .reduce(f32::min)
                .unwrap();
            let max = vertices
                .iter()
                .map(|v| v.world_position([-32, 64, -48])[axis])
                .reduce(f32::max)
                .unwrap();
            assert!(
                (min - (base + expected[axis])).abs() < 3.0e-4,
                "{name} min axis {axis}: {min}"
            );
            assert!(
                (max - (base + expected[axis] + 1.0)).abs() < 3.0e-4,
                "{name} max axis {axis}: {max}"
            );
        }
    }
}

const UP: [f32; 3] = [0.0, 1.0, 0.0];
const DOWN: [f32; 3] = [0.0, -1.0, 0.0];
const NORTH: [f32; 3] = [0.0, 0.0, -1.0];
const SOUTH: [f32; 3] = [0.0, 0.0, 1.0];
const WEST: [f32; 3] = [-1.0, 0.0, 0.0];
const EAST: [f32; 3] = [1.0, 0.0, 0.0];

/// A decoded GPU vertex plus the facing of the triangles the GPU draws for its quad.
#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    light: [f32; 2],
    color: [u8; 4],
    normal: [f32; 3],
}

fn decode(mesh: &TerrainMesh) -> Vec<Vertex> {
    let mut normals = vec![None; mesh.vertices.len() / 4];
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let quad = triangle[0] as usize / 4;
        assert!(triangle.iter().all(|index| *index as usize / 4 == quad));
        if normals[quad].is_none() {
            let [a, b, c] =
                [0, 1, 2].map(|corner| mesh.vertices[triangle[corner] as usize].local_position());
            normals[quad] = Some(unit_normal(a, b, c));
        }
    }
    mesh.vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| Vertex {
            position: vertex.local_position(),
            uv: vertex.uv_f32(),
            light: vertex
                .light_coords()
                .map(|value| f32::from(value) / 16.0 / 15.0),
            color: vertex.color,
            normal: normals[index / 4].expect("every quad is indexed"),
        })
        .collect()
}

fn unit_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    n.map(|value| value / length)
}

fn same_direction(left: [f32; 3], right: [f32; 3]) -> bool {
    left.iter().zip(right).all(|(a, b)| (a - b).abs() < 3.0e-4)
}

/// Quads drawn for the block at `cell`: inside its unit box, excluding a
/// neighbor's face lying on the shared boundary plane.
fn cell_quads(mesh: &TerrainMesh, cell: [i32; 3]) -> Vec<[Vertex; 4]> {
    let eps = 3.0e-4;
    decode(mesh)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|quad| [quad[0], quad[1], quad[2], quad[3]])
        .filter(|quad| {
            quad.iter().all(|vertex| {
                (0..3).all(|axis| {
                    let min = cell[axis] as f32;
                    (min - eps..=min + 1.0 + eps).contains(&vertex.position[axis])
                })
            })
        })
        .filter(|quad| {
            (0..3).all(|axis| {
                let on = |plane: f32| {
                    quad.iter()
                        .all(|vertex| (vertex.position[axis] - plane).abs() < eps)
                };
                let normal = quad[0].normal[axis];
                !(on(cell[axis] as f32) && normal > 0.5
                    || on(cell[axis] as f32 + 1.0) && normal < -0.5)
            })
        })
        .collect()
}

fn cell_vertices(mesh: &TerrainMesh, cell: [i32; 3]) -> Vec<Vertex> {
    cell_quads(mesh, cell).into_iter().flatten().collect()
}

/// Vertices of the block's quads facing `normal` (sloped fluid tops included).
fn face_vertices(mesh: &TerrainMesh, cell: [i32; 3], normal: [f32; 3]) -> Vec<Vertex> {
    cell_vertices(mesh, cell)
        .into_iter()
        .filter(|vertex| {
            (0..3)
                .map(|axis| vertex.normal[axis] * normal[axis])
                .sum::<f32>()
                > 0.7
        })
        .collect()
}

/// The front quad on the block's north side; the south side's reversed fluid
/// back face also faces north.
fn north_side(mesh: &TerrainMesh, cell: [i32; 3]) -> Vec<Vertex> {
    face_vertices(mesh, cell, NORTH)
        .into_iter()
        .filter(|vertex| vertex.position[2] < cell[2] as f32 + 0.5)
        .collect()
}

fn vertex_at(vertices: &[Vertex], position: [f32; 3]) -> Vertex {
    *vertices
        .iter()
        .find(|vertex| vertex.position == position)
        .expect("vertex exists at position")
}

fn vertex_at_approx(vertices: &[Vertex], position: [f32; 3]) -> Vertex {
    *vertices
        .iter()
        .find(|vertex| same_direction(vertex.position, position))
        .expect("vertex exists near position")
}

fn vertex_at_xz(vertices: &[Vertex], x: f32, z: f32) -> Vertex {
    *vertices
        .iter()
        .find(|vertex| vertex.position[0] == x && vertex.position[2] == z)
        .expect("vertex exists at x/z")
}

fn assert_float_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 3.0e-4,
        "expected {expected}, got {actual}"
    );
}

fn assert_uv_eq(actual: [f32; 2], expected: [f32; 2]) {
    assert_float_eq(actual[0], expected[0]);
    assert_float_eq(actual[1], expected[1]);
}

/// The packed vertex color of `tint` under a face shade and ambient occlusion.
fn color(tint: TerrainTint, shade: f32, ambient_occlusion: f32) -> [u8; 4] {
    TerrainGpuVertex::pack([0.0; 3], [0.0; 2], 0, tint, shade, ambient_occlusion).color
}

fn assert_face_shade(mesh: &TerrainMesh, normal: [f32; 3], expected: f32) {
    let vertices = face_vertices(mesh, [1, 0, 2], normal);
    assert_eq!(vertices.len(), 4);
    assert!(vertices
        .iter()
        .all(|vertex| vertex.color == color(TerrainTint::WHITE, expected, 1.0)));
}

fn assert_face_light(mesh: &TerrainMesh, cell: [i32; 3], normal: [f32; 3], expected: [f32; 2]) {
    let vertices = face_vertices(mesh, cell, normal);
    assert_eq!(vertices.len(), 4);
    assert!(vertices.iter().all(|vertex| {
        (vertex.light[0] - expected[0]).abs() < 0.0001
            && (vertex.light[1] - expected[1]).abs() < 0.0001
    }));
}

fn chunk(height: usize, cells: Vec<TerrainCell>) -> TerrainChunkFixture {
    TerrainChunkFixture::new(0, 0, 0, height, cells)
}

/// Meshes every section of the first chunk with the others as its neighborhood.
fn build_with(
    chunks: &[TerrainChunkFixture],
    atlas: TerrainTextureAtlas,
    camera_position: [f32; 3],
) -> TerrainSectionMeshLayers {
    TerrainFixtureWorld::new(chunks, atlas).build_all(0, camera_position)
}

fn build(chunk: TerrainChunkFixture) -> TerrainSectionMeshLayers {
    build_with(&[chunk], TerrainTextureAtlas::unit(), [0.0; 3])
}

fn build_atlas(chunk: TerrainChunkFixture, atlas: TerrainTextureAtlas) -> TerrainSectionMeshLayers {
    build_with(&[chunk], atlas, [0.0; 3])
}

/// Each chunk meshed with all others as its neighborhood.
fn build_each(chunks: &[TerrainChunkFixture]) -> Vec<TerrainSectionMeshLayers> {
    let world = TerrainFixtureWorld::new(chunks, TerrainTextureAtlas::unit());
    (0..chunks.len())
        .map(|target| world.build_all(target, [0.0; 3]))
        .collect()
}

fn opaque(layers: &TerrainSectionMeshLayers) -> &TerrainMesh {
    &layers.opaque[0].mesh
}

fn cutout(layers: &TerrainSectionMeshLayers) -> &TerrainMesh {
    &layers.cutout[0].mesh
}

fn translucent(layers: &TerrainSectionMeshLayers) -> &TerrainMesh {
    &layers.translucent[0].mesh
}

fn cube(material: TerrainMaterialClass) -> TerrainCell {
    TerrainCell::with_texture(material, 0)
}

fn atlas(rects: Vec<TerrainUvRect>) -> TerrainTextureAtlas {
    TerrainTextureAtlas {
        rects,
        fallback_index: 0,
    }
}

fn rect(min: [f32; 2], max: [f32; 2]) -> TerrainUvRect {
    TerrainUvRect { min, max }
}

fn fluid_atlas() -> TerrainTextureAtlas {
    atlas(vec![
        TerrainUvRect::UNIT,
        rect([0.0, 0.0], [0.25, 0.25]),
        rect([0.5, 0.25], [1.0, 0.75]),
    ])
}

#[test]
fn meshes_single_opaque_block_as_six_faces() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(TerrainChunkFixture::new(2, -3, 0, 1, cells));
    let mesh = opaque(&layers);

    assert_eq!(layers.source_sections, 1);
    assert_eq!(mesh.opaque_faces, 6);
    assert_eq!(mesh.culled_faces, 0);
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
    let center = [1.5, 0.5, 2.5];
    for quad in cell_quads(mesh, [1, 0, 2]) {
        let centroid: [f32; 3] =
            std::array::from_fn(|axis| quad.iter().map(|v| v.position[axis]).sum::<f32>() / 4.0);
        let outward: f32 = (0..3)
            .map(|axis| (centroid[axis] - center[axis]) * quad[0].normal[axis])
            .sum();
        assert!(
            outward > 0.0,
            "opaque terrain triangles face outward: {quad:?}"
        );
    }
    assert_eq!(cell_vertices(mesh, [1, 0, 2]).len(), 24);
    assert!(mesh.vertices.iter().all(|v| {
        let world = v.world_position([32, 0, -48]);
        (33.0..=34.0).contains(&world[0])
            && (0.0..=1.0).contains(&world[1])
            && (-46.0..=-45.0).contains(&world[2])
    }));
}

#[test]
fn culls_internal_faces_between_opaque_blocks() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(2, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(1, cells));
    let mesh = opaque(&layers);

    assert_eq!(mesh.opaque_faces, 10);
    assert_eq!(mesh.culled_faces, 2);
    assert_eq!(mesh.vertices.len(), 40);
    assert_eq!(mesh.indices.len(), 60);
}

#[test]
fn culls_internal_faces_between_fluid_blocks() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    cells[cell_index(2, 0, 2, 1)] = water_cell(8);
    let layers = build(chunk(1, cells));
    let mesh = translucent(&layers);

    // Ten outward quads remain after the shared side is culled in both
    // directions. Vanilla also emits reversed top/side quads for the submerged
    // view: four open back faces per block add eight more quads.
    assert_eq!(mesh.translucent_faces, 18);
    assert_eq!(mesh.culled_faces, 2);
    assert_eq!(mesh.vertices.len(), 72);
    assert_eq!(mesh.indices.len(), 108);
}

#[test]
fn keeps_faces_between_different_fluid_kinds() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    cells[cell_index(2, 0, 2, 1)] = lava_cell(8);
    let layers = build(chunk(1, cells));

    // 6 box faces + 1 reversed top + 4 reversed sides per fluid.
    assert_eq!(translucent(&layers).translucent_faces, 22);
    assert_eq!(translucent(&layers).culled_faces, 0);
}

#[test]
fn fluid_box_extends_to_above_fluid_without_internal_top_face() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = water_cell(8);
    cells[cell_index(1, 1, 2, 2)] = water_cell(8);
    let layers = build(chunk(2, cells));
    let mesh = translucent(&layers);

    assert_eq!(mesh.translucent_faces, 19);
    assert_eq!(mesh.culled_faces, 2);
    assert!(cell_vertices(mesh, [1, 0, 2])
        .iter()
        .any(|vertex| vertex.position[1] == 1.0));
    assert!(face_vertices(mesh, [1, 0, 2], UP).is_empty());
    let upper_top = face_vertices(mesh, [1, 1, 2], UP);
    let upper_top_back: Vec<_> = face_vertices(mesh, [1, 1, 2], DOWN)
        .into_iter()
        .filter(|vertex| vertex.position[1] > 1.0)
        .collect();
    assert_eq!(upper_top.len(), 4);
    assert_eq!(upper_top_back.len(), 4);
    assert!(upper_top
        .iter()
        .chain(&upper_top_back)
        .all(|vertex| vertex.position[1] > 1.0 && vertex.position[1] < 1.875));
}

#[test]
fn fluid_faces_use_flat_vanilla_light_without_ambient_occlusion() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 3 * 16];
    cells[cell_index(1, 1, 2, 3)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(16))
            .with_light(TerrainLight { sky: 4, block: 2 })
            .with_ambient_occlusion(true);
    cells[cell_index(1, 2, 2, 3)] = TerrainCell::EMPTY
        .with_light(TerrainLight { sky: 15, block: 1 })
        .with_state_light(12, false);
    cells[cell_index(1, 0, 2, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 3, block: 10 });
    cells[cell_index(0, 2, 2, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    cells[cell_index(1, 2, 1, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    cells[cell_index(0, 2, 1, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    let layers = build(chunk(3, cells));
    let mesh = translucent(&layers);

    assert_face_light(mesh, [1, 1, 2], UP, [12.0 / 15.0, 1.0]);
    assert_face_light(mesh, [1, 1, 2], NORTH, [12.0 / 15.0, 1.0]);
    assert_face_light(mesh, [1, 1, 2], DOWN, [10.0 / 15.0, 4.0 / 15.0]);
    assert!(face_vertices(mesh, [1, 1, 2], UP)
        .iter()
        .all(|vertex| vertex.color == color(TerrainTint::WHITE, 1.0, 1.0)));
}

#[test]
fn fluid_top_face_averages_corner_heights_from_adjacent_fluids() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(8));
    cells[cell_index(1, 0, 1, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    let layers = build(chunk(1, cells));
    let top = face_vertices(translucent(&layers), [1, 0, 2], UP);

    assert_float_eq(vertex_at_xz(&top, 1.0, 2.0).position[1], 0.999);
    assert_float_eq(vertex_at_xz(&top, 2.0, 2.0).position[1], 0.999);
    assert_float_eq(vertex_at_xz(&top, 1.0, 3.0).position[1], 0.999);
    assert_float_eq(vertex_at_xz(&top, 2.0, 3.0).position[1], 1.0 / 6.0 - 0.001);
}

#[test]
fn fluid_top_height_uses_vanilla_own_height() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    for z in 1..=3 {
        for x in 0..=2 {
            cells[cell_index(x, 0, z, 1)] = water_cell(8);
        }
    }
    let layers = build(chunk(1, cells));
    let top = face_vertices(translucent(&layers), [1, 0, 2], UP);

    assert_eq!(top.len(), 4);
    assert!(top
        .iter()
        .all(|vertex| (vertex.position[1] - (8.0 / 9.0 - 0.001)).abs() < 3.0e-4));
}

#[test]
fn fluid_side_uvs_follow_corner_heights() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(8));
    cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    let layers = build(chunk(1, cells));
    let north = face_vertices(translucent(&layers), [1, 0, 2], NORTH);

    let east_bottom = vertex_at_approx(&north, [2.0, 0.001, 2.001]);
    let east_top = vertex_at_approx(&north, [2.0, 1.0 / 6.0 - 0.001, 2.001]);
    let west_top = vertex_at_approx(&north, [1.0, 0.999, 2.001]);
    let west_bottom = vertex_at_approx(&north, [1.0, 0.001, 2.001]);

    assert_eq!(east_bottom.uv, [0.5, 0.5]);
    assert_float_eq(east_top.uv[0], 0.5);
    assert_float_eq(east_top.uv[1], 5.0 / 12.0 + 0.0005);
    assert_uv_eq(west_top.uv, [0.0, 0.0005]);
    assert_eq!(west_bottom.uv, [0.0, 0.5]);
}

#[test]
fn fluid_down_face_uses_vanilla_bottom_offset() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    let layers = build(chunk(1, cells));
    // The downward-facing quads are the bottom and the reversed top.
    let down = face_vertices(translucent(&layers), [1, 0, 2], DOWN);

    assert_eq!(down.len(), 8);
    assert_eq!(
        down.iter()
            .filter(|vertex| (vertex.position[1] - 0.001).abs() < 0.0001)
            .count(),
        4
    );
}

#[test]
fn fluid_static_top_uses_still_texture_uvs() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    let layers = build_atlas(chunk(1, cells), fluid_atlas());
    let top = face_vertices(translucent(&layers), [1, 0, 2], UP);

    assert_eq!(vertex_at_xz(&top, 1.0, 2.0).uv, [0.0, 0.0]);
    assert_eq!(vertex_at_xz(&top, 2.0, 2.0).uv, [0.25, 0.0]);
    assert_eq!(vertex_at_xz(&top, 2.0, 3.0).uv, [0.25, 0.25]);
    assert_eq!(vertex_at_xz(&top, 1.0, 3.0).uv, [0.0, 0.25]);
}

#[test]
fn fluid_top_emits_backward_face_when_above_is_open() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    let layers = build(chunk(1, cells));
    let mesh = translucent(&layers);
    let top = face_vertices(mesh, [1, 0, 2], UP);
    let top_back: Vec<_> = face_vertices(mesh, [1, 0, 2], DOWN)
        .into_iter()
        .filter(|vertex| vertex.position[1] > 0.5)
        .collect();

    // Six box faces plus the reversed top and one reversed face per horizontal
    // side (Vanilla `FluidRenderer.addFace(..., !isOverlay)`): 6 + 1 + 4 = 11.
    assert_eq!(mesh.translucent_faces, 11);
    for face in [&top, &top_back] {
        assert_eq!(face.len(), 4);
        assert_eq!(face.iter().filter(|v| v.position[0] == 1.0).count(), 2);
        assert_eq!(face.iter().filter(|v| v.position[0] == 2.0).count(), 2);
    }
}

#[test]
fn fluid_side_face_emits_reversed_back_face_for_submerged_view() {
    // Vanilla `FluidRenderer.tesselate` draws horizontal fluid side faces with
    // `addFace(..., !isOverlay)`, re-emitting the four vertices in reverse. The
    // terrain pipeline culls back faces, so this reversed quad keeps the water
    // wall visible from inside the fluid.
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    let layers = build(chunk(1, cells));
    let north_side: Vec<_> = cell_quads(translucent(&layers), [1, 0, 2])
        .into_iter()
        .filter(|quad| quad.iter().all(|v| (v.position[2] - 2.001).abs() < 1.0e-4))
        .collect();

    // Front quad (emitted first) followed by its reversed back quad.
    assert_eq!(north_side.len(), 2);
    let (front, back) = (north_side[0], north_side[1]);
    for corner in 0..4 {
        assert_eq!(back[corner].position, front[3 - corner].position);
    }
    assert!(same_direction(front[0].normal, NORTH));
    assert!(same_direction(back[0].normal, SOUTH));
}

#[test]
fn water_side_overlay_neighbor_uses_overlay_texture_without_reversed_back_face() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8).with_fluid_overlay_texture(3);
    cells[cell_index(1, 0, 1, 1)] = cube(TerrainMaterialClass::Opaque)
        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6])
        .with_fluid_overlay_neighbor()
        .with_face_occlusion_shape_indices([0; 6]);
    let layers = build_atlas(
        chunk(1, cells),
        atlas(vec![
            TerrainUvRect::UNIT,
            TerrainUvRect::UNIT,
            rect([0.0, 0.0], [0.25, 0.25]),
            rect([0.5, 0.5], [1.0, 1.0]),
        ]),
    );
    let mesh = translucent(&layers);
    let north = north_side(mesh, [1, 0, 2]);

    assert_eq!(north.len(), 4);
    assert!(north
        .iter()
        .all(|vertex| vertex.uv[0] >= 0.5 && vertex.uv[1] >= 0.5));
    assert!(face_vertices(mesh, [1, 0, 2], SOUTH)
        .iter()
        .all(|vertex| vertex.position[2] > 2.5));
    assert_eq!(cell_quads(mesh, [1, 0, 2]).len(), 10);
}

#[test]
fn water_side_is_occluded_by_full_half_transparent_block_that_can_occlude() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8).with_fluid_overlay_texture(3);
    cells[cell_index(1, 0, 1, 1)] = cube(TerrainMaterialClass::Opaque)
        .with_face_transparency([TerrainTransparency::TRANSLUCENT; 6])
        .with_fluid_overlay_neighbor();
    let layers = build(chunk(1, cells));

    assert!(north_side(translucent(&layers), [1, 0, 2]).is_empty());
}

#[test]
fn fluid_flowing_top_uses_flowing_texture_and_rotated_uvs() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    cells[cell_index(2, 0, 2, 1)] = water_cell(4);
    let layers = build_atlas(chunk(1, cells), fluid_atlas());
    let top = face_vertices(translucent(&layers), [1, 0, 2], UP);

    assert_uv_eq(vertex_at_xz(&top, 1.0, 2.0).uv, [0.875, 0.375]);
    assert_uv_eq(vertex_at_xz(&top, 2.0, 2.0).uv, [0.875, 0.625]);
    assert_uv_eq(vertex_at_xz(&top, 2.0, 3.0).uv, [0.625, 0.625]);
    assert_uv_eq(vertex_at_xz(&top, 1.0, 3.0).uv, [0.625, 0.375]);
}

#[test]
fn fluid_top_flow_ignores_different_fluid_kind() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = water_cell(8);
    cells[cell_index(2, 0, 2, 1)] = lava_cell(4);
    let layers = build_atlas(chunk(1, cells), fluid_atlas());
    let top = face_vertices(translucent(&layers), [1, 0, 2], UP);

    assert_eq!(vertex_at_xz(&top, 1.0, 2.0).uv, [0.0, 0.0]);
    assert_eq!(vertex_at_xz(&top, 2.0, 2.0).uv, [0.25, 0.0]);
    assert_eq!(vertex_at_xz(&top, 2.0, 3.0).uv, [0.25, 0.25]);
    assert_eq!(vertex_at_xz(&top, 1.0, 3.0).uv, [0.0, 0.25]);
}

#[test]
fn culls_faces_between_adjacent_chunk_snapshots() {
    let meshes = build_each(&[
        single_block_chunk(0, 0, 15, 0, 2),
        single_block_chunk(1, 0, 0, 0, 2),
    ]);
    let sum = |field: fn(&TerrainMesh) -> usize| {
        meshes
            .iter()
            .map(|layers| field(opaque(layers)))
            .sum::<usize>()
    };

    assert_eq!(sum(|mesh| mesh.opaque_faces), 10);
    assert_eq!(sum(|mesh| mesh.culled_faces), 2);
    assert_eq!(sum(|mesh| mesh.vertices.len()), 40);
    assert_eq!(sum(|mesh| mesh.indices.len()), 60);
}

#[test]
fn incremental_chunk_mesh_uses_neighbors_without_rebuilding_them() {
    let layers = build_with(
        &[
            single_block_chunk(0, 0, 15, 0, 2),
            single_block_chunk(1, 0, 0, 0, 2),
        ],
        TerrainTextureAtlas::unit(),
        [0.0; 3],
    );

    assert_eq!(layers.source_sections, 1);
    assert_eq!(layers.opaque.len(), 1);
    assert_eq!(layers.cutout.len(), 1);
    assert_eq!(layers.translucent.len(), 1);
    assert_eq!(opaque(&layers).opaque_faces, 5);
    assert_eq!(opaque(&layers).culled_faces, 1);
    assert_eq!(cell_vertices(opaque(&layers), [15, 0, 2]).len(), 20);
}

#[test]
fn culls_fluid_faces_between_adjacent_chunk_snapshots() {
    let meshes = build_each(&[
        single_fluid_chunk(0, 0, 15, 0, 2),
        single_fluid_chunk(1, 0, 0, 0, 2),
    ]);

    assert_eq!(
        meshes
            .iter()
            .map(|layers| translucent(layers).translucent_faces)
            .sum::<usize>(),
        18
    );
    assert_eq!(
        meshes
            .iter()
            .map(|layers| translucent(layers).culled_faces)
            .sum::<usize>(),
        2
    );
}

#[test]
fn maps_face_uvs_into_texture_atlas_rect() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(TerrainMaterialClass::Opaque, 1);
    let layers = build_atlas(
        chunk(1, cells),
        atlas(vec![TerrainUvRect::UNIT, rect([0.25, 0.5], [0.5, 0.75])]),
    );
    let mesh = opaque(&layers);

    assert_eq!(mesh.vertices[0].uv_f32(), [0.25, 0.5]);
    assert_eq!(mesh.vertices[1].uv_f32(), [0.5, 0.5]);
    assert_eq!(mesh.vertices[2].uv_f32(), [0.5, 0.75]);
    assert_eq!(mesh.vertices[3].uv_f32(), [0.25, 0.75]);
}

#[test]
fn cube_faces_follow_vanilla_face_info_uv_vertices() {
    let layers = build(single_block_chunk(0, 0, 1, 0, 2));
    let mesh = opaque(&layers);

    for (normal, expected) in [
        (DOWN, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        (UP, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        (NORTH, [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
        (SOUTH, [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
        (WEST, [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
        (EAST, [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]),
    ] {
        let uvs: Vec<_> = face_vertices(mesh, [1, 0, 2], normal)
            .iter()
            .map(|vertex| vertex.uv)
            .collect();
        assert_eq!(uvs, expected);
    }
}

#[test]
fn maps_different_faces_to_different_texture_rects() {
    let mut cell = cube(TerrainMaterialClass::Opaque);
    cell.texture_indices = [1, 2, 0, 0, 0, 0];
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cell;
    let layers = build_atlas(
        chunk(1, cells),
        atlas(vec![
            TerrainUvRect::UNIT,
            rect([0.0, 0.0], [0.25, 0.25]),
            rect([0.5, 0.5], [0.75, 0.75]),
        ]),
    );

    assert_eq!(opaque(&layers).vertices[0].uv_f32(), [0.0, 0.0]);
    assert_eq!(opaque(&layers).vertices[4].uv_f32(), [0.5, 0.5]);
}

#[test]
fn cutout_cube_meshes_into_the_cutout_layer() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).vertices.len(), 0);
    assert_eq!(cutout(&layers).cutout_faces, 6);
    assert_eq!(cutout(&layers).vertices.len(), 24);
    assert_eq!(cutout(&layers).indices.len(), 36);
}

#[test]
fn cross_cutout_mesh_emits_vanilla_cross_quads() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: false,
            light_emission: 0,
        },
    );
    let layers = build(chunk(1, cells));
    let mesh = cutout(&layers);

    assert_eq!(opaque(&layers).vertices.len(), 0);
    assert_eq!(mesh.cutout_faces, 4);
    assert_eq!(mesh.culled_faces, 0);
    assert_eq!(mesh.vertices.len(), 16);
    assert_eq!(mesh.indices.len(), 24);
    assert!(mesh.vertices.iter().all(|vertex| vertex.color == [255; 4]));
    // `block/cross` planes run from 0.8 to 15.2 across the diagonal, not corner to corner.
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 3.0e-4);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| near(vertex.local_position(), [1.05, 0.0, 2.05])));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| near(vertex.local_position(), [1.95, 1.0, 2.95])));
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| (1.05 - 3.0e-4..=1.95 + 3.0e-4).contains(&vertex.local_position()[0])));
}

#[test]
fn cross_cutout_faces_are_not_neighbor_culled() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: true,
            light_emission: 0,
        },
    );
    cells[cell_index(2, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(cutout(&layers).cutout_faces, 4);
    assert_eq!(cutout(&layers).culled_faces, 0);
}

#[test]
fn cross_layers_preserve_emissive_light() {
    let cross = |light_emission| TerrainCross {
        texture_indices: [0; 6],
        tint: [TerrainTint::WHITE; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
        shade: false,
        light_emission,
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell {
        material: TerrainMaterialClass::Cutout,
        is_air: false,
        light: TerrainLight { sky: 4, block: 2 },
        render_shape: TerrainRenderShape::Crosses(std::sync::Arc::from(vec![cross(0), cross(15)])),
        ..TerrainCell::EMPTY
    };
    let layers = build(chunk(1, cells));
    let vertices = decode(cutout(&layers));

    assert_eq!(cutout(&layers).cutout_faces, 8);
    assert_eq!(vertices.len(), 32);
    assert_eq!(vertices[0].light, [2.0 / 15.0, 4.0 / 15.0]);
    // `LightCoordsUtil.lightCoordsWithEmission` raises both channels.
    assert_eq!(vertices[16].light, [1.0, 1.0]);
    assert!(vertices.iter().all(|vertex| vertex.color == [255; 4]));
}

#[test]
fn box_model_mesh_uses_bounds_and_face_uv_crop() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 1, slab_box_shape());
    let layers = build_atlas(
        chunk(1, cells),
        atlas(vec![TerrainUvRect::UNIT, rect([0.25, 0.25], [0.75, 0.75])]),
    );
    let mesh = opaque(&layers);

    assert_eq!(mesh.opaque_faces, 6);
    assert_eq!(mesh.vertices.len(), 24);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.local_position()[1] == 0.5));
    assert!(!mesh
        .vertices
        .iter()
        .any(|vertex| vertex.local_position()[1] == 1.0));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.uv_f32() == [0.25, 0.5]));
}

#[test]
fn box_model_mesh_rotates_face_uv_crop() {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    face_uvs[TerrainFace::Down.index()] = [4, 4, 8, 12];
    let mut face_uv_rotations = [0; 6];
    face_uv_rotations[TerrainFace::Down.index()] = 1;
    let shape = TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations,
        face_shade: [false; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(TerrainMaterialClass::Opaque, 1, shape);
    let layers = build(chunk(1, cells));
    let mesh = opaque(&layers);

    assert_eq!(mesh.vertices[0].uv_f32(), [0.25, 0.75]);
    assert_eq!(mesh.vertices[0].color, [255; 4]);
    assert_eq!(mesh.vertices[1].uv_f32(), [0.25, 0.25]);
    assert_eq!(mesh.vertices[2].uv_f32(), [0.5, 0.25]);
    assert_eq!(mesh.vertices[3].uv_f32(), [0.5, 0.75]);
}

#[test]
fn box_model_vertical_face_uvs_follow_vanilla_face_info_vertices() {
    let mut face_present = [false; 6];
    face_present[TerrainFace::North.index()] = true;
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    face_uvs[TerrainFace::North.index()] = [2, 3, 10, 15];
    let shape = TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present,
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [false; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(TerrainMaterialClass::Opaque, 1, shape);
    let layers = build(chunk(1, cells));
    let mesh = opaque(&layers);

    assert_eq!(mesh.vertices.len(), 4);
    assert_eq!(mesh.vertices[0].uv_f32(), [2.0 / 16.0, 15.0 / 16.0]);
    assert_eq!(mesh.vertices[1].uv_f32(), [2.0 / 16.0, 3.0 / 16.0]);
    assert_eq!(mesh.vertices[2].uv_f32(), [10.0 / 16.0, 3.0 / 16.0]);
    assert_eq!(mesh.vertices[3].uv_f32(), [10.0 / 16.0, 15.0 / 16.0]);
}

#[test]
fn box_model_culls_only_faces_marked_by_cullface() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, slab_box_shape())
            .with_face_occlusion_shape_indices(bottom_slab_face_shapes());
    cells[cell_index(1, 0, 1, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(1, 0, 3, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(1, cells));

    // The slab's north/south faces cull against the full cubes, but the cubes keep
    // their faces toward the slab: its half-height sides do not cover a full face
    // (`Shapes.joinIsNotEmpty(cubeFace, slabHalfFace, ONLY_FIRST)`).
    assert_eq!(opaque(&layers).culled_faces, 2);
    assert_eq!(opaque(&layers).opaque_faces, 16);
}

#[test]
fn face_culling_consumes_vanilla_face_shape_palette_not_render_cuboids() {
    let masks = super::super::vanilla_face_occlusion_masks();
    assert_eq!(masks[0], [0; 4]);
    assert_eq!(masks[1], [u64::MAX; 4]);
    let partial = masks
        .iter()
        .position(|mask| *mask != [0; 4] && *mask != [u64::MAX; 4])
        .unwrap() as u8;

    assert!(super::super::face_occlusion_culls(partial, 1));
    assert!(!super::super::face_occlusion_culls(1, partial));
    assert!(!super::super::face_occlusion_culls(0, partial));
    assert!(super::super::face_occlusion_culls(0, 1));
    assert!(!super::super::face_occlusion_culls(1, u8::MAX));
}

#[test]
fn fluid_face_occlusion_uses_vanilla_height_and_top_surface_ordering() {
    let lower_half = face_shape_index([u64::MAX, u64::MAX, 0, 0]);
    let fifteen_sixteenths =
        face_shape_index([u64::MAX, u64::MAX, u64::MAX, 0x0000_ffff_ffff_ffff]);
    let occludes = |own: [u8; 6], face, neighbor: &TerrainCell, height| {
        super::super::fluid_face_occlusion_culls(own, face, &neighbor.block_state(), height)
    };
    let mut neighbor = TerrainCell::EMPTY;
    neighbor.face_occlusion_shape_indices[TerrainFace::South.index()] = lower_half;
    assert!(!occludes([0; 6], TerrainFace::North, &neighbor, 8.0 / 9.0));

    neighbor.face_occlusion_shape_indices[TerrainFace::South.index()] = fifteen_sixteenths;
    assert!(occludes([0; 6], TerrainFace::North, &neighbor, 8.0 / 9.0));

    neighbor.face_occlusion_shape_indices = [1; 6];
    assert!(!occludes([0; 6], TerrainFace::Up, &neighbor, 8.0 / 9.0));
    assert!(occludes([0; 6], TerrainFace::Up, &neighbor, 1.0));
    let mut own_shapes = [0; 6];
    own_shapes[TerrainFace::Up.index()] = 1;
    assert!(occludes(
        own_shapes,
        TerrainFace::Up,
        &TerrainCell::EMPTY,
        8.0 / 9.0
    ));
    assert!(occludes([0; 6], TerrainFace::Down, &neighbor, 1.0));
}

#[test]
fn stacked_slabs_cull_the_filled_middle_boundary() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, top_slab_box_shape())
            .with_face_occlusion_shape_indices(top_slab_face_shapes());
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, slab_box_shape())
            .with_face_occlusion_shape_indices(bottom_slab_face_shapes());
    let layers = build(chunk(2, cells));
    let mesh = opaque(&layers);

    assert_eq!(mesh.culled_faces, 2);
    assert_eq!(mesh.opaque_faces, 10);
    assert!(!decode(mesh).iter().any(|vertex| {
        vertex.position[1] == 1.0
            && (same_direction(vertex.normal, UP) || same_direction(vertex.normal, DOWN))
    }));
}

#[test]
fn same_orientation_slabs_cull_identical_partial_side_faces() {
    // Vanilla compares both exact face shapes with `ONLY_FIRST`: identical
    // half-height slab sides cover each other although neither is full.
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    for x in [1, 2] {
        cells[cell_index(x, 0, 2, 1)] =
            TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, slab_box_shape())
                .with_face_occlusion_shape_indices(bottom_slab_face_shapes());
    }
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).culled_faces, 2);
    assert_eq!(opaque(&layers).opaque_faces, 10);
}

#[test]
fn stairs_full_back_face_occludes_neighbor_cube_via_box_union() {
    // The stair's north back is full only via the union of its two boxes.
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, stair_boxes_shape());
    cells[cell_index(1, 0, 1, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(1, cells));
    let mesh = opaque(&layers);

    assert_eq!(cell_vertices(mesh, [1, 0, 1]).len(), 20);
    assert!(face_vertices(mesh, [1, 0, 1], SOUTH).is_empty());
    // The full cube also occludes both of the stair's north faces (bottom + step).
    assert!(face_vertices(mesh, [1, 0, 2], NORTH).is_empty());
}

#[test]
fn cutout_slab_neighbor_does_not_occlude_opaque_face() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] =
        TerrainCell::with_shape(TerrainMaterialClass::Cutout, 0, top_slab_box_shape());
    cells[cell_index(1, 1, 2, 2)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(2, cells));

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(opaque(&layers).culled_faces, 0);
}

fn translucent_pair(left: TerrainSkipRendering, right: TerrainSkipRendering) -> Vec<TerrainCell> {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        cube(TerrainMaterialClass::Translucent).with_skip_rendering(left);
    let mut right_cell = cube(TerrainMaterialClass::Translucent).with_skip_rendering(right);
    // A distinct state: only the skip-rendering key may decide the cull.
    right_cell.texture_indices = [1; 6];
    cells[cell_index(2, 0, 2, 1)] = right_cell;
    cells
}

#[test]
fn same_block_translucent_cubes_skip_shared_face() {
    let skip = same_block_all_faces_skip(7);
    let layers = build(chunk(1, translucent_pair(skip, skip)));
    let mesh = translucent(&layers);

    assert_eq!(mesh.translucent_faces, 10);
    assert_eq!(mesh.culled_faces, 2);
    assert!(face_vertices(mesh, [1, 0, 2], EAST).is_empty());
    assert!(face_vertices(mesh, [2, 0, 2], WEST).is_empty());
}

#[test]
fn different_translucent_block_keys_keep_shared_face() {
    let layers = build(chunk(
        1,
        translucent_pair(same_block_all_faces_skip(7), same_block_all_faces_skip(8)),
    ));

    assert_eq!(translucent(&layers).translucent_faces, 12);
    assert_eq!(translucent(&layers).culled_faces, 0);
}

#[test]
fn fast_leaves_skip_shared_face_across_leaf_kinds() {
    let fast_leaves = TerrainSkipRendering {
        culls_with_leaves: true,
        ..TerrainSkipRendering::NONE
    };
    let neighbor = TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: true,
            light_emission: 0,
        },
    )
    .with_skip_rendering(fast_leaves)
    .block_state();

    for (skip, culls) in [(fast_leaves, true), (TerrainSkipRendering::NONE, false)] {
        assert_eq!(
            culls_face_between_cells(
                [0; 6],
                skip,
                TerrainMaterialClass::Cutout,
                None,
                TerrainFace::East,
                &neighbor,
            ),
            culls
        );
    }
}

#[test]
fn same_block_vertical_skip_only_culls_up_down_faces() {
    let skip = same_block_vertical_faces_skip(9);
    let mut vertical_cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    vertical_cells[cell_index(1, 0, 2, 2)] =
        cube(TerrainMaterialClass::Translucent).with_skip_rendering(skip);
    vertical_cells[cell_index(1, 1, 2, 2)] =
        cube(TerrainMaterialClass::Translucent).with_skip_rendering(skip);
    let vertical = build(chunk(2, vertical_cells));
    assert_eq!(translucent(&vertical).translucent_faces, 10);
    assert_eq!(translucent(&vertical).culled_faces, 2);

    let horizontal = build(chunk(1, translucent_pair(skip, skip)));
    assert_eq!(translucent(&horizontal).translucent_faces, 12);
    assert_eq!(translucent(&horizontal).culled_faces, 0);
}

#[test]
fn iron_bars_skip_connected_same_tag_faces() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout)
        .with_skip_rendering(iron_bars_skip(20, true, false, false, false, true));
    cells[cell_index(2, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout)
        .with_skip_rendering(iron_bars_skip(21, true, false, false, true, false));
    let layers = build(chunk(1, cells));
    let mesh = cutout(&layers);

    assert_eq!(mesh.cutout_faces, 10);
    assert_eq!(mesh.culled_faces, 2);
    assert!(face_vertices(mesh, [1, 0, 2], EAST).is_empty());
    assert!(face_vertices(mesh, [2, 0, 2], WEST).is_empty());
}

#[test]
fn iron_bars_tag_branch_uses_neighbor_property_presence_not_block_class() {
    let current = iron_bars_skip(20, true, false, false, false, true);
    let tagged = TerrainSkipRendering {
        same_block_key: 99,
        bars_tag: true,
        west: true,
        connection_property_mask: 1 << 2,
        ..TerrainSkipRendering::NONE
    };
    let culls = |neighbor_skip| {
        culls_face_between_cells(
            [0; 6],
            current,
            TerrainMaterialClass::Cutout,
            None,
            TerrainFace::East,
            &cube(TerrainMaterialClass::Cutout)
                .with_skip_rendering(neighbor_skip)
                .block_state(),
        )
    };

    assert!(culls(tagged));
    assert!(!culls(TerrainSkipRendering {
        connection_property_mask: 0,
        ..tagged
    }));
}

#[test]
fn iron_bars_keep_unconnected_horizontal_same_block_face() {
    let skip = iron_bars_skip(20, false, false, false, false, false);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout).with_skip_rendering(skip);
    cells[cell_index(2, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout).with_skip_rendering(skip);
    let layers = build(chunk(1, cells));

    assert_eq!(cutout(&layers).cutout_faces, 12);
    assert_eq!(cutout(&layers).culled_faces, 0);
}

#[test]
fn cross_neighbor_does_not_occlude_opaque_face() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(2, 0, 2, 1)] = TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: false,
            light_emission: 0,
        },
    );
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(opaque(&layers).culled_faces, 0);
}

#[test]
fn cross_chunk_slab_neighbor_keeps_partial_face() {
    // A bottom slab at the west edge of chunk (1,0) faces a full cube at the east
    // edge of chunk (0,0): the half-height slab side does not cull the cube face.
    let mut right_cells = vec![TerrainCell::EMPTY; 16 * 16];
    right_cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, slab_box_shape())
            .with_face_occlusion_shape_indices(bottom_slab_face_shapes());
    let layers = build_with(
        &[
            single_block_chunk(0, 0, 15, 0, 2),
            TerrainChunkFixture::new(1, 0, 0, 1, right_cells),
        ],
        TerrainTextureAtlas::unit(),
        [0.0; 3],
    );

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(opaque(&layers).culled_faces, 0);
    assert_eq!(face_vertices(opaque(&layers), [15, 0, 2], EAST).len(), 4);
}

#[test]
fn box_model_uses_cullface_direction_not_rendered_face_direction() {
    let mut face_present = [false; 6];
    face_present[TerrainFace::North.index()] = true;
    let mut face_cull = [None; 6];
    face_cull[TerrainFace::North.index()] = Some(TerrainFace::South);
    let shape = TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present,
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull,
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(TerrainMaterialClass::Opaque, 0, shape);
    cells[cell_index(1, 0, 3, 1)] = cube(TerrainMaterialClass::Opaque);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).culled_faces, 2);
    assert!(opaque(&layers)
        .vertices
        .iter()
        .all(|vertex| vertex.local_position()[2] >= 3.0));
}

fn plain_box(from: [u8; 3], to: [u8; 3], texture_index: u32, tint: TerrainTint) -> TerrainBox {
    TerrainBox {
        from,
        to,
        face_present: [true; 6],
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        texture_indices: [texture_index; 6],
        tint: [tint; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn solid_boxes(boxes: Vec<TerrainBox>) -> TerrainCell {
    TerrainCell::with_shape(
        TerrainMaterialClass::Opaque,
        0,
        TerrainRenderShape::Boxes(std::sync::Arc::from(boxes)),
    )
    .with_ambient_light_facts(true, 15, 51)
}

#[test]
fn multi_box_model_skips_absent_faces() {
    let mut upper = plain_box([8, 8, 0], [16, 16, 16], 0, TerrainTint::WHITE);
    upper.face_present[TerrainFace::Down.index()] = false;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = solid_boxes(vec![
        plain_box([0, 0, 0], [16, 8, 16], 0, TerrainTint::WHITE),
        upper,
    ]);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 11);
    assert_eq!(opaque(&layers).vertices.len(), 44);
    assert_eq!(opaque(&layers).indices.len(), 66);
}

#[test]
fn empty_multi_box_model_emits_no_geometry_or_occlusion() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(2, 0, 2, 1)] = TerrainCell::with_shape(
        TerrainMaterialClass::Opaque,
        0,
        TerrainRenderShape::Boxes(std::sync::Arc::from(Vec::new())),
    )
    .with_face_occlusion_shape_indices([0; 6]);
    let layers = build(chunk(1, cells));
    let mesh = opaque(&layers);

    assert_eq!(mesh.opaque_faces, 6);
    assert_eq!(mesh.culled_faces, 0);
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
    assert_eq!(cell_vertices(mesh, [1, 0, 2]).len(), 24);
}

#[test]
fn boxes_use_per_box_texture_and_tint() {
    let grass_tint = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
    let foliage_tint = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = solid_boxes(vec![
        plain_box([0, 0, 0], [16, 8, 16], 1, grass_tint),
        plain_box([0, 8, 0], [16, 16, 16], 2, foliage_tint),
    ]);
    let layers = build_atlas(
        chunk(1, cells),
        atlas(vec![
            TerrainUvRect::UNIT,
            rect([0.25, 0.25], [0.5, 0.5]),
            rect([0.5, 0.5], [0.75, 0.75]),
        ]),
    );
    let mesh = opaque(&layers);

    // Both sampled vertices sit on down faces (Vanilla shade 0.5). The full-collision block
    // makes the upper box's inner down face `faceCubic`, so it samples below the block too.
    assert_eq!(mesh.vertices[0].uv_f32(), [0.25, 0.25]);
    assert_eq!(mesh.vertices[0].color, color(grass_tint, 0.5, 1.0));
    assert_eq!(mesh.vertices[24].uv_f32(), [0.5, 0.5]);
    assert_eq!(mesh.vertices[24].color, color(foliage_tint, 0.5, 1.0));
}

#[test]
fn mesh_vertices_carry_terrain_light() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        cube(TerrainMaterialClass::Opaque).with_light(TerrainLight { sky: 9, block: 6 });
    let layers = build(chunk(1, cells));

    assert_eq!(decode(opaque(&layers))[0].light, [6.0 / 15.0, 9.0 / 15.0]);
}

#[test]
fn mesh_vertices_apply_state_emission_and_emissive_rendering() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 3 * 16];
    for (x, y, z) in [
        (1, 0, 2),
        (1, 2, 2),
        (1, 1, 1),
        (1, 1, 3),
        (0, 1, 2),
        (2, 1, 2),
    ] {
        cells[cell_index(x, y, z, 3)] =
            TerrainCell::EMPTY.with_light(TerrainLight { sky: 9, block: 2 });
    }
    cells[cell_index(1, 1, 2, 3)] = cube(TerrainMaterialClass::Opaque)
        .with_light(TerrainLight { sky: 9, block: 2 })
        .with_state_light(7, false)
        .with_ambient_occlusion(false);
    cells[cell_index(3, 1, 2, 3)] = cube(TerrainMaterialClass::Opaque)
        .with_light(TerrainLight { sky: 3, block: 1 })
        .with_state_light(3, true)
        .with_ambient_occlusion(false);
    let layers = build(chunk(3, cells));
    let mesh = opaque(&layers);

    let emitting = cell_vertices(mesh, [1, 1, 2]);
    let emissive = cell_vertices(mesh, [3, 1, 2]);
    assert_eq!((emitting.len(), emissive.len()), (24, 24));
    assert!(emitting
        .iter()
        .all(|vertex| vertex.light == [7.0 / 15.0, 9.0 / 15.0]));
    assert!(emissive.iter().all(|vertex| vertex.light == [1.0, 1.0]));
}

#[test]
fn ambient_cubic_face_offset_samples_neighbor_brightness_and_state() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque)
        .with_light(TerrainLight { sky: 2, block: 1 })
        .with_state_light(3, true);
    for x in 0..=2 {
        for z in 1..=3 {
            cells[cell_index(x, 1, z, 2)] =
                TerrainCell::EMPTY.with_light(TerrainLight { sky: 9, block: 4 });
        }
    }
    let world = TerrainFixtureWorld::new(&[chunk(2, cells)], TerrainTextureAtlas::unit());
    let lookup = TerrainChunkLookup::new(&world.snapshots[0], [], &world.table);

    let (lights, _) = quad_lighting(
        TerrainFace::Up,
        &FACES[1].corners,
        false,
        Some(TerrainFace::Up),
        0,
        true,
        false,
        (1, 0, 2),
        TerrainLight { sky: 2, block: 1 },
        3,
        true,
        &lookup,
    );

    assert_eq!(lights, [4 << 4 | 9 << 20; 4]);
}

#[test]
fn flat_cubic_face_keeps_current_state_at_neighbor_brightness_position() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque)
        .with_light(TerrainLight { sky: 2, block: 1 })
        .with_state_light(7, false);
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 9, block: 4 });
    let world = TerrainFixtureWorld::new(&[chunk(2, cells)], TerrainTextureAtlas::unit());
    let lookup = TerrainChunkLookup::new(&world.snapshots[0], [], &world.table);

    let (lights, _) = quad_lighting(
        TerrainFace::Up,
        &FACES[1].corners,
        false,
        Some(TerrainFace::Up),
        0,
        false,
        false,
        (1, 0, 2),
        TerrainLight { sky: 2, block: 1 },
        7,
        false,
        &lookup,
    );

    assert_eq!(lights, [7 << 4 | 9 << 20; 4]);
}

#[test]
fn ambient_occlusion_smooths_cubic_face_vertex_light() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] =
        cube(TerrainMaterialClass::Opaque).with_light(TerrainLight { sky: 1, block: 1 });
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 12, block: 12 });
    cells[cell_index(0, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 8, block: 4 });
    cells[cell_index(1, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 4, block: 8 });
    cells[cell_index(0, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 0 });
    let layers = build(chunk(2, cells));
    let top = face_vertices(opaque(&layers), [1, 0, 2], UP);

    assert_float_eq(vertex_at(&top, [1.0, 1.0, 2.0]).light[0], 9.0 / 15.0);
    assert_float_eq(vertex_at(&top, [1.0, 1.0, 2.0]).light[1], 9.0 / 15.0);
}

#[test]
fn ambient_occlusion_flag_disables_light_smoothing() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque)
        .with_light(TerrainLight { sky: 5, block: 6 })
        .with_ambient_occlusion(false);
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 5, block: 6 });
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] =
            TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 15 });
    }
    let layers = build(chunk(2, cells));

    assert!(face_vertices(opaque(&layers), [1, 0, 2], UP)
        .iter()
        .all(|vertex| vertex.light == [6.0 / 15.0, 5.0 / 15.0]));
}

#[test]
fn mesh_vertices_carry_face_tint() {
    let down_tint = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
    let up_tint = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
    let mut face_tints = [TerrainTint::WHITE; 6];
    face_tints[TerrainFace::Down.index()] = down_tint;
    face_tints[TerrainFace::Up.index()] = up_tint;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque).with_tint(face_tints);
    let layers = build(chunk(1, cells));

    assert_eq!(
        opaque(&layers).vertices[0].color,
        color(down_tint, 0.5, 1.0)
    );
    assert_eq!(opaque(&layers).vertices[4].color, color(up_tint, 1.0, 1.0));
}

#[test]
fn terrain_diffuse_vertices_carry_vanilla_default_cardinal_shade() {
    let layers = build(single_block_chunk(0, 0, 1, 0, 2));

    for (normal, shade) in [
        (DOWN, 0.5),
        (UP, 1.0),
        (NORTH, 0.8),
        (SOUTH, 0.8),
        (WEST, 0.6),
        (EAST, 0.6),
    ] {
        assert_face_shade(opaque(&layers), normal, shade);
    }
}

#[test]
fn terrain_diffuse_uses_vanilla_nether_cardinal_shade() {
    // Vanilla `CardinalLighting.NETHER = (0.9, 0.9, 0.8, 0.8, 0.6, 0.6)`.
    let layers = build(
        single_block_chunk(0, 0, 1, 0, 2).with_cardinal_lighting(TerrainCardinalLighting::Nether),
    );

    for (normal, shade) in [
        (DOWN, 0.9),
        (UP, 0.9),
        (NORTH, 0.8),
        (SOUTH, 0.8),
        (WEST, 0.6),
        (EAST, 0.6),
    ] {
        assert_face_shade(opaque(&layers), normal, shade);
    }
}

#[test]
fn ambient_occlusion_darkens_cubic_face_corners_from_outer_neighbors() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque);
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] =
            cube(TerrainMaterialClass::Opaque).with_ambient_light_facts(true, 15, 51);
    }
    let layers = build(chunk(2, cells));
    let top = face_vertices(opaque(&layers), [1, 0, 2], UP);

    assert_eq!(top.len(), 4);
    assert_eq!(
        vertex_at(&top, [1.0, 1.0, 2.0]).color,
        color(TerrainTint::WHITE, 1.0, 0.4)
    );
    assert_eq!(vertex_at(&top, [2.0, 1.0, 3.0]).color, [255; 4]);
}

#[test]
fn light_emitting_states_skip_ambient_occlusion() {
    // `ModelBlockRenderer.tesselateBlock` takes the flat path when `getLightEmission() != 0`
    // (glowstone, sea lanterns), whatever the model's `ambientocclusion`.
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque).with_state_light(15, false);
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] =
            cube(TerrainMaterialClass::Opaque).with_ambient_light_facts(true, 15, 51);
    }
    let layers = build(chunk(2, cells));
    let top = face_vertices(opaque(&layers), [1, 0, 2], UP);

    assert_eq!(top.len(), 4);
    assert!(top.iter().all(|vertex| vertex.color == [255; 4]));
}

#[test]
fn ambient_occlusion_uses_shade_brightness_independently_of_view_blocking() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(2, 0, 2, 2)] = cube(TerrainMaterialClass::Opaque);
    for z in 1..=3 {
        for x in 1..=3 {
            if (x, z) != (2, 2) {
                cells[cell_index(x, 1, z, 2)] =
                    cube(TerrainMaterialClass::Cutout).with_ambient_light_facts(false, 1, 51);
            }
        }
    }
    let world = TerrainFixtureWorld::new(&[chunk(2, cells)], TerrainTextureAtlas::unit());
    let lookup = TerrainChunkLookup::new(&world.snapshots[0], [], &world.table);

    let (_, ambient) = up_face_smooth_lighting(&lookup);

    for value in ambient {
        assert_float_eq(value, 0.4);
    }
}

#[test]
fn ambient_occlusion_queries_corner_blockers_beyond_side_brightness_samples() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 3 * 16];
    cells[cell_index(2, 0, 2, 3)] = cube(TerrainMaterialClass::Opaque);
    for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let (x, z) = ((2 + dx) as usize, (2 + dz) as usize);
        cells[cell_index(x, 1, z, 3)] =
            cube(TerrainMaterialClass::Translucent).with_ambient_light_facts(false, 0, 255);
        cells[cell_index(x, 2, z, 3)] =
            cube(TerrainMaterialClass::Opaque).with_ambient_light_facts(true, 15, 51);
    }
    for (dx, dz) in [(-1, -1), (-1, 1), (1, -1), (1, 1)] {
        cells[cell_index((2 + dx) as usize, 1, (2 + dz) as usize, 3)] =
            cube(TerrainMaterialClass::Cutout).with_ambient_light_facts(false, 1, 51);
    }
    let world = TerrainFixtureWorld::new(&[chunk(3, cells)], TerrainTextureAtlas::unit());
    let lookup = TerrainChunkLookup::new(&world.snapshots[0], [], &world.table);

    let (_, ambient) = up_face_smooth_lighting(&lookup);

    for value in ambient {
        assert_float_eq(value, 1.0);
    }
}

#[test]
fn ambient_occlusion_flag_disables_corner_darkening() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] =
        cube(TerrainMaterialClass::Opaque).with_ambient_occlusion(false);
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] = cube(TerrainMaterialClass::Opaque);
    }
    let layers = build(chunk(2, cells));
    let top = face_vertices(opaque(&layers), [1, 0, 2], UP);

    assert_eq!(top.len(), 4);
    assert!(top.iter().all(|vertex| vertex.color == [255; 4]));
}

#[test]
fn layers_split_opaque_cutout_and_translucent_faces() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(3, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout);
    cells[cell_index(5, 0, 2, 1)] = cube(TerrainMaterialClass::Translucent);
    cells[cell_index(7, 0, 2, 1)] = cube(TerrainMaterialClass::Fluid);
    let layers = build(chunk(1, cells));
    let faces = |mesh: &TerrainMesh| (mesh.opaque_faces, mesh.cutout_faces, mesh.translucent_faces);

    assert_eq!(layers.source_sections, 1);
    assert_eq!(faces(opaque(&layers)), (6, 0, 0));
    assert_eq!(faces(cutout(&layers)), (0, 6, 0));
    assert_eq!(faces(translucent(&layers)), (0, 0, 12));
}

#[test]
fn cutout_blocks_do_not_occlude_neighbor_opaque_faces() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cube(TerrainMaterialClass::Opaque);
    cells[cell_index(2, 0, 2, 1)] = cube(TerrainMaterialClass::Cutout);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(opaque(&layers).culled_faces, 0);
    assert_eq!(cutout(&layers).cutout_faces, 5);
    assert_eq!(cutout(&layers).culled_faces, 1);
}

fn single_face_transparency(transparency: TerrainTransparency) -> TerrainCell {
    let mut face_transparency = [TerrainTransparency::OPAQUE; 6];
    face_transparency[TerrainFace::Up.index()] = transparency;
    cube(TerrainMaterialClass::Opaque).with_face_transparency(face_transparency)
}

#[test]
fn forced_translucent_cube_faces_emit_in_translucent_layer() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = single_face_transparency(TerrainTransparency::TRANSLUCENT);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 5);
    assert_eq!(translucent(&layers).translucent_faces, 1);
    assert_eq!(cutout(&layers).vertices.len(), 0);
}

#[test]
fn transparent_cube_faces_emit_in_cutout_layer() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = single_face_transparency(TerrainTransparency {
        has_transparent: true,
        has_translucent: false,
    });
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 5);
    assert_eq!(cutout(&layers).cutout_faces, 1);
    assert_eq!(translucent(&layers).vertices.len(), 0);
}

fn near_and_far_translucent_quads() -> TerrainCell {
    let near = TerrainQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [16.0, 0.0, 0.0],
            [16.0, 16.0, 0.0],
            [0.0, 16.0, 0.0],
        ],
        normal: [0.0, 0.0, -1.0],
        uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        cull: None,
        texture_index: 0,
        tint: TerrainTint::WHITE,
        transparency: TerrainTransparency::TRANSLUCENT,
        shade: true,
        light_emission: 0,
    };
    let far = TerrainQuad {
        corners: [
            [0.0, 0.0, 16.0],
            [16.0, 0.0, 16.0],
            [16.0, 16.0, 16.0],
            [0.0, 16.0, 16.0],
        ],
        normal: [0.0, 0.0, 1.0],
        ..near
    };
    TerrainCell::with_shape(
        TerrainMaterialClass::Opaque,
        0,
        TerrainRenderShape::Quads(std::sync::Arc::from(vec![near, far])),
    )
}

#[test]
fn translucent_layer_sorts_quad_indices_by_distance_from_camera_like_vanilla() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = near_and_far_translucent_quads();
    let layers = build_with(
        &[chunk(1, cells)],
        TerrainTextureAtlas::unit(),
        [1.5, 0.5, -4.0],
    );

    assert_eq!(
        translucent(&layers).indices,
        vec![4, 5, 6, 4, 6, 7, 0, 2, 1, 0, 3, 2]
    );
}

#[test]
fn translucent_layer_sort_uses_world_camera_equivalent_to_vanilla_section_local_sorting() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = near_and_far_translucent_quads();
    let section_origin = [48.0, 0.0, -32.0];
    let camera_local = [1.5, 0.5, -4.0];
    let layers = build_with(
        &[TerrainChunkFixture::new(3, -2, 0, 1, cells)],
        TerrainTextureAtlas::unit(),
        std::array::from_fn(|axis| section_origin[axis] + camera_local[axis]),
    );

    assert_eq!(
        translucent(&layers).indices,
        vec![4, 5, 6, 4, 6, 7, 0, 2, 1, 0, 3, 2]
    );
}

#[test]
fn quad_shape_emits_custom_vertices_in_texture_layer() {
    let tint = TerrainTint::from_rgb_u8(128, 255, 64);
    let quad = TerrainQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [16.0, 0.0, 0.0],
            [16.0, 16.0, 0.0],
            [0.0, 16.0, 0.0],
        ],
        normal: [0.0, 0.0, -1.0],
        uvs: [[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]],
        cull: None,
        texture_index: 0,
        tint,
        transparency: TerrainTransparency {
            has_transparent: true,
            has_translucent: false,
        },
        shade: false,
        light_emission: 15,
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        TerrainMaterialClass::Opaque,
        0,
        TerrainRenderShape::Quads(std::sync::Arc::from(vec![quad])),
    );
    let layers = build(chunk(1, cells));
    let vertices = decode(cutout(&layers));

    assert_eq!(opaque(&layers).vertices.len(), 0);
    assert_eq!(cutout(&layers).cutout_faces, 1);
    assert_eq!(vertices.len(), 4);
    assert_eq!(vertices[0].position, [1.0, 0.0, 2.0]);
    assert_eq!(vertices[2].position, [2.0, 1.0, 2.0]);
    assert_eq!(vertices[0].uv, [0.0, 0.0]);
    assert_eq!(vertices[0].color, color(tint, 1.0, 1.0));
    assert_eq!(vertices[0].light[0], 1.0);
    assert_eq!(translucent(&layers).vertices.len(), 0);
}

#[test]
fn forced_translucent_box_faces_emit_in_translucent_layer() {
    let mut shape = slab_box_shape();
    let TerrainRenderShape::Box {
        face_transparency, ..
    } = &mut shape
    else {
        panic!("slab helper builds a box");
    };
    face_transparency[TerrainFace::Up.index()] = TerrainTransparency::TRANSLUCENT;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(TerrainMaterialClass::Opaque, 1, shape);
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 5);
    assert_eq!(translucent(&layers).translucent_faces, 1);
}

fn waterlogged_slab(water_tint: TerrainTint) -> TerrainCell {
    TerrainCell::with_shape(TerrainMaterialClass::Opaque, 5, slab_box_shape())
        .with_face_occlusion_shape_indices(bottom_slab_face_shapes())
        .with_fluid_render_data(
            TerrainFluid::new(TerrainFluidKind::Water, 8, false),
            [9, 9, 10, 10, 10, 10],
            [water_tint; 6],
        )
}

#[test]
fn waterlogged_block_emits_block_and_fluid_layers() {
    let water_tint = TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4);
    let cell = waterlogged_slab(water_tint);
    assert!(super::super::fluid_face_occlusion_culls(
        cell.face_occlusion_shape_indices,
        TerrainFace::Down,
        &TerrainCell::EMPTY.block_state(),
        1.0,
    ));
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = cell.with_light(TerrainLight { sky: 13, block: 2 });
    let layers = build(chunk(1, cells));

    assert_eq!(opaque(&layers).opaque_faces, 6);
    assert_eq!(cutout(&layers).vertices.len(), 0);
    // The slab's full down occlusion face hides the internal water bottom.
    // Five front faces plus the reversed top and four side faces remain.
    assert_eq!(translucent(&layers).translucent_faces, 10);
    let water_shades = [0.5, 0.6, 0.8, 1.0].map(|shade| color(water_tint, shade, 1.0));
    assert!(translucent(&layers)
        .vertices
        .iter()
        .all(|vertex| water_shades.contains(&vertex.color)));
}

#[test]
fn adjacent_waterlogged_blocks_cull_internal_fluid_faces() {
    let water_tint = TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(1, 0, 2, 1)] = waterlogged_slab(water_tint);
    cells[cell_index(2, 0, 2, 1)] = waterlogged_slab(water_tint);
    let layers = build(chunk(1, cells));

    // Each slab hides its fluid bottom and the equal fluid states hide both
    // shared sides. Four front faces plus four reversed top/side faces remain
    // per block.
    assert_eq!(translucent(&layers).translucent_faces, 16);
    assert_eq!(translucent(&layers).culled_faces, 4);
}

fn single_block_chunk(
    chunk_x: i32,
    chunk_z: i32,
    x: usize,
    y: usize,
    z: usize,
) -> TerrainChunkFixture {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(x, y, z, 1)] = cube(TerrainMaterialClass::Opaque);
    TerrainChunkFixture::new(chunk_x, chunk_z, 0, 1, cells)
}

fn single_fluid_chunk(
    chunk_x: i32,
    chunk_z: i32,
    x: usize,
    y: usize,
    z: usize,
) -> TerrainChunkFixture {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16];
    cells[cell_index(x, y, z, 1)] = water_cell(8);
    TerrainChunkFixture::new(chunk_x, chunk_z, 0, 1, cells)
}

fn slab_box_shape() -> TerrainRenderShape {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    for face in [
        TerrainFace::North,
        TerrainFace::South,
        TerrainFace::West,
        TerrainFace::East,
    ] {
        face_uvs[face.index()] = [0, 8, 16, 16];
    }
    let mut face_cull = all_face_cull();
    face_cull[TerrainFace::Up.index()] = None;
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 8, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull,
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn face_shape_index(mask: [u64; 4]) -> u8 {
    super::super::vanilla_face_occlusion_masks()
        .iter()
        .position(|candidate| *candidate == mask)
        .expect("Vanilla face-shape palette contains the synthetic test mask") as u8
}

fn bottom_slab_face_shapes() -> [u8; 6] {
    let side = face_shape_index([0x00ff_00ff_00ff_00ff; 4]);
    [1, 0, side, side, side, side]
}

fn top_slab_face_shapes() -> [u8; 6] {
    let side = face_shape_index([0xff00_ff00_ff00_ff00; 4]);
    [0, 1, side, side, side, side]
}

fn top_slab_box_shape() -> TerrainRenderShape {
    TerrainRenderShape::Box {
        from: [0, 8, 0],
        to: [16, 16, 16],
        face_present: [true; 6],
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: all_face_cull(),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

/// A straight, bottom-half stair: a full bottom slab plus an upper step that
/// only reaches half-way in `+z`; its `north` back face is full only via the
/// union of the two boxes.
fn stair_boxes_shape() -> TerrainRenderShape {
    let make = |from, to| TerrainBox {
        face_cull: all_face_cull(),
        ..plain_box(from, to, 0, TerrainTint::WHITE)
    };
    TerrainRenderShape::Boxes(std::sync::Arc::from(vec![
        make([0, 0, 0], [16, 8, 16]),
        make([0, 8, 0], [16, 16, 8]),
    ]))
}

fn fluid_box_shape(height: u8) -> TerrainRenderShape {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    for face in [
        TerrainFace::North,
        TerrainFace::South,
        TerrainFace::West,
        TerrainFace::East,
    ] {
        face_uvs[face.index()] = [0, 16 - height, 16, 16];
    }
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: all_face_cull(),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn water_cell(amount: u8) -> TerrainCell {
    fluid_cell(TerrainFluidKind::Water, amount)
}

fn lava_cell(amount: u8) -> TerrainCell {
    fluid_cell(TerrainFluidKind::Lava, amount)
}

fn fluid_cell(kind: TerrainFluidKind, amount: u8) -> TerrainCell {
    let amount = amount.clamp(1, 8);
    let height = ((amount as u16 * 16 + 4) / 9).clamp(1, 16) as u8;
    let mut cell = TerrainCell::with_shape(TerrainMaterialClass::Fluid, 0, fluid_box_shape(height))
        .with_fluid(TerrainFluid::new(kind, amount, false));
    cell.texture_indices = [1, 1, 2, 2, 2, 2];
    cell
}

fn all_face_cull() -> [Option<TerrainFace>; 6] {
    TerrainFace::ALL.map(Some)
}

fn same_block_all_faces_skip(same_block_key: u64) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key,
        same_block_culls_all_faces: true,
        ..TerrainSkipRendering::NONE
    }
}

fn same_block_vertical_faces_skip(same_block_key: u64) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key,
        same_block_culls_vertical_faces: true,
        ..TerrainSkipRendering::NONE
    }
}

fn iron_bars_skip(
    same_block_key: u64,
    bars_tag: bool,
    north: bool,
    south: bool,
    west: bool,
    east: bool,
) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key,
        iron_bars_block: true,
        bars_tag,
        north,
        south,
        west,
        east,
        connection_property_mask: 0b1111,
        ..TerrainSkipRendering::NONE
    }
}

/// The inner `west` face of `cube_all_inner_faces`: on the east side, facing west, culled by
/// the east neighbour.
fn inner_west_face() -> TerrainQuad {
    let x = 15.998;
    TerrainQuad {
        corners: [
            [x, 16.0, 0.0],
            [x, 0.0, 0.0],
            [x, 0.0, 16.0],
            [x, 16.0, 16.0],
        ],
        normal: [-1.0, 0.0, 0.0],
        uvs: [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        cull: Some(TerrainFace::East),
        texture_index: 0,
        tint: TerrainTint::WHITE,
        transparency: TerrainTransparency {
            has_transparent: true,
            has_translucent: false,
        },
        shade: true,
        light_emission: 0,
    }
}

fn quad_light_cells(quad: TerrainQuad, ambient_occlusion: bool) -> Vec<TerrainCell> {
    let dark = TerrainLight { sky: 0, block: 0 };
    let mut cells = vec![TerrainCell::EMPTY.with_light(dark); 16 * 16];
    cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 12 });
    cells[cell_index(2, 0, 2, 1)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 5 });
    cells[cell_index(1, 0, 1, 1)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 9 });
    cells[cell_index(1, 0, 2, 1)] = TerrainCell {
        collision_full_block: true,
        ..TerrainCell::with_shape(
            TerrainMaterialClass::Cutout,
            0,
            TerrainRenderShape::Quads(std::sync::Arc::from(vec![quad])),
        )
        .with_face_occlusion_shape_indices([0; 6])
        .with_ambient_occlusion(ambient_occlusion)
        .with_light(dark)
    };
    cells
}

#[test]
fn flat_quads_take_light_from_the_cull_face_or_the_cubic_facing_neighbour() {
    // A culled quad reads the neighbour behind its cull face (`ModelBlockRenderer.tesselateFlat`).
    let layers = build(chunk(1, quad_light_cells(inner_west_face(), false)));
    let vertices = decode(cutout(&layers));
    assert_eq!(vertices.len(), 4);
    assert!(vertices.iter().all(|vertex| vertex.light[0] == 5.0 / 15.0));

    // An unculled flat quad on a full-collision block is `faceCubic`, so it reads the
    // neighbour it faces (`BlockModelLighter.prepareQuadFlat`).
    let north_plane = TerrainQuad {
        corners: [
            [16.0, 0.0, 8.0],
            [16.0, 16.0, 8.0],
            [0.0, 16.0, 8.0],
            [0.0, 0.0, 8.0],
        ],
        normal: [0.0, 0.0, -1.0],
        cull: None,
        ..inner_west_face()
    };
    let layers = build(chunk(1, quad_light_cells(north_plane, false)));
    let vertices = decode(cutout(&layers));
    assert!(vertices.iter().all(|vertex| vertex.light[0] == 9.0 / 15.0));
}

#[test]
fn smooth_quads_light_by_their_baked_facing_not_the_cull_face() {
    // `prepareQuadAmbientOcclusion` follows `quad.direction()`: the inner west face takes the
    // light of the west neighbour even though the east neighbour culls it.
    let layers = build(chunk(1, quad_light_cells(inner_west_face(), true)));
    let vertices = decode(cutout(&layers));
    assert_eq!(vertices.len(), 4);
    for vertex in vertices {
        assert_float_eq(vertex.light[0], 12.0 / 15.0);
    }
}

fn up_face_smooth_lighting(lookup: &TerrainChunkLookup<'_>) -> ([u32; 4], [f32; 4]) {
    quad_lighting(
        TerrainFace::Up,
        &FACES[1].corners,
        false,
        Some(TerrainFace::Up),
        0,
        true,
        false,
        (2, 0, 2),
        TerrainLight { sky: 0, block: 0 },
        0,
        false,
        lookup,
    )
}

fn north_plane() -> TerrainQuad {
    TerrainQuad {
        corners: [
            [16.0, 16.0, 8.0],
            [16.0, 0.0, 8.0],
            [0.0, 0.0, 8.0],
            [0.0, 16.0, 8.0],
        ],
        normal: [0.0, 0.0, -1.0],
        cull: None,
        ..inner_west_face()
    }
}

fn quad_cell(quad: TerrainQuad) -> TerrainCell {
    TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Quads(std::sync::Arc::from(vec![quad])),
    )
    .with_face_occlusion_shape_indices([0; 6])
}

#[test]
fn non_cubic_corner_occlusion_probes_one_step_along_the_facing() {
    // `prepareQuadAmbientOcclusion` tests `base + corner + direction` even when `base` is the
    // block itself: stone west and below with air north of both lets the diagonal through.
    let stone = cube(TerrainMaterialClass::Opaque).with_ambient_light_facts(true, 15, 51);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 1, 2, 2)] = quad_cell(north_plane());
    cells[cell_index(0, 1, 2, 2)] = stone.clone();
    cells[cell_index(1, 0, 2, 2)] = stone;
    let layers = build(chunk(2, cells));
    let vertex = vertex_at(&decode(cutout(&layers)), [1.0, 1.0, 2.5]);

    let dark = 51.0_f32 / 255.0;
    let ambient = (dark + dark + 1.0 + 1.0) * 0.25;
    assert_eq!(vertex.color, color(TerrainTint::WHITE, 0.8, ambient));
}

#[test]
fn face_cubic_needs_the_quad_strictly_inside_the_boundary_epsilon() {
    // `prepareQuadShape`: a west face is cubic only for `minX < 1.0E-4`; a cubic unculled flat
    // quad reads the west neighbour (block 12), otherwise the block's own light (0).
    let west_face = |x: f32| TerrainQuad {
        corners: [
            [x, 16.0, 0.0],
            [x, 0.0, 0.0],
            [x, 0.0, 16.0],
            [x, 16.0, 16.0],
        ],
        cull: None,
        ..inner_west_face()
    };
    for (x, block) in [(0.5E-4_f32 * 16.0, 12.0), (1.0E-4_f32 * 16.0, 0.0)] {
        let mut cells = quad_light_cells(west_face(x), false);
        cells[cell_index(1, 0, 2, 1)].collision_full_block = false;
        let layers = build(chunk(1, cells));
        let vertices = decode(cutout(&layers));
        assert_eq!(vertices.len(), 4);
        assert!(vertices
            .iter()
            .all(|vertex| vertex.light[0] == block / 15.0));
    }
}

#[test]
fn element_light_emission_keeps_ambient_occlusion() {
    // Smooth lighting is chosen per block state; an emissive element only raises its light.
    let mut top = north_plane();
    top.corners = [
        [0.0, 16.0, 0.0],
        [0.0, 16.0, 16.0],
        [16.0, 16.0, 16.0],
        [16.0, 16.0, 0.0],
    ];
    top.normal = [0.0, 1.0, 0.0];
    top.cull = Some(TerrainFace::Up);
    top.light_emission = 5;
    let dark = TerrainLight { sky: 0, block: 0 };
    let mut cells = vec![TerrainCell::EMPTY.with_light(dark); 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = quad_cell(top).with_light(dark);
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] = cube(TerrainMaterialClass::Opaque)
            .with_ambient_light_facts(true, 15, 51)
            .with_light(dark);
    }
    let layers = build(chunk(2, cells));
    let vertex = vertex_at(&decode(cutout(&layers)), [1.0, 1.0, 2.0]);

    let dark_shade = 51.0_f32 / 255.0;
    let ambient = (dark_shade + dark_shade + dark_shade + 1.0) * 0.25;
    assert_eq!(vertex.color, color(TerrainTint::WHITE, 1.0, ambient));
    assert_eq!(vertex.light, [5.0 / 15.0, 5.0 / 15.0]);
}

#[test]
fn partial_faces_weight_the_corner_blends_by_their_extent() {
    // A top face spanning z 0..8 is `facePartial`: its south vertices take half of the south
    // blends (96) and half of the north ones (64), `smoothWeightedBlend` = 80.
    let dark = TerrainLight { sky: 0, block: 0 };
    let mut cells = vec![TerrainCell::EMPTY.with_light(dark); 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = TerrainCell::with_shape(
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Boxes(std::sync::Arc::from(vec![plain_box(
            [0, 0, 0],
            [16, 16, 8],
            0,
            TerrainTint::WHITE,
        )])),
    )
    .with_face_occlusion_shape_indices([0; 6])
    .with_light(dark);
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 4 });
    cells[cell_index(1, 1, 3, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 12 });
    let layers = build(chunk(2, cells));
    let top = face_vertices(cutout(&layers), [1, 0, 2], UP);

    assert_eq!(top.len(), 4);
    for (position, coords) in [
        ([1.0, 1.0, 2.0], 64.0),
        ([2.0, 1.0, 2.0], 64.0),
        ([1.0, 1.0, 2.5], 80.0),
        ([2.0, 1.0, 2.5], 80.0),
    ] {
        assert_eq!(vertex_at(&top, position).light, [coords / 16.0 / 15.0, 0.0]);
    }
}

#[test]
fn rotated_element_quads_light_by_baked_vertex_index() {
    // `FaceBakery` keeps a rotated element's vertex order, so `prepareQuadAmbientOcclusion`
    // lights vertex 0 with the `FaceInfo.UP` vertex 0 (min x, min z) corner even when it sits
    // at max x, max z.
    let mut top = north_plane();
    top.corners = [
        [16.0, 16.0, 16.0],
        [0.0, 16.0, 16.0],
        [0.0, 16.0, 0.0],
        [16.0, 16.0, 0.0],
    ];
    top.normal = [0.0, 1.0, 0.0];
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = quad_cell(top);
    for (x, z) in [(0, 2), (1, 1), (0, 1)] {
        cells[cell_index(x, 1, z, 2)] =
            cube(TerrainMaterialClass::Opaque).with_ambient_light_facts(true, 15, 51);
    }
    let layers = build(chunk(2, cells));
    let vertices = decode(cutout(&layers));

    let dark = 51.0_f32 / 255.0;
    let ambient = (dark + dark + dark + 1.0) * 0.25;
    assert_eq!(
        vertex_at(&vertices, [2.0, 1.0, 3.0]).color,
        color(TerrainTint::WHITE, 1.0, ambient)
    );
    assert_eq!(vertex_at(&vertices, [1.0, 1.0, 2.0]).color, [255; 4]);
}

#[test]
fn cube_faces_split_along_face_info_vertices_zero_and_two() {
    // Vanilla draws quads as triangles (0, 1, 2) and (2, 3, 0) of the `FaceInfo` vertex order,
    // which decides how smooth light and AO interpolate across a face.
    let layers = build(single_block_chunk(0, 0, 1, 0, 2));
    let mesh = opaque(&layers);
    let vertices = decode(mesh);
    let diagonals = [
        (DOWN, [[1.0, 0.0, 3.0], [2.0, 0.0, 2.0]]),
        (UP, [[1.0, 1.0, 2.0], [2.0, 1.0, 3.0]]),
        (NORTH, [[2.0, 1.0, 2.0], [1.0, 0.0, 2.0]]),
        (SOUTH, [[1.0, 1.0, 3.0], [2.0, 0.0, 3.0]]),
        (WEST, [[1.0, 1.0, 2.0], [1.0, 0.0, 3.0]]),
        (EAST, [[2.0, 1.0, 3.0], [2.0, 0.0, 2.0]]),
    ];
    assert_eq!(mesh.indices.len(), 36);
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let normal = vertices[triangle[0] as usize].normal;
        let (_, diagonal) = diagonals
            .iter()
            .find(|(face, _)| *face == normal)
            .expect("cube face");
        let positions: Vec<_> = triangle
            .iter()
            .map(|index| vertices[*index as usize].position)
            .collect();
        for corner in diagonal {
            assert!(positions.contains(corner), "{normal:?}: {positions:?}");
        }
    }
}

#[test]
fn neighbor_cache_matches_slow_lookup_across_18x18x18_and_outer_ao_halo() {
    let mut chunks = Vec::new();
    for cz in -1..=1_i32 {
        for cx in -1..=1_i32 {
            // Skip one corner column so unloaded `None` neighbors are also exercised.
            if cx == 1 && cz == -1 {
                continue;
            }
            // Give one neighbor partial height to exercise `contains_world_y` boundaries inside 18^3.
            let (min_y, height) = if cx == -1 && cz == 0 {
                (2, 20)
            } else {
                (0, 48)
            };
            let mut cells = Vec::with_capacity(16 * 16 * height);
            for y in 0..height {
                for z in 0..16 {
                    for x in 0..16 {
                        let mat = match (cx + cz + y as i32 + z as i32 + x as i32).rem_euclid(3) {
                            0 => TerrainMaterialClass::Empty,
                            1 => TerrainMaterialClass::Opaque,
                            _ => TerrainMaterialClass::Cutout,
                        };
                        cells.push(TerrainCell::with_texture(mat, 1).with_light(TerrainLight {
                            sky: ((x + y) & 15) as u8,
                            block: ((z + y * 3) & 15) as u8,
                        }));
                    }
                }
            }
            chunks.push(TerrainChunkFixture::new(cx, cz, min_y, height, cells));
        }
    }
    let world = TerrainFixtureWorld::new(&chunks, TerrainTextureAtlas::unit());
    let center = world
        .snapshots
        .iter()
        .find(|s| s.chunk_x == 0 && s.chunk_z == 0)
        .unwrap();
    let mut lookup = TerrainChunkLookup::new(
        center,
        world
            .snapshots
            .iter()
            .filter(|s| (s.chunk_x, s.chunk_z) != (0, 0)),
        &world.table,
    );
    lookup.prepare_section(1);
    for y in 14..=33 {
        for z in -2..=17 {
            for x in -2..=17 {
                let fast = lookup.cell(x, y, z).map(|c| (c.material, c.light));
                let slow = lookup.cell_slow(x, y, z).map(|c| (c.material, c.light));
                assert_eq!(fast, slow, "mismatch at ({x}, {y}, {z})");
            }
        }
    }
}
