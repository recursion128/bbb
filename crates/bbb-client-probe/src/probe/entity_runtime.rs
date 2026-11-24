use std::time::Duration;

use anyhow::{bail, Context, Result};
use bbb_client_world::{EntityState, EntityVec3, LocalPlayerPoseState, WorldStore};
use bbb_protocol::{
    entity_synched_data::{VANILLA_ITEM_FRAME_ITEM_DATA_ID, VANILLA_ITEM_FRAME_ROTATION_DATA_ID},
    entity_types::{
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
    },
    equipment_slots::EquipmentSlot,
    item_types::{VANILLA_ITEM_IRON_HELMET_ID, VANILLA_ITEM_STONE_ID},
    packets::{
        self, DataComponentPatchSummary, EntityDataValue, EntityDataValueKind, InteractionHand,
        ItemStackSummary, SetCreativeModeSlot, Vec3d,
    },
};
use tokio::time::{timeout, Instant};
use uuid::Uuid;

use crate::types::{
    EntityRuntimeReplayReport, GeneralEntityReplayReport, ItemFrameReplayReport,
    PaintingReplayReport,
};
use bbb_client_net::{ConnectionState, PlayPacketBundleAssembler};

use super::{send_encoded, ProbeContext, ProbeProgress};

pub(super) const ARMOR_STAND_UUID: Uuid =
    Uuid::from_u128(0x6262_6200_0000_0000_0000_0000_0000_0001);
pub(super) const ITEM_FRAME_UUID: Uuid = Uuid::from_u128(0x6262_6200_0000_0000_0000_0000_0000_0002);
pub(super) const GLOW_ITEM_FRAME_UUID: Uuid =
    Uuid::from_u128(0x6262_6200_0000_0000_0000_0000_0000_0003);
pub(super) const PAINTING_UUID: Uuid = Uuid::from_u128(0x6262_6200_0000_0000_0000_0000_0000_0004);

const MOVED_ARMOR_STAND_POSITION: [f64; 3] = [1.5, 65.0, 0.5];
const CREATIVE_HOTBAR_ZERO_MENU_SLOT: i16 = 36;
/// Short enough to stay in the session's own chunk section and well inside the
/// 6.0 an interaction reaches (`isWithinEntityInteractionRange`, Player.java:1981-1985).
const DRIVE_STEP_BLOCKS: f64 = 0.25;
/// Two ticks, so the pairing a pending chunk refused is retried while the wait
/// it blocks is still outstanding.
const DRIVE_INTERVAL: Duration = Duration::from_millis(100);
const ARMOR_STAND_HURT_EVENT: i8 = 32;
const STAGED_ENTITY_TYPES: &[&str] = &[
    "minecraft:armor_stand",
    "minecraft:item_frame",
    "minecraft:glow_item_frame",
    "minecraft:painting",
];
const MILESTONE_ORDER: &[&str] = &[
    "staged_entities_spawned",
    "armor_stand_moved",
    "armor_stand_equipped",
    "item_frame_item_inserted",
    "item_frame_item_rotated",
    "survival_mode_applied",
    "armor_stand_first_attack",
    "armor_stand_removed",
    "item_frame_item_removed",
    "item_frame_removed",
    "painting_removed_after_move",
];

#[derive(Debug, Clone, Copy)]
struct TargetIds {
    armor_stand: i32,
    item_frame: i32,
    glow_item_frame: i32,
    painting: i32,
}

/// Keeps this session's movement packets going out while the leg waits.
///
/// `ChunkMap.addEntity` pairs a tracked entity to the arriving player
/// but its chunks are still pending then so
/// `isChunkTracked` refuses; `ChunkMap.tick`
/// re-pairs only for an entity that changed section
/// or a player that moved, and the staged entities never change section. That
/// leaves `ChunkMap.move`, which every accepted
/// movement packet reaches — so the
/// packets have to keep coming until the pending chunks let a pairing through.
struct PlayerDrive {
    anchor: Vec3d,
    stepped_off: bool,
    next_at: Instant,
}

