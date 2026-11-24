//! The suggestion half of `BlockStateParser` (BlockStateParser.java): the
//! provider the parse left installed, and what that provider puts on the
//! builder `fillSuggestions` re-bases onto the cursor it stopped at (`:140`).

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_kind, vanilla_block_property_values,
    vanilla_block_state_identity, VanillaBlockPropertyKind,
};
use bbb_protocol::block_types::vanilla_block_type;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::JavaString;

use super::MOD_STATE_OFFSET_PROPERTY;
use crate::block_queries::block_state_has_block_entity;

/// What one stage offers and where: `start` is the cursor the parse stopped
/// at, as a byte offset into the text the argument started at, which is the
/// start every candidate below carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockArgumentSuggestions {
    pub start: usize,
    pub candidates: BlockArgumentCandidates,
}

/// The three registry lists are named rather than spelled out: their ids are
/// the caller's catalogs, filtered by `SharedSuggestionProvider`'s resource
/// rule rather than by anything this parse knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockArgumentCandidates {
    /// `suggestItem` (BlockStateParser.java:344-346).
    BlockIds,
    /// `suggestBlockIdOrTag` (`:348-352`).
    BlockIdsOrTags,
    /// `suggestTag` (`:340-342`), which spells each id with its `#`.
    Tags,
    /// Everything the reader itself names: the `[` and `{` openers, property
    /// names, `=`, property values, `,` and `]`.
    Literals(Vec<BlockArgumentCandidate>),
}

/// One literal candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockArgumentCandidate {
    pub text: String,
    /// The value behind an `IntegerProperty`'s name, which `addSuggestions`
    /// (`:259-269`) puts on the builder as an `IntegerSuggestion`: that one is
    /// kept where a text candidate equal to the remainder is dropped
    /// (SuggestionsBuilder.java:54-61 against `:72-75`) and ordered by value
    /// rather than by text (IntegerSuggestion.java:60-63).
    pub integer: Option<i32>,
}

/// The provider `parse` (`:143-170`) and the two property readers install,
/// beside the fields those providers read back.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) enum SuggestionProvider {
    /// `SUGGEST_NOTHING` (`:67`), which is both the field's initial value
    /// (`:79`) and what a `{` installs (`:167`).
    #[default]
    Nothing,
    Item,
    BlockIdOrTag,
    Tag,
    OpenPropertiesOrNbt,
    OpenVaguePropertiesOrNbt,
    OpenNbt,
    PropertyNameOrEnd,
    PropertyName,
    Equals,
    PropertyValue(JavaString),
    NextPropertyOrEnd,
    VaguePropertyNameOrEnd,
    VaguePropertyName,
    VaguePropertyValue(JavaString),
}

/// The parser fields the providers read: `this.state` and `this.definition`
/// (`:361-362`), `this.properties`, `this.tag` and `this.vagueProperties`.
///
/// The block is kept as its id and its *default* state because every provider
/// that reads `this.state` asks the block-wide question -- which properties
/// the definition declares, and whether the block has a block entity.
#[derive(Debug, Default)]
pub(super) struct SuggestionTrace {
    pub(super) provider: SuggestionProvider,
    pub(super) block: Option<(String, i32)>,
    pub(super) defined: Vec<String>,
    /// The block type ids of `this.tag`.
    pub(super) tag: Option<Vec<i32>>,
    pub(super) vague: BTreeMap<JavaString, JavaString>,
}

impl SuggestionTrace {
    /// The installed provider applied to a builder whose remainder is
    /// `remaining`, which is the text from the stop cursor to the end of the
    /// input (SuggestionsBuilder.java:18-26).
    pub(super) fn candidates(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
    ) -> BlockArgumentCandidates {
        match &self.provider {
            SuggestionProvider::Item => BlockArgumentCandidates::BlockIds,
            SuggestionProvider::BlockIdOrTag => BlockArgumentCandidates::BlockIdsOrTags,
            SuggestionProvider::Tag => BlockArgumentCandidates::Tags,
            SuggestionProvider::Nothing => literals(Vec::new()),
            SuggestionProvider::OpenPropertiesOrNbt => {
                literals(self.open_properties_or_nbt(remaining, registry))
            }
            SuggestionProvider::OpenVaguePropertiesOrNbt => {
                literals(self.open_vague_properties_or_nbt(remaining, registry))
            }
            SuggestionProvider::OpenNbt => literals(self.open_nbt(remaining, registry)),
            SuggestionProvider::PropertyNameOrEnd => {
                // `:172-178`.
                let mut out = end_of_list(remaining);
                self.property_names(remaining, registry, &mut out);
                literals(out)
            }
            SuggestionProvider::PropertyName => {
                let mut out = Vec::new();
                self.property_names(remaining, registry, &mut out);
                literals(out)
            }
            SuggestionProvider::Equals => {
                // `:239-245`.
                let mut out = Vec::new();
                if remaining.is_empty() {
                    push(&mut out, text("="));
                }
                literals(out)
            }
            SuggestionProvider::PropertyValue(key) => {
                let block = self.block.as_ref().map(|(id, _)| id.as_str());
                literals(
                    block
                        .and_then(|id| key.with_utf8(|key| property_values(id, key, registry)))
                        .unwrap_or_default(),
                )
            }
            SuggestionProvider::NextPropertyOrEnd => {
                literals(self.next_property_or_end(remaining, registry))
            }
            SuggestionProvider::VaguePropertyNameOrEnd => {
                // `:180-186`.
                let mut out = end_of_list(remaining);
                self.vague_property_names(remaining, registry, &mut out);
                literals(out)
            }
            SuggestionProvider::VaguePropertyName => {
                let mut out = Vec::new();
                self.vague_property_names(remaining, registry, &mut out);
                literals(out)
            }
            SuggestionProvider::VaguePropertyValue(key) => {
                literals(self.vague_property_value(key, registry))
            }
        }
    }

