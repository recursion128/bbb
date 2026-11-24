//! A session's `LivingEntity.useItem` / `useItemRemaining`
//! (`LivingEntity.java:248-249`) and the `DATA_LIVING_ENTITY_FLAGS` byte
//! (`:178`) that publishes them, driven by `startUsingItem`,
//! `updatingUsingItem`, `completeUsingItem`, `releaseUsingItem` and
//! `stopUsingItem` (`:3361-3562`) with `ServerPlayer`'s overrides. The class
//! hooks those reach (`getUseDuration`, `onUseTick`, `finishUsingItem`,
//! `releaseUsing`) are matched in [`UseItemClass`], which is where a ranged
//! weapon joins this state.

mod blocking;
mod brush;
mod components;
mod consume;
mod death_protection;
mod equippable;
mod random_teleport;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::inventory::is_empty;
use crate::item_durability::{
    hurt_and_break, MineBlockOutcome, ENTITY_EVENT_MAINHAND_ITEM_BROKEN,
    ENTITY_EVENT_OFFHAND_ITEM_BROKEN,
};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaItemUseOnOverride, VanillaItemUseOverride,
};
use bbb_protocol::packets::{EntityEvent, InteractionHand, ItemStackSummary};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_BROKEN_ID;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::advancement_triggers::USING_ITEM;
use super::item_use::{ItemUseContext, ItemUseResult};
use super::{send, PlayerState, ServerWorld, WorldEvent};

pub(crate) use components::weapon_disable_blocking_seconds;
pub(in crate::world) use components::{instrument as instrument_component, InstrumentRef};

fn remainder_created_on_use(
    used: &RuntimeItemStack,
    before: &RuntimeItemStack,
    infinite_materials: bool,
) -> Result<Option<RuntimeItemStack>, String> {
    if infinite_materials || used.count() >= before.count() {
        return Ok(None);
    }
    Ok(components::use_remainder(before.summary())?.map(RuntimeItemStack::from_summary))
}

/// `LivingEntity.LIVING_ENTITY_FLAG_IS_USING` (`LivingEntity.java:175`).
const LIVING_ENTITY_FLAG_IS_USING: i8 = 1;
/// `LivingEntity.LIVING_ENTITY_FLAG_OFF_HAND` (`LivingEntity.java:176`).
const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 2;
/// `LivingEntity.LIVING_ENTITY_FLAG_SPIN_ATTACK` (`LivingEntity.java:177`).
const LIVING_ENTITY_FLAG_SPIN_ATTACK: i8 = 4;

/// `ServerPlayer.completeUsingItem`'s `ClientboundEntityEventPacket(this, 9)`
/// (`ServerPlayer.java:1567-1572`).
const ENTITY_EVENT_USE_ITEM_COMPLETE: i8 = 9;

/// `SpyglassItem.USE_DURATION` (`SpyglassItem.java:12`).
const SPYGLASS_USE_DURATION: i32 = 1200;
/// `BrushItem.USE_DURATION` (`BrushItem.java:31`).
const BRUSH_USE_DURATION: i32 = 200;
/// `Item.getUseDuration`'s blocking and kinetic answer (`Item.java:309-316`).
const HELD_USE_DURATION: i32 = 72000;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct UsingItemState {
    pub(super) use_item: RuntimeItemStack,
    use_item_remaining: i32,
    /// `LivingEntity.autoSpinAttackTicks`, `autoSpinAttackDmg` and
    /// `autoSpinAttackItemStack` (`LivingEntity.java:256-258`).
    pub(super) auto_spin_attack: Option<(i32, f32, RuntimeItemStack)>,
    /// `LivingEntity.recentKineticEnemies` (`LivingEntity.java:259`): when each
    /// entity id was last struck by this kinetic use, and whether it is living.
    pub(super) recent_kinetic_enemies: Option<std::collections::BTreeMap<i32, (i64, bool)>>,
    living_entity_flags: i8,
    living_entity_flags_dirty: bool,
}

impl Default for UsingItemState {
    fn default() -> Self {
        Self {
            use_item: RuntimeItemStack::empty(),
            use_item_remaining: 0,
            auto_spin_attack: None,
            recent_kinetic_enemies: None,
            living_entity_flags: 0,
            living_entity_flags_dirty: false,
        }
    }
}

