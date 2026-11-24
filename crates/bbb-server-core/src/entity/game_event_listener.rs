//! Entity vibration and jukebox listeners.

use crate::block_entity::game_event::{
    containing_block_pos, EntityGameEventListenerKind, GameEventContext,
};
pub(crate) use crate::block_entity::vibration::TAG_LISTENER;
use crate::block_entity::vibration::{
    GameEventTag, VibrationData, VibrationEntity, VibrationSystem,
};
use crate::connection::ClientboundPlayPacket;
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use crate::world::simulation_tick::vibration::WorldVibrationReads;
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_synched_data::VANILLA_ALLAY_DANCING_DATA_ID;
use bbb_protocol::entity_types::{
    vanilla_entity_type, VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
};
use bbb_protocol::packets::{BlockPos, EntityEvent, LevelParticles, ParticlePayload, Vec3d};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_VIBRATION_ID;
use bbb_protocol::NbtSummaryEntry;
use uuid::Uuid;

use crate::entity::ai::{MemoryModuleType, MemoryValue};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData, GeneralLivingData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VibrationUserKind {
    Allay,
    Warden,
}

impl VibrationUserKind {
    const fn of(entity_type_id: i32) -> Option<Self> {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_ALLAY_ID => Some(Self::Allay),
            VANILLA_ENTITY_TYPE_WARDEN_ID => Some(Self::Warden),
            _ => None,
        }
    }
}

/// One dynamic listener an entity type registers, in the order its
/// `updateDynamicGameEventListener` hands them to the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityGameEventListenerSpec {
    pub kind: EntityGameEventListenerKind,
    /// `GameEventListener.getListenerRadius()`.
    pub radius: i32,
}

/// `Allay.updateDynamicGameEventListener` (`Allay.java:369-374`): the
/// vibration listener (`:552-554`), then the jukebox listener at
/// `GameEvent.JUKEBOX_PLAY`'s notification radius (`:109-111`).
const ALLAY_LISTENERS: [EntityGameEventListenerSpec; 2] = [
    EntityGameEventListenerSpec {
        kind: EntityGameEventListenerKind::Vibration,
        radius: 16,
    },
    EntityGameEventListenerSpec {
        kind: EntityGameEventListenerKind::AllayJukebox,
        radius: 10,
    },
];

/// `Warden.updateDynamicGameEventListener` (`Warden.java:382-386`), whose one
/// listener answers radius 16 (`:569-571`).
const WARDEN_LISTENERS: [EntityGameEventListenerSpec; 1] = [EntityGameEventListenerSpec {
    kind: EntityGameEventListenerKind::Vibration,
    radius: 16,
}];

#[must_use]
pub fn entity_game_event_listeners(entity_type_id: i32) -> &'static [EntityGameEventListenerSpec] {
    match VibrationUserKind::of(entity_type_id) {
        Some(VibrationUserKind::Allay) => &ALLAY_LISTENERS,
        Some(VibrationUserKind::Warden) => &WARDEN_LISTENERS,
        None => &[],
    }
}

fn listener_y_offset(entity_type_id: i32) -> f32 {
    vanilla_entity_type(entity_type_id)
        .map_or(0.0, |entry| entry.eye_height.unwrap_or(entry.height * 0.85))
}

