//! Canonical world facts consumed by vanilla's generic entity name-tag pass.
//!
//! Camera distance, crosshair selection, `hideGui`, glyph layout, and draw ordering
//! remain native/renderer-owned. This module keeps the packet-backed Component,
//! scoreboard/team, metadata, attachment, and entity-renderer policy facts together.

use bbb_protocol::{
    entity_types::{
        vanilla_entity_type, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    },
    packets::{EntityDataValue, EntityDataValueKind, NumberFormatSummary},
    ComponentStyle, NbtSummaryValue, RichTextComponent, StyledTextRun,
};

use super::{
    dimensions::{item_frame_facing, item_frame_item, sniffer_is_digging},
    schema, EntityModelSourceState, EntityVec3, ItemFrameFacing,
};
use crate::client::scoreboard_team_formatting_style;
use crate::WorldStore;

/// One exact or synthetic Component fragment in a name-tag line.
///
/// `Rich` retains validated network NBT so native localization can resolve
/// translatable components. `Styled` owns literal/number-format fragments,
/// while `Translation` represents client-created EntityType descriptions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityNameTagTextSegmentState {
    Rich(RichTextComponent),
    Styled(Vec<StyledTextRun>),
    Translation { key: String },
}

/// A line after vanilla Component composition but before client-language resolution.
/// Team formatting is a parent style and therefore only fills style fields that a
/// child run/component did not set itself.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntityNameTagTextState {
    pub segments: Vec<EntityNameTagTextSegmentState>,
    pub inherited_style: ComponentStyle,
}

/// The non-camera branch of `LivingEntityRenderer.shouldShowName`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityNameTagLivingVisibilityState {
    /// No target-team early return: native must apply `renderNames`, camera-entity,
    /// visibility-to-player, and vehicle checks.
    General {
        visible_to_local_player: bool,
        is_vehicle: bool,
    },
    /// A target team selected one of vanilla's four visibility branches. Those
    /// branches return before `renderNames`, camera-entity, and vehicle checks.
    TeamOverride { visible: bool },
}

/// Renderer-specific name eligibility after the universal 64-block distance gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityNameTagVisibilityPolicy {
    /// Base `EntityRenderer`: custom-name-visible, or a hidden custom name under
    /// the crosshair.
    Entity {
        entity_should_show_name: bool,
        has_custom_name: bool,
    },
    /// `MobRenderer` and `AvatarRenderer`: living/team visibility and the same
    /// custom-name gate. Player's `shouldShowName()` is always true; mannequins
    /// retain Entity's metadata-backed value.
    MobOrAvatar {
        living: EntityNameTagLivingVisibilityState,
        entity_should_show_name: bool,
        has_custom_name: bool,
        is_discrete: bool,
    },
    /// `ArmorStandRenderer` bypasses the living/team/discrete rules entirely.
    ArmorStand { custom_name_visible: bool },
    /// `ItemFrameRenderer` requires global name rendering plus the crosshair
    /// target. The source only exists when the displayed stack has a custom
    /// name; facing drives the renderer-specific dispatcher-root offset.
    ItemFrame { facing: ItemFrameFacing },
}

/// World-owned input to one generic entity name-tag submission.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityNameTagRenderSourceState {
    pub entity_id: i32,
    /// Raw interpolated position used by the dispatcher's name-distance gate.
    pub distance_position: EntityVec3,
    /// Dispatcher-root position shared with the model source. Renderer-owned
    /// offsets (creepy Enderman jitter and Avatar crouch offset) are composed later.
    pub position: EntityVec3,
    /// Relative `EntityAttachment.NAME_TAG` Y before the renderer's fixed `+0.5`.
    pub attachment_y: f32,
    /// AvatarRenderer's Deadmau5 ears clearance (`-10` pixels), otherwise zero.
    pub line_offset: i32,
    pub is_discrete: bool,
    pub name: EntityNameTagTextState,
    pub below_name: Option<EntityNameTagTextState>,
    pub visibility: EntityNameTagVisibilityPolicy,
}

