//! Data-driven GPU-skinned entity rig baking and keyframe pose evaluation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{bail, Result};
use bbb_render_types::{
    DataDrivenRigAnimation, DataDrivenRigBone, DataDrivenRigChannel, DataDrivenRigCube,
    DataDrivenRigInterpolation, DataDrivenRigKeyframe, DataDrivenRigSpec, DataDrivenRigTarget,
};
use hashbrown::HashMap;

use super::geometry::PartPose;
use super::keyframe::{sample_keyframe_channel, AnimationTarget, Keyframe, KeyframeInterpolation};
use super::model::{ModelCubes, ModelPart, PartName};
use super::model_layer::{cube, ModelCube};
use super::EntityRenderState;

pub(crate) const MAX_DATA_DRIVEN_RIG_BONES: usize = 512;
pub(crate) const MAX_DATA_DRIVEN_RIG_CUBES: usize = 4_096;
pub(crate) const MAX_DATA_DRIVEN_RIG_DEPTH: usize = 32;
pub(crate) const MAX_DATA_DRIVEN_RIG_ANIMATIONS: usize = 128;
pub(crate) const MAX_DATA_DRIVEN_RIG_ANIMATION_TARGETS: usize = 512;
pub(crate) const MAX_DATA_DRIVEN_RIG_KEYFRAMES: usize = 4_096;

static NEXT_RIG_BONE_CUBES_ID: AtomicU64 = AtomicU64::new(1);

fn allocate_rig_cubes(cubes: Vec<ModelCube>) -> ModelCubes {
    if cubes.is_empty() {
        return ModelCubes::Static(&[]);
    }
    ModelCubes::Owned {
        id: NEXT_RIG_BONE_CUBES_ID.fetch_add(1, Ordering::Relaxed),
        cubes: Arc::from(cubes),
    }
}

#[derive(Debug, Clone)]
struct BakedFlatBone {
    name: PartName,
    bind_pose: PartPose,
    bind_scale: [f32; 3],
    cubes: ModelCubes,
    child_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct OwnedAnimationChannel {
    target: AnimationTarget,
    keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone)]
struct IndexedBoneAnimation {
    bone_index: usize,
    channels: Vec<OwnedAnimationChannel>,
}

#[derive(Debug, Clone)]
struct OwnedAnimationDefinition {
    length_seconds: f32,
    looping: bool,
    bones: Vec<IndexedBoneAnimation>,
}

impl OwnedAnimationDefinition {
    fn elapsed_seconds(&self, seconds_since_start: f32) -> f32 {
        if self.looping {
            seconds_since_start.rem_euclid(self.length_seconds)
        } else {
            seconds_since_start
        }
    }

    fn walk_sample(
        &self,
        walk_animation_pos: f32,
        walk_animation_speed: f32,
        speed_factor: f32,
        scale_factor: f32,
    ) -> (f32, f32) {
        let millis = (walk_animation_pos * 50.0 * speed_factor) as i64;
        let seconds = self.elapsed_seconds(millis as f32 / 1000.0);
        let scale = (walk_animation_speed * scale_factor).min(1.0);
        (seconds, scale)
    }
}

fn convert_keyframe(kf: &DataDrivenRigKeyframe) -> Keyframe {
    Keyframe {
        timestamp: kf.timestamp.max(0.0),
        pre_target: kf.pre,
        post_target: kf.post,
        interpolation: match kf.interpolation {
            DataDrivenRigInterpolation::Linear => KeyframeInterpolation::Linear,
            DataDrivenRigInterpolation::CatmullRom => KeyframeInterpolation::CatmullRom,
        },
    }
}

fn convert_channel(
    ch: &DataDrivenRigChannel,
    total_keyframes: &mut usize,
) -> Result<OwnedAnimationChannel> {
    *total_keyframes = total_keyframes.saturating_add(ch.keyframes.len());
    if *total_keyframes > MAX_DATA_DRIVEN_RIG_KEYFRAMES {
        bail!("data-driven rig keyframe count exceeds limit {MAX_DATA_DRIVEN_RIG_KEYFRAMES}");
    }
    let mut frames: Vec<Keyframe> = ch.keyframes.iter().map(convert_keyframe).collect();
    frames.sort_by(|a, b| a.timestamp.total_cmp(&b.timestamp));
    Ok(OwnedAnimationChannel {
        target: match ch.target {
            DataDrivenRigTarget::Position => AnimationTarget::Position,
            DataDrivenRigTarget::Rotation => AnimationTarget::Rotation,
            DataDrivenRigTarget::Scale => AnimationTarget::Scale,
        },
        keyframes: frames,
    })
}

