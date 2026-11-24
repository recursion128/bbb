//! Baked special-item geometry keyed by the special model value and the
//! resource/atlas generations its texture rectangles were resolved against.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::discriminant;
use std::sync::Arc;

use super::ItemModelQuad;
use crate::item_render::item_models::residency::{
    ItemModelAssetIdentity, ItemModelAtlasKind, ItemModelDrawPhase, ItemModelGeometryClass,
};
use crate::{SpecialItemModel, SpecialItemPatternLayer, SpecialItemTexture};

/// Entries unused for this many frames are dropped; animated models (chest
/// lids, book pages) otherwise leave one entry per sampled pose.
const SPECIAL_GEOMETRY_IDLE_FRAMES: u64 = 2;

pub(crate) type SpecialGeometryGeneration = [u64; 6];

pub(crate) struct SpecialItemResidentPart {
    pub(crate) identity: ItemModelAssetIdentity,
    pub(crate) semantic_part: Arc<str>,
    pub(crate) atlas: ItemModelAtlasKind,
    pub(crate) geometry_class: ItemModelGeometryClass,
    pub(crate) phase: ItemModelDrawPhase,
    pub(crate) quads: Arc<[ItemModelQuad]>,
    pub(crate) tint: [f32; 4],
    pub(crate) accepts_foil: bool,
}

struct CachedSpecialItemGeometry {
    model: SpecialItemModel,
    generation: SpecialGeometryGeneration,
    parts: Arc<[SpecialItemResidentPart]>,
    last_used_frame: u64,
}

/// Cached parts bucketed by a hash of the typed model value and generation;
/// a bucket holds the (rare) hash collisions, told apart by `PartialEq`.
#[derive(Default)]
pub(crate) struct SpecialItemGeometryCache {
    buckets: HashMap<u64, Vec<CachedSpecialItemGeometry>>,
    swept_frame: u64,
}

impl SpecialItemGeometryCache {
    pub(crate) fn get(
        &mut self,
        model: &SpecialItemModel,
        generation: SpecialGeometryGeneration,
        frame: u64,
    ) -> Option<Arc<[SpecialItemResidentPart]>> {
        self.sweep(frame);
        let entry = self
            .buckets
            .get_mut(&cache_hash(model, generation))?
            .iter_mut()
            .find(|entry| entry.generation == generation && entry.model == *model)?;
        entry.last_used_frame = frame;
        Some(Arc::clone(&entry.parts))
    }

    pub(crate) fn insert(
        &mut self,
        model: SpecialItemModel,
        generation: SpecialGeometryGeneration,
        frame: u64,
        parts: Arc<[SpecialItemResidentPart]>,
    ) {
        self.sweep(frame);
        self.buckets
            .entry(cache_hash(&model, generation))
            .or_default()
            .push(CachedSpecialItemGeometry {
                model,
                generation,
                parts,
                last_used_frame: frame,
            });
    }

    /// Drops idle entries once per frame.
    fn sweep(&mut self, frame: u64) {
        if self.swept_frame == frame {
            return;
        }
        self.swept_frame = frame;
        self.buckets.retain(|_, bucket| {
            bucket.retain(|entry| entry.last_used_frame + SPECIAL_GEOMETRY_IDLE_FRAMES >= frame);
            !bucket.is_empty()
        });
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.buckets.values().map(Vec::len).sum()
    }
}

