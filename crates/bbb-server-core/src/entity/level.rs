//! Entity interaction inputs and registry facts.

use crate::world::simulation_level::LevelCtx;
use rustc_hash::FxHashMap;

use crate::world::scoreboard::PlayerTeam;
#[cfg(test)]
use crate::world::scoreboard::Scoreboard;
use bbb_protocol::JavaString;
use std::sync::Arc;

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_facts::scoreboard_team_options::TeamCollisionRule;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
    vanilla_block_state_path_facts, VanillaWaterPathRule,
};
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::PersistentRegistryTagMembers;

use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_types::fluid::{block_state_fluid_state, FluidState};
use bbb_server_types::geometry::Aabb;

use crate::entity::Entity;

/// Registry facts retained by saved or pending entity inputs.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityRegistryTagView {
    /// The generated base Vanilla 26.1 registry tags used by standalone callers.
    Builtin26_1,
    /// The effective datapack generation installed for one level.
    Frozen(Arc<RegistryTagSnapshot>),
}

impl EntityRegistryTagView {
    pub fn contains(&self, registry_id: &str, tag_id: &str, element_resource_id: &str) -> bool {
        match self {
            Self::Builtin26_1 => {
                builtin_registry_tag_membership(registry_id, tag_id, element_resource_id)
            }
            Self::Frozen(snapshot) => snapshot.contains(registry_id, tag_id, element_resource_id),
        }
    }
}

/// The level answers mob target goals read that no entity carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetLevelFacts {
    /// `GameRules.UNIVERSAL_ANGER`.
    pub universal_anger: bool,
    /// `GameRules.FORGIVE_DEAD_PLAYERS`.
    pub forgive_dead_players: bool,
    pub pvp: bool,
    pub bright_outside: Option<bool>,
}

impl Default for TargetLevelFacts {
    /// The rules at their registered defaults.
    fn default() -> Self {
        Self {
            universal_anger: false,
            forgive_dead_players: true,
            pvp: true,
            bright_outside: None,
        }
    }
}

/// One `Player` as the entity mechanism sees it.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearbyPlayer {
    pub collision: Option<crate::entity::player_collision::PlayerCollisionFacts>,
    pub in_water: bool,
    pub uuid: uuid::Uuid,
    /// `Entity.position()`, the player's feet.
    pub position: Vec3d,
    pub dimensions: [f32; 2],
    /// `Entity.getEyeHeight()`, which the orb aims at half of (`:165`).
    pub eye_height: f64,
    /// `Player.isSpectator()`.
    pub spectator: bool,
    /// `LivingEntity.isDeadOrDying()`.
    pub dead_or_dying: bool,
    pub invulnerable: bool,
    /// `Player.getAbilities().invulnerable` (`Player.java:723-724`).
    pub abilities_invulnerable: bool,
    pub discrete: bool,
    pub sleeping: bool,
    pub invisible: bool,
    pub armor_cover: f32,
    /// `Entity.getId()`, the network id a `DamageSource` naming this player
    /// carries and a projectile owned by it names in its add packet.
    pub entity_id: i32,
    pub instabuild: bool,
    pub may_build: bool,
    /// `Player.getMainHandItem().getItem()` as a built-in item type id, `None`
    /// for an empty hand.
    pub main_hand_item_id: Option<i32>,
    /// `Player.getOffhandItem().getItem()`, the same way.
    pub off_hand_item_id: Option<i32>,
    pub head_item_resource_id: Option<&'static str>,
    pub y_rot: f32,
    pub x_rot: f32,
    /// `Entity.getDeltaMovement()`, used by ranged mobs to lead shots.
    pub delta_movement: Vec3d,
    pub known_movement: Vec3d,
    /// The target's current health for source-exact attack selection.
    pub health: f32,
    pub has_slowness: bool,
    pub has_poison: bool,
    pub has_weakness: bool,
    pub luck: f32,
    /// The session's `lastHurtByMob`/`lastHurtMob` memories, which a tamed
    /// animal's owner goals read off its owner.
    pub hurt: PlayerHurtMemory,
}

