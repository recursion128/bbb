use super::*;

pub(super) fn entity_data_bool(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

/// Vanilla `Creeper.isPowered()` = the generated synced `DATA_IS_POWERED` boolean. Read only for
/// entity types owning the Creeper accessor bundle, gating the `CreeperPowerLayer` energy swirl.
pub(super) fn creeper_powered(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(accessors) = vanilla_entity_metadata_creeper_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    entity_data_bool(values, accessors.powered_data_id, accessors.powered_default)
}

/// Vanilla `WitherBoss.isPowered()` = `getHealth() <= getMaxHealth() / 2.0`, gating the
/// `WitherArmorLayer` energy swirl. The current health is the synced `LivingEntity.DATA_HEALTH_ID`
/// float (index `9`); the wither's `Attributes.MAX_HEALTH` base is `300` (mirroring the wolf tail's
/// hardcoded `TAME_MAX_HEALTH` precedent — bbb does not yet track per-entity max-health attribute
/// overrides). A wither with no synced health defaults to full, so it reads un-powered.
/// Maps a projected world armor material onto the renderer's `EntityArmorMaterial` for the
/// `HumanoidArmorLayer` overlay (1:1; the two enums mirror the vanilla `ArmorMaterials` set).
pub(crate) fn armor_material(
    material: Option<WorldArmorMaterialKind>,
) -> Option<EntityArmorMaterial> {
    material.map(|material| match material {
        WorldArmorMaterialKind::Leather => EntityArmorMaterial::Leather,
        WorldArmorMaterialKind::Copper => EntityArmorMaterial::Copper,
        WorldArmorMaterialKind::Chainmail => EntityArmorMaterial::Chainmail,
        WorldArmorMaterialKind::Iron => EntityArmorMaterial::Iron,
        WorldArmorMaterialKind::Gold => EntityArmorMaterial::Gold,
        WorldArmorMaterialKind::Diamond => EntityArmorMaterial::Diamond,
        WorldArmorMaterialKind::TurtleScute => EntityArmorMaterial::TurtleScute,
        WorldArmorMaterialKind::Netherite => EntityArmorMaterial::Netherite,
        WorldArmorMaterialKind::ArmadilloScute => EntityArmorMaterial::ArmadilloScute,
    })
}

pub(super) fn equipment_asset(
    asset: Option<WorldEquipmentAssetId>,
) -> Option<EntityEquipmentAssetId> {
    asset.map(|asset| EntityEquipmentAssetId::new(asset.generation, asset.index))
}

pub(super) fn armor_trim(
    trim: Option<&WorldArmorTrimSprite>,
    registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityArmorTrim> {
    let trim = trim?;
    let registries = registries?;
    Some(EntityArmorTrim {
        // Vanilla `TextureAtlas.getSprite` returns the atlas missing sprite when a requested trim
        // sprite is absent. The startup armor-trim atlas always uploads `minecraft:missingno`, so
        // preserve the trim submission instead of silently dropping custom/non-generated layers.
        texture: registries
            .ready_texture(&trim.sprite_id)
            .or_else(|| registries.ready_texture("minecraft:missingno"))?,
        decal: trim.decal,
    })
}

pub(super) fn wolf_armor_crackiness(
    crackiness: WorldWolfArmorCrackiness,
) -> Option<WolfArmorCrackiness> {
    match crackiness {
        WorldWolfArmorCrackiness::None => None,
        WorldWolfArmorCrackiness::Low => Some(WolfArmorCrackiness::Low),
        WorldWolfArmorCrackiness::Medium => Some(WolfArmorCrackiness::Medium),
        WorldWolfArmorCrackiness::High => Some(WolfArmorCrackiness::High),
    }
}

/// Carries a projected per-slot `DyedItemColor` (a packed RGB `i32`) onto the renderer's armor dye
/// tint (`u32`). The renderer forces it opaque and applies it only to leather, matching vanilla
/// `DyedItemColor.getOrDefault` → `EquipmentLayerRenderer.getColorForLayer`.
pub(super) fn armor_dye(dye: Option<i32>) -> Option<u32> {
    dye.map(|dye| dye as u32)
}

/// Maps the world-owned vanilla `DyeColor` from `Equippable.llamaSwag(color)` onto the renderer's
/// shared dye enum for `LlamaDecorLayer` `LLAMA_BODY` equipment textures.
pub(super) fn llama_body_decor_color(
    color: Option<WorldLlamaBodyDecorColor>,
) -> Option<EntityDyeColor> {
    color.map(|color| match color {
        WorldLlamaBodyDecorColor::White => EntityDyeColor::White,
        WorldLlamaBodyDecorColor::Orange => EntityDyeColor::Orange,
        WorldLlamaBodyDecorColor::Magenta => EntityDyeColor::Magenta,
        WorldLlamaBodyDecorColor::LightBlue => EntityDyeColor::LightBlue,
        WorldLlamaBodyDecorColor::Yellow => EntityDyeColor::Yellow,
        WorldLlamaBodyDecorColor::Lime => EntityDyeColor::Lime,
        WorldLlamaBodyDecorColor::Pink => EntityDyeColor::Pink,
        WorldLlamaBodyDecorColor::Gray => EntityDyeColor::Gray,
        WorldLlamaBodyDecorColor::LightGray => EntityDyeColor::LightGray,
        WorldLlamaBodyDecorColor::Cyan => EntityDyeColor::Cyan,
        WorldLlamaBodyDecorColor::Purple => EntityDyeColor::Purple,
        WorldLlamaBodyDecorColor::Blue => EntityDyeColor::Blue,
        WorldLlamaBodyDecorColor::Brown => EntityDyeColor::Brown,
        WorldLlamaBodyDecorColor::Green => EntityDyeColor::Green,
        WorldLlamaBodyDecorColor::Red => EntityDyeColor::Red,
        WorldLlamaBodyDecorColor::Black => EntityDyeColor::Black,
    })
}

pub(super) fn happy_ghast_harness_color(
    color: Option<WorldHappyGhastHarnessColor>,
) -> Option<EntityDyeColor> {
    color.map(|color| match color {
        WorldHappyGhastHarnessColor::White => EntityDyeColor::White,
        WorldHappyGhastHarnessColor::Orange => EntityDyeColor::Orange,
        WorldHappyGhastHarnessColor::Magenta => EntityDyeColor::Magenta,
        WorldHappyGhastHarnessColor::LightBlue => EntityDyeColor::LightBlue,
        WorldHappyGhastHarnessColor::Yellow => EntityDyeColor::Yellow,
        WorldHappyGhastHarnessColor::Lime => EntityDyeColor::Lime,
        WorldHappyGhastHarnessColor::Pink => EntityDyeColor::Pink,
        WorldHappyGhastHarnessColor::Gray => EntityDyeColor::Gray,
        WorldHappyGhastHarnessColor::LightGray => EntityDyeColor::LightGray,
        WorldHappyGhastHarnessColor::Cyan => EntityDyeColor::Cyan,
        WorldHappyGhastHarnessColor::Purple => EntityDyeColor::Purple,
        WorldHappyGhastHarnessColor::Blue => EntityDyeColor::Blue,
        WorldHappyGhastHarnessColor::Brown => EntityDyeColor::Brown,
        WorldHappyGhastHarnessColor::Green => EntityDyeColor::Green,
        WorldHappyGhastHarnessColor::Red => EntityDyeColor::Red,
        WorldHappyGhastHarnessColor::Black => EntityDyeColor::Black,
    })
}

/// Maps `ShulkerRenderState.attachFace` from world metadata onto the renderer root transform input.
pub(super) fn entity_attachment_face(face: WorldEntityAttachmentFace) -> EntityAttachmentFace {
    match face {
        WorldEntityAttachmentFace::Down => EntityAttachmentFace::Down,
        WorldEntityAttachmentFace::Up => EntityAttachmentFace::Up,
        WorldEntityAttachmentFace::North => EntityAttachmentFace::North,
        WorldEntityAttachmentFace::South => EntityAttachmentFace::South,
        WorldEntityAttachmentFace::West => EntityAttachmentFace::West,
        WorldEntityAttachmentFace::East => EntityAttachmentFace::East,
    }
}

/// Maps a projected guardian attack beam onto the renderer's `GuardianBeamRenderState` (1:1; the two
/// structs mirror vanilla `GuardianRenderState`'s beam fields).
pub(super) fn guardian_beam(
    beam: Option<WorldGuardianBeamSource>,
) -> Option<GuardianBeamRenderState> {
    beam.map(|beam| GuardianBeamRenderState {
        eye_to_target: beam.eye_to_target,
        eye_height: beam.eye_height,
        attack_time: beam.attack_time,
        attack_scale: beam.attack_scale,
    })
}

/// Maps the world-owned `GuardianRenderer.extractRenderState` eye inputs onto the renderer-owned
/// model state without deriving the eye pose in native.
pub(super) fn guardian_eye_tracking(
    tracking: Option<WorldGuardianEyeTrackingSource>,
) -> Option<GuardianEyeTrackingRenderState> {
    tracking.map(|tracking| GuardianEyeTrackingRenderState {
        eye_position: tracking.eye_position,
        look_direction: tracking.look_direction,
        look_at_position: tracking.look_at_position,
    })
}

/// Maps the projected leash ropes onto the renderer's fixed-size `EntityLeashRopesRenderState`
/// (1:1 per rope; vanilla connections carry one rope, or four for a ghast-to-boat quad leash).
/// `None` for an unleashed entity's empty projection.
pub(super) fn leash_ropes(source: &[WorldLeashRopeSource]) -> Option<EntityLeashRopesRenderState> {
    let ropes = source
        .iter()
        .map(|rope| LeashRopeRenderState {
            offset: rope.offset,
            delta: rope.delta,
            start_block_light: rope.start_block_light,
            start_sky_light: rope.start_sky_light,
            end_block_light: rope.end_block_light,
            end_sky_light: rope.end_sky_light,
            slack: rope.slack,
        })
        .collect::<Vec<_>>();
    EntityLeashRopesRenderState::from_ropes(&ropes)
}

/// Maps a projected end-crystal healing beam onto the renderer's `EndCrystalBeamRenderState`.
pub(super) fn end_crystal_beam(
    beam: Option<WorldEndCrystalBeamSource>,
) -> Option<EndCrystalBeamRenderState> {
    beam.map(|beam| EndCrystalBeamRenderState {
        beam_offset: beam.beam_offset,
    })
}

/// Maps the projected ender-dragon procedural-animation inputs onto the renderer's
/// `EnderDragonAnimationRenderState` (1:1; both mirror vanilla `EnderDragonRenderState`).
pub(super) fn ender_dragon_animation(
    source: Option<&WorldEnderDragonAnimationSource>,
) -> Option<EnderDragonAnimationRenderState> {
    source.map(|animation| EnderDragonAnimationRenderState {
        flap_time: animation.flap_time,
        is_landing_or_taking_off: animation.is_landing_or_taking_off,
        is_sitting: animation.is_sitting,
        distance_to_egg: animation.distance_to_egg,
        historical: animation.historical.map(|sample| EnderDragonFlightSample {
            y: sample.y,
            y_rot: sample.y_rot,
        }),
    })
}

/// Maps a projected ender-dragon healing beam onto the renderer's `EnderDragonBeamRenderState`.
pub(super) fn ender_dragon_beam(
    beam: Option<WorldEnderDragonBeamSource>,
) -> Option<EnderDragonBeamRenderState> {
    beam.map(|beam| EnderDragonBeamRenderState {
        beam_offset: beam.beam_offset,
    })
}

pub(super) fn wither_powered(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    const WITHER_MAX_HEALTH: f32 = 300.0;
    if entity_type_id != VANILLA_ENTITY_TYPE_WITHER_ID {
        return false;
    }
    vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_float(values, accessors.health_data_id, WITHER_MAX_HEALTH)
                <= WITHER_MAX_HEALTH / 2.0
        },
    )
}

/// Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(getHealth() / getMaxHealth())`,
/// the iron golem's base `Attributes.MAX_HEALTH` being the constant `100.0`. The synced
/// `LivingEntity.DATA_HEALTH_ID` drives the damage-crack overlay tier.
pub(super) fn iron_golem_crackiness(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> IronGolemCrackiness {
    const IRON_GOLEM_MAX_HEALTH: f32 = 100.0;
    let health = vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
        .map(|accessors| entity_data_float(values, accessors.health_data_id, IRON_GOLEM_MAX_HEALTH))
        .unwrap_or(IRON_GOLEM_MAX_HEALTH);
    IronGolemCrackiness::from_health_fraction(health / IRON_GOLEM_MAX_HEALTH)
}

/// Vanilla `CopperGolemRenderer.extractRenderState`: `state.weathering = entity.getWeatherState()`.
/// The synced `WeatheringCopper.WeatherState` ordinal maps 0..=3 to unaffected/exposed/weathered/
/// oxidized, clamping out-of-range values like vanilla's `ByIdMap.OutOfBoundsStrategy.CLAMP`.
pub(super) fn copper_golem_weathering(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> CopperGolemWeathering {
    let Some(accessors) =
        vanilla_entity_metadata_copper_golem_accessors_for_entity_type(entity_type_id)
    else {
        return CopperGolemWeathering::Unaffected;
    };
    values
        .iter()
        .rev()
        .find(|value| value.data_id == accessors.weather_state_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::WeatheringCopperState,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(CopperGolemWeathering::from_vanilla_id)
        .unwrap_or_else(|| CopperGolemWeathering::from_vanilla_id(accessors.weather_state_default))
}

pub(super) fn entity_data_int(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i32,
) -> i32 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

pub(super) fn entity_data_long(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i64,
) -> i64 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Long(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

pub(super) fn entity_data_optional_component<'a>(
    values: &'a [bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&'a str> {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.text()),
            _ => None,
        })
}

pub(super) fn entity_data_byte(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i8,
) -> i8 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

pub(super) fn entity_data_float(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: f32,
) -> f32 {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Float(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

pub(super) fn entity_data_rotations(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: [f32; 3],
) -> [f32; 3] {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Rotations { x, y, z } => Some([*x, *y, *z]),
            _ => None,
        })
        .unwrap_or(default)
}

pub(super) fn thrown_trident_foil(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(entity_type_id)
        .map(|accessors| entity_data_bool(values, accessors.foil_data_id, accessors.foil_default))
        .unwrap_or(false)
}
