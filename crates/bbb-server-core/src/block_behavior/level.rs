//! Block behavior inputs, queries and placement requests.

use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;

/// Block.java:92: UPDATE_CLIENTS.
pub const SET_BLOCK_UPDATE_CLIENTS: i32 = 2;

/// Block.java:94: UPDATE_IMMEDIATE.
pub const SET_BLOCK_UPDATE_IMMEDIATE: i32 = 8;

/// Block.java:102: UPDATE_INVISIBLE | UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS.
pub const SET_BLOCK_UPDATE_NONE: i32 = 260;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightLayer {
    Sky,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockEntityQuery {
    BlockTriggerAll,
    BlockTriggerLiving,
    Minecart,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DetectorRailMinecarts {
    pub command_block_success_count: Option<i32>,
    // An unknown item stack limit cannot supply an analog signal.
    pub container_signal: Option<Option<i32>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InsideEntity {
    // The ticking body is outside the store and its occupancy snapshot.
    pub source: Option<i32>,
    pub bounds: Aabb,
    pub minecart: bool,
    pub living: bool,
    pub ignores_block_triggers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockMobSpawnRequest {
    HatchedTurtle { position: Vec3d, home_pos: BlockPos },
    HatchedSniffer { position: Vec3d, y_rot: f32 },
    HydratedHappyGhast { position: Vec3d, y_rot: f32 },
    FrogspawnTadpole { position: Vec3d, y_rot: f32 },
    PortalZombifiedPiglin { position: Vec3d, y_rot: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GolemKind {
    Snow,
    Iron,
    Copper,
    Wither,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BlockPlacedBy<'a> {
    pub block_entity_data: Option<&'a [bbb_protocol::NbtSummaryEntry]>,
    pub peaceful: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopperWeatherState {
    Unaffected,
    Exposed,
    Weathered,
    Oxidized,
}

impl CopperWeatherState {
    pub(super) fn from_age(age: i32) -> Option<Self> {
        Some(match age {
            0 => Self::Unaffected,
            1 => Self::Exposed,
            2 => Self::Weathered,
            3 => Self::Oxidized,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GolemSpawnRequest {
    pub kind: GolemKind,
    pub pos: BlockPos,
    pub player_created: bool,
    pub copper_weather: Option<CopperWeatherState>,
    pub y_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallingBlockFallRequest {
    pub pos: BlockPos,
    pub block_state_id: i32,
    pub disable_drop: bool,
    pub hurt_entities: Option<FallingBlockHurt>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallingBlockHurt {
    pub damage_per_distance: f32,
    pub damage_max: i32,
}
