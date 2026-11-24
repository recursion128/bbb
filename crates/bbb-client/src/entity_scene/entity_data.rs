use super::*;

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

/// Vanilla `WitherBoss.isPowered()` = `getHealth() <= getMaxHealth() / 2.0`, gating the
/// `WitherArmorLayer` energy swirl. The current health is the synced `LivingEntity.DATA_HEALTH_ID`
/// float; the wither's `Attributes.MAX_HEALTH` base is `300`. A wither with no synced health defaults
/// to full, so it reads un-powered.
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

pub(super) fn entity_data_optional_component(
    values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&str> {
    values
        .iter()
        .rev()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalComponent(Some(value)) => Some(value.text()),
            _ => None,
        })
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
