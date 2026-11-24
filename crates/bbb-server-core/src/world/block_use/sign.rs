//! `SignBlock` (`SignBlock.java`) and the two hanging-sign subclasses.
//!

use crate::world::LevelId;
use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VanillaBlockUseItemOnOverride};
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::item_types::{vanilla_item_type, VanillaBlockItemKind};
use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::packets::{
    BlockPos, BlockUpdate, DialogHolder, Direction, ItemStackSummary, OpenSignEditor, ShowDialog,
};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::{NbtSummaryValue, RichTextComponent};
use bbb_registry_codec::{decode_exact_synchronized_registry_element, RegistryElementData};
use bbb_server_types::block_queries::block_entity_type_for_state;
use bbb_server_types::directions::opposite;

use super::BlockUseContext;
use crate::block_behavior::{direction_property, BlockUseResult};
use crate::block_entity::{sign, BlockEntity};
use crate::commands::{
    resolve_text, CommandOrigin, CommandPermission, CommandSource, ResultCallback,
};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::level_event::VanillaGameEvent;
use crate::world::server_functions::CommandContinuationReport;
use crate::world::{send, ServerWorld, WorldEvent};
use bbb_server_types::stats::StatKey;

/// Whether the clicked block is one of the four sign blocks, which is the
/// `SignBlockEntity` / `HangingSignBlockEntity` type the state carries rather
/// than a class list written here.
pub(super) fn is_sign(block_state_id: i32) -> bool {
    matches!(
        block_entity_type_for_state(block_state_id),
        Some(type_id)
            if type_id == VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
                || type_id == VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
    )
}

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_sign_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    let Some(selected_item) = context.projected_item() else {
        return BlockUseResult::Pass;
    };
    world.use_sign_item_on(context, selected_item, events)
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_sign_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_sign_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

impl ServerWorld {
    ///
    /// `InteractionResult.PASS` when there is no `SignBlockEntity` at the
    /// position (`:121-123`). The applicator arm (`:103-114`) consumes the
    /// click when a dye, ink sac, glow ink sac or honeycomb actually mutates
    /// the selected face; every other held item, and every applicator that
    /// refuses, answers `TRY_WITH_EMPTY_HAND` so the editor path still runs.
    pub(super) fn use_sign_item_on(
        &mut self,
        context: &BlockUseContext<'_>,
        selected_item: &ItemStackSummary,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let connection_id = context.connection_id;
        let dimension = context.dimension;
        let pos = context.update.hit.pos;
        let block_state_id = context.block_state_id;
        let hand = context.update.hand;
        let face = context.update.hit.direction;
        let Some(sign_entity) = self.sign_at(dimension, pos) else {
            return BlockUseResult::Pass;
        };
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        if chains_hanging_sign(block_state_id, selected_item, face) {
            let is_front_text = sign::is_facing_front_text(
                block_state_id,
                true,
                pos,
                player.position().x,
                player.position().z,
            );
            // SignBlockEntity.java:185-187 and SignText.java:127-137.
            let can_execute_click_commands = sign_entity.is_waxed()
                && sign_entity
                    .text(is_front_text)
                    .messages(player.cold.client_information.text_filtering_enabled)
                    .iter()
                    .filter_map(line_click_event)
                    .any(|click| {
                        string_field(click, "action").is_some_and(|action| action == "run_command")
                    });
            if !can_execute_click_commands {
                return BlockUseResult::Pass;
            }
        }
        let applicator = sign::sign_applicator(selected_item);
        let has_applicator_to_use = applicator.is_some() && player.may_build();
        if !has_applicator_to_use {
            return BlockUseResult::TryWithEmptyHand;
        }
        let sign = self
            .sign_at(dimension, pos)
            .expect("presence was checked above");
        if sign.is_waxed() {
            return BlockUseResult::TryWithEmptyHand;
        }
        let editor = sign.player_who_may_edit();
        if editor.is_some_and(|editor| editor != player.profile.uuid) {
            return BlockUseResult::TryWithEmptyHand;
        }
        let text_filtering = player.cold.client_information.text_filtering_enabled;
        let hanging = self.block_entity_type_is_hanging_sign(dimension, pos);
        let is_front_text = sign::is_facing_front_text(
            block_state_id,
            hanging,
            pos,
            player.position().x,
            player.position().z,
        );
        let applicator = applicator.expect("has_applicator_to_use");
        if !sign::can_apply_to_sign(
            &applicator,
            self.sign_at(dimension, pos)
                .expect("presence was checked above")
                .text(is_front_text),
            text_filtering,
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        let applied = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::sign_mut)
            .is_some_and(|sign| sign::try_apply_to_sign(sign, &applicator, is_front_text));
        if !applied {
            return BlockUseResult::TryWithEmptyHand;
        }
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        self.level_mut_by_id(&dimension)
            .changed_block_broadcasts
            .insert((pos.x, pos.y, pos.z));
        match &applicator {
            sign::SignApplicator::Honeycomb => {
                self.level_mut_by_id(&dimension).event_sink.level_event(
                    sign::LEVEL_EVENT_WAX_ON,
                    pos,
                    0,
                );
            }
            sign::SignApplicator::Dye { .. } => {
                self.play_block_sound(dimension, pos, "minecraft:item.dye.use", events);
            }
            sign::SignApplicator::InkSac => {
                self.play_block_sound(dimension, pos, "minecraft:item.ink_sac.use", events);
            }
            sign::SignApplicator::GlowInkSac => {
                self.play_block_sound(dimension, pos, "minecraft:item.glow_ink_sac.use", events);
            }
        }
        // `SignBlock.useItemOn` success (`:107-111`).
        self.execute_sign_click_commands(connection_id, dimension, pos, is_front_text, events);
        let item_id = selected_item
            .item_id
            .expect("a SignApplicator stack has an item");
        let entity_id = self.levels[connection_id].entity_id();
        let instabuild = self.levels[connection_id].abilities.instabuild;
        self.levels[connection_id].award_stat_once(
            &mut self.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::BLOCK_CHANGE,
                pos,
                Some(entity_id),
                Some(block_state_id),
            );
        self.levels[connection_id]
            .inventory
            .consume_in_hand(hand, 1, instabuild);
        BlockUseResult::Consumed
    }

