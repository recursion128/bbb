use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROAKING_ID, VANILLA_ENTITY_POSE_DIGGING_ID,
    VANILLA_ENTITY_POSE_EMERGING_ID, VANILLA_ENTITY_POSE_FALL_FLYING_ID,
    VANILLA_ENTITY_POSE_INHALING_ID, VANILLA_ENTITY_POSE_LONG_JUMPING_ID,
    VANILLA_ENTITY_POSE_ROARING_ID, VANILLA_ENTITY_POSE_SHOOTING_ID,
    VANILLA_ENTITY_POSE_SLIDING_ID, VANILLA_ENTITY_POSE_SNIFFING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID, VANILLA_ENTITY_POSE_USING_TONGUE_ID,
};
use bbb_protocol::packets::{EntityDataEnumSerializer, EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{LevelEventSoundRandomState, WorldStore};

use super::dimensions::entity_data_pose;
use super::dragon::{
    vanilla_ender_dragon_phase_is_landing_or_takeoff, DragonFlightHistorySample,
    EnderDragonAnimationState, EnderDragonClientSoundTriggers,
};
use super::renderer_catalog::vanilla_entity_renderer_uses_abstract_horse_renderer;
use super::schema::{
    vanilla_ender_dragon_phase_is_sitting,
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_boat_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
    vanilla_entity_metadata_allay_dancing_accessor_for_entity_type,
    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type,
    vanilla_entity_metadata_axolotl_playing_dead_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_accessors_for_entity_type,
    vanilla_entity_metadata_camel_dash_accessor_for_entity_type,
    vanilla_entity_metadata_cat_lying_accessor_for_entity_type,
    vanilla_entity_metadata_cat_relax_state_one_accessor_for_entity_type,
    vanilla_entity_metadata_copper_golem_accessors_for_entity_type,
    vanilla_entity_metadata_creaking_can_move_accessor_for_entity_type,
    vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type,
    vanilla_entity_metadata_creeper_accessors_for_entity_type,
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_guardian_accessors_for_entity_type,
    vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_panda_accessors_for_entity_type,
    vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type,
    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_sniffer_state_accessor_for_entity_type,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_guardian,
    vanilla_entity_metadata_type_is_hoglin_base,
    vanilla_entity_metadata_type_is_player_rideable_jumping, vanilla_entity_metadata_type_is_slime,
    vanilla_entity_metadata_vehicle_accessors_for_entity_type,
    vanilla_entity_metadata_warden_anger_level_accessor_for_entity_type,
    vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type,
    vanilla_entity_metadata_wolf_interested_accessor_for_entity_type,
    VanillaVehicleMetadataAccessors,
    VANILLA_ARMADILLO_STATE_ROLLING_ID as ARMADILLO_STATE_ROLLING_ID,
    VANILLA_ARMADILLO_STATE_SCARED_ID as ARMADILLO_STATE_SCARED_ID,
    VANILLA_ARMADILLO_STATE_UNROLLING_ID as ARMADILLO_STATE_UNROLLING_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID as COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_ITEM_ID as COPPER_GOLEM_STATE_GETTING_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID as COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID,
    VANILLA_COPPER_GOLEM_STATE_IDLE_ID as COPPER_GOLEM_STATE_IDLE_ID,
    VANILLA_SNIFFER_STATE_DIGGING_ID as SNIFFER_STATE_DIGGING_ID,
    VANILLA_SNIFFER_STATE_FEELING_HAPPY_ID as SNIFFER_STATE_FEELING_HAPPY_ID,
    VANILLA_SNIFFER_STATE_RISING_ID as SNIFFER_STATE_RISING_ID,
    VANILLA_SNIFFER_STATE_SCENTING_ID as SNIFFER_STATE_SCENTING_ID,
    VANILLA_SNIFFER_STATE_SEARCHING_ID as SNIFFER_STATE_SEARCHING_ID,
    VANILLA_SNIFFER_STATE_SNIFFING_ID as SNIFFER_STATE_SNIFFING_ID,
};
use super::EnderDragonAnimationRenderState;
use super::{
    is_vanilla_abstract_nautilus_type, is_vanilla_boat_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type, vanilla_entity_type_is_squid,
    EntityCameraViewBobState, EntityTransform, EntityVec3, VanillaEntityEventKind,
    VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
};
use bbb_protocol::entity_types::*;

mod body_rotation;
mod dash_and_random;
mod kind_states;
mod mob_queries;
mod render_queries;

mod constants;
mod living_states;
mod mob_animators;
mod mob_states;
mod tick_advance;
mod vehicle_states;
use crate::entities::entity_data::{
    entity_data_bool, entity_data_byte, entity_data_enum_id, entity_data_float, entity_data_int,
    entity_data_value,
};
pub(crate) use body_rotation::MobBodyRotationState;
pub(crate) use constants::*;
pub use kind_states::*;
pub use living_states::*;
pub use mob_animators::*;
pub use mob_states::*;
pub use vehicle_states::*;

pub(crate) use tick_advance::*;
// Vanilla `EntityType.ELDER_GUARDIAN` / `EntityType.GUARDIAN` both share
// `GuardianModel` and the same client `Guardian.aiStep` tail accumulator (the
// elder is the guardian mesh scaled 2.35×, with no animation override). Their
// generated metadata aliases follow `Entity`/`LivingEntity`/`Mob` at ids 16/17.

/// Vanilla `FlyingAnimal` test used by `calculateEntityAnimation` to decide
/// whether the limb-swing travel distance includes the vertical component.
fn is_flying_animal(entity_type_id: i32) -> bool {
    super::schema::vanilla_entity_metadata_type_is_flying_animal(entity_type_id)
}

/// Vanilla `updateWalkAnimation`'s per-entity distance→target-speed mapping, fed to
/// `WalkAnimationState.update`. The base `LivingEntity` clamps `distance · 4` to `1.0`; `Creaking`
/// overrides it to `distance · 25` clamped to `3.0` (a faster, higher-amplitude gait that ramps the
/// limb-swing position ~3× quicker). `Frog.updateWalkAnimation` uses `distance · 25` clamped to `1.0`,
/// but forces the target to `0` while `jumpAnimationState.isStarted()`. `Camel.updateWalkAnimation`
/// uses `distance · 6` clamped to `1.0` with a slower `0.2` easing factor, and targets `0` when the
/// camel is not standing or the dash animation is active.
fn walk_update_target_speed(
    entity_type_id: i32,
    distance: f32,
    frog_jump_started: bool,
    camel_walk_active: bool,
) -> (f32, f32) {
    match entity_type_id {
        entity_type_id
            if vanilla_entity_metadata_type_is_camel(entity_type_id) && camel_walk_active =>
        {
            ((distance * 6.0).min(1.0), 0.2)
        }
        entity_type_id if vanilla_entity_metadata_type_is_camel(entity_type_id) => (0.0, 0.2),
        VANILLA_ENTITY_TYPE_CREAKING_ID => ((distance * 25.0).min(3.0), 0.4),
        VANILLA_ENTITY_TYPE_FROG_ID if frog_jump_started => (0.0, 0.4),
        VANILLA_ENTITY_TYPE_FROG_ID => ((distance * 25.0).min(1.0), 0.4),
        _ => ((distance * 4.0).min(1.0), 0.4),
    }
}

/// Vanilla `WitherBoss.getAlternativeTarget(1/2)`: side heads read
/// `DATA_TARGET_B/C`. The generated accessor pair supplies membership, wire indices and defaults;
/// target `0` means "no target".
pub(crate) fn wither_side_head_target_ids(
    entity_type_id: i32,
    data_values: &[EntityDataValue],
) -> [i32; 2] {
    vanilla_entity_metadata_wither_side_head_target_accessors_for_entity_type(entity_type_id)
        .map(|accessors| {
            [
                entity_data_int(
                    data_values,
                    accessors.target_b_data_id,
                    accessors.target_b_default,
                ),
                entity_data_int(
                    data_values,
                    accessors.target_c_data_id,
                    accessors.target_c_default,
                ),
            ]
        })
        .unwrap_or_default()
}

/// Vanilla `WitherBoss.aiStep` target-angle math for side head `head_index`
/// (`0` = right/head B, `1` = left/head C). The returned yaw is absolute
/// `yRotHeads[index]`, later converted in the model as `yHeadRot - bodyRot`.
pub(crate) fn wither_side_head_target_rotation(
    wither_position: EntityVec3,
    y_body_rot: f32,
    scale: f32,
    head_index: usize,
    target_position: EntityVec3,
    target_eye_height: f32,
) -> WitherHeadTargetRotations {
    let vanilla_index = head_index + 1;
    let head_angle = (y_body_rot + 180.0 * (vanilla_index as f32 - 1.0)).to_radians();
    let hx = wither_position.x + f64::from(head_angle.cos() * 1.3 * scale);
    let hy = wither_position.y + f64::from(2.2 * scale);
    let hz = wither_position.z + f64::from(head_angle.sin() * 1.3 * scale);
    let xd = target_position.x - hx;
    let yd = target_position.y + f64::from(target_eye_height) - hy;
    let zd = target_position.z - hz;
    let sd = (xd * xd + zd * zd).sqrt();
    WitherHeadTargetRotations {
        x_rot: Some(-(yd.atan2(sd).to_degrees() as f32)),
        y_rot: zd.atan2(xd).to_degrees() as f32 - 90.0,
    }
}

fn rotlerp(a: f32, b: f32, max: f32) -> f32 {
    let diff = wrap_degrees(b - a).clamp(-max, max);
    a + diff
}

fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

impl EntityClientAnimationState {
    pub(crate) fn advance_nautilus_subclass_tick(
        &mut self,
        entity_id: i32,
        is_dashing: bool,
        in_water: bool,
        delta_movement: EntityVec3,
    ) -> EntityClientTickParticleTriggers {
        let mut triggers = EntityClientTickParticleTriggers::default();
        let Some(nautilus) = self.nautilus_kind_mut() else {
            return triggers;
        };
        let clear_dash = nautilus
            .dash_cooldown
            .as_mut()
            .map_or(is_dashing, |cooldown| {
                cooldown.advance_subclass_tick(is_dashing)
            });
        triggers.nautilus_clear_dash = clear_dash;
        if nautilus
            .dash_cooldown
            .is_some_and(NautilusDashCooldownState::is_settled)
        {
            nautilus.dash_cooldown = None;
        }

        let random_seed = nautilus.random.get_or_insert_with(|| {
            java_random_seed(i64::from(entity_id).wrapping_add(NAUTILUS_CLIENT_RANDOM_SALT as i64))
        });
        if in_water {
            let speed = (delta_movement.x * delta_movement.x
                + delta_movement.y * delta_movement.y
                + delta_movement.z * delta_movement.z)
                .sqrt();
            let probability = (speed * NAUTILUS_BUBBLE_PROBABILITY_MULTIPLIER).clamp(
                NAUTILUS_BUBBLE_PROBABILITY_MIN,
                NAUTILUS_BUBBLE_PROBABILITY_MAX,
            );
            if f64::from(java_random_next_float(random_seed)) < probability {
                let spread = java_random_next_double(random_seed)
                    * NAUTILUS_BUBBLE_SPREAD_FACTOR
                    * (1.0 + speed);
                triggers.nautilus_bubble = Some(EntityVec3 {
                    x: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                    y: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                    z: (f64::from(java_random_next_float(random_seed)) - 0.5) * spread,
                });
            }
        }
        triggers
    }

    pub(crate) fn initialize_spawn_defaults(&mut self, entity_type_id: i32, uuid: Uuid) {
        let seed = |salt: u64| java_random_seed(spawn_random_bits(uuid, salt) as i64);
        let typed_seed = |salt: u64| Some(seed(salt ^ entity_type_id as u64));
        self.kind = EntityKindAnimation::for_entity_type(entity_type_id);
        match &mut self.kind {
            EntityKindAnimation::Item(offset) => {
                *offset = spawn_random_unit_float(uuid, 0x6974_656d) * std::f32::consts::TAU;
            }
            EntityKindAnimation::EndCrystal(offset) => {
                *offset = (spawn_random_bits(uuid, 0x0063_7279_7374_616c) % 100_000) as u32;
            }
            EntityKindAnimation::Guardian(guardian) => {
                let mut random_seed = seed(GUARDIAN_CLIENT_RANDOM_SALT ^ entity_type_id as u64);
                guardian.tail = Some(GuardianTailAnimationState::with_initial_phase(
                    java_random_next_float(&mut random_seed),
                ));
                guardian.random = Some(random_seed);
            }
            EntityKindAnimation::Minecart(minecart)
                if entity_type_id == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID =>
            {
                minecart.furnace_random = Some(seed(0x6675_726e_6163_655f));
            }
            EntityKindAnimation::Blaze { random } => *random = Some(seed(BLAZE_CLIENT_RANDOM_SALT)),
            EntityKindAnimation::EnderAmbient { random, .. } => {
                *random = typed_seed(ENDER_AMBIENT_RANDOM_SALT);
            }
            EntityKindAnimation::Warden(warden) => {
                warden.random = typed_seed(WARDEN_CLIENT_RANDOM_SALT);
            }
            EntityKindAnimation::EnderDragon(dragon) => {
                if let Some(seed_val) = typed_seed(ENDER_DRAGON_CLIENT_RANDOM_SALT) {
                    dragon.random_seed = seed_val;
                    dragon.random_initialized = true;
                } else {
                    dragon.random_seed = 0;
                    dragon.random_initialized = false;
                }
            }
            EntityKindAnimation::LightningBolt(bolt) => {
                let (random_seed, render_seed) = lightning_bolt_spawn_random(uuid, entity_type_id);
                bolt.random = Some(random_seed);
                bolt.render_seed = Some(render_seed);
            }
            EntityKindAnimation::Nautilus(nautilus) => {
                nautilus.random = typed_seed(NAUTILUS_CLIENT_RANDOM_SALT);
            }
            EntityKindAnimation::Slime(slime) => {
                slime.random = typed_seed(SLIME_CLIENT_RANDOM_SALT)
            }
            _ => {}
        }
        self.item_based_steering =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                entity_type_id,
            )
            .map(|_| ItemBasedSteeringClientState::default());
        self.ridden_jump = vanilla_entity_metadata_type_is_player_rideable_jumping(entity_type_id)
            .then(RiddenJumpState::default);
    }

    pub(crate) fn sync_events_from_metadata_update(
        &mut self,
        entity_type_id: i32,
        updated_values: &[EntityDataValue],
        data_values: &[EntityDataValue],
    ) {
        // Vanilla `AbstractArrow.onSyncedDataUpdated(IN_GROUND)`: once the entity is past
        // its first client tick, setting `IN_GROUND` true starts the seven-tick impact
        // wobble if the previous shake has already settled. This is class inheritance,
        // not the `#arrows` entity-type tag: ThrownTrident follows the same callback.
        if let Some(accessors) =
            vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
        {
            if self.age_ticks > 0
                && updated_values
                    .iter()
                    .any(|value| value.data_id == accessors.in_ground_data_id)
                && entity_data_bool(
                    data_values,
                    accessors.in_ground_data_id,
                    accessors.in_ground_default,
                )
                && self
                    .arrow_kind()
                    .is_none_or(ArrowShakeAnimationState::is_settled)
            {
                if let Some(slot) = self.arrow_kind_mut() {
                    *slot = Some(ArrowShakeAnimationState::started());
                }
            }
        }
        if let Some(accessors) =
            vanilla_entity_metadata_vehicle_accessors_for_entity_type(entity_type_id)
        {
            if updated_values.iter().any(|value| {
                value.data_id == accessors.hurt_time_data_id
                    || value.data_id == accessors.hurt_dir_data_id
                    || value.data_id == accessors.damage_data_id
            }) {
                let damage = BoatDamageAnimationState::from_metadata(accessors, data_values);
                if let Some(slot) = self.vehicle_damage_mut() {
                    *slot = (!damage.is_settled()).then_some(damage);
                }
            }
        }
        // `AbstractNautilus.onSyncedDataUpdated(DASH)` seeds the cooldown after
        // the first entity tick regardless of the synchronized boolean value.
        if is_vanilla_abstract_nautilus_type(entity_type_id)
            && self.age_ticks > 0
            && updated_values
                .iter()
                .any(|value| value.data_id == VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID)
        {
            if let Some(nautilus) = self.nautilus_kind_mut() {
                nautilus
                    .dash_cooldown
                    .get_or_insert_with(Default::default)
                    .seed_from_synced_update();
            }
        }
        if let Some((boost_time_data_id, _)) =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                entity_type_id,
            )
        {
            if updated_values
                .iter()
                .any(|value| value.data_id == boost_time_data_id)
            {
                self.item_based_steering
                    .get_or_insert_with(Default::default)
                    .on_synced();
            }
        }
    }

    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        self.sync_targets_from_metadata_with_registry(
            entity_type_id,
            data_values,
            ContentRegistry::vanilla(),
        );
    }

    pub(crate) fn sync_targets_from_metadata_with_registry(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
        content_registry: &ContentRegistry,
    ) {
        self.ensure_kind(entity_type_id);
        let age_ticks = self.age_ticks;
        // Vanilla `LivingEntity.isDeadOrDying()` (`getHealth() <= 0`): a living
        // entity whose synced health has reached zero begins the death animation;
        // restoring health clears it. Only living entities carry the health float,
        // so the per-type model animations below are unaffected.
        let living_accessors =
            super::content_facts::entity_living_accessors(entity_type_id, content_registry);
        if super::content_facts::entity_is_living(entity_type_id, content_registry)
            && entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
        {
            if let Some(health) = living_accessors
                .and_then(|accessors| entity_data_value(data_values, accessors.health_data_id))
                .and_then(|value| match value {
                    EntityDataValueKind::Float(health) => Some(*health),
                    _ => None,
                })
            {
                if health <= 0.0 {
                    self.death
                        .get_or_insert(DeathAnimationState { death_time: 0 });
                } else {
                    self.death = None;
                }
            }
            // Vanilla `LivingEntity.isVisuallySwimming()` also treats the
            // FALL_FLYING pose as swimming while the shared fall-flying flag is
            // clear. This is the crawling transition used by humanoid head/root
            // animation, not merely `Entity.isSwimming()`.
            let pose = entity_data_pose(data_values);
            let visually_swimming = pose == VANILLA_ENTITY_POSE_SWIMMING_ID
                || (pose == VANILLA_ENTITY_POSE_FALL_FLYING_ID
                    && !entity_is_fall_flying(data_values));
            if let Some(swim) = self.living_swim.as_mut() {
                swim.set_visually_swimming(visually_swimming);
            } else if visually_swimming {
                self.living_swim = Some(LivingSwimAmountState {
                    visually_swimming,
                    previous_swim_amount: 0.0,
                    current_swim_amount: 0.0,
                });
            }
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            if let Some(health) = living_accessors
                .and_then(|accessors| entity_data_value(data_values, accessors.health_data_id))
                .and_then(|value| match value {
                    EntityDataValueKind::Float(health) => Some(*health),
                    _ => None,
                })
            {
                if let Some(dragon) = self.ender_dragon_kind_mut() {
                    dragon
                        .state
                        .get_or_insert_with(EnderDragonAnimationState::default)
                        .set_dragon_dying(health <= 0.0);
                }
            }
        }
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                let target_standing =
                    vanilla_entity_metadata_polar_bear_standing_accessor_for_entity_type(
                        entity_type_id,
                    )
                    .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
                    .unwrap_or_default();
                if let Some(standing) = self.polar_bear_kind_mut().and_then(|slot| slot.as_mut()) {
                    standing.set_target(target_standing);
                } else if target_standing {
                    if let Some(slot) = self.polar_bear_kind_mut() {
                        *slot = Some(PolarBearStandingAnimationState {
                            target_standing,
                            ..PolarBearStandingAnimationState::default()
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                let swell_dir = creeper_effective_swell_dir(entity_type_id, data_values);
                if let Some(swell) = self.creeper_kind_mut().and_then(|slot| slot.as_mut()) {
                    swell.set_swell_dir(swell_dir);
                } else if swell_dir > 0 {
                    if let Some(slot) = self.creeper_kind_mut() {
                        *slot = Some(CreeperSwellAnimationState {
                            swell_dir,
                            previous_swell: 0,
                            current_swell: 0,
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                let target_peek_amount = shulker_target_peek_amount(entity_type_id, data_values);
                if let Some(peek) = self
                    .shulker_kind_mut()
                    .map(|k| &mut k.peek)
                    .and_then(|slot| slot.as_mut())
                {
                    peek.set_target(target_peek_amount);
                } else if target_peek_amount > 0.0 {
                    if let Some(slot) = self.shulker_kind_mut().map(|k| &mut k.peek) {
                        *slot = Some(ShulkerPeekAnimationState {
                            target_peek_amount,
                            ..ShulkerPeekAnimationState::default()
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_BEE_ID => {
                let rolling = bee_is_rolling(entity_type_id, data_values);
                if let Some(roll) = self.bee_kind_mut().and_then(|slot| slot.as_mut()) {
                    roll.set_rolling(rolling);
                } else if rolling {
                    if let Some(slot) = self.bee_kind_mut() {
                        *slot = Some(BeeRollAnimationState {
                            rolling,
                            ..BeeRollAnimationState::default()
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_PANDA_ID => {
                let (sitting, on_back, rolling) = panda_pose_flags(entity_type_id, data_values);
                if let Some(panda) = self.panda_kind_mut().and_then(|slot| slot.as_mut()) {
                    panda.set_flags(sitting, on_back, rolling);
                } else if sitting || on_back || rolling {
                    if let Some(slot) = self.panda_kind_mut() {
                        *slot = Some(PandaAnimationState {
                            sitting,
                            on_back,
                            rolling,
                            ..PandaAnimationState::default()
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_FOX_ID => {
                let accessors =
                    vanilla_entity_metadata_fox_accessors_for_entity_type(entity_type_id)
                        .expect("generated Fox metadata accessors must resolve");
                let flags = entity_data_byte(
                    data_values,
                    accessors.flags_data_id,
                    accessors.flags_default,
                );
                let interested = flags & accessors.interested_mask != 0;
                let crouching = flags & accessors.crouching_mask != 0;
                if let Some(fox) = self.fox_kind_mut().and_then(|slot| slot.as_mut()) {
                    fox.set_flags(interested, crouching);
                } else if interested || crouching {
                    if let Some(slot) = self.fox_kind_mut() {
                        *slot = Some(FoxAnimationState {
                            interested,
                            crouching,
                            ..FoxAnimationState::default()
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_CAT_ID => {
                let is_lying = cat_is_lying(entity_type_id, data_values);
                let relax_state_one = cat_relax_state_one(entity_type_id, data_values);
                if let Some(cat) = self.cat_kind_mut().and_then(|slot| slot.as_mut()) {
                    cat.set_flags(is_lying, relax_state_one);
                } else if is_lying || relax_state_one {
                    if let Some(slot) = self.cat_kind_mut() {
                        *slot = Some(CatLieDownAnimationState {
                            is_lying,
                            relax_state_one,
                            ..CatLieDownAnimationState::default()
                        });
                    }
                }
            }
            entity_type_id
                if vanilla_entity_renderer_uses_abstract_horse_renderer(entity_type_id) =>
            {
                let (eating, standing, open_mouth) = equine_pose_flags(entity_type_id, data_values);
                if let Some(pose) = self
                    .equine_kind_mut()
                    .map(|k| &mut k.pose)
                    .and_then(|slot| slot.as_mut())
                {
                    pose.set_flags(eating, standing, open_mouth);
                } else if eating || standing || open_mouth {
                    if let Some(slot) = self.equine_kind_mut().map(|k| &mut k.pose) {
                        *slot = Some(EquinePoseAnimationState::from_flags(
                            eating, standing, open_mouth,
                        ));
                    }
                }
            }
            VANILLA_ENTITY_TYPE_FROG_ID => {
                // Vanilla `Frog.onSyncedDataUpdated`: when `DATA_POSE` changes, the croak animation
                // is `animateWhen(pose == CROAKING, tickCount)` and the jump animation is started
                // when `pose == LONG_JUMPING` and stopped otherwise — both started on the rising edge
                // into their pose and stopped on leaving it. The frog's `aiStep` runs client-side for
                // remote entities, so the synced pose drives both directly.
                let pose = entity_data_pose(data_values);
                let croaking = pose == VANILLA_ENTITY_POSE_CROAKING_ID;
                if let Some(croak) = self
                    .frog_kind_mut()
                    .map(|k| &mut k.croak)
                    .and_then(|slot| slot.as_mut())
                {
                    croak.animate_when(croaking, age_ticks);
                } else if croaking {
                    if let Some(slot) = self.frog_kind_mut().map(|k| &mut k.croak) {
                        *slot = Some(KeyframeAnimationState {
                            start_age: Some(age_ticks),
                        });
                    }
                }
                // Vanilla `Frog.onSyncedDataUpdated`: `tongueAnimationState` is started on the synced
                // `Pose.USING_TONGUE` and stopped otherwise — the same rising-edge pattern as the
                // croak, driving the `FROG_TONGUE` keyframe (head dip + the tongue's z-scale lash).
                let using_tongue = pose == VANILLA_ENTITY_POSE_USING_TONGUE_ID;
                if let Some(tongue) = self
                    .frog_kind_mut()
                    .map(|k| &mut k.tongue)
                    .and_then(|slot| slot.as_mut())
                {
                    tongue.animate_when(using_tongue, age_ticks);
                } else if using_tongue {
                    if let Some(slot) = self.frog_kind_mut().map(|k| &mut k.tongue) {
                        *slot = Some(KeyframeAnimationState {
                            start_age: Some(age_ticks),
                        });
                    }
                }
                let jumping = pose == VANILLA_ENTITY_POSE_LONG_JUMPING_ID;
                if let Some(jump) = self
                    .frog_kind_mut()
                    .map(|k| &mut k.jump)
                    .and_then(|slot| slot.as_mut())
                {
                    jump.animate_when(jumping, age_ticks);
                } else if jumping {
                    if let Some(slot) = self.frog_kind_mut().map(|k| &mut k.jump) {
                        *slot = Some(KeyframeAnimationState {
                            start_age: Some(age_ticks),
                        });
                    }
                }
            }
            VANILLA_ENTITY_TYPE_WARDEN_ID => {
                // Vanilla `Warden.onSyncedDataUpdated(DATA_POSE)`: the pose-change `switch` that
                // `.start()`s the roar/sniff/emerge/dig one-shot when the synced `DATA_POSE` CHANGES
                // to `Pose.ROARING`/`Pose.SNIFFING`/`Pose.EMERGING`/`Pose.DIGGING`. We track the
                // previous pose and restart the timer only on the transition; vanilla never
                // auto-stops on a pose leave, so the non-looping keyframe holds its final frame. The
                // warden's `aiStep` runs client-side for remote entities, so the synced pose drives
                // the pose directly. (The attack/sonic-boom one-shots are event-driven.)
                let pose_id = entity_data_pose(data_values);
                if let Some(warden) = self.warden_kind_mut() {
                    warden
                        .combat
                        .get_or_insert_with(WardenCombatAnimationState::default)
                        .set_pose(pose_id, age_ticks);
                }
            }
            VANILLA_ENTITY_TYPE_BREEZE_ID => {
                // Vanilla `Breeze.onSyncedDataUpdated(DATA_POSE)` + `Breeze.tick`: the synced pose
                // drives the shoot/inhale/slide/longJump one-shots (active while their pose holds) and
                // the slideBack return (on leaving SLIDING). The breeze's `tick` runs client-side for
                // remote entities, so the synced pose drives all of them directly.
                let pose_id = entity_data_pose(data_values);
                if let Some(breeze) = self.breeze_kind_mut() {
                    breeze
                        .get_or_insert_with(BreezeAnimationState::default)
                        .set_pose(pose_id, age_ticks);
                }
            }
            VANILLA_ENTITY_TYPE_SNIFFER_ID => {
                // Vanilla `Sniffer.onSyncedDataUpdated(DATA_STATE)`: the synced state selects
                // the one mutually-exclusive `AnimationState` to start, and a state change
                // `resetAnimations()` + restarts it. The sniffer's `aiStep` runs client-side for
                // remote entities, so the synced state drives the pose directly. `DATA_STATE` is
                // serialized as the `SnifferState` enum (serializer 35), so it arrives as an
                // `EnumId`, not a plain `Int`.
                if let Some(state_id) = sniffer_state_id(entity_type_id, data_values) {
                    if let Some(sniffer) = self.sniffer_kind_mut().and_then(|slot| slot.as_mut()) {
                        sniffer.set_state(state_id, age_ticks);
                    } else if sniffer_animated_state(state_id).is_some() {
                        if let Some(slot) = self.sniffer_kind_mut() {
                            *slot = Some(SnifferAnimationState {
                                state_id,
                                keyframe: KeyframeAnimationState {
                                    start_age: Some(age_ticks),
                                },
                            });
                        }
                    }
                }
            }
            VANILLA_ENTITY_TYPE_ARMADILLO_ID => {
                // Vanilla `Armadillo.onSyncedDataUpdated(ARMADILLO_STATE)` → `setupAnimationStates`:
                // the synced `ArmadilloState` id selects the shell-ball hide window and the
                // rollUp/rollOut/peek `AnimationState`s. The armadillo's `aiStep` runs client-side for
                // remote entities, so the synced state drives the transitions directly. We track the
                // state and the `age_ticks` at the change (for `inStateTicks`); IDLE carries no hide
                // or transition timer but must still be tracked so a later roll re-anchors correctly.
                // `ARMADILLO_STATE` is serialized as the `ArmadilloState` enum (serializer 36), so it
                // arrives as an `EnumId`, not a plain `Int`.
                if let Some((data_id, default)) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
                {
                    let state_id = entity_data_enum_id(
                        data_values,
                        data_id,
                        EntityDataEnumSerializer::ArmadilloState,
                        default,
                    );
                    if let Some(armadillo) =
                        self.armadillo_kind_mut().and_then(|slot| slot.as_mut())
                    {
                        armadillo.set_state(state_id, age_ticks);
                    } else if state_id != default {
                        if let Some(slot) = self.armadillo_kind_mut() {
                            *slot = Some(ArmadilloAnimationState::new(state_id, age_ticks));
                        }
                    }
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                if let Some((data_id, default)) =
                    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
                        entity_type_id,
                    )
                {
                    let phase_id = entity_data_int(data_values, data_id, default);
                    // Vanilla `Mob.isNoAi()` reads the shared mob-flags bit; the client flap
                    // advance pins `flapTime = 0.5` while it is set.
                    let no_ai =
                        vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
                            .is_some_and(|accessors| {
                                entity_data_byte(
                                    data_values,
                                    accessors.flags_data_id,
                                    accessors.flags_default,
                                ) & accessors.no_ai_mask
                                    != 0
                            });
                    if let Some(dragon) = self
                        .ender_dragon_kind_mut()
                        .map(|k| &mut k.state)
                        .and_then(|slot| slot.as_mut())
                    {
                        dragon.set_phase(phase_id);
                        dragon.no_ai = no_ai;
                    } else {
                        if let Some(slot) = self.ender_dragon_kind_mut().map(|k| &mut k.state) {
                            *slot = Some(EnderDragonAnimationState {
                                phase_id,
                                no_ai,
                                ..EnderDragonAnimationState::default()
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Projects a client entity event into client animation state. Vanilla
    /// `ArmorStand.handleEntityEvent(32)` resets the hit wobble, and
    /// `Sheep.handleEntityEvent` resets the eat-grass animation on event `10`.
    pub(crate) fn handle_entity_event(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        event_kind: VanillaEntityEventKind,
    ) {
        self.ensure_kind(entity_type_id);
        let age_ticks = self.age_ticks;
        match event_kind {
            VanillaEntityEventKind::LivingKineticHit => {
                // Vanilla `LivingEntity.handleEntityEvent(2)` calls `onKineticHit()`, which
                // refreshes `lastKineticHitFeedbackTime` only when more than ten game ticks
                // elapsed since the previous accepted feedback.
                if self
                    .kinetic_hit_feedback
                    .is_none_or(|state| state.elapsed_ticks > KINETIC_HIT_FEEDBACK_RETRIGGER_TICKS)
                {
                    self.kinetic_hit_feedback =
                        Some(KineticHitFeedbackAnimationState { elapsed_ticks: 0 });
                }
            }
            VanillaEntityEventKind::ArmorStandHit => {
                // Vanilla `ArmorStand.handleEntityEvent(32)`: `lastHit = level.getGameTime()`;
                // the renderer derives `wiggle = gameTime - lastHit + partialTicks`.
                if let Some(slot) = self.armor_stand_kind_mut() {
                    *slot = Some(ArmorStandWiggleAnimationState { elapsed_ticks: 0 });
                }
            }
            VanillaEntityEventKind::SheepEatGrass => {
                if let Some(slot) = self.sheep_kind_mut() {
                    *slot = Some(SheepEatAnimationState {
                        eat_animation_tick: SHEEP_EAT_ANIMATION_TICKS,
                    });
                }
            }
            VanillaEntityEventKind::MinecartTntPrime => {
                if let Some(slot) = self.minecart_kind_mut().map(|k| &mut k.tnt_fuse) {
                    *slot = Some(MinecartTntFuseAnimationState {
                        fuse_remaining_ticks: TNT_MINECART_FUSE_TICKS,
                    });
                }
            }
            VanillaEntityEventKind::MinecartSpawnerDelayReset => {
                // `MinecartSpawner.handleEntityEvent` delegates every id to
                // `BaseSpawner.onEventTriggered`; id 1 on the client sets
                // `spawnDelay = minSpawnDelay`. Minecart spawners start with the BaseSpawner defaults.
                let Some(slot) = self.minecart_kind_mut().map(|k| &mut k.spawner) else {
                    return;
                };
                let spawner = slot.get_or_insert(MinecartSpawnerClientState {
                    spawn_delay: 20,
                    min_spawn_delay: 200,
                });
                spawner.spawn_delay = spawner.min_spawn_delay;
            }
            VanillaEntityEventKind::WardenTendril => {
                // Vanilla `Warden.handleEntityEvent(61)`: `tendrilAnimation = 10`. Only
                // `tendrilAnimation` (`current`) is set; `tendrilAnimationO` (`previous`)
                // is untouched, so the lerp fades in from the prior frame.
                if let Some(slot) = self.warden_kind_mut().map(|k| &mut k.tendril) {
                    slot.get_or_insert(WardenTendrilAnimationState {
                        previous: 0,
                        current: 0,
                    })
                    .current = WARDEN_TENDRIL_ANIMATION_TICKS;
                }
            }
            VanillaEntityEventKind::WardenAttack => {
                // Vanilla `Warden.handleEntityEvent(4)`: `roarAnimationState.stop()` then
                // `attackAnimationState.start(tickCount)` — the melee swing also cancels any roar.
                if let Some(slot) = self.warden_kind_mut().map(|k| &mut k.combat) {
                    slot.get_or_insert_with(WardenCombatAnimationState::default)
                        .start_attack(age_ticks);
                }
            }
            VanillaEntityEventKind::WardenSonicBoom => {
                // Vanilla `Warden.handleEntityEvent(62)`: `sonicBoomAnimationState.start(tickCount)`.
                if let Some(slot) = self.warden_kind_mut().map(|k| &mut k.combat) {
                    slot.get_or_insert_with(WardenCombatAnimationState::default)
                        .start_sonic_boom(age_ticks);
                }
            }
            VanillaEntityEventKind::SquidResetMovement => {
                // Vanilla `Squid.handleEntityEvent(19)`: `tentacleMovement = 0`. Only an
                // already-ticked squid has a state; if the event arrives first the
                // animation will start fresh on the next tick anyway, so nothing to do.
                if let Some(squid) = self.squid_kind_mut().and_then(|slot| slot.as_mut()) {
                    squid.reset_movement();
                }
            }
            VanillaEntityEventKind::WolfStartShaking => {
                if let Some(slot) = self.wolf_kind_mut().map(|k| &mut k.wet) {
                    slot.get_or_insert(WolfWetAnimationState {
                        is_wet: false,
                        is_shaking: false,
                        previous_shake_anim: 0.0,
                        current_shake_anim: 0.0,
                    })
                    .start_shaking();
                }
            }
            VanillaEntityEventKind::WolfCancelShaking => {
                if let Some(wolf_wet) = self
                    .wolf_kind_mut()
                    .map(|k| &mut k.wet)
                    .and_then(|slot| slot.as_mut())
                {
                    wolf_wet.cancel_shake();
                    if wolf_wet.is_settled() {
                        if let Some(slot) = self.wolf_kind_mut().map(|k| &mut k.wet) {
                            *slot = None;
                        }
                    }
                }
            }
            VanillaEntityEventKind::GoatLowerHead | VanillaEntityEventKind::GoatRaiseHead => {
                // Vanilla `Goat.handleEntityEvent`: event 58 → `isLoweringHead = true`, 59 → `false`. The
                // `aiStep` counter advances from there; an idle goat with no state is created on the first
                // lower so its `lowerHeadTick` can climb (a raise event before any state is a no-op rest).
                if let Some(slot) = self.goat_kind_mut() {
                    slot.get_or_insert(GoatRammingAnimationState {
                        lowering_head: false,
                        lower_head_tick: 0,
                    })
                    .lowering_head = event_kind == VanillaEntityEventKind::GoatLowerHead;
                }
            }
            VanillaEntityEventKind::IronGolemAttack
            | VanillaEntityEventKind::IronGolemOfferFlower
            | VanillaEntityEventKind::IronGolemStopOfferFlower => {
                // Vanilla `IronGolem.handleEntityEvent`: event 4 → `attackAnimationTick = 10`, 11 →
                // `offerFlowerTick = 400`, 34 → `offerFlowerTick = 0`. The `aiStep` decrements both.
                let Some(slot) = self.iron_golem_kind_mut() else {
                    return;
                };
                let golem = slot.get_or_insert(IronGolemAnimationState {
                    attack_animation_tick: 0,
                    offer_flower_tick: 0,
                });
                match event_kind {
                    VanillaEntityEventKind::IronGolemAttack => {
                        golem.attack_animation_tick = IRON_GOLEM_ATTACK_TICKS
                    }
                    VanillaEntityEventKind::IronGolemOfferFlower => {
                        golem.offer_flower_tick = IRON_GOLEM_OFFER_FLOWER_TICKS
                    }
                    _ => golem.offer_flower_tick = 0,
                }
            }
            VanillaEntityEventKind::RavagerAttack | VanillaEntityEventKind::RavagerStun => {
                // Vanilla `Ravager.handleEntityEvent`: event 4 → `attackTick = 10`, 39 → `stunnedTick = 40`.
                // The `aiStep` decrements them and arms the post-stun roar.
                let Some(slot) = self.ravager_kind_mut() else {
                    return;
                };
                let ravager = slot.get_or_insert_with(|| RavagerAnimationState::new(entity_id));
                if event_kind == VanillaEntityEventKind::RavagerAttack {
                    ravager.attack_tick = RAVAGER_ATTACK_TICKS;
                } else {
                    ravager.stunned_tick = RAVAGER_STUN_TICKS;
                }
            }
            VanillaEntityEventKind::HoglinAttack | VanillaEntityEventKind::ZoglinAttack => {
                // Vanilla `Hoglin`/`Zoglin.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 10`.
                if let Some(slot) = self.hoglin_kind_mut() {
                    slot.get_or_insert(HoglinAnimationState {
                        attack_animation_tick: 0,
                    })
                    .attack_animation_tick = HOGLIN_ATTACK_TICKS;
                }
            }
            VanillaEntityEventKind::RabbitJump => {
                // Vanilla `Rabbit.handleEntityEvent(1)`: seed the hop reconstruction (`jumpDuration = 15;
                // jumpTicks = 0`); the next tick's `setupAnimationStates` starts the hop keyframe.
                if let Some(slot) = self.rabbit_kind_mut() {
                    slot.get_or_insert(RabbitHopAnimationState {
                        jump_ticks: 0,
                        jump_duration: 0,
                        hop: KeyframeAnimationState { start_age: None },
                    })
                    .start_jump();
                }
            }
            VanillaEntityEventKind::ArmadilloPeek => {
                // Vanilla `Armadillo.handleEntityEvent(64)`: set `peekReceivedClient`; the next
                // `setupAnimationStates()` consumes it by stopping the old peek and immediately
                // restarting it for the current SCARED state.
                if let Some((_, default)) =
                    vanilla_entity_metadata_armadillo_state_accessor_for_entity_type(entity_type_id)
                {
                    if let Some(slot) = self.armadillo_kind_mut() {
                        slot.get_or_insert_with(|| {
                            ArmadilloAnimationState::new(default, age_ticks)
                        })
                        .peek_received_client = true;
                    }
                }
            }
            VanillaEntityEventKind::CreakingAttack
            | VanillaEntityEventKind::CreakingInvulnerable => {
                // Vanilla `Creaking.handleEntityEvent`: event 4 → `attackAnimationRemainingTicks = 15`,
                // 66 → `invulnerabilityAnimationRemainingTicks = 8`. The `aiStep` countdown advances from
                // there; the next tick's `setupAnimationStates` starts the matching keyframe one-shot.
                let Some(slot) = self.creaking_kind_mut() else {
                    return;
                };
                let creaking = slot.get_or_insert_with(|| CreakingAnimationState::new(entity_id));
                if event_kind == VanillaEntityEventKind::CreakingAttack {
                    creaking.start_attack();
                } else {
                    creaking.start_invulnerable();
                }
            }
            VanillaEntityEventKind::EvokerFangsAttack => {
                // Vanilla `EvokerFangs.handleEntityEvent`: event 4 → `clientSideAttackStarted = true`,
                // arming the `lifeTicks` countdown that `tick` runs.
                if let Some(slot) = self.evoker_fangs_kind_mut() {
                    slot.get_or_insert_with(|| EvokerFangsAnimationState::new(entity_id))
                        .started = true;
                }
            }
            _ => {}
        }
    }

    /// Resets the hurt animation countdown to [`HURT_ANIMATION_DURATION`],
    /// mirroring vanilla `LivingEntity.animateHurt` / `handleDamageEvent` setting
    /// `hurtTime = hurtDuration`.
    pub(crate) fn trigger_hurt(&mut self) {
        self.hurt = Some(HurtAnimationState {
            hurt_time: HURT_ANIMATION_DURATION,
        });
    }

    /// Vanilla `LivingEntity.handleDamageEvent` starts by applying an immediate
    /// `1.5F` walk-animation speed impulse, before the ordinary next-tick
    /// low-pass update.
    pub(crate) fn set_damage_event_walk_speed(&mut self) {
        self.walk_animation
            .get_or_insert_with(WalkAnimationState::default)
            .set_speed(1.5);
    }

    /// Vanilla `AbstractBoat.animateHurt` / `AbstractMinecart.animateHurt`: flip the shared
    /// `VehicleEntity.hurtDir`, reset `hurtTime` to ten ticks, and multiply damage by eleven.
    pub(crate) fn trigger_vehicle_hurt(&mut self) {
        if let Some(damage) = self.vehicle_damage_mut() {
            damage
                .get_or_insert_with(BoatDamageAnimationState::default)
                .animate_hurt();
        }
    }

    /// `LocalPlayer.move` and `LivingEntity.calculateEntityAnimation` consume actual
    /// travel. Each catch-up input step supplies its own positions after collision.
    pub(crate) fn advance_local_player_walk_animation(
        &mut self,
        old_position: EntityVec3,
        position: EntityVec3,
    ) {
        let walk = self
            .walk_animation
            .get_or_insert_with(WalkAnimationState::default);
        walk.previous_feet_position = Some(old_position);
        walk.advance_client_tick(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            position,
            false,
            false,
            self.death.is_none(),
            false,
            false,
            false,
        );
        if let Some(cloak) = self.player_kind_mut() {
            cloak
                .get_or_insert_with(PlayerCloakAnimationState::default)
                .advance_walk_distance(EntityVec3 {
                    x: position.x - old_position.x,
                    y: 0.0,
                    z: position.z - old_position.z,
                });
        }
    }

    pub(crate) fn start_head_rotation_lerp(&mut self, target_y_head_rot: f32, steps: i32) {
        self.head_rotation_lerp = Some(HeadRotationLerpState {
            target_y_head_rot,
            steps_remaining: steps.max(1),
        });
    }

    pub(crate) fn advance_head_rotation_lerp(&mut self, y_head_rot: &mut f32) {
        let Some(lerp) = self.head_rotation_lerp.as_mut() else {
            return;
        };
        *y_head_rot +=
            wrap_degrees(lerp.target_y_head_rot - *y_head_rot) / lerp.steps_remaining as f32;
        lerp.steps_remaining -= 1;
        if lerp.steps_remaining <= 0 {
            self.head_rotation_lerp = None;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_client_tick(
        &mut self,
        entity_type_id: i32,
        entity_id: i32,
        transform: EntityTransform,
        // Vanilla `ClientLevel.shouldTickDeath`: only this LivingEntity death
        // counter is simulation-distance gated. Other client animation/tick
        // state remains active for tracked entities.
        should_tick_death: bool,
        is_passenger: bool,
        is_baby: bool,
        is_fall_flying: bool,
        is_crouching: bool,
        // Vanilla `Bat.isResting()`, read from the synced flags byte before `Bat.tick` calls
        // `setupAnimationStates`. It selects which of the two bat AnimationStates remains started.
        bat_is_resting: bool,
        // The per-tick world fact `Guardian.aiStep` reads via `isInWater()`,
        // resolved by [`WorldStore::advance_entity_client_animations`] (which holds
        // the chunk/fluid data this context lacks). `false` for entities that do
        // not consume it. See [`entity_animation_uses_in_water`].
        in_water: bool,
        // The synced `Guardian.DATA_ID_MOVING` flag (`isMoving()`), read from the
        // entity metadata in the tick loop. `false` for entities that do not
        // consume it.
        is_moving: bool,
        // Vanilla dry Guardian `loadedAndEntityCanStandOn(blockPosition().below())`,
        // resolved by WorldStore before the mutable ECS pass.
        guardian_can_stand_on_below: bool,
        // Vanilla `Warden.getHeartBeatDelay()` over the synced anger, read from the
        // entity metadata in the tick loop ([`warden_heartbeat_delay`]). The calm
        // `40` for entities that do not consume it.
        warden_heartbeat_delay: u32,
        // The synced `Guardian.DATA_ID_ATTACK_TARGET` (`isMoving`'s sibling), read from the entity
        // metadata in the tick loop ([`guardian_attack_target_id`]). `0` (no target) for entities that
        // do not consume it.
        guardian_attack_target_id: i32,
        // Narrow live target dimensions resolved once per unique protocol id
        // after the direct-movement pass. `None` for missing/non-living targets.
        guardian_attack_target: Option<GuardianAttackTargetTickInput>,
        guardian_eye_height: Option<f32>,
        // Vanilla `WitherBoss.aiStep` side-head targets, pre-resolved by `EntityStore` because the
        // tick needs other entities' current positions and eye heights. `None` for non-withers.
        wither_head_targets: Option<[WitherHeadTargetRotations; 2]>,
        // Vanilla `Camel.updateWalkAnimation` checks `getPose() == Pose.STANDING`; read from
        // `DATA_POSE` in the tick loop. `false` for entities that do not consume it.
        camel_is_standing: bool,
        // The synced `Camel.DASH` boolean (`isDashing()`), read from the entity metadata in the tick
        // loop ([`camel_is_dashing`]). `false` for entities that do not consume it.
        camel_is_dashing: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == IDLE`, read from entity metadata in the tick
        // loop. It gates the golem's delayed idle head-spin keyframe timer.
        copper_golem_is_idle: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == GETTING_ITEM`, read from entity metadata in
        // the tick loop. It gates the no-item/get chest interaction keyframe.
        copper_golem_is_getting_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == GETTING_NO_ITEM`, read from entity metadata
        // in the tick loop. It gates the no-item/noget chest interaction keyframe.
        copper_golem_is_getting_no_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == DROPPING_ITEM`, read from entity metadata in
        // the tick loop. It gates the item/drop chest interaction keyframe.
        copper_golem_is_dropping_item: bool,
        // The synced `CopperGolem.COPPER_GOLEM_STATE == DROPPING_NO_ITEM`, read from entity metadata
        // in the tick loop. It gates the item/nodrop chest interaction keyframe.
        copper_golem_is_dropping_no_item: bool,
        // The synced `Allay.DATA_DANCING` boolean (`isDancing()`), read from the entity metadata in the
        // tick loop ([`allay_is_dancing`]). `false` for entities that do not consume it.
        allay_is_dancing: bool,
        // Vanilla `Allay.hasItemInHand()`, resolved from synced main-hand equipment before the tick.
        // `false` for entities that do not consume it.
        allay_has_item_in_hand: bool,
        // The synced `Axolotl.DATA_PLAYING_DEAD` boolean (`isPlayingDead()`), read from the entity
        // metadata in the tick loop ([`axolotl_is_playing_dead`]). `false` for entities that do not
        // consume it.
        axolotl_is_playing_dead: bool,
        // The synced `Creaking.IS_TEARING_DOWN` boolean (`isTearingDown()`), read from the entity
        // metadata in the tick loop ([`creaking_is_tearing_down`]). `false` for entities that do not
        // consume it.
        creaking_is_tearing_down: bool,
        // The synced `Pillager.IS_CHARGING_CROSSBOW` boolean (`isChargingCrossbow()`), read from the
        // entity metadata in the tick loop ([`pillager_is_charging_crossbow`]). `false` for entities that
        // do not consume it.
        pillager_is_charging_crossbow: bool,
        // The synced `Piglin.DATA_IS_CHARGING_CROSSBOW` boolean (`isChargingCrossbow()`), read from the
        // entity metadata in the tick loop ([`piglin_is_charging_crossbow`]). `false` for entities that
        // do not consume it.
        piglin_is_charging_crossbow: bool,
        // The synced `LivingEntity` `isUsingItem()` bit (`DATA_LIVING_ENTITY_FLAGS & 1`), read from the
        // entity metadata in the tick loop ([`player_is_using_item`]). Drives the player's shared
        // crossbow-draw counter (the item-agnostic `getTicksUsingItem` reconstruction). `false` for
        // entities that do not consume it.
        player_is_using_item: bool,
        // The synced `Wolf.DATA_INTERESTED_ID` boolean (`isInterested()`), read from metadata in the
        // tick loop ([`wolf_is_interested`]). It drives the wolf's client-side interested-angle ease.
        wolf_is_interested: bool,
        // The synced `Cat.IS_LYING` / `RELAX_STATE_ONE` booleans, read from metadata in the tick loop.
        // They drive the cat's client-side lie-down and relaxed-head easing. `false` for non-cats.
        cat_is_lying: bool,
        cat_relax_state_one: bool,
        // The direct locally controlled root boat advances paddles later in
        // the input/control owner, after `controlBoat` and `move`, exactly as
        // Vanilla does. Remote and uncontrolled boats advance here.
        advance_boat_paddles_here: bool,
        // Vanilla `AbstractBoat.getPaddleState(side)`: synced left/right paddle
        // booleans gated by the presence of a controlling passenger.
        boat_paddle_left: bool,
        boat_paddle_right: bool,
        // Vanilla `AbstractBoat.DATA_ID_BUBBLE_TIME`, read each client tick by
        // `tickBubbleColumn()` to ease the wobble multiplier.
        boat_bubble_time: i32,
        // Vanilla `MinecartFurnace.DATA_ID_FUEL`. The server owns fuel/push;
        // the client reads only this synced bit for its presentation smoke.
        furnace_minecart_has_fuel: bool,
        // Current `getDimensions(getPose()).width()`, including size, pose,
        // and the LivingEntity SCALE attribute.
        slime_width: f32,
        // Vanilla `Entity.isSilent()`, used by Blaze, Warden, Guardian,
        // and EnderDragon client-local sound branches. Each gate precedes
        // that branch's sound random draws.
        is_silent: bool,
        // Vanilla `Entity.isSwimming()` for player cape bob suppression.
        is_swimming: bool,
        defer_local_player_walk: bool,
        content_registry: &ContentRegistry,
    ) -> EntityClientTickParticleTriggers {
        let is_non_dragon_living =
            super::content_facts::is_non_dragon_living(entity_type_id, content_registry);
        self.ensure_kind(entity_type_id);
        self.age_ticks = self.age_ticks.saturating_add(1);
        self.advance_age_lock_particle_timer_tick();
        self.spawn_body_y_rot = None;
        let age_ticks = self.age_ticks;
        let mut particle_triggers = EntityClientTickParticleTriggers::default();
        // AbstractArrow owns the ordering-sensitive shake decrement in the
        // direct movement pass: an initial containment hit precedes it, while
        // `onHitBlock` installs a fresh seven-tick shake after it.
        // Vanilla `LivingEntity.baseTick`: `if (hurtTime > 0) hurtTime--`.
        if let Some(hurt) = self.hurt.as_mut() {
            hurt.hurt_time = (hurt.hurt_time - 1).max(0);
            if hurt.hurt_time == 0 {
                self.hurt = None;
            }
        }
        // Vanilla `LivingEntity.tickDeath`: `deathTime++` each tick while dying.
        if should_tick_death {
            if let Some(death) = self.death.as_mut() {
                death.death_time = (death.death_time + 1).min(DEATH_ANIMATION_MAX_TICKS);
            }
        }
        // Vanilla `LivingEntity.updateSwimAmount`: save `swimAmountO`, then ease
        // `swimAmount` toward the current `isVisuallySwimming()` target by 0.09.
        if let Some(swim) = self.living_swim.as_mut() {
            swim.advance_client_tick();
            if swim.is_settled() {
                self.living_swim = None;
            }
        }
        // Vanilla `LivingEntity` (`oAttackAnim = attackAnim` at tick start, then
        // `updateSwingTime`). The state is dropped once the swing has fully decayed.
        if let Some(feedback) = self.kinetic_hit_feedback.as_mut() {
            feedback.elapsed_ticks = feedback.elapsed_ticks.saturating_add(1);
        }
        if let Some(swing) = self.attack_swing.as_mut() {
            let duration = swing.duration.max(1);
            swing.prev_attack_anim = swing.attack_anim;
            if swing.swinging {
                swing.swing_time += 1;
                if swing.swing_time >= duration {
                    swing.swing_time = 0;
                    swing.swinging = false;
                }
            } else {
                swing.swing_time = 0;
            }
            swing.attack_anim = swing.swing_time as f32 / duration as f32;
            if !swing.swinging && swing.attack_anim == 0.0 && swing.prev_attack_anim == 0.0 {
                self.attack_swing = None;
            }
        }
        // Vanilla `LivingEntity.tick` always advances `elytraAnimationState`, independent of
        // whether an elytra item is currently equipped.
        if is_non_dragon_living {
            self.elytra
                .get_or_insert_with(ElytraAnimationState::default)
                .advance_client_tick(is_fall_flying, is_crouching, transform.delta_movement);
        }
        // Vanilla `CROSSBOW_CHARGE` reads `getTicksUsingItem()`; the player variant is
        // item-agnostic, and the pose layer applies it only for an uncharged crossbow.
        match entity_type_id {
            VANILLA_ENTITY_TYPE_PILLAGER_ID => {
                self.advance_crossbow_charge(pillager_is_charging_crossbow)
            }
            VANILLA_ENTITY_TYPE_PIGLIN_ID => {
                self.advance_crossbow_charge(piglin_is_charging_crossbow)
            }
            VANILLA_ENTITY_TYPE_PLAYER_ID => self.advance_crossbow_charge(player_is_using_item),
            _ => {}
        }
        // Vanilla `isAlive() = getHealth() > 0`: the death state is present exactly while dying.
        let is_alive = self.death.is_none();
        // `walkAnimation.isMoving()` reads the PRIOR tick's limb swing: the walk accumulator below
        // advances after the per-kind tick, mirroring Vanilla's `tick`/`baseTick`-before-`aiStep`.
        let walk_is_moving = self
            .walk_animation
            .as_ref()
            .is_some_and(WalkAnimationState::is_moving);
        let mut boat_paddles_here = false;
        match &mut self.kind {
            EntityKindAnimation::LightningBolt(bolt) if age_ticks == 1 => {
                let random_seed = bolt.random.get_or_insert_with(|| {
                    let mut random_seed = java_random_seed(
                        i64::from(entity_id).wrapping_add(LIGHTNING_BOLT_CLIENT_RANDOM_SALT as i64),
                    );
                    java_random_next_long(&mut random_seed);
                    java_random_next_int_bound(&mut random_seed, 3);
                    random_seed
                });
                particle_triggers.lightning_bolt_sounds = Some(LightningBoltSoundParameters {
                    thunder_pitch: 0.8 + java_random_next_float(random_seed) * 0.2,
                    impact_pitch: 0.5 + java_random_next_float(random_seed) * 0.2,
                });
            }
            EntityKindAnimation::ArmorStand(wiggle) => {
                if let Some(state) = wiggle.as_mut() {
                    state.elapsed_ticks += 1;
                    if state.elapsed_ticks >= ARMOR_STAND_WOBBLE_TIME {
                        *wiggle = None;
                    }
                }
            }
            EntityKindAnimation::Minecart(minecart) => {
                tick_vehicle_damage(&mut minecart.damage);
                if entity_type_id == VANILLA_ENTITY_TYPE_TNT_MINECART_ID {
                    if let Some(fuse) = minecart.tnt_fuse.as_mut() {
                        particle_triggers.minecart_tnt_smoke = fuse.fuse_remaining_ticks > 0;
                        fuse.advance_client_tick();
                    }
                }
                if entity_type_id == VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID
                    && furnace_minecart_has_fuel
                {
                    let random_seed = minecart.furnace_random.get_or_insert_with(|| {
                        java_random_seed(i64::from(entity_id).wrapping_add(0x6675_726e))
                    });
                    particle_triggers.minecart_furnace_large_smoke =
                        java_random_next_int_bound(random_seed, 4) == 0;
                }
            }
            EntityKindAnimation::Blaze { random } => {
                let random_seed = random.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id).wrapping_add(BLAZE_CLIENT_RANDOM_SALT as i64),
                    )
                });
                if java_random_next_int_bound(random_seed, 24) == 0 && !is_silent {
                    particle_triggers.blaze_burn_sound = Some(BlazeBurnSoundParameters {
                        volume: 1.0 + java_random_next_float(random_seed),
                        pitch: java_random_next_float(random_seed) * 0.7 + 0.3,
                    });
                }
                particle_triggers.blaze_large_smoke =
                    Some(std::array::from_fn(|_| BlazeSmokeParticleOffset {
                        x_unit: java_random_next_double(random_seed) - 0.5,
                        y_unit: java_random_next_double(random_seed),
                        z_unit: java_random_next_double(random_seed) - 0.5,
                    }));
            }
            EntityKindAnimation::EnderAmbient { random, .. } => {
                let random_seed = random.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id)
                            .wrapping_add(ENDER_AMBIENT_RANDOM_SALT as i64)
                            .wrapping_add(i64::from(entity_type_id)),
                    )
                });
                particle_triggers.ender_portal =
                    Some(std::array::from_fn(|_| EnderAmbientParticleOffset {
                        x_unit: java_random_next_double(random_seed) - 0.5,
                        y_unit: java_random_next_double(random_seed),
                        z_unit: java_random_next_double(random_seed) - 0.5,
                        velocity: EntityVec3 {
                            x: (java_random_next_double(random_seed) - 0.5) * 2.0,
                            y: -java_random_next_double(random_seed),
                            z: (java_random_next_double(random_seed) - 0.5) * 2.0,
                        },
                    }));
            }
            EntityKindAnimation::Guardian(guardian) => {
                guardian.advance_client_tick(
                    entity_type_id,
                    entity_id,
                    transform,
                    in_water,
                    is_moving,
                    guardian_can_stand_on_below,
                    guardian_attack_target_id,
                    guardian_attack_target,
                    guardian_eye_height,
                    is_silent,
                    &mut particle_triggers,
                );
            }
            EntityKindAnimation::EnderDragon(dragon) => {
                if dragon.state.is_some_and(|state| state.dragon_dying) {
                    dragon
                        .state
                        .get_or_insert_with(EnderDragonAnimationState::default)
                        .advance_dragon_death_tick(ENDER_DRAGON_DEATH_ANIMATION_MAX_TICKS);
                } else if let Some(state) = dragon.state.as_mut() {
                    state.clear_dragon_death();
                }
                let phase_id = dragon.state.map_or(10, |state| state.phase_id);
                let phase_id_i8 = phase_id.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8;
                if phase_id_i8 != dragon.last_phase_id {
                    dragon.last_phase_id = phase_id_i8;
                    dragon.flame_ticks = 0;
                    dragon.death_phase_ticks = 0;
                }
                if !dragon.random_initialized {
                    dragon.random_seed = java_random_seed(
                        i64::from(entity_id).wrapping_add(ENDER_DRAGON_CLIENT_RANDOM_SALT as i64),
                    );
                    dragon.random_initialized = true;
                }
                let random_seed = &mut dragon.random_seed;
                particle_triggers.ender_dragon_sounds = dragon
                    .state
                    .get_or_insert_with(EnderDragonAnimationState::default)
                    .advance_client_tick(
                        transform,
                        is_silent,
                        random_seed,
                        java_random_next_float,
                        java_random_next_int_bound,
                    );
                if !dragon
                    .state
                    .is_some_and(|state| state.dragon_dying || state.no_ai)
                {
                    // Vanilla `DragonSittingFlamingPhase.doClientTick` (`phase_id == 5`) and
                    // `DragonDeathPhase.doClientTick` (`phase_id == 9`).
                    if phase_id == 5 {
                        dragon.flame_ticks = dragon.flame_ticks.saturating_add(1);
                    } else if phase_id == 9 {
                        dragon.death_phase_ticks = dragon.death_phase_ticks.saturating_add(1);
                    }
                }
            }
            EntityKindAnimation::Creeper(swell) => {
                if let Some(state) = swell.as_mut() {
                    state.advance_client_tick();
                    if state.is_settled() {
                        *swell = None;
                    }
                }
            }
            EntityKindAnimation::PolarBear(standing) => {
                if let Some(standing) = standing.as_mut() {
                    standing.advance_client_tick();
                }
            }
            EntityKindAnimation::Sheep(eat) => {
                if let Some(state) = eat.as_mut() {
                    state.advance_client_tick();
                    if state.eat_animation_tick == 0 {
                        *eat = None;
                    }
                }
            }
            EntityKindAnimation::Goat(ramming) => {
                if let Some(state) = ramming.as_mut() {
                    state.advance_client_tick();
                    if state.is_settled() {
                        *ramming = None;
                    }
                }
            }
            EntityKindAnimation::IronGolem(golem) => {
                if let Some(state) = golem.as_mut() {
                    state.advance_client_tick();
                    if state.is_settled() {
                        *golem = None;
                    }
                }
            }
            EntityKindAnimation::Ravager(ravager) => {
                if let Some(state) = ravager.as_mut() {
                    if let Some(offset) = state.advance_client_tick(entity_id) {
                        particle_triggers.ravager_stun = Some(offset);
                    }
                    if state.is_settled() {
                        *ravager = None;
                    }
                }
            }
            EntityKindAnimation::Hoglin(hoglin) => {
                if let Some(state) = hoglin.as_mut() {
                    state.advance_client_tick();
                    if state.is_settled() {
                        *hoglin = None;
                    }
                }
            }
            EntityKindAnimation::Rabbit(rabbit_hop) => {
                if let Some(state) = rabbit_hop.as_mut() {
                    state.advance_client_tick(age_ticks);
                    if state.is_settled() {
                        *rabbit_hop = None;
                    }
                }
            }
            EntityKindAnimation::Creaking(creaking) => {
                // The death one-shot is driven by the synced `isTearingDown()` directly, so a
                // creaking that begins tearing down spins up the state even without a prior
                // attack/invulnerable event; otherwise an idle creaking carries no state.
                if creaking.is_some() || creaking_is_tearing_down {
                    let state =
                        creaking.get_or_insert_with(|| CreakingAnimationState::new(entity_id));
                    state.advance_client_tick(entity_id, age_ticks, creaking_is_tearing_down);
                    if state.is_settled() {
                        *creaking = None;
                    }
                }
            }
            EntityKindAnimation::Bat(flying) => flying
                .get_or_insert_with(KeyframeAnimationState::default)
                .animate_when(!bat_is_resting, age_ticks),
            EntityKindAnimation::Shulker(shulker) => {
                if let Some(peek) = shulker.peek.as_mut() {
                    peek.advance_client_tick();
                }
                if let Some(teleport) = shulker.teleport.as_mut() {
                    if teleport.interpolation_ticks > 0 {
                        teleport.interpolation_ticks -= 1;
                        if teleport.interpolation_ticks == 0 {
                            teleport.old_attach_position = None;
                        }
                    }
                }
            }
            EntityKindAnimation::Bee(roll) => {
                if let Some(roll) = roll.as_mut() {
                    roll.advance_client_tick();
                }
            }
            EntityKindAnimation::Panda(panda) => {
                if let Some(state) = panda.as_mut() {
                    state.advance_client_tick();
                    if state.is_settled() {
                        *panda = None;
                    }
                }
            }
            EntityKindAnimation::Armadillo(armadillo) => {
                if let Some(armadillo) = armadillo.as_mut() {
                    armadillo.setup_animation_states(age_ticks);
                }
            }
            EntityKindAnimation::Fox(fox) => {
                if let Some(fox) = fox.as_mut() {
                    fox.advance_client_tick();
                }
            }
            EntityKindAnimation::Cat(cat) => {
                if cat.is_some() || cat_is_lying || cat_relax_state_one {
                    let state = cat.get_or_insert_with(CatLieDownAnimationState::default);
                    state.advance_client_tick(cat_is_lying, cat_relax_state_one);
                    if state.is_settled() {
                        *cat = None;
                    }
                }
            }
            EntityKindAnimation::Wolf(wolf) => {
                if wolf.head_roll.is_some() || wolf_is_interested {
                    let head_roll = wolf.head_roll.get_or_insert(WolfHeadRollAnimationState {
                        interested: false,
                        previous_interested_angle: 0.0,
                        current_interested_angle: 0.0,
                    });
                    head_roll.advance_client_tick(wolf_is_interested);
                    if head_roll.is_settled() {
                        wolf.head_roll = None;
                    }
                }
                if wolf.wet.is_some() || in_water {
                    let wet = wolf.wet.get_or_insert(WolfWetAnimationState {
                        is_wet: false,
                        is_shaking: false,
                        previous_shake_anim: 0.0,
                        current_shake_anim: 0.0,
                    });
                    wet.advance_client_tick(in_water);
                    if wet.is_settled() {
                        wolf.wet = None;
                    }
                }
            }
            EntityKindAnimation::Warden(warden) => {
                if let Some(tendril) = warden.tendril.as_mut() {
                    tendril.advance_client_tick();
                    if tendril.is_settled() {
                        warden.tendril = None;
                    }
                }
                // The heart beats forever (vanilla resets it every `getHeartBeatDelay()` ticks).
                let heartbeat = warden
                    .heart
                    .get_or_insert_with(WardenHeartAnimationState::default)
                    .advance_client_tick(age_ticks, warden_heartbeat_delay);
                if heartbeat && !is_silent {
                    let random_seed = warden.random.get_or_insert_with(|| {
                        java_random_seed(
                            i64::from(entity_id).wrapping_add(WARDEN_CLIENT_RANDOM_SALT as i64),
                        )
                    });
                    particle_triggers.warden_heartbeat_pitch = Some(
                        (java_random_next_float(random_seed) - java_random_next_float(random_seed))
                            * 0.2
                            + 1.0,
                    );
                }
            }
            EntityKindAnimation::Wither(heads) => {
                let targets = wither_head_targets.unwrap_or_else(|| {
                    [
                        WitherHeadTargetRotations::fallback_to_body(transform.y_rot),
                        WitherHeadTargetRotations::fallback_to_body(transform.y_rot),
                    ]
                });
                heads
                    .get_or_insert_with(WitherHeadRotationsState::default)
                    .advance_client_tick(targets);
            }
            EntityKindAnimation::Squid(squid) => squid
                .get_or_insert_with(|| {
                    SquidAnimationState::new_with_y_body_rot(entity_id, transform.y_head_rot)
                })
                .advance_client_tick(transform.delta_movement, in_water),
            // Llamas share the `AbstractHorse` random but not the horse tail/pose animation.
            EntityKindAnimation::Equine(equine)
                if vanilla_entity_renderer_uses_abstract_horse_renderer(entity_type_id) =>
            {
                equine.advance_client_tick(entity_id)
            }
            EntityKindAnimation::Frog(frog) => {
                // Vanilla `Frog.tick` (client side): `swimIdleAnimationState.animateWhen(isInWater()
                // && !walkAnimation.isMoving(), tickCount)`.
                frog.swim_idle
                    .get_or_insert(KeyframeAnimationState { start_age: None })
                    .animate_when(in_water && !walk_is_moving, age_ticks);
            }
            EntityKindAnimation::Camel(camel) => {
                camel
                    .idle
                    .get_or_insert_with(|| CamelIdleAnimationState::new(entity_id))
                    .advance_client_tick(age_ticks);
                // Vanilla `Camel.setupAnimationStates`: `dashAnimationState.animateWhen(isDashing(),
                // tickCount)` starts the looping gallop on the synced `DASH` rising edge.
                camel
                    .dash
                    .get_or_insert(KeyframeAnimationState { start_age: None })
                    .animate_when(camel_is_dashing, age_ticks);
                let cooldown = camel.dash_cooldown.get_or_insert_with(Default::default);
                cooldown.advance_client_tick(camel_is_dashing);
                if cooldown.is_settled() {
                    camel.dash_cooldown = None;
                }
            }
            EntityKindAnimation::CopperGolem(golem) => {
                let spun = golem
                    .idle
                    .get_or_insert_with(|| CopperGolemIdleAnimationState::new(entity_id))
                    .advance_client_tick(age_ticks, copper_golem_is_idle);
                if spun && !is_silent {
                    particle_triggers.copper_golem_spin_sound = true;
                }
                for (state, active) in [
                    (&mut golem.get_item, copper_golem_is_getting_item),
                    (&mut golem.get_no_item, copper_golem_is_getting_no_item),
                    (&mut golem.drop_item, copper_golem_is_dropping_item),
                    (&mut golem.drop_no_item, copper_golem_is_dropping_no_item),
                ] {
                    state
                        .get_or_insert_with(KeyframeAnimationState::default)
                        .animate_when(active, age_ticks);
                }
            }
            EntityKindAnimation::Allay(allay) => {
                // Vanilla `Allay.tick`: while the synced `DATA_DANCING` flag is set, advance the dance
                // and spin counters; otherwise reset them.
                allay
                    .dance
                    .get_or_insert_with(AllayDanceAnimationState::default)
                    .advance_client_tick(allay_is_dancing);
                let holding_item = allay
                    .holding_item
                    .get_or_insert_with(AllayHoldingItemAnimationState::default);
                holding_item.advance_client_tick(allay_has_item_in_hand);
                if holding_item.is_settled() {
                    allay.holding_item = None;
                }
            }
            EntityKindAnimation::Player(cloak) => {
                cloak
                    .get_or_insert_with(PlayerCloakAnimationState::default)
                    .advance_client_tick(
                        transform.position,
                        transform.delta_movement,
                        transform.on_ground.unwrap_or(false),
                        is_alive,
                        is_swimming,
                        is_fall_flying,
                        !defer_local_player_walk,
                    );
            }
            EntityKindAnimation::Axolotl(axolotl) => {
                // Vanilla `Axolotl.baseTick` selects `tickBabyAnimations` or `tickAdultAnimations`;
                // both paths update the moving animator (prior-tick limb swing OR a rotation change).
                axolotl
                    .get_or_insert_with(AxolotlAnimationState::default)
                    .advance_client_tick(
                        is_baby,
                        age_ticks,
                        axolotl_is_playing_dead,
                        in_water,
                        transform.on_ground.unwrap_or(false),
                        walk_is_moving,
                        transform.x_rot,
                        transform.y_rot,
                    );
            }
            EntityKindAnimation::Chicken(flap) => flap
                .get_or_insert_with(ChickenFlapAnimationState::default)
                // Vanilla `Chicken.aiStep` reads `onGround()`; no synced flag defaults to on-ground.
                .advance_client_tick(transform.on_ground.unwrap_or(true)),
            EntityKindAnimation::Slime(slime) => {
                let random_seed = slime.random.get_or_insert_with(|| {
                    java_random_seed(
                        i64::from(entity_id).wrapping_add(SLIME_CLIENT_RANDOM_SALT as i64),
                    )
                });
                slime
                    .state
                    .get_or_insert_with(SlimeAnimationState::default)
                    // Vanilla's entity field initializes `onGround` to false; the first movement
                    // packet replaces this fallback.
                    .advance_client_tick(
                        transform.on_ground.unwrap_or(false),
                        entity_type_id == VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
                        slime_width,
                        random_seed,
                        &mut particle_triggers,
                    );
            }
            EntityKindAnimation::EvokerFangs(fangs) => {
                // Vanilla `EvokerFangs.tick` counts down `lifeTicks` once the attack has started.
                if let Some(fangs) = fangs.as_mut() {
                    if let Some(offsets) = fangs.advance_client_tick(entity_id) {
                        particle_triggers.evoker_fangs_crit = Some(offsets);
                    }
                }
            }
            EntityKindAnimation::Parrot(flap) => flap
                .get_or_insert_with(ParrotFlapAnimationState::default)
                // Vanilla `Parrot.calculateFlapping` reads `!onGround() && !isPassenger()`.
                .advance_client_tick(transform.on_ground.unwrap_or(true), is_passenger),
            EntityKindAnimation::Boat(boat) => {
                tick_vehicle_damage(&mut boat.damage);
                if boat.bubble.is_some() || boat_bubble_time > 0 {
                    let bubble = boat
                        .bubble
                        .get_or_insert_with(BoatBubbleAnimationState::default);
                    bubble.advance_client_tick(age_ticks, boat_bubble_time);
                    if bubble.is_settled(boat_bubble_time) {
                        boat.bubble = None;
                    }
                }
                boat_paddles_here = advance_boat_paddles_here;
            }
            _ => {}
        }
        if boat_paddles_here {
            self.advance_boat_paddles(boat_paddle_left, boat_paddle_right);
        }
        // Vanilla `LivingEntity.calculateEntityAnimation` runs every client tick for every living
        // entity, feeding the per-tick travel to the limb-swing accumulator.
        if is_non_dragon_living && !defer_local_player_walk {
            let use_y = is_flying_animal(entity_type_id);
            let frog_jump_started = self
                .frog_kind()
                .and_then(|frog| frog.jump)
                .is_some_and(|jump| jump.start_age.is_some());
            let camel_walk_active = camel_is_standing && !camel_is_dashing;
            self.walk_animation
                .get_or_insert_with(WalkAnimationState::default)
                .advance_client_tick(
                    entity_type_id,
                    transform.position,
                    use_y,
                    is_passenger,
                    is_alive,
                    is_baby,
                    frog_jump_started,
                    camel_walk_active,
                );
        }
        particle_triggers
    }

    pub(crate) fn advance_boat_paddles(&mut self, left_active: bool, right_active: bool) {
        let Some(boat) = self.boat_kind_mut() else {
            return;
        };
        if boat.paddles.is_none() && !left_active && !right_active {
            return;
        }
        let paddles = boat
            .paddles
            .get_or_insert_with(BoatPaddleAnimationState::default);
        paddles.advance_client_tick(left_active, right_active);
        if paddles.is_settled() {
            boat.paddles = None;
        }
    }
}
