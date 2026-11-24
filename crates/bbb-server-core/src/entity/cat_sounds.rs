//! Published cat soundsets.

use bbb_protocol::packets::SoundEventHolder;

use super::{Entity, EntityData, EntityStore};

pub(crate) enum CatSound {
    Ambient,
    StrayAmbient,
    Purreow,
    Eat,
    Hiss,
    Purr,
    BegForFood,
}

pub(crate) fn resolve(
    store: &EntityStore,
    cat: &Entity,
    sound: CatSound,
) -> Option<SoundEventHolder> {
    let resolver = store.item_resolver();
    let name = super::general_living::sound_variant_resource_id(cat)?;
    let holder_id = resolver.holder_id("minecraft:cat_sound_variant", name)?;
    let variant = resolver.cat_sound_variant(holder_id)?;
    let EntityData::GeneralLiving(data) = cat.data() else {
        return None;
    };
    // `Cat.getSoundSet` (`Cat.java:133-135`).
    let set = if data.ageable_retained().is_some_and(|age| age.is_baby()) {
        &variant.baby_sounds
    } else {
        &variant.adult_sounds
    };
    let sound = match sound {
        CatSound::Ambient => &set.ambient_sound,
        CatSound::StrayAmbient => &set.stray_ambient_sound,
        CatSound::Purreow => &set.purreow_sound,
        CatSound::Eat => &set.eat_sound,
        CatSound::Hiss => &set.hiss_sound,
        CatSound::Purr => &set.purr_sound,
        CatSound::BegForFood => &set.beg_for_food_sound,
    };
    super::sound_variants::sound_holder(sound)
}