#[must_use]
pub fn entity_listener_position(entity: &Entity) -> Vec3d {
    let position = entity.position();
    Vec3d {
        x: position.x,
        y: position.y + f64::from(listener_y_offset(entity.entity_type_id())),
        z: position.z,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReceivedVibration {
    pub pos: BlockPos,
    pub game_event: &'static str,
    pub source: Option<Uuid>,
    pub projectile_owner: Option<Uuid>,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct AllayJukeboxState {
    jukebox_pos: Option<BlockPos>,
    pub(crate) dancing: bool,
    pub(super) duplication_cooldown: i64,
}

impl AllayJukeboxState {
    pub(crate) fn can_duplicate(&self) -> bool {
        self.duplication_cooldown == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityGameEventListenerState {
    user: VibrationUserKind,
    vibration: VibrationData,
    pub(crate) allay: Option<AllayJukeboxState>,
}

impl EntityGameEventListenerState {
    pub(crate) fn new(entity_type_id: i32) -> Option<Self> {
        let user = VibrationUserKind::of(entity_type_id)?;
        Some(Self {
            user,
            vibration: VibrationData::default(),
            allay: (user == VibrationUserKind::Allay).then(AllayJukeboxState::default),
        })
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        if let (Some(current), Some(previous)) = (&mut self.allay, &previous.allay) {
            current.jukebox_pos = previous.jukebox_pos;
            current.dancing = previous.dancing;
        }
    }
}

impl GeneralLivingData {
    /// `VibrationSystem.getVibrationData()` of an allay or a warden.
    #[must_use]
    pub fn vibration_data(&self) -> Option<&VibrationData> {
        Some(&self.game_event_listener_state()?.vibration)
    }

    #[must_use]
    pub fn allay_dancing(&self) -> Option<bool> {
        Some(self.game_event_listener_state()?.allay?.dancing)
    }

    #[must_use]
    pub fn allay_jukebox_pos(&self) -> Option<BlockPos> {
        self.game_event_listener_state()?.allay?.jukebox_pos
    }
}

fn listener_state_mut(entity: &mut Entity) -> Option<&mut EntityGameEventListenerState> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data.game_event_listener_state_mut(),
        _ => None,
    }
}

pub struct EntityVibrationListener {
    user: VibrationUserKind,
    data: VibrationData,
    position: Vec3d,
    can_receive: bool,
}

/// Takes `entity`'s vibration listener out for one delivery, with
/// `canReceiveVibration` already answered (it reads only the entity and the
/// facts the caller passed, so asking it first changes nothing).
pub fn take_vibration_listener(
    entity: &mut Entity,
    can_receive: bool,
) -> Option<EntityVibrationListener> {
    let position = entity_listener_position(entity);
    let state = listener_state_mut(entity)?;
    Some(EntityVibrationListener {
        user: state.user,
        data: std::mem::take(&mut state.vibration),
        position,
        can_receive,
    })
}

/// Puts a listener [`take_vibration_listener`] lent out back.
pub fn put_back_vibration_listener(entity: &mut Entity, listener: EntityVibrationListener) {
    if let Some(state) = listener_state_mut(entity) {
        state.vibration = listener.data;
    }
}

impl VibrationSystem for EntityVibrationListener {
    fn vibration_data(&self) -> &VibrationData {
        &self.data
    }

    fn vibration_data_mut(&mut self) -> &mut VibrationData {
        &mut self.data
    }

    fn vibration_listener_position(&self) -> Option<Vec3d> {
        Some(self.position)
    }

    fn vibration_listenable_events(&self) -> GameEventTag {
        match self.user {
            VibrationUserKind::Allay => GameEventTag::AllayCanListen,
            VibrationUserKind::Warden => GameEventTag::WardenCanListen,
        }
    }

    fn can_trigger_avoid_vibration(&self) -> bool {
        matches!(self.user, VibrationUserKind::Warden)
    }

    fn can_receive_vibration(
        &self,
        _level: &mut WorldVibrationReads<'_>,
        _pos: BlockPos,
        _event: VanillaGameEvent,
        _context: GameEventContext,
    ) -> bool {
        self.can_receive
    }
}

/// What `canReceiveVibration` reads off the level rather than off the entity.
#[derive(Debug, Clone, Copy)]
pub struct VibrationReceiveFacts<'a> {
    /// `level.dimension()`, as its resource id.
    pub dimension: &'a str,
    /// `level.getWorldBorder().isWithinBounds(pos)`.
    pub pos_within_border: bool,
    /// `Warden.canTargetEntity(context.sourceEntity())` when the source is a
    /// `LivingEntity`; `None` for any other source.
    pub living_source_targetable: Option<bool>,
}

/// `VibrationSystem.User.canReceiveVibration` of the two entity users.
#[must_use]
pub fn can_receive_vibration(
    entity: &Entity,
    pos: BlockPos,
    facts: VibrationReceiveFacts<'_>,
) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    let Some(state) = data.game_event_listener_state() else {
        return false;
    };
    if data.is_no_ai() {
        return false;
    }
    match state.user {
        VibrationUserKind::Allay => {
            match brain_memory(entity, MemoryModuleType::LikedNoteblockPosition) {
                None => true,
                Some(MemoryValue::GlobalPos(liked)) => {
                    let here = entity.block_position();
                    liked.dimension == facts.dimension
                        && (liked.pos.x - here.x)
                            .abs()
                            .max((liked.pos.y - here.y).abs())
                            .max((liked.pos.z - here.z).abs())
                            <= 1024
                        && liked.pos == pos
                }
                Some(_) => false,
            }
        }
        VibrationUserKind::Warden => {
            !data.living().is_dead_or_dying()
                && data
                    .warden_state()
                    .is_some_and(|state| !state.vibration_cooldown())
                && !is_digging_or_emerging(entity)
                && facts.pos_within_border
                && facts.living_source_targetable != Some(false)
        }
    }
}

/// `Brain.getMemory` over the brain record the entity carries: neither user has
/// a live brain in this server yet, so the record is the brain's whole state.
fn brain_memory(entity: &Entity, memory_type: MemoryModuleType) -> Option<&MemoryValue> {
    crate::entity::general_living::spawner_brain_memory(entity, memory_type)
        .map(|memory| &memory.value)
}

fn is_digging_or_emerging(entity: &Entity) -> bool {
    matches!(
        entity.pose_id(),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_DIGGING_ID
            | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_EMERGING_ID
    )
}

pub fn allay_jukebox_handle_game_event(
    store: &mut EntityStore,
    entity_id: i32,
    event: VanillaGameEvent,
    source_position: Vec3d,
) -> bool {
    let playing = if event == VanillaGameEvent::JUKEBOX_PLAY {
        true
    } else if event == VanillaGameEvent::JUKEBOX_STOP_PLAY {
        false
    } else {
        return false;
    };
    let panicking = is_panicking(store, entity_id);
    let Some(entity) = store.get_mut(entity_id) else {
        return false;
    };
    set_jukebox_playing(
        entity,
        containing_block_pos(source_position),
        playing,
        panicking,
    );
    true
}

fn is_panicking(store: &EntityStore, entity_id: i32) -> bool {
    store
        .mob_ai(entity_id)
        .is_some_and(crate::entity::ai::mob::MobAiRuntime::is_panicking)
}

fn set_jukebox_playing(entity: &mut Entity, jukebox: BlockPos, playing: bool, panicking: bool) {
    let Some(allay) = listener_state_mut(entity).and_then(|state| state.allay.as_mut()) else {
        return;
    };
    let dancing = if playing {
        if allay.dancing {
            return;
        }
        allay.jukebox_pos = Some(jukebox);
        true
    } else if allay.jukebox_pos.is_none_or(|pos| pos == jukebox) {
        allay.jukebox_pos = None;
        false
    } else {
        return;
    };
    set_dancing(entity, dancing, panicking);
}

fn set_dancing(entity: &mut Entity, dancing: bool, panicking: bool) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    if data.is_no_ai() || (dancing && panicking) {
        return;
    }
    let Some(allay) = data
        .game_event_listener_state_mut()
        .and_then(|state| state.allay.as_mut())
    else {
        return;
    };
    if allay.dancing != dancing {
        allay.dancing = dancing;
        entity.mark_synched_dirty(VANILLA_ALLAY_DANCING_DATA_ID);
    }
}

