//! Retained leash relationships, restoration, and live leash physics.

use crate::connection::ClientboundPlayPacket;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, ItemStackSummary, SetEntityLink, SoundSource, Vec3d,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::{mth_cos, mth_sin, Aabb, Vec3Ops};
use uuid::Uuid;

use crate::entity::ai::mob::navigation::MobPathingSnapshot;
use crate::entity::ai::mob::spec::is_instance_of;
use crate::entity::display::save_fields::{codec_uuid, uuid_int_array};
use crate::entity::level::NearbyPlayer;
use crate::entity::save::EntitySaveError;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::Entity;

mod interact;

pub use interact::{
    bind_player_mobs, interact_leash, interact_leash_with_loot, EntityInteractionLoot,
    LeadBindOutcome, LeashInteractingPlayer, LeashInteraction,
};
pub(crate) use interact::{drop_all_leash_connections, shear_off_all_leash_connections};

/// `Leashable.LEASH_TAG` (`Leashable.java:29`).
const LEASH_TAG: &str = "leash";

/// `Leashable.leashSnapDistance` (`:181-183`), which is `LEASH_TOO_FAR_DIST`.
const LEASH_SNAP_DISTANCE: f64 = 12.0;

/// `Ghast.leashSnapDistance` (`Ghast.java:183-185`) and
/// `HappyGhast.leashSnapDistance` (`HappyGhast.java:505-507`).
fn leash_snap_distance(entity: &Entity) -> f64 {
    if matches!(
        entity.data().entity_resource_id(),
        "minecraft:ghast" | "minecraft:happy_ghast"
    ) {
        16.0
    } else {
        LEASH_SNAP_DISTANCE
    }
}

/// `Leashable.leashElasticDistance` (`Leashable.java:185-187`), overridden to
/// `10.0` by `Ghast` (`Ghast.java:178-180`) and `HappyGhast`
/// (`HappyGhast.java:500-502`).
const LEASH_ELASTIC_DISTANCE: f64 = 6.0;

fn leash_elastic_distance(entity: &Entity) -> f64 {
    if matches!(
        entity.data().entity_resource_id(),
        "minecraft:ghast" | "minecraft:happy_ghast"
    ) {
        10.0
    } else {
        LEASH_ELASTIC_DISTANCE
    }
}

/// `Leashable.supportQuadLeash` (`Leashable.java:209`), overridden by
/// `AbstractBoat`, `AbstractHorse` (`AbstractHorse.java:198`, except `Llama`
/// `Llama.java:417`), and `Sniffer` (`Sniffer.java:152`).
fn supports_quad_leash(entity: &Entity) -> bool {
    if crate::entity::boat::boat(entity.data()).is_some() {
        return true;
    }
    let id = entity.entity_type_id();
    id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID
        || (crate::entity::general_living::is_abstract_horse_entity_type(id)
            && !matches!(
                id,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LLAMA_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            ))
}

/// `Leashable.AXIS_SPECIFIC_ELASTICITY` (`:33`).
const AXIS_SPECIFIC_ELASTICITY: Vec3d = Vec3d {
    x: 0.8,
    y: 0.2,
    z: 0.8,
};

/// `Leashable.TORSIONAL_ELASTICITY` (`:35`).
const TORSIONAL_ELASTICITY: f64 = 10.0;

/// `Leashable.STIFFNESS` (`:36`).
const STIFFNESS: f64 = 0.11;

/// `Leashable.ENTITY_ATTACHMENT_POINT` (`:37`).
const ENTITY_ATTACHMENT_POINT: [Vec3d; 1] = [Vec3d {
    x: 0.0,
    y: 0.5,
    z: 0.5,
}];

/// `Leashable.LEASHER_ATTACHMENT_POINT` (`:38`).
const LEASHER_ATTACHMENT_POINT: [Vec3d; 1] = [Vec3d {
    x: 0.0,
    y: 0.5,
    z: 0.0,
}];

/// `Leashable.SHARED_QUAD_ATTACHMENT_POINTS` (`:39-41`).
const SHARED_QUAD_ATTACHMENT_POINTS: [Vec3d; 4] = [
    Vec3d {
        x: -0.5,
        y: 0.5,
        z: 0.5,
    },
    Vec3d {
        x: -0.5,
        y: 0.5,
        z: -0.5,
    },
    Vec3d {
        x: 0.5,
        y: 0.5,
        z: -0.5,
    },
    Vec3d {
        x: 0.5,
        y: 0.5,
        z: 0.5,
    },
];