fn flatten_rig_bone<'a>(
    spec: &'a DataDrivenRigBone,
    depth: usize,
    bones: &mut Vec<BakedFlatBone>,
    bone_index_by_name: &mut HashMap<&'a str, usize>,
    total_cubes: &mut usize,
) -> Result<usize> {
    if depth >= MAX_DATA_DRIVEN_RIG_DEPTH {
        bail!("data-driven rig bone nesting depth exceeds limit {MAX_DATA_DRIVEN_RIG_DEPTH}");
    }
    if bones.len() >= MAX_DATA_DRIVEN_RIG_BONES {
        bail!("data-driven rig bone count exceeds limit {MAX_DATA_DRIVEN_RIG_BONES}");
    }
    *total_cubes = total_cubes.saturating_add(spec.cubes.len());
    if *total_cubes > MAX_DATA_DRIVEN_RIG_CUBES {
        bail!("data-driven rig cube count exceeds limit {MAX_DATA_DRIVEN_RIG_CUBES}");
    }
    let cubes = spec
        .cubes
        .iter()
        .map(|c: &DataDrivenRigCube| {
            cube(
                c.origin,
                c.size,
                [c.inflate; 3],
                c.uv,
                c.mirror.unwrap_or(spec.mirror),
                0b11_1111,
            )
        })
        .collect();
    let bind_scale = spec.scale.map(|s| if s.is_finite() { s } else { 1.0 });
    let index = bones.len();
    // Preserve DFS pre-order first-match semantics matching `ModelPart::part_mut`.
    bone_index_by_name
        .entry(spec.name.as_str())
        .or_insert(index);
    bones.push(BakedFlatBone {
        name: PartName::Shared(Arc::from(spec.name.as_str())),
        bind_pose: PartPose {
            offset: spec.pivot,
            rotation: spec.rotation_deg.map(f32::to_radians),
        },
        bind_scale,
        cubes: allocate_rig_cubes(cubes),
        child_indices: Vec::with_capacity(spec.children.len()),
    });
    let mut child_indices = Vec::with_capacity(spec.children.len());
    for child in &spec.children {
        let child_index =
            flatten_rig_bone(child, depth + 1, bones, bone_index_by_name, total_cubes)?;
        child_indices.push(child_index);
    }
    bones[index].child_indices = child_indices;
    Ok(index)
}

fn convert_animation(
    anim: &DataDrivenRigAnimation,
    bone_index_by_name: &HashMap<&str, usize>,
    total_keyframes: &mut usize,
) -> Result<(String, OwnedAnimationDefinition)> {
    if anim.bones.len() > MAX_DATA_DRIVEN_RIG_ANIMATION_TARGETS {
        bail!(
            "data-driven rig animation '{}' bone target count {} exceeds limit {MAX_DATA_DRIVEN_RIG_ANIMATION_TARGETS}",
            anim.name,
            anim.bones.len()
        );
    }
    let mut bones = Vec::new();
    for bc in &anim.bones {
        if bc.channels.is_empty() {
            continue;
        }
        let Some(&bone_index) = bone_index_by_name.get(bc.bone.as_str()) else {
            continue;
        };
        let mut channels = Vec::new();
        for ch in &bc.channels {
            if ch.keyframes.is_empty() {
                continue;
            }
            channels.push(convert_channel(ch, total_keyframes)?);
        }
        if !channels.is_empty() {
            bones.push(IndexedBoneAnimation {
                bone_index,
                channels,
            });
        }
    }
    Ok((
        anim.name.to_ascii_lowercase(),
        OwnedAnimationDefinition {
            length_seconds: anim.length_seconds.max(0.05),
            looping: anim.looping,
            bones,
        },
    ))
}

/// Renderer-side baked representation of a [`DataDrivenRigSpec`], storing a
/// flattened DFS pre-order bone table and pre-resolved animation bone indices
/// so per-frame pose evaluation runs in `O(A + B)` time.
#[derive(Debug, Clone)]
pub(crate) struct BakedDataDrivenRig {
    texture_size: [f32; 2],
    bones: Vec<BakedFlatBone>,
    root_indices: Vec<usize>,
    head_bone_index: Option<usize>,
    animations: Vec<OwnedAnimationDefinition>,
    idle_animation: Option<usize>,
    walk_animation: Option<usize>,
    enraged_animation: Option<usize>,
}

