//! `LightningBolt`: a short-lived strike `Entity` that sets fire, powers a
//! lightning rod, strikes what stands under it and then removes itself.

use crate::block_behavior::weathering;
use crate::entity::conversion::SingleMobConversionSnapshot;
use crate::entity::item::plain_stack;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};

use crate::entity::level::EntityBlockContext;
use crate::entity::living::SynchedValue;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{base_tick, Entity, EntityCommand, EntityData, GeneralLivingData};
use crate::level_event::{PendingPlayerHurt, VanillaGameEvent};
use bbb_server_types::geometry::Aabb;

/// `LightningBolt.START_LIFE` (`LightningBolt.java:32`).
const START_LIFE: i32 = 2;

/// `LightningBolt.DAMAGE_RADIUS` (`LightningBolt.java:33`): the horizontal and
/// lower reach of the strike box; the box also extends six blocks further up.
const DAMAGE_RADIUS: f64 = 3.0;

/// The extra upward reach of the strike box (`LightningBolt.java:146`,
/// `getY() + 6.0 + 3.0`).
const STRIKE_BOX_TOP_EXTRA: f64 = 6.0;

/// `Entity.thunderHit`'s damage amount (`Entity.java:2768`).
const THUNDER_DAMAGE: f32 = 5.0;

/// `Entity.getDefaultGravity()` (`Entity.java:1432-1434`): the bolt never moves,
/// so gravity is never applied either way.
pub const DEFAULT_GRAVITY: f64 = 0.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightningBoltData {
    life: i32,
    seed: i64,
    flashes: i32,
    visual_only: bool,
    /// `ServerPlayer cause` (`:39`) reduced to its UUID.
    cause: Option<uuid::Uuid>,
    /// `hitEntities` (`:40`) by entity id.
    hit_entities: Vec<i32>,
    blocks_set_on_fire: i32,
}

impl LightningBoltData {
    pub(crate) fn create(random: &crate::random::GameRandom) -> Self {
        Self::new(random.next_long(), random.next_int(3) + 1)
    }

    /// `LightningBolt(EntityType, Level)` (`LightningBolt.java:43-48`): the
    /// constructor draws `seed` then `flashes` off the entity's random after
    /// the base constructor has drawn its UUID.
    pub const fn new(seed: i64, flashes: i32) -> Self {
        Self {
            life: START_LIFE,
            seed,
            flashes,
            visual_only: false,
            cause: None,
            hit_entities: Vec::new(),
            blocks_set_on_fire: 0,
        }
    }

    pub fn set_visual_only(&mut self, visual_only: bool) {
        self.visual_only = visual_only;
    }

    pub fn set_cause(&mut self, cause: Option<uuid::Uuid>) {
        self.cause = cause;
    }

    pub const fn life(&self) -> i32 {
        self.life
    }

    pub const fn seed(&self) -> i64 {
        self.seed
    }

    pub const fn flashes(&self) -> i32 {
        self.flashes
    }

    pub const fn visual_only(&self) -> bool {
        self.visual_only
    }

    pub const fn cause(&self) -> Option<uuid::Uuid> {
        self.cause
    }

    pub fn hit_entities(&self) -> &[i32] {
        &self.hit_entities
    }

    pub const fn blocks_set_on_fire(&self) -> i32 {
        self.blocks_set_on_fire
    }
}

impl Entity {
    pub(crate) fn set_lightning_cause(&mut self, cause: Option<uuid::Uuid>) {
        if let EntityData::LightningBolt(data) = &mut self.data {
            data.set_cause(cause);
        }
    }
}

