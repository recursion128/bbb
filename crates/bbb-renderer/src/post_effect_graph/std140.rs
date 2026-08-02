use bbb_render_types::{PostEffectUniform, PostEffectUniformValue};

use super::PostEffectGlobals;

pub(super) fn pack_uniforms(uniforms: &[PostEffectUniform]) -> Vec<u8> {
    let mut writer = Std140Writer::default();
    for uniform in uniforms {
        match &uniform.value {
            PostEffectUniformValue::Int(value) => writer.write_i32(*value),
            PostEffectUniformValue::IVec3(value) => writer.write_ivec3(*value),
            PostEffectUniformValue::Float(value) => writer.write_f32(*value),
            PostEffectUniformValue::Vec2(value) => writer.write_vec2(*value),
            PostEffectUniformValue::Vec3(value) => writer.write_vec3(*value),
            PostEffectUniformValue::Vec4(value) => writer.write_vec4(*value),
            PostEffectUniformValue::Matrix4x4(value) => writer.write_matrix4x4(*value),
        }
    }
    writer.finish()
}

pub(super) fn pack_sampler_info(output_size: [u32; 2], input_sizes: &[[u32; 2]]) -> Vec<u8> {
    let mut writer = Std140Writer::default();
    writer.write_vec2([output_size[0] as f32, output_size[1] as f32]);
    for size in input_sizes {
        writer.write_vec2([size[0] as f32, size[1] as f32]);
    }
    writer.finish()
}

pub(super) fn pack_globals(globals: PostEffectGlobals, screen_size: [u32; 2]) -> Vec<u8> {
    let mut writer = Std140Writer::default();
    writer.write_ivec3(globals.camera_block_pos);
    writer.write_vec3(globals.camera_offset);
    writer.write_vec2([screen_size[0] as f32, screen_size[1] as f32]);
    writer.write_f32(globals.glint_alpha);
    writer.write_f32(globals.game_time);
    writer.write_i32(globals.menu_blur_radius);
    writer.write_i32(i32::from(globals.use_rgss));
    writer.finish()
}

#[derive(Debug, Default)]
struct Std140Writer {
    bytes: Vec<u8>,
}

impl Std140Writer {
    fn align(&mut self, alignment: usize) {
        let aligned = self.bytes.len().div_ceil(alignment) * alignment;
        self.bytes.resize(aligned, 0);
    }

    fn write_i32(&mut self, value: i32) {
        self.align(4);
        self.bytes.extend_from_slice(&value.to_ne_bytes());
    }

    fn write_f32(&mut self, value: f32) {
        self.align(4);
        self.bytes.extend_from_slice(&value.to_ne_bytes());
    }

    fn write_ivec3(&mut self, value: [i32; 3]) {
        self.align(16);
        for component in value {
            self.bytes.extend_from_slice(&component.to_ne_bytes());
        }
        self.bytes.extend_from_slice(&0_i32.to_ne_bytes());
    }

    fn write_vec2(&mut self, value: [f32; 2]) {
        self.align(8);
        for component in value {
            self.bytes.extend_from_slice(&component.to_ne_bytes());
        }
    }

    fn write_vec3(&mut self, value: [f32; 3]) {
        self.align(16);
        for component in value {
            self.bytes.extend_from_slice(&component.to_ne_bytes());
        }
        self.bytes.extend_from_slice(&0.0_f32.to_ne_bytes());
    }

    fn write_vec4(&mut self, value: [f32; 4]) {
        self.align(16);
        for component in value {
            self.bytes.extend_from_slice(&component.to_ne_bytes());
        }
    }

    fn write_matrix4x4(&mut self, value: [f32; 16]) {
        self.align(16);
        for column in 0..4 {
            for row in 0..4 {
                self.bytes
                    .extend_from_slice(&value[row * 4 + column].to_ne_bytes());
            }
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bytes.is_empty() {
            self.bytes.resize(4, 0);
        }
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use bbb_render_types::{PostEffectUniform, PostEffectUniformValue};

    use super::*;

    #[test]
    fn uniform_packing_matches_vanilla_std140_alignment_and_matrix_order() {
        let bytes = pack_uniforms(&[
            PostEffectUniform {
                name: "A".to_string(),
                value: PostEffectUniformValue::Float(1.0),
            },
            PostEffectUniform {
                name: "B".to_string(),
                value: PostEffectUniformValue::Vec3([2.0, 3.0, 4.0]),
            },
            PostEffectUniform {
                name: "C".to_string(),
                value: PostEffectUniformValue::Int(5),
            },
            PostEffectUniform {
                name: "M".to_string(),
                value: PostEffectUniformValue::Matrix4x4([
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0,
                    15.0, 16.0,
                ]),
            },
        ]);

        assert_eq!(bytes.len(), 112);
        assert_eq!(f32_at(&bytes, 0), 1.0);
        assert_eq!(
            [f32_at(&bytes, 16), f32_at(&bytes, 20), f32_at(&bytes, 24)],
            [2.0, 3.0, 4.0]
        );
        assert_eq!(i32_at(&bytes, 32), 5);
        assert_eq!(
            (0..16)
                .map(|index| f32_at(&bytes, 48 + index * 4))
                .collect::<Vec<_>>(),
            vec![
                1.0, 5.0, 9.0, 13.0, 2.0, 6.0, 10.0, 14.0, 3.0, 7.0, 11.0, 15.0, 4.0, 8.0, 12.0,
                16.0,
            ]
        );
    }

    fn f32_at(bytes: &[u8], offset: usize) -> f32 {
        f32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn i32_at(bytes: &[u8], offset: usize) -> i32 {
        i32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }
}
