//! Using an item on nothing -- `ServerPlayerGameMode.useItem` -- and which
//! items answer it with an `Item.use` of their own.
//!
//! [`ServerWorld::use_item_in_hand`] is the `useItem` skeleton
//! (`ServerPlayerGameMode.java:304-347`): the spectator and cooldown refusals,
//! `ItemStack.use`'s after-use side effects, and the stack comparison that
//! decides whether the menu is republished. It names no item. Which item
//! runs which body is the [`item_use_dispatch!`] table, one row per vanilla
//! `Item` subclass that overrides `use`, keyed by the generated catalog's
//! `use_override` column so a row cannot name an item the catalog does not
//! give that class.
//!
//! Giving another item its `use` is a new `world/item_use/<item>.rs` holding
//! the body, its `mod` line, and one row naming the module and the
//! `VanillaItemUseOverride` variant it answers for. A class with no row yet
//! answers `PASS`; an item with no override runs the generic `Item.use`
//! (`Item.java:188-210`) off its components.
//!
//! The `useItemOn` cooldown gate (`ServerPlayerGameMode.java:386`) is not
//! mirrored here: it stands in front of an item's own `useOn`, and no table
//! item overrides `useOn`, so a right click on a block never reaches a throw.
//! The one cooldown a table item starts -- the ender pearl's -- is gated in
//! [`ServerWorld::use_item_in_hand`], which is the `useItem` (air) path the
//! throw actually takes.

mod boat;
mod book;
mod bottle;
pub(super) mod bow;
pub(in crate::world) mod bucket;
pub(in crate::world) mod bundle;
pub(super) mod crossbow;
mod egg;
mod empty_map;
mod ender_eye;
mod ender_pearl;
mod experience_bottle;
mod firework_rocket;
mod fishing_rod;
mod food_on_a_stick;
mod instrument;
mod place_on_water;
mod projectile_weapon;
mod snowball;
mod spawn_egg;
mod spyglass;
#[cfg(test)]
mod tests;
mod throwable_potion;
pub(super) mod trident;
mod wind_charge;

use book::{knowledge_book, writable_book, written_book};
use throwable_potion::{lingering_potion, splash_potion};

use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::{vanilla_item_type, VanillaItemUseOverride};
use bbb_protocol::packets::{
    BlockPos, InteractionHand, ItemStackSummary, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::fluid::declared_block_type_ids_in_tag;
use bbb_server_types::item_stack::{static_item_component, RuntimeItemStack};
use uuid::Uuid;

use crate::entity::projectile_shoot::{
    shoot_from_rotation, ProjectileShot, ShotParameters, ShotSource,
};
use crate::entity::store::EntityStore;
use bbb_server_types::stats::StatKey;

use super::using_item::{runtime_item_in_hand, HeldItem};
use super::{LevelId, ServerWorld, WorldEvent};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

/// The table that says which `Item` subclass answers `ItemStack.use` here, and
/// the only place an item class is named on this surface.
///
/// A row is:
///
/// ```text
/// module: VanillaItemUseOverride::Class,
/// ```
///
/// * the row key is the module -- `world/item_use/<module>.rs` -- holding the
///   body as `use_item`. The `mod` line stays above for the same rustfmt
///   reason [`super::block_use`] keeps its own.
/// * the variant is the generated catalog's `use_override` for every item the
///   class owns (`ItemTypeCatalogEntry.use_override`), which is how three egg
///   items reach one `EggItem` row without a list written here.
///
/// The guard test holds every variant the table names against the catalog:
/// a row for a class no item carries is dead, and the items of a class the
/// table lacks fall through to the generic `Item.use`, which is the honest
/// answer for them until their row exists.
macro_rules! item_use_dispatch {
    ($($module:ident: VanillaItemUseOverride::$class:ident),* $(,)?) => {
        /// The table, in the order the dispatcher walks it.
        const ITEM_USE_ROWS: &[ItemUseRow] = &[
            $(
                ItemUseRow {
                    class: VanillaItemUseOverride::$class,
                    use_item: |world, context, events| $module::use_item(world, context, events).into(),
                },
            )*
        ];

        /// The row for one item, if its class has one.
        fn item_use_row(item: &ItemStackSummary) -> Option<&'static ItemUseRow> {
            let class = vanilla_item_type(item.item_id?)?.use_override?;
            ITEM_USE_ROWS.iter().find(|row| row.class == class)
        }
    };
}