impl PlayerDrive {
    fn new(probe: &ProbeContext) -> Result<Self> {
        let pose = probe
            .world
            .local_player_pose()
            .context("the entity runtime leg needs the session's own pose")?;
        Ok(Self {
            anchor: pose.position,
            stepped_off: false,
            next_at: Instant::now(),
        })
    }

    async fn step(&mut self, probe: &mut ProbeContext) -> Result<()> {
        self.stepped_off = !self.stepped_off;
        self.send(probe).await
    }

    /// Puts the session back on its anchor, so both servers answer every
    /// interaction the leg sends from the same place.
    async fn recenter(&mut self, probe: &mut ProbeContext) -> Result<()> {
        self.stepped_off = false;
        self.send(probe).await
    }

    async fn send(&mut self, probe: &mut ProbeContext) -> Result<()> {
        let mut pose: LocalPlayerPoseState = probe
            .world
            .local_player_pose()
            .context("the entity runtime leg lost the session's own pose")?;
        pose.position = self.anchor;
        if self.stepped_off {
            pose.position.z -= DRIVE_STEP_BLOCKS;
        }
        pose.on_ground = true;
        send_encoded(
            &mut probe.conn,
            packets::encode_play_move_player_pos_rot(
                pose.position.x,
                pose.position.y,
                pose.position.z,
                pose.y_rot,
                pose.x_rot,
                pose.on_ground,
                false,
            ),
        )
        .await?;
        probe.world.set_local_player_pose(pose);
        self.next_at = Instant::now() + DRIVE_INTERVAL;
        Ok(())
    }
}