///
/// The two constructor draws are taken off the store's shared random in
/// vanilla's order -- `seed = nextLong()` then `flashes = nextInt(3) + 1` --
/// continuing the stream after the UUID the base constructor drew.
pub fn spawn_lightning_bolt(
    store: &mut EntityStore,
    position: Vec3d,
    visual_only: bool,
    cause: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let mut data = LightningBoltData::create(store.random());
    data.set_visual_only(visual_only);
    data.set_cause(cause);
    let id = store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
        position,
        EntityData::LightningBolt(data),
    )?;
    let entity = store.get_mut(id).expect("restore leaves the entity live");
    entity.set_needs_sync(true);
    Some(id)
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    // `:77`.
    if !base_tick(entity, store, level) {
        return false;
    }
    let data = bolt_data(entity);

    // `:78-112`; the client sound branch (`:79-101`) is the client's.
    if data.life == START_LIFE {
        if matches!(
            store.difficulty(),
            Some(Difficulty::Normal | Difficulty::Hard)
        ) {
            spawn_fire(entity, store, level, 4);
        }
        power_lightning_rod(entity, store, level);
        clear_copper_on_lightning_strike(entity, store, level);
        level.game_event_at(
            VanillaGameEvent::LIGHTNING_STRIKE,
            entity.position(),
            Some(entity.id()),
        );
    }

    // `:114`.
    let data = bolt_data_mut(entity);
    data.life -= 1;
    if data.life < 0 {
        if data.flashes == 0 {
            queue_lightning_strike_trigger(entity, store, level);
            entity.discard();
        } else if data.life < -store.random().next_int(10) {
            data.flashes -= 1;
            data.life = 1;
            data.seed = store.random().next_long();
            if matches!(
                store.difficulty(),
                Some(Difficulty::Normal | Difficulty::Hard)
            ) {
                spawn_fire(entity, store, level, 0);
            }
        }
    }

    // `:139-159`.
    let data = bolt_data(entity);
    if data.life >= 0 && !data.visual_only {
        strike_entities(entity, store, level);
    }

    true
}

fn bolt_data(entity: &Entity) -> &LightningBoltData {
    let EntityData::LightningBolt(data) = &entity.data else {
        unreachable!("lightning bolt state");
    };
    data
}

fn bolt_data_mut(entity: &mut Entity) -> &mut LightningBoltData {
    let EntityData::LightningBolt(data) = &mut entity.data else {
        unreachable!("lightning bolt state");
    };
    data
}

/// Work a strike hands to the world: a conversion that needs
/// `finalizeSpawn`, a session's fire, and the two advancement triggers.
#[derive(Debug, Clone, PartialEq)]
pub enum LightningAction {
    /// `Villager.thunderHit` (`Villager.java:756-770`).
    VillagerToWitch { villager_id: i32 },
    /// `Entity.thunderHit`'s hurt (`Entity.java:2768`) on a vehicle or end
    /// crystal.
    Hurt {
        entity_id: i32,
        amount: f32,
        registry_tags: crate::entity::EntityRegistryTagView,
    },
    /// `Entity.thunderHit`'s fire on a session (`Entity.java:2763-2766`).
    IgnitePlayer { player: uuid::Uuid },
    /// `CriteriaTriggers.CHANNELED_LIGHTNING` (`LightningBolt.java:155-157`).
    Channeled {
        cause: uuid::Uuid,
        victims: Vec<StrikeWitness>,
    },
    /// `CriteriaTriggers.LIGHTNING_STRIKE` (`LightningBolt.java:117-128`):
    /// the bolt as it fades and the living-or-not bystanders it never hit.
    Faded {
        bolt: Box<Entity>,
        bystanders: Vec<StrikeWitness>,
    },
}

/// An entity a trigger reads after the strike, including a converted source.
#[derive(Debug, Clone, PartialEq)]
pub enum StrikeWitness {
    Stored(Box<Entity>),
    Player { entity_id: i32 },
}

/// `LightningBolt.DETECTION_RADIUS` (`LightningBolt.java:34`).
const DETECTION_RADIUS: f64 = 15.0;

