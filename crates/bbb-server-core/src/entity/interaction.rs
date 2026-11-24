//! `net/minecraft/world/entity/Interaction.java`: an invisible hit box that
//! remembers the last player who attacked it and the last who used it.
//!
//! `noPhysics` (`Interaction.java:42`), `getPistonPushReaction` = `IGNORE`
//! (`:89-92`) and `isIgnoringBlockTriggers` (`:94-97`) have no consumer in
//! this crate: an interaction never moves, and pistons and block triggers do
//! not reach stored entities here. `getLastAttacker`/`getTarget`
//! (`:132-140`) resolve the recorded UUIDs to players; their `Attackable`
//! and `Targeting` readers are mob AI, which has no owner here, so the UUIDs
//! are exposed as recorded.

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_INTERACTION_HEIGHT_DATA_ID, VANILLA_INTERACTION_RESPONSE_DATA_ID,
    VANILLA_INTERACTION_WIDTH_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::display::save_fields::{
    bool_or, byte_bool, codec_long, codec_uuid, entry, float, float_or, optional_field,
    uuid_int_array,
};
use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `Interaction.DEFAULT_WIDTH`/`DEFAULT_HEIGHT`/`DEFAULT_RESPONSE`
/// (`Interaction.java:34-36`).
pub const DEFAULT_WIDTH: f32 = 1.0;
pub const DEFAULT_HEIGHT: f32 = 1.0;
pub const DEFAULT_RESPONSE: bool = false;

/// `Interaction.canBeHitByProjectile()` (`Interaction.java:80-82`).
pub const CAN_BE_HIT_BY_PROJECTILE: bool = false;
/// `Interaction.isPickable()` (`Interaction.java:85-87`).
pub const IS_PICKABLE: bool = true;

/// `Interaction.PlayerAction` (`Interaction.java:180-188`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerAction {
    pub player: Uuid,
    pub timestamp: i64,
}

/// `InteractionResult` as `Interaction.interact` answers it on the server
/// (`Interaction.java:119-126`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionUse {
    Consume,
}

/// The `Interaction` fields (`Interaction.java:37-38`) beside its three
/// synched values (`:26-28`).
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionData {
    width: f32,
    height: f32,
    response: bool,
    attack: Option<PlayerAction>,
    interaction: Option<PlayerAction>,
}

impl Default for InteractionData {
    fn default() -> Self {
        Self::new()
    }
}

impl InteractionData {
    /// `defineSynchedData` (`Interaction.java:46-50`) with no action recorded.
    pub const fn new() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            response: DEFAULT_RESPONSE,
            attack: None,
            interaction: None,
        }
    }

    pub const fn width(&self) -> f32 {
        self.width
    }

    pub const fn height(&self) -> f32 {
        self.height
    }

    pub const fn response(&self) -> bool {
        self.response
    }

    pub const fn attack(&self) -> Option<PlayerAction> {
        self.attack
    }

    pub const fn interaction(&self) -> Option<PlayerAction> {
        self.interaction
    }

    /// `Interaction.getDimensions()` (`Interaction.java:166-168`):
    /// `EntityDimensions.scalable(width, height)`.
    pub const fn dimensions(&self) -> [f32; 2] {
        [self.width, self.height]
    }
}

pub fn interaction(entity: &Entity) -> Option<&InteractionData> {
    match entity.data() {
        EntityData::Interaction(data) => Some(data),
        _ => None,
    }
}

pub fn interaction_mut(entity: &mut Entity) -> Option<&mut InteractionData> {
    match &mut entity.data {
        EntityData::Interaction(data) => Some(data),
        _ => None,
    }
}

/// `new Interaction(EntityType.INTERACTION, level)` (`Interaction.java:40-43`)
/// added to the level: the UUID draw of the `Entity` constructor
/// (`Entity.java:278`), then the constructor's `setPos` (`:321`), whose
/// `makeBoundingBox` is the override at `Interaction.java:176-178`.
pub fn spawn_interaction(store: &mut EntityStore, position: Vec3d) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
        position,
        EntityData::Interaction(InteractionData::new()),
        refresh_dimensions,
    )
}

/// `Entity.refreshDimensions()` (`Entity.java:3306-3322`) for an interaction:
/// `getDimensions(Pose)` is the width/height box (`Interaction.java:171-173`)
/// and `noPhysics` keeps `fudgePositionAfterSizeChange` out.
pub fn refresh_dimensions(entity: &mut Entity) {
    let Some(data) = interaction(entity) else {
        return;
    };
    let [width, height] = data.dimensions();
    entity.set_dimensions(width, height);
}

/// `SynchedEntityData.set` (`SynchedEntityData.java:60-68`) of a float row:
/// `Float.equals` compares bits, and `onSyncedDataUpdated`
/// (`Interaction.java:72-77`) refreshes the box for either size row.
fn set_size_row(
    entity: &mut Entity,
    data_id: u8,
    value: f32,
    pick: fn(&mut InteractionData) -> &mut f32,
) {
    let Some(data) = interaction_mut(entity) else {
        return;
    };
    let slot = pick(data);
    if slot.to_bits() == value.to_bits() {
        return;
    }
    *slot = value;
    entity.mark_synched_dirty(data_id);
    refresh_dimensions(entity);
}

/// `Interaction.setWidth` (`Interaction.java:142-144`).
pub fn set_width(entity: &mut Entity, width: f32) {
    set_size_row(entity, VANILLA_INTERACTION_WIDTH_DATA_ID, width, |data| {
        &mut data.width
    });
}

