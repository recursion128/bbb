//! `AbstractCauldronBlock.useItemOn` (`AbstractCauldronBlock.java:56-67`)
//! through `CauldronInteractions` bucket and water-bottle fill/empty.

use crate::world::LevelId;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
};
use bbb_protocol::item_types::{
    vanilla_cauldron_item_interaction, vanilla_cauldron_tag_interactions, vanilla_item_type,
    VanillaCauldronInteraction, VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
    VANILLA_ITEM_LAVA_BUCKET_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
    VANILLA_ITEM_SHULKER_BOX_ID, VANILLA_ITEM_WATER_BUCKET_ID,
};
use bbb_protocol::packets::{DataComponentPatchSummary, Direction, ItemStackSummary};
#[cfg(test)]
use bbb_protocol::potion_types::vanilla_potion_for_resource_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::BlockUseContext;
use crate::block_behavior::cauldron as cauldron_block;
use crate::block_behavior::BlockUseResult;
use crate::connection::ConnectionId;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use bbb_server_types::stats::{custom_stat, StatKey};

pub(super) use cauldron_block::is_cauldron;

/// `SoundEvents.BUCKET_EMPTY` (`SoundEvents.java:247`).
const BUCKET_EMPTY: &str = "minecraft:item.bucket.empty";
/// `SoundEvents.BUCKET_EMPTY_LAVA` (`SoundEvents.java:250`).
const BUCKET_EMPTY_LAVA: &str = "minecraft:item.bucket.empty_lava";
/// `SoundEvents.BUCKET_EMPTY_POWDER_SNOW` (`SoundEvents.java:251`).
const BUCKET_EMPTY_POWDER_SNOW: &str = "minecraft:item.bucket.empty_powder_snow";
/// `SoundEvents.BUCKET_FILL` (`SoundEvents.java:253`).
const BUCKET_FILL: &str = "minecraft:item.bucket.fill";
/// `SoundEvents.BUCKET_FILL_LAVA` (`SoundEvents.java:256`).
const BUCKET_FILL_LAVA: &str = "minecraft:item.bucket.fill_lava";
/// `SoundEvents.BUCKET_FILL_POWDER_SNOW` (`SoundEvents.java:257`).
const BUCKET_FILL_POWDER_SNOW: &str = "minecraft:item.bucket.fill_powder_snow";
/// `SoundEvents.BOTTLE_EMPTY` (`SoundEvents.java:219`).
const BOTTLE_EMPTY: &str = "minecraft:item.bottle.empty";
/// `SoundEvents.BOTTLE_FILL` (`SoundEvents.java:220`).
const BOTTLE_FILL: &str = "minecraft:item.bottle.fill";

/// The `item_on:` column: [`ServerWorld::use_cauldron_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_cauldron_item_on(context, events)
}