/// The tick count `restoreLeashFromSave` gives an unresolved holder before it
/// drops the lead (`:104`).
pub(crate) const DELAYED_RESTORE_TICKS: i32 = 100;

/// `SoundEvents.LEAD_BREAK` (`SoundEvents.java:921`).
pub(crate) const LEAD_BREAK_SOUND: &str = "minecraft:item.lead.break";

/// `Items.LEAD`, the stack `dropLeash` spawns (`:130`).
const LEAD_ITEM: &str = "minecraft:lead";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeashAttachment {
    Holder(Uuid),
    Knot(BlockPos),
}

/// `Leashable.LeashData` (`:366-402`).
///
/// `delayedLeashHolderId` (`:380`) is the client's own resolution path
/// (`getLeashHolder`, `:340-344`) and has nothing to do on a server.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeashData {
    leash_holder: Option<i32>,
    delayed_leash_info: Option<LeashAttachment>,
    angular_momentum: f64,
    /// What the encoder would draw off the live holder (`:370-378`), captured
    /// when the holder is set: the save writer has no store to read it through,
    /// and neither a knot's position nor an entity's UUID can change under it.
    holder_attachment: Option<LeashAttachment>,
    /// Whether `leash_holder` names a session rather than a stored entity.
    /// Vanilla holds the object and needs no flag; here it decides whether an
    /// unresolved id means a removed holder (drop) or a player in another
    /// level (hold).
    holder_is_player: bool,
}

impl LeashData {
    /// `LeashData(Either<UUID, BlockPos>)` (`:385-387`), the decoded form.
    const fn delayed(attachment: LeashAttachment) -> Self {
        Self {
            leash_holder: None,
            delayed_leash_info: Some(attachment),
            angular_momentum: 0.0,
            holder_attachment: None,
            holder_is_player: false,
        }
    }

    /// `LeashData(Entity)` (`:389-391`).
    const fn held(holder: i32, attachment: LeashAttachment, is_player: bool) -> Self {
        Self {
            leash_holder: Some(holder),
            delayed_leash_info: None,
            angular_momentum: 0.0,
            holder_attachment: Some(attachment),
            holder_is_player: is_player,
        }
    }

    /// `LeashData.setLeashHolder` (`:397-401`).
    fn set_leash_holder(&mut self, holder: i32, attachment: LeashAttachment, is_player: bool) {
        self.leash_holder = Some(holder);
        self.delayed_leash_info = None;
        self.holder_attachment = Some(attachment);
        self.holder_is_player = is_player;
    }

    pub const fn leash_holder(&self) -> Option<i32> {
        self.leash_holder
    }

    pub const fn delayed_leash_info(&self) -> Option<LeashAttachment> {
        self.delayed_leash_info
    }

    pub const fn angular_momentum(&self) -> f64 {
        self.angular_momentum
    }
}

/// `Leashable.isLeashed` (`:47-49`).
pub fn is_leashed(entity: &Entity) -> bool {
    leash_holder(entity).is_some()
}

/// `Leashable.getLeashHolder` (`:330-347`) reduced to the id the store or
/// session named.
pub fn leash_holder(entity: &Entity) -> Option<i32> {
    leash_data(entity).and_then(LeashData::leash_holder)
}

pub(crate) fn leash_data(entity: &Entity) -> Option<&LeashData> {
    if let crate::entity::EntityData::GeneralLiving(data) = entity.data() {
        return data.mob.as_ref().and_then(|mob| mob.leash.as_ref());
    }
    crate::entity::boat::boat(entity.data()).and_then(crate::entity::boat::BoatData::leash_data)
}

fn set_leash_data(entity: &mut Entity, data: Option<LeashData>) {
    if let crate::entity::EntityData::GeneralLiving(living) = &mut entity.data {
        if let Some(mob) = living.mob.as_mut() {
            mob.leash = data;
        }
    } else if let Some(boat) = crate::entity::boat::boat_mut(&mut entity.data) {
        boat.set_leash_data(data);
    }
}

fn leash_data_mut(entity: &mut Entity) -> Option<&mut LeashData> {
    match &mut entity.data {
        crate::entity::EntityData::GeneralLiving(data) => {
            data.mob.as_mut().and_then(|mob| mob.leash.as_mut())
        }
        data => crate::entity::boat::boat_mut(data)
            .and_then(crate::entity::boat::BoatData::leash_data_mut),
    }
}