impl UsingItemState {
    /// `LivingEntity.isUsingItem` (`LivingEntity.java:3353-3355`).
    pub(super) fn is_using_item(&self) -> bool {
        self.living_entity_flags & LIVING_ENTITY_FLAG_IS_USING != 0
    }

    /// `LivingEntity.getUsedItemHand` (`LivingEntity.java:3357-3359`).
    pub(super) fn used_item_hand(&self) -> InteractionHand {
        if self.living_entity_flags & LIVING_ENTITY_FLAG_OFF_HAND != 0 {
            InteractionHand::OffHand
        } else {
            InteractionHand::MainHand
        }
    }

    /// `LivingEntity.isAutoSpinAttack` (`LivingEntity.java:3212-3214`).
    pub(super) fn is_auto_spin_attack(&self) -> bool {
        self.living_entity_flags & LIVING_ENTITY_FLAG_SPIN_ATTACK != 0
    }

    /// `Player.startAutoSpinAttack` (`Player.java:510-518`).
    pub(super) fn start_auto_spin_attack(
        &mut self,
        ticks: i32,
        damage: f32,
        item: RuntimeItemStack,
    ) {
        self.auto_spin_attack = Some((ticks, damage, item));
        self.set_living_entity_flag(LIVING_ENTITY_FLAG_SPIN_ATTACK, true);
    }

    /// The end of `LivingEntity.checkAutoSpinAttack` (`LivingEntity.java:3198-3202`).
    pub(super) fn end_auto_spin_attack(&mut self) {
        self.auto_spin_attack = None;
        self.set_living_entity_flag(LIVING_ENTITY_FLAG_SPIN_ATTACK, false);
    }

    pub(super) fn living_entity_flags(&self) -> i8 {
        self.living_entity_flags
    }

    pub(super) fn take_living_entity_flags_dirty(&mut self) -> bool {
        std::mem::take(&mut self.living_entity_flags_dirty)
    }

    /// `LivingEntity.setLivingEntityFlag` (`LivingEntity.java:3422-3431`)
    /// through `SynchedEntityData.set`, which dirties only a changed value.
    fn set_living_entity_flag(&mut self, flag: i8, value: bool) {
        let next = if value {
            self.living_entity_flags | flag
        } else {
            self.living_entity_flags & !flag
        };
        if next != self.living_entity_flags {
            self.living_entity_flags = next;
            self.living_entity_flags_dirty = true;
        }
    }
}

impl PlayerState {
    /// `LivingEntity.useItem.isEmpty()` (`LivingEntity.java:3506`).
    pub(super) fn use_item_is_empty(&self) -> bool {
        self.using_item.use_item.is_empty()
    }
}

pub(super) fn runtime_item_in_hand(
    player: &PlayerState,
    hand: InteractionHand,
) -> &bbb_server_types::item_stack::RuntimeItemStack {
    match hand {
        InteractionHand::MainHand => player.inventory.selected_stack(),
        InteractionHand::OffHand => player.inventory.off_hand_stack(),
    }
}

/// The `Item` subclass whose use hooks a stack reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseItemClass {
    Spyglass,
    Instrument,
    Brush,
    Bundle,
    Bow,
    Crossbow,
    Trident,
    Item,
}

fn use_item_class(stack: &ItemStackSummary) -> UseItemClass {
    let Some(entry) = stack.item_id.and_then(vanilla_item_type) else {
        return UseItemClass::Item;
    };
    match (entry.use_override, entry.use_on_override) {
        (Some(VanillaItemUseOverride::SpyglassItem), _) => UseItemClass::Spyglass,
        (Some(VanillaItemUseOverride::InstrumentItem), _) => UseItemClass::Instrument,
        (Some(VanillaItemUseOverride::BundleItem), _) => UseItemClass::Bundle,
        (Some(VanillaItemUseOverride::BowItem), _) => UseItemClass::Bow,
        (Some(VanillaItemUseOverride::CrossbowItem), _) => UseItemClass::Crossbow,
        (Some(VanillaItemUseOverride::TridentItem), _) => UseItemClass::Trident,
        (_, Some(VanillaItemUseOnOverride::BrushItem)) => UseItemClass::Brush,
        _ => UseItemClass::Item,
    }
}

/// The stack an `ItemStack` call answers with: the one it ran on, in whatever
/// state it left that stack, or another.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum HeldItem {
    Same,
    Replaced(RuntimeItemStack),
}