/// `Interaction.setHeight` (`Interaction.java:150-152`).
pub fn set_height(entity: &mut Entity, height: f32) {
    set_size_row(entity, VANILLA_INTERACTION_HEIGHT_DATA_ID, height, |data| {
        &mut data.height
    });
}

/// `Interaction.setResponse` (`Interaction.java:158-160`).
pub fn set_response(entity: &mut Entity, response: bool) {
    let Some(data) = interaction_mut(entity) else {
        return;
    };
    if data.response == response {
        return;
    }
    data.response = response;
    entity.mark_synched_dirty(VANILLA_INTERACTION_RESPONSE_DATA_ID);
}

/// `Interaction.tick()` (`Interaction.java:129-130`): empty, so neither
/// `baseTick` nor `checkBelowWorld` runs.
pub fn tick(_entity: &mut Entity) {}

/// `Interaction.skipAttackInteraction(Entity)` (`Interaction.java:100-111`)
/// for a player source: the attack is recorded against the level's game time
/// and the attack is skipped unless `response` is set. The world fires the
/// `PLAYER_HURT_ENTITY` trigger (`:104`).
pub fn skip_attack_interaction(entity: &mut Entity, player: Uuid, game_time: i64) -> bool {
    let Some(data) = interaction_mut(entity) else {
        return false;
    };
    data.attack = Some(PlayerAction {
        player,
        timestamp: game_time,
    });
    !data.response
}

/// The server half of `Interaction.interact` (`Interaction.java:119-126`):
/// the use is recorded against the level's game time and answers `CONSUME`.
pub fn interact(entity: &mut Entity, player: Uuid, game_time: i64) -> InteractionUse {
    if let Some(data) = interaction_mut(entity) {
        data.interaction = Some(PlayerAction {
            player,
            timestamp: game_time,
        });
    }
    InteractionUse::Consume
}

/// The `Entity` rows, then `defineSynchedData` (`Interaction.java:46-50`) in
/// id order.
pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    let Some(data) = interaction(entity) else {
        return rows;
    };
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_INTERACTION_WIDTH_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(data.width),
        },
        is_default: data.width.to_bits() == DEFAULT_WIDTH.to_bits(),
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_INTERACTION_HEIGHT_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(data.height),
        },
        is_default: data.height.to_bits() == DEFAULT_HEIGHT.to_bits(),
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_INTERACTION_RESPONSE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.response),
        },
        is_default: data.response == DEFAULT_RESPONSE,
    });
    rows
}

/// `Interaction.PlayerAction.CODEC` (`Interaction.java:181-187`) written.
fn player_action_record(action: PlayerAction) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("player", uuid_int_array(action.player)),
        entry("timestamp", NbtSummaryValue::Long(action.timestamp)),
    ])
}

/// `Interaction.PlayerAction.CODEC` (`Interaction.java:181-187`) read: both
/// fields are required, so a malformed record decodes to nothing.
fn read_player_action(value: &NbtSummaryValue) -> Option<PlayerAction> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    Some(PlayerAction {
        player: codec_uuid(optional_field(entries, "player")?)?,
        timestamp: codec_long(optional_field(entries, "timestamp")?)?,
    })
}

/// `Interaction.addAdditionalSaveData` (`Interaction.java:63-69`).
pub fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, data: &InteractionData) {
    entries.push(entry("width", float(data.width)));
    entries.push(entry("height", float(data.height)));
    if let Some(attack) = data.attack {
        entries.push(entry("attack", player_action_record(attack)));
    }
    if let Some(interaction) = data.interaction {
        entries.push(entry("interaction", player_action_record(interaction)));
    }
    entries.push(entry("response", byte_bool(data.response)));
}

/// `Interaction.readAdditionalSaveData` (`Interaction.java:53-60`) as the
/// field values it leaves; [`finish_load`] is the rest of it.
pub fn read_save_data(entries: &[NbtSummaryEntry]) -> InteractionData {
    InteractionData {
        width: float_or(entries, "width", DEFAULT_WIDTH),
        height: float_or(entries, "height", DEFAULT_HEIGHT),
        attack: optional_field(entries, "attack").and_then(read_player_action),
        interaction: optional_field(entries, "interaction").and_then(read_player_action),
        response: bool_or(entries, "response", DEFAULT_RESPONSE),
    }
}

/// What `readAdditionalSaveData` (`Interaction.java:53-60`) does beyond the
/// field values on a freshly constructed entity: each `entityData.set` of a
/// value away from its `define`d default marks the row dirty
/// (`SynchedEntityData.java:60-68`), and `setBoundingBox(makeBoundingBox())`
/// (`:59`) rebuilds the box from the loaded size.
pub fn finish_load(entity: &mut Entity) {
    let Some(data) = interaction(entity) else {
        return;
    };
    let dirty = [
        (
            VANILLA_INTERACTION_WIDTH_DATA_ID,
            data.width.to_bits() != DEFAULT_WIDTH.to_bits(),
        ),
        (
            VANILLA_INTERACTION_HEIGHT_DATA_ID,
            data.height.to_bits() != DEFAULT_HEIGHT.to_bits(),
        ),
        (
            VANILLA_INTERACTION_RESPONSE_DATA_ID,
            data.response != DEFAULT_RESPONSE,
        ),
    ];
    for (data_id, changed) in dirty {
        if changed {
            entity.mark_synched_dirty(data_id);
        }
    }
    refresh_dimensions(entity);
}
