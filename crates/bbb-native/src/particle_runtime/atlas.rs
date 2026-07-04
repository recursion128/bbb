use super::*;

#[derive(Debug, Clone)]
pub(super) struct NativeParticleAtlas {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba: Vec<u8>,
    pub(super) sprite_uvs: Vec<ParticleSpriteUv>,
    animation: Option<NativeParticleAtlasAnimation>,
}

impl NativeParticleAtlas {
    pub(super) fn has_animation(&self) -> bool {
        self.animation.is_some()
    }

    pub(super) fn animation_atlas_frame(
        &self,
        tick: u64,
    ) -> Result<Option<NativeParticleAtlasFrame>> {
        self.animation
            .as_ref()
            .map(|animation| animation.atlas_frame(tick))
            .transpose()
    }
}

#[derive(Debug, Clone)]
struct NativeParticleAtlasAnimation {
    packer: AtlasPacker,
    images: Vec<SpriteImage>,
}

impl NativeParticleAtlasAnimation {
    fn new(packer: AtlasPacker, images: Vec<SpriteImage>) -> Option<Self> {
        images
            .iter()
            .any(|image| image.animation.is_some())
            .then_some(Self { packer, images })
    }

    fn atlas_frame(&self, tick: u64) -> Result<NativeParticleAtlasFrame> {
        let atlas = self.packer.stitch_animation_frame(&self.images, tick)?;
        Ok(NativeParticleAtlasFrame {
            width: atlas.layout.width,
            height: atlas.layout.height,
            rgba: atlas.rgba,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NativeParticleAtlasFrame {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) rgba: Vec<u8>,
}

pub(super) fn particle_atlas_from_images(images: Vec<SpriteImage>) -> Result<NativeParticleAtlas> {
    let packer = AtlasPacker::new(4096, 1)?;
    let atlas = packer.stitch(&images)?;
    let sprite_uvs = atlas
        .layout
        .sprites
        .iter()
        .map(|sprite| ParticleSpriteUv {
            id: sprite.id.clone(),
            uv: particle_uv_rect(&atlas.layout, sprite),
            has_translucent: sprite.transparency.has_translucent,
        })
        .collect();
    Ok(NativeParticleAtlas {
        width: atlas.layout.width,
        height: atlas.layout.height,
        rgba: atlas.rgba,
        sprite_uvs,
        animation: NativeParticleAtlasAnimation::new(packer, images),
    })
}

pub(super) fn advance_particle_texture_animation_tick(
    runtime: &mut NativeParticleRuntime,
    now: Instant,
) -> Option<u64> {
    let Some(last) = runtime.last_texture_animation_at else {
        runtime.last_texture_animation_at = Some(now);
        return None;
    };
    let elapsed = now.saturating_duration_since(last);
    let ticks = elapsed.as_millis() / PARTICLE_TEXTURE_ANIMATION_INTERVAL.as_millis();
    if ticks == 0 {
        return None;
    }

    let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
    runtime.texture_animation_tick = runtime.texture_animation_tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(PARTICLE_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    runtime.last_texture_animation_at = last.checked_add(advanced).or(Some(now));
    Some(runtime.texture_animation_tick)
}

fn particle_uv_rect(layout: &AtlasLayout, sprite: &AtlasSprite) -> ParticleUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    ParticleUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}
