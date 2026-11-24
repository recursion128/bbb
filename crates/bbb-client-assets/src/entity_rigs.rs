//! Data-driven entity rig catalog (`bbb_rigs/*.json`) and built-in rig assets.

use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use bbb_render_types::{
    DataDrivenRigAnimation, DataDrivenRigBone, DataDrivenRigBoneChannels, DataDrivenRigChannel,
    DataDrivenRigCube, DataDrivenRigInterpolation, DataDrivenRigKeyframe, DataDrivenRigSpec,
    DataDrivenRigTarget,
};

use crate::{
    resources::{PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityRigCatalog {
    rigs: Vec<DataDrivenRigSpec>,
    index_by_id: BTreeMap<String, u32>,
}

impl EntityRigCatalog {
    pub fn builtin_default() -> Self {
        let mut by_id = BTreeMap::new();
        insert_builtin_rigs(&mut by_id);
        Self::from_sorted_map(by_id)
    }

    pub fn load(roots: Option<&PackRoots>) -> Result<Self> {
        match roots {
            Some(roots) => Self::load_resource_stack(&roots.resource_stack()),
            None => Ok(Self::builtin_default()),
        }
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut by_id = BTreeMap::new();
        insert_builtin_rigs(&mut by_id);
        for resource in stack.list_resources("bbb_rigs", ".json")? {
            let rig_id = rig_id_from_resource(&resource.location)?;
            let mut spec: DataDrivenRigSpec =
                stack.read_json_resource(&resource, "data-driven entity rig")?;
            spec.rig_id = rig_id.clone();
            if spec.texture_size[0] == 0 || spec.texture_size[1] == 0 {
                spec.texture_size = [64, 64];
            }
            by_id.insert(rig_id, spec);
        }
        Ok(Self::from_sorted_map(by_id))
    }

    fn from_sorted_map(by_id: BTreeMap<String, DataDrivenRigSpec>) -> Self {
        let mut rigs = Vec::with_capacity(by_id.len());
        let mut index_by_id = BTreeMap::new();
        for (idx, (id, spec)) in by_id.into_iter().enumerate() {
            if let Ok(index) = u32::try_from(idx) {
                index_by_id.insert(id, index);
                rigs.push(spec);
            }
        }
        Self { rigs, index_by_id }
    }

    pub fn rigs(&self) -> &[DataDrivenRigSpec] {
        &self.rigs
    }

    pub fn rig_index(&self, rig_id: &str) -> Option<u32> {
        let normalized = ResourceLocation::parse(rig_id).ok()?.id();
        self.index_by_id.get(&normalized).copied()
    }

    pub fn rig(&self, index: u32) -> Option<&DataDrivenRigSpec> {
        self.rigs.get(index as usize)
    }

    pub fn rig_for_entity(
        &self,
        entity: &bbb_protocol::content_registry::ModEntityTypeSpec,
    ) -> Option<(u32, &DataDrivenRigSpec)> {
        let key = entity.rig_id.as_deref().unwrap_or(&entity.resource_id);
        let index = self.rig_index(key)?;
        Some((index, &self.rigs[index as usize]))
    }

    pub fn len(&self) -> usize {
        self.rigs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rigs.is_empty()
    }
}

fn insert_builtin_rigs(by_id: &mut BTreeMap<String, DataDrivenRigSpec>) {
    let inferno = builtin_inferno_warden_rig_spec();
    by_id.insert(inferno.rig_id.clone(), inferno);
}

fn rig_id_from_resource(location: &ResourceLocation) -> Result<String> {
    let path = location
        .path()
        .strip_prefix("bbb_rigs/")
        .and_then(|path| path.strip_suffix(".json"))
        .ok_or_else(|| anyhow!("entity rig resource {} is outside bbb_rigs", location.id()))?;
    ResourceLocation::new(location.namespace().to_string(), path.to_string()).map(|id| id.id())
}

fn bone(
    name: &str,
    pivot: [f32; 3],
    cubes: Vec<DataDrivenRigCube>,
    children: Vec<DataDrivenRigBone>,
) -> DataDrivenRigBone {
    DataDrivenRigBone {
        name: name.to_string(),
        pivot,
        rotation_deg: [0.0; 3],
        scale: [1.0, 1.0, 1.0],
        mirror: false,
        cubes,
        children,
    }
}

fn cube(origin: [f32; 3], size: [f32; 3], uv: [f32; 2]) -> DataDrivenRigCube {
    DataDrivenRigCube {
        origin,
        size,
        inflate: 0.0,
        uv,
        mirror: None,
    }
}

fn kf(timestamp: f32, target: [f32; 3]) -> DataDrivenRigKeyframe {
    DataDrivenRigKeyframe {
        timestamp,
        pre: target,
        post: target,
        interpolation: DataDrivenRigInterpolation::Linear,
    }
}

/// Returns the built-in articulated Blockbench rig for `bbb_demo:inferno_warden`.
pub fn builtin_inferno_warden_rig_spec() -> DataDrivenRigSpec {
    let torso = bone(
        "body",
        [0.0, 2.0, 0.0],
        vec![cube([-9.0, -14.0, -5.0], [18.0, 22.0, 10.0], [0.0, 0.0])],
        vec![
            bone(
                "chest_core",
                [0.0, -4.0, -5.5],
                vec![cube([-4.0, -4.0, -1.0], [8.0, 8.0, 2.0], [40.0, 32.0])],
                Vec::new(),
            ),
            bone(
                "head",
                [0.0, -14.0, 0.0],
                vec![cube([-7.0, -14.0, -5.0], [14.0, 14.0, 10.0], [0.0, 32.0])],
                vec![
                    bone(
                        "left_horn",
                        [-7.0, -11.0, 0.0],
                        vec![cube([-6.0, -8.0, -2.0], [6.0, 8.0, 4.0], [0.0, 52.0])],
                        Vec::new(),
                    ),
                    bone(
                        "right_horn",
                        [7.0, -11.0, 0.0],
                        vec![cube([0.0, -8.0, -2.0], [6.0, 8.0, 4.0], [20.0, 52.0])],
                        Vec::new(),
                    ),
                ],
            ),
            bone(
                "left_arm",
                [-12.0, -11.0, 0.0],
                vec![cube([-3.0, -2.0, -4.0], [6.0, 24.0, 8.0], [36.0, 0.0])],
                Vec::new(),
            ),
            bone(
                "right_arm",
                [12.0, -11.0, 0.0],
                vec![cube([-3.0, -2.0, -4.0], [6.0, 24.0, 8.0], [36.0, 0.0])],
                Vec::new(),
            ),
            bone(
                "left_leg",
                [-5.0, 8.0, 0.0],
                vec![cube([-3.0, 0.0, -3.5], [6.0, 14.0, 7.0], [0.0, 0.0])],
                Vec::new(),
            ),
            bone(
                "right_leg",
                [5.0, 8.0, 0.0],
                vec![cube([-3.0, 0.0, -3.5], [6.0, 14.0, 7.0], [0.0, 0.0])],
                Vec::new(),
            ),
        ],
    );

    let animations = vec![
        DataDrivenRigAnimation {
            name: "idle".to_string(),
            length_seconds: 2.0,
            looping: true,
            bones: vec![
                DataDrivenRigBoneChannels {
                    bone: "body".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Position,
                        keyframes: vec![
                            kf(0.0, [0.0, 0.0, 0.0]),
                            kf(1.0, [0.0, -0.5, 0.0]),
                            kf(2.0, [0.0, 0.0, 0.0]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "chest_core".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Scale,
                        keyframes: vec![
                            kf(0.0, [0.0, 0.0, 0.0]),
                            kf(1.0, [0.12, 0.12, 0.12]),
                            kf(2.0, [0.0, 0.0, 0.0]),
                        ],
                    }],
                },
            ],
        },
        DataDrivenRigAnimation {
            name: "walk".to_string(),
            length_seconds: 1.0,
            looping: true,
            bones: vec![
                DataDrivenRigBoneChannels {
                    bone: "left_leg".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [0.55, 0.0, 0.0]),
                            kf(0.5, [-0.55, 0.0, 0.0]),
                            kf(1.0, [0.55, 0.0, 0.0]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "right_leg".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [-0.55, 0.0, 0.0]),
                            kf(0.5, [0.55, 0.0, 0.0]),
                            kf(1.0, [-0.55, 0.0, 0.0]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "left_arm".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [-0.45, 0.0, 0.0]),
                            kf(0.5, [0.45, 0.0, 0.0]),
                            kf(1.0, [-0.45, 0.0, 0.0]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "right_arm".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [0.45, 0.0, 0.0]),
                            kf(0.5, [-0.45, 0.0, 0.0]),
                            kf(1.0, [0.45, 0.0, 0.0]),
                        ],
                    }],
                },
            ],
        },
        DataDrivenRigAnimation {
            name: "enraged".to_string(),
            length_seconds: 0.6,
            looping: true,
            bones: vec![
                DataDrivenRigBoneChannels {
                    bone: "left_arm".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [-1.35, 0.0, -0.2]),
                            kf(0.3, [-1.55, 0.0, -0.35]),
                            kf(0.6, [-1.35, 0.0, -0.2]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "right_arm".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Rotation,
                        keyframes: vec![
                            kf(0.0, [-1.35, 0.0, 0.2]),
                            kf(0.3, [-1.55, 0.0, 0.35]),
                            kf(0.6, [-1.35, 0.0, 0.2]),
                        ],
                    }],
                },
                DataDrivenRigBoneChannels {
                    bone: "chest_core".to_string(),
                    channels: vec![DataDrivenRigChannel {
                        target: DataDrivenRigTarget::Scale,
                        keyframes: vec![
                            kf(0.0, [0.25, 0.25, 0.25]),
                            kf(0.3, [0.45, 0.45, 0.45]),
                            kf(0.6, [0.25, 0.25, 0.25]),
                        ],
                    }],
                },
            ],
        },
    ];

    DataDrivenRigSpec {
        rig_id: "bbb_demo:inferno_warden".to_string(),
        texture_asset_id: Some("bbb_demo:entity/inferno_warden".to_string()),
        emissive_texture_asset_id: None,
        texture_size: [64, 64],
        shadow_radius: Some(0.9),
        bones: vec![torso],
        animations,
    }
}