impl BakedDataDrivenRig {
    fn empty(texture_size: [u32; 2]) -> Self {
        Self {
            texture_size: [texture_size[0].max(1) as f32, texture_size[1].max(1) as f32],
            bones: Vec::new(),
            root_indices: Vec::new(),
            head_bone_index: None,
            animations: Vec::new(),
            idle_animation: None,
            walk_animation: None,
            enraged_animation: None,
        }
    }

    pub(crate) fn try_from_spec(spec: &DataDrivenRigSpec) -> Result<Self> {
        if spec.animations.len() > MAX_DATA_DRIVEN_RIG_ANIMATIONS {
            bail!(
                "data-driven rig animation count {} exceeds limit {MAX_DATA_DRIVEN_RIG_ANIMATIONS}",
                spec.animations.len()
            );
        }
        let tex_w = spec.texture_size[0].max(1) as f32;
        let tex_h = spec.texture_size[1].max(1) as f32;
        let mut bones = Vec::new();
        let mut root_indices = Vec::with_capacity(spec.bones.len());
        let mut bone_index_by_name = HashMap::new();
        let mut total_cubes = 0usize;
        for root_bone in &spec.bones {
            let root_index = flatten_rig_bone(
                root_bone,
                0,
                &mut bones,
                &mut bone_index_by_name,
                &mut total_cubes,
            )?;
            root_indices.push(root_index);
        }
        let head_bone_index = bone_index_by_name.get("head").copied();

        let mut animations = Vec::with_capacity(spec.animations.len());
        let mut idle_animation = None;
        let mut walk_animation = None;
        let mut enraged_animation = None;
        let mut total_keyframes = 0usize;
        for anim in &spec.animations {
            let (name, definition) =
                convert_animation(anim, &bone_index_by_name, &mut total_keyframes)?;
            let idx = animations.len();
            match name.as_str() {
                "idle" if idle_animation.is_none() => idle_animation = Some(idx),
                "walk" if walk_animation.is_none() => walk_animation = Some(idx),
                "enraged" if enraged_animation.is_none() => enraged_animation = Some(idx),
                _ => {}
            }
            animations.push(definition);
        }

        Ok(Self {
            texture_size: [tex_w, tex_h],
            bones,
            root_indices,
            head_bone_index,
            animations,
            idle_animation,
            walk_animation,
            enraged_animation,
        })
    }

    pub(crate) fn from_spec(spec: &DataDrivenRigSpec) -> Self {
        match Self::try_from_spec(spec) {
            Ok(rig) => rig,
            Err(error) => {
                tracing::warn!(rig_id = %spec.rig_id, %error, "rejecting oversized data-driven rig");
                Self::empty(spec.texture_size)
            }
        }
    }

    pub(in crate::entity_models) fn build_posed_root(
        &self,
        state: &EntityRenderState,
        anim_state: u8,
    ) -> ModelPart {
        let mut poses: Vec<(PartPose, [f32; 3])> = self
            .bones
            .iter()
            .map(|bone| (bone.bind_pose, bone.bind_scale))
            .collect();
        if let Some(head_idx) = self.head_bone_index {
            poses[head_idx].0.rotation[1] += state.head_yaw.to_radians();
            poses[head_idx].0.rotation[0] += state.head_pitch.to_radians();
        }
        let age_seconds = (state.age_in_ticks.max(0.0)) / 20.0;
        if let Some(idle_idx) = self.idle_animation {
            let idle = &self.animations[idle_idx];
            apply_animation_def(&mut poses, idle, idle.elapsed_seconds(age_seconds), 1.0);
        }
        if let Some(walk_idx) = self.walk_animation {
            let walk = &self.animations[walk_idx];
            let (seconds, scale) = walk.walk_sample(
                state.walk_animation_pos,
                state.walk_animation_speed,
                2.0,
                2.5,
            );
            if scale > 0.0 {
                apply_animation_def(&mut poses, walk, seconds, scale);
            }
        }
        if anim_state >= 1 {
            if let Some(enraged_idx) = self.enraged_animation {
                let enraged = &self.animations[enraged_idx];
                apply_animation_def(
                    &mut poses,
                    enraged,
                    enraged.elapsed_seconds(age_seconds),
                    1.0,
                );
            }
        }

        // Reconstruct the `ModelPart` tree bottom-up in reverse pre-order (`child_index > index`)
        // so each bone is visited once with zero recursive name lookups.
        let mut built: Vec<Option<ModelPart>> = (0..self.bones.len()).map(|_| None).collect();
        for index in (0..self.bones.len()).rev() {
            let bone = &self.bones[index];
            let (pose, scale) = poses[index];
            let children = bone
                .child_indices
                .iter()
                .map(|&child_index| {
                    (
                        self.bones[child_index].name.clone(),
                        built[child_index]
                            .take()
                            .expect("reverse pre-order builds every child before parent"),
                    )
                })
                .collect();
            built[index] = Some(
                ModelPart::new_owned(pose, bone.cubes.clone(), children, self.texture_size)
                    .with_scale(scale),
            );
        }
        let root_children = self
            .root_indices
            .iter()
            .map(|&root_index| {
                (
                    self.bones[root_index].name.clone(),
                    built[root_index].take().expect("every root bone was built"),
                )
            })
            .collect();
        ModelPart::new_owned(
            PartPose {
                offset: [0.0; 3],
                rotation: [0.0; 3],
            },
            ModelCubes::Static(&[]),
            root_children,
            self.texture_size,
        )
    }
}

