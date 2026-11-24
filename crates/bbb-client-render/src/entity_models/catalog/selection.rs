use super::super::model_layers::*;
use super::*;

pub(in crate::entity_models) fn chicken_texture_ref(
    variant: ChickenModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    variant.builtin_texture_ref(baby)
}

pub(in crate::entity_models) fn pig_texture_ref(
    variant: PigModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    variant.builtin_texture_ref(baby)
}

pub(in crate::entity_models) fn cow_texture_ref(
    variant: CowModelVariant,
    baby: bool,
) -> EntityModelTextureRef {
    variant.builtin_texture_ref(baby)
}

pub(in crate::entity_models) fn villager_type_texture_ref(
    villager_type: VillagerModelType,
    baby: bool,
) -> EntityModelTextureRef {
    let refs = if baby {
        &VILLAGER_BABY_TYPE_TEXTURE_REFS
    } else {
        &VILLAGER_TYPE_TEXTURE_REFS
    };
    refs[villager_type.vanilla_registry_index()]
}

pub(in crate::entity_models) fn zombie_villager_type_texture_ref(
    villager_type: VillagerModelType,
    baby: bool,
) -> EntityModelTextureRef {
    let refs = if baby {
        &ZOMBIE_VILLAGER_BABY_TYPE_TEXTURE_REFS
    } else {
        &ZOMBIE_VILLAGER_TYPE_TEXTURE_REFS
    };
    refs[villager_type.vanilla_registry_index()]
}

pub(in crate::entity_models) fn villager_profession_texture_ref(
    profession: VillagerModelProfession,
) -> Option<EntityModelTextureRef> {
    profession
        .texture_index()
        .map(|index| VILLAGER_PROFESSION_TEXTURE_REFS[index])
}

pub(in crate::entity_models) fn zombie_villager_profession_texture_ref(
    profession: VillagerModelProfession,
) -> Option<EntityModelTextureRef> {
    profession
        .texture_index()
        .map(|index| ZOMBIE_VILLAGER_PROFESSION_TEXTURE_REFS[index])
}

pub(in crate::entity_models) fn villager_level_texture_ref(level: i32) -> EntityModelTextureRef {
    VILLAGER_LEVEL_TEXTURE_REFS[villager_level_index(level)]
}

pub(in crate::entity_models) fn zombie_villager_level_texture_ref(
    level: i32,
) -> EntityModelTextureRef {
    ZOMBIE_VILLAGER_LEVEL_TEXTURE_REFS[villager_level_index(level)]
}

fn villager_level_index(level: i32) -> usize {
    (level.clamp(1, 5) - 1) as usize
}