/// Hashes the fields that tell special models apart. Every field compared by
/// `PartialEq` that is cheap to hash is covered; equal values hash equally
/// except for float zero signs, which only cost a cache miss.
fn cache_hash(model: &SpecialItemModel, generation: SpecialGeometryGeneration) -> u64 {
    let mut hasher = DefaultHasher::new();
    generation.hash(&mut hasher);
    discriminant(model).hash(&mut hasher);
    let h = &mut hasher;
    match model {
        SpecialItemModel::Bed { texture, part } => {
            hash_texture(texture, h);
            discriminant(part).hash(h);
        }
        SpecialItemModel::Bell { texture }
        | SpecialItemModel::Conduit { texture }
        | SpecialItemModel::Trident { texture } => hash_texture(texture, h),
        SpecialItemModel::Banner {
            attachment,
            base_texture,
            pattern_base_texture,
            base_color,
            patterns,
        } => {
            discriminant(attachment).hash(h);
            hash_texture(base_texture, h);
            hash_texture(pattern_base_texture, h);
            discriminant(base_color).hash(h);
            hash_patterns(patterns, h);
        }
        SpecialItemModel::Book {
            texture,
            open_angle_radians,
            page1,
            page2,
        } => {
            hash_texture(texture, h);
            [*open_angle_radians, *page1, *page2]
                .map(f32::to_bits)
                .hash(h);
        }
        SpecialItemModel::Chest {
            texture,
            openness,
            half,
        } => {
            hash_texture(texture, h);
            openness.to_bits().hash(h);
            discriminant(half).hash(h);
        }
        SpecialItemModel::CopperGolemStatue { texture, pose } => {
            hash_texture(texture, h);
            discriminant(pose).hash(h);
        }
        SpecialItemModel::Head {
            texture,
            kind,
            animation,
        } => {
            hash_texture(texture, h);
            discriminant(kind).hash(h);
            animation.to_bits().hash(h);
        }
        SpecialItemModel::PlayerHead {
            skin,
            fallback_texture,
        } => {
            skin.map(|skin| skin.handle).hash(h);
            fallback_texture.path.hash(h);
        }
        SpecialItemModel::ShulkerBox { texture, openness } => {
            hash_texture(texture, h);
            openness.to_bits().hash(h);
        }
        SpecialItemModel::Shield {
            base_texture,
            pattern_base_texture,
            base_color,
            patterns,
        } => {
            hash_texture(base_texture, h);
            if let Some(texture) = pattern_base_texture {
                hash_texture(texture, h);
            }
            base_color.map(|color| discriminant(&color)).hash(h);
            hash_patterns(patterns, h);
        }
        SpecialItemModel::DecoratedPot {
            base_texture,
            sides,
            decorations,
        } => {
            hash_texture(base_texture, h);
            for side in sides {
                hash_texture(side, h);
            }
            decorations
                .map(|pattern| pattern.map(|pattern| discriminant(&pattern)))
                .hash(h);
        }
        SpecialItemModel::StandingSign {
            texture,
            attachment,
        }
        | SpecialItemModel::HangingSign {
            texture,
            attachment,
        } => {
            hash_texture(texture, h);
            discriminant(attachment).hash(h);
        }
        SpecialItemModel::EndCube {
            sky_texture,
            portal_texture,
            effect,
        } => {
            hash_texture(sky_texture, h);
            hash_texture(portal_texture, h);
            discriminant(effect).hash(h);
        }
    }
    hasher.finish()
}

fn hash_texture(texture: &SpecialItemTexture, h: &mut DefaultHasher) {
    discriminant(texture).hash(h);
    match texture {
        SpecialItemTexture::Static(texture) => texture.path.hash(h),
        SpecialItemTexture::Pack(texture) => texture.hash(h),
        SpecialItemTexture::Dynamic {
            texture,
            logical_size,
        } => {
            texture.handle.hash(h);
            logical_size.hash(h);
        }
        SpecialItemTexture::DynamicPlayerSkin(skin) => skin.handle.hash(h),
    }
}

fn hash_patterns(patterns: &[SpecialItemPatternLayer], h: &mut DefaultHasher) {
    patterns.len().hash(h);
    for layer in patterns {
        hash_texture(&layer.texture, h);
        discriminant(&layer.color).hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityModelOwnedTextureRef;

    fn shulker(path: &str) -> SpecialItemModel {
        SpecialItemModel::ShulkerBox {
            texture: SpecialItemTexture::Pack(EntityModelOwnedTextureRef::new(
                Arc::from(path),
                [64, 64],
            )),
            openness: 0.0,
        }
    }

    #[test]
    fn lookups_hit_by_value_and_generation_and_idle_entries_expire() {
        let mut cache = SpecialItemGeometryCache::default();
        let generation = [1; 6];
        let parts: Arc<[SpecialItemResidentPart]> = Arc::from(Vec::new());
        for index in 0..64 {
            cache.insert(
                shulker(&format!("textures/entity/shulker/{index}.png")),
                generation,
                1,
                Arc::clone(&parts),
            );
        }
        assert!(cache
            .get(&shulker("textures/entity/shulker/7.png"), generation, 1)
            .is_some());
        assert!(cache
            .get(&shulker("textures/entity/shulker/7.png"), [2; 6], 1)
            .is_none());
        assert!(cache
            .get(&shulker("textures/entity/shulker/x.png"), generation, 1)
            .is_none());

        // Entry 7 stays in use; the rest go idle past the window.
        assert!(cache
            .get(&shulker("textures/entity/shulker/7.png"), generation, 3)
            .is_some());
        assert!(cache
            .get(&shulker("textures/entity/shulker/7.png"), generation, 4)
            .is_some());
        assert_eq!(cache.len(), 1);
    }
}