    /// `suggestOpenPropertiesOrNbt` (`:326-338`).
    fn open_properties_or_nbt(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
    ) -> Vec<BlockArgumentCandidate> {
        let mut out = Vec::new();
        let Some((_, state)) = self.block else {
            return out;
        };
        if !remaining.is_empty() {
            return out;
        }
        if has_declared_properties(state, registry) {
            push(&mut out, text("["));
        }
        if block_state_has_block_entity(state) {
            push(&mut out, text("{"));
        }
        out
    }

    /// `suggestOpenVaguePropertiesOrNbt` (`:300-324`).
    fn open_vague_properties_or_nbt(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
    ) -> Vec<BlockArgumentCandidate> {
        let mut out = Vec::new();
        if !remaining.is_empty() {
            return out;
        }
        let (mut has_properties, mut has_entity) = (false, false);
        for (_, state) in self.tag_members(registry) {
            has_properties |= has_declared_properties(state, registry);
            has_entity |= block_state_has_block_entity(state);
        }
        if has_properties {
            push(&mut out, text("["));
        }
        if has_entity {
            push(&mut out, text("{"));
        }
        out
    }

    /// `suggestOpenNbt` (`:215-221`) over `hasBlockEntity` (`:223-237`).
    fn open_nbt(&self, remaining: &str, registry: &ContentRegistry) -> Vec<BlockArgumentCandidate> {
        let has_block_entity = match self.block {
            Some((_, state)) => block_state_has_block_entity(state),
            None => self
                .tag_members(registry)
                .into_iter()
                .any(|(_, state)| block_state_has_block_entity(state)),
        };
        let mut out = Vec::new();
        if remaining.is_empty() && has_block_entity {
            push(&mut out, text("{"));
        }
        out
    }

    /// `suggestPropertyName` (`:188-198`).
    fn property_names(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
        out: &mut Vec<BlockArgumentCandidate>,
    ) {
        let prefix = remaining.to_lowercase();
        let Some((_, state)) = self.block else {
            return;
        };
        if let Some(properties) = declared_properties(state) {
            for name in properties.keys() {
                if !self.defined.contains(name) && name.starts_with(&prefix) {
                    push(out, text(&format!("{name}=")));
                }
            }
            return;
        }
        if mod_has_state_offset_property(state, registry)
            && !self
                .defined
                .iter()
                .any(|name| name == MOD_STATE_OFFSET_PROPERTY)
            && MOD_STATE_OFFSET_PROPERTY.starts_with(&prefix)
        {
            push(out, text(&format!("{MOD_STATE_OFFSET_PROPERTY}=")));
        }
    }

    /// `suggestNextPropertyOrEnd` (`:247-257`).
    fn next_property_or_end(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
    ) -> Vec<BlockArgumentCandidate> {
        let mut out = end_of_list(remaining);
        let declared = self.block.as_ref().map_or(0, |(_, state)| {
            declared_properties(*state).map_or_else(
                || usize::from(mod_has_state_offset_property(*state, registry)),
                BTreeMap::len,
            )
        });
        if remaining.is_empty() && self.defined.len() < declared {
            push(&mut out, text(","));
        }
        out
    }

    /// `suggestVaguePropertyName` (`:200-213`): every member's own definition,
    /// there being no one definition a tag agrees on.
    fn vague_property_names(
        &self,
        remaining: &str,
        registry: &ContentRegistry,
        out: &mut Vec<BlockArgumentCandidate>,
    ) {
        let prefix = remaining.to_lowercase();
        for (_, state) in self.tag_members(registry) {
            if let Some(properties) = declared_properties(state) {
                for name in properties.keys() {
                    if !self.vague.keys().any(|key| key == name) && name.starts_with(&prefix) {
                        push(out, text(&format!("{name}=")));
                    }
                }
            } else if mod_has_state_offset_property(state, registry)
                && !self
                    .vague
                    .keys()
                    .any(|key| key == MOD_STATE_OFFSET_PROPERTY)
                && MOD_STATE_OFFSET_PROPERTY.starts_with(&prefix)
            {
                push(out, text(&format!("{MOD_STATE_OFFSET_PROPERTY}=")));
            }
        }
    }

