use super::*;

pub(super) struct DimensionProfile {
    pub(super) dimension: WorldDimension,
    pub(super) name: Option<&'static str>,
}

/// Vanilla `ClientPacketListener.determineLevelLoadingReason`
/// (ClientPacketListener.java:1341-1353): a death respawn always stays
/// `OTHER`, otherwise either side of the dimension change picks the portal
/// background.
pub(super) fn level_loading_reason(
    player_died: bool,
    dimension: &str,
    previous_dimension: Option<&str>,
) -> WorldLevelLoadReason {
    if player_died {
        return WorldLevelLoadReason::Other;
    }
    let touches = |name: &str| dimension == name || previous_dimension == Some(name);
    if touches("minecraft:the_nether") {
        WorldLevelLoadReason::NetherPortal
    } else if touches("minecraft:the_end") {
        WorldLevelLoadReason::EndPortal
    } else {
        WorldLevelLoadReason::Other
    }
}

pub(super) fn dimension_profile(dimension_type_id: i32, dimension: &str) -> DimensionProfile {
    match (dimension_type_id, dimension) {
        (0, _) | (_, "minecraft:overworld") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld"),
        },
        (1, _) | (_, "minecraft:the_nether") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_nether"),
        },
        (2, _) | (_, "minecraft:the_end") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_end"),
        },
        (3, _) | (_, "minecraft:overworld_caves") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld_caves"),
        },
        _ => DimensionProfile {
            dimension: WorldDimension::default(),
            name: None,
        },
    }
}

pub(super) fn built_in_dimension_cardinal_lighting(
    dimension_type_id: i32,
    dimension: &str,
) -> WorldCardinalLighting {
    if dimension_profile(dimension_type_id, dimension).name == Some("minecraft:the_nether") {
        WorldCardinalLighting::Nether
    } else {
        WorldCardinalLighting::Default
    }
}

pub(super) fn built_in_dimension_has_sky_light(dimension_type_id: i32, dimension: &str) -> bool {
    !matches!(
        dimension_profile(dimension_type_id, dimension).name,
        Some("minecraft:the_nether" | "minecraft:the_end")
    )
}

pub(super) fn built_in_dimension_has_ceiling(dimension_type_id: i32, dimension: &str) -> bool {
    dimension_profile(dimension_type_id, dimension).name == Some("minecraft:the_nether")
}

pub(super) fn built_in_dimension_skybox(dimension_type_id: i32, dimension: &str) -> WorldSkybox {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:the_nether") => WorldSkybox::None,
        Some("minecraft:the_end") => WorldSkybox::End,
        _ => WorldSkybox::Overworld,
    }
}

pub(super) fn built_in_dimension_default_clock_resource_id(
    dimension_type_id: i32,
    dimension: &str,
) -> Option<&'static str> {
    match dimension_profile(dimension_type_id, dimension).name {
        Some("minecraft:overworld" | "minecraft:overworld_caves") => Some("minecraft:overworld"),
        Some("minecraft:the_end") => Some("minecraft:the_end"),
        _ => None,
    }
}

pub(crate) fn rounded_game_event_param(param: f32) -> i32 {
    // ClientPacketListener.handleGameEvent uses Mth.floor(paramFloat + 0.5F)
    // once for every integer-valued game-event consumer.
    if param.is_finite() {
        (param + 0.5).floor() as i32
    } else {
        VANILLA_GAME_TYPE_SURVIVAL_ID
    }
}

pub(super) fn advance_client_clock(clock: &mut ClockUpdateState, game_time_delta: i64) {
    if game_time_delta == 0 {
        return;
    }

    let new_partial_ticks =
        f64::from(clock.partial_tick) + (game_time_delta as f64) * f64::from(clock.rate);
    let full_ticks = new_partial_ticks.floor() as i64;
    clock.partial_tick = (new_partial_ticks - full_ticks as f64) as f32;
    clock.total_ticks = clock.total_ticks.saturating_add(full_ticks);
}

pub(super) fn default_clock_total_ticks(
    clock_updates: &[ClockUpdateState],
    default_clock_id: Option<i32>,
) -> i64 {
    default_clock_id
        .and_then(|clock_id| {
            clock_updates
                .iter()
                .find(|clock| clock.clock_id == clock_id)
        })
        .map(|clock| clock.total_ticks)
        .unwrap_or(0)
}

pub(super) fn positive_tick_steps(tick_steps: i32) -> u32 {
    u32::try_from(tick_steps).unwrap_or(0)
}