pub(super) async fn run(
    probe: &mut ProbeContext,
    packets_seen: &mut usize,
    progress: &ProbeProgress,
) -> Result<EntityRuntimeReplayReport> {
    let mut bundle = PlayPacketBundleAssembler::default();
    let mut drive = PlayerDrive::new(probe)?;
    drive.step(probe).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the four staged entity-runtime targets",
        |world| Ok(find_targets(world)?.is_some()),
    )
    .await?;
    let targets = find_targets(&probe.world)?
        .context("entity runtime probe lost a staged target after observing all four")?;
    let armor_spawn = entity_position(&probe.world, targets.armor_stand)?;
    let frame_spawn = entity_position(&probe.world, targets.item_frame)?;
    let glow_frame_spawn = entity_position(&probe.world, targets.glow_item_frame)?;
    let painting_spawn = entity_position(&probe.world, targets.painting)?;

    send_command(
        probe,
        "teleport @e[type=minecraft:armor_stand,limit=1] 1.5 65 0.5",
    )
    .await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the staged armor stand movement",
        |world| {
            Ok(world
                .probe_entity(targets.armor_stand)
                .is_some_and(|entity| position_matches(&entity, MOVED_ARMOR_STAND_POSITION)))
        },
    )
    .await?;

    send_creative_stack(probe, stack(VANILLA_ITEM_IRON_HELMET_ID)).await?;
    send_interact(
        probe,
        &mut drive,
        targets.armor_stand,
        Vec3d {
            x: 0.0,
            y: 1.8,
            z: 0.0,
        },
    )
    .await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the armor stand head equipment update",
        |world| {
            Ok(world
                .equipment_item(targets.armor_stand, EquipmentSlot::Head)
                .and_then(|item| item.item_id)
                == Some(VANILLA_ITEM_IRON_HELMET_ID))
        },
    )
    .await?;

    send_creative_stack(probe, stack(VANILLA_ITEM_STONE_ID)).await?;
    send_interact(probe, &mut drive, targets.item_frame, Vec3d::default()).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the item frame inserted item",
        |world| {
            let Some(entity) = world.probe_entity(targets.item_frame) else {
                return Ok(false);
            };
            Ok(frame_item_id(&entity)? == Some(VANILLA_ITEM_STONE_ID))
        },
    )
    .await?;
    send_interact(probe, &mut drive, targets.item_frame, Vec3d::default()).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the item frame rotation update",
        |world| {
            let Some(entity) = world.probe_entity(targets.item_frame) else {
                return Ok(false);
            };
            Ok(frame_rotation(&entity)? == 1)
        },
    )
    .await?;

    send_creative_stack(probe, ItemStackSummary::empty()).await?;
    send_command(probe, "gamemode survival").await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the survival game-mode update",
        |world| Ok(world.local_player_is_survival()),
    )
    .await?;

    // ArmorStand makes survival attacks a two-step event/removal sequence.
    send_attack(probe, &mut drive, targets.armor_stand).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "armor stand hurt event 32",
        |world| {
            Ok(world
                .probe_entity(targets.armor_stand)
                .is_some_and(|entity| entity.last_event_id == Some(ARMOR_STAND_HURT_EVENT)))
        },
    )
    .await?;
    send_attack(probe, &mut drive, targets.armor_stand).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the armor stand removal after its second attack",
        |world| Ok(world.probe_entity(targets.armor_stand).is_none()),
    )
    .await?;

    // ItemFrame removes a framed item before base hanging damage removes the frame.
    send_attack(probe, &mut drive, targets.item_frame).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the item frame item removal after its first attack",
        |world| {
            let Some(entity) = world.probe_entity(targets.item_frame) else {
                return Ok(false);
            };
            Ok(frame_item_id(&entity)?.is_none())
        },
    )
    .await?;
    send_attack(probe, &mut drive, targets.item_frame).await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the item frame removal after its second attack",
        |world| Ok(world.probe_entity(targets.item_frame).is_none()),
    )
    .await?;

    // BlockAttachedEntity removes a hanging entity when it is moved.
    send_command(
        probe,
        "teleport @e[type=minecraft:painting,limit=1] -4 66 3",
    )
    .await?;
    pump_until(
        probe,
        &mut drive,
        &mut bundle,
        packets_seen,
        progress,
        "the painting removal after a movement command",
        |world| Ok(world.probe_entity(targets.painting).is_none()),
    )
    .await?;

    Ok(EntityRuntimeReplayReport {
        staged_entity_types: STAGED_ENTITY_TYPES
            .iter()
            .map(|entity_type| (*entity_type).to_owned())
            .collect(),
        milestone_order: MILESTONE_ORDER
            .iter()
            .map(|milestone| (*milestone).to_owned())
            .collect(),
        general: GeneralEntityReplayReport {
            spawn_position: armor_spawn,
            moved_position: MOVED_ARMOR_STAND_POSITION,
            equipped_item_id: VANILLA_ITEM_IRON_HELMET_ID,
            first_attack_event: ARMOR_STAND_HURT_EVENT,
            removed_after_second_attack: true,
        },
        item_frame: ItemFrameReplayReport {
            spawn_position: frame_spawn,
            inserted_item_id: VANILLA_ITEM_STONE_ID,
            rotation_after_second_interaction: 1,
            item_removed_after_first_attack: true,
            removed_after_second_attack: true,
        },
        painting: PaintingReplayReport {
            spawn_position: painting_spawn,
            removed_after_move: true,
        },
        glow_item_frame_spawn_position: glow_frame_spawn,
    })
}

async fn pump_until(
    probe: &mut ProbeContext,
    drive: &mut PlayerDrive,
    bundle: &mut PlayPacketBundleAssembler,
    packets_seen: &mut usize,
    progress: &ProbeProgress,
    waiting_for: &str,
    mut ready: impl FnMut(&WorldStore) -> Result<bool>,
) -> Result<()> {
    progress.note(waiting_for);
    while !ready(&probe.world)? {
        let Ok(read) = timeout(
            drive.next_at.saturating_duration_since(Instant::now()),
            probe.session.read_packet_or_send_play_tick(),
        )
        .await
        else {
            drive.step(probe).await?;
            continue;
        };
        let (packet_id, payload) = read?;
        *packets_seen = packets_seen.saturating_add(1);
        if probe.state != ConnectionState::Play {
            bail!("entity runtime probe left play while waiting for {waiting_for}");
        }
        let packet = probe.session.decode_play_packet(packet_id, &payload)?;
        if let Some(batch) = bundle.push(packet)? {
            for packet in batch {
                probe.apply_observed_play_packet(packet).await?;
            }
        }
    }
    Ok(())
}

