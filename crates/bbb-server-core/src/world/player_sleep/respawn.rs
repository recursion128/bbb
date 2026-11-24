use super::*;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::VANILLA_BLOCK_RESPAWN_ANCHOR_ID;
use bbb_protocol::packets::{SoundEvent, SoundEventHolder, SoundSource};
#[derive(Debug)]
pub(in crate::world) enum SleepRespawnError {
    Save(bbb_server_types::save::SaveError),
    Unavailable,
}
impl From<bbb_server_types::save::SaveError> for SleepRespawnError {
    fn from(error: bbb_server_types::save::SaveError) -> Self {
        Self::Save(error)
    }
}
impl std::fmt::Display for SleepRespawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Save(error) => error.fmt(f),
            Self::Unavailable => f.write_str("respawn landing facts are unavailable"),
        }
    }
}

impl ServerWorld {
    // ServerPlayer.java:995-1013,1033-1067: only a resolved anchor landing consumes charge.
    pub(in crate::world) fn resolve_sleep_respawn(
        &mut self,
        config: Option<&PlayerRespawnConfig>,
        consume: bool,
    ) -> Result<(super::super::WorldSpawn, bool), SleepRespawnError> {
        let mut missing = false;
        if let Some(config) = config.filter(|config| self.has_level_key(&config.dimension)) {
            let pos = config.pos;
            let chunks: Vec<_> = [-4, 4]
                .into_iter()
                .flat_map(|x| {
                    [-4, 4].into_iter().map(move |z| {
                        super::super::player_chunk(Vec3d {
                            x: f64::from(pos.x.wrapping_add(x)),
                            y: f64::from(pos.y),
                            z: f64::from(pos.z.wrapping_add(z)),
                        })
                    })
                })
                .collect();
            self.install_full_chunks_by_key(&self.require_level_id(&config.dimension), &chunks)?;
            if let Some((position, rotation)) = self
                .sleep_respawn_at(config, consume)
                .ok_or(SleepRespawnError::Unavailable)?
            {
                return Ok((
                    super::super::WorldSpawn {
                        dimension: self.require_level_id(&config.dimension),
                        position,
                        rotation,
                    },
                    false,
                ));
            }
            missing = true;
        }
        self.find_default_spawn()
            .map(|spawn| (spawn, missing))
            .map_err(Into::into)
    }

    fn sleep_respawn_at(
        &mut self,
        config: &PlayerRespawnConfig,
        consume: bool,
    ) -> Option<Option<(Vec3d, PlayerRotation)>> {
        let pos = config.pos;
        let dimension = &self.require_level_id(&config.dimension);
        let state = stored_block_state(&self.level_by_id(dimension).chunks, pos);
        let anchor =
            vanilla_block_state_block_type_id(state) == Some(VANILLA_BLOCK_RESPAWN_ANCHOR_ID);
        let charges = bbb_server_types::block_queries::int_property(state, "charges").unwrap_or(0);
        let facing =
            if anchor && (config.forced || charges > 0) && self.anchor_works_at(dimension, pos)? {
                None
            } else if bed::is_bed(state)
                && self
                    .bed_rule_at(dimension, pos)?
                    .can_set_spawn
                    .allows(self.is_dark_outside(dimension)?)
            {
                Some(direction_property(state, "facing")?)
            } else {
                let chunks = &self.level_by_id(dimension).chunks;
                let free = |pos: BlockPos| {
                    chunks
                        .get_block_state(pos.x, pos.y, pos.z)
                        .and_then(|state| possible_to_respawn_in(state, chunks.content_registry()))
                };
                return Some(
                    (config.forced
                        && free(pos)?
                        && free(BlockPos {
                            y: pos.y.wrapping_add(1),
                            ..pos
                        })?)
                    .then_some((
                        Vec3d {
                            x: f64::from(pos.x) + 0.5,
                            y: f64::from(pos.y) + 0.1,
                            z: f64::from(pos.z) + 0.5,
                        },
                        PlayerRotation {
                            y_rot: config.yaw,
                            x_rot: config.pitch,
                        },
                    )),
                );
            };
        let Some(position) = self.sleep_stand_up_position(
            dimension,
            pos,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            facing,
            config.yaw,
        )?
        else {
            return Some(None);
        };
        if anchor && !config.forced && consume {
            let state = with_property(state, "charges", &(charges - 1).to_string())?;
            self.set_block(dimension, pos, state, SET_BLOCK_UPDATE_ALL);
        }
        Some(Some((
            position,
            PlayerRotation {
                y_rot: look_at_bed_yaw(position, pos),
                x_rot: 0.0,
            },
        )))
    }

    // PlayerList.java:426-448: the private sound uses the destination level's random.
    pub(in crate::world) fn send_respawn_anchor_deplete(
        &mut self,
        id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(config) = self.levels[id]
            .cold
            .respawn
            .clone()
            .filter(|config| self.has_level_key(&config.dimension))
        else {
            return;
        };
        let pos = config.pos;
        if vanilla_block_state_block_type_id(stored_block_state(
            &self
                .level_by_id(&self.require_level_id(&config.dimension))
                .chunks,
            pos,
        )) != Some(VANILLA_BLOCK_RESPAWN_ANCHOR_ID)
        {
            return;
        }
        let registry_id = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:block.respawn_anchor.deplete",
        )
        .expect("generated anchor sound");
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(pos.x),
                y: f64::from(pos.y),
                z: f64::from(pos.z),
            },
            volume: 1.0,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        events.push(send(id, ClientboundPlayPacket::Sound(sound)));
    }
}

/// `Block.isPossibleToRespawnInThis` (`Block.java:486-488`) and its overrides:
/// `BasePressurePlateBlock.java:49`, `SignBlock.java:80` (every sign family)
/// and `AbstractBannerBlock.java:25`.
fn possible_to_respawn_in(
    state: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Option<bool> {
    let Some(facts) = vanilla_block_state_support_facts(state) else {
        let (block, _) = content_registry.block_by_state_id(state)?;
        return Some(!block.full_cube_collision);
    };
    let always = vanilla_block_state_block_type_id(state)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .is_some_and(|block| {
            let name = block.resource_id;
            name.ends_with("_pressure_plate")
                || name.ends_with("_sign")
                || name.ends_with("_banner")
        });
    Some(always || !facts.legacy_solid() && !facts.liquid())
}