/// What `ServerPlayerGameMode.useItem` has in hand when it calls
/// `ItemStack.use`: the session, its level, the hand and the stack read out of
/// that hand once.
pub(super) struct ItemUseContext {
    pub(super) connection_id: ConnectionId,
    pub(super) dimension: LevelId,
    pub(super) hand: InteractionHand,
    pub(super) item: ItemStackSummary,
}

/// `Item.use` (`Item.java:188`) for one item class.
type ItemUseHandler = fn(&mut ServerWorld, &ItemUseContext, &mut Vec<WorldEvent>) -> ItemUseOutcome;

struct ItemUseOutcome {
    result: ItemUseResult,
    transformed: Option<HeldItem>,
}

impl From<ItemUseResult> for ItemUseOutcome {
    fn from(result: ItemUseResult) -> Self {
        Self {
            result,
            transformed: None,
        }
    }
}

/// One row of the [`item_use_dispatch!`] table.
struct ItemUseRow {
    class: VanillaItemUseOverride,
    use_item: ItemUseHandler,
}

/// The `InteractionResult` shapes `ServerPlayerGameMode.useItem` and
/// `handleUseItem` read off an `Item.use`
/// (`ServerPlayerGameMode.java:315-321`, `ServerGamePacketListenerImpl.java:1413-1416`):
/// `Success` with its `swingSource`, and the pass every other record collapses
/// into for a body that transforms no held item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ItemUseResult {
    /// `InteractionResult.SUCCESS` (`InteractionResult.java:11`): a success
    /// whose swing the client already played.
    Success,
    /// `InteractionResult.SUCCESS_SERVER` (`InteractionResult.java:12`): the
    /// server swings (`ServerGamePacketListenerImpl.java:1413-1416`) because
    /// the client did not predict it.
    SuccessServer,
    /// `InteractionResult.CONSUME` (`InteractionResult.java:13`): a success
    /// nobody swings for.
    Consume,
    /// `InteractionResult.PASS` (`InteractionResult.java:15`).
    Pass,
    /// `InteractionResult.FAIL` (`InteractionResult.java:14`): the use was
    /// refused by a game-mode gate rather than falling through to the generic
    /// `Item.use`.
    Fail,
}

impl ItemUseResult {
    fn is_success(self) -> bool {
        matches!(self, Self::Success | Self::SuccessServer | Self::Consume)
    }
}

item_use_dispatch! {
    snowball: VanillaItemUseOverride::SnowballItem,
    egg: VanillaItemUseOverride::EggItem,
    empty_map: VanillaItemUseOverride::EmptyMapItem,
    ender_pearl: VanillaItemUseOverride::EnderpearlItem,
    experience_bottle: VanillaItemUseOverride::ExperienceBottleItem,
    wind_charge: VanillaItemUseOverride::WindChargeItem,
    spawn_egg: VanillaItemUseOverride::SpawnEggItem,
    spyglass: VanillaItemUseOverride::SpyglassItem,
    instrument: VanillaItemUseOverride::InstrumentItem,
    bucket: VanillaItemUseOverride::BucketItem,
    bottle: VanillaItemUseOverride::BottleItem,
    place_on_water: VanillaItemUseOverride::PlaceOnWaterBlockItem,
    boat: VanillaItemUseOverride::BoatItem,
    ender_eye: VanillaItemUseOverride::EnderEyeItem,
    fishing_rod: VanillaItemUseOverride::FishingRodItem,
    firework_rocket: VanillaItemUseOverride::FireworkRocketItem,
    splash_potion: VanillaItemUseOverride::SplashPotionItem,
    lingering_potion: VanillaItemUseOverride::LingeringPotionItem,
    written_book: VanillaItemUseOverride::WrittenBookItem,
    writable_book: VanillaItemUseOverride::WritableBookItem,
    knowledge_book: VanillaItemUseOverride::KnowledgeBookItem,
    bundle: VanillaItemUseOverride::BundleItem,
    bow: VanillaItemUseOverride::BowItem,
    crossbow: VanillaItemUseOverride::CrossbowItem,
    trident: VanillaItemUseOverride::TridentItem,
    food_on_a_stick: VanillaItemUseOverride::FoodOnAStickItem,
}

