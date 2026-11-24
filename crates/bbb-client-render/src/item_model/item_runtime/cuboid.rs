//! Resource-model geometry retained by ordinary item leaves.
use super::*;
use crate::terrain::{
    bake_block_item_quads, TerrainFace, TerrainQuad, TerrainRenderShape, TerrainTextureAtlas,
    TerrainTint, TerrainTransparency,
};
use bbb_client_assets::{BlockModelBox, BlockModelFace, BlockModelShape};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CuboidPart {
    pub model_id: Arc<str>,
    pub part: u32,
    pub geometry: Arc<crate::item_render::ItemModelGeometry>,
    pub ground_model_extents: Option<(glam::Vec3, glam::Vec3)>,
    pub tint: ItemIconTint,
}

#[derive(Debug)]
pub(super) struct BakedCuboidGroup {
    pub tint_index: Option<i32>,
    pub geometry: Arc<crate::item_render::ItemModelGeometry>,
    pub default_ground_extents: Option<(glam::Vec3, glam::Vec3)>,
}

#[derive(Debug)]
pub(super) struct BakedCuboidGeometry {
    pub model_id: Arc<str>,
    pub groups: Vec<BakedCuboidGroup>,
}

pub(super) fn bake_geometry(
    model: &ItemCuboidModel,
    textures: &ItemTextureState,
) -> (Arc<BakedCuboidGeometry>, usize) {
    let atlas = textures.cuboid_atlas();
    let mut groups: BTreeMap<Option<i32>, Vec<crate::ItemModelQuad>> = BTreeMap::new();
    let fallback = model.face_textures.as_ref();
    let material = |texture: &str, force: bool| {
        let index = textures.texture_index(texture);
        (
            index,
            TerrainTransparency {
                has_transparent: true,
                has_translucent: force || textures.texture_has_translucent(index),
            },
        )
    };
    let mut add_box = |b: &BlockModelBox| {
        for face in 0..6 {
            if !b.face_present[face] {
                continue;
            }
            let texture = b.face_textures[face]
                .as_deref()
                .unwrap_or_else(|| &fallback.unwrap().textures[face]);
            let (index, transparency) = material(texture, b.face_force_translucent[face]);
            let shape = TerrainRenderShape::Box {
                from: b.from,
                to: b.to,
                face_present: std::array::from_fn(|i| i == face),
                face_uvs: b.face_uvs,
                face_uv_rotations: b.face_uv_rotations,
                face_shade: b.face_shade,
                face_light_emission: b.face_light_emission,
                face_cull: [None; 6],
                face_transparency: [transparency; 6],
            };
            groups
                .entry(b.face_tint_indices[face])
                .or_default()
                .extend(bake_block_item_quads(
                    &shape,
                    [index; 6],
                    [TerrainTint::WHITE; 6],
                    atlas,
                ));
        }
    };
    match &model.shape {
        BlockModelShape::Box(b) => add_box(b),
        BlockModelShape::Boxes(boxes) => {
            for b in boxes {
                add_box(b)
            }
        }
        BlockModelShape::Quads(quads) => {
            for q in quads {
                let face = q.face.index();
                let texture = q
                    .texture
                    .as_deref()
                    .unwrap_or_else(|| &fallback.unwrap().textures[face]);
                let (index, transparency) = material(texture, q.force_translucent);
                let shape = TerrainRenderShape::Quads(
                    vec![TerrainQuad {
                        corners: q.corners,
                        normal: q.normal,
                        uvs: q.uvs,
                        cull: q.cull.map(terrain_face),
                        texture_index: index,
                        tint: TerrainTint::WHITE,
                        transparency,
                        shade: q.shade,
                        light_emission: q.light_emission,
                    }]
                    .into(),
                );
                groups
                    .entry(q.tint_index)
                    .or_default()
                    .extend(bake_block_item_quads(
                        &shape,
                        [index; 6],
                        [TerrainTint::WHITE; 6],
                        atlas,
                    ));
            }
        }
        BlockModelShape::Cube => {
            let faces = fallback.unwrap();
            add_box(&BlockModelBox {
                from: [0; 3],
                to: [16; 3],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                face_tint_indices: faces.tint_indices,
                face_textures: std::array::from_fn(|i| Some(faces.textures[i].clone())),
                face_force_translucent: faces.force_translucent,
            });
        }
        BlockModelShape::Cross {
            shade,
            light_emission,
        } => {
            add_cross(
                &mut groups,
                &bbb_client_assets::BlockModelCross {
                    face_textures: std::array::from_fn(|_| None),
                    face_tint_indices: [None; 6],
                    face_force_translucent: [false; 6],
                    shade: *shade,
                    light_emission: *light_emission,
                },
                fallback.unwrap(),
                textures,
                atlas,
            );
        }
        BlockModelShape::Crosses(crosses) => {
            for cross in crosses {
                add_cross(&mut groups, cross, fallback.unwrap(), textures, atlas);
            }
        }
    }
    let ground_display = model
        .display_transforms
        .get(BlockModelDisplayContext::Ground);
    let ground_transform = super::item_display_matrix(&ground_display, false);
    let mut total_quads = 0usize;
    let baked_groups: Vec<BakedCuboidGroup> = groups
        .into_iter()
        .map(|(tint_index, quads)| {
            total_quads = total_quads.saturating_add(quads.len());
            let geometry = Arc::new(crate::item_render::ItemModelGeometry::new(quads));
            let default_ground_extents = geometry.transformed_extents(ground_transform);
            BakedCuboidGroup {
                tint_index,
                geometry,
                default_ground_extents,
            }
        })
        .collect();
    let estimated_bytes = total_quads
        .saturating_mul(std::mem::size_of::<crate::ItemModelQuad>())
        .saturating_add(baked_groups.len().saturating_mul(128))
        .saturating_add(model.id.len());
    (
        Arc::new(BakedCuboidGeometry {
            model_id: Arc::from(model.id.as_str()),
            groups: baked_groups,
        }),
        estimated_bytes,
    )
}