async fn send_command(probe: &mut ProbeContext, command: &str) -> Result<()> {
    send_encoded(
        &mut probe.conn,
        packets::encode_play_chat_command(&packets::ChatCommand {
            command: command.to_owned(),
        }),
    )
    .await
}

async fn send_creative_stack(probe: &mut ProbeContext, item: ItemStackSummary) -> Result<()> {
    send_encoded(&mut probe.conn, packets::encode_play_set_carried_item(0)).await?;
    send_encoded(
        &mut probe.conn,
        packets::encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: CREATIVE_HOTBAR_ZERO_MENU_SLOT,
            item,
        })?,
    )
    .await
}

async fn send_interact(
    probe: &mut ProbeContext,
    drive: &mut PlayerDrive,
    entity_id: i32,
    location: Vec3d,
) -> Result<()> {
    drive.recenter(probe).await?;
    send_encoded(
        &mut probe.conn,
        packets::encode_play_interact_entity(packets::InteractEntity {
            entity_id,
            hand: InteractionHand::MainHand,
            location,
            using_secondary_action: false,
        }),
    )
    .await
}

async fn send_attack(
    probe: &mut ProbeContext,
    drive: &mut PlayerDrive,
    entity_id: i32,
) -> Result<()> {
    drive.recenter(probe).await?;
    send_encoded(
        &mut probe.conn,
        packets::encode_play_attack_entity(packets::AttackEntity { entity_id }),
    )
    .await
}

fn stack(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn find_targets(world: &WorldStore) -> Result<Option<TargetIds>> {
    let armor_stand = find_target(world, ARMOR_STAND_UUID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID)?;
    let item_frame = find_target(world, ITEM_FRAME_UUID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID)?;
    let glow_item_frame = find_target(
        world,
        GLOW_ITEM_FRAME_UUID,
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
    )?;
    let painting = find_target(world, PAINTING_UUID, VANILLA_ENTITY_TYPE_PAINTING_ID)?;
    Ok(match (armor_stand, item_frame, glow_item_frame, painting) {
        (Some(armor_stand), Some(item_frame), Some(glow_item_frame), Some(painting)) => {
            Some(TargetIds {
                armor_stand,
                item_frame,
                glow_item_frame,
                painting,
            })
        }
        _ => None,
    })
}

fn find_target(world: &WorldStore, uuid: Uuid, expected_type_id: i32) -> Result<Option<i32>> {
    let Some(entity) = world
        .probe_entities()
        .into_iter()
        .find(|entity| entity.uuid == uuid)
    else {
        return Ok(None);
    };
    if entity.entity_type_id != expected_type_id {
        bail!(
            "staged entity {uuid} has type {}, expected {expected_type_id}",
            entity.entity_type_id
        );
    }
    Ok(Some(entity.id))
}

fn entity_position(world: &WorldStore, entity_id: i32) -> Result<[f64; 3]> {
    let entity = world
        .probe_entity(entity_id)
        .with_context(|| format!("entity runtime target {entity_id} disappeared before replay"))?;
    let position = published_position(&entity);
    if !position.iter().all(|component| component.is_finite()) {
        bail!("entity runtime target {entity_id} has a non-finite position");
    }
    Ok(position)
}

/// `InterpolationHandler.position`: where a
/// published move puts the target, rather than the point the client has lerped
/// as far as. The leg runs no client ticks, so an interpolation it started
/// would never finish and the lerped point would never leave the old position.
fn published_position(entity: &EntityState) -> [f64; 3] {
    published_position_of(
        entity.position,
        entity
            .interpolation
            .as_ref()
            .map(|state| state.target_position),
    )
}

fn published_position_of(position: EntityVec3, target: Option<EntityVec3>) -> [f64; 3] {
    let position = target.unwrap_or(position);
    [position.x, position.y, position.z]
}

fn position_matches(entity: &EntityState, expected: [f64; 3]) -> bool {
    published_position(entity)
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() <= f64::EPSILON)
}