impl ServerWorld {
    /// `ItemStack.getUseDuration` (`ItemStack.java:732-734`).
    pub(super) fn use_duration(&self, stack: &ItemStackSummary) -> Result<i32, String> {
        if is_empty(stack) {
            return Ok(0);
        }
        match use_item_class(stack) {
            UseItemClass::Spyglass => Ok(SPYGLASS_USE_DURATION),
            // `InstrumentItem.getUseDuration` (`InstrumentItem.java:46-49`).
            UseItemClass::Instrument => Ok(self
                .instrument_facts(stack)?
                .map_or(0, |instrument| instrument.use_ticks())),
            UseItemClass::Brush => Ok(BRUSH_USE_DURATION),
            UseItemClass::Bundle => Ok(super::item_use::bundle::BUNDLE_USE_DURATION),
            UseItemClass::Bow => Ok(super::item_use::bow::USE_DURATION),
            UseItemClass::Crossbow => Ok(super::item_use::crossbow::USE_DURATION),
            UseItemClass::Trident => Ok(super::item_use::trident::USE_DURATION),
            UseItemClass::Item => {
                if let Some(consumable) = components::consumable(stack)? {
                    return Ok(consumable.component.consume_ticks());
                }
                let held = components::has_blocks_attacks(stack)?
                    || components::has_kinetic_weapon(stack)?;
                Ok(if held { HELD_USE_DURATION } else { 0 })
            }
        }
    }

    /// `BlockItem.useOn` falling through to `Item.use` (`BlockItem.java:41-46`): a refused placement
    /// of a block item with `consumable` starts eating it; true on success.
    pub(super) fn consume_unplaced_block_item(
        &mut self,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Ok(Some(consumable)) = components::consumable(&context.item) else {
            return false;
        };
        match self.start_consuming(context, &consumable, events) {
            Ok((result, _)) => matches!(result, ItemUseResult::Consume | ItemUseResult::Success),
            Err(error) => {
                tracing::warn!("item use: {error}");
                false
            }
        }
    }

    /// `Item.use` (`Item.java:188-210`) for a stack whose class keeps it.
    pub(super) fn use_item_by_components(
        &mut self,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(ItemUseResult, Option<HeldItem>), String> {
        if let Some(consumable) = components::consumable(&context.item)? {
            return self.start_consuming(context, &consumable, events);
        }
        if components::swappable_equippable(&context.item)? {
            return self.swap_player_equipment(context, events);
        }
        if components::has_blocks_attacks(&context.item)? {
            self.start_using_item(context.connection_id, context.hand);
            return Ok((ItemUseResult::Consume, None));
        }
        if let Some(kinetic) = crate::entity::spear::kinetic_weapon(&context.item) {
            self.start_using_item(context.connection_id, context.hand);
            // `KineticWeapon.makeSound` (`KineticWeapon.java:86-91`).
            if let Some(sound) = kinetic.sound {
                let id = context.connection_id;
                self.play_player_sound(
                    id,
                    Some(id),
                    sound,
                    bbb_protocol::packets::SoundSource::Players,
                    self.levels[id].position(),
                    1.0,
                    1.0,
                    events,
                );
            }
            return Ok((ItemUseResult::Consume, None));
        }
        Ok((ItemUseResult::Pass, None))
    }

    /// `LivingEntity.startUsingItem` (`LivingEntity.java:3433-3447`).
    pub(super) fn start_using_item(&mut self, connection_id: ConnectionId, hand: InteractionHand) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let stack = runtime_item_in_hand(player, hand).clone();
        if stack.is_empty() || player.using_item.is_using_item() {
            return;
        }
        let summary = stack.summary().clone();
        let duration = match self.use_duration(&summary) {
            Ok(duration) => duration,
            Err(error) => {
                tracing::warn!("item use: {error}");
                return;
            }
        };
        let state = &mut self.levels[connection_id].using_item;
        state.use_item = stack;
        state.use_item_remaining = duration;
        state.set_living_entity_flag(LIVING_ENTITY_FLAG_IS_USING, true);
        state.set_living_entity_flag(
            LIVING_ENTITY_FLAG_OFF_HAND,
            hand == InteractionHand::OffHand,
        );
        if components::has_kinetic_weapon(&summary).unwrap_or(false) {
            state.recent_kinetic_enemies = Some(Default::default());
        }
        self.cause_use_vibration(
            connection_id,
            &summary,
            VanillaGameEvent::ITEM_INTERACT_START,
        );
    }

