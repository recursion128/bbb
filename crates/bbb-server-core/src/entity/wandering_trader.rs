use crate::entity::item_slots::set_living_item_in_tick;
use crate::entity::living::item_use::set_using_item;
use crate::entity::living::mob_effects::remove_all_active_effects_in_level;
use crate::entity::thrown_potion::{apply_registry_potion_in_level, registry_potion_stack};
use crate::entity::{Entity, EntityData, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_MILK_BUCKET_ID,
    VANILLA_ITEM_POTION_ID,
};
use bbb_protocol::packets::{BlockPos, InteractionHand, ItemStackSummary, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TraderState {
    delay: i32,
    target: Option<BlockPos>,
    drink: Option<DrinkUse>,
}
impl TraderState {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.drink = previous.drink;
    }

    pub(crate) fn new(id: i32) -> Self {
        Self {
            delay: if id == VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID {
                47_999
            } else {
                0
            },
            target: None,
            drink: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraderDrinkKind {
    Invisibility,
    Milk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrinkUse {
    kind: TraderDrinkKind,
    remaining: u8,
}

impl TraderDrinkKind {
    fn item_id(self) -> i32 {
        match self {
            Self::Invisibility => VANILLA_ITEM_POTION_ID,
            Self::Milk => VANILLA_ITEM_MILK_BUCKET_ID,
        }
    }

    fn consume_sound(self) -> &'static str {
        // WanderingTrader.java:181-182.
        match self {
            Self::Invisibility => "minecraft:entity.wandering_trader.drink_potion",
            Self::Milk => "minecraft:entity.wandering_trader.drink_milk",
        }
    }
}

fn drink_mut(entity: &mut Entity) -> Option<&mut Option<DrinkUse>> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    Some(&mut data.trader_state_mut()?.drink)
}

pub(crate) fn apply_drink_change(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    stop: Option<TraderDrinkKind>,
    start: Option<TraderDrinkKind>,
) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
        return;
    }
    // UseItemGoal.java:35-45.
    if let Some(kind) = stop {
        set_using_item(entity, level, None);
        if let Some(drink) = drink_mut(entity) {
            *drink = None;
        }
        let _ = set_living_item_in_tick(entity, store, level, EquipmentSlot::MainHand, None);
        let sound = match kind {
            TraderDrinkKind::Invisibility => "minecraft:entity.wandering_trader.disappeared",
            TraderDrinkKind::Milk => "minecraft:entity.wandering_trader.reappeared",
        };
        let pitch = store.random().next_float() * 0.2 + 0.9;
        play_drink_sound(entity, store, sound, 1.0, pitch);
    }
    if let Some(kind) = start {
        let stack = match kind {
            TraderDrinkKind::Invisibility => {
                registry_potion_stack("minecraft:potion", "minecraft:invisibility")
            }
            TraderDrinkKind::Milk => Some(fresh_stack(kind.item_id())),
        };
        let Some(stack) = stack else {
            return;
        };
        if set_living_item_in_tick(entity, store, level, EquipmentSlot::MainHand, Some(stack))
            .is_err()
        {
            return;
        }
        set_using_item(entity, level, Some(InteractionHand::MainHand));
        if let Some(drink) = drink_mut(entity) {
            // Consumables.java:65, 72-73: both canonical drinks last 1.6 seconds.
            *drink = Some(DrinkUse {
                kind,
                remaining: 32,
            });
        }
    }
}

pub(crate) fn tick_drinking(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
        return true;
    }
    let Some(drink) = drink_mut(entity).and_then(|drink| *drink) else {
        return true;
    };
    // LivingEntity.java:3361-3369: a changed or empty hand ends actual use.
    if !entity.living().is_some_and(|living| {
        living.living_entity_flags() & 3 == 1
            && living
                .item_by_slot(EquipmentSlot::MainHand)
                .is_some_and(|stack| {
                    !stack.is_empty() && stack.item_id() == Some(drink.kind.item_id())
                })
    }) {
        set_using_item(entity, level, None);
        *drink_mut(entity).unwrap() = None;
        return true;
    }
    // Consumable.java:119-123; ItemStack.java:1094-1097.
    if 32 - drink.remaining > 7 && drink.remaining % 4 == 0 {
        let pitch = 0.9 + store.random().next_float() * 0.1;
        play_drink_sound(entity, store, drink.kind.consume_sound(), 0.5, pitch);
    }
    let remaining = drink.remaining.saturating_sub(1);
    if remaining > 0 {
        drink_mut(entity).unwrap().as_mut().unwrap().remaining = remaining;
        return true;
    }
    let mut stack = entity
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .unwrap()
        .clone();
    let applied = match drink.kind {
        TraderDrinkKind::Invisibility => match super::item_components::has(
            &stack,
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ) {
            Ok(false) => true,
            Ok(true) => apply_registry_potion_in_level(entity, store, level, &stack).is_some(),
            Err(_) => false,
        },
        TraderDrinkKind::Milk => remove_all_active_effects_in_level(entity, store, level).is_some(),
    };
    if !applied {
        return false;
    }
    // Consumable.java:77-93; ItemStack.java:391-404.
    let pitch = 0.9 + store.random().next_float() * 0.1;
    play_drink_sound(entity, store, drink.kind.consume_sound(), 0.5, pitch);
    level.game_event_at(
        crate::level_event::VanillaGameEvent::DRINK,
        entity.position(),
        Some(entity.id()),
    );
    stack.shrink(1);
    let remainder = if stack.is_empty() {
        fresh_stack(match drink.kind {
            TraderDrinkKind::Invisibility => VANILLA_ITEM_GLASS_BOTTLE_ID,
            TraderDrinkKind::Milk => VANILLA_ITEM_BUCKET_ID,
        })
    } else {
        stack
    };
    set_using_item(entity, level, None);
    *drink_mut(entity).unwrap() = None;
    set_living_item_in_tick(
        entity,
        store,
        level,
        EquipmentSlot::MainHand,
        Some(remainder),
    )
    .is_ok()
}