    /// `suggestVaguePropertyValue` (`:271-298`): the values every member that
    /// declares `key` accepts, then a `,` while some member has a property the
    /// argument has not named, and the `]` unconditionally.
    fn vague_property_value(
        &self,
        key: &JavaString,
        registry: &ContentRegistry,
    ) -> Vec<BlockArgumentCandidate> {
        let mut out = Vec::new();
        let mut has_more_properties = false;
        for (id, state) in self.tag_members(registry) {
            for candidate in key
                .with_utf8(|key| property_values(&id, key, registry))
                .unwrap_or_default()
            {
                push(&mut out, candidate);
            }
            if !has_more_properties {
                has_more_properties = declared_properties(state).map_or_else(
                    || {
                        mod_has_state_offset_property(state, registry)
                            && !self
                                .vague
                                .keys()
                                .any(|key| key == MOD_STATE_OFFSET_PROPERTY)
                    },
                    |properties| {
                        properties
                            .keys()
                            .any(|name| !self.vague.keys().any(|key| key == name))
                    },
                );
            }
        }
        if has_more_properties {
            push(&mut out, text(","));
        }
        push(&mut out, text("]"));
        out
    }

    /// The blocks `this.tag` holds, as their ids and default states.
    fn tag_members(&self, registry: &ContentRegistry) -> Vec<(String, i32)> {
        let Some(tag) = &self.tag else {
            return Vec::new();
        };
        let mut members: Vec<(String, i32)> = tag
            .iter()
            .filter_map(|&id| {
                if let (Some(block), Some(state)) =
                    (vanilla_block_type(id), vanilla_block_default_state_id(id))
                {
                    return Some((block.resource_id.to_string(), state));
                }
                let spec = registry.block_by_id(id)?;
                Some((spec.resource_id.clone(), spec.first_state_id))
            })
            .collect();
        // The tag's own order never reaches the wire -- `Suggestions.create`
        // sorts (Suggestions.java:100-101) -- but a stable one keeps this
        // answer reproducible.
        members.sort();
        members
    }
}

/// `addSuggestions` (`:259-269`) for one property of one block.
fn property_values(
    block: &str,
    property: &str,
    registry: &ContentRegistry,
) -> Vec<BlockArgumentCandidate> {
    if let Some(values) = vanilla_block_property_values(block, property) {
        let integer = matches!(
            vanilla_block_property_kind(block, property),
            Some(VanillaBlockPropertyKind::Integer { .. })
        );
        return values
            .iter()
            .map(|&value| BlockArgumentCandidate {
                text: value.to_owned(),
                integer: integer.then(|| value.parse::<i32>().ok()).flatten(),
            })
            .collect();
    }
    if property == MOD_STATE_OFFSET_PROPERTY {
        if let Some(spec) = registry.block_by_resource_id(block) {
            if spec.state_count > 1 {
                return (0..i32::from(spec.state_count))
                    .map(|offset| BlockArgumentCandidate {
                        text: offset.to_string(),
                        integer: Some(offset),
                    })
                    .collect();
            }
        }
    }
    Vec::new()
}

fn mod_has_state_offset_property(state: i32, registry: &ContentRegistry) -> bool {
    registry
        .block_by_state_id(state)
        .is_some_and(|(spec, _)| spec.state_count > 1)
}

fn has_declared_properties(state: i32, registry: &ContentRegistry) -> bool {
    declared_properties(state).is_some_and(|properties| !properties.is_empty())
        || mod_has_state_offset_property(state, registry)
}

/// The `]` the two list providers open with (`:173-175`, `:181-183`,
/// `:248-250`).
fn end_of_list(remaining: &str) -> Vec<BlockArgumentCandidate> {
    if remaining.is_empty() {
        vec![text("]")]
    } else {
        Vec::new()
    }
}

fn literals(candidates: Vec<BlockArgumentCandidate>) -> BlockArgumentCandidates {
    BlockArgumentCandidates::Literals(candidates)
}

fn text(value: &str) -> BlockArgumentCandidate {
    BlockArgumentCandidate {
        text: value.to_string(),
        integer: None,
    }
}

/// A candidate two members of a tag both offer is one candidate:
/// `Suggestions.create` collects into a set (Suggestions.java:94-98) before
/// anything reaches the wire.
fn push(out: &mut Vec<BlockArgumentCandidate>, candidate: BlockArgumentCandidate) {
    if !out.contains(&candidate) {
        out.push(candidate);
    }
}

/// The properties the block's `StateDefinition` declares, which every state of
/// that block carries, read off the default state's own value map.
fn declared_properties(state: i32) -> Option<&'static BTreeMap<String, String>> {
    Some(&vanilla_block_state_identity(state)?.properties)
}