/// Returns procedural RGBA texture pixels (`width x height`) for built-in mod rigs.
pub fn builtin_rig_texture_rgba(asset_id: &str) -> Option<([u32; 2], Vec<u8>)> {
    if asset_id != "bbb_demo:entity/inferno_warden" {
        return None;
    }
    let width = 64_u32;
    let height = 64_u32;
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let in_core_uv = (40..56).contains(&x) && (32..48).contains(&y);
            let in_horn_uv = y >= 52 && x < 40;
            let magma_vein = (x ^ y) % 7 == 0 || ((x + y * 2) % 13 == 0);
            let color = if in_core_uv {
                let border = x == 40 || x == 55 || y == 32 || y == 47;
                if border {
                    [235, 92, 16, 255]
                } else {
                    [255, 212, 64, 255]
                }
            } else if in_horn_uv {
                [215, 76, 20, 255]
            } else if magma_vein {
                [192, 62, 14, 255]
            } else {
                [34, 24, 30, 255]
            };
            rgba[idx..idx + 4].copy_from_slice(&color);
        }
    }
    Some(([width, height], rgba))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builtin_inferno_warden_rig_and_texture_are_present() {
        let catalog = EntityRigCatalog::load(None).expect("builtin catalog");
        let idx = catalog
            .rig_index("bbb_demo:inferno_warden")
            .expect("inferno_warden rig index");
        let spec = catalog.rig(idx).expect("inferno_warden spec");
        assert_eq!(spec.bones[0].name, "body");
        assert_eq!(spec.animations.len(), 3);

        let (size, rgba) =
            builtin_rig_texture_rgba("bbb_demo:entity/inferno_warden").expect("builtin texture");
        assert_eq!(size, [64, 64]);
        assert_eq!(rgba.len(), 64 * 64 * 4);
    }

    #[test]
    fn rig_catalog_loads_and_overrides_from_resource_pack_dir() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("bbb-rigs-test-{nanos}"));
        let vanilla_assets = root
            .join("sources")
            .join(crate::MC_VERSION)
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(&vanilla_assets).unwrap();

        let pack_dir = root.join("custom_pack");
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(
            pack_dir.join("pack.mcmeta"),
            r#"{"pack":{"description":"test","min_format":1,"max_format":999}}"#,
        )
        .unwrap();
        let rig_path = pack_dir
            .join("assets")
            .join("bbb_demo")
            .join("bbb_rigs")
            .join("inferno_warden.json");
        std::fs::create_dir_all(rig_path.parent().unwrap()).unwrap();
        std::fs::write(
            &rig_path,
            r#"{
                "texture_size": [128, 128],
                "bones": [{
                    "name": "body",
                    "cubes": [{ "origin": [-4.0, 0.0, -4.0], "size": [8.0, 8.0, 8.0] }]
                }]
            }"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([&pack_dir]);
        let catalog = EntityRigCatalog::load(Some(&roots)).unwrap();
        let idx = catalog.rig_index("bbb_demo:inferno_warden").unwrap();
        assert_eq!(catalog.rig(idx).unwrap().texture_size, [128, 128]);
        let _ = std::fs::remove_dir_all(root);
    }
}