fn queue_lightning_strike_trigger(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    let data = bolt_data(entity);
    let position = entity.position();
    let box_ = Aabb {
        min_x: position.x - DETECTION_RADIUS,
        min_y: position.y - DETECTION_RADIUS,
        min_z: position.z - DETECTION_RADIUS,
        max_x: position.x + DETECTION_RADIUS,
        max_y: position.y + STRIKE_BOX_TOP_EXTRA + DETECTION_RADIUS,
        max_z: position.z + DETECTION_RADIUS,
    };
    let mut bystanders: Vec<StrikeWitness> = store
        .ids_in_box_sections(box_)
        .into_iter()
        .filter(|id| !data.hit_entities.contains(id))
        .filter_map(|id| store.get(id))
        .filter(|target| target.is_alive_with_health() && target.bounding_box().intersects(box_))
        .map(|target| StrikeWitness::Stored(Box::new(target.clone())))
        .collect();
    bystanders.extend(
        level
            .players_in_box(box_)
            .filter(|player| {
                !player.dead_or_dying && !data.hit_entities.contains(&player.entity_id)
            })
            .map(|player| StrikeWitness::Player {
                entity_id: player.entity_id,
            }),
    );
    store.queue_command(EntityCommand::Lightning(LightningAction::Faded {
        bolt: Box::new(entity.clone()),
        bystanders,
    }));
}

fn strike_entities(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let position = entity.position();
    let data = bolt_data_mut(entity);
    let box_ = Aabb {
        min_x: position.x - DAMAGE_RADIUS,
        min_y: position.y - DAMAGE_RADIUS,
        min_z: position.z - DAMAGE_RADIUS,
        max_x: position.x + DAMAGE_RADIUS,
        max_y: position.y + STRIKE_BOX_TOP_EXTRA + DAMAGE_RADIUS,
        max_z: position.z + DAMAGE_RADIUS,
    };

    let stored_targets: Vec<i32> = store
        .ids_in_box_sections(box_)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|target| {
                target.is_alive_with_health() && target.bounding_box().intersects(box_)
            })
        })
        .collect();
    let players: Vec<_> = level
        .players_in_box(box_)
        .filter(|player| !player.dead_or_dying)
        .collect();
    let mut victims = Vec::new();
    for target_id in stored_targets {
        let first_hit = !data.hit_entities.contains(&target_id);
        if let Some(mut target) = store.detach(target_id) {
            thunder_hit(&mut target, store, level, first_hit);
            if data.cause.is_some() {
                // LightningBolt.java:150-157: predicates read the struck source after thunderHit.
                victims.push(StrikeWitness::Stored(target.clone()));
            }
            store.attach(target);
        }
        if first_hit {
            data.hit_entities.push(target_id);
        }
    }
    for player in players {
        store.queue_command(EntityCommand::Lightning(LightningAction::IgnitePlayer {
            player: player.uuid,
        }));
        level.hurt_player(PendingPlayerHurt {
            player: player.uuid,
            damage_type: LIGHTNING_BOLT_DAMAGE,
            amount: THUNDER_DAMAGE,
            causing: None,
            direct: None,
            post_attack: false,
            impulse: None,
        });
        victims.push(StrikeWitness::Player {
            entity_id: player.entity_id,
        });
        if !data.hit_entities.contains(&player.entity_id) {
            data.hit_entities.push(player.entity_id);
        }
    }
    if let Some(cause) = data.cause {
        store.queue_command(EntityCommand::Lightning(LightningAction::Channeled {
            cause,
            victims,
        }));
    }
}

const LIGHTNING_BOLT_DAMAGE: &str = "minecraft:lightning_bolt";

