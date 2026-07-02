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

/// Vanilla `Creeper.isPowered()` = the synced `DATA_IS_POWERED` boolean (entity-data index `17`:
/// `Entity` `0..=7`, `LivingEntity` `8..=14`, `Mob` `15`, then `Creeper`'s `DATA_SWELL_DIR` `16` and
/// `DATA_IS_POWERED` `17`). Read only for the creeper, gating the `CreeperPowerLayer` energy swirl.
pub(super) fn creeper_powered(
    entity_type_id: i32,
    values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    const CREEPER_IS_POWERED_DATA_ID: u8 = 17;
    entity_type_id == VANILLA_ENTITY_TYPE_CREEPER_ID
        && entity_data_bool(values, CREEPER_IS_POWERED_DATA_ID, false)
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

/// Maps a projected end-crystal healing beam onto the renderer's `EndCrystalBeamRenderState`.
pub(super) fn end_crystal_beam(
    beam: Option<WorldEndCrystalBeamSource>,
) -> Option<EndCrystalBeamRenderState> {
    beam.map(|beam| EndCrystalBeamRenderState {
        beam_offset: beam.beam_offset,
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
    entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID
        && entity_data_float(values, LIVING_ENTITY_HEALTH_DATA_ID, WITHER_MAX_HEALTH)
            <= WITHER_MAX_HEALTH / 2.0
}

/// Vanilla `IronGolem.getCrackiness()` = `Crackiness.GOLEM.byFraction(getHealth() / getMaxHealth())`,
/// the iron golem's base `Attributes.MAX_HEALTH` being the constant `100.0`. The synced
/// `LivingEntity.DATA_HEALTH_ID` (index 9) drives the damage-crack overlay tier.
pub(super) fn iron_golem_crackiness(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> IronGolemCrackiness {
    const IRON_GOLEM_MAX_HEALTH: f32 = 100.0;
    let health = entity_data_float(values, LIVING_ENTITY_HEALTH_DATA_ID, IRON_GOLEM_MAX_HEALTH);
    IronGolemCrackiness::from_health_fraction(health / IRON_GOLEM_MAX_HEALTH)
}

/// Vanilla `CopperGolemRenderer.extractRenderState`: `state.weathering = entity.getWeatherState()`.
/// The synced `WeatheringCopper.WeatherState` ordinal maps 0..=3 to unaffected/exposed/weathered/
/// oxidized, clamping out-of-range values like vanilla's `ByIdMap.OutOfBoundsStrategy.CLAMP`.
pub(super) fn copper_golem_weathering(
    values: &[bbb_protocol::packets::EntityDataValue],
) -> CopperGolemWeathering {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == COPPER_GOLEM_WEATHER_STATE_DATA_ID)
        .and_then(|value| match &value.value {
            EntityDataValueKind::EnumId {
                serializer: EntityDataEnumSerializer::WeatheringCopperState,
                id,
            } => Some(*id),
            _ => None,
        })
        .map(CopperGolemWeathering::from_vanilla_id)
        .unwrap_or(CopperGolemWeathering::Unaffected)
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
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.as_str()),
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
    entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID
        && entity_data_bool(values, TRIDENT_FOIL_DATA_ID, false)
}