pub(crate) fn warden_before_super(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID {
        tick_vibration(entity, store, level);
    }
}

pub(crate) fn allay_after_super(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_ALLAY_ID {
        return;
    }
    tick_vibration(entity, store, level);
    if is_panicking(store, entity.id()) {
        set_dancing(entity, false, true);
    }
}

pub(crate) fn allay_after_ai_step(
    entity: &mut Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    crate::entity::allay::tick_duplication_cooldown(entity);
    let Some(allay) = listener_state_mut(entity).and_then(|state| state.allay) else {
        return;
    };
    if !allay.dancing || entity.tick_count() % 20 != 0 {
        return;
    }
    let position = entity.position();
    let should_stop = allay.jukebox_pos.is_none_or(|pos| {
        let dx = f64::from(pos.x) + 0.5 - position.x;
        let dy = f64::from(pos.y) + 0.5 - position.y;
        let dz = f64::from(pos.z) + 0.5 - position.z;
        dx * dx + dy * dy + dz * dz >= 10.0 * 10.0
            || level
                .block_state_checked(pos)
                .and_then(bbb_server_types::block_queries::block_type_of)
                != Some(bbb_protocol::block_types::VANILLA_BLOCK_JUKEBOX_ID)
    });
    if should_stop {
        let panicking = is_panicking(store, entity.id());
        set_dancing(entity, false, panicking);
        if let Some(allay) = listener_state_mut(entity).and_then(|state| state.allay.as_mut()) {
            allay.jukebox_pos = None;
        }
    }
}