    /// `LivingEntity.stopUsingItem` (`LivingEntity.java:3550-3562`).
    pub(super) fn stop_using_item(&mut self, connection_id: ConnectionId) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let state = &mut player.using_item;
        let was_using_item = state.is_using_item();
        state.set_living_entity_flag(LIVING_ENTITY_FLAG_IS_USING, false);
        let use_item = std::mem::replace(&mut state.use_item, RuntimeItemStack::empty());
        state.use_item_remaining = 0;
        state.recent_kinetic_enemies = None;
        if was_using_item {
            let summary = use_item.summary();
            self.cause_use_vibration(
                connection_id,
                summary,
                VanillaGameEvent::ITEM_INTERACT_FINISH,
            );
        }
    }

    /// `LivingEntity.releaseUsingItem` (`LivingEntity.java:3537-3548`).
    pub(super) fn release_using_item(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Err(error) = self.release_using_item_checked(connection_id, events) {
            self.cancel_failed_item_use(connection_id, error, events);
        }
    }

    fn release_using_item_checked(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return Ok(());
        };
        let hand = player.using_item.used_item_hand();
        let in_used_hand = runtime_item_in_hand(player, hand).clone();
        if !player.using_item.use_item.is_empty()
            && in_used_hand.item_id() == player.using_item.use_item.item_id()
        {
            player.using_item.use_item = in_used_hand;
            let remaining = player.using_item.use_item_remaining;
            self.release_using(connection_id, hand, remaining, events)?;
            let use_item = self.levels[connection_id].using_item.use_item.clone();
            if use_on_release(&use_item) {
                self.updating_using_item_checked(connection_id, events)?;
            }
        }
        self.stop_using_item(connection_id);
        Ok(())
    }

    fn cancel_failed_item_use(
        &mut self,
        connection_id: ConnectionId,
        error: String,
        events: &mut Vec<WorldEvent>,
    ) {
        self.stop_using_item(connection_id);
        let inventory = &mut self.levels[connection_id].inventory;
        match inventory.send_all_data_to_remote() {
            Ok(menu) => {
                tracing::warn!(?connection_id, "item use: {error}");
                Self::send_menu_sync(connection_id, [menu], events);
            }
            Err(sync_error) => {
                tracing::warn!(
                    ?connection_id,
                    "item use: {error}; inventory sync: {sync_error}"
                );
            }
        }
    }

    /// `LivingEntity.tick`'s `updatingUsingItem` (`LivingEntity.java:2692`)
    /// for every session with a body.
    pub(super) fn tick_player_using_items(&mut self, events: &mut Vec<WorldEvent>) {
        let using = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                !player.avatar_removed
                    && player.body_tick_enabled
                    && player.using_item.is_using_item()
            })
            .map(|(connection_id, _)| *connection_id)
            .collect::<Vec<_>>();
        for connection_id in using {
            if let Err(error) = self.updating_using_item_checked(connection_id, events) {
                self.cancel_failed_item_use(connection_id, error, events);
            }
        }
    }

    /// `LivingEntity.updatingUsingItem` (`LivingEntity.java:3361-3370`).
    fn updating_using_item_checked(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return Ok(());
        };
        if !player.using_item.is_using_item() {
            return Ok(());
        }
        let in_hand = runtime_item_in_hand(player, player.using_item.used_item_hand()).clone();
        if in_hand.item_id() == player.using_item.use_item.item_id() {
            player.using_item.use_item = in_hand;
            self.update_using_item(connection_id, events)?;
        } else {
            self.stop_using_item(connection_id);
        }
        Ok(())
    }

    /// `ServerPlayer.updateUsingItem` (`ServerPlayer.java:2059-2062`) over
    /// `LivingEntity.updateUsingItem` (`LivingEntity.java:3406-3411`).
    fn update_using_item(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let use_item = self.levels[connection_id].using_item.use_item.clone();
        let summary = use_item.summary();
        self.trigger_item_criterion(connection_id, USING_ITEM, summary, events);
        let remaining = self.levels[connection_id].using_item.use_item_remaining;
        self.on_use_tick(connection_id, &use_item, remaining, events)?;
        let Some(player) = self.levels.player_mut(connection_id) else {
            return Ok(());
        };
        player.using_item.use_item_remaining -= 1;
        if player.using_item.use_item_remaining == 0 && !use_on_release(&use_item) {
            self.complete_using_item(connection_id, events)?;
        }
        Ok(())
    }

    /// `ServerPlayer.completeUsingItem` (`ServerPlayer.java:1567-1572`) over
    /// `LivingEntity.completeUsingItem` (`LivingEntity.java:3500-3516`).
    fn complete_using_item(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let player = self.levels.player_at(connection_id);
        if player.using_item.use_item.is_empty() || !player.using_item.is_using_item() {
            return Ok(());
        }
        let entity_id = player.entity_id();
        let completion_index = events.len();
        let hand = player.using_item.used_item_hand();
        // Approximates `useItem != getItemInHand(hand)` by value.
        if player.using_item.use_item != *runtime_item_in_hand(player, hand) {
            self.release_using_item_checked(connection_id, events)?;
        } else {
            if let HeldItem::Replaced(stack) = self.finish_using_item(connection_id, events)? {
                self.set_player_item_in_hand(connection_id, hand, stack, events);
            }
            self.stop_using_item(connection_id);
        }
        events.insert(
            completion_index,
            send(
                connection_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id,
                    event_id: ENTITY_EVENT_USE_ITEM_COMPLETE,
                }),
            ),
        );
        Ok(())
    }

    /// `ItemStack.finishUsingItem` (`ItemStack.java:391-395`) on the stack in
    /// `hand`, which is the session's `useItem`.
    fn finish_using_item(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Result<HeldItem, String> {
        let using = &self.levels[connection_id].using_item;
        let (hand, before) = (using.used_item_hand(), using.use_item.clone());
        let summary = before.summary();
        match use_item_class(summary) {
            UseItemClass::Spyglass => self.stop_using_spyglass(connection_id, events),
            UseItemClass::Instrument
            | UseItemClass::Brush
            | UseItemClass::Bundle
            | UseItemClass::Bow
            | UseItemClass::Crossbow
            | UseItemClass::Trident
            | UseItemClass::Item => {
                if let Some(consumable) = components::consumable(summary)? {
                    self.on_consume(connection_id, hand, &consumable, events)?;
                }
            }
        }
        let used = self.levels[connection_id].using_item.use_item.clone();
        self.apply_after_use_component_side_effects(connection_id, &used, &before, events)
    }

    /// `ItemStack.releaseUsing` (`ItemStack.java:740-748`).
    fn release_using(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        remaining: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let before = self.levels[connection_id].using_item.use_item.clone();
        let summary = before.summary();
        let released = match use_item_class(summary) {
            UseItemClass::Spyglass => {
                self.stop_using_spyglass(connection_id, events);
                true
            }
            UseItemClass::Bow => {
                super::item_use::bow::release_using(self, connection_id, hand, remaining, events)
            }
            UseItemClass::Crossbow => {
                super::item_use::crossbow::release_using(self, connection_id, &before, remaining)
            }
            UseItemClass::Trident => super::item_use::trident::release_using(
                self,
                connection_id,
                hand,
                remaining,
                events,
            ),
            UseItemClass::Instrument
            | UseItemClass::Brush
            | UseItemClass::Bundle
            | UseItemClass::Item => false,
        };
        if !released {
            return Ok(());
        }
        if let HeldItem::Replaced(stack) =
            self.apply_after_use_component_side_effects(connection_id, &before, &before, events)?
        {
            self.set_player_item_in_hand(connection_id, hand, stack, events);
        }
        Ok(())
    }

    /// `ItemStack.onUseTick` (`ItemStack.java:1094-1106`).
    fn on_use_tick(
        &mut self,
        connection_id: ConnectionId,
        use_item: &RuntimeItemStack,
        remaining: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let summary = use_item.summary();
        if let Some(consumable) = components::consumable(summary)? {
            if consumable.component.emits_feedback(remaining) {
                self.emit_player_consume_sound(connection_id, &consumable, events);
            }
        }
        if components::has_kinetic_weapon(summary)? {
            self.player_kinetic_use_tick(connection_id, use_item, remaining, events);
            return Ok(());
        }
        match use_item_class(summary) {
            UseItemClass::Brush => {
                self.brush_on_use_tick(connection_id, use_item, remaining, events);
            }
            UseItemClass::Bundle => {
                let hand = self.levels[connection_id].using_item.used_item_hand();
                self.bundle_on_use_tick(connection_id, hand, remaining, events);
            }
            UseItemClass::Crossbow => {
                let hand = self.levels[connection_id].using_item.used_item_hand();
                super::item_use::crossbow::on_use_tick(
                    self,
                    connection_id,
                    hand,
                    use_item,
                    remaining,
                    events,
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// `ItemStack.applyAfterUseComponentSideEffects` (`ItemStack.java:397-411`)
    /// on `used`, given its copy from before the use.
    pub(super) fn apply_after_use_component_side_effects(
        &mut self,
        connection_id: ConnectionId,
        used: &RuntimeItemStack,
        before: &RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) -> Result<HeldItem, String> {
        let result = self.apply_use_remainder(connection_id, used, before)?;
        self.apply_use_cooldown(connection_id, before.summary(), events);
        Ok(result)
    }

    pub(super) fn apply_use_remainder(
        &mut self,
        connection_id: ConnectionId,
        used: &RuntimeItemStack,
        before: &RuntimeItemStack,
    ) -> Result<HeldItem, String> {
        let mut result = HeldItem::Same;
        if let Some(remainder) = remainder_created_on_use(
            used,
            before,
            self.levels[connection_id].abilities.instabuild,
        )? {
            if used.is_empty() {
                result = HeldItem::Replaced(remainder);
            } else {
                self.handle_extra_items_created_on_use(connection_id, remainder)?;
            }
        }
        Ok(result)
    }

    /// `ServerPlayer.handleExtraItemsCreatedOnUse` (`ServerPlayer.java:2078-2082`).
    fn handle_extra_items_created_on_use(
        &mut self,
        connection_id: ConnectionId,
        mut extra: RuntimeItemStack,
    ) -> Result<(), String> {
        let resolver = self.item_stack_resolver();
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the session outlives its own use");
        let creative = player.abilities.instabuild;
        let added = player
            .inventory
            .add_anywhere(&resolver, &mut extra, creative)
            .map_err(|error| error.to_string())?;
        if !added && !extra.is_empty() {
            self.drop_player_original_stack(connection_id, extra, false)
                .ok_or_else(|| "use remainder publication refused".to_owned())?;
        }
        Ok(())
    }

    /// `LivingEntity.setItemInHand` (`LivingEntity.java:2224-2234`) through
    /// `setItemSlot`'s `onEquipItem` (`:2248-2250`).
    pub(super) fn set_player_item_in_hand(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        stack: RuntimeItemStack,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the session outlives its own use");
        let old = runtime_item_in_hand(player, hand).clone();
        player.inventory.set_item_in_hand(hand, stack.clone());
        let slot = match hand {
            InteractionHand::MainHand => bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
            InteractionHand::OffHand => bbb_protocol::equipment_slots::EquipmentSlot::OffHand,
        };
        self.player_equip_item(connection_id, slot, &old, &stack, events);
    }

    /// Writes the stack in `hand` in place; the in-use copy follows it while that hand is in use.
    pub(super) fn update_hand_stack(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        stack: RuntimeItemStack,
    ) {
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the session outlives its own use");
        if player.using_item.is_using_item() && player.using_item.used_item_hand() == hand {
            player.using_item.use_item = stack.clone();
        }
        *player.inventory.item_in_hand_mut(hand) = stack;
    }

    /// `ItemStack.hurtAndBreak` on the stack in `hand`; answers whether it is still there.
    pub(super) fn hurt_and_break_in_hand(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        amount: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let has_infinite_materials = self.player_has_infinite_materials(connection_id);
        let table = std::sync::Arc::clone(&self.datapack_tables.enchantment_item_damage);
        let player = self.levels.player_at(connection_id);
        let entity_id = player.entity_id();
        let value_level = self.value_effect_level(&player.dimension);
        let native_stack = runtime_item_in_hand(player, hand).clone();
        let mut stack = native_stack.summary().clone();
        let enchantments = Self::resolve_value_effects(&stack, &table);
        let mut outcome = MineBlockOutcome::default();
        hurt_and_break(
            &mut stack,
            amount,
            has_infinite_materials,
            &enchantments,
            &self.random,
            &value_level,
            &mut outcome,
        );
        drop(enchantments);
        if let Some(raw_new_damage) = outcome.raw_new_damage {
            self.trigger_item_durability_changed(
                connection_id,
                &native_stack,
                raw_new_damage,
                events,
            );
        }
        let stack = RuntimeItemStack::from_summary(stack);
        let unbroken = !stack.is_empty();
        self.update_hand_stack(connection_id, hand, stack);
        let Some(broken_item_id) = outcome.broken_item_id else {
            return unbroken;
        };
        let event_id = match hand {
            InteractionHand::MainHand => ENTITY_EVENT_MAINHAND_ITEM_BROKEN,
            InteractionHand::OffHand => ENTITY_EVENT_OFFHAND_ITEM_BROKEN,
        };
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id,
            }),
            events,
        );
        self.levels[connection_id].award_stat_once(
            &mut self.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_BROKEN_ID, broken_item_id),
        );
        unbroken
    }

    /// `ItemStack.causeUseVibration` (`ItemStack.java:750-755`).
    pub(super) fn cause_use_vibration(
        &mut self,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
        event: VanillaGameEvent,
    ) {
        if !bbb_server_types::item_stack::rules::interact_vibrations(stack) {
            return;
        }
        self.player_game_event(connection_id, event);
    }

    /// `Entity.gameEvent(event)` (`Entity.java:1335-1337`) for a session.
    pub(super) fn player_game_event(
        &mut self,
        connection_id: ConnectionId,
        event: VanillaGameEvent,
    ) {
        let player = &self
            .levels
            .player(connection_id)
            .expect("the player is online");
        let position = player.position();
        self.player_game_event_at(connection_id, event, position, None);
    }

    pub(super) fn player_game_event_at(
        &mut self,
        target: ConnectionId,
        event: VanillaGameEvent,
        position: bbb_protocol::packets::Vec3d,
        affected_state: Option<i32>,
    ) {
        let player = self
            .levels
            .player(target)
            .expect("game event source is online");
        let dimension = player.dimension;
        let entity_id = player.entity_id();
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source_at(event, position, Some(entity_id), affected_state);
    }
}

