//! `ZombieVillager`'s cure and the villager record a conversion carries
//! between a villager and a zombie villager in either direction.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID;
use bbb_protocol::item_types::VANILLA_ITEM_GOLDEN_APPLE_ID;
use bbb_protocol::packets::{
    BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent, ItemStackSummary,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::PendingZombieAction;
use crate::connection::ClientboundPlayPacket;
use crate::entity::item::ItemResolver;
use crate::entity::living::{ActiveMobEffect, SynchedValue};
use crate::entity::save::{optional_int, optional_uuid, uuid_int_array};
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore};
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

// `ZombieVillager.java:67-71`.
const CONVERSION_WAIT_MIN: i32 = 3600;
const CONVERSION_WAIT_MAX: i32 = 6000;
const MAX_SPECIAL_BLOCKS_COUNT: i32 = 14;
const SPECIAL_BLOCK_RADIUS: i32 = 4;
const NOT_CONVERTING: i32 = -1;
const CURE_STARTED_EVENT: i8 = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ZombieVillagerCure {
    time: i32,
    starter: Option<Uuid>,
    converting: bool,
}

impl ZombieVillagerCure {
    pub(crate) const fn converting(self) -> bool {
        self.converting
    }

    pub(crate) const fn starter(self) -> Option<Uuid> {
        self.starter
    }

    pub(crate) fn owns_save_field(name: &str) -> bool {
        matches!(name, "ConversionTime" | "ConversionPlayer")
    }

    // `ZombieVillager.java:118-125`.
    pub(crate) fn read_save(entries: &[NbtSummaryEntry]) -> Self {
        match optional_int(entries, "ConversionTime").unwrap_or(NOT_CONVERTING) {
            NOT_CONVERTING => Self::default(),
            time => Self {
                time,
                starter: optional_uuid(entries, "ConversionPlayer"),
                converting: true,
            },
        }
    }

    // `ZombieVillager.java:101-102`.
    pub(crate) fn write_save(&self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.push(NbtSummaryEntry {
            name: "ConversionTime".into(),
            value: NbtSummaryValue::Int(if self.converting {
                self.time
            } else {
                NOT_CONVERTING
            }),
        });
        if let Some(starter) = self.starter {
            entries.push(NbtSummaryEntry {
                name: "ConversionPlayer".into(),
                value: uuid_int_array(starter),
            });
        }
    }

    pub(crate) fn synched_value(&self) -> SynchedValue {
        SynchedValue {
            value: EntityDataValue {
                data_id: VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(self.converting),
            },
            is_default: !self.converting,
        }
    }
}

fn cure_mut(entity: &mut Entity) -> Option<&mut ZombieVillagerCure> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => data.zombie_villager_cure_mut(),
        _ => None,
    }
}

/// What `ZombieVillager.mobInteract` answered (`ZombieVillager.java:168-184`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CureInteraction {
    Pass,
    /// `SUCCESS_SERVER`: one golden apple spent and the cure started.
    Started,
    /// `CONSUME`: a golden apple on a zombie villager without weakness.
    Refused,
}

pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    hand_stack: &mut ItemStackSummary,
    player_uuid: Uuid,
    infinite_materials: bool,
) -> CureInteraction {
    if hand_stack.item_id != Some(VANILLA_ITEM_GOLDEN_APPLE_ID) {
        return CureInteraction::Pass;
    }
    let Some(target) = store
        .get(target_id)
        .filter(|target| target.entity_type_id() == VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID)
    else {
        return CureInteraction::Pass;
    };
    if target
        .living()
        .and_then(|living| living.active_effect("minecraft:weakness"))
        .is_none()
    {
        return CureInteraction::Refused;
    }
    let Some(mut target) = store.detach(target_id) else {
        return CureInteraction::Pass;
    };
    if !infinite_materials {
        hand_stack.count -= 1;
        if hand_stack.count <= 0 {
            *hand_stack = ItemStackSummary::empty();
        }
    }
    let time = store
        .random()
        .next_int(CONVERSION_WAIT_MAX - CONVERSION_WAIT_MIN + 1)
        + CONVERSION_WAIT_MIN;
    start_converting(&mut target, store, player_uuid, time);
    store.attach(target);
    CureInteraction::Started
}

/// `ZombieVillager.startConverting` (`ZombieVillager.java:200-207`).
fn start_converting(entity: &mut Entity, store: &mut EntityStore, starter: Uuid, time: i32) {
    let Some(cure) = cure_mut(entity) else {
        return;
    };
    *cure = ZombieVillagerCure {
        time,
        starter: Some(starter),
        converting: true,
    };
    entity.mark_synched_dirty(VANILLA_ZOMBIE_VILLAGER_CONVERTING_DATA_ID);
    apply_conversion_effects(entity, store, time);
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: CURE_STARTED_EVENT,
        }),
    );
}

fn apply_conversion_effects(entity: &mut Entity, store: &mut EntityStore, time: i32) {
    crate::entity::living::mob_effects::remove_active_effect(entity, store, "minecraft:weakness");
    // `min(difficulty - 1, 0)` never rises above the amplifier floor of 0.
    if let Some(published) = ActiveMobEffect::new("minecraft:strength", time, 0)
        .and_then(|effect| entity.living_mut()?.add_active_effect_published(effect))
    {
        crate::entity::living::mob_effects::send_effect_update(store, entity.id(), &published);
    }
}