/// `Leashable.readLeashData` (`:76-83`): an undecodable tag leaves no leash.
///
/// The `removeLeash` arm at `:78-80` is for a reload over live data; a record
/// read builds the entity, so the entity being filled never has leash data yet.
pub(crate) fn read_leash_data(entries: &[NbtSummaryEntry]) -> Option<LeashData> {
    read_attachment(crate::entity::save::optional_field(entries, LEASH_TAG)?)
        .map(LeashData::delayed)
}

/// `Codec.xor(UUIDUtil.CODEC.fieldOf("UUID").codec(), BlockPos.CODEC)` (`:367`).
fn read_attachment(value: &NbtSummaryValue) -> Option<LeashAttachment> {
    match value {
        NbtSummaryValue::Compound(entries) => {
            codec_uuid(crate::entity::save::optional_field(entries, "UUID")?)
                .map(LeashAttachment::Holder)
        }
        _ => crate::entity::save::read_block_pos_codec_input(value).map(LeashAttachment::Knot),
    }
}

/// `Leashable.writeLeashData` (`:85-87`) over `storeNullable`, which writes
/// nothing for a null `LeashData`.
pub(crate) fn write_leash_data(
    entries: &mut Vec<NbtSummaryEntry>,
    data: Option<&LeashData>,
) -> Result<(), EntitySaveError> {
    let Some(data) = data else {
        return Ok(());
    };
    // `:374-377`: a holder encodes through its live attachment, and a list that
    // has neither is the encoder's `Objects.requireNonNull` failure.
    let attachment = if data.leash_holder.is_some() {
        data.holder_attachment
    } else {
        data.delayed_leash_info
    };
    let attachment = attachment.ok_or_else(|| field_error("had no attachment"))?;
    entries.push(NbtSummaryEntry {
        name: LEASH_TAG.into(),
        value: match attachment {
            LeashAttachment::Holder(uuid) => NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "UUID".into(),
                value: uuid_int_array(uuid),
            }]),
            LeashAttachment::Knot(pos) => NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
        },
    });
    Ok(())
}

fn field_error(detail: &str) -> EntitySaveError {
    EntitySaveError::Field {
        field: "leash",
        detail: detail.to_string(),
    }
}

fn attachment_of(holder: &Entity) -> LeashAttachment {
    match holder.data() {
        crate::entity::EntityData::LeashKnot(knot) => LeashAttachment::Knot(knot.attached()),
        _ => LeashAttachment::Holder(holder.uuid()),
    }
}

fn supports_quad_leash_as_holder(holder: &Entity) -> bool {
    matches!(
        holder.data().entity_resource_id(),
        "minecraft:ghast" | "minecraft:happy_ghast"
    )
}

///
/// `None` is the refusal for a controlling session whose facts this level
/// cannot answer.
fn holder_movement(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Vec3d> {
    if matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_no_ai()) {
        return Some(Vec3d::default());
    }
    if let Some(controller_id) = store
        .first_external_player_passenger(entity.id())
        .filter(|_| alive_for_leash(entity))
    {
        return level
            .leash_holder_player(controller_id)
            .map(|player| player.delta_movement);
    }
    Some(entity.delta_movement())
}

/// The holder facts `tickLeash` reads while the leashed entity is borrowed out
/// of the store.
#[derive(Debug, Clone, Copy)]
struct LeashHolder {
    id: i32,
    position: Vec3d,
    center: Vec3d,
    width: f32,
    height: f32,
    y_rot: f32,
    supports_quad_leash: bool,
    known_movement: Option<Vec3d>,
}

impl LeashHolder {
    fn of_entity(holder: &Entity) -> Self {
        Self {
            id: holder.id(),
            position: holder.position(),
            center: holder.bounding_box().center(),
            width: holder.bb_width(),
            height: holder.bb_height(),
            y_rot: holder.y_rot(),
            supports_quad_leash: supports_quad_leash_as_holder(holder),
            known_movement: None,
        }
    }

    fn of_player(player: &NearbyPlayer) -> Self {
        let [width, height] = player.dimensions;
        Self {
            id: player.entity_id,
            position: player.position,
            center: Vec3d {
                x: player.position.x,
                y: player.position.y + f64::from(height) / 2.0,
                z: player.position.z,
            },
            width,
            height,
            y_rot: player.y_rot,
            supports_quad_leash: false,
            known_movement: Some(player.delta_movement),
        }
    }
}

/// What `tickLeash`'s two holder reads (`:148`, `:156-157`) see. `OffLevel`
/// is a session this level cannot name: its `canInteractWithLevel` is still
/// true and its `level()` is another one, so the leash holds without a pull.
enum ResolvedLeashHolder {
    Interactable(LeashHolder),
    NotInteractable,
    OffLevel,
}