/// `ItemStack.useOnRelease` (`ItemStack.java:757-759`): only
/// `CrossbowItem` answers true (`CrossbowItem.java:269-271`).
fn use_on_release(stack: &RuntimeItemStack) -> bool {
    use_item_class(stack.summary()) == UseItemClass::Crossbow
}

#[cfg(test)]
impl ServerWorld {
    /// `(isUsingItem, getUsedItemHand, useItemRemaining, useItem)`.
    pub(crate) fn player_use_state_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (bool, InteractionHand, i32, ItemStackSummary) {
        let state = &self.levels[connection_id].using_item;
        (
            state.is_using_item(),
            state.used_item_hand(),
            state.use_item_remaining,
            state.use_item.summary().clone(),
        )
    }

    /// `(FoodData.foodLevel, FoodData.saturationLevel)`.
    pub(crate) fn player_food_levels_for_test(&self, connection_id: ConnectionId) -> (i32, f32) {
        let food = &self.levels[connection_id].food;
        (food.food_level(), food.saturation_level())
    }

    pub(crate) fn player_is_on_cooldown_for_test(
        &self,
        connection_id: ConnectionId,
        stack: &ItemStackSummary,
    ) -> bool {
        self.levels[connection_id]
            .cold
            .cooldowns
            .is_on_cooldown(stack)
            .unwrap_or(false)
    }

    pub(crate) fn player_inventory_slot_for_test(
        &self,
        connection_id: ConnectionId,
        index: usize,
    ) -> ItemStackSummary {
        self.levels[connection_id].inventory.inventory_slots()[index]
            .summary()
            .clone()
    }

    pub(crate) fn player_runtime_inventory_slot_for_test(
        &self,
        connection_id: ConnectionId,
        index: usize,
    ) -> bbb_server_types::item_stack::RuntimeItemStack {
        self.levels[connection_id].inventory.inventory_slots()[index].clone()
    }

    pub(crate) fn item_stack_resolver_for_test(
        &self,
    ) -> crate::world::registry_runtime::PlayerItemStackResolver {
        self.item_stack_resolver()
    }
}