pub(in crate::entity_models) fn squid_texture_ref(glow: bool, baby: bool) -> EntityModelTextureRef {
    match (glow, baby) {
        (false, false) => SQUID_TEXTURE_REF,
        (false, true) => SQUID_BABY_TEXTURE_REF,
        (true, false) => GLOW_SQUID_TEXTURE_REF,
        (true, true) => GLOW_SQUID_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn camel_texture_ref(
    family: CamelModelFamily,
    baby: bool,
) -> EntityModelTextureRef {
    match (family, baby) {
        // `CamelHuskRenderer` reuses the adult camel model with `camel_husk.png` and is
        // never a baby, so the husk maps to the husk texture regardless of the age flag.
        (CamelModelFamily::CamelHusk, _) => CAMEL_HUSK_TEXTURE_REF,
        (CamelModelFamily::Camel, false) => CAMEL_TEXTURE_REF,
        (CamelModelFamily::Camel, true) => CAMEL_BABY_TEXTURE_REF,
    }
}

/// The living horse's base coat (vanilla `HorseRenderer.LOCATION_BY_VARIANT`, then
/// `state.isBaby ? variant.baby : variant.adult`): one of seven `horse_<color>(_baby).png` 64×64
/// textures. The markings overlay is a separate deferred layer.
pub(in crate::entity_models) fn horse_coat_texture_ref(
    variant: HorseColorVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (HorseColorVariant::White, false) => HORSE_WHITE_TEXTURE_REF,
        (HorseColorVariant::White, true) => HORSE_WHITE_BABY_TEXTURE_REF,
        (HorseColorVariant::Creamy, false) => HORSE_CREAMY_TEXTURE_REF,
        (HorseColorVariant::Creamy, true) => HORSE_CREAMY_BABY_TEXTURE_REF,
        (HorseColorVariant::Chestnut, false) => HORSE_CHESTNUT_TEXTURE_REF,
        (HorseColorVariant::Chestnut, true) => HORSE_CHESTNUT_BABY_TEXTURE_REF,
        (HorseColorVariant::Brown, false) => HORSE_BROWN_TEXTURE_REF,
        (HorseColorVariant::Brown, true) => HORSE_BROWN_BABY_TEXTURE_REF,
        (HorseColorVariant::Black, false) => HORSE_BLACK_TEXTURE_REF,
        (HorseColorVariant::Black, true) => HORSE_BLACK_BABY_TEXTURE_REF,
        (HorseColorVariant::Gray, false) => HORSE_GRAY_TEXTURE_REF,
        (HorseColorVariant::Gray, true) => HORSE_GRAY_BABY_TEXTURE_REF,
        (HorseColorVariant::DarkBrown, false) => HORSE_DARKBROWN_TEXTURE_REF,
        (HorseColorVariant::DarkBrown, true) => HORSE_DARKBROWN_BABY_TEXTURE_REF,
    }
}

/// The living horse's white-markings overlay texture (vanilla `HorseMarkingLayer.LOCATION_BY_MARKINGS`,
/// then `state.isBaby ? variant.baby : variant.adult`). `Markings.NONE` maps to vanilla's
/// `INVISIBLE_TEXTURE` (no overlay), so it returns `None` and the overlay pass is skipped.
pub(in crate::entity_models) fn horse_markings_texture_ref(
    markings: HorseMarkings,
    baby: bool,
) -> Option<EntityModelTextureRef> {
    Some(match (markings, baby) {
        (HorseMarkings::None, _) => return None,
        (HorseMarkings::White, false) => HORSE_MARKINGS_WHITE_TEXTURE_REF,
        (HorseMarkings::White, true) => HORSE_MARKINGS_WHITE_BABY_TEXTURE_REF,
        (HorseMarkings::WhiteField, false) => HORSE_MARKINGS_WHITEFIELD_TEXTURE_REF,
        (HorseMarkings::WhiteField, true) => HORSE_MARKINGS_WHITEFIELD_BABY_TEXTURE_REF,
        (HorseMarkings::WhiteDots, false) => HORSE_MARKINGS_WHITEDOTS_TEXTURE_REF,
        (HorseMarkings::WhiteDots, true) => HORSE_MARKINGS_WHITEDOTS_BABY_TEXTURE_REF,
        (HorseMarkings::BlackDots, false) => HORSE_MARKINGS_BLACKDOTS_TEXTURE_REF,
        (HorseMarkings::BlackDots, true) => HORSE_MARKINGS_BLACKDOTS_BABY_TEXTURE_REF,
    })
}

pub(in crate::entity_models) fn sheep_wool_layer_color(wool_color: SheepWoolColor) -> [f32; 4] {
    sheep_wool_layer_color_from_bytes(sheep_wool_layer_color_bytes(wool_color))
}

pub(in crate::entity_models) fn sheep_wool_render_color(
    wool_color: SheepWoolColor,
    jeb: bool,
    age_ticks: f32,
) -> [f32; 4] {
    if jeb {
        sheep_jeb_wool_layer_color(age_ticks)
    } else {
        sheep_wool_layer_color(wool_color)
    }
}

pub(in crate::entity_models) fn sheep_jeb_wool_layer_color(age_ticks: f32) -> [f32; 4] {
    let tick = age_ticks.max(0.0);
    let tick_floor = tick.floor() as i32;
    let color_step = tick_floor / bbb_render_types::VANILLA_SHEEP_COLOR_DURATION_TICKS;
    let color_count = bbb_render_types::VANILLA_DYE_COLOR_COUNT as i32;
    let from = color_step.rem_euclid(color_count) as usize;
    let to = (color_step + 1).rem_euclid(color_count) as usize;
    let alpha = ((tick_floor % bbb_render_types::VANILLA_SHEEP_COLOR_DURATION_TICKS) as f32
        + tick.fract())
        / bbb_render_types::VANILLA_SHEEP_COLOR_DURATION_TICKS as f32;
    sheep_wool_layer_color_from_bytes(sheep_lerp_color_bytes(
        sheep_wool_layer_color_bytes(SheepWoolColor::from_vanilla_id(from as i32)),
        sheep_wool_layer_color_bytes(SheepWoolColor::from_vanilla_id(to as i32)),
        alpha,
    ))
}

fn sheep_wool_layer_color_bytes(wool_color: SheepWoolColor) -> [u8; 3] {
    let color = wool_color.entry().sheep_wool_color;
    [
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    ]
}

fn sheep_wool_layer_color_from_bytes([red, green, blue]: [u8; 3]) -> [f32; 4] {
    [
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        1.0,
    ]
}

fn sheep_lerp_color_bytes(from: [u8; 3], to: [u8; 3], alpha: f32) -> [u8; 3] {
    [
        sheep_lerp_color_channel(from[0], to[0], alpha),
        sheep_lerp_color_channel(from[1], to[1], alpha),
        sheep_lerp_color_channel(from[2], to[2], alpha),
    ]
}

fn sheep_lerp_color_channel(from: u8, to: u8, alpha: f32) -> u8 {
    let from = i32::from(from);
    let to = i32::from(to);
    (from + (alpha * (to - from) as f32).floor() as i32).clamp(0, 255) as u8
}

pub(in crate::entity_models) fn mooshroom_texture_ref(
    baby: bool,
    variant: MooshroomVariant,
) -> EntityModelTextureRef {
    // Vanilla `MushroomCowRenderer.getTextureLocation`: `isBaby ? variant.baby : variant.adult`.
    match (variant, baby) {
        (MooshroomVariant::Red, false) => MOOSHROOM_TEXTURE_REF,
        (MooshroomVariant::Red, true) => MOOSHROOM_BABY_TEXTURE_REF,
        (MooshroomVariant::Brown, false) => MOOSHROOM_BROWN_TEXTURE_REF,
        (MooshroomVariant::Brown, true) => MOOSHROOM_BROWN_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn wolf_texture_ref(
    baby: bool,
    tame: bool,
    angry: bool,
    variant: WolfModelVariant,
) -> EntityModelTextureRef {
    // Vanilla `Wolf.getTexture`: pick adult/baby info, then tame → angry → wild. Built-in
    // fallback paths come from the source-generated `WolfVariants.register` catalog.
    variant.builtin_texture_ref(baby, tame, angry)
}

#[cfg(test)]
pub(in crate::entity_models) fn player_texture_ref(slim: bool) -> EntityModelTextureRef {
    if slim {
        PLAYER_SLIM_STEVE_TEXTURE_REF
    } else {
        PLAYER_WIDE_STEVE_TEXTURE_REF
    }
}

pub(in crate::entity_models) fn llama_texture_ref(
    variant: LlamaVariant,
    baby: bool,
) -> EntityModelTextureRef {
    match (variant, baby) {
        (LlamaVariant::Creamy, false) => LLAMA_CREAMY_TEXTURE_REF,
        (LlamaVariant::Creamy, true) => LLAMA_CREAMY_BABY_TEXTURE_REF,
        (LlamaVariant::White, false) => LLAMA_WHITE_TEXTURE_REF,
        (LlamaVariant::White, true) => LLAMA_WHITE_BABY_TEXTURE_REF,
        (LlamaVariant::Brown, false) => LLAMA_BROWN_TEXTURE_REF,
        (LlamaVariant::Brown, true) => LLAMA_BROWN_BABY_TEXTURE_REF,
        (LlamaVariant::Gray, false) => LLAMA_GRAY_TEXTURE_REF,
        (LlamaVariant::Gray, true) => LLAMA_GRAY_BABY_TEXTURE_REF,
    }
}

pub(in crate::entity_models) fn boat_texture_ref(
    family: BoatModelFamily,
    chest: bool,
) -> EntityModelTextureRef {
    match (family, chest) {
        (BoatModelFamily::Acacia, false) => BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Acacia, true) => CHEST_BOAT_ACACIA_TEXTURE_REF,
        (BoatModelFamily::Bamboo, false) => BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Bamboo, true) => CHEST_BOAT_BAMBOO_TEXTURE_REF,
        (BoatModelFamily::Birch, false) => BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Birch, true) => CHEST_BOAT_BIRCH_TEXTURE_REF,
        (BoatModelFamily::Cherry, false) => BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::Cherry, true) => CHEST_BOAT_CHERRY_TEXTURE_REF,
        (BoatModelFamily::DarkOak, false) => BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::DarkOak, true) => CHEST_BOAT_DARK_OAK_TEXTURE_REF,
        (BoatModelFamily::Jungle, false) => BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Jungle, true) => CHEST_BOAT_JUNGLE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, false) => BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Mangrove, true) => CHEST_BOAT_MANGROVE_TEXTURE_REF,
        (BoatModelFamily::Oak, false) => BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::Oak, true) => CHEST_BOAT_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, false) => BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::PaleOak, true) => CHEST_BOAT_PALE_OAK_TEXTURE_REF,
        (BoatModelFamily::Spruce, false) => BOAT_SPRUCE_TEXTURE_REF,
        (BoatModelFamily::Spruce, true) => CHEST_BOAT_SPRUCE_TEXTURE_REF,
    }
}