fn resolve_holder(
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    holder_id: i32,
    holder_is_player: bool,
) -> ResolvedLeashHolder {
    if let Some(holder) = store.get(holder_id) {
        return if alive_for_leash(holder) {
            ResolvedLeashHolder::Interactable(LeashHolder::of_entity(holder))
        } else {
            ResolvedLeashHolder::NotInteractable
        };
    }
    if let Some(player) = level.leash_holder_player(holder_id) {
        return if !player.dead_or_dying && !player.spectator {
            ResolvedLeashHolder::Interactable(LeashHolder::of_player(&player))
        } else {
            ResolvedLeashHolder::NotInteractable
        };
    }
    if holder_is_player {
        ResolvedLeashHolder::OffLevel
    } else {
        ResolvedLeashHolder::NotInteractable
    }
}

pub(crate) fn tick_leash(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    // `:142-145`.
    if leash_data(entity).is_some_and(|data| data.delayed_leash_info.is_some())
        && !restore_leash_from_save(entity, store, level)
    {
        return false;
    }
    // `:147`: the restore updates the same `LeashData` in place, so reading it
    // again names the object vanilla's local reference names.
    let Some((holder_id, holder_is_player)) =
        leash_data(entity).and_then(|data| Some((data.leash_holder?, data.holder_is_player)))
    else {
        return true;
    };
    // `:148-154`.
    let resolved = resolve_holder(store, level, holder_id, holder_is_player);
    if !alive_for_leash(entity) || matches!(resolved, ResolvedLeashHolder::NotInteractable) {
        let drop_lead = level.entity_drops();
        if !drop_leash(entity, store, level, true, drop_lead) {
            return false;
        }
    }
    // `:156-157`: the pull needs a holder this level can name.
    if leash_data(entity)
        .and_then(|data| data.leash_holder)
        .is_none()
    {
        return true;
    }
    let ResolvedLeashHolder::Interactable(holder) = resolved else {
        return true;
    };
    let elastic_distance = leash_elastic_distance(entity);
    // PathfinderMob.java:70-72.
    if is_instance_of(entity.entity_type_id(), "PathfinderMob") {
        entity.set_mob_home(
            BlockPos {
                x: holder.position.x.floor() as i32,
                y: holder.position.y.floor() as i32,
                z: holder.position.z.floor() as i32,
            },
            elastic_distance as i32 - 1,
        );
    }
    let angular_momentum = leash_data(entity).map_or(0.0, |data| data.angular_momentum);
    // `:158`, `Leashable.leashDistanceTo` (`:63-65`).
    let distance_to = distance_to(holder.center, entity.bounding_box().center());
    if supports_quad_leash(entity) {
        if let Some(crate::entity::EntityData::GeneralLiving(data)) =
            store.get_mut(holder.id).map(|holder| &mut holder.data)
        {
            data.note_leash_holder();
        }
    }
    if distance_to > leash_snap_distance(entity) {
        // `:161`.
        store.play_entity_sound(PendingEntitySound {
            position: holder.position,
            resource_id: LEAD_BREAK_SOUND.to_owned(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch: 1.0,
        });
        // `:162`: `AbstractBoat` keeps `Leashable.leashTooFarBehaviour`
        // (`:201-203`).
        if !drop_leash(entity, store, level, true, true) {
            return false;
        }
    } else if distance_to
        > elastic_distance - f64::from(holder.width) - f64::from(entity.bb_width())
    {
        // `:163-165`.
        match check_elastic_interactions(entity, store, level, holder) {
            None => return false,
            Some(true) => {
                if crate::entity::camel::stand_up_if_ready(entity, store, level).is_none() {
                    return false;
                }
                on_elastic_leash_pull(entity);
            }
            Some(false) => {
                if !close_range_navigation(entity, store, level, holder.position) {
                    return false;
                }
            }
        }
    } else if !close_range_navigation(entity, store, level, holder.position) {
        return false;
    }
    // `:170-171`, which run on the local `LeashData` reference even when the
    // branch above detached it from the entity.
    let angular_momentum =
        leash_data(entity).map_or(angular_momentum, |data| data.angular_momentum);
    let angular_friction = angular_friction(entity, level);
    entity.set_rot(
        (f64::from(entity.y_rot()) - angular_momentum) as f32,
        entity.x_rot(),
    );
    if let Some(data) = leash_data_mut(entity) {
        data.angular_momentum = angular_momentum * f64::from(angular_friction);
    }
    true
}

fn close_range_navigation(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    holder_position: Vec3d,
) -> bool {
    // Allay.java:443-445; HappyGhast.java:192-194.
    if !is_instance_of(entity.entity_type_id(), "PathfinderMob")
        || matches!(
            entity.data().entity_resource_id(),
            "minecraft:allay" | "minecraft:happy_ghast"
        )
    {
        return true;
    }
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return true;
    };
    // PathfinderMob.java:58-65.
    if runtime.is_panicking() {
        return true;
    }
    let has_target = runtime.has_target();
    let Some(malus) = store.effective_pathfinding_malus(entity, level) else {
        return false;
    };
    let Some(mob) = MobPathingSnapshot::from_entity(
        entity,
        has_target,
        store.vehicle(entity.id()).is_some(),
        store.difficulty(),
        malus,
    ) else {
        return false;
    };
    let delta = minus(holder_position, entity.position());
    let wanted = entity.position().plus(
        delta
            .normalize()
            .scale((distance_to(holder_position, entity.position()) - 2.0).max(0.0)),
    );
    // Llama.java:412-413.
    let speed = if matches!(
        entity.data().entity_resource_id(),
        "minecraft:llama" | "minecraft:trader_llama"
    ) {
        2.0
    } else {
        1.0
    };
    if let Some(runtime) = store.mob_ai_mut(entity.id()) {
        runtime.navigate_for_leash(
            level,
            mob,
            BlockPos {
                x: wanted.x.floor() as i32,
                y: wanted.y.floor() as i32,
                z: wanted.z.floor() as i32,
            },
            speed,
        );
    }
    true
}