fn fresh_stack(item_id: i32) -> RuntimeItemStack {
    RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    })
}

fn play_drink_sound(
    entity: &Entity,
    store: &mut EntityStore,
    sound: &str,
    volume: f32,
    pitch: f32,
) {
    if !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: sound.to_owned(),
            source: SoundSource::Neutral,
            volume,
            pitch,
        });
    }
}

impl Entity {
    pub fn trader_despawn_delay(&self) -> Option<i32> {
        let EntityData::GeneralLiving(data) = self.data() else {
            return None;
        };
        Some(data.trader_state()?.delay)
    }
    pub fn set_trader_despawn_delay(&mut self, delay: i32) -> bool {
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(trader) = data.trader_state_mut() else {
            return false;
        };
        trader.delay = delay;
        true
    }
    pub fn wandering_trader_target(&self) -> Option<BlockPos> {
        if self.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
            return None;
        }
        let EntityData::GeneralLiving(data) = self.data() else {
            return None;
        };
        data.trader_state()?.target
    }
    pub fn set_wandering_trader_target(&mut self, target: Option<BlockPos>) -> bool {
        if self.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
            return false;
        }
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(trader) = data.trader_state_mut() else {
            return false;
        };
        trader.target = target;
        true
    }
    /// The `mobInteract` gates after the held item: alive and not trading,
    /// and not a baby (`WanderingTrader.java:109`) or not sleeping
    /// (`Villager.java:299`).
    pub fn merchant_accepts_trade(&self) -> bool {
        let gate = match self.entity_type_id() {
            VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID => !is_baby(self),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID => {
                self.sleeping_pos().is_none()
            }
            _ => false,
        };
        gate && crate::entity::name_tag::living_entity_is_alive(self)
            && self.merchant_trading_player().is_none()
    }
    pub fn set_merchant_trading_player(&mut self, player: Option<i32>) -> bool {
        let EntityData::GeneralLiving(data) = &mut self.data else {
            return false;
        };
        let Some(merchant) = data.abstract_villager_retained_mut() else {
            return false;
        };
        merchant.trading_player = player;
        true
    }
    pub fn set_spawn_rotation(&mut self, yaw: f32) {
        self.set_rot(yaw, 0.0);
        if let Some(living) = self.living_mut() {
            living.y_head_rot = yaw;
            living.y_body_rot = yaw;
        }
    }
}

// WanderingTrader.java:205-213; TraderLlama.java:87-106.
pub(crate) fn after_ai_step(entity: &mut Entity, store: &mut EntityStore) {
    let Some(delay) = entity.trader_despawn_delay() else {
        return;
    };
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
        if delay > 0
            && data
                .abstract_villager_retained()
                .is_some_and(|merchant| merchant.trading_player.is_none())
        {
            entity.set_trader_despawn_delay(delay - 1);
            if delay == 1 {
                entity.discard();
            }
        }
        return;
    }
    let holder = crate::entity::leash::leash_holder(entity);
    let trader_delay = holder
        .and_then(|id| store.get(id))
        .filter(|holder| holder.entity_type_id() == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID)
        .and_then(Entity::trader_despawn_delay);
    let passengers = store.passengers(entity.id());
    if data
        .abstract_horse_retained()
        .is_some_and(|horse| horse.is_tame())
        || (holder.is_some() && trader_delay.is_none())
        || (passengers.len() == 1 && store.is_external_player_passenger(passengers[0]))
        || data.ageable_retained().is_some_and(|age| age.age_locked())
        || data
            .mob
            .as_ref()
            .is_some_and(|mob| mob.persistence_required())
    {
        return;
    }
    let delay = trader_delay.unwrap_or(delay).wrapping_sub(1);
    entity.set_trader_despawn_delay(delay);
    if delay <= 0 {
        crate::entity::leash::drop_leash_store_spawn(entity, store, true, false);
        entity.discard();
    }
}