fn tick_vibration(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let entity_id = entity.id();
    let Some(mut listener) = take_vibration_listener(entity, false) else {
        return;
    };
    let tick = listener
        .data
        .tick(level.game_time(), Some(listener.position), true);
    for particle in tick.particles.into_iter().flatten() {
        store.send_particles(LevelParticles {
            override_limiter: false,
            always_show: false,
            position: particle.origin,
            offset: Vec3d::default(),
            max_speed: 0.0,
            count: 1,
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_VIBRATION_ID,
                raw_options: entity_vibration_particle_options(
                    entity_id,
                    listener_y_offset(entity.entity_type_id()),
                    particle.travel_time_in_ticks,
                ),
            },
        });
    }
    let received = tick.arrival.map(|arrival| {
        let source = arrival
            .info
            .entity(|uuid| resolve_vibration_entity(store, level, uuid));
        let owner = arrival
            .info
            .projectile_owner(source, |uuid| resolve_vibration_entity(store, level, uuid));
        ReceivedVibration {
            pos: containing_block_pos(arrival.info.pos),
            game_event: arrival.info.game_event,
            source: source.map(|entity| entity.uuid),
            projectile_owner: owner.map(|entity| entity.uuid),
            distance: arrival.distance,
        }
    });
    put_back_vibration_listener(entity, listener);
    if let Some(received) = received {
        on_receive_vibration(entity, store, level, received);
    }
}

fn on_receive_vibration(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    received: ReceivedVibration,
) {
    let Some(user) = listener_state_mut(entity).map(|state| state.user) else {
        return;
    };
    match user {
        VibrationUserKind::Allay => {
            if received.game_event == VanillaGameEvent::NOTE_BLOCK_PLAY.resource_id() {
                crate::entity::allay::hear_noteblock(entity, level, received.pos);
            }
        }
        VibrationUserKind::Warden => {
            let dead_or_dying = entity
                .living()
                .is_some_and(crate::entity::LivingEntityState::is_dead_or_dying);
            if dead_or_dying {
                return;
            }
            crate::entity::warden::anger::on_vibration(entity, store, level, received);
            let entity_id = entity.id();
            store.broadcast_to_tracking(
                entity_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: 61,
                }),
            );
            // baby, so `getVoicePitch` takes the adult arm off the mob's own random.
            let pitch = store
                .mob_ai_mut(entity_id)
                .map(|runtime| runtime.voice_pitch(false));
            if let Some(pitch) = pitch.filter(|_| !entity.is_silent()) {
                store.play_entity_sound(PendingEntitySound {
                    position: entity.position(),
                    resource_id: "minecraft:entity.warden.tendril_clicks".to_owned(),
                    source: crate::entity::living::living_sound_source(entity),
                    volume: 5.0,
                    pitch,
                });
            }
        }
    }
}

fn resolve_vibration_entity(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    uuid: Uuid,
) -> Option<VibrationEntity> {
    if let Some(entity) = store
        .visible_entity_id_for_uuid(uuid)
        .and_then(|id| store.get(id))
    {
        return Some(VibrationEntity {
            uuid,
            is_player: false,
            is_projectile: entity.is_projectile_class(),
            controlling_passenger_player: None,
            projectile_owner: entity.data().projectile_owner(),
            item_owner: entity.data().item_thrower(),
        });
    }
    level.player(uuid).map(|_| VibrationEntity {
        uuid,
        is_player: true,
        is_projectile: false,
        controlling_passenger_player: None,
        projectile_owner: None,
        item_owner: None,
    })
}

fn entity_vibration_particle_options(
    entity_id: i32,
    y_offset: f32,
    arrival_in_ticks: i32,
) -> Vec<u8> {
    let mut encoder = Encoder::default();
    encoder.write_var_i32(1);
    encoder.write_var_i32(entity_id);
    encoder.write_f32(y_offset);
    encoder.write_var_i32(arrival_in_ticks);
    encoder.into_inner()
}

impl EntityGameEventListenerState {
    pub(crate) fn read_save_data(&mut self, entries: &[NbtSummaryEntry]) {
        if let Some(allay) = &mut self.allay {
            allay.duplication_cooldown =
                crate::entity::save::optional_long(entries, "DuplicationCooldown").unwrap_or(0);
        }
        self.vibration = entries
            .iter()
            .find(|entry| entry.name == TAG_LISTENER)
            .and_then(|entry| match &entry.value {
                bbb_protocol::NbtSummaryValue::Compound(fields) => VibrationData::load(fields),
                _ => None,
            })
            .unwrap_or_default();
    }
}

/// `addAdditionalSaveData`'s `output.store("listener", Data.CODEC, ...)`.
pub(crate) fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    state: &EntityGameEventListenerState,
) {
    // Allay.java:427-432.
    if let Some(allay) = &state.allay {
        entries.push(NbtSummaryEntry {
            name: "DuplicationCooldown".into(),
            value: bbb_protocol::NbtSummaryValue::Long(allay.duplication_cooldown),
        });
    }
    entries.push(NbtSummaryEntry {
        name: TAG_LISTENER.into(),
        value: bbb_protocol::NbtSummaryValue::Compound(state.vibration.save()),
    });
}