fn distance_to(a: Vec3d, b: Vec3d) -> f64 {
    a.distance_to_sqr(b).sqrt()
}

/// `Leashable.angularFriction` (`:189-195`).
fn angular_friction(entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    if entity.on_ground() {
        level.block_friction(entity.block_pos_below_that_affects_my_movement()) * 0.91
    } else if entity.is_in_water() || entity.is_in_lava() {
        0.8
    } else {
        0.91
    }
}

fn on_elastic_leash_pull(entity: &mut Entity) {
    if entity.delta_movement().y > -0.5 && entity.fall_distance() > 1.0 {
        entity.set_fall_distance(1.0);
    }
}

/// `Leashable.restoreLeashFromSave` (`:89-109`).
fn restore_leash_from_save(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let Some(delayed) = leash_data(entity).and_then(|data| data.delayed_leash_info) else {
        return true;
    };
    match delayed {
        // `:93-98`: `ServerLevel.getEntity` sees this level's entities and
        // sessions, so a player in another level stays unresolved and times
        // the lead out, as vanilla's does.
        LeashAttachment::Holder(uuid) => {
            if let Some(holder_id) = store.visible_entity_id_for_uuid(uuid) {
                return set_leashed_to(entity, store, holder_id, true);
            }
            if let Some(player) = level.player(uuid) {
                return set_leashed_to_with(
                    entity,
                    store,
                    player.entity_id,
                    LeashAttachment::Holder(uuid),
                    true,
                    true,
                );
            }
        }
        LeashAttachment::Knot(pos) => {
            let Some(knot_id) = get_or_create_knot(store, pos) else {
                return false;
            };
            return set_leashed_to(entity, store, knot_id, true);
        }
    }
    // `:104-107`.
    if entity.tick_count() > DELAYED_RESTORE_TICKS {
        if !spawn_lead_at_location(entity, level) {
            return false;
        }
        set_leash_data(entity, None);
    }
    true
}

fn find_knot(store: &EntityStore, pos: BlockPos) -> Option<i32> {
    let area = Aabb::of_block(pos).inflate(1.0, 1.0, 1.0);
    store.ids_in_box_sections(area).into_iter().find_map(|id| {
        let entity = store.get(id)?;
        matches!(
            entity.data(),
            crate::entity::EntityData::LeashKnot(knot) if knot.attached() == pos
        )
        .then_some(id)
    })
}

/// `getKnot` over `createKnot` (`:144-149`).
fn get_or_create_knot(store: &mut EntityStore, pos: BlockPos) -> Option<i32> {
    match find_knot(store, pos) {
        Some(id) => Some(id),
        None => crate::entity::hanging::spawn_leash_knot(store, pos),
    }
}

fn lead_item_id() -> Option<i32> {
    vanilla_item_type_for_resource_id(LEAD_ITEM).map(|item| item.id)
}