impl WorldStore {
    /// Projects generic name-tag facts from the exact model-source generation that
    /// survived renderer culling. This keeps transform/light sampling coherent and
    /// avoids a second heap-heavy ECS projection.
    pub fn fill_entity_name_tag_render_sources_for_model_sources(
        &self,
        model_sources: &[EntityModelSourceState],
        sources: &mut Vec<EntityNameTagRenderSourceState>,
    ) {
        sources.clear();
        sources.reserve(model_sources.len());
        sources.extend(
            model_sources
                .iter()
                .filter_map(|source| self.entity_name_tag_render_source(source)),
        );
    }

    /// Convenience query for diagnostics/tests. The runtime reuses its already
    /// culled model-source generation through the allocation-retaining method above.
    pub fn entity_name_tag_render_sources_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<EntityNameTagRenderSourceState> {
        let model_sources = self.entity_model_sources_at_partial_tick(partial_ticks);
        let mut sources = Vec::new();
        self.fill_entity_name_tag_render_sources_for_model_sources(&model_sources, &mut sources);
        sources
    }

    fn entity_name_tag_render_source(
        &self,
        source: &EntityModelSourceState,
    ) -> Option<EntityNameTagRenderSourceState> {
        let entity_accessors = schema::vanilla_entity_metadata_entity_accessors()?;

        // ItemFrameRenderer replaces both eligibility and text with the displayed
        // stack's custom hover name. It still submits through the generic name-tag
        // feature after applying its facing-dependent dispatcher-root offset.
        if matches!(
            source.entity_type_id,
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID | VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
        ) {
            let accessors = schema::vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                source.entity_type_id,
            )?;
            let item = item_frame_item(accessors, &source.data_values)?;
            let patch = &item.component_patch;
            let custom_name = patch.custom_name.as_deref()?;
            let identity = self.entities.identity(source.entity_id)?;
            let shared_flags = metadata_byte(
                &source.data_values,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            );
            let is_discrete = shared_flags & entity_accessors.shift_key_down_mask != 0;
            let scoreboard_name =
                self.scoreboard_name_for_identity(source.entity_type_id, source.uuid);
            return Some(EntityNameTagRenderSourceState {
                entity_id: source.entity_id,
                distance_position: source.distance_position,
                position: source.position,
                // ItemFrame's dynamic pick box may grow to 0.75/1.0, but
                // EntityAttachment.NAME_TAG uses its fixed EntityType height.
                attachment_y: vanilla_entity_type(source.entity_type_id)?.height,
                line_offset: 0,
                is_discrete,
                name: EntityNameTagTextState {
                    segments: vec![stored_component_segment(
                        patch.custom_name_nbt.as_ref(),
                        patch.custom_name_styled.as_deref().unwrap_or_default(),
                        custom_name,
                    )],
                    inherited_style: ComponentStyle::default(),
                },
                below_name: self.scoreboard_below_name(&scoreboard_name),
                visibility: EntityNameTagVisibilityPolicy::ItemFrame {
                    facing: item_frame_facing(accessors, identity.data, &source.data_values),
                },
            });
        }