fn stack(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

/// `PotionContents.createItemStack(Items.POTION, Potions.WATER)`
/// (`PotionContents.java:73-77`).
const WATER_POTION_ID: i32 = bbb_protocol::potion_types::VANILLA_POTIONS[0].id;

pub(in crate::world) fn water_potion_stack() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_POTION_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
            potion_contents: bbb_protocol::packets::PotionContentsSummary {
                potion_id: Some(WATER_POTION_ID),
                ..Default::default()
            },
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

/// `itemInHand.get(POTION_CONTENTS) != null && potion.is(Potions.WATER)`
/// (`CauldronInteractions.java:50-51`, `PotionContents.java:79-81`).
pub(in crate::world) fn is_water_potion(stack: &ItemStackSummary) -> bool {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
    {
        return false;
    }
    let Some(potion_id) = stack.component_patch.potion_contents.potion_id else {
        return false;
    };
    potion_id == WATER_POTION_ID
        && stack
            .component_patch
            .potion_contents
            .custom_effects
            .is_empty()
}

/// `CauldronInteractions.isUnderWater` (`CauldronInteractions.java:314-317`).
fn is_under_water(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    pos: bbb_protocol::packets::BlockPos,
) -> bool {
    let dimension = dimension.into();
    world
        .block_state(dimension, relative(pos, Direction::Up))
        .is_some_and(|state| block_state_fluid_state(state).family() == Some(FluidFamily::Water))
}

impl ServerWorld {
    /// `AbstractCauldronBlock.useItemOn` (`AbstractCauldronBlock.java:56-67`).
    pub(super) fn use_cauldron_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let Some(kind) = cauldron_block::cauldron_kind(context.block_state_id) else {
            return BlockUseResult::TryWithEmptyHand;
        };
        let Some(item_id) = context.selected_item.item_id() else {
            return BlockUseResult::TryWithEmptyHand;
        };
        // `CauldronInteraction.Dispatcher.get`: the item's own entry, then its tags.
        let Some(interaction) = vanilla_cauldron_item_interaction(kind, item_id).or_else(|| {
            let item = vanilla_item_type(item_id)?.resource_id;
            let tags = self
                .level_by_id(&context.dimension)
                .chunks
                .registry_tags()?;
            vanilla_cauldron_tag_interactions(kind)
                .find(|entry| tags.contains("minecraft:item", entry.item_tag_resource_id, item))
                .map(|entry| entry.interaction)
        }) else {
            return BlockUseResult::TryWithEmptyHand;
        };
        match interaction {
            // `CauldronInteractions.fillWaterInteraction` (`:229-235`).
            VanillaCauldronInteraction::PourWaterBucket => self.empty_cauldron_bucket(
                context,
                events,
                cauldron_block::filled_water_cauldron_state(),
                BUCKET_EMPTY,
            ),
            // `CauldronInteractions.fillLavaInteraction` (`:237-243`).
            VanillaCauldronInteraction::PourLavaBucket => {
                if is_under_water(self, context.dimension, context.update.hit.pos) {
                    return BlockUseResult::Consumed;
                }
                self.empty_cauldron_bucket(
                    context,
                    events,
                    cauldron_block::lava_cauldron_state(),
                    BUCKET_EMPTY_LAVA,
                )
            }
            // `CauldronInteractions.fillPowderSnowInteraction` (`:245-259`).
            VanillaCauldronInteraction::PourPowderSnowBucket => {
                if is_under_water(self, context.dimension, context.update.hit.pos) {
                    return BlockUseResult::Consumed;
                }
                self.empty_cauldron_bucket(
                    context,
                    events,
                    cauldron_block::filled_powder_snow_cauldron_state(),
                    BUCKET_EMPTY_POWDER_SNOW,
                )
            }
            // WATER + BUCKET (`:68-81`): fill only at `LEVEL == 3`.
            VanillaCauldronInteraction::TakeWaterBucket => {
                if cauldron_block::cauldron_level(context.block_state_id) != Some(3) {
                    return BlockUseResult::TryWithEmptyHand;
                }
                self.fill_cauldron_bucket(
                    context,
                    events,
                    VANILLA_ITEM_WATER_BUCKET_ID,
                    BUCKET_FILL,
                )
            }
            // LAVA + BUCKET (`:149-154`).
            VanillaCauldronInteraction::TakeLavaBucket => self.fill_cauldron_bucket(
                context,
                events,
                VANILLA_ITEM_LAVA_BUCKET_ID,
                BUCKET_FILL_LAVA,
            ),
            // POWDER_SNOW + BUCKET (`:156-169`): fill only at `LEVEL == 3`.
            VanillaCauldronInteraction::TakePowderSnowBucket => {
                if cauldron_block::cauldron_level(context.block_state_id) != Some(3) {
                    return BlockUseResult::TryWithEmptyHand;
                }
                self.fill_cauldron_bucket(
                    context,
                    events,
                    VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
                    BUCKET_FILL_POWDER_SNOW,
                )
            }
            // EMPTY + POTION (`:49-66`).
            VanillaCauldronInteraction::PourWaterPotionIntoEmpty => self.empty_water_bottle(
                context,
                events,
                cauldron_block::default_water_cauldron_state(),
            ),
            // WATER + GLASS_BOTTLE (`:82-94`).
            VanillaCauldronInteraction::TakeWaterBottle => self.fill_water_bottle(context, events),
            // WATER + POTION (`:95-115`): refuse a full cauldron before the potion check.
            VanillaCauldronInteraction::PourWaterPotionIntoWater => {
                if cauldron_block::cauldron_level(context.block_state_id) == Some(3) {
                    return BlockUseResult::TryWithEmptyHand;
                }
                let Some(new_state) = cauldron_block::cycled_fill_level(context.block_state_id)
                else {
                    return BlockUseResult::TryWithEmptyHand;
                };
                self.empty_water_bottle(context, events, new_state)
            }
            VanillaCauldronInteraction::CleanDyedItem => self.clean_dyed_item(context),
            VanillaCauldronInteraction::CleanBanner => self.clean_banner(context),
            VanillaCauldronInteraction::CleanShulkerBox => self.clean_shulker_box(context),
        }
    }

    /// `CauldronInteractions.dyedItemIteration` (`CauldronInteractions.java:298-312`).
    fn clean_dyed_item(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        if !context
            .selected_item
            .has_component(VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID)
            .unwrap_or(false)
        {
            return BlockUseResult::TryWithEmptyHand;
        }
        let resolver = self.item_stack_resolver();
        let Ok(cleaned) = context.selected_item.as_ref().clone().with_component_nbt(
            VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
            None,
            &resolver,
        ) else {
            return BlockUseResult::TryWithEmptyHand;
        };
        let player = &mut self.levels[context.connection_id];
        *player.inventory.item_in_hand_mut(context.update.hand) = cleaned;
        player.award_stat_once(&mut self.scoreboard, custom_stat("minecraft:clean_armor"));
        self.lower_cauldron_fill_level(context);
        BlockUseResult::Consumed
    }

    /// `CauldronInteractions.bannerInteraction` (`CauldronInteractions.java:279-296`).
    fn clean_banner(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        let resolver = self.item_stack_resolver();
        let Ok(Some(NbtSummaryValue::List(mut layers))) = context
            .selected_item
            .component_nbt(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, &resolver)
        else {
            return BlockUseResult::TryWithEmptyHand;
        };
        if layers.pop().is_none() {
            return BlockUseResult::TryWithEmptyHand;
        }
        let Ok(cleaned) = context.selected_item.with_count(1).with_component_nbt(
            VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
            Some(&NbtSummaryValue::List(layers)),
            &resolver,
        ) else {
            return BlockUseResult::TryWithEmptyHand;
        };
        drop(resolver);
        self.clean_into_filled_result(context, cleaned, "minecraft:clean_banner")
    }

    /// `CauldronInteractions.shulkerBoxInteraction` (`CauldronInteractions.java:261-277`):
    /// `transmuteCopy` keeps the stack's component patch on the plain box.
    fn clean_shulker_box(&mut self, context: &BlockUseContext<'_>) -> BlockUseResult {
        let mut cleaned = context.selected_item.with_count(1);
        cleaned.summary_mut().item_id = Some(VANILLA_ITEM_SHULKER_BOX_ID);
        self.clean_into_filled_result(context, cleaned, "minecraft:clean_shulker_box")
    }

    /// `ItemUtils.createFilledResult(.., false)`, the stat and `lowerFillLevel`.
    fn clean_into_filled_result(
        &mut self,
        context: &BlockUseContext<'_>,
        cleaned: RuntimeItemStack,
        stat: &str,
    ) -> BlockUseResult {
        if !self.create_filled_result_with(
            context.connection_id,
            context.update.hand,
            cleaned,
            false,
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.levels[context.connection_id].award_stat_once(&mut self.scoreboard, custom_stat(stat));
        self.lower_cauldron_fill_level(context);
        BlockUseResult::Consumed
    }

    /// `CauldronInteractions.fillBucket` (`CauldronInteractions.java:179-205`).
    fn fill_cauldron_bucket(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
        filled_item_id: i32,
        sound: &str,
    ) -> BlockUseResult {
        let used_item_id = context
            .selected_item
            .item_id()
            .expect("a take interaction names the held bucket");
        if !self.create_filled_result(
            context.connection_id,
            context.update.hand,
            stack(filled_item_id),
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.award_cauldron_stats(
            context.connection_id,
            "minecraft:use_cauldron",
            used_item_id,
        );
        self.set_block(
            context.dimension,
            context.update.hit.pos,
            cauldron_block::empty_cauldron_state(),
            SET_BLOCK_UPDATE_ALL,
        );
        self.play_block_sound(context.dimension, context.update.hit.pos, sound, events);
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event(VanillaGameEvent::FLUID_PICKUP, context.update.hit.pos, None);
        BlockUseResult::Consumed
    }

    /// `CauldronInteractions.emptyBucket` (`CauldronInteractions.java:207-227`).
    fn empty_cauldron_bucket(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
        new_state: i32,
        sound: &str,
    ) -> BlockUseResult {
        let used_item_id = context
            .selected_item
            .item_id()
            .expect("a pour interaction names the held bucket");
        if !self.create_filled_result(
            context.connection_id,
            context.update.hand,
            stack(VANILLA_ITEM_BUCKET_ID),
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.award_cauldron_stats(
            context.connection_id,
            "minecraft:fill_cauldron",
            used_item_id,
        );
        self.set_block(
            context.dimension,
            context.update.hit.pos,
            new_state,
            SET_BLOCK_UPDATE_ALL,
        );
        self.play_block_sound(context.dimension, context.update.hit.pos, sound, events);
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event(VanillaGameEvent::FLUID_PLACE, context.update.hit.pos, None);
        BlockUseResult::Consumed
    }

    /// EMPTY + POTION (`CauldronInteractions.java:49-66`) and WATER + POTION (`:95-115`).
    fn empty_water_bottle(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
        new_state: i32,
    ) -> BlockUseResult {
        let Some(selected_item) = context.projected_item() else {
            return BlockUseResult::Pass;
        };
        if !is_water_potion(selected_item) {
            return BlockUseResult::TryWithEmptyHand;
        }
        let used_item_id = context
            .selected_item
            .item_id()
            .expect("a pour interaction names the held potion");
        if !self.create_filled_result(
            context.connection_id,
            context.update.hand,
            stack(VANILLA_ITEM_GLASS_BOTTLE_ID),
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.award_cauldron_stats(
            context.connection_id,
            "minecraft:use_cauldron",
            used_item_id,
        );
        self.set_block(
            context.dimension,
            context.update.hit.pos,
            new_state,
            SET_BLOCK_UPDATE_ALL,
        );
        self.play_block_sound(
            context.dimension,
            context.update.hit.pos,
            BOTTLE_EMPTY,
            events,
        );
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event(VanillaGameEvent::FLUID_PLACE, context.update.hit.pos, None);
        BlockUseResult::Consumed
    }

    /// WATER + GLASS_BOTTLE (`CauldronInteractions.java:82-94`).
    fn fill_water_bottle(
        &mut self,
        context: &BlockUseContext<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let used_item_id = context
            .selected_item
            .item_id()
            .expect("a take interaction names the held bottle");
        if !self.create_filled_result(
            context.connection_id,
            context.update.hand,
            water_potion_stack(),
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.award_cauldron_stats(
            context.connection_id,
            "minecraft:use_cauldron",
            used_item_id,
        );
        self.lower_cauldron_fill_level(context);
        self.play_block_sound(
            context.dimension,
            context.update.hit.pos,
            BOTTLE_FILL,
            events,
        );
        self.level_mut_by_id(&context.dimension)
            .event_sink
            .game_event(VanillaGameEvent::FLUID_PICKUP, context.update.hit.pos, None);
        BlockUseResult::Consumed
    }

    /// `LayeredCauldronBlock.lowerFillLevel` (`LayeredCauldronBlock.java:108-113`).
    fn lower_cauldron_fill_level(&mut self, context: &BlockUseContext<'_>) {
        self.with_level(context.dimension, |level| {
            cauldron_block::lower_fill_level(level, context.update.hit.pos, context.block_state_id);
        });
    }

    /// `ItemUtils.createFilledResult` (`ItemUtils.java:16-42`).
    pub(in crate::world) fn create_filled_result(
        &mut self,
        connection_id: ConnectionId,
        hand: bbb_protocol::packets::InteractionHand,
        filled: ItemStackSummary,
    ) -> bool {
        self.create_filled_result_with(
            connection_id,
            hand,
            RuntimeItemStack::from_summary(filled),
            true,
        )
    }

    /// `limit_creative_stack_size` false always hands a creative player the
    /// result, true only when they do not already hold one.
    pub(in crate::world) fn create_filled_result_with(
        &mut self,
        connection_id: ConnectionId,
        hand: bbb_protocol::packets::InteractionHand,
        filled: RuntimeItemStack,
        limit_creative_stack_size: bool,
    ) -> bool {
        let result = (|| {
            let mut filled = filled;
            let resolver = self.item_stack_resolver();
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            let instabuild = player.abilities.instabuild;
            let inventory = &mut player.inventory;
            if instabuild {
                if !limit_creative_stack_size
                    || inventory
                        .find_slot_matching_item(&resolver, &filled)?
                        .is_none()
                {
                    inventory.add_anywhere(&resolver, &mut filled, true)?;
                }
                filled = bbb_server_types::item_stack::RuntimeItemStack::empty();
            } else {
                inventory.consume_in_hand(hand, 1, false);
                if inventory.item_in_hand_mut(hand).is_empty() {
                    inventory.set_item_in_hand(hand, filled);
                    filled = bbb_server_types::item_stack::RuntimeItemStack::empty();
                } else {
                    inventory.add_anywhere(&resolver, &mut filled, false)?;
                }
            }
            Ok::<_, bbb_server_types::ServerWorldError>(filled)
        })();
        let leftover = match result {
            Ok(leftover) => leftover,
            Err(error) => {
                tracing::warn!("filled item output: {error}");
                return false;
            }
        };
        if !leftover.is_empty() {
            self.drop_player_stack(connection_id, &leftover, false);
        }
        true
    }

    fn award_cauldron_stats(
        &mut self,
        connection_id: ConnectionId,
        custom: &str,
        used_item_id: i32,
    ) {
        if let Some(player) = self.levels.player_mut(connection_id) {
            player.award_stat_once(&mut self.scoreboard, custom_stat(custom));
            player.award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, used_item_id),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_behavior::cauldron as cauldron_block;
    use crate::connection::ConnectionId;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::ServerWorld;
    use crate::world::{
        ClientboundPlayPacket, JoinContext, ServerWorldConfig, WorldCommand, WorldEvent,
    };
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use bbb_protocol::game_types::GameType;
    use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
    use bbb_protocol::packets::{
        BlockHitResult, BlockPos, ClientInformation, GameProfile, InteractionHand, PlayServerbound,
        UseItemOn,
    };
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn item_id(name: &str) -> i32 {
        vanilla_item_type_for_resource_id(name)
            .unwrap_or_else(|| panic!("{name} is a built-in item"))
            .id
    }

    fn default_state(name: &str) -> i32 {
        let entry = vanilla_block_type_for_resource_id(name)
            .unwrap_or_else(|| panic!("{name} is a built-in block"));
        bbb_protocol::block_state_types::vanilla_block_default_state_id(entry.id)
            .expect("default state")
    }

    fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
        vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap_or_else(|| panic!("missing built-in state {name} {properties:?}"))
    }

    fn stack_named(name: &str, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id(name)),
            count,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn join_world(survival: bool) -> (ServerWorld, ConnectionId) {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: if survival {
                GameType::Survival
            } else {
                GameType::Creative
            },
            force_game_mode: survival,
            ..ServerWorldConfig::default()
        })
        .expect("fixture world");
        world.set_test_respawn_radius(0);
        let (sender, receiver) =
            world_command_mailbox(WorldMailboxLimits::default()).expect("mailbox");
        let connection_id = ConnectionId(1);
        sender
            .try_send(WorldCommand::Join {
                connection_id,
                profile: GameProfile {
                    uuid: Uuid::from_u128(1),
                    name: "alice".to_string(),
                    properties: Vec::new(),
                },
                client_information: ClientInformation::default(),
                context: JoinContext::default(),
            })
            .expect("join");
        let joined = world.tick(receiver.drain());
        let teleport_id = joined
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                    ..
                } => Some(update.id),
                _ => None,
            })
            .expect("join sends the spawn teleport");
        sender
            .try_send(WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::AcceptTeleportation { id: teleport_id },
            })
            .expect("accept teleport");
        let _ = world.tick(receiver.drain());
        sender
            .try_send(WorldCommand::Play {
                connection_id,
                packet: PlayServerbound::PlayerLoaded,
            })
            .expect("player loaded");
        let _ = world.tick(receiver.drain());
        (world, connection_id)
    }

    fn cauldron_context(
        connection_id: ConnectionId,
        pos: BlockPos,
        block_state_id: i32,
        selected: &ItemStackSummary,
    ) -> BlockUseContext<'_> {
        BlockUseContext {
            connection_id,
            dimension: Java26BuiltinDimension::Overworld.into(),
            update: UseItemOn {
                hand: InteractionHand::MainHand,
                hit: BlockHitResult {
                    pos,
                    direction: Direction::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                sequence: 1,
            },
            block_state_id,
            selected_item: std::borrow::Cow::Owned(crate::tests::native_stack(selected.clone())),
        }
    }

    fn place(world: &mut ServerWorld, pos: BlockPos, state: i32) -> i32 {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            SET_BLOCK_UPDATE_ALL,
        ));
        state
    }

    fn hold(world: &mut ServerWorld, connection_id: ConnectionId, name: &str, count: i32) {
        hold_stack(world, connection_id, stack_named(name, count));
    }

    fn hold_stack(world: &mut ServerWorld, connection_id: ConnectionId, held: ItemStackSummary) {
        let held = bbb_server_types::item_stack::RuntimeItemStack::from_summary(held);
        *world.levels[connection_id]
            .inventory
            .item_in_hand_mut(InteractionHand::MainHand) = held;
    }

    fn potion_named(potion_resource_id: &str) -> ItemStackSummary {
        let potion_id =
            bbb_protocol::potion_types::vanilla_potion_for_resource_id(potion_resource_id)
                .unwrap_or_else(|| panic!("{potion_resource_id} is a built-in potion"))
                .id;
        ItemStackSummary {
            item_id: Some(item_id("minecraft:potion")),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                ],
                potion_contents: bbb_protocol::packets::PotionContentsSummary {
                    potion_id: Some(potion_id),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }.into(),
        }
    }

    fn held_item(world: &ServerWorld, connection_id: ConnectionId) -> (Option<i32>, i32) {
        let stack = world.levels[connection_id].inventory.selected_stack();
        (stack.item_id(), stack.count())
    }

    fn held_potion_id(world: &ServerWorld, connection_id: ConnectionId) -> Option<i32> {
        world.levels[connection_id]
            .inventory
            .selected_item()
            .expect("fixture projection")
            .component_patch
            .potion_contents
            .potion_id
    }

    fn inventory_count(world: &ServerWorld, connection_id: ConnectionId, name: &str) -> i32 {
        let want = Some(item_id(name));
        world.levels[connection_id]
            .inventory
            .inventory_slots()
            .iter()
            .filter(|slot| slot.item_id() == want)
            .map(|slot| slot.count())
            .sum()
    }

    fn saw_game_event(world: &ServerWorld, pos: BlockPos, event: VanillaGameEvent) -> bool {
        let (events, _) = world.game_events(Java26BuiltinDimension::Overworld);
        events
            .iter()
            .any(|recorded| recorded.game_event == event && recorded.pos == pos)
    }

    fn custom_stat_value(world: &ServerWorld, connection_id: ConnectionId, name: &str) -> i32 {
        world.levels[connection_id]
            .cold
            .stats
            .value(custom_stat(name))
    }

    fn item_used(world: &ServerWorld, connection_id: ConnectionId, name: &str) -> i32 {
        world.levels[connection_id]
            .cold
            .stats
            .value(StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id(name)))
    }

    fn dropped_item_ids(world: &ServerWorld) -> Vec<String> {
        let level = world.level(Java26BuiltinDimension::Overworld);
        level
            .entities
            .ids()
            .into_iter()
            .filter_map(|id| {
                let entity = level.entities.get(id)?;
                let crate::entity::EntityData::Item(data) = entity.data() else {
                    return None;
                };
                Some(data.item().item_resource_id()?.to_string())
            })
            .collect()
    }

    fn use_on(
        world: &mut ServerWorld,
        connection_id: ConnectionId,
        pos: BlockPos,
        state: i32,
        selected: &ItemStackSummary,
    ) -> BlockUseResult {
        let mut events = Vec::new();
        world.use_cauldron_item_on(
            &cauldron_context(connection_id, pos, state, selected),
            &mut events,
        )
    }

    #[test]
    fn cauldron_use_item_on_fills_from_a_water_bucket() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        hold(&mut world, connection_id, "minecraft:water_bucket", 1);
        let selected = stack_named("minecraft:water_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::filled_water_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:bucket")), 1)
        );
        assert!(saw_game_event(&world, pos, VanillaGameEvent::FLUID_PLACE));
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:fill_cauldron"),
            1
        );
        assert_eq!(
            item_used(&world, connection_id, "minecraft:water_bucket"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_takes_a_full_water_cauldron() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let full = place(
            &mut world,
            pos,
            cauldron_block::filled_water_cauldron_state(),
        );
        hold(&mut world, connection_id, "minecraft:bucket", 1);
        let selected = stack_named("minecraft:bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, full, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::empty_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:water_bucket")), 1)
        );
        assert!(saw_game_event(&world, pos, VanillaGameEvent::FLUID_PICKUP));
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:use_cauldron"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_refuses_a_partial_water_cauldron() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let partial = place(
            &mut world,
            pos,
            cauldron_block::layered_cauldron_state(
                bbb_protocol::block_types::VANILLA_BLOCK_WATER_CAULDRON_ID,
                2,
            )
            .expect("LEVEL=2"),
        );
        hold(&mut world, connection_id, "minecraft:bucket", 1);
        let selected = stack_named("minecraft:bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, partial, &selected),
            BlockUseResult::TryWithEmptyHand
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(partial)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:bucket")), 1)
        );
        assert!(!saw_game_event(&world, pos, VanillaGameEvent::FLUID_PICKUP));
    }

    #[test]
    fn cauldron_use_item_on_fills_and_takes_lava() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        hold(&mut world, connection_id, "minecraft:lava_bucket", 1);
        let selected = stack_named("minecraft:lava_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::lava_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:bucket")), 1)
        );

        hold(&mut world, connection_id, "minecraft:bucket", 1);
        let selected = stack_named("minecraft:bucket", 1);
        let lava = cauldron_block::lava_cauldron_state();
        assert_eq!(
            use_on(&mut world, connection_id, pos, lava, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::empty_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:lava_bucket")), 1)
        );
    }

    #[test]
    fn cauldron_use_item_on_fills_and_takes_powder_snow() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        hold(&mut world, connection_id, "minecraft:powder_snow_bucket", 1);
        let selected = stack_named("minecraft:powder_snow_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::filled_powder_snow_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:bucket")), 1)
        );

        hold(&mut world, connection_id, "minecraft:bucket", 1);
        let selected = stack_named("minecraft:bucket", 1);
        let full = cauldron_block::filled_powder_snow_cauldron_state();
        assert_eq!(
            use_on(&mut world, connection_id, pos, full, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::empty_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:powder_snow_bucket")), 1)
        );
    }

    #[test]
    fn cauldron_use_item_on_consumes_lava_and_powder_snow_when_under_water() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let above = BlockPos { x: 0, y: 65, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        place(&mut world, above, default_state("minecraft:water"));

        hold(&mut world, connection_id, "minecraft:lava_bucket", 1);
        let selected = stack_named("minecraft:lava_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(empty)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:lava_bucket")), 1)
        );
        assert!(!saw_game_event(&world, pos, VanillaGameEvent::FLUID_PLACE));

        hold(&mut world, connection_id, "minecraft:powder_snow_bucket", 1);
        let selected = stack_named("minecraft:powder_snow_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(empty)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:powder_snow_bucket")), 1)
        );
    }

    #[test]
    fn cauldron_use_item_on_passes_a_non_bucket() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        hold(&mut world, connection_id, "minecraft:stick", 1);
        let selected = stack_named("minecraft:stick", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::TryWithEmptyHand
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(empty)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:stick")), 1)
        );
    }

    #[test]
    fn cauldron_use_item_on_does_not_shrink_the_source_bucket_in_creative() {
        let (mut world, connection_id) = join_world(false);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        hold(&mut world, connection_id, "minecraft:water_bucket", 2);
        let selected = stack_named("minecraft:water_bucket", 2);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::filled_water_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:water_bucket")), 2)
        );
        assert_eq!(
            inventory_count(&world, connection_id, "minecraft:bucket"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_empties_a_water_potion_into_an_empty_cauldron() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        let selected = potion_named("minecraft:water");
        hold_stack(&mut world, connection_id, selected.clone());
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::default_water_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:glass_bottle")), 1)
        );
        assert!(saw_game_event(&world, pos, VanillaGameEvent::FLUID_PLACE));
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:use_cauldron"),
            1
        );
        assert_eq!(item_used(&world, connection_id, "minecraft:potion"), 1);
    }

    #[test]
    fn cauldron_use_item_on_fills_a_glass_bottle_from_full_water() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let full = place(
            &mut world,
            pos,
            cauldron_block::filled_water_cauldron_state(),
        );
        hold(&mut world, connection_id, "minecraft:glass_bottle", 1);
        let selected = stack_named("minecraft:glass_bottle", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, full, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(
                cauldron_block::layered_cauldron_state(
                    bbb_protocol::block_types::VANILLA_BLOCK_WATER_CAULDRON_ID,
                    2,
                )
                .expect("LEVEL=2")
            )
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:potion")), 1)
        );
        assert_eq!(
            held_potion_id(&world, connection_id),
            vanilla_potion_for_resource_id("minecraft:water").map(|potion| potion.id)
        );
        assert!(saw_game_event(&world, pos, VanillaGameEvent::FLUID_PICKUP));
        assert!(saw_game_event(&world, pos, VanillaGameEvent::BLOCK_CHANGE));
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:use_cauldron"),
            1
        );
        assert_eq!(
            item_used(&world, connection_id, "minecraft:glass_bottle"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_cycles_a_water_potion_into_level_one_and_refuses_level_three() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let level1 = place(
            &mut world,
            pos,
            cauldron_block::default_water_cauldron_state(),
        );
        let selected = potion_named("minecraft:water");
        hold_stack(&mut world, connection_id, selected.clone());
        assert_eq!(
            use_on(&mut world, connection_id, pos, level1, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(
                cauldron_block::layered_cauldron_state(
                    bbb_protocol::block_types::VANILLA_BLOCK_WATER_CAULDRON_ID,
                    2,
                )
                .expect("LEVEL=2")
            )
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:glass_bottle")), 1)
        );

        let full = place(
            &mut world,
            pos,
            cauldron_block::filled_water_cauldron_state(),
        );
        hold_stack(&mut world, connection_id, selected.clone());
        assert_eq!(
            use_on(&mut world, connection_id, pos, full, &selected),
            BlockUseResult::TryWithEmptyHand
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(full)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:potion")), 1)
        );
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:use_cauldron"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_refuses_a_non_water_potion() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        let selected = potion_named("minecraft:swiftness");
        hold_stack(&mut world, connection_id, selected.clone());
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::TryWithEmptyHand
        );
        let partial = place(
            &mut world,
            pos,
            cauldron_block::default_water_cauldron_state(),
        );
        assert_eq!(
            use_on(&mut world, connection_id, pos, partial, &selected),
            BlockUseResult::TryWithEmptyHand
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(partial)
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:potion")), 1)
        );
        assert!(!saw_game_event(&world, pos, VanillaGameEvent::FLUID_PLACE));
        assert_eq!(
            custom_stat_value(&world, connection_id, "minecraft:use_cauldron"),
            0
        );
    }

    #[test]
    fn cauldron_use_item_on_does_not_shrink_the_source_bottle_in_creative() {
        let (mut world, connection_id) = join_world(false);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        let selected = potion_named("minecraft:water");
        hold_stack(&mut world, connection_id, selected.clone());
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::default_water_cauldron_state())
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:potion")), 1)
        );
        assert_eq!(
            held_potion_id(&world, connection_id),
            vanilla_potion_for_resource_id("minecraft:water").map(|potion| potion.id)
        );
        assert_eq!(
            inventory_count(&world, connection_id, "minecraft:glass_bottle"),
            1
        );
    }

    #[test]
    fn cauldron_use_item_on_refuses_bottles_on_lava_and_powder_snow() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let water = potion_named("minecraft:water");
        let bottle = stack_named("minecraft:glass_bottle", 1);
        for state in [
            cauldron_block::lava_cauldron_state(),
            cauldron_block::filled_powder_snow_cauldron_state(),
        ] {
            let placed = place(&mut world, pos, state);
            for selected in [water.clone(), bottle.clone()] {
                hold_stack(&mut world, connection_id, selected.clone());
                assert_eq!(
                    use_on(&mut world, connection_id, pos, placed, &selected),
                    BlockUseResult::TryWithEmptyHand
                );
                assert_eq!(
                    world.block_state(Java26BuiltinDimension::Overworld, pos),
                    Some(placed)
                );
            }
        }
    }

    #[test]
    fn cauldron_use_item_on_drops_the_filled_bucket_when_inventory_is_full() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let empty = place(&mut world, pos, cauldron_block::empty_cauldron_state());
        {
            let stone = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack_named(
                "minecraft:stone",
                64,
            ));
            let player = world.levels.player_mut(connection_id).expect("joined");
            for index in 0..player.inventory.inventory_slots().len() {
                player.inventory.set_inventory_slot(index, stone.clone());
            }
        }
        hold(&mut world, connection_id, "minecraft:water_bucket", 2);
        let selected = stack_named("minecraft:water_bucket", 2);
        assert_eq!(
            use_on(&mut world, connection_id, pos, empty, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            held_item(&world, connection_id),
            (Some(item_id("minecraft:water_bucket")), 1)
        );
        assert_eq!(
            inventory_count(&world, connection_id, "minecraft:bucket"),
            0
        );
        assert_eq!(dropped_item_ids(&world), ["minecraft:bucket"]);
    }

    #[test]
    fn cauldron_use_item_on_replaces_any_cauldron_with_a_poured_bucket() {
        let (mut world, connection_id) = join_world(true);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        let lava = place(&mut world, pos, cauldron_block::lava_cauldron_state());
        hold(&mut world, connection_id, "minecraft:water_bucket", 1);
        let selected = stack_named("minecraft:water_bucket", 1);
        assert_eq!(
            use_on(&mut world, connection_id, pos, lava, &selected),
            BlockUseResult::Consumed
        );
        assert_eq!(
            world.block_state(Java26BuiltinDimension::Overworld, pos),
            Some(cauldron_block::filled_water_cauldron_state())
        );
    }

    #[test]
    fn cauldron_use_item_on_answers_all_four_registrations() {
        assert!(cauldron_block::is_cauldron(
            cauldron_block::empty_cauldron_state()
        ));
        assert!(cauldron_block::is_cauldron(
            cauldron_block::filled_water_cauldron_state()
        ));
        assert!(cauldron_block::is_cauldron(
            cauldron_block::lava_cauldron_state()
        ));
        assert!(cauldron_block::is_cauldron(
            cauldron_block::filled_powder_snow_cauldron_state()
        ));
        assert!(!cauldron_block::is_cauldron(default_state(
            "minecraft:stone"
        )));
        let _ = state_of("minecraft:water_cauldron", &[("level", "1")]);
    }
}