/// `Entity.thunderHit` (`Entity.java:2762-2769`) and its overrides on a stored
/// body. `first_hit` stands in for the mooshroom's `lastLightningBoltUUID`.
fn thunder_hit(
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    first_hit: bool,
) {
    let peaceful = store.difficulty() == Some(Difficulty::Peaceful);
    match target.data() {
        // `ArmorStand.java:602-603`, `BlockAttachedEntity.java:152-153`.
        EntityData::ArmorStand(_)
        | EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_) => {}
        EntityData::GeneralLiving(_) => match target.entity_type_id() {
            // `Creeper.java:204-207`.
            VANILLA_ENTITY_TYPE_CREEPER_ID => {
                burn_and_hurt(target, store, level);
                crate::entity::ai::mob::creeper::set_powered(target);
            }
            // `Pig.java:192-204`.
            VANILLA_ENTITY_TYPE_PIG_ID => {
                if peaceful || !zombify_pig(target, store) {
                    burn_and_hurt(target, store, level);
                }
            }
            // `Villager.java:756-770`.
            VANILLA_ENTITY_TYPE_VILLAGER_ID if !peaceful => {
                store.queue_command(EntityCommand::Lightning(LightningAction::VillagerToWitch {
                    villager_id: target.id(),
                }));
            }
            // `MushroomCow.java:76-83`.
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID => {
                if first_hit {
                    if let Some(variant) = crate::entity::mooshroom::variant_of(target) {
                        crate::entity::mooshroom::set_variant(
                            target,
                            crate::entity::mooshroom::other_variant(variant),
                        );
                        if !target.is_silent() {
                            store.play_entity_sound(PendingEntitySound {
                                position: target.position(),
                                resource_id: "minecraft:entity.mooshroom.convert".to_string(),
                                source: SoundSource::Neutral,
                                volume: 2.0,
                                pitch: 1.0,
                            });
                        }
                    }
                }
            }
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
                burn_and_hurt(target, store, level);
                if first_hit {
                    crate::entity::copper_golem::thunder_hit(target);
                }
            }
            // `Turtle.java:278-280`.
            VANILLA_ENTITY_TYPE_TURTLE_ID => hurt(target, store, level, f32::MAX),
            _ => burn_and_hurt(target, store, level),
        },
        _ => burn_and_hurt(target, store, level),
    }
}

fn burn_and_hurt(target: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    target.set_remaining_fire_ticks(target.remaining_fire_ticks() + 1);
    if target.remaining_fire_ticks() == 0 {
        crate::entity::living::ignite_living_for_seconds(target, 8.0);
    }
    hurt(target, store, level, THUNDER_DAMAGE);
}

fn hurt(target: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, amount: f32) {
    if matches!(target.data(), EntityData::GeneralLiving(_)) {
        let _ = crate::entity::general_living::hurt_from_environment_outcome(
            target,
            store,
            level,
            LIGHTNING_BOLT_DAMAGE,
            amount,
        );
    } else if crate::entity::damage::supports_detached_environment_hurt(target.data()) {
        let _ = crate::entity::damage::hurt_detached_from_environment(
            target,
            store,
            level,
            LIGHTNING_BOLT_DAMAGE,
            amount,
        );
    } else {
        // Vehicles and end crystals are hurt by id once the strike files them back.
        store.queue_command(EntityCommand::Lightning(LightningAction::Hurt {
            entity_id: target.id(),
            amount,
            registry_tags: level.capture_registry_tags(),
        }));
    }
}

/// `convertTo(ZOMBIFIED_PIGLIN, single(this, false, true), ...)` with
/// `populateDefaultEquipmentSlots` (`ZombifiedPiglin.java:224-226`) and
/// `setPersistenceRequired`.
fn zombify_pig(source: &mut Entity, store: &mut EntityStore) -> bool {
    if source.removal_reason().is_some() {
        return false;
    }
    let Some(data) = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID) else {
        return false;
    };
    let snapshot = SingleMobConversionSnapshot::capture_with(source, false, true);
    let weapon = if store.random().next_int(20) == 0 {
        "minecraft:golden_spear"
    } else {
        "minecraft:golden_sword"
    };
    let uuid = store.next_uuid();
    let Some(converted_id) = store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        snapshot.position(),
        EntityData::GeneralLiving(data),
        |target| {
            snapshot.apply(target);
            if let Some(living) = target.living_mut() {
                living.set_item_slot(EquipmentSlot::MainHand, plain_stack(weapon, 1));
            }
            crate::entity::conversion::set_persistence_required(target);
        },
    ) else {
        return false;
    };
    crate::entity::conversion::transfer_single_relations_from(source, store, converted_id);
    source.discard();
    true
}