        let custom_name = self
            .entities
            .metadata_optional_rich_component_override(
                source.entity_id,
                entity_accessors.custom_name_data_id,
            )
            .flatten();
        let has_custom_name = custom_name.is_some();
        let custom_name_visible = metadata_bool(
            &source.data_values,
            entity_accessors.custom_name_visible_data_id,
            entity_accessors.custom_name_visible_default,
        );
        // `NameTagFeatureRenderer.Storage.add` drops both score and name when
        // `nameTagAttachment` is null. EntityRenderer only sets that attachment
        // when the ordinary name passes its eligibility gate, so an unnamed,
        // untargetable entity cannot submit a below-name line on its own.
        if source.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            && !custom_name_visible
            && custom_name.is_none()
        {
            return None;
        }
        let shared_flags = metadata_byte(
            &source.data_values,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        );
        let is_discrete = shared_flags & entity_accessors.shift_key_down_mask != 0;
        let scoreboard_name = self.scoreboard_name_for_identity(source.entity_type_id, source.uuid);
        let base_name = if source.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            EntityNameTagTextSegmentState::Styled(vec![plain_run(
                self.player_info_entry(source.uuid).map_or_else(
                    || scoreboard_name.clone(),
                    |entry| entry.profile.name.clone(),
                ),
            )])
        } else if let Some(custom_name) = custom_name {
            EntityNameTagTextSegmentState::Rich(custom_name)
        } else {
            EntityNameTagTextSegmentState::Translation {
                key: entity_description_key(source.entity_type_id)?,
            }
        };
        let name = self.team_formatted_name(&scoreboard_name, base_name);
        let below_name = if source.entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID {
            self.mannequin_below_name(source)
        } else {
            self.scoreboard_below_name(&scoreboard_name)
        };

        let entity_should_show_name =
            source.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID || custom_name_visible;
        let visibility = if source.entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            EntityNameTagVisibilityPolicy::ArmorStand {
                custom_name_visible,
            }
        } else if source.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            || source.entity_type_id == VANILLA_ENTITY_TYPE_MANNEQUIN_ID
            || (schema::vanilla_entity_metadata_type_is_mob(source.entity_type_id)
                && source.entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID)
        {
            EntityNameTagVisibilityPolicy::MobOrAvatar {
                living: self.living_name_visibility(source, &scoreboard_name),
                entity_should_show_name,
                has_custom_name,
                is_discrete,
            }
        } else {
            EntityNameTagVisibilityPolicy::Entity {
                entity_should_show_name,
                has_custom_name,
            }
        };

        Some(EntityNameTagRenderSourceState {
            entity_id: source.entity_id,
            distance_position: source.distance_position,
            position: source.position,
            attachment_y: name_tag_attachment_y(source),
            line_offset: if source.show_extra_ears { -10 } else { 0 },
            is_discrete,
            name,
            below_name,
            visibility,
        })
    }

    fn team_formatted_name(
        &self,
        scoreboard_name: &str,
        base_name: EntityNameTagTextSegmentState,
    ) -> EntityNameTagTextState {
        let Some(parameters) = self
            .scoreboard
            .team_for_scoreboard_name(scoreboard_name)
            .and_then(|team| team.parameters.as_ref())
        else {
            return EntityNameTagTextState {
                segments: vec![base_name],
                inherited_style: ComponentStyle::default(),
            };
        };

        EntityNameTagTextState {
            segments: vec![
                stored_component_segment(
                    parameters.player_prefix_component.as_ref(),
                    &parameters.player_prefix_styled,
                    &parameters.player_prefix,
                ),
                base_name,
                stored_component_segment(
                    parameters.player_suffix_component.as_ref(),
                    &parameters.player_suffix_styled,
                    &parameters.player_suffix,
                ),
            ],
            inherited_style: scoreboard_team_formatting_style(&parameters.color),
        }
    }

    fn living_name_visibility(
        &self,
        source: &EntityModelSourceState,
        target_scoreboard_name: &str,
    ) -> EntityNameTagLivingVisibilityState {
        let visible_to_local_player = !source.invisible_to_player;
        let local_player_id = self.local_player_id();
        if Some(source.entity_id) != local_player_id {
            if let Some(target_team) = self
                .scoreboard
                .team_for_scoreboard_name(target_scoreboard_name)
            {
                let target_parameters = target_team.parameters.as_ref();
                let visibility = target_parameters.map_or("always", |parameters| {
                    parameters.nametag_visibility.as_str()
                });
                let local_team = self.local_player_scoreboard_name().and_then(|name| {
                    self.scoreboard
                        .team_for_scoreboard_name(&name)
                        .map(|team| team.name.clone())
                });
                let allied = local_team.as_deref() == Some(target_team.name.as_str());
                let can_see_friendly_invisibles = target_parameters
                    .is_some_and(|parameters| parameters.can_see_friendly_invisibles());
                let visible = match visibility {
                    "never" => false,
                    "hideForOtherTeams" => {
                        (local_team.is_none() && visible_to_local_player)
                            || (allied && (can_see_friendly_invisibles || visible_to_local_player))
                    }
                    "hideForOwnTeam" => {
                        (local_team.is_none() && visible_to_local_player)
                            || (!allied && visible_to_local_player)
                    }
                    _ => visible_to_local_player,
                };
                return EntityNameTagLivingVisibilityState::TeamOverride { visible };
            }
        }

        EntityNameTagLivingVisibilityState::General {
            visible_to_local_player,
            is_vehicle: self.entities.entity_has_passengers(source.entity_id),
        }
    }

    fn scoreboard_below_name(&self, scoreboard_name: &str) -> Option<EntityNameTagTextState> {
        let objective_name = self.scoreboard.display_slots.get("below_name")?;
        let objective = self.scoreboard.objectives.get(objective_name)?;
        let score = self
            .scoreboard
            .scores
            .get(scoreboard_name)
            .and_then(|scores| scores.get(objective_name));
        let value = score.map_or(0, |score| score.value);
        let number_format = score
            .and_then(|score| score.number_format.as_ref())
            .or(objective.number_format.as_ref());
        let mut segments = Vec::with_capacity(3);
        match number_format {
            Some(NumberFormatSummary::Blank) => {}
            Some(NumberFormatSummary::Styled(style)) => {
                segments.push(EntityNameTagTextSegmentState::Styled(vec![StyledTextRun {
                    text: value.to_string(),
                    style: style.style().clone(),
                }]));
            }
            Some(NumberFormatSummary::Fixed(component)) => {
                segments.push(EntityNameTagTextSegmentState::Rich(component.clone()));
            }
            None => segments.push(EntityNameTagTextSegmentState::Styled(vec![plain_run(
                value.to_string(),
            )])),
        }
        segments.push(EntityNameTagTextSegmentState::Styled(vec![plain_run(" ")]));
        segments.push(stored_component_segment(
            objective.display_name_component.as_ref(),
            &objective.display_name_styled,
            &objective.display_name,
        ));
        Some(EntityNameTagTextState {
            segments,
            inherited_style: ComponentStyle::default(),
        })
    }

    fn mannequin_below_name(
        &self,
        source: &EntityModelSourceState,
    ) -> Option<EntityNameTagTextState> {
        let accessor = schema::vanilla_entity_metadata_accessor_for_entity_type(
            source.entity_type_id,
            "DATA_DESCRIPTION",
        )?;
        let segment = match self
            .entities
            .metadata_optional_rich_component_override(source.entity_id, accessor.data_id)
        {
            Some(Some(component)) => EntityNameTagTextSegmentState::Rich(component),
            Some(None) => return None,
            None => EntityNameTagTextSegmentState::Translation {
                key: "entity.minecraft.mannequin.label".to_string(),
            },
        };
        Some(EntityNameTagTextState {
            segments: vec![segment],
            inherited_style: ComponentStyle::default(),
        })
    }
}