fn frame_item_id(entity: &EntityState) -> Result<Option<i32>> {
    frame_item_id_from_values(&entity.data_values)
}

fn frame_item_id_from_values(values: &[EntityDataValue]) -> Result<Option<i32>> {
    let Some(value) = values
        .iter()
        .rev()
        .find(|value| value.data_id == VANILLA_ITEM_FRAME_ITEM_DATA_ID)
    else {
        return Ok(None);
    };
    let EntityDataValueKind::ItemStack(stack) = &value.value else {
        bail!("item frame item metadata used the wrong serializer value");
    };
    Ok((stack.count > 0).then_some(stack.item_id).flatten())
}

fn frame_rotation(entity: &EntityState) -> Result<i32> {
    frame_rotation_from_values(&entity.data_values)
}

fn frame_rotation_from_values(values: &[EntityDataValue]) -> Result<i32> {
    let Some(value) = values
        .iter()
        .rev()
        .find(|value| value.data_id == VANILLA_ITEM_FRAME_ROTATION_DATA_ID)
    else {
        return Ok(0);
    };
    let EntityDataValueKind::Int(rotation) = value.value else {
        bail!("item frame rotation metadata used the wrong serializer value");
    };
    Ok(rotation.rem_euclid(8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID;

    fn data(data_id: u8, value: EntityDataValueKind) -> EntityDataValue {
        EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value,
        }
    }

    #[test]
    fn entity_runtime_item_frame_metadata_defaults_and_updates() {
        assert_eq!(frame_item_id_from_values(&[]).unwrap(), None);
        assert_eq!(frame_rotation_from_values(&[]).unwrap(), 0);

        let values = vec![
            data(
                VANILLA_ITEM_FRAME_ITEM_DATA_ID,
                EntityDataValueKind::ItemStack(stack(VANILLA_ITEM_STONE_ID)),
            ),
            data(
                VANILLA_ITEM_FRAME_ROTATION_DATA_ID,
                EntityDataValueKind::Int(9),
            ),
        ];
        assert_eq!(
            frame_item_id_from_values(&values).unwrap(),
            Some(VANILLA_ITEM_STONE_ID)
        );
        assert_eq!(frame_rotation_from_values(&values).unwrap(), 1);
    }

    #[test]
    fn entity_runtime_empty_item_metadata_is_absent() {
        let values = [data(
            VANILLA_ITEM_FRAME_ITEM_DATA_ID,
            EntityDataValueKind::ItemStack(ItemStackSummary::empty()),
        )];
        assert_eq!(frame_item_id_from_values(&values).unwrap(), None);
    }

    #[test]
    fn entity_runtime_reads_the_published_position_not_the_lerped_one() {
        let spawn = EntityVec3 {
            x: 1.5,
            y: 64.0,
            z: 0.5,
        };
        let moved = EntityVec3 { y: 65.0, ..spawn };
        assert_eq!(published_position_of(spawn, None), [1.5, 64.0, 0.5]);
        assert_eq!(
            published_position_of(spawn, Some(moved)),
            MOVED_ARMOR_STAND_POSITION
        );
    }

    #[test]
    fn entity_runtime_contract_has_fixed_targets_and_order() {
        assert_eq!(
            [
                ARMOR_STAND_UUID,
                ITEM_FRAME_UUID,
                GLOW_ITEM_FRAME_UUID,
                PAINTING_UUID,
            ]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
            4
        );
        assert_eq!(MILESTONE_ORDER.first(), Some(&"staged_entities_spawned"));
        assert_eq!(MILESTONE_ORDER.last(), Some(&"painting_removed_after_move"));
    }
}