fn apply_animation_def(
    poses: &mut [(PartPose, [f32; 3])],
    definition: &OwnedAnimationDefinition,
    seconds: f32,
    target_scale: f32,
) {
    for bone in &definition.bones {
        let Some((pose, scale)) = poses.get_mut(bone.bone_index) else {
            continue;
        };
        for channel in &bone.channels {
            if channel.keyframes.is_empty() {
                continue;
            }
            let value = sample_keyframe_channel(&channel.keyframes, seconds, target_scale);
            let target = match channel.target {
                AnimationTarget::Position => &mut pose.offset,
                AnimationTarget::Rotation => &mut pose.rotation,
                AnimationTarget::Scale => scale,
            };
            for axis in 0..3 {
                target[axis] += value[axis];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::geometry::EntityModelTexturedMesh;
    use crate::entity_models::EntityModelUvRect;

    fn sample_spec() -> DataDrivenRigSpec {
        serde_json::from_str(
            r#"{
                "rig_id": "bbb_demo:inferno_warden",
                "texture_size": [64, 64],
                "bones": [{
                    "name": "body",
                    "pivot": [0.0, 2.0, 0.0],
                    "cubes": [{ "origin": [-4.0, -4.0, -4.0], "size": [8.0, 8.0, 8.0], "uv": [0.0, 0.0] }],
                    "children": [{
                        "name": "head",
                        "pivot": [0.0, -4.0, 0.0],
                        "cubes": [{ "origin": [-3.0, -6.0, -3.0], "size": [6.0, 6.0, 6.0], "uv": [32.0, 0.0] }]
                    }, {
                        "name": "left_arm",
                        "pivot": [-6.0, -2.0, 0.0],
                        "cubes": [{ "origin": [-2.0, 0.0, -2.0], "size": [4.0, 12.0, 4.0], "uv": [0.0, 16.0] }]
                    }]
                }],
                "animations": [
                    {
                        "name": "idle",
                        "length_seconds": 2.0,
                        "looping": true,
                        "bones": [{
                            "bone": "body",
                            "channels": [{
                                "target": "position",
                                "keyframes": [
                                    { "timestamp": 0.0, "pre": [0.0, 0.0, 0.0], "post": [0.0, 0.0, 0.0] },
                                    { "timestamp": 1.0, "pre": [0.0, -1.0, 0.0], "post": [0.0, -1.0, 0.0] }
                                ]
                            }]
                        }]
                    },
                    {
                        "name": "enraged",
                        "length_seconds": 1.0,
                        "looping": true,
                        "bones": [{
                            "bone": "left_arm",
                            "channels": [{
                                "target": "rotation",
                                "keyframes": [
                                    { "timestamp": 0.0, "pre": [-1.2, 0.0, 0.0], "post": [-1.2, 0.0, 0.0] }
                                ]
                            }]
                        }]
                    }
                ]
            }"#,
        )
        .expect("valid sample rig")
    }

    #[test]
    fn baked_rig_evaluates_head_idle_enraged_and_emits_retained_parts() {
        let spec = sample_spec();
        let baked = BakedDataDrivenRig::from_spec(&spec);
        let cloned = baked.clone();
        // Cloning a baked rig shares the Arc-backed cube slice without leaking 'static memory.
        assert_eq!(baked.bones[0].cubes, cloned.bones[0].cubes);
        assert!(std::ptr::eq(
            baked.bones[0].cubes.as_slice().as_ptr(),
            cloned.bones[0].cubes.as_slice().as_ptr()
        ));

        // Baking past the former 4,096 static arena cap continues producing intact bones/animations.
        for _ in 0..4_100 {
            let _ = BakedDataDrivenRig::from_spec(&spec);
        }
        let after_many = BakedDataDrivenRig::from_spec(&spec);
        assert_eq!(after_many.bones[0].cubes.as_slice().len(), 1);
        assert_eq!(after_many.animations.len(), 2);

        let state = EntityRenderState {
            age_in_ticks: 20.0, // 1.0 second into idle animation
            head_yaw: 45.0,
            ..EntityRenderState::default()
        };

        let mut idle_root = baked.build_posed_root(&state, 0);
        let head = idle_root.part_mut("head").expect("head bone");
        assert!((head.pose.rotation[1] - 45.0_f32.to_radians()).abs() < 1e-4);
        let body_idle = idle_root.part_mut("body").expect("body bone");
        assert!((body_idle.pose.offset[1] - 1.0).abs() < 1e-4);

        let mut enraged_root = baked.build_posed_root(&state, 1);
        let arm_enraged = enraged_root.part_mut("left_arm").expect("left_arm bone");
        assert!((arm_enraged.pose.rotation[0] + 1.2).abs() < 1e-4);

        let mut mesh = EntityModelTexturedMesh {
            retain_parts: true,
            ..EntityModelTexturedMesh::default()
        };
        enraged_root.render_textured(
            &mut mesh,
            glam::Mat4::IDENTITY,
            EntityModelUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            [1.0; 4],
        );
        // 3 cubes (body + head + left_arm) retain 3 GPU ModelPartDraw jobs.
        assert_eq!(mesh.parts.len(), 3);
    }

    #[test]
    fn baked_rig_rejects_over_cap_specs_and_evaluates_flattened_bones_in_linear_time() {
        use bbb_render_types::{
            DataDrivenRigAnimation, DataDrivenRigBone, DataDrivenRigBoneChannels,
            DataDrivenRigChannel, DataDrivenRigInterpolation, DataDrivenRigKeyframe,
            DataDrivenRigTarget,
        };

        let make_bone = |name: String| DataDrivenRigBone {
            name,
            pivot: [0.0; 3],
            rotation_deg: [0.0; 3],
            scale: [1.0; 3],
            mirror: false,
            cubes: Vec::new(),
            children: Vec::new(),
        };

        // R23-05: exceeding the bone cap is rejected cleanly without panicking.
        let too_many_bones = DataDrivenRigSpec {
            rig_id: "test:too_many_bones".into(),
            texture_asset_id: None,
            emissive_texture_asset_id: None,
            shadow_radius: Some(0.5),
            texture_size: [64, 64],
            bones: (0..=MAX_DATA_DRIVEN_RIG_BONES)
                .map(|i| make_bone(format!("b{i}")))
                .collect(),
            animations: Vec::new(),
        };
        assert!(BakedDataDrivenRig::try_from_spec(&too_many_bones).is_err());
        let degraded = BakedDataDrivenRig::from_spec(&too_many_bones);
        assert!(degraded.bones.is_empty());

        // Large valid rig at cap (512 bones + 512 animation targets) poses accurately in O(A + B).
        let at_cap = DataDrivenRigSpec {
            rig_id: "test:at_cap".into(),
            texture_asset_id: None,
            emissive_texture_asset_id: None,
            shadow_radius: Some(0.5),
            texture_size: [64, 64],
            bones: (0..MAX_DATA_DRIVEN_RIG_BONES)
                .map(|i| make_bone(format!("bone_{i}")))
                .collect(),
            animations: vec![DataDrivenRigAnimation {
                name: "idle".into(),
                length_seconds: 1.0,
                looping: true,
                bones: (0..MAX_DATA_DRIVEN_RIG_ANIMATION_TARGETS)
                    .map(|i| DataDrivenRigBoneChannels {
                        bone: format!("bone_{i}"),
                        channels: vec![DataDrivenRigChannel {
                            target: DataDrivenRigTarget::Position,
                            keyframes: vec![DataDrivenRigKeyframe {
                                timestamp: 0.0,
                                pre: [1.0, 2.0, 3.0],
                                post: [1.0, 2.0, 3.0],
                                interpolation: DataDrivenRigInterpolation::Linear,
                            }],
                        }],
                    })
                    .collect(),
            }],
        };
        let baked = BakedDataDrivenRig::try_from_spec(&at_cap).expect("at-cap rig bakes");
        let mut posed = baked.build_posed_root(&EntityRenderState::default(), 0);
        let last = posed
            .part_mut(&format!("bone_{}", MAX_DATA_DRIVEN_RIG_BONES - 1))
            .expect("last bone");
        assert_eq!(last.pose.offset, [1.0, 2.0, 3.0]);
    }
}