pub(super) fn parts_from_baked_geometry(
    baked: &BakedCuboidGeometry,
    tints: Option<&Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
) -> (Vec<ItemIconTint>, Arc<[CuboidPart]>) {
    let resolved_tints: Vec<ItemIconTint> = baked
        .groups
        .iter()
        .map(|group| {
            group
                .tint_index
                .and_then(|i| tints.and_then(|tints| tints.get(i as usize)))
                .map(|source| item_tint_source(source, colormaps))
                .unwrap_or(ItemIconTint::Static(ITEM_TINT_WHITE))
        })
        .collect();
    let parts = baked
        .groups
        .iter()
        .zip(resolved_tints.iter())
        .enumerate()
        .map(|(part, (group, tint))| CuboidPart {
            model_id: Arc::clone(&baked.model_id),
            part: part as u32,
            geometry: Arc::clone(&group.geometry),
            ground_model_extents: group.default_ground_extents,
            tint: tint.clone(),
        })
        .collect::<Vec<_>>()
        .into();
    (resolved_tints, parts)
}

pub(super) fn bake_model(
    model: &ItemCuboidModel,
    tints: Option<&Vec<ItemTintSource>>,
    colormaps: Option<&TerrainColorMaps>,
    textures: &ItemTextureState,
) -> Arc<[CuboidPart]> {
    let (baked, _) = bake_geometry(model, textures);
    parts_from_baked_geometry(&baked, tints, colormaps).1
}

fn terrain_face(face: BlockModelFace) -> TerrainFace {
    match face {
        BlockModelFace::Down => TerrainFace::Down,
        BlockModelFace::Up => TerrainFace::Up,
        BlockModelFace::North => TerrainFace::North,
        BlockModelFace::South => TerrainFace::South,
        BlockModelFace::West => TerrainFace::West,
        BlockModelFace::East => TerrainFace::East,
    }
}

fn add_cross(
    groups: &mut BTreeMap<Option<i32>, Vec<crate::ItemModelQuad>>,
    cross: &bbb_client_assets::BlockModelCross,
    fallback: &bbb_client_assets::BlockFaceTextures,
    textures: &ItemTextureState,
    atlas: &TerrainTextureAtlas,
) {
    for (face, normal, corners) in crate::terrain::mesh::geometry::CROSS_FACES {
        let i = face.index();
        let explicit = cross.face_textures[i].is_some();
        let index = textures.texture_index(
            cross.face_textures[i]
                .as_deref()
                .unwrap_or(&fallback.textures[i]),
        );
        let tint_index = if explicit {
            cross.face_tint_indices[i]
        } else {
            fallback.tint_indices[i]
        };
        let translucent = cross.face_force_translucent[i]
            || (!explicit && fallback.force_translucent[i])
            || textures.texture_has_translucent(index);
        let shape = TerrainRenderShape::Quads(
            vec![TerrainQuad {
                corners: corners.map(|p| p.map(|v| v * 16.0)),
                normal,
                uvs: [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
                cull: None,
                texture_index: index,
                tint: TerrainTint::WHITE,
                transparency: TerrainTransparency {
                    has_transparent: true,
                    has_translucent: translucent,
                },
                shade: cross.shade,
                light_emission: cross.light_emission,
            }]
            .into(),
        );
        groups
            .entry(tint_index)
            .or_default()
            .extend(bake_block_item_quads(
                &shape,
                [index; 6],
                [TerrainTint::WHITE; 6],
                atlas,
            ));
    }
}