// EntityType.java:1296-1297,1327-1344.
pub fn create_trader_event_root(
    store: &mut EntityStore,
    resource_id: &str,
    position: Vec3d,
) -> Option<Box<Entity>> {
    if !matches!(
        resource_id,
        "minecraft:wandering_trader" | "minecraft:trader_llama"
    ) {
        return None;
    }
    crate::entity::save::create_fresh_entity(store, resource_id, position)
}

impl EntityStore {
    pub fn play_trader_spawn_ambient(&mut self, entity_id: i32) -> bool {
        if !self.get(entity_id).is_some_and(|entity| {
            matches!(
                entity.entity_type_id(),
                VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            )
        }) {
            return false;
        }
        self.play_spawn_ambient(entity_id, None)
    }
}

// WanderingTrader.java:166-168.
pub(super) fn ambient_sound(entity: &Entity) -> &'static str {
    if entity.merchant_trading_player().is_some() {
        "minecraft:entity.wandering_trader.trade"
    } else {
        "minecraft:entity.wandering_trader.ambient"
    }
}

// `Mob.baseTick` (`Mob.java:282-285`) over `WanderingTrader.getAmbientSound`
// (`WanderingTrader.java:166-168`).
pub(crate) fn tick_ambient_sound(entity: &Entity, store: &mut EntityStore) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
        return;
    }
    let baby = is_baby(entity);
    let Some(runtime) = store.mob_ai_mut(entity.id()) else {
        return;
    };
    if !runtime.tick_mob_ambient_sound(entity.is_alive()) {
        return;
    }
    let pitch = runtime.voice_pitch(baby);
    if !entity.is_silent() {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position: entity.position(),
            resource_id: ambient_sound(entity).into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }
}

fn is_baby(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(data) if data.ageable_retained().is_some_and(|age| age.is_baby()))
}

impl EntityStore {
    /// `AbstractVillager.notifyTrade` (`AbstractVillager.java:134-141`) for a
    /// wandering trader: the offer's uses, the ambient clock and
    /// `WanderingTrader.rewardTradeXp` (`WanderingTrader.java:158-163`). The
    /// `TRADE` criterion belongs to the caller.
    pub fn wandering_trader_notify_trade(&mut self, entity_id: i32, offer: usize) -> bool {
        let Some(entity) = self.get_mut(entity_id) else {
            return false;
        };
        if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
            return false;
        }
        let position = entity.position();
        let Some(offer) = entity
            .merchant_offers_mut()
            .and_then(|offers| offers.offers_mut().get_mut(offer))
        else {
            return false;
        };
        offer.increase_uses();
        let reward_exp = offer.reward_exp();
        let Some(runtime) = self.mob_ai_mut(entity_id) else {
            return false;
        };
        runtime.set_ambient_sound_time(-80);
        if reward_exp {
            let xp = runtime.merchant_trade_xp();
            crate::entity::experience_orb::spawn_experience_orb(
                self,
                Vec3d {
                    x: position.x,
                    y: position.y + 0.5,
                    z: position.z,
                },
                xp,
            );
        }
        true
    }

    /// `AbstractVillager.notifyTradeUpdated` (`AbstractVillager.java:151-156`)
    /// with `WanderingTrader.getTradeUpdatedSound` (`WanderingTrader.java:186-188`).
    pub fn wandering_trader_notify_trade_updated(&mut self, entity_id: i32, valid: bool) {
        let Some(entity) = self.get(entity_id) else {
            return;
        };
        if entity.entity_type_id() != VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
            return;
        }
        let position = entity.position();
        let silent = entity.is_silent();
        let baby = is_baby(entity);
        let Some(runtime) = self.mob_ai_mut(entity_id) else {
            return;
        };
        if runtime.ambient_sound_time() <= -80 + 20 {
            return;
        }
        runtime.set_ambient_sound_time(-80);
        let pitch = runtime.voice_pitch(baby);
        if !silent {
            self.play_entity_sound(crate::entity::store::PendingEntitySound {
                position,
                resource_id: if valid {
                    "minecraft:entity.wandering_trader.yes"
                } else {
                    "minecraft:entity.wandering_trader.no"
                }
                .into(),
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch,
            });
        }
    }
}

pub(crate) fn read_save_data(id: i32, entries: &[NbtSummaryEntry]) -> TraderState {
    let mut state = TraderState::new(id);
    state.delay = crate::entity::save::optional_int(entries, "DespawnDelay").unwrap_or(state.delay);
    if id == VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID {
        state.target = crate::entity::save::optional_block_pos(entries, "wander_target");
    }
    state
}
pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: &TraderState) {
    entries.push(NbtSummaryEntry {
        name: "DespawnDelay".into(),
        value: NbtSummaryValue::Int(state.delay),
    });
    if let Some(target) = state.target {
        entries.push(block_pos_entry("wander_target", target));
    }
}
pub(crate) fn block_pos_entry(name: &str, pos: BlockPos) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
    }
}