fn spawn_lead_at_location(entity: &Entity, level: &mut LevelCtx<'_, '_>) -> bool {
    let Some(item_id) = lead_item_id() else {
        return false;
    };
    level.spawn_item(
        entity.position(),
        &ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    true
}

/// `Leashable.setLeashedTo` (`:302-328`) for a holder this store holds.
pub(crate) fn set_leashed_to(
    entity: &mut Entity,
    store: &mut EntityStore,
    holder_id: i32,
    synch: bool,
) -> bool {
    let Some(attachment) = store.get(holder_id).map(attachment_of) else {
        return false;
    };
    set_leashed_to_with(entity, store, holder_id, attachment, false, synch)
}

/// The body of `setLeashedTo` (`:308-328`) with the holder's identity already
/// resolved, which is how a session holder outside the store is named.
pub(crate) fn set_leashed_to_with(
    entity: &mut Entity,
    store: &mut EntityStore,
    holder_id: i32,
    attachment: LeashAttachment,
    holder_is_player: bool,
    synch: bool,
) -> bool {
    if !supports_leash(entity) {
        return false;
    }
    // `:303`.
    if entity.id() == holder_id {
        return true;
    }
    // `:309-319`.
    let previous_holder = match leash_data_mut(entity) {
        None => {
            set_leash_data(
                entity,
                Some(LeashData::held(holder_id, attachment, holder_is_player)),
            );
            None
        }
        Some(data) => {
            let previous = data.leash_holder;
            data.set_leash_holder(holder_id, attachment, holder_is_player);
            previous
        }
    };
    if let Some(previous) = previous_holder.filter(|previous| *previous != holder_id) {
        notify_leashee_removed(store, previous, entity.id());
    }
    // `:321-323`.
    if synch {
        let entity_id = entity.id();
        store.broadcast_to_tracking(
            entity_id,
            ClientboundPlayPacket::SetEntityLink(SetEntityLink {
                source_id: entity_id,
                dest_id: holder_id,
            }),
        );
    }
    // `Entity.removePassenger` sets the detached passenger cooldown (`Entity.java:2449`).
    if store.stop_invalid_riding(entity.id()) {
        entity.set_boarding_cooldown(60);
    }
    true
}

/// `Leashable.dropLeash(E, boolean, boolean)` (`:122-139`); `dropLeash()` is
/// `(true, true)` and `removeLeash()` is `(true, false)` (`:111-117`).
pub(crate) fn drop_leash(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    send_packet: bool,
    drop_lead: bool,
) -> bool {
    let Some(holder_id) = leash_data(entity).and_then(|data| data.leash_holder) else {
        return true;
    };
    if drop_lead && lead_item_id().is_none() {
        return false;
    }
    set_leash_data(entity, None);
    if drop_lead {
        spawn_lead_at_location(entity, level);
    }
    finish_drop_leash(entity, store, holder_id, send_packet);
    true
}

pub(crate) fn drop_leash_store_spawn(
    entity: &mut Entity,
    store: &mut EntityStore,
    send_packet: bool,
    drop_lead: bool,
) -> bool {
    let Some(holder_id) = leash_data(entity).and_then(|data| data.leash_holder) else {
        return true;
    };
    let lead = if drop_lead {
        match lead_stack() {
            Some(lead) => Some(lead),
            None => return false,
        }
    } else {
        None
    };
    set_leash_data(entity, None);
    if let Some(lead) = lead {
        crate::entity::item::spawn_dropped_item_entity(store, entity.position(), lead);
    }
    finish_drop_leash(entity, store, holder_id, send_packet);
    true
}

/// `:132-136`, shared by both spawn paths.
fn finish_drop_leash(
    entity: &mut Entity,
    store: &mut EntityStore,
    holder_id: i32,
    send_packet: bool,
) {
    if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.home_radius = -1;
        }
    }
    if send_packet {
        let entity_id = entity.id();
        store.broadcast_to_tracking(
            entity_id,
            ClientboundPlayPacket::SetEntityLink(SetEntityLink {
                source_id: entity_id,
                dest_id: 0,
            }),
        );
    }
    notify_leashee_removed(store, holder_id, entity.id());
}

fn lead_stack() -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
    let summary = ItemStackSummary {
        item_id: Some(lead_item_id()?),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    Some(bbb_server_types::item_stack::RuntimeItemStack::from_summary(summary))
}

pub(crate) fn drop_leash_on_destroy(entity: &mut Entity, store: &mut EntityStore) {
    if is_leashed(entity) {
        drop_leash_store_spawn(entity, store, true, true);
    }
}