/// `LightningBolt.spawnFire` (`LightningBolt.java:167-187`).
fn spawn_fire(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    additional_sources: i32,
) {
    if bolt_data(entity).visual_only {
        return;
    }
    let pos = entity.block_position();
    if !level.can_spread_fire_around(pos) {
        return;
    }
    place_fire(entity, store, level, pos);
    for _ in 0..additional_sources {
        let nearby = BlockPos {
            x: pos.x + store.random().next_int(3) - 1,
            y: pos.y + store.random().next_int(3) - 1,
            z: pos.z + store.random().next_int(3) - 1,
        };
        place_fire(entity, store, level, nearby);
    }
}

fn place_fire(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) {
    let Some(fire) = crate::block_behavior::fire::surviving_fire_state(level, pos) else {
        return;
    };
    level.set_entity_block(
        EntityBlockContext::new(Some(store), Some(entity)),
        pos,
        fire,
        SET_BLOCK_UPDATE_ALL,
    );
    bolt_data_mut(entity).blocks_set_on_fire += 1;
}

/// `LightningBolt.powerLightningRod` (`LightningBolt.java:67-73`).
fn power_lightning_rod(entity: &Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let pos = strike_position(entity);
    let state = level.block_state(pos);
    if !crate::block_behavior::lightning_rod::is_lightning_rod(state) {
        return;
    }
    let mut level = level.with_entity_context(EntityBlockContext::new(Some(store), Some(entity)));
    crate::block_behavior::lightning_rod::on_lightning_strike(&mut level, pos, state);
}

/// `LevelEvent.PARTICLES_ELECTRIC_SPARK` (`LevelEvent.java:67`), sent per
/// cleaned block.
const LEVEL_EVENT_PARTICLES_ELECTRIC_SPARK: i32 = 3002;

fn strike_position(entity: &Entity) -> BlockPos {
    let position = entity.position();
    BlockPos {
        x: position.x.floor() as i32,
        y: (position.y - 1.0e-6).floor() as i32,
        z: position.z.floor() as i32,
    }
}

///
/// Vanilla draws from the level random; the store's random stands in, since
/// only the cleaned result is player-visible.
fn clear_copper_on_lightning_strike(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let struck = strike_position(entity);
    let struck_state = level.block_state(struck);
    let weathering = weathering::age(struck_state).is_some();
    if !weathering && !weathering::is_waxed(struck_state) {
        return;
    }
    if weathering {
        let first = weathering::first_state(struck_state);
        level.set_entity_block(
            EntityBlockContext::new(Some(&mut *store), Some(entity)),
            struck,
            first,
            SET_BLOCK_UPDATE_ALL,
        );
    }
    let strikes = store.random().next_int(3) + 3;
    for _ in 0..strikes {
        let steps = store.random().next_int(8) + 1;
        let mut work = struck;
        for _ in 0..steps {
            match random_step_cleaning_copper(entity, store, level, work) {
                Some(next) => work = next,
                None => break,
            }
        }
    }
}

fn random_step_cleaning_copper(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<BlockPos> {
    for _ in 0..10 {
        let candidate = BlockPos {
            x: pos.x + store.random().next_int(3) - 1,
            y: pos.y + store.random().next_int(3) - 1,
            z: pos.z + store.random().next_int(3) - 1,
        };
        let state = level.block_state(candidate);
        if weathering::age(state).is_none() {
            continue;
        }
        if let Some(previous) = weathering::previous_state(state) {
            level.set_entity_block(
                EntityBlockContext::new(Some(&mut *store), Some(entity)),
                candidate,
                previous,
                SET_BLOCK_UPDATE_ALL,
            );
        }
        level.level_event(LEVEL_EVENT_PARTICLES_ELECTRIC_SPARK, candidate, -1);
        return Some(candidate);
    }
    None
}

pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    crate::entity::living::entity_synched_values(entity)
}