fn name_tag_attachment_y(source: &EntityModelSourceState) -> f32 {
    let height = source.bounding_box_height.max(0.0);
    let Some(entity_type) = vanilla_entity_type(source.entity_type_id) else {
        return height;
    };
    if source.entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID
        && sniffer_is_digging(&source.data_values)
    {
        // Sniffer's digging dimensions are newly constructed and therefore use
        // NAME_TAG's height fallback instead of EntityType's explicit 2.05 point.
        return height;
    }
    entity_type.name_tag_offset.map_or(height, |offset| {
        if entity_type.height > 0.0 {
            offset * height / entity_type.height
        } else {
            height
        }
    })
}

fn entity_description_key(entity_type_id: i32) -> Option<String> {
    let resource_id = vanilla_entity_type(entity_type_id)?.resource_id;
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    Some(format!("entity.{namespace}.{}", path.replace('/', ".")))
}

fn stored_component_segment(
    exact: Option<&NbtSummaryValue>,
    styled: &[StyledTextRun],
    plain: &str,
) -> EntityNameTagTextSegmentState {
    if let Some(component) = exact
        .cloned()
        .and_then(|component| RichTextComponent::try_from_component_nbt(component).ok())
    {
        return EntityNameTagTextSegmentState::Rich(component);
    }
    if !styled.is_empty() {
        EntityNameTagTextSegmentState::Styled(styled.to_vec())
    } else {
        EntityNameTagTextSegmentState::Styled(vec![plain_run(plain)])
    }
}

fn plain_run(text: impl Into<String>) -> StyledTextRun {
    StyledTextRun {
        text: text.into(),
        style: ComponentStyle::default(),
    }
}

fn metadata_byte(values: &[EntityDataValue], data_id: u8, default: i8) -> i8 {
    values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn metadata_bool(values: &[EntityDataValue], data_id: u8, default: bool) -> bool {
    values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}