/// The three numbers `Projectile.spawnProjectileFromRotation`
/// (`Projectile.java:175-190`) is handed by an `Item.use` that throws.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ThrowShape {
    y_offset: f32,
    pow: f32,
    uncertainty: f32,
}

/// The sound an `Item.use` that throws plays first
/// (`SnowballItem.java:26-35`): `playSound(null, x, y, z, event, source,
/// 0.5F, 0.4F / (random.nextFloat() * 0.4F + 0.8F))` at the player's feet.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ThrowSound {
    resource_id: &'static str,
    source: SoundSource,
}

/// The spawner of the projectile an `Item.use` throws: the
/// `ThrowableItemProjectile(type, owner, level, stack)` constructor leg
/// (`ThrowableItemProjectile.java:21-25`), handed the store, the position that
/// constructor computes, the shot `shootFromRotation` installs, the stack it
/// copies down to one, and the owner.
type ThrowSpawner =
    fn(&mut EntityStore, Vec3d, ProjectileShot, RuntimeItemStack, Option<Uuid>) -> Option<i32>;

impl ServerWorld {
    /// `AdventureModePredicate.test(new BlockInWorld(level, pos, false))` for the
    /// stack's `CAN_BREAK` (`can_break`) or `CAN_PLACE_ON`.
    pub(super) fn adventure_mode_allows(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
        item: &ItemStackSummary,
        can_break: bool,
    ) -> bool {
        let chunks = &self.level_by_id(dimension).chunks;
        crate::inventory::adventure_mode_predicate_matches_with_registry(
            item,
            can_break,
            chunks.block_state_at(pos),
            chunks.content_registry(),
            |tag, block| match chunks.registry_tags() {
                Some(tags) => tags.contains_block(tag, block),
                None => declared_block_type_ids_in_tag(tag).is_some_and(|ids| ids.contains(&block)),
            },
            || {
                chunks
                    .block_entity_command_data(pos, Some(&self.item_stack_resolver()))
                    .ok()
                    .flatten()
            },
        )
    }

    /// `Level.mayInteract` (`ServerLevel.java:857-859`) and `Player.mayUseItemAt`
    /// (`Player.java:1593-1600`) at the clicked block.
    pub(super) fn may_use_item_at(&self, context: &ItemUseContext, pos: BlockPos) -> bool {
        self.player_may_interact_block(context.connection_id, pos)
            && self.player_may_place_block(context.connection_id, &context.item, pos)
    }

