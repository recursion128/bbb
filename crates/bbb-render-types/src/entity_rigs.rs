//! Shared Blockbench / Bedrock-compatible data-driven entity rig and animation definitions.

use serde::{Deserialize, Serialize};

fn default_bone_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

fn default_texture_size() -> [u32; 2] {
    [64, 64]
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataDrivenRigTarget {
    Position,
    #[default]
    Rotation,
    Scale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataDrivenRigInterpolation {
    #[default]
    Linear,
    CatmullRom,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigKeyframe {
    pub timestamp: f32,
    pub pre: [f32; 3],
    pub post: [f32; 3],
    #[serde(default)]
    pub interpolation: DataDrivenRigInterpolation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigChannel {
    pub target: DataDrivenRigTarget,
    #[serde(default)]
    pub keyframes: Vec<DataDrivenRigKeyframe>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigBoneChannels {
    pub bone: String,
    #[serde(default)]
    pub channels: Vec<DataDrivenRigChannel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigAnimation {
    pub name: String,
    pub length_seconds: f32,
    #[serde(default)]
    pub looping: bool,
    #[serde(default)]
    pub bones: Vec<DataDrivenRigBoneChannels>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigCube {
    pub origin: [f32; 3],
    pub size: [f32; 3],
    #[serde(default)]
    pub inflate: f32,
    #[serde(default)]
    pub uv: [f32; 2],
    #[serde(default)]
    pub mirror: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigBone {
    pub name: String,
    #[serde(default)]
    pub pivot: [f32; 3],
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    #[serde(default = "default_bone_scale")]
    pub scale: [f32; 3],
    #[serde(default)]
    pub mirror: bool,
    #[serde(default)]
    pub cubes: Vec<DataDrivenRigCube>,
    #[serde(default)]
    pub children: Vec<DataDrivenRigBone>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataDrivenRigSpec {
    #[serde(default)]
    pub rig_id: String,
    #[serde(default)]
    pub texture_asset_id: Option<String>,
    #[serde(default)]
    pub emissive_texture_asset_id: Option<String>,
    #[serde(default = "default_texture_size")]
    pub texture_size: [u32; 2],
    #[serde(default)]
    pub shadow_radius: Option<f32>,
    #[serde(default)]
    pub bones: Vec<DataDrivenRigBone>,
    #[serde(default)]
    pub animations: Vec<DataDrivenRigAnimation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_spec_json_round_trip_and_defaults() {
        let json = r#"{
            "rig_id": "bbb_demo:crystal_golem",
            "texture_asset_id": "bbb_demo:entity/crystal_golem",
            "bones": [{
                "name": "body",
                "pivot": [0.0, 24.0, 0.0],
                "cubes": [{ "origin": [-4.0, 0.0, -2.0], "size": [8.0, 12.0, 4.0], "uv": [0.0, 16.0] }]
            }],
            "animations": [{
                "name": "idle",
                "length_seconds": 2.0,
                "looping": true,
                "bones": [{
                    "bone": "body",
                    "channels": [{
                        "target": "rotation",
                        "keyframes": [
                            { "timestamp": 0.0, "pre": [0.0, 0.0, 0.0], "post": [0.0, 0.0, 0.0] },
                            { "timestamp": 1.0, "pre": [0.1, 0.0, 0.0], "post": [0.1, 0.0, 0.0], "interpolation": "catmull_rom" }
                        ]
                    }]
                }]
            }]
        }"#;
        let parsed: DataDrivenRigSpec = serde_json::from_str(json).expect("valid rig JSON");
        assert_eq!(parsed.texture_size, [64, 64]);
        assert_eq!(parsed.bones[0].scale, [1.0, 1.0, 1.0]);
        assert_eq!(
            parsed.animations[0].bones[0].channels[0].keyframes[1].interpolation,
            DataDrivenRigInterpolation::CatmullRom
        );
        let serialized = serde_json::to_string(&parsed).expect("serialize");
        let reparsed: DataDrivenRigSpec = serde_json::from_str(&serialized).expect("reparse");
        assert_eq!(parsed, reparsed);
    }
}