pub fn drop_leashes_held_by_departed_player(
    store: &mut EntityStore,
    player_entity_id: i32,
    entity_drops: bool,
) -> bool {
    let held: Vec<i32> = store
        .bodies()
        .filter(|entity| {
            leash_data(entity).and_then(|data| data.leash_holder) == Some(player_entity_id)
        })
        .map(Entity::id)
        .collect();
    if held.is_empty() {
        return false;
    }
    for id in held {
        let Some(mut entity) = store.detach(id) else {
            continue;
        };
        drop_leash_store_spawn(&mut entity, store, true, entity_drops);
        store.attach(entity);
    }
    true
}

fn notify_leashee_removed(store: &mut EntityStore, holder_id: i32, removed_id: i32) {
    let is_knot = store
        .get(holder_id)
        .is_some_and(|holder| matches!(holder.data(), crate::entity::EntityData::LeashKnot(_)));
    if !is_knot {
        return;
    }
    // `Leashable.leashableLeashedTo` (`:349-355`) over `leashableInArea`
    // (`:357-364`): a 32-block box centred on the knot.
    let Some(scan_area) = store
        .get(holder_id)
        .map(|holder| aabb_of_size(holder.bounding_box().center(), 32.0))
    else {
        return;
    };
    let still_leashed = store.ids_in_box_sections(scan_area).into_iter().any(|id| {
        id != removed_id
            && store.get(id).is_some_and(|entity| {
                scan_area.intersects(entity.bounding_box())
                    && leash_data(entity).and_then(|data| data.leash_holder) == Some(holder_id)
            })
    });
    if !still_leashed {
        store.remove(holder_id, crate::entity::RemovalReason::Discarded);
    }
}

/// `Leashable.checkElasticInteractions` (`:208-225`): `Some(true)` when the
/// spring pulled, `None` when a movement this level cannot answer is needed.
fn check_elastic_interactions(
    entity: &mut Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    holder: LeashHolder,
) -> Option<bool> {
    let quad_connection = holder.supports_quad_leash && supports_quad_leash(entity);
    let (entity_points, leasher_points): (&[Vec3d], &[Vec3d]) = if quad_connection {
        (
            &SHARED_QUAD_ATTACHMENT_POINTS,
            &SHARED_QUAD_ATTACHMENT_POINTS,
        )
    } else {
        (&ENTITY_ATTACHMENT_POINT, &LEASHER_ATTACHMENT_POINT)
    };
    // `:222` reads the holder's movement and `:235` the boat's own.
    let leasher_movement = match holder.known_movement {
        Some(movement) => movement,
        None => holder_movement(store.get(holder.id)?, store, level)?,
    };
    let known_movement = holder_movement(entity, store, level)?;
    let wrenches = compute_elastic_interaction(
        entity,
        holder,
        entity_points,
        leasher_points,
        known_movement,
    );
    // `:216-218`.
    if wrenches.is_empty() {
        return Some(false);
    }
    // `:220-223`.
    let result = Wrench::accumulate(&wrenches).scale(if quad_connection { 0.25 } else { 1.0 });
    if let Some(data) = leash_data_mut(entity) {
        data.angular_momentum += TORSIONAL_ELASTICITY * result.torque;
    }
    let relative_velocity = minus(leasher_movement, known_movement);
    let delta = entity.delta_movement().plus(
        result
            .force
            .multiply(
                AXIS_SPECIFIC_ELASTICITY.x,
                AXIS_SPECIFIC_ELASTICITY.y,
                AXIS_SPECIFIC_ELASTICITY.z,
            )
            .plus(relative_velocity.scale(STIFFNESS)),
    );
    entity.set_delta_movement(delta);
    Some(true)
}

/// `Leashable.computeElasticInteraction` (`:231-251`).
fn compute_elastic_interaction(
    entity: &Entity,
    holder: LeashHolder,
    entity_points: &[Vec3d],
    leasher_points: &[Vec3d],
    known_movement: Vec3d,
) -> Vec<Wrench> {
    let entity_y_rot = entity.y_rot() * DEGREES_TO_RADIANS;
    let entity_dimensions = Vec3d {
        x: f64::from(entity.bb_width()),
        y: f64::from(entity.bb_height()),
        z: f64::from(entity.bb_width()),
    };
    let holder_y_rot = holder.y_rot * DEGREES_TO_RADIANS;
    let holder_dimensions = Vec3d {
        x: f64::from(holder.width),
        y: f64::from(holder.height),
        z: f64::from(holder.width),
    };
    let mut wrenches = Vec::new();
    for (entity_point, leasher_point) in entity_points.iter().zip(leasher_points) {
        let entity_attach_vector = y_rot(
            entity_point.multiply(
                entity_dimensions.x,
                entity_dimensions.y,
                entity_dimensions.z,
            ),
            -entity_y_rot,
        );
        let entity_attach_pos = entity.position().plus(entity_attach_vector);
        let leasher_attach_vector = y_rot(
            leasher_point.multiply(
                holder_dimensions.x,
                holder_dimensions.y,
                holder_dimensions.z,
            ),
            -holder_y_rot,
        );
        let leasher_attach_pos = holder.position.plus(leasher_attach_vector);
        if let Some(wrench) = compute_dampened_spring_interaction(
            leasher_attach_pos,
            entity_attach_pos,
            leash_elastic_distance(entity),
            known_movement,
            entity_attach_vector,
        ) {
            wrenches.push(wrench);
        }
    }
    wrenches
}