    /// `ServerPlayerGameMode.useItem` (`ServerPlayerGameMode.java:304-347`).
    ///
    /// `Err` is a component value this server cannot read, which declines the
    /// use before anything changed.
    pub(super) fn use_item_in_hand(
        &mut self,
        context: ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> Result<ItemUseResult, String> {
        let (connection_id, hand) = (context.connection_id, context.hand);
        let result = self.use_item_in_hand_checked(context, events);
        if result == Ok(ItemUseResult::SuccessServer) {
            self.player_swing(connection_id, hand, true, events);
        }
        result
    }

    fn use_item_in_hand_checked(
        &mut self,
        context: ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> Result<ItemUseResult, String> {
        let player = self
            .levels
            .player(context.connection_id)
            .expect("connection presence was checked");
        if player.game_type == GameType::Spectator {
            return Ok(ItemUseResult::Pass);
        }
        if player
            .cold
            .cooldowns
            .is_on_cooldown(&context.item)
            .unwrap_or(false)
        {
            return Ok(ItemUseResult::Pass);
        }
        let row = item_use_row(&context.item);
        if row.is_none()
            && context
                .item
                .item_id
                .and_then(vanilla_item_type)
                .is_some_and(|entry| entry.use_override.is_some())
        {
            return Ok(ItemUseResult::Pass);
        }
        let old_count = context.item.count;
        let old_damage = damage_value(&context.item);
        let before = runtime_item_in_hand(player, context.hand).clone();
        // `ItemStack.use` (`ItemStack.java:378-389`).
        let instantly_used = self.use_duration(&context.item)? <= 0;
        let (result, transformed) = match row {
            Some(row) => {
                let outcome = (row.use_item)(self, &context, events);
                (outcome.result, outcome.transformed)
            }
            None => self.use_item_by_components(&context, events)?,
        };
        let transformed = if instantly_used && result.is_success() {
            let used = match &transformed {
                Some(HeldItem::Replaced(stack)) => stack.clone(),
                Some(HeldItem::Same) | None => {
                    runtime_item_in_hand(self.levels.player_at(context.connection_id), context.hand)
                        .clone()
                }
            };
            match self.apply_after_use_component_side_effects(
                context.connection_id,
                &used,
                &before,
                events,
            )? {
                HeldItem::Same => Some(transformed.unwrap_or(HeldItem::Same)),
                replaced => Some(replaced),
            }
        } else {
            transformed
        };
        let (result_stack, same_stack) = match transformed.filter(|_| result.is_success()) {
            Some(HeldItem::Replaced(stack)) => (stack, false),
            Some(HeldItem::Same) | None => {
                // Approximates `itemStack == itemStack2` by value.
                let result = runtime_item_in_hand(
                    self.levels.player_at(context.connection_id),
                    context.hand,
                )
                .clone();
                let same = result == before;
                (result, same)
            }
        };
        let projected_result = result_stack.summary();
        let result_duration = self.use_duration(projected_result)?;
        if same_stack
            && result_stack.count() == old_count
            && result_duration <= 0
            && damage_value(projected_result) == old_damage
        {
            return Ok(result);
        }
        let using = self.levels[context.connection_id]
            .using_item
            .is_using_item();
        if result == ItemUseResult::Fail && result_duration > 0 && !using {
            return Ok(result);
        }
        if result_stack.is_empty() {
            self.set_player_item_in_hand(
                context.connection_id,
                context.hand,
                bbb_server_types::item_stack::RuntimeItemStack::empty(),
                events,
            );
        } else if !same_stack {
            self.set_player_item_in_hand(context.connection_id, context.hand, result_stack, events);
        }
        if !using {
            let player = self
                .levels
                .player_mut(context.connection_id)
                .expect("connection presence was checked");
            let menu = player
                .inventory
                .send_all_data_to_remote()
                .map_err(|error| error.to_string())?;
            Self::send_menu_sync(context.connection_id, [menu], events);
        }
        Ok(result)
    }

    /// `UseCooldown.apply` (`UseCooldown.java:35-39`), which
    /// `ServerItemCooldowns.onCooldownStarted` (`ServerItemCooldowns.java:15-18`)
    /// tells the client.
    pub(super) fn apply_use_cooldown(
        &mut self,
        connection_id: ConnectionId,
        stack_before_use: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        let Ok(Some(cooldown)) = static_item_component(
            stack_before_use,
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        ) else {
            return;
        };
        let ticks = cooldown
            .use_cooldown_ticks()
            .expect("validated use cooldown duration");
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let packet = match player.cold.cooldowns.add_cooldown(stack_before_use, ticks) {
            Ok(packet) => packet,
            Err(_) => return,
        };
        events.push(super::send(
            connection_id,
            ClientboundPlayPacket::Cooldown(packet),
        ));
    }

    /// `SnowballItem.use` / `EggItem.use` / `EnderpearlItem.use` /
    /// `ExperienceBottleItem.use` (`SnowballItem.java:24-43` and its three
    /// twins): the throw sound, `Projectile.spawnProjectileFromRotation`
    /// (`Projectile.java:175-190`), the `ITEM_USED` stat and
    /// `itemStack.consume(1, player)`, answering `SUCCESS`.
    fn throw_item_projectile(
        &mut self,
        context: &ItemUseContext,
        sound: ThrowSound,
        shape: ThrowShape,
        spawn: ThrowSpawner,
        events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        let player = self
            .levels
            .player(context.connection_id)
            .expect("connection presence was checked");
        let dimension = context.dimension;
        let position = player.position();
        let rotation = player.rotation();
        let owner = player.profile.uuid;
        let projectile_eye = player.eye_position().ok();
        let source = ShotSource {
            known_movement: player.known_movement,
            on_ground: player.on_ground(),
        };
        let has_infinite_materials = player.abilities.instabuild;
        // `:26-35`: the pitch draws from `level.getRandom()` before
        // `playSeededSound` draws the sound seed.
        let pitch = 0.4 / (self.random.next_float() * 0.4 + 0.8);
        if let Some(registry_id) = vanilla_sound_event_registry_id(sound.resource_id) {
            let sound = SoundEvent {
                sound: SoundEventHolder::Reference { registry_id },
                source: sound.source,
                position,
                volume: 0.5,
                pitch,
                seed: self.random.next_long(),
            };
            self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
        }
        // `:36-38` on the server: `ThrowableItemProjectile(type, owner, level,
        // stack)` (`ThrowableItemProjectile.java:21-25`) stands the projectile
        // at `(owner.getX(), owner.getEyeY() - 0.1F, owner.getZ())` holding
        // `stack.copyWithCount(1)`, then `shootFromRotation` with the owner's
        // angles.
        let projectile_position = projectile_eye.map(|eye| Vec3d {
            x: position.x,
            y: eye.y - f64::from(0.1_f32),
            z: position.z,
        });
        // `stack.copyWithCount(1)`.
        let item = RuntimeItemStack::from_summary(ItemStackSummary {
            count: 1,
            ..context.item.clone()
        });
        if let Some(projectile_position) = projectile_position {
            let store = &mut self.level_mut_by_id(&dimension).entities;
            shoot_from_rotation(
                store,
                source,
                ShotParameters {
                    x_rot: rotation.x_rot,
                    y_rot: rotation.y_rot,
                    y_offset: shape.y_offset,
                    pow: shape.pow,
                    uncertainty: shape.uncertainty,
                },
                |store, shot| {
                    let input = item.clone();
                    let id = spawn(store, projectile_position, shot, item, Some(owner))?;
                    crate::entity::projectile_weapon::queue_spawn_effects(store, id, input);
                    Some(id)
                },
            );
        }
        // `:40-41`.
        let player = self
            .levels
            .player_mut(context.connection_id)
            .expect("connection presence was checked");
        if let Some(item_id) = context.item.item_id {
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        player
            .inventory
            .consume_in_hand(context.hand, 1, has_infinite_materials);
        ItemUseResult::Success
    }
}

/// `new ItemStack(item)`.
fn single_item(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        ..ItemStackSummary::empty()
    }
}

/// `ItemStack.getDamageValue` (`ItemStack.java:425-427`).
fn damage_value(stack: &ItemStackSummary) -> i32 {
    crate::item_durability::damage_and_max(stack).map_or(0, |(damage, _)| damage)
}
