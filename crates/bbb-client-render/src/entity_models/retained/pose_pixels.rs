use crate::entity_models::model_layers::{
    BAT_TEXTURE_REF, COW_TEMPERATE_TEXTURE_REF, MINECART_TEXTURE_REF,
};
use crate::{
    CameraPose, EntityModelInstance, EntityModelTextureImage, Renderer, RendererCameraFrame,
};

fn camera() -> RendererCameraFrame {
    RendererCameraFrame {
        pose: Some(CameraPose {
            position: [0.0, 0.32, -2.2],
            eye_height: 0.0,
            y_rot: 0.0,
            x_rot: 0.0,
        }),
        ..Default::default()
    }
}

fn setup() -> Option<Renderer> {
    let mut r = Renderer::new_offscreen(320, 240)?;
    let pixels = (0..32 * 32)
        .flat_map(|i| {
            [
                160 + (i % 72) as u8,
                (i % 183) as u8,
                75,
                if i % 13 == 0 { 0 } else { 255 },
            ]
        })
        .collect();
    r.upload_entity_model_textures(&[EntityModelTextureImage::new(BAT_TEXTURE_REF, pixels)])
        .unwrap();
    r.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    });
    Some(r)
}

fn bats(count: usize, tick: usize, outline: bool) -> Vec<EntityModelInstance> {
    let t = (tick as f32 + 1.0) / 16.0;
    (0..count)
        .map(|i| {
            let x = ((i % 3) as f32 - (count.min(3) - 1) as f32 / 2.0) * 0.72;
            let bat = EntityModelInstance::bat(
                i as i32 + 1,
                [x, 0.04 * ((tick + i) as f32).sin(), (i / 3) as f32 * 0.9],
                (tick as f32 * 3.0) % 10.0,
            )
            .with(|s| {
                s.kind.bat_mut().flying_seconds = t + i as f32 * 0.17;
                s.age_in_ticks = t * 20.0;
                s.light_coords = (5 << 4) | (11 << 20);
            });
            if outline && i == 1 {
                bat.with_outline_color(0xff4411cc)
            } else {
                bat
            }
        })
        .collect()
}

#[test]
fn moving_cuboids_keep_lit_cutout_pixels_across_mixed_and_reordered_draws() {
    let Some(mut renderer) = setup() else { return };
    let Some(pipelines) = renderer.entity_pose_pipelines.take() else {
        return;
    };
    let frames = [
        (1, false, false),
        (2, false, false),
        (3, true, false),
        (4, false, false),
        (5, false, true),
        (6, false, true),
    ];
    let render = |r: &mut Renderer, (tick, outline, reverse)| {
        let mut scene = bats(2, tick, outline);
        if reverse {
            scene.reverse();
        }
        r.replace_scene_frame(camera(), scene);
        r.render_offscreen_frame().unwrap().rgba
    };
    let reference = frames.map(|frame| render(&mut renderer, frame));
    renderer.set_entity_model_instances(Vec::new());
    renderer.entity_pose_pipelines = Some(pipelines);
    for (frame, expected) in frames.into_iter().zip(reference) {
        let actual = render(&mut renderer, frame);
        assert!(expected
            .as_chunks::<4>()
            .0
            .iter()
            .any(|&p| p != [0, 0, 255, 255]));
        for (&a, &b) in actual
            .as_chunks::<4>()
            .0
            .iter()
            .zip(expected.as_chunks::<4>().0)
        {
            assert_eq!(
                a == [0, 0, 255, 255],
                b == [0, 0, 255, 255],
                "alpha cutout coverage"
            );
            assert!(
                a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1),
                "{a:?} vs {b:?}"
            );
        }
        let mesh = renderer.pack.entity_meshes[crate::renderer::EntityMeshLane::TexturedCull]
            .as_ref()
            .unwrap();
        if frame.1 {
            assert!(mesh.vertex_pose.is_none());
        }
        if frame.0 == 2 || frame.0 == 6 {
            assert!(mesh.vertex_pose.is_some());
        }
    }
}