/// `Leashable.computeDampenedSpringInteraction` (`:253-269`).
fn compute_dampened_spring_interaction(
    pivot_point: Vec3d,
    object_position: Vec3d,
    spring_slack: f64,
    object_motion: Vec3d,
    lever_arm: Vec3d,
) -> Option<Wrench> {
    let distance = distance_to(object_position, pivot_point);
    if distance < spring_slack {
        return None;
    }
    let displacement = minus(pivot_point, object_position)
        .normalize()
        .scale(distance - spring_slack);
    let torque = Wrench::torque_from_force(lever_arm, displacement);
    // `:263-266`: the torque comes off the undampened displacement.
    let same_direction_to_movement = dot(object_motion, displacement) >= 0.0;
    let displacement = if same_direction_to_movement {
        displacement.scale(f64::from(0.3_f32))
    } else {
        displacement
    };
    Some(Wrench {
        force: displacement,
        torque,
    })
}

/// `(float)(Math.PI / 180.0)` (`:236`).
const DEGREES_TO_RADIANS: f32 = (std::f64::consts::PI / 180.0) as f32;

/// `Leashable.Wrench` (`:404-435`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Wrench {
    force: Vec3d,
    torque: f64,
}

impl Wrench {
    /// `:407-409`.
    fn torque_from_force(lever_arm: Vec3d, force: Vec3d) -> f64 {
        lever_arm.z * force.x - lever_arm.x * force.z
    }

    /// `:411-430`.
    fn accumulate(wrenches: &[Self]) -> Self {
        let mut force = Vec3d::default();
        let mut torque = 0.0;
        for wrench in wrenches {
            force = force.plus(wrench.force);
            torque += wrench.torque;
        }
        Self { force, torque }
    }

    /// `:432-434`.
    fn scale(self, scale: f64) -> Self {
        Self {
            force: self.force.scale(scale),
            torque: self.torque * scale,
        }
    }
}

fn aabb_of_size(center: Vec3d, size: f64) -> bbb_server_types::geometry::Aabb {
    let half = size / 2.0;
    bbb_server_types::geometry::Aabb {
        min_x: center.x - half,
        min_y: center.y - half,
        min_z: center.z - half,
        max_x: center.x + half,
        max_y: center.y + half,
        max_z: center.z + half,
    }
}

fn minus(a: Vec3d, b: Vec3d) -> Vec3d {
    Vec3d {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

fn dot(a: Vec3d, b: Vec3d) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn y_rot(vector: Vec3d, radians: f32) -> Vec3d {
    let cos = f64::from(mth_cos(f64::from(radians)));
    let sin = f64::from(mth_sin(f64::from(radians)));
    Vec3d {
        x: vector.x * cos + vector.z * sin,
        y: vector.y,
        z: vector.z * cos - vector.x * sin,
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn supports_leash(entity: &Entity) -> bool {
    crate::entity::boat::boat(entity.data()).is_some()
        || matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if data.mob.is_some())
}

pub(crate) fn alive_for_leash(entity: &Entity) -> bool {
    entity.is_alive() && entity.living().is_none_or(|living| living.health() > 0.0)
}

pub(crate) fn transfer_conversion_leash(
    source: &mut Entity,
    store: &mut EntityStore,
    target_id: i32,
) {
    let Some(data) = leash_data(source)
        .copied()
        .filter(|data| data.leash_holder.is_some())
    else {
        return;
    };
    set_leash_data(source, None);
    if let (Some(holder), Some(attachment)) = (data.leash_holder, data.holder_attachment) {
        store.with_entity_mut(target_id, |target, store| {
            set_leashed_to_with(
                target,
                store,
                holder,
                attachment,
                data.holder_is_player,
                true,
            );
        });
    }
    if let Some(holder) = data.leash_holder {
        notify_leashee_removed(store, holder, source.id());
    }
}