    pub(super) fn use_sign_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let Some(sign) = self.sign_at(dimension, pos) else {
            return false;
        };
        let is_waxed = sign.is_waxed();
        let editor = sign.player_who_may_edit();
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let text_filtering = player.cold.client_information.text_filtering_enabled;
        let is_front_text = sign::is_facing_front_text(
            block_state_id,
            self.block_entity_type_is_hanging_sign(dimension, pos),
            pos,
            player.position().x,
            player.position().z,
        );
        let executed_click_command =
            self.execute_sign_click_commands(connection_id, dimension, pos, is_front_text, events);
        if is_waxed {
            let sound = if self.block_entity_type_is_hanging_sign(dimension, pos) {
                "minecraft:block.hanging_sign.waxed_interact_fail"
            } else {
                "minecraft:block.sign.waxed_interact_fail"
            };
            self.play_block_sound(dimension, pos, sound, events);
            self.mark_use_item_on_server_swing();
            return true;
        }
        if executed_click_command {
            self.mark_use_item_on_server_swing();
            return true;
        }
        let other_player_is_editing =
            editor.is_some_and(|editor| editor != self.levels[connection_id].profile.uuid);
        if other_player_is_editing || !self.levels[connection_id].may_build() {
            return false;
        }
        let Some(sign) = self.sign_at(dimension, pos) else {
            return false;
        };
        if !sign::has_editable_text(sign.text(is_front_text), text_filtering) {
            return false;
        }
        self.open_sign_text_edit(connection_id, dimension, pos, is_front_text, events);
        self.mark_use_item_on_server_swing();
        true
    }

    pub(super) fn open_sign_text_edit(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        is_front_text: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let uuid = self.levels[connection_id].profile.uuid;
        if let Some(sign) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::sign_mut)
        {
            sign.set_allowed_player_editor(Some(uuid));
        }
        if let Some(block_state_id) = self.block_state(dimension, pos) {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::BlockUpdate(BlockUpdate {
                    pos,
                    block_state_id,
                }),
            ));
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenSignEditor(OpenSignEditor { pos, is_front_text }),
        ));
    }

    /// `SignBlockEntity.executeClickCommandsIfPresent`
    /// (`SignBlockEntity.java:189-214`) over the face's lines as the player
    /// reads them.
    fn execute_sign_click_commands(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        pos: BlockPos,
        is_front_text: bool,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let filtering = self.levels[connection_id]
            .cold
            .client_information
            .text_filtering_enabled;
        let Some(sign) = self.sign_at(dimension, pos) else {
            return false;
        };
        let clicks = sign
            .text(is_front_text)
            .messages(filtering)
            .iter()
            .filter_map(line_click_event)
            .cloned()
            .collect::<Vec<_>>();
        let mut any = false;
        for click in clicks {
            match string_field(&click, "action") {
                Some(action) if action == "run_command" => {
                    if let Some(command) = string_field(&click, "command") {
                        self.run_sign_command(
                            connection_id,
                            dimension,
                            pos,
                            command.units(),
                            events,
                        );
                        any = true;
                    }
                }
                Some(action) if action == "show_dialog" => {
                    if let Some(dialog) =
                        field(&click, "dialog").and_then(|dialog| self.dialog_holder(dialog))
                    {
                        events.push(send(
                            connection_id,
                            ClientboundPlayPacket::ShowDialog(ShowDialog { dialog }),
                        ));
                    }
                    any = true;
                }
                // `MinecraftServer.handleCustomClickAction` only logs (`:2061-2063`).
                Some(action) if action == "custom" => any = true,
                _ => {}
            }
        }
        any
    }

    /// `performPrefixedCommand` with `createCommandSourceStack(player, ...)`
    /// (`SignBlockEntity.java:214-222`): the player's name and entity at the
    /// sign's centre with gamemaster permission, telling no one its output.
    fn run_sign_command(
        &mut self,
        connection_id: ConnectionId,
        dimension: LevelId,
        pos: BlockPos,
        command: &[u16],
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let source = CommandSource::player(
            &player.profile.name,
            player.profile.uuid,
            CommandPermission::GameMaster,
        );
        let origin = sign_origin(self, dimension, pos);
        let mut report = CommandContinuationReport::default();
        self.perform_command(
            std::sync::Arc::from(command.strip_prefix(&[u16::from(b'/')]).unwrap_or(command)),
            &source,
            &origin,
            ResultCallback::Empty,
            Some(&mut report),
            events,
        );
    }

    /// `ClickEvent.ShowDialog`'s `Dialog.CODEC` holder: a registry name or an
    /// inline dialog.
    fn dialog_holder(&self, dialog: &NbtSummaryValue) -> Option<DialogHolder> {
        if let NbtSummaryValue::String(id) = dialog {
            let registry_id = self
                .registry_ids("minecraft:dialog")
                .iter()
                .position(|candidate| id == candidate)?;
            return Some(DialogHolder::Reference {
                registry_id: i32::try_from(registry_id).ok()?,
            });
        }
        let data = RegistryElementData::from_network_nbt(dialog.clone());
        let dialog = decode_exact_synchronized_registry_element("minecraft:dialog", &data)
            .ok()?
            .projected_network_nbt()
            .ok()?;
        Some(DialogHolder::Direct { dialog })
    }

    /// `SignBlockEntity.loadLines` (`SignBlockEntity.java:109-127`) for lines
    /// the sign loaded: each resolved against a `Sign` source at its centre
    /// with gamemaster permission, a line that fails kept as it was.
    pub(in crate::world) fn resolve_loaded_sign_lines(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
    ) {
        if !self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sign)
            .is_some_and(sign::SignBlockEntity::has_lines_to_resolve)
        {
            return;
        }
        let Some(mut sign) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::sign_mut)
            .and_then(|sign| sign.take_lines_to_resolve().then(|| sign.clone()))
        else {
            return;
        };
        let source = CommandSource::block(RichTextComponent::plain("Sign"));
        let origin = sign_origin(self, dimension, pos);
        self.with_entity_reads(|reads| {
            sign.resolve_lines(|line| {
                resolve_text(line, &source, &origin, reads, None).unwrap_or_else(|_| line.clone())
            });
        });
        let level = self.level_mut_by_id(&dimension);
        if let Some(current) = level
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::sign_mut)
        {
            *current = sign;
        }
        level.chunks.block_entities_mut().set_changed(pos);
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
    }
}

