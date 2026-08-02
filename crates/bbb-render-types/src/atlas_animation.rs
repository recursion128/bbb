use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasMipPatch {
    pub level: u32,
    pub rect: AtlasRect,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct AtlasGpuAnimationPlan {
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub padding: u32,
    pub mip_level: u32,
    pub sprites: Vec<AtlasGpuAnimationSprite>,
}

#[derive(Debug, Clone)]
pub struct AtlasGpuAnimationSprite {
    pub id: String,
    pub destination: AtlasRect,
    pub content_width: u32,
    pub content_height: u32,
    pub animation: SpriteAnimation,
    pub frames: Vec<AtlasGpuAnimationFrame>,
}

#[derive(Debug, Clone)]
pub struct AtlasGpuAnimationFrame {
    pub index: u32,
    pub mips: Vec<SpriteMipLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteMipLevel {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteAnimation {
    pub frame_count: u32,
    pub default_frame_time: u32,
    pub interpolate: bool,
    pub frames: Vec<SpriteAnimationFrame>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteAnimationFrame {
    pub index: u32,
    pub time: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteAnimationFrameTick {
    pub frame_index: u32,
    pub next_frame_index: u32,
    pub frame_progress: f32,
}

impl SpriteAnimation {
    pub fn frame_index_at_tick(&self, tick: u64) -> Option<u32> {
        self.frame_at_tick(tick).map(|frame| frame.frame_index)
    }

    pub fn frame_at_tick(&self, tick: u64) -> Option<SpriteAnimationFrameTick> {
        if self.frames.is_empty() {
            return None;
        }
        let total_duration = self.frames.iter().try_fold(0u64, |total, frame| {
            total.checked_add(u64::from(frame.time))
        })?;
        if total_duration == 0 {
            return None;
        }

        let mut remaining = tick % total_duration;
        for (position, frame) in self.frames.iter().enumerate() {
            let duration = u64::from(frame.time);
            if remaining < duration {
                let next = self.frames[(position + 1) % self.frames.len()].index;
                return Some(SpriteAnimationFrameTick {
                    frame_index: frame.index,
                    next_frame_index: next,
                    frame_progress: remaining as f32 / duration as f32,
                });
            }
            remaining -= duration;
        }
        self.frames.last().map(|frame| SpriteAnimationFrameTick {
            frame_index: frame.index,
            next_frame_index: self.frames[0].index,
            frame_progress: 0.0,
        })
    }
}
