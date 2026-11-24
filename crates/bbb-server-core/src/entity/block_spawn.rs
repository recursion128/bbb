use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
    VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_protocol::NbtSummaryValue;

use crate::entity::{Entity, EntityData, EntityStore, GeneralLivingData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct BlockOriginSpawnResult {
    entity_id: i32,
}

impl BlockOriginSpawnResult {
    pub const fn entity_id(self) -> i32 {
        self.entity_id
    }
}

/// A block-pattern summon, created after its pattern matched
/// (`CarvedPumpkinBlock.java:65-117`, `WitherSkullBlock.java:56-77`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlockPatternMob {
    SnowGolem,
    IronGolem,
    /// `WeatheringCopper.WeatherState` serialized name.
    CopperGolem {
        weather_state: &'static str,
    },
    Wither {
        y_rot: f32,
    },
}

impl EntityStore {
    /// `EntityType.create(level, TRIGGERED)`, the summon's own setup and
    /// `addFreshEntity`, with the entity snapped to `spawn_pos`'s centre.
    pub fn spawn_block_pattern_mob(
        &mut self,
        mob: BlockPatternMob,
        spawn_pos: BlockPos,
    ) -> Option<i32> {
        let (entity_type_id, y_offset, y_rot) = match mob {
            BlockPatternMob::SnowGolem => (VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, 0.05, 0.0),
            BlockPatternMob::IronGolem => (VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, 0.05, 0.0),
            BlockPatternMob::CopperGolem { .. } => (VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, 0.05, 0.0),
            BlockPatternMob::Wither { y_rot } => (VANILLA_ENTITY_TYPE_WITHER_ID, 0.55, y_rot),
        };
        let mut data = GeneralLivingData::new(entity_type_id)?;
        match mob {
            // `IronGolem.setPlayerCreated(true)` (`CarvedPumpkinBlock.java:79`).
            BlockPatternMob::IronGolem => {
                let flag = data
                    .opaque_fields_mut()
                    .iter_mut()
                    .find(|entry| entry.name == "PlayerCreated")?;
                flag.value = NbtSummaryValue::Byte(1);
                data.set_iron_golem_player_created(true);
            }
            // `CopperGolem.spawn` sets the pattern's weather state
            // (`CopperGolem.java:363-366`).
            BlockPatternMob::CopperGolem { weather_state } => {
                if !crate::entity::copper_golem::set_weather_state(&mut data, weather_state) {
                    return None;
                }
            }
            // `WitherBoss.makeInvulnerable` (`WitherBoss.java:359-363`).
            BlockPatternMob::Wither { .. } => {
                *data.wither_retained_mut()? =
                    crate::entity::wither::WitherRetainedState::from_saved(220);
            }
            BlockPatternMob::SnowGolem => {}
        }
        let uuid = self.next_uuid();
        // `snapTo` keeps the constructor's head yaw (`LivingEntity.java:279-280`).
        let constructor_yaw = self.random().next_float() * std::f32::consts::TAU;
        let position = Vec3d {
            x: f64::from(spawn_pos.x) + 0.5,
            y: f64::from(spawn_pos.y) + y_offset,
            z: f64::from(spawn_pos.z) + 0.5,
        };
        let entity_id = self.restore_initialized(
            uuid,
            entity_type_id,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(y_rot, 0.0);
                let living = entity.living_mut().expect("a pattern summon is living");
                living.y_head_rot = constructor_yaw;
                if matches!(mob, BlockPatternMob::Wither { .. }) {
                    living.y_body_rot = y_rot;
                    let health = living.max_health() / 3.0;
                    crate::entity::living::set_health(entity, health);
                }
            },
        )?;
        if matches!(mob, BlockPatternMob::CopperGolem { .. }) {
            self.with_entity_mut(entity_id, |entity, store| {
                crate::entity::copper_golem::play_spawn_sound(entity, store);
            });
        }
        Some(entity_id)
    }

    /// Creates one turtle after `TurtleEggBlock` has committed its hatch.
    pub fn spawn_hatched_turtle(&mut self, position: Vec3d, home_pos: BlockPos) -> Option<i32> {
        // `TurtleEggBlock.randomTick` initializes age, home and snap rotation
        // before publication (`TurtleEggBlock.java:95-119`).
        let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_TURTLE_ID)?;
        if !crate::entity::turtle::initialize_hatch(&mut data, home_pos) {
            return None;
        }
        let uuid = self.next_uuid();
        self.restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(0.0, 0.0);
                entity
                    .living_mut()
                    .expect("hatched turtle is living")
                    .y_head_rot = 0.0;
            },
        )
    }

    /// `SnifferEggBlock.tick`'s post-destroy construction
    /// (`SnifferEggBlock.java:73-78`).
    pub fn spawn_hatched_sniffer(&mut self, position: Vec3d, y_rot: f32) -> Option<i32> {
        let data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_SNIFFER_ID)?;
        let uuid = self.next_uuid();
        // `LivingEntity` keeps its constructor yaw as head rotation after
        // `snapTo` replaces only entity rotation (`LivingEntity.java:272-281`).
        let constructor_yaw = self.random().next_float() * std::f32::consts::TAU;
        self.restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                assert!(crate::entity::set_spawner_age(entity, -24_000));
                entity.set_rot(y_rot, 0.0);
                entity
                    .living_mut()
                    .expect("hatched Sniffer is living")
                    .y_head_rot = constructor_yaw;
            },
        )
    }

    /// `DriedGhastBlock.spawnGhastling` (`DriedGhastBlock.java:115-126`).
    pub fn spawn_hydrated_happy_ghast(&mut self, position: Vec3d, y_rot: f32) -> Option<i32> {
        let data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID)?;
        let uuid = self.next_uuid();
        let entity_id = self.restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                assert!(crate::entity::set_spawner_age(entity, -24_000));
                entity.set_rot(y_rot, 0.0);
                entity
                    .living_mut()
                    .expect("hydrated Happy Ghast is living")
                    .y_head_rot = y_rot;
            },
        )?;
        let _ = self.play_sound_from_entity(
            entity_id,
            "minecraft:entity.ghastling.spawn",
            SoundSource::Blocks,
            1.0,
            1.0,
        );
        Some(entity_id)
    }

    /// Creates one caller-positioned Tadpole after Frogspawn commits hatching.
    pub fn spawn_frogspawn_tadpole(
        &mut self,
        position: Vec3d,
        y_rot: f32,
    ) -> Option<BlockOriginSpawnResult> {
        // Frogspawn supplies position and yaw, then makes the Tadpole
        // persistent before publication (`FrogspawnBlock.java:118-137`).
        let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_TADPOLE_ID)?;
        data.mob.as_mut()?.set_persistence_required(true);
        let uuid = self.next_uuid();
        // Living construction draws yaw before Frogspawn supplies the published yaw
        // (`LivingEntity.java:272-280`, `FrogspawnBlock.java:118-137`).
        let entity_id = self.restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_TADPOLE_ID,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(y_rot, 0.0);
                entity
                    .living_mut()
                    .expect("frogspawn tadpole is living")
                    .y_head_rot = y_rot;
            },
        )?;
        Some(BlockOriginSpawnResult { entity_id })
    }

    pub fn create_portal_zombified_piglin(
        &mut self,
        position: Vec3d,
        y_rot: f32,
    ) -> Option<Box<Entity>> {
        let data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID)?;
        let uuid = self.next_uuid();
        // `EntityType.spawn` snaps yaw and aligns a Mob's head/body before
        // `finalizeSpawn` and publication (`EntityType.java:1327-1344`).
        let id = self.restore_initialized(
            uuid,
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(y_rot, 0.0);
                let living = entity
                    .living_mut()
                    .expect("portal Zombified Piglin is living");
                living.y_head_rot = y_rot;
                living.y_body_rot = y_rot;
            },
        )?;
        self.take_fresh(id)
    }
}

#[cfg(test)]
mod tests;
