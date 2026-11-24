use crate::connection::ClientboundPlayPacket;
use crate::connection::ConnectionId;
use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::packets::{RecipeBookAdd, RecipeBookAddEntry, SystemChat};
use bbb_protocol::{
    component_literal_nbt, component_node_style_from_nbt_value, component_styled_literal_nbt,
    component_translatable_nbt, component_with_siblings_nbt, ComponentHoverEvent, ComponentStyle,
    JavaString, NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
};
use bbb_server_datapack::player_advancements::{AdvancementDisplay, AdvancementFrame};

use crate::commands::{
    CommandOrigin, CommandPermission, CommandSource, ExecutionSource, ResultCallback,
};

use super::{send, ServerWorld, WorldEvent};

/// `GameRules.SHOW_ADVANCEMENT_MESSAGES` (`GameRules.java:78`), which 26.1
/// renamed from `announceAdvancements`.
const RULE_SHOW_ADVANCEMENT_MESSAGES: &str = "minecraft:show_advancement_messages";

impl ServerWorld {
    // PlayerAdvancements.java:167-190 rewards only an incomplete-to-complete transition.
    pub(super) fn award_advancement_criterion(
        &mut self,
        connection_id: ConnectionId,
        index: usize,
        criterion: &JavaString,
        epoch: i64,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return false;
        };
        let outcome =
            player
                .cold
                .advancements
                .award(&self.cold.advancement_tree, index, criterion, epoch);
        if outcome.completed {
            self.grant_advancement_rewards(connection_id, index, events);
            self.announce_advancement(connection_id, index, events);
        }
        outcome.granted
    }

    /// The announcement the same completion arm broadcasts (`:177-181`), after
    /// the rewards and after every reward the rewards themselves complete.
    fn announce_advancement(
        &mut self,
        connection_id: ConnectionId,
        index: usize,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(display) = self.cold.advancement_tree.node(index).display() else {
            return;
        };
        if !display.announce_to_chat || !self.game_rule_bool(RULE_SHOW_ADVANCEMENT_MESSAGES, true) {
            return;
        }
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let message = match announcement(display, self.display_name_of(&player.profile)) {
            Ok(message) => message,
            Err(error) => {
                let id = self.cold.advancement_tree.node(index).id().to_string();
                tracing::warn!("{id}: announcement: {error}");
                return;
            }
        };
        // `PlayerList.broadcastSystemMessage` (`PlayerList.java:711-724`):
        // everyone, the earner included.
        self.broadcast(
            ClientboundPlayPacket::SystemChat(SystemChat::from_component(message, false)),
            events,
        );
    }

    // AdvancementRewards.java:39-90 orders XP, loot, recipes, then the suppressed player function.
    fn grant_advancement_rewards(
        &mut self,
        connection_id: ConnectionId,
        index: usize,
        events: &mut Vec<WorldEvent>,
    ) {
        let id = self.cold.advancement_tree.node(index).id().to_string();
        let Some(rewards) = self
            .cold
            .config
            .datapacks
            .as_ref()
            .and_then(|startup| startup.advancements.advancement(&id))
            .map(|element| element.advancement.rewards.clone())
        else {
            return;
        };
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        if rewards.experience != 0 {
            player.experience.give_experience_points(rewards.experience);
            player.last_sent_exp = -1;
        }
        if let Err(error) = self.grant_advancement_loot(connection_id, &rewards.loot, events) {
            tracing::warn!("{id}: {error}");
            return;
        }
        self.award_recipes(connection_id, &rewards.recipes, events);
        if let Some(function) = rewards.function {
            let Some(player) = self.levels.player(connection_id) else {
                return;
            };
            let sender = ExecutionSource {
                source: CommandSource::player(
                    &player.profile.name,
                    player.profile.uuid,
                    CommandPermission::GameMaster,
                ),
                origin: CommandOrigin {
                    position: [
                        player.position().x,
                        player.position().y,
                        player.position().z,
                    ],
                    rotation: [player.rotation().y_rot, player.rotation().x_rot],
                    level: self.level_key(player.dimension).clone(),
                    anchor: EntityAnchor::Feet,
                    eye_height: player.eye_height().ok(),
                },
                silent: true,
                trace: None,
                executing: None,
                callback: ResultCallback::Empty,
                chat_signer: None,
            };
            self.execute_effect_function(&function, sender, events);
        }
    }

    /// `ServerPlayer.awardRecipes` (`ServerPlayer.java:1509-1511`): learn the
    /// recipes, tell the client and fire `recipe_unlocked`; returns how many
    /// displays the client was sent.
    pub(super) fn award_recipes(
        &mut self,
        connection_id: ConnectionId,
        recipes: &[String],
        events: &mut Vec<WorldEvent>,
    ) -> usize {
        let entries = self.award_player_recipes(connection_id, recipes, events);
        let count = entries.len();
        if count > 0 {
            events.push(send(
                connection_id,
                ClientboundPlayPacket::RecipeBookAdd(RecipeBookAdd {
                    entries,
                    replace: false,
                }),
            ));
        }
        count
    }

    // ServerRecipeBook.java:61-79 publishes each known recipe before its reentrant trigger.
    pub(super) fn award_player_recipes(
        &mut self,
        connection_id: ConnectionId,
        recipes: &[String],
        events: &mut Vec<WorldEvent>,
    ) -> Vec<RecipeBookAddEntry> {
        let Some(startup) = self.cold.config.datapacks.clone() else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        for id in recipes {
            let Some(element) = startup.recipes.recipe(id) else {
                continue;
            };
            if element.recipe.is_special() {
                continue;
            }
            let Some(player) = self.levels.player_mut(connection_id) else {
                break;
            };
            if !player.inventory.recipe_book_mut().award(id.clone()) {
                continue;
            }
            entries.extend(
                startup
                    .recipe_displays
                    .displays_for_recipe(id)
                    .iter()
                    .cloned()
                    .map(|display| {
                        RecipeBookAddEntry::new(display, element.recipe.show_notification(), true)
                    }),
            );
            self.trigger_recipe_unlocked(connection_id, id, events);
        }
        entries
    }
}