impl NearbyPlayer {
    #[must_use]
    pub fn bounding_box(&self) -> Aabb {
        Aabb::of_entity(self.position, self.dimensions[0], self.dimensions[1])
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerHurtMemory {
    pub tick_count: i32,
    pub last_hurt_by_mob: Option<uuid::Uuid>,
    pub last_hurt_by_mob_timestamp: i32,
    pub last_hurt_mob: Option<uuid::Uuid>,
    pub last_hurt_mob_timestamp: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreboardTeams {
    pub(crate) teams: Vec<PlayerTeam>,
    pub(crate) members: FxHashMap<JavaString, u32>,
}

impl ScoreboardTeams {
    #[cfg(test)]
    pub(crate) fn add_team(&mut self, members: &[JavaString], collision_rule: TeamCollisionRule) {
        self.add_team_with_friendly_fire(members, collision_rule, true);
    }

    #[cfg(test)]
    pub(crate) fn add_team_with_friendly_fire(
        &mut self,
        members: &[JavaString],
        collision_rule: TeamCollisionRule,
        allow_friendly_fire: bool,
    ) {
        let name = self.teams.len().to_string();
        let mut board = Scoreboard::default();
        board.teams = std::mem::take(self);
        board.add_team(&name);
        let team = board.team_mut(&name).expect("the team was just added");
        team.collision_rule = collision_rule;
        team.allow_friendly_fire = allow_friendly_fire;
        for member in members {
            board.add_player_to_team(member, &name);
        }
        *self = board.teams;
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// The teams whose `allowFriendlyFire` is set.
    #[must_use]
    pub(crate) fn allows_friendly_fire(&self, team: u32) -> bool {
        self.teams[team as usize].allow_friendly_fire
    }

    fn collision_rule(&self, team: Option<u32>) -> TeamCollisionRule {
        team.map_or(TeamCollisionRule::Always, |index| {
            self.teams[index as usize].collision_rule
        })
    }

    #[must_use]
    pub fn pushes_nothing(&self, own: Option<u32>) -> bool {
        self.collision_rule(own) == TeamCollisionRule::Never
    }

    #[must_use]
    pub fn collision_admits(&self, own: Option<u32>, their: Option<u32>) -> bool {
        use TeamCollisionRule::{Never, PushOtherTeams, PushOwnTeam};
        let own_rule = self.collision_rule(own);
        let their_rule = self.collision_rule(their);
        if own_rule == Never || their_rule == Never {
            return false;
        }
        let same_team = Self::allied(own, their);
        if (own_rule == PushOwnTeam || their_rule == PushOwnTeam) && same_team {
            return false;
        }
        (own_rule != PushOtherTeams && their_rule != PushOtherTeams) || same_team
    }

    /// `Scoreboard.getPlayersTeam(name)` (`:308-310`).
    #[must_use]
    pub fn team_of(&self, name: &str) -> Option<u32> {
        if self.is_empty() {
            return None;
        }
        JavaString::with_utf16(name, |name| self.members.get(name).copied())
    }

    pub(crate) fn entity_team(&self, uuid: uuid::Uuid) -> Option<u32> {
        if self.is_empty() {
            return None;
        }
        let mut buffer = uuid::Uuid::encode_buffer();
        self.team_of(uuid.hyphenated().encode_lower(&mut buffer))
    }

    ///
    /// Both indices must refer to the same scoreboard team list.
    #[must_use]
    pub fn allied(first: Option<u32>, second: Option<u32>) -> bool {
        first.is_some() && first == second
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SculkSensorStepSource {
    pub entity_id: i32,
    pub uuid: uuid::Uuid,
    pub position: Vec3d,
    pub is_warden: bool,
    pub is_projectile: bool,
    pub projectile_owner: Option<uuid::Uuid>,
}

pub type SculkPlayerIdentity = crate::entity::passenger::PlayerIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SculkShriekerStepSource {
    pub player: SculkPlayerIdentity,
    pub warden_nearby: Option<bool>,
}

/// Short borrow of the actual store and the body temporarily ticking outside it.
pub struct EntityBlockContext<'a> {
    pub store: Option<&'a mut crate::entity::EntityStore>,
    pub current: Option<&'a Entity>,
    pub ancestors: Option<&'a crate::entity::store::DetachedEntityBodies<'a>>,
}

impl<'a> EntityBlockContext<'a> {
    pub fn new(
        store: Option<&'a mut crate::entity::EntityStore>,
        current: Option<&'a Entity>,
    ) -> Self {
        Self {
            store,
            current,
            ancestors: None,
        }
    }

    pub fn reborrow(&mut self) -> EntityBlockContext<'_> {
        EntityBlockContext {
            store: self.store.as_deref_mut(),
            current: self.current,
            ancestors: self.ancestors,
        }
    }
}

pub(crate) fn pathfinding_cost_from_light_levels(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<f32> {
    Some(light_level_dependent_magic_value(level, pos)? - 0.5)
}

pub(crate) fn light_level_dependent_magic_value(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<f32> {
    let v = level.max_local_raw_brightness(pos)? as f32 / 15.0;
    let curved = v / (4.0 - 3.0 * v);
    let ambient = level.dimension_ambient_light()?;
    Some(curved + ambient * (1.0 - curved))
}

pub(crate) fn builtin_registry_tag_membership(
    registry_id: &str,
    tag_id: &str,
    element_resource_id: &str,
) -> bool {
    match vanilla_persistent_registry_tag_members_26_1(registry_id, tag_id) {
        PersistentRegistryTagMembers::Present(members) => {
            members.iter().any(|member| member == element_resource_id)
        }
        PersistentRegistryTagMembers::Absent | PersistentRegistryTagMembers::Unavailable => false,
    }
}

pub(crate) fn block_state_tag_membership(
    level: &LevelCtx<'_, '_>,
    block_state_id: i32,
    tag_id: &str,
) -> bool {
    vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .is_some_and(|block| {
            level.registry_tag_membership("minecraft:block", tag_id, block.resource_id)
        })
}

pub(crate) fn fluid_state_tag_membership(
    level: &LevelCtx<'_, '_>,
    fluid_state: FluidState,
    tag_id: &str,
) -> bool {
    level.registry_tag_membership(
        "minecraft:fluid",
        tag_id,
        fluid_state.identity().resource_id(),
    )
}

pub(crate) fn block_state_water_pathfindable(
    level: &LevelCtx<'_, '_>,
    state_id: i32,
) -> Option<bool> {
    let Some(facts) = vanilla_block_state_path_facts(state_id) else {
        level.content_registry().block_by_state_id(state_id)?;
        return Some(false);
    };
    match facts.water_path_rule()? {
        VanillaWaterPathRule::Static => Some(facts.water_pathfindable()),
        VanillaWaterPathRule::StateFluidWater => Some(fluid_state_tag_membership(
            level,
            block_state_fluid_state(state_id),
            "minecraft:water",
        )),
        VanillaWaterPathRule::LiquidSourceNotLava => {
            let block_id = vanilla_block_state_block_type_id(state_id)?;
            let default_state = vanilla_block_default_state_id(block_id)?;
            Some(!fluid_state_tag_membership(
                level,
                block_state_fluid_state(default_state),
                "minecraft:lava",
            ))
        }
    }
}

#[cfg(test)]
mod block_write_tests;
#[cfg(test)]
mod light_tests;
