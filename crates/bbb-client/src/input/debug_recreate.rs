//! F3+I copy-recreate command (`KeyboardHandler.copyRecreateCommand`).
//!
//! With gamemaster permission and no Shift the tag comes from a server tag query. Otherwise the
//! client writes only what its world holds: the block entity data it received, or the entity's
//! motion, rotation, received synched data mapped through [`SYNCHED_DATA_NBT`], equipment,
//! effects, attributes, leash and passengers. Nothing the client never received is invented.

use super::debug_keys::{push_debug_feedback_chat_message, DebugKeyContext};
use super::*;
use bbb_client_world::{EntityState, TagQueryResponseState};
use bbb_protocol::entity_types::{
    vanilla_entity_resource_id_for_type_id, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
    VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
};
use bbb_protocol::packets::{
    BlockEntityTagQuery, EntityDataValueKind, EntityTagQuery, ItemStackSummary,
};
use bbb_protocol::{encode_nbt_summary, JavaString, NbtSummaryEntry, NbtSummaryValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugRecreateServerQueryRequest {
    BlockEntityTag {
        transaction_id: i32,
        pos: ProtocolBlockPos,
    },
    EntityTag {
        transaction_id: i32,
        entity_id: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugRecreateTarget {
    Block(BlockPos),
    Entity(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingDebugRecreateServerQuery {
    pub(super) transaction_id: i32,
    pub(super) client_level_generation: u64,
    pub(super) target: PendingDebugRecreateServerQueryTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum PendingDebugRecreateServerQueryTarget {
    Block {
        pos: BlockPos,
        description: String,
    },
    Entity {
        entity_id: i32,
        entity_uuid: uuid::Uuid,
        entity_type: String,
        position: [f64; 3],
    },
}

pub(super) struct DebugRecreateCopy {
    pub(super) command: String,
    feedback_message: &'static str,
}

impl ClientInputState {
    pub(crate) fn take_debug_recreate_server_query_requests(
        &mut self,
    ) -> Vec<DebugRecreateServerQueryRequest> {
        std::mem::take(&mut self.debug_recreate_server_query_requests)
    }

    pub(crate) fn take_debug_recreate_server_query_requests_for_level(
        &mut self,
        client_level_generation: u64,
    ) -> Vec<DebugRecreateServerQueryRequest> {
        if self
            .pending_debug_recreate_server_query
            .as_ref()
            .is_some_and(|pending| pending.client_level_generation != client_level_generation)
        {
            self.pending_debug_recreate_server_query = None;
            self.debug_recreate_server_query_requests.clear();
            return Vec::new();
        }
        self.take_debug_recreate_server_query_requests()
    }

    pub(crate) fn consume_debug_recreate_server_query_response(
        &mut self,
        world: &mut WorldStore,
        clipboard: &mut dyn DebugClipboard,
    ) -> bool {
        let Some(pending) = self.pending_debug_recreate_server_query.clone() else {
            return false;
        };
        if pending.client_level_generation != world.client_level_generation() {
            self.pending_debug_recreate_server_query = None;
            push_debug_feedback_chat_message(
                Some(world),
                "Debug recreate level changed before server response",
            );
            return true;
        }
        let Some(response) = world.last_tag_query().cloned() else {
            return false;
        };
        if response.transaction_id != pending.transaction_id {
            return false;
        }

        self.pending_debug_recreate_server_query = None;
        if let PendingDebugRecreateServerQueryTarget::Entity {
            entity_id,
            entity_uuid,
            ..
        } = &pending.target
        {
            if world
                .probe_entity(*entity_id)
                .is_none_or(|entity| entity.uuid != *entity_uuid)
            {
                push_debug_feedback_chat_message(
                    Some(world),
                    "Debug recreate target disappeared before server response",
                );
                return true;
            }
        }
        if !response.tag_present {
            push_debug_feedback_chat_message(Some(world), "Server returned no recreate data");
            return true;
        }
        let Ok(copy) = server_response_command(&pending.target, &response) else {
            push_debug_feedback_chat_message(
                Some(world),
                "Failed to decode server-side recreate data",
            );
            return true;
        };
        if clipboard.set_debug_clipboard_text(&copy.command) {
            push_debug_feedback_chat_message(Some(world), copy.feedback_message);
        }
        true
    }

    /// `copyRecreateCommand(hasPermission(COMMANDS_GAMEMASTER), !hasShiftDown())`
    pub(super) fn copy_debug_recreate_command(&mut self, cx: &mut DebugKeyContext<'_, '_, '_>) {
        let Some(world) = cx.world.as_deref_mut() else {
            return;
        };
        let add_nbt = world.local_player_has_gamemaster_permission();
        let Some(target) = debug_recreate_target(world) else {
            return;
        };
        if add_nbt && !self.shift_down() {
            self.debug_query_transaction_id = self.debug_query_transaction_id.saturating_add(1);
            let transaction_id = self.debug_query_transaction_id;
            if let Some(pending) = pending_server_query(world, target, transaction_id) {
                self.pending_debug_recreate_server_query = Some(pending);
                self.debug_recreate_server_query_requests
                    .push(server_query_request(target, transaction_id));
            }
            return;
        }
        let copy = match target {
            DebugRecreateTarget::Block(pos) => client_block_command(world, pos, add_nbt),
            DebugRecreateTarget::Entity(entity_id) => {
                client_entity_command(world, entity_id, add_nbt)
            }
        };
        if let (Some(copy), Some(clipboard)) = (copy, cx.clipboard.as_deref_mut()) {
            if clipboard.set_debug_clipboard_text(&copy.command) {
                push_debug_feedback_chat_message(Some(world), copy.feedback_message);
            }
        }
    }
}

pub(crate) fn queue_debug_recreate_server_query_request(
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    request: DebugRecreateServerQueryRequest,
) {
    match request {
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos,
        } => queue_block_entity_tag_query_command(
            counters,
            net_commands,
            BlockEntityTagQuery {
                transaction_id,
                pos,
            },
        ),
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        } => queue_entity_tag_query_command(
            counters,
            net_commands,
            EntityTagQuery {
                transaction_id,
                entity_id,
            },
        ),
    }
}

fn debug_recreate_target(world: &WorldStore) -> Option<DebugRecreateTarget> {
    if world.local_player_id().is_none() || world.local_player_has_reduced_debug_info() {
        return None;
    }
    let target = crosshair_target_from_camera(world, camera_pose_from_world(world))?;
    match target {
        CrosshairTarget::Block(hit) => Some(DebugRecreateTarget::Block(hit.pos)),
        CrosshairTarget::Entity(hit) => Some(DebugRecreateTarget::Entity(hit.entity_id)),
    }
}

fn server_query_request(
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> DebugRecreateServerQueryRequest {
    match target {
        DebugRecreateTarget::Block(pos) => DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id,
            pos,
        },
        DebugRecreateTarget::Entity(entity_id) => DebugRecreateServerQueryRequest::EntityTag {
            transaction_id,
            entity_id,
        },
    }
}

fn pending_server_query(
    world: &WorldStore,
    target: DebugRecreateTarget,
    transaction_id: i32,
) -> Option<PendingDebugRecreateServerQuery> {
    let target = match target {
        DebugRecreateTarget::Block(pos) => PendingDebugRecreateServerQueryTarget::Block {
            pos,
            description: block_state_description(world, pos)?,
        },
        DebugRecreateTarget::Entity(entity_id) => {
            let entity = world.probe_entity(entity_id)?;
            let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
            PendingDebugRecreateServerQueryTarget::Entity {
                entity_id,
                entity_uuid: entity.uuid,
                entity_type: entity_type.to_string(),
                position: [entity.position.x, entity.position.y, entity.position.z],
            }
        }
    };
    Some(PendingDebugRecreateServerQuery {
        transaction_id,
        client_level_generation: world.client_level_generation(),
        target,
    })
}

fn server_response_command(
    target: &PendingDebugRecreateServerQueryTarget,
    response: &TagQueryResponseState,
) -> bbb_client_world::Result<DebugRecreateCopy> {
    Ok(match target {
        PendingDebugRecreateServerQueryTarget::Block { pos, description } => DebugRecreateCopy {
            command: setblock_command(*pos, description, response.compact_snbt()?),
            feedback_message: "Copied server-side block data to clipboard",
        },
        PendingDebugRecreateServerQueryTarget::Entity {
            entity_type,
            position,
            ..
        } => DebugRecreateCopy {
            command: summon_command(
                entity_type,
                *position,
                response.pretty_snbt_without_root_keys(&["UUID", "Pos"])?,
            ),
            feedback_message: "Copied server-side entity data to clipboard",
        },
    })
}

fn client_block_command(
    world: &WorldStore,
    pos: BlockPos,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let description = block_state_description(world, pos)?;
    // The client block entity's `saveWithoutMetadata` is the data it received.
    let tag = add_nbt
        .then(|| world.block_entity_raw_nbt_at(pos))
        .flatten()
        .and_then(|raw_nbt| {
            TagQueryResponseState {
                transaction_id: 0,
                tag_present: raw_nbt.first().is_some_and(|tag_id| *tag_id != 0),
                raw_nbt: raw_nbt.to_vec(),
            }
            .compact_snbt()
            .ok()
            .flatten()
        });
    Some(DebugRecreateCopy {
        command: setblock_command(pos, &description, tag),
        feedback_message: "Copied client-side block data to clipboard",
    })
}

pub(super) fn client_entity_command(
    world: &WorldStore,
    entity_id: i32,
    add_nbt: bool,
) -> Option<DebugRecreateCopy> {
    let entity = world.probe_entity(entity_id)?;
    let entity_type = vanilla_entity_resource_id_for_type_id(entity.entity_type_id)?;
    let tag = add_nbt
        .then(|| pretty_snbt(client_entity_tag(world, &entity, None, 0)))
        .flatten();
    Some(DebugRecreateCopy {
        command: summon_command(
            &entity_type,
            [entity.position.x, entity.position.y, entity.position.z],
            tag,
        ),
        feedback_message: "Copied client-side entity data to clipboard",
    })
}

/// `copyCreateBlockCommand`.
fn setblock_command(pos: BlockPos, description: &str, tag: Option<String>) -> String {
    format!(
        "/setblock {} {} {} {description}{}",
        pos.x,
        pos.y,
        pos.z,
        tag.unwrap_or_default()
    )
}

/// `copyCreateEntityCommand`.
fn summon_command(entity_type: &str, position: [f64; 3], tag: Option<String>) -> String {
    let [x, y, z] = position;
    match tag {
        Some(tag) => format!("/summon {entity_type} {x:.2} {y:.2} {z:.2} {tag}"),
        None => format!("/summon {entity_type} {x:.2} {y:.2} {z:.2}"),
    }
}

/// `BlockStateParser.serialize`.
fn block_state_description(world: &WorldStore, pos: BlockPos) -> Option<String> {
    let block = world.probe_block(pos)?;
    let name = block.block_name.as_deref()?;
    if block.block_properties.is_empty() {
        return Some(name.to_string());
    }
    let properties = block
        .block_properties
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    Some(format!("{name}[{properties}]"))
}

/// `NbtUtils.toPrettyComponent(tag).getString()`, through the same formatter as server replies.
fn pretty_snbt(entries: Vec<NbtSummaryEntry>) -> Option<String> {
    TagQueryResponseState {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: encode_nbt_summary(&NbtSummaryValue::Compound(entries), usize::MAX).ok()?,
    }
    .pretty_snbt_without_root_keys(&[])
    .ok()
    .flatten()
}

const MAX_PASSENGER_DEPTH: usize = 16;

/// The client's counterpart of `Entity.saveWithoutId`. The copied entity
/// itself drops `Pos` and `UUID`; a passenger keeps them, with its vehicle's X and Z.
fn client_entity_tag(
    world: &WorldStore,
    entity: &EntityState,
    vehicle: Option<&EntityState>,
    depth: usize,
) -> Vec<NbtSummaryEntry> {
    let mut tag = Vec::new();
    if let Some(vehicle) = vehicle {
        let (x, z) = (vehicle.position.x, vehicle.position.z);
        put(&mut tag, "Pos", doubles(&[x, entity.position.y, z]));
    }
    let motion = entity.delta_movement;
    put(&mut tag, "Motion", doubles(&[motion.x, motion.y, motion.z]));
    put(&mut tag, "Rotation", floats(&[entity.y_rot, entity.x_rot]));
    if let Some(on_ground) = entity.on_ground {
        put(
            &mut tag,
            "OnGround",
            NbtSummaryValue::Byte(i8::from(on_ground)),
        );
    }
    if vehicle.is_some() {
        put(&mut tag, "UUID", uuid_nbt(entity.uuid));
    }

    let mut values = entity.data_values.iter().collect::<Vec<_>>();
    values.sort_by_key(|value| value.data_id);
    for value in values {
        let Some(accessor) =
            bbb_client_world::vanilla_entity_metadata_accessor_for_entity_type_and_data_id(
                entity.entity_type_id,
                value.data_id,
            )
        else {
            continue;
        };
        let owner = accessor.owner.rsplit('.').next().unwrap_or_default();
        for &(_, _, key, encoding, _) in SYNCHED_DATA_NBT.iter().filter(|row| {
            row.0 == owner
                && row.1 == accessor.java_name
                && (row.4.is_empty() || row.4.contains(&entity.entity_type_id))
        }) {
            if let Some(nbt) = synched_value_nbt(world, &value.value, encoding) {
                put(&mut tag, key, nbt);
            }
        }
    }

    if let Some(effects) = entity_effects_nbt(entity) {
        put(&mut tag, "active_effects", effects);
    }
    if let Some(attributes) = entity_attributes_nbt(entity) {
        put(&mut tag, "attributes", attributes);
    }
    let equipment = entity
        .equipment
        .iter()
        .filter_map(|update| {
            let slot = bbb_protocol::equipment_slots::vanilla_equipment_slot_by_ordinal(
                update.slot.ordinal(),
            )?;
            Some(entry(
                slot.serialized_name,
                item_stack_nbt(world, &update.item)?,
            ))
        })
        .collect::<Vec<_>>();
    if !equipment.is_empty() {
        put(&mut tag, "equipment", NbtSummaryValue::Compound(equipment));
    }
    if let Some(leash) = leash_nbt(world, entity) {
        put(&mut tag, "leash", leash);
    }
    if entity.entity_type_id == VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
        // A falling block's state arrives as the add-entity data.
        if let Some(state) = block_state_nbt(world, entity.data) {
            put(&mut tag, "BlockState", state);
        }
    }
    if let Some(owner) = entity
        .projectile_lifecycle
        .and_then(|projectile| projectile.owner_uuid)
    {
        put(&mut tag, "Owner", uuid_nbt(owner));
    }
    if let Some(projectile) = entity.hurting_projectile {
        put(
            &mut tag,
            "acceleration_power",
            double(projectile.acceleration_power),
        );
    }

    if depth < MAX_PASSENGER_DEPTH {
        let passengers = entity
            .passengers
            .iter()
            .filter_map(|id| {
                let passenger = world.probe_entity(*id)?;
                let passenger_type =
                    vanilla_entity_resource_id_for_type_id(passenger.entity_type_id)?;
                let mut passenger_tag = vec![entry("id", string(&passenger_type))];
                passenger_tag.extend(client_entity_tag(
                    world,
                    &passenger,
                    Some(entity),
                    depth + 1,
                ));
                Some(NbtSummaryValue::Compound(passenger_tag))
            })
            .collect::<Vec<_>>();
        if !passengers.is_empty() {
            put(&mut tag, "Passengers", NbtSummaryValue::List(passengers));
        }
    }
    tag
}

/// How one synched data value maps onto its saved NBT field.
#[derive(Clone, Copy)]
enum SynchedNbt {
    Bool,
    /// Written only when true.
    True,
    Byte,
    /// INT accessor saved as a byte tag.
    IntByte,
    Short,
    Int,
    /// Written only when positive.
    PositiveInt,
    /// INT accessor saved as `value - 1`.
    IntMinusOne,
    Long,
    Float,
    String,
    Text,
    Item,
    BlockState,
    BlockPos,
    Uuid,
    /// One UUID appended to a list shared with other rows of the same key.
    UuidList,
    /// `Direction.get3DDataValue` as a byte.
    Direction3d,
    /// `Direction.get2DDataValue` as a byte.
    Direction2d,
    /// Registry holder written as its key.
    Registry,
    /// Enum id written as its serialized name.
    Names(&'static [&'static str]),
    Floats,
    VillagerData,
    /// `(flags & mask) != 0`, written only when set.
    Flag(i8),
    /// `(flags & mask) != 0`, always written.
    FlagBool(i8),
    /// `flags & mask` as a byte.
    ByteMask(i8),
    /// One nibble of a packed light value, absent for the `-1` "no override" value.
    LightNibble(u8),
    /// `Display.TextDisplay.getAlign` from the style flags.
    TextAlign,
    /// `ResolvableProfile.CODEC`.
    Profile,
    /// `Mannequin.LAYERS_CODEC`: the model parts whose bit is clear.
    HiddenLayers,
    /// `Mannequin.POSE_CODEC`, which rejects poses outside `VALID_POSES`.
    MannequinPose,
    /// `1b` when the optional value is empty.
    Absent,
}

use SynchedNbt as N;

const ANY: &[i32] = &[];
const ARMOR_STAND_AND_FRAMES: &[i32] = &[
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
];
const ITEM_FRAMES: &[i32] = &[
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
];
const PAINTING: &[i32] = &[VANILLA_ENTITY_TYPE_PAINTING_ID];
const MANNEQUIN: &[i32] = &[VANILLA_ENTITY_TYPE_MANNEQUIN_ID];
const PANDA_GENES: &[&str] = &[
    "normal",
    "lazy",
    "worried",
    "playful",
    "brown",
    "weak",
    "aggressive",
];
const FALL_FLYING: i8 = 0x80_u8 as i8;

/// Synched data accessors (owner class, Java field) whose value Vanilla saves, with the saved key
/// and the entity types the row is limited to (empty: every type with the accessor). `Parent/Child`
/// keys land in a nested compound. Keys and encodings follow each owner's `addAdditionalSaveData`.
#[rustfmt::skip]
const SYNCHED_DATA_NBT: &[(&str, &str, &str, SynchedNbt, &[i32])] = &[
    // Entity.saveWithoutId
    ("Entity", "DATA_AIR_SUPPLY_ID", "Air", N::Short, ANY),
    ("Entity", "DATA_CUSTOM_NAME", "CustomName", N::Text, ANY),
    ("Entity", "DATA_CUSTOM_NAME_VISIBLE", "CustomNameVisible", N::True, ANY),
    ("Entity", "DATA_SILENT", "Silent", N::True, ANY),
    ("Entity", "DATA_NO_GRAVITY", "NoGravity", N::True, ANY),
    ("Entity", "DATA_SHARED_FLAGS_ID", "Glowing", N::Flag(0x40), ANY),
    ("Entity", "DATA_SHARED_FLAGS_ID", "FallFlying", N::Flag(FALL_FLYING), ANY),
    ("Entity", "DATA_SHARED_FLAGS_ID", "Invisible", N::FlagBool(0x20), ARMOR_STAND_AND_FRAMES),
    ("Entity", "DATA_TICKS_FROZEN", "TicksFrozen", N::PositiveInt, ANY),
    ("LivingEntity", "DATA_HEALTH_ID", "Health", N::Float, ANY),
    ("LivingEntity", "SLEEPING_POS_ID", "sleeping_pos", N::BlockPos, ANY),
    ("Player", "DATA_PLAYER_ABSORPTION_ID", "AbsorptionAmount", N::Float, ANY),
    ("Player", "DATA_SCORE_ID", "Score", N::Int, ANY),
    ("Mob", "DATA_MOB_FLAGS_ID", "LeftHanded", N::FlagBool(0x02), ANY),
    ("Mob", "DATA_MOB_FLAGS_ID", "NoAI", N::Flag(0x01), ANY),
    ("AgeableMob", "AGE_LOCKED", "AgeLocked", N::Bool, ANY),
    ("Tadpole", "AGE_LOCKED", "AgeLocked", N::Bool, ANY),
    ("TamableAnimal", "DATA_FLAGS_ID", "Sitting", N::FlagBool(0x01), ANY),
    ("TamableAnimal", "DATA_OWNERUUID_ID", "Owner", N::Uuid, ANY),
    ("AbstractHorse", "DATA_ID_FLAGS", "EatingHaystack", N::FlagBool(0x10), ANY),
    ("AbstractHorse", "DATA_ID_FLAGS", "Bred", N::FlagBool(0x08), ANY),
    ("AbstractHorse", "DATA_ID_FLAGS", "Tame", N::FlagBool(0x02), ANY),
    ("AbstractChestedHorse", "DATA_ID_CHEST", "ChestedHorse", N::Bool, ANY),
    ("Horse", "DATA_ID_TYPE_VARIANT", "Variant", N::Int, ANY),
    ("Llama", "DATA_VARIANT_ID", "Variant", N::Int, ANY),
    ("Llama", "DATA_STRENGTH_ID", "Strength", N::Int, ANY),
    ("Axolotl", "DATA_VARIANT", "Variant", N::Int, ANY),
    ("Axolotl", "FROM_BUCKET", "FromBucket", N::Bool, ANY),
    ("AbstractFish", "FROM_BUCKET", "FromBucket", N::Bool, ANY),
    ("Pufferfish", "PUFF_STATE", "PuffState", N::Int, ANY),
    ("Salmon", "DATA_TYPE", "type", N::Names(&["small", "medium", "large"]), ANY),
    ("TropicalFish", "DATA_ID_TYPE_VARIANT", "Variant", N::Int, ANY),
    ("Bee", "DATA_FLAGS_ID", "HasNectar", N::FlagBool(0x08), ANY),
    ("Bee", "DATA_FLAGS_ID", "HasStung", N::FlagBool(0x04), ANY),
    ("Bee", "DATA_ANGER_END_TIME", "anger_end_time", N::Long, ANY),
    ("Camel", "LAST_POSE_CHANGE_TICK", "LastPoseTick", N::Long, ANY),
    ("Chicken", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Chicken", "DATA_SOUND_VARIANT_ID", "sound_variant", N::Registry, ANY),
    ("Cow", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Cow", "DATA_SOUND_VARIANT_ID", "sound_variant", N::Registry, ANY),
    ("MushroomCow", "DATA_TYPE", "Type", N::Names(&["red", "brown"]), ANY),
    ("Pig", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Pig", "DATA_SOUND_VARIANT_ID", "sound_variant", N::Registry, ANY),
    ("Cat", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Cat", "DATA_SOUND_VARIANT_ID", "sound_variant", N::Registry, ANY),
    ("Cat", "DATA_COLLAR_COLOR", "CollarColor", N::IntByte, ANY),
    ("Ocelot", "DATA_TRUSTING", "Trusting", N::Bool, ANY),
    ("Wolf", "DATA_COLLAR_COLOR", "CollarColor", N::IntByte, ANY),
    ("Wolf", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Wolf", "DATA_ANGER_END_TIME", "anger_end_time", N::Long, ANY),
    ("Wolf", "DATA_SOUND_VARIANT_ID", "sound_variant", N::Registry, ANY),
    ("Frog", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("ZombieNautilus", "DATA_VARIANT_ID", "variant", N::Registry, ANY),
    ("Parrot", "DATA_VARIANT_ID", "Variant", N::Int, ANY),
    ("Rabbit", "DATA_TYPE_ID", "RabbitType", N::Int, ANY),
    ("Sheep", "DATA_WOOL_ID", "Sheared", N::FlagBool(0x10), ANY),
    ("Sheep", "DATA_WOOL_ID", "Color", N::ByteMask(0x0F), ANY),
    ("Dolphin", "GOT_FISH", "GotFish", N::Bool, ANY),
    ("Dolphin", "MOISTNESS_LEVEL", "Moistness", N::Int, ANY),
    ("Fox", "DATA_TRUSTED_ID_0", "Trusted", N::UuidList, ANY),
    ("Fox", "DATA_TRUSTED_ID_1", "Trusted", N::UuidList, ANY),
    ("Fox", "DATA_FLAGS_ID", "Sleeping", N::FlagBool(0x20), ANY),
    ("Fox", "DATA_FLAGS_ID", "Sitting", N::FlagBool(0x01), ANY),
    ("Fox", "DATA_FLAGS_ID", "Crouching", N::FlagBool(0x04), ANY),
    ("Fox", "DATA_TYPE_ID", "Type", N::Names(&["red", "snow"]), ANY),
    ("Armadillo", "ARMADILLO_STATE", "state", N::Names(&["idle", "rolling", "scared", "unrolling"]), ANY),
    ("CopperGolem", "DATA_WEATHER_STATE", "weather_state", N::Names(&["unaffected", "exposed", "weathered", "oxidized"]), ANY),
    ("IronGolem", "DATA_FLAGS_ID", "PlayerCreated", N::FlagBool(0x01), ANY),
    ("SnowGolem", "DATA_PUMPKIN_ID", "Pumpkin", N::FlagBool(0x10), ANY),
    ("Goat", "DATA_IS_SCREAMING_GOAT", "IsScreamingGoat", N::Bool, ANY),
    ("Goat", "DATA_HAS_LEFT_HORN", "HasLeftHorn", N::Bool, ANY),
    ("Goat", "DATA_HAS_RIGHT_HORN", "HasRightHorn", N::Bool, ANY),
    ("Panda", "MAIN_GENE_ID", "MainGene", N::Names(PANDA_GENES), ANY),
    ("Panda", "HIDDEN_GENE_ID", "HiddenGene", N::Names(PANDA_GENES), ANY),
    ("GlowSquid", "DATA_DARK_TICKS_REMAINING", "DarkTicksRemaining", N::Int, ANY),
    ("Turtle", "HAS_EGG", "has_egg", N::Bool, ANY),
    ("Creeper", "DATA_IS_POWERED", "powered", N::Bool, ANY),
    ("Creeper", "DATA_IS_IGNITED", "ignited", N::Bool, ANY),
    ("EnderMan", "DATA_CARRY_STATE", "carriedBlockState", N::BlockState, ANY),
    ("Phantom", "ID_SIZE", "size", N::Int, ANY),
    ("Shulker", "DATA_ATTACH_FACE_ID", "AttachFace", N::Direction3d, ANY),
    ("Shulker", "DATA_PEEK_ID", "Peek", N::Byte, ANY),
    ("Shulker", "DATA_COLOR_ID", "Color", N::Byte, ANY),
    ("Slime", "ID_SIZE", "Size", N::IntMinusOne, ANY),
    ("Zoglin", "DATA_BABY_ID", "IsBaby", N::Bool, ANY),
    ("Piglin", "DATA_BABY_ID", "IsBaby", N::Bool, ANY),
    ("Zombie", "DATA_BABY_ID", "IsBaby", N::Bool, ANY),
    ("AbstractPiglin", "DATA_IMMUNE_TO_ZOMBIFICATION", "IsImmuneToZombification", N::Bool, ANY),
    ("Hoglin", "DATA_IMMUNE_TO_ZOMBIFICATION", "IsImmuneToZombification", N::Bool, ANY),
    ("Bogged", "DATA_SHEARED", "sheared", N::Bool, ANY),
    ("Creaking", "HOME_POS", "home_pos", N::BlockPos, ANY),
    ("ZombieVillager", "DATA_VILLAGER_DATA", "VillagerData", N::VillagerData, ANY),
    ("ZombieVillager", "DATA_VILLAGER_DATA_FINALIZED", "VillagerDataFinalized", N::Bool, ANY),
    ("Villager", "DATA_VILLAGER_DATA", "VillagerData", N::VillagerData, ANY),
    ("Villager", "DATA_VILLAGER_DATA_FINALIZED", "VillagerDataFinalized", N::Bool, ANY),
    ("Bat", "DATA_ID_FLAGS", "BatFlags", N::Byte, ANY),
    ("EnderDragon", "DATA_PHASE", "DragonPhase", N::Int, ANY),
    ("WitherBoss", "DATA_ID_INV", "Invul", N::Int, ANY),
    ("EndCrystal", "DATA_BEAM_TARGET", "beam_target", N::BlockPos, ANY),
    ("EndCrystal", "DATA_SHOW_BOTTOM", "ShowBottom", N::Bool, ANY),
    ("ArmorStand", "DATA_CLIENT_FLAGS", "Small", N::FlagBool(0x01), ANY),
    ("ArmorStand", "DATA_CLIENT_FLAGS", "ShowArms", N::FlagBool(0x04), ANY),
    ("ArmorStand", "DATA_CLIENT_FLAGS", "NoBasePlate", N::FlagBool(0x08), ANY),
    ("ArmorStand", "DATA_CLIENT_FLAGS", "Marker", N::Flag(0x10), ANY),
    ("ArmorStand", "DATA_HEAD_POSE", "Pose/Head", N::Floats, ANY),
    ("ArmorStand", "DATA_BODY_POSE", "Pose/Body", N::Floats, ANY),
    ("ArmorStand", "DATA_LEFT_ARM_POSE", "Pose/LeftArm", N::Floats, ANY),
    ("ArmorStand", "DATA_RIGHT_ARM_POSE", "Pose/RightArm", N::Floats, ANY),
    ("ArmorStand", "DATA_LEFT_LEG_POSE", "Pose/LeftLeg", N::Floats, ANY),
    ("ArmorStand", "DATA_RIGHT_LEG_POSE", "Pose/RightLeg", N::Floats, ANY),
    ("HangingEntity", "DATA_DIRECTION", "Facing", N::Direction3d, ITEM_FRAMES),
    ("HangingEntity", "DATA_DIRECTION", "facing", N::Direction2d, PAINTING),
    ("ItemFrame", "DATA_ITEM", "Item", N::Item, ANY),
    ("ItemFrame", "DATA_ROTATION", "ItemRotation", N::IntByte, ANY),
    ("Painting", "DATA_PAINTING_VARIANT_ID", "variant", N::Registry, ANY),
    // Mannequin.addAdditionalSaveData
    ("Mannequin", "DATA_PROFILE", "profile", N::Profile, ANY),
    ("Avatar", "DATA_PLAYER_MODE_CUSTOMISATION", "hidden_layers", N::HiddenLayers, MANNEQUIN),
    ("Avatar", "DATA_PLAYER_MAIN_HAND", "main_hand", N::Names(&["left", "right"]), MANNEQUIN),
    ("Entity", "DATA_POSE", "pose", N::MannequinPose, MANNEQUIN),
    ("Mannequin", "DATA_IMMOVABLE", "immovable", N::Bool, ANY),
    ("Mannequin", "DATA_DESCRIPTION", "description", N::Text, ANY),
    ("Mannequin", "DATA_DESCRIPTION", "hide_description", N::Absent, ANY),
    ("ItemEntity", "DATA_ITEM", "Item", N::Item, ANY),
    ("PrimedTnt", "DATA_FUSE_ID", "fuse", N::Short, ANY),
    ("PrimedTnt", "DATA_BLOCK_STATE_ID", "block_state", N::BlockState, ANY),
    ("ExperienceOrb", "DATA_VALUE", "Value", N::Short, ANY),
    ("AreaEffectCloud", "DATA_RADIUS", "Radius", N::Float, ANY),
    ("OminousItemSpawner", "DATA_ITEM", "item", N::Item, ANY),
    ("Interaction", "DATA_WIDTH_ID", "width", N::Float, ANY),
    ("Interaction", "DATA_HEIGHT_ID", "height", N::Float, ANY),
    ("Interaction", "DATA_RESPONSE_ID", "response", N::Bool, ANY),
    ("Display", "DATA_TRANSFORMATION_INTERPOLATION_DURATION_ID", "interpolation_duration", N::Int, ANY),
    ("Display", "DATA_POS_ROT_INTERPOLATION_DURATION_ID", "teleport_duration", N::Int, ANY),
    ("Display", "DATA_TRANSLATION_ID", "transformation/translation", N::Floats, ANY),
    ("Display", "DATA_SCALE_ID", "transformation/scale", N::Floats, ANY),
    ("Display", "DATA_LEFT_ROTATION_ID", "transformation/left_rotation", N::Floats, ANY),
    ("Display", "DATA_RIGHT_ROTATION_ID", "transformation/right_rotation", N::Floats, ANY),
    ("Display", "DATA_BILLBOARD_RENDER_CONSTRAINTS_ID", "billboard", N::Names(&["fixed", "vertical", "horizontal", "center"]), ANY),
    ("Display", "DATA_BRIGHTNESS_OVERRIDE_ID", "brightness/block", N::LightNibble(4), ANY),
    ("Display", "DATA_BRIGHTNESS_OVERRIDE_ID", "brightness/sky", N::LightNibble(20), ANY),
    ("Display", "DATA_VIEW_RANGE_ID", "view_range", N::Float, ANY),
    ("Display", "DATA_SHADOW_RADIUS_ID", "shadow_radius", N::Float, ANY),
    ("Display", "DATA_SHADOW_STRENGTH_ID", "shadow_strength", N::Float, ANY),
    ("Display", "DATA_WIDTH_ID", "width", N::Float, ANY),
    ("Display", "DATA_HEIGHT_ID", "height", N::Float, ANY),
    ("Display", "DATA_GLOW_COLOR_OVERRIDE_ID", "glow_color_override", N::Int, ANY),
    ("BlockDisplay", "DATA_BLOCK_STATE_ID", "block_state", N::BlockState, ANY),
    ("ItemDisplay", "DATA_ITEM_STACK_ID", "item", N::Item, ANY),
    ("ItemDisplay", "DATA_ITEM_DISPLAY_ID", "item_display", N::Names(&["none", "thirdperson_lefthand", "thirdperson_righthand", "firstperson_lefthand", "firstperson_righthand", "head", "gui", "ground", "fixed", "on_shelf"]), ANY),
    ("TextDisplay", "DATA_TEXT_ID", "text", N::Text, ANY),
    ("TextDisplay", "DATA_LINE_WIDTH_ID", "line_width", N::Int, ANY),
    ("TextDisplay", "DATA_BACKGROUND_COLOR_ID", "background", N::Int, ANY),
    ("TextDisplay", "DATA_TEXT_OPACITY_ID", "text_opacity", N::Byte, ANY),
    ("TextDisplay", "DATA_STYLE_FLAGS_ID", "shadow", N::FlagBool(0x01), ANY),
    ("TextDisplay", "DATA_STYLE_FLAGS_ID", "see_through", N::FlagBool(0x02), ANY),
    ("TextDisplay", "DATA_STYLE_FLAGS_ID", "default_background", N::FlagBool(0x04), ANY),
    ("TextDisplay", "DATA_STYLE_FLAGS_ID", "alignment", N::TextAlign, ANY),
    ("AbstractMinecart", "DATA_ID_CUSTOM_DISPLAY_BLOCK", "DisplayState", N::BlockState, ANY),
    ("AbstractMinecart", "DATA_ID_DISPLAY_OFFSET", "DisplayOffset", N::Int, ANY),
    ("MinecartCommandBlock", "DATA_ID_COMMAND_NAME", "Command", N::String, ANY),
    ("AbstractArrow", "ID_FLAGS", "crit", N::FlagBool(0x01), ANY),
    ("AbstractArrow", "PIERCE_LEVEL", "PierceLevel", N::Byte, ANY),
    ("AbstractArrow", "IN_GROUND", "inGround", N::Bool, ANY),
    ("EyeOfEnder", "DATA_ITEM_STACK", "Item", N::Item, ANY),
    ("FireworkRocketEntity", "DATA_ID_FIREWORKS_ITEM", "FireworksItem", N::Item, ANY),
    ("FireworkRocketEntity", "DATA_SHOT_AT_ANGLE", "ShotAtAngle", N::Bool, ANY),
    ("Fireball", "DATA_ITEM_STACK", "Item", N::Item, ANY),
    ("WitherSkull", "DATA_DANGEROUS", "dangerous", N::Bool, ANY),
    ("ThrowableItemProjectile", "DATA_ITEM_STACK", "Item", N::Item, ANY),
];

fn synched_value_nbt(
    world: &WorldStore,
    value: &EntityDataValueKind,
    encoding: SynchedNbt,
) -> Option<NbtSummaryValue> {
    use EntityDataValueKind as V;
    Some(match (encoding, value) {
        (N::Bool, V::Boolean(value)) => NbtSummaryValue::Byte(i8::from(*value)),
        (N::True, V::Boolean(true)) => NbtSummaryValue::Byte(1),
        (N::Byte, V::Byte(value)) => NbtSummaryValue::Byte(*value),
        (N::IntByte, V::Int(value)) => NbtSummaryValue::Byte(*value as i8),
        (N::Short, V::Int(value)) => NbtSummaryValue::Short(*value as i16),
        (N::Int, V::Int(value)) => NbtSummaryValue::Int(*value),
        (N::Int, V::Byte(value)) => NbtSummaryValue::Int(i32::from(*value)),
        (N::PositiveInt, V::Int(value)) if *value > 0 => NbtSummaryValue::Int(*value),
        (N::IntMinusOne, V::Int(value)) => NbtSummaryValue::Int(value.saturating_sub(1)),
        (N::Long, V::Long(value)) => NbtSummaryValue::Long(*value),
        (N::Float, V::Float(value)) => float(*value),
        (N::String, V::String(value)) => value.to_nbt(),
        (N::Text, V::Component(text) | V::OptionalComponent(Some(text))) => {
            text.component().clone()
        }
        (N::Item, V::ItemStack(stack)) => item_stack_nbt(world, stack)?,
        (N::BlockState, V::BlockState(id) | V::OptionalBlockState(Some(id))) => {
            block_state_nbt(world, *id)?
        }
        (N::BlockPos, V::BlockPos(pos) | V::OptionalBlockPos(Some(pos))) => {
            NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z])
        }
        (N::Uuid, V::OptionalLivingEntityReference(Some(uuid))) => uuid_nbt(*uuid),
        (N::UuidList, V::OptionalLivingEntityReference(Some(uuid))) => {
            NbtSummaryValue::List(vec![uuid_nbt(*uuid)])
        }
        (N::Direction3d, V::Direction(direction)) => NbtSummaryValue::Byte(*direction as i8),
        (N::Direction2d, V::Direction(direction)) => NbtSummaryValue::Byte(match direction {
            3 => 0,
            4 => 1,
            2 => 2,
            5 => 3,
            _ => return None,
        }),
        (N::Registry, V::RegistryId { value, .. }) => string(value.identity()?.resource_id()),
        (N::Registry, V::PaintingVariant(variant)) => {
            string(variant.registry.as_ref()?.identity()?.resource_id())
        }
        (N::Names(names), V::EnumId { id, .. } | V::Int(id) | V::HumanoidArm(id)) => {
            string(names.get(usize::try_from(*id).ok()?)?)
        }
        (N::Names(names), V::Byte(id)) => string(names.get(usize::try_from(*id).ok()?)?),
        (N::Floats, V::Rotations { x, y, z } | V::Vector3f { x, y, z }) => floats(&[*x, *y, *z]),
        (N::Floats, V::Quaternionf { x, y, z, w }) => floats(&[*x, *y, *z, *w]),
        (
            N::VillagerData,
            V::VillagerData {
                villager_type,
                profession,
                level,
            },
        ) => NbtSummaryValue::Compound(vec![
            entry("level", NbtSummaryValue::Int(*level)),
            entry("profession", string(profession.identity()?.resource_id())),
            entry("type", string(villager_type.identity()?.resource_id())),
        ]),
        (N::Flag(mask), V::Byte(flags)) if flags & mask != 0 => NbtSummaryValue::Byte(1),
        (N::FlagBool(mask), V::Byte(flags)) => NbtSummaryValue::Byte(i8::from(flags & mask != 0)),
        (N::ByteMask(mask), V::Byte(flags)) => NbtSummaryValue::Byte(flags & mask),
        (N::LightNibble(shift), V::Int(packed)) if *packed != -1 => {
            NbtSummaryValue::Int((packed >> shift) & 15)
        }
        (N::Profile, V::ResolvableProfile(profile)) => profile_nbt(profile),
        (N::HiddenLayers, V::Byte(mask)) => NbtSummaryValue::List(
            [
                "cape",
                "jacket",
                "left_sleeve",
                "right_sleeve",
                "left_pants_leg",
                "right_pants_leg",
                "hat",
            ]
            .into_iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) == 0)
            .map(|(_, part)| string(part))
            .collect(),
        ),
        (N::MannequinPose, V::Pose(pose)) => string(match *pose {
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID => "standing",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID => "fall_flying",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID => "sleeping",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID => "swimming",
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID => "crouching",
            _ => return None,
        }),
        (N::Absent, V::OptionalComponent(None)) => NbtSummaryValue::Byte(1),
        (N::TextAlign, V::Byte(flags)) => string(if flags & 0x08 != 0 {
            "left"
        } else if flags & 0x10 != 0 {
            "right"
        } else {
            "center"
        }),
        _ => return None,
    })
}

/// `ResolvableProfile.FULL_CODEC`: the profile half, then the skin patch.
fn profile_nbt(profile: &bbb_protocol::packets::ResolvableProfileData) -> NbtSummaryValue {
    use bbb_protocol::packets::ResolvableProfileKind as K;
    let mut fields = Vec::new();
    let (name, id, properties) = match &profile.profile {
        K::GameProfile {
            id,
            name,
            properties,
        } => (Some(name.as_str()), Some(*id), properties.as_slice()),
        K::Partial {
            name,
            id,
            properties,
        } => (name.as_deref(), *id, properties.as_slice()),
    };
    if let Some(name) = name {
        fields.push(entry("name", string(name)));
    }
    if let Some(id) = id {
        fields.push(entry("id", uuid_nbt(id)));
    }
    if !properties.is_empty() {
        fields.push(entry(
            "properties",
            NbtSummaryValue::List(
                properties
                    .iter()
                    .map(|prop| {
                        let mut entries = vec![
                            entry("name", string(&prop.name)),
                            entry("value", string(&prop.value)),
                        ];
                        if let Some(sig) = &prop.signature {
                            entries.push(entry("signature", string(sig)));
                        }
                        NbtSummaryValue::Compound(entries)
                    })
                    .collect(),
            ),
        ));
    }
    let skin = &profile.skin_patch;
    for (key, texture) in [
        ("texture", &skin.body),
        ("cape", &skin.cape),
        ("elytra", &skin.elytra),
    ] {
        if let Some(texture) = texture {
            fields.push(entry(key, string(texture)));
        }
    }
    if let Some(slim) = skin.slim_model {
        fields.push(entry("model", string(if slim { "slim" } else { "wide" })));
    }
    NbtSummaryValue::Compound(fields)
}

/// `MobEffectInstance.CODEC` per client-held effect; defaults are omitted as the codec does.
fn entity_effects_nbt(entity: &EntityState) -> Option<NbtSummaryValue> {
    let effects = entity
        .mob_effects
        .values()
        .filter_map(|effect| {
            let mut fields = vec![entry(
                "id",
                string(
                    bbb_protocol::mob_effect_types::vanilla_mob_effect_type(effect.effect_id)?
                        .resource_id,
                ),
            )];
            if effect.amplifier != 0 {
                fields.push(entry(
                    "amplifier",
                    NbtSummaryValue::Byte(effect.amplifier as i8),
                ));
            }
            if effect.duration_ticks != 0 {
                fields.push(entry(
                    "duration",
                    NbtSummaryValue::Int(effect.duration_ticks),
                ));
            }
            if effect.ambient {
                fields.push(entry("ambient", NbtSummaryValue::Byte(1)));
            }
            if !effect.visible {
                fields.push(entry("show_particles", NbtSummaryValue::Byte(0)));
            }
            fields.push(entry(
                "show_icon",
                NbtSummaryValue::Byte(i8::from(effect.show_icon)),
            ));
            Some(NbtSummaryValue::Compound(fields))
        })
        .collect::<Vec<_>>();
    (!effects.is_empty()).then_some(NbtSummaryValue::List(effects))
}

/// `AttributeInstance.Packed.CODEC` per attribute the server sent.
fn entity_attributes_nbt(entity: &EntityState) -> Option<NbtSummaryValue> {
    let attributes = entity
        .attributes
        .iter()
        .filter_map(|attribute| {
            let descriptor =
                bbb_protocol::attribute_types::vanilla_attribute_type(attribute.attribute_id)?;
            let mut fields = vec![
                entry("id", string(descriptor.resource_id)),
                entry("base", double(attribute.base)),
            ];
            let modifiers = attribute
                .modifiers
                .iter()
                .filter_map(|modifier| {
                    Some(NbtSummaryValue::Compound(vec![
                        entry("amount", double(modifier.amount)),
                        entry("id", string(&modifier.id)),
                        entry(
                            "operation",
                            string(
                                bbb_protocol::attribute_modifier_types::attribute_modifier_operation_name(
                                    modifier.operation_id,
                                )?,
                            ),
                        ),
                    ]))
                })
                .collect::<Vec<_>>();
            if !modifiers.is_empty() {
                fields.push(entry("modifiers", NbtSummaryValue::List(modifiers)));
            }
            Some(NbtSummaryValue::Compound(fields))
        })
        .collect::<Vec<_>>();
    (!attributes.is_empty()).then_some(NbtSummaryValue::List(attributes))
}

/// `Leashable.writeLeashData`: a knot is saved by position, any other holder by UUID.
fn leash_nbt(world: &WorldStore, entity: &EntityState) -> Option<NbtSummaryValue> {
    let holder = world.probe_entity(entity.leash_holder_id?)?;
    if holder.entity_type_id == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
        let pos = holder.position_base;
        return Some(NbtSummaryValue::IntArray(vec![
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        ]));
    }
    Some(NbtSummaryValue::Compound(vec![entry(
        "UUID",
        uuid_nbt(holder.uuid),
    )]))
}

/// One item stack as `ItemStack.CODEC` writes it from what the client holds. Components the
/// persistent codec cannot write are dropped rather than guessed.
fn item_stack_nbt(world: &WorldStore, stack: &ItemStackSummary) -> Option<NbtSummaryValue> {
    stack.item_id?;
    let with_added = |added_type_ids: Vec<i32>, removed_type_ids: Vec<i32>| {
        let mut component_patch = stack.component_patch.clone();
        component_patch.added_type_ids = added_type_ids;
        component_patch.removed_type_ids = removed_type_ids;
        world.persistent_item_stack_to_nbt(&ItemStackSummary {
            component_patch,
            ..stack.clone()
        })
    };
    let nbt = world.persistent_item_stack_to_nbt(stack).or_else(|_| {
        let patch = &stack.component_patch;
        let encodable = patch
            .added_type_ids
            .iter()
            .copied()
            .filter(|&type_id| with_added(vec![type_id], Vec::new()).is_ok())
            .collect();
        with_added(encodable, patch.removed_type_ids.clone())
    });
    let NbtSummaryValue::Compound(fields) = nbt.ok()? else {
        return None;
    };
    (!fields.is_empty()).then_some(NbtSummaryValue::Compound(fields))
}

fn block_state_nbt(world: &WorldStore, state_id: i32) -> Option<NbtSummaryValue> {
    let state = world.registries().block_state(state_id)?;
    let mut fields = vec![entry("Name", string(&state.name))];
    if !state.properties.is_empty() {
        fields.push(entry(
            "Properties",
            NbtSummaryValue::Compound(
                state
                    .properties
                    .iter()
                    .map(|(key, value)| entry(key, string(value)))
                    .collect(),
            ),
        ));
    }
    Some(NbtSummaryValue::Compound(fields))
}

/// Appends `key`, nesting `Parent/Child` keys into the parent compound and extending a list
/// already stored under the same key.
fn put(tag: &mut Vec<NbtSummaryEntry>, key: &str, value: NbtSummaryValue) {
    let Some((parent, child)) = key.split_once('/') else {
        let name = JavaString::from(key);
        match (tag.iter_mut().find(|existing| existing.name == name), value) {
            (
                Some(NbtSummaryEntry {
                    value: NbtSummaryValue::List(items),
                    ..
                }),
                NbtSummaryValue::List(more),
            ) => items.extend(more),
            (_, value) => tag.push(entry(key, value)),
        }
        return;
    };
    let name = JavaString::from(parent);
    if let Some(NbtSummaryValue::Compound(children)) = tag
        .iter_mut()
        .find(|existing| existing.name == name)
        .map(|existing| &mut existing.value)
    {
        children.push(entry(child, value));
    } else {
        tag.push(entry(
            parent,
            NbtSummaryValue::Compound(vec![entry(child, value)]),
        ));
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: JavaString::from(name),
        value,
    }
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(value.to_bits())
}

fn floats(values: &[f32]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.iter().map(|value| float(*value)).collect())
}

fn doubles(values: &[f64]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.iter().map(|value| double(*value)).collect())
}

/// `UUIDUtil.CODEC`: four big-endian ints.
fn uuid_nbt(uuid: uuid::Uuid) -> NbtSummaryValue {
    let (most, least) = uuid.as_u64_pair();
    NbtSummaryValue::IntArray(vec![
        (most >> 32) as i32,
        most as i32,
        (least >> 32) as i32,
        least as i32,
    ])
}