/// `AdvancementType.createAnnouncement` (`AdvancementType.java:39-41`) over
/// `Advancement.name` (`Advancement.java:79-81`), which for an announced
/// advancement is always the decorated display name rather than the id literal.
fn announcement(
    display: &AdvancementDisplay,
    player: NbtSummaryValue,
) -> Result<RichTextComponent, String> {
    let key = format!("chat.type.advancement.{}", display.frame.serialized_name());
    let value = component_translatable_nbt(
        &key,
        vec![player, decorate_name(display)?],
        &ComponentStyle::default(),
    )?;
    RichTextComponent::try_from_component_nbt(value)
}

/// `Advancement.decorateName` (`Advancement.java:71-77`).
pub(crate) fn decorate_name(display: &AdvancementDisplay) -> Result<NbtSummaryValue, String> {
    let title = display.title.component().clone();
    let color = ComponentStyle::with_chat_formatting_color(chat_color(display.frame))
        .expect("both advancement chat colours resolve");
    // `ComponentUtils.mergeStyles` (`ComponentUtils.java:37-48`): the title's own
    // keys win and the colour fills in the rest, which for an empty style is the
    // colour alone.
    let tooltip = with_root_style(
        title.clone(),
        &component_node_style_from_nbt_value(&title).apply_to(&color),
    )?;
    let tooltip = component_with_siblings_nbt(
        tooltip,
        vec![
            component_literal_nbt("\n"),
            display.description.component().clone(),
        ],
    )?;
    let mut hovered = component_node_style_from_nbt_value(&title);
    hovered.hover_event = Some(ComponentHoverEvent::ShowText {
        value: Vec::new(),
        component: Some(tooltip),
    });
    // `ComponentUtils.wrapInSquareBrackets` (`:140-142`).
    component_translatable_nbt(
        "chat.square_brackets",
        vec![with_root_style(title, &hovered)?],
        &color,
    )
}

/// `AdvancementType`'s chat colour (`AdvancementType.java:11-13`).
fn chat_color(frame: AdvancementFrame) -> ChatFormatting {
    match frame {
        AdvancementFrame::Task | AdvancementFrame::Goal => ChatFormatting::Green,
        AdvancementFrame::Challenge => ChatFormatting::DarkPurple,
    }
}

/// `MutableComponent.setStyle`: the contents keep their fields and the ten
/// `Style.Serializer` keys are replaced by the new style's.
fn with_root_style(
    component: NbtSummaryValue,
    style: &ComponentStyle,
) -> Result<NbtSummaryValue, String> {
    const STYLE_KEYS: [&str; 10] = [
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
        "color",
        "click_event",
        "hover_event",
        "insertion",
        "font",
    ];
    let mut entries = match component {
        NbtSummaryValue::String(text) => vec![NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String(text),
        }],
        NbtSummaryValue::Compound(entries) => entries,
        _ => return Err("only a string or compound component carries a style".to_string()),
    };
    entries.retain(|entry| !STYLE_KEYS.iter().any(|name| entry.name == *name));
    entries.extend(style_entries(style)?);
    Ok(NbtSummaryValue::Compound(entries))
}

/// One style's `Style.Serializer` keys, taken from the only encoder of them:
/// a styled empty literal writes exactly those keys beside its `text`.
fn style_entries(style: &ComponentStyle) -> Result<Vec<NbtSummaryEntry>, String> {
    let NbtSummaryValue::Compound(entries) = component_styled_literal_nbt("", style)? else {
        return Err("a styled literal is a compound".to_string());
    };
    Ok(entries
        .into_iter()
        .filter(|entry| entry.name != "text")
        .collect())
}

#[cfg(test)]
mod tests;