pub(crate) fn finish_load(entity: &mut Entity, store: &mut EntityStore) {
    let Some(cure) = cure_mut(entity).filter(|cure| cure.converting()) else {
        return;
    };
    let time = cure.time;
    apply_conversion_effects(entity, store, time);
}

/// `ZombieVillager.tick` before `super.tick` (`ZombieVillager.java:155-165`);
/// the world swaps the body once the countdown runs out.
pub(crate) fn tick_cure(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let converting = matches!(
        entity.data(),
        EntityData::GeneralLiving(data)
            if data.zombie_villager_cure().is_some_and(ZombieVillagerCure::converting)
    );
    if !converting
        || !entity.is_alive()
        || entity.living().is_none_or(|living| living.health() <= 0.0)
    {
        return;
    }
    let amount = conversion_progress(entity, level, store.random());
    let Some(cure) = cure_mut(entity) else {
        return;
    };
    cure.time = cure.time.wrapping_sub(amount);
    if cure.time > 0 {
        return;
    }
    // `finishConversion`'s `dropPreservedEquipment` (`ZombieVillager.java:240-242`).
    crate::entity::mob_death_loot::drop_preserved_equipment(store, level, entity, true);
    store.queue_command(EntityCommand::Zombie(PendingZombieAction::Cure {
        zombie_villager_id: entity.id(),
    }));
}

/// `ZombieVillager.getConversionProgress` (`ZombieVillager.java:282-305`):
/// iron bars and beds near the zombie villager speed the cure.
fn conversion_progress(entity: &Entity, level: &LevelCtx<'_, '_>, random: &GameRandom) -> i32 {
    let mut amount = 1;
    if random.next_float() < 0.01 {
        let position = entity.position();
        let (x, y, z) = (position.x as i32, position.y as i32, position.z as i32);
        let mut special_blocks = 0;
        'scan: for xx in x - SPECIAL_BLOCK_RADIUS..x + SPECIAL_BLOCK_RADIUS {
            for yy in y - SPECIAL_BLOCK_RADIUS..y + SPECIAL_BLOCK_RADIUS {
                for zz in z - SPECIAL_BLOCK_RADIUS..z + SPECIAL_BLOCK_RADIUS {
                    if special_blocks >= MAX_SPECIAL_BLOCKS_COUNT {
                        break 'scan;
                    }
                    let state = level.block_state(BlockPos {
                        x: xx,
                        y: yy,
                        z: zz,
                    });
                    let special = vanilla_block_state_block_type_id(state)
                        == Some(bbb_protocol::block_types::VANILLA_BLOCK_IRON_BARS_ID)
                        || level.block_state_in_tag(state, "minecraft:beds");
                    if special {
                        if random.next_float() < 0.3 {
                            amount += 1;
                        }
                        special_blocks += 1;
                    }
                }
            }
        }
    }
    amount
}

fn opaque<'a>(entity: &'a Entity, name: &str) -> Option<&'a NbtSummaryValue> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    data.opaque_fields()
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn opaque_mut<'a>(entity: &'a mut Entity, name: &str) -> Option<&'a mut NbtSummaryValue> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    let fields = data.opaque_fields_mut();
    if !fields.iter().any(|entry| entry.name == name) {
        fields.push(NbtSummaryEntry {
            name: name.into(),
            value: NbtSummaryValue::List(Vec::new()),
        });
    }
    fields
        .iter_mut()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &mut entry.value)
}

/// The villager data, gossips, offers and experience a conversion carries
/// (`Zombie.java:263-266`, `ZombieVillager.java:249-258`).
pub(crate) fn carry_villager_record(source: &Entity, target: &mut Entity, items: ItemResolver<'_>) {
    let _ = crate::entity::general_living::copy_villager_data(target, source);
    if let EntityData::GeneralLiving(source_data) = source.data() {
        if let Some(gossips) = source_data.villager_gossips() {
            if let EntityData::GeneralLiving(target_data) = &mut target.data {
                if let Some(target_gossips) = target_data.villager_gossips_mut() {
                    *target_gossips = gossips.to_vec();
                }
            }
        }
    }
    if let Some(offers) = source.merchant_offers() {
        let value = crate::entity::merchant::write_offers(offers, items);
        if let Some(slot) = opaque_mut(target, "Offers") {
            *slot = value;
        }
    } else if let Some(offers) = opaque(source, "Offers")
        .and_then(|value| crate::entity::merchant::read_offers(value, items))
    {
        let _ = target.initialize_merchant_offers(offers);
    }
}

/// `Villager.onReputationEventFrom(ZOMBIE_VILLAGER_CURED)` (`Villager.java:856-858`).
pub(crate) fn add_cure_gossips(villager: &mut Entity, player: Uuid) {
    add_gossip(villager, player, "major_positive", 20, 20);
    add_gossip(villager, player, "minor_positive", 25, 25);
}

/// `GossipContainer.add` (`GossipContainer.java:117-124`, `:164-167`, `:204-213`).
pub(crate) fn add_gossip(villager: &mut Entity, player: Uuid, kind: &str, amount: i32, max: i32) {
    let Some(kind) = crate::entity::general_living::GossipKind::from_id(kind) else {
        return;
    };
    let EntityData::GeneralLiving(data) = &mut villager.data else {
        return;
    };
    let Some(gossips) = data.villager_gossips_mut() else {
        return;
    };
    crate::entity::general_living::add_gossip_entry(gossips, player, kind, amount, max);
}