/// `Vec3.atCenterOf(pos)` and `Vec2.ZERO` (`SignBlockEntity.java:218-220`).
fn sign_origin(world: &ServerWorld, dimension: LevelId, pos: BlockPos) -> CommandOrigin {
    CommandOrigin {
        position: [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ],
        rotation: [0.0, 0.0],
        level: world.level_key(dimension).clone(),
        anchor: EntityAnchor::Feet,
        eye_height: None,
    }
}

/// `shouldTryToChainAnotherHangingSign` minus the click-command test: a
/// hanging sign item clicked on the bottom of a ceiling hanging sign
/// (`CeilingHangingSignBlock.java:82-87`) or on a non-text face of a wall
/// hanging sign (`WallHangingSignBlock.java:75-85`).
fn chains_hanging_sign(block_state_id: i32, held: &ItemStackSummary, face: Direction) -> bool {
    let holds_hanging_sign = held
        .item_id
        .and_then(vanilla_item_type)
        .and_then(|item| item.block_item_behavior)
        .is_some_and(|behavior| behavior.kind == VanillaBlockItemKind::HangingSignItem);
    if !holds_hanging_sign {
        return false;
    }
    match vanilla_block_state_block_type_id(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.use_item_on_override)
    {
        Some(VanillaBlockUseItemOnOverride::CeilingHangingSignBlock) => face == Direction::Down,
        Some(VanillaBlockUseItemOnOverride::WallHangingSignBlock) => {
            direction_property(block_state_id, "facing")
                .is_some_and(|facing| face != facing && face != opposite(facing))
        }
        _ => false,
    }
}

/// `Component.getStyle().getClickEvent()`: the root's style, which for a list
/// form is its first element's.
fn line_click_event(line: &NbtSummaryValue) -> Option<&NbtSummaryValue> {
    match line {
        NbtSummaryValue::List(values) => values.first().and_then(line_click_event),
        _ => field(line, "click_event"),
    }
}

fn string_field<'a>(
    value: &'a NbtSummaryValue,
    name: &str,
) -> Option<&'a bbb_protocol::JavaString> {
    match field(value, name)? {
        NbtSummaryValue::String(text) => Some(text),
        _ => None,
    }
}