#[test]
fn minecart_first_person_pixels_stay_identical_across_world_coordinates_in_npot_atlas() {
    let Some(mut renderer) = Renderer::new_offscreen(640, 360) else {
        return;
    };
    let minecart_pixels = (0..(64 * 32))
        .flat_map(|i| {
            let x = (i % 64) as u8;
            let y = (i / 64) as u8;
            [
                x.wrapping_mul(37).wrapping_add(y.wrapping_mul(13)),
                x.wrapping_mul(73).wrapping_add(y.wrapping_mul(29)),
                x.wrapping_mul(19).wrapping_add(y.wrapping_mul(61)),
                255,
            ]
        })
        .collect();
    renderer
        .upload_entity_model_textures(&[
            EntityModelTextureImage::new(BAT_TEXTURE_REF, vec![220; 32 * 32 * 4]),
            EntityModelTextureImage::new(COW_TEMPERATE_TEXTURE_REF, vec![180; 64 * 64 * 4]),
            EntityModelTextureImage::new(MINECART_TEXTURE_REF, minecart_pixels),
        ])
        .unwrap();
    renderer.set_clear_color(crate::ClearColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });

    let xs = [-124.5_f32, -124.481, -87.319, 0.0, 119.5];
    let render_at = |renderer: &mut Renderer, x: f32| {
        let cart = EntityModelInstance::minecart(42, [x, 120.0625, -48.5], 0.0).with(|s| {
            s.kind.minecart_mut().new_render = true;
            s.light_coords = (15 << 4) | (15 << 20);
        });
        renderer.replace_scene_frame(
            RendererCameraFrame {
                pose: Some(CameraPose {
                    position: [x, 119.5, -48.5],
                    eye_height: 1.62,
                    y_rot: -90.0,
                    x_rot: 20.0,
                }),
                ..Default::default()
            },
            vec![cart],
        );
        renderer.render_offscreen_frame().unwrap().rgba
    };

    let _ = render_at(&mut renderer, -125.0);
    let baseline = render_at(&mut renderer, xs[0]);
    assert!(baseline
        .as_chunks::<4>()
        .0
        .iter()
        .any(|&p| p != [0, 0, 0, 255]));
    for &x in &xs[1..] {
        let actual = render_at(&mut renderer, x);
        assert!(
            renderer.pack.entity_meshes[crate::renderer::EntityMeshLane::Textured]
                .as_ref()
                .unwrap()
                .vertex_pose
                .is_some()
        );
        assert_eq!(actual, baseline, "minecart pixels drifted at x={x}");
    }
}

#[test]
fn pose_binding_rebinds_when_other_lane_grows_local_geometry() {
    let Some(mut renderer) = setup() else { return };
    if renderer.entity_pose_pipelines.is_none() {
        return;
    }
    renderer
        .upload_entity_model_textures(&[
            EntityModelTextureImage::new(BAT_TEXTURE_REF, vec![220; 32 * 32 * 4]),
            EntityModelTextureImage::new(COW_TEMPERATE_TEXTURE_REF, vec![180; 64 * 64 * 4]),
            EntityModelTextureImage::new(MINECART_TEXTURE_REF, vec![160; 64 * 32 * 4]),
        ])
        .unwrap();
    renderer.replace_scene_frame(camera(), bats(2, 1, false));
    renderer.render_offscreen_frame().unwrap();
    renderer.replace_scene_frame(camera(), bats(2, 2, false));
    renderer.render_offscreen_frame().unwrap();

    let initial_rev = renderer.pack.entity_geometry.geometry_revision();
    assert_eq!(
        renderer.pack.entity_meshes[crate::renderer::EntityMeshLane::TexturedCull]
            .as_ref()
            .and_then(|gpu| gpu.vertex_pose.as_ref())
            .expect("bats entered vertex_pose path")
            .geometry_revision(),
        initial_rev
    );

    // Add distinct entities in other lanes so `entity_geometry` reallocates its storage buffer
    // while `TexturedCull` stays on the `vertex_pose` path.
    let mut scene = bats(2, 3, false);
    scene.extend([
        EntityModelInstance::cow(100, [1.5, 0.0, 2.0], 0.0, false),
        EntityModelInstance::cow(101, [-1.5, 0.0, 2.0], 0.0, true),
        EntityModelInstance::minecart(102, [0.0, 0.0, 2.5], 0.0),
    ]);
    renderer.replace_scene_frame(camera(), scene);
    renderer.render_offscreen_frame().unwrap();

    let grown_rev = renderer.pack.entity_geometry.geometry_revision();
    assert!(
        grown_rev > initial_rev,
        "expected entity_geometry buffer growth ({initial_rev} -> {grown_rev})"
    );
    let pose_rev = renderer.pack.entity_meshes[crate::renderer::EntityMeshLane::TexturedCull]
        .as_ref()
        .and_then(|gpu| gpu.vertex_pose.as_ref())
        .expect("bats remained on vertex_pose path")
        .geometry_revision();
    assert_eq!(
        pose_rev, grown_rev,
        "pose bind group must rebind to the current local-geometry buffer after growth"
    );
}
