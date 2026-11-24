//! `SignBlockEntity` (`SignBlockEntity.java:36-280`) and the sign-block
//! geometry the interaction reads.
//!
//! `HangingSignBlockEntity` (`HangingSignBlockEntity.java:8-29`) is the same
//! object: everything it overrides — `getTextLineHeight`, `getMaxTextLineWidth`
//! and `getSignInteractionFailedSoundEvent` — is either a client render metric
//! or a sound this server does not own yet, so the two registry types share one
//! payload and are told apart by the type id the block declares.
//!
//! # Components as NBT
//!
//! A `SignText` line is a `Component`, and the only thing this server does with
//! one is store it, hand it back to a client, and replace its text while
//! keeping its style. Round-tripping through a parsed component tree would lose
//! whatever 26.1 shape this crate does not model, so a line is held as the value
//! `ComponentSerialization.CODEC` (`ComponentSerialization.java:95-115`) writes:
//! a bare string when `Component.tryCollapseToString` succeeds — literal
//! contents, no siblings, empty style — and the full record otherwise.

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::builtin_enum_component_values::{
    builtin_enum_component_value_for_name, vanilla_dye_color_name,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID;
use bbb_protocol::item_types::{
    vanilla_item_type, VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
    VANILLA_ITEM_GLOW_INK_SAC_ID, VANILLA_ITEM_INK_SAC_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::value_input::{boolean_or, codec_boolean};
use super::{entry, unique_field};

/// `SignText.LINES` (`SignText.java:37`).
pub const SIGN_TEXT_LINES: usize = 4;

/// `SignBlockEntity.playerIsTooFarAwayToEdit`'s reach
/// (`SignBlockEntity.java:260-263`): the editor is dropped once the player who
/// claimed it is further than this from the sign.
pub const SIGN_EDIT_RANGE_BUFFER: f64 = 4.0;

/// `DyeColor.BLACK`, the `SignText.DIRECT_CODEC` default for `color`
/// (`SignText.java:32`).
const DEFAULT_SIGN_TEXT_COLOR: &str = "black";

/// The field names `Style.Serializer.MAP_CODEC` reads and writes
/// (`Style.java:613-628`), in its own order.
///
/// This is the closed set that separates a component's style from its contents
/// in the flat form: replacing a line's text while keeping its style is exactly
/// "keep these keys, drop the rest", which is what
/// `SignBlockEntity.setMessages` (`SignBlockEntity.java:145-159`) does when it
/// builds `Component.literal(line).setStyle(currentTextStyle)`.
const COMPONENT_STYLE_FIELDS: [&str; 11] = [
    "color",
    "shadow_color",
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
    "click_event",
    "hover_event",
    "insertion",
    "font",
];

/// `SignText` (`SignText.java:20-137`).
///
/// Vanilla's is immutable and every mutator answers a fresh instance; the
/// identity comparison `setFrontText`/`setBackText` make
/// (`SignBlockEntity.java:165-183`) is therefore always "different" once a
/// mutator has run, which is why the callers here mark the sign updated
/// unconditionally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignText {
    /// `SignText.messages` (`SignText.java:38`).
    messages: [NbtSummaryValue; SIGN_TEXT_LINES],
    /// `SignText.filteredMessages` (`SignText.java:39`).
    filtered_messages: [NbtSummaryValue; SIGN_TEXT_LINES],
    /// `SignText.color` (`SignText.java:40`) as its `DyeColor.CODEC` name.
    color: String,
    /// `SignText.hasGlowingText` (`SignText.java:41`).
    has_glowing_text: bool,
}

impl Default for SignText {
    /// `SignText()` (`SignText.java:45-47`): four `CommonComponents.EMPTY`
    /// lines on both faces, black, not glowing. `CommonComponents.EMPTY` is a
    /// literal with no style, so its flat form is the empty string.
    fn default() -> Self {
        Self {
            messages: empty_messages(),
            filtered_messages: empty_messages(),
            color: DEFAULT_SIGN_TEXT_COLOR.to_string(),
            has_glowing_text: false,
        }
    }
}

/// `SignText.emptyMessages` (`SignText.java:56-58`).
fn empty_messages() -> [NbtSummaryValue; SIGN_TEXT_LINES] {
    std::array::from_fn(|_| NbtSummaryValue::string(String::new()))
}

impl SignText {
    /// `SignText.getMessages(shouldFilter)` (`SignText.java:100-102`).
    #[must_use]
    pub fn messages(&self, should_filter: bool) -> &[NbtSummaryValue; SIGN_TEXT_LINES] {
        if should_filter {
            &self.filtered_messages
        } else {
            &self.messages
        }
    }

    /// `SignText.getMessage(index, shouldFilter)` (`SignText.java:80-82`).
    #[must_use]
    pub fn message(&self, index: usize, should_filter: bool) -> &NbtSummaryValue {
        &self.messages(should_filter)[index]
    }

    /// The four lines as the plain strings a caller reads back, i.e. each
    /// component's collapsed text.
    #[must_use]
    pub fn plain_lines(&self, should_filter: bool) -> [bbb_protocol::JavaString; SIGN_TEXT_LINES] {
        let messages = self.messages(should_filter);
        std::array::from_fn(|index| component_plain_text(&messages[index]))
    }

    /// `SignText.getColor` (`SignText.java:72-74`).
    #[must_use]
    pub fn color(&self) -> &str {
        &self.color
    }

    /// `SignText.hasGlowingText` (`SignText.java:64-66`).
    #[must_use]
    pub const fn has_glowing_text(&self) -> bool {
        self.has_glowing_text
    }

    /// `SignText.setHasGlowingText` (`SignText.java:68-70`).
    #[must_use]
    pub fn set_has_glowing_text(mut self, has_glowing_text: bool) -> Self {
        if has_glowing_text == self.has_glowing_text {
            self
        } else {
            self.has_glowing_text = has_glowing_text;
            self
        }
    }

    /// `SignText.setColor` (`SignText.java:76-78`).
    #[must_use]
    pub fn set_color(mut self, color: &str) -> Self {
        if color == self.color {
            self
        } else {
            self.color = color.to_string();
            self
        }
    }

    /// `SignText.hasMessage` (`SignText.java:96-98`).
    #[must_use]
    pub fn has_message(&self, should_filter: bool) -> bool {
        self.messages(should_filter)
            .iter()
            .any(|message| !component_plain_text(message).is_empty())
    }

    /// `SignText.setMessage(index, rawMessage, filteredMessage)`
    /// (`SignText.java:88-94`).
    fn set_message(&mut self, index: usize, raw: NbtSummaryValue, filtered: NbtSummaryValue) {
        self.messages[index] = raw;
        self.filtered_messages[index] = filtered;
    }

    /// `SignText.DIRECT_CODEC`'s encode (`SignText.java:28-36`).
    ///
    /// `color` and `has_glowing_text` are `fieldOf(...).orElse(...)`, which
    /// defaults only on the way in and always writes on the way out;
    /// `filtered_messages` is `lenientOptionalFieldOf` over
    /// `SignText.filteredMessages` (`SignText.java:117-125`), which answers
    /// empty — and so writes nothing — while every filtered line equals its
    /// raw line.
    #[must_use]
    pub fn save(&self) -> NbtSummaryValue {
        let mut entries = vec![entry(
            "messages",
            NbtSummaryValue::List(self.messages.to_vec()),
        )];
        if self.filtered_messages != self.messages {
            entries.push(entry(
                "filtered_messages",
                NbtSummaryValue::List(self.filtered_messages.to_vec()),
            ));
        }
        entries.push(entry("color", NbtSummaryValue::string(self.color.clone())));
        entries.push(entry(
            "has_glowing_text",
            NbtSummaryValue::Byte(i8::from(self.has_glowing_text)),
        ));
        NbtSummaryValue::Compound(entries)
    }

    /// `SignText.DIRECT_CODEC`'s decode through `SignText.load`
    /// (`SignText.java:60-62`): a missing `filtered_messages` copies the raw
    /// lines, and the two scalar fields fall back to their `orElse` defaults.
    ///
    /// `None` where the codec would fail: `LINES_CODEC` is `Util.fixedSize(4)`
    /// (`SignText.java:21-27`), so a list of any other length is not a
    /// `SignText`.
    #[must_use]
    pub fn load(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(entries) = value else {
            return None;
        };
        let messages = fixed_size_lines(unique_field(entries, "messages")?)?;
        let filtered_messages = match unique_field(entries, "filtered_messages") {
            Some(value) => fixed_size_lines(value)?,
            None => messages.clone(),
        };
        let color = match unique_field(entries, "color") {
            Some(NbtSummaryValue::String(color)) => color
                .try_to_string()
                .unwrap_or_else(|_| DEFAULT_SIGN_TEXT_COLOR.into()),
            _ => DEFAULT_SIGN_TEXT_COLOR.to_string(),
        };
        let has_glowing_text = unique_field(entries, "has_glowing_text")
            .and_then(codec_boolean)
            .unwrap_or(false);
        Some(Self {
            messages,
            filtered_messages,
            color,
            has_glowing_text,
        })
    }
}

fn fixed_size_lines(value: &NbtSummaryValue) -> Option<[NbtSummaryValue; SIGN_TEXT_LINES]> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    if values.len() != SIGN_TEXT_LINES {
        return None;
    }
    Some(std::array::from_fn(|index| values[index].clone()))
}

/// `SignBlockEntity` (`SignBlockEntity.java:36-54`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignBlockEntity {
    /// `SignBlockEntity.frontText` (`SignBlockEntity.java:42`).
    front_text: SignText,
    /// `SignBlockEntity.backText` (`SignBlockEntity.java:43`).
    back_text: SignText,
    /// `SignBlockEntity.isWaxed` (`SignBlockEntity.java:44`).
    is_waxed: bool,
    /// `SignBlockEntity.playerWhoMayEdit` (`SignBlockEntity.java:41`), which
    /// `saveAdditional` never writes: an editor claim does not survive a save.
    player_who_may_edit: Option<Uuid>,
    /// Loaded lines `loadLines` (`SignBlockEntity.java:109-127`) still has to
    /// resolve; the level does it with the command source the object lacks.
    lines_to_resolve: bool,
}

/// What one `SignBlockEntity.updateSignText` did, so the caller can run the
/// level-side half — the `setChanged` mark and the block update — outside the
/// object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignTextUpdate {
    /// The lines were written; `markUpdated` (`SignBlockEntity.java:241-244`)
    /// and the second `sendBlockUpdated` (`:134`) are owed.
    Updated,
    /// `SignBlockEntity.java:135-137`: the sign is waxed, or the player is not
    /// the one who claimed the editor. Vanilla logs and changes nothing.
    Refused,
}

impl SignBlockEntity {
    /// `SignBlockEntity.getText(isFrontText)` (`SignBlockEntity.java:73-75`).
    #[must_use]
    pub const fn text(&self, is_front_text: bool) -> &SignText {
        if is_front_text {
            &self.front_text
        } else {
            &self.back_text
        }
    }

    /// `SignBlockEntity.isWaxed` (`SignBlockEntity.java:246-248`).
    #[must_use]
    pub const fn is_waxed(&self) -> bool {
        self.is_waxed
    }

    /// `SignBlockEntity.getPlayerWhoMayEdit` (`SignBlockEntity.java:237-239`).
    #[must_use]
    pub const fn player_who_may_edit(&self) -> Option<Uuid> {
        self.player_who_may_edit
    }

    /// `SignBlockEntity.setAllowedPlayerEditor` (`SignBlockEntity.java:233-235`).
    pub fn set_allowed_player_editor(&mut self, player: Option<Uuid>) {
        self.player_who_may_edit = player;
    }

    /// `SignBlockEntity.updateSignText(player, frontText, lines)`
    /// (`SignBlockEntity.java:130-138`).
    ///
    /// `lines` are the four strings the packet carried, already through
    /// `ChatFormatting.stripFormatting` (`ChatFormatting.java:107-109`) and
    /// through the text filter — which is `TextFilter.DUMMY`'s pass-through
    /// (`TextFilter.java:8-18`) on a server with no filter service, so the raw
    /// and filtered halves of every `FilteredText` are the same string.
    pub fn update_sign_text(
        &mut self,
        player: Uuid,
        text_filtering_enabled: bool,
        is_front_text: bool,
        lines: &[String; SIGN_TEXT_LINES],
    ) -> SignTextUpdate {
        if self.is_waxed || self.player_who_may_edit != Some(player) {
            return SignTextUpdate::Refused;
        }
        let mut text = self.text(is_front_text).clone();
        set_messages(&mut text, text_filtering_enabled, lines);
        self.set_text(text, is_front_text);
        // `SignBlockEntity.java:133`: the claim is released by the write, so
        // the next click opens the editor again.
        self.set_allowed_player_editor(None);
        SignTextUpdate::Updated
    }

    /// `SignBlockEntity.setText(text, isFrontText)`
    /// (`SignBlockEntity.java:161-163`).
    pub fn set_text(&mut self, text: SignText, is_front_text: bool) {
        if is_front_text {
            self.front_text = text;
        } else {
            self.back_text = text;
        }
    }

    /// `SignBlockEntity.updateText` (`SignBlockEntity.java:140-143`) through
    /// `setFrontText`/`setBackText` (`:165-183`): the mutator is applied and
    /// the face is written only when the resulting `SignText` is a different
    /// value. Vanilla compares by identity because every mutator that actually
    /// changes a field allocates a new instance and otherwise returns `this`.
    pub fn update_text(
        &mut self,
        is_front_text: bool,
        function: impl FnOnce(SignText) -> SignText,
    ) -> bool {
        let current = self.text(is_front_text).clone();
        let next = function(current.clone());
        if next == current {
            return false;
        }
        self.set_text(next, is_front_text);
        true
    }

    /// `SignBlockEntity.setWaxed` (`SignBlockEntity.java:250-258`).
    pub fn set_waxed(&mut self, is_waxed: bool) -> bool {
        if self.is_waxed == is_waxed {
            false
        } else {
            self.is_waxed = is_waxed;
            true
        }
    }

    /// `SignBlockEntity.saveAdditional` (`SignBlockEntity.java:94-99`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        vec![
            entry("front_text", self.front_text.save()),
            entry("back_text", self.back_text.save()),
            entry("is_waxed", NbtSummaryValue::Byte(i8::from(self.is_waxed))),
        ]
    }

    /// `SignBlockEntity.loadAdditional` (`SignBlockEntity.java:102-107`).
    ///
    /// A face the record does not carry, or carries in a shape the codec
    /// rejects, falls back to `SignText::new` exactly as the `orElseGet` does.
    /// A bare string line is a plain literal, which `loadLines` leaves as is.
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.front_text = unique_field(record, "front_text")
            .and_then(SignText::load)
            .unwrap_or_default();
        self.back_text = unique_field(record, "back_text")
            .and_then(SignText::load)
            .unwrap_or_default();
        self.is_waxed = boolean_or(record, "is_waxed", false);
        self.lines_to_resolve = [&self.front_text, &self.back_text].iter().any(|text| {
            text.messages
                .iter()
                .chain(&text.filtered_messages)
                .any(|line| !matches!(line, NbtSummaryValue::String(_)))
        });
    }

    /// Whether lines were loaded that still need `loadLines`; clears it.
    pub const fn has_lines_to_resolve(&self) -> bool {
        self.lines_to_resolve
    }

    pub fn take_lines_to_resolve(&mut self) -> bool {
        std::mem::take(&mut self.lines_to_resolve)
    }

    /// `loadLines` over both faces, raw and filtered.
    pub fn resolve_lines(&mut self, mut resolve: impl FnMut(&NbtSummaryValue) -> NbtSummaryValue) {
        for text in [&mut self.front_text, &mut self.back_text] {
            for line in text
                .messages
                .iter_mut()
                .chain(text.filtered_messages.iter_mut())
            {
                *line = resolve(line);
            }
        }
    }

    /// `SignBlockEntity.tick` (`SignBlockEntity.java:265-270`) with its
    /// `clearInvalidPlayerWhoMayEdit` (`:272-276`) folded in: the one thing a
    /// sign does every tick is drop an editor claim whose player has walked
    /// out of range.
    ///
    /// `playerIsTooFarAwayToEdit` (`:260-263`) is answered by the caller
    /// because it needs the level's player list; a player who is not online at
    /// all is "too far" there, which is the `editingPlayer == null` half.
    pub fn tick(&mut self, player_is_too_far_away_to_edit: impl FnOnce(Uuid) -> bool) {
        let Some(player) = self.player_who_may_edit else {
            return;
        };
        if player_is_too_far_away_to_edit(player) {
            self.set_allowed_player_editor(None);
        }
    }
}

/// `SignBlockEntity.setMessages(player, lines, text)`
/// (`SignBlockEntity.java:145-159`): each line becomes a literal carrying the
/// style the line it replaces had.
///
/// Vanilla's two branches differ only in where the filtered half comes from —
/// `:150` writes one component into both halves, `:152-154` writes
/// `line.raw()` into the raw half and `line.filteredOrEmpty()` into the
/// filtered one. Under `TextFilter.DUMMY` (`TextFilter.java:8-18`) every
/// `FilteredText` is `passThrough` with `FilterMask.PASS_THROUGH`
/// (`FilteredText.java:10-12`), so `filteredOrEmpty()` (`:22-24`) *is*
/// `raw()`, and both branches write the same component to both halves. The
/// player's filtering flag still picks which half `currentTextStyle` is read
/// from (`:148`), so it is not dropped.
fn set_messages(
    text: &mut SignText,
    text_filtering_enabled: bool,
    lines: &[String; SIGN_TEXT_LINES],
) {
    for (index, line) in lines.iter().enumerate() {
        let style = component_style(text.message(index, text_filtering_enabled));
        let message = literal_with_style(line, &style);
        text.set_message(index, message.clone(), message);
    }
}

/// `SignBlock.hasEditableText` (`SignBlock.java:154-158`): every line is
/// `CommonComponents.EMPTY` or has `PlainTextContents`.
#[must_use]
pub fn has_editable_text(text: &SignText, should_filter: bool) -> bool {
    text.messages(should_filter).iter().all(is_plain_text)
}

/// `HoneycombItem.tryApplyToSign`'s `levelEvent(null, 3003, pos, 0)`
/// (`HoneycombItem.java:173`).
pub const LEVEL_EVENT_WAX_ON: i32 = 3003;

/// The four `SignApplicator` item classes (`SignApplicator.java:8-14`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignApplicator {
    /// `DyeItem` (`DyeItem.java:14-46`).
    Dye { color: String },
    /// `InkSacItem` (`InkSacItem.java:9-23`).
    InkSac,
    /// `GlowInkSacItem` (`GlowInkSacItem.java:9-23`).
    GlowInkSac,
    /// `HoneycombItem` (`HoneycombItem.java:27-184`).
    Honeycomb,
}

/// The item class that implements `SignApplicator`, when the held stack is one.
///
/// Dyes are the generated `DyeItem` living-entity override; honeycomb is the
/// generated `HoneycombItem` `useOn` override; the two ink sacs have no other
/// generated class tag and are the catalog ids those registrations occupy.
#[must_use]
pub fn sign_applicator(item: &ItemStackSummary) -> Option<SignApplicator> {
    let item_id = item.item_id?;
    let entry = vanilla_item_type(item_id)?;
    if entry.interact_living_entity_override
        == Some(VanillaItemInteractLivingEntityOverride::DyeItem)
    {
        return Some(SignApplicator::Dye {
            color: dye_color_for_stack(item)?,
        });
    }
    if item_id == VANILLA_ITEM_INK_SAC_ID {
        return Some(SignApplicator::InkSac);
    }
    if item_id == VANILLA_ITEM_GLOW_INK_SAC_ID {
        return Some(SignApplicator::GlowInkSac);
    }
    if entry.use_on_override == Some(VanillaItemUseOnOverride::HoneycombItem) {
        return Some(SignApplicator::Honeycomb);
    }
    None
}

/// `ItemStack.get(DataComponents.DYE)` (`DyeItem.java:39`): the patch wins,
/// a removed `minecraft:dye` component answers none, and otherwise the
/// default color the `DyeItem` registration carries.
fn dye_color_for_stack(item: &ItemStackSummary) -> Option<String> {
    if let Some(id) = item.component_patch.dye_color_id {
        return Some(vanilla_dye_color_name(id).to_string());
    }
    if item
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DYE_TYPE_ID)
    {
        return None;
    }
    let resource_id = vanilla_item_type(item.item_id?)?.resource_id;
    let name = resource_id
        .strip_prefix("minecraft:")
        .unwrap_or(resource_id);
    let color = name.strip_suffix("_dye")?;
    builtin_enum_component_value_for_name(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, color)
        .map(|entry| entry.serialized_name.to_string())
}

/// `SignApplicator.canApplyToSign` (`SignApplicator.java:11-13`), with
/// `HoneycombItem`'s override (`HoneycombItem.java:181-183`).
#[must_use]
pub fn can_apply_to_sign(
    applicator: &SignApplicator,
    text: &SignText,
    text_filtering_enabled: bool,
) -> bool {
    match applicator {
        SignApplicator::Honeycomb => true,
        SignApplicator::Dye { .. } | SignApplicator::InkSac | SignApplicator::GlowInkSac => {
            text.has_message(text_filtering_enabled)
        }
    }
}

/// `SignApplicator.tryApplyToSign` for the four item classes
/// (`DyeItem.java:38-46`, `InkSacItem.java:15-22`, `GlowInkSacItem.java:15-22`,
/// `HoneycombItem.java:171-178`). Sounds and the wax-on level event stay with
/// the caller, which holds the level.
pub fn try_apply_to_sign(
    sign: &mut SignBlockEntity,
    applicator: &SignApplicator,
    is_front_text: bool,
) -> bool {
    match applicator {
        SignApplicator::Dye { color } => {
            sign.update_text(is_front_text, |text| text.set_color(color))
        }
        SignApplicator::InkSac => {
            sign.update_text(is_front_text, |text| text.set_has_glowing_text(false))
        }
        SignApplicator::GlowInkSac => {
            sign.update_text(is_front_text, |text| text.set_has_glowing_text(true))
        }
        SignApplicator::Honeycomb => sign.set_waxed(true),
    }
}

/// Whether a flat component value carries `PlainTextContents`.
///
/// `ComponentSerialization.createCodec` (`ComponentSerialization.java:95-115`)
/// writes a bare string for a collapsible literal, a one-element-or-more list
/// for `createFromList` (`:74-81`, whose contents are the first element's),
/// and otherwise the record whose contents map is picked by the `type`
/// discriminator or, without one, by `FuzzyCodec` over the seven content
/// codecs (`:107-125`) — of which only `"text"` is `PlainTextContents`.
fn is_plain_text(value: &NbtSummaryValue) -> bool {
    match value {
        NbtSummaryValue::String(_) => true,
        NbtSummaryValue::List(values) => values.first().is_some_and(is_plain_text),
        NbtSummaryValue::Compound(entries) => match unique_field(entries, "type") {
            Some(NbtSummaryValue::String(kind)) => kind == "text" || kind == "minecraft:text",
            Some(_) => false,
            None => unique_field(entries, "text").is_some(),
        },
        _ => false,
    }
}

/// The text a component collapses to (`Component.getString`), delegated to the
/// server language walker so `"extra"` siblings and `"translate"` resolve.
fn component_plain_text(value: &NbtSummaryValue) -> bbb_protocol::JavaString {
    bbb_server_datapack::language::component_plain_text(value).into()
}

/// The style half of a flat component: the keys
/// `Style.Serializer.MAP_CODEC` owns (`Style.java:613-628`) and nothing else.
/// A bare string has none, which is the empty `Style` its
/// `Component.literal` carries.
fn component_style(value: &NbtSummaryValue) -> Vec<NbtSummaryEntry> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Vec::new();
    };
    COMPONENT_STYLE_FIELDS
        .iter()
        .filter_map(|name| unique_field(entries, name).map(|value| entry(*name, value.clone())))
        .collect()
}

/// `Component.literal(raw).setStyle(style)` in its flat form: a bare string
/// when the style is empty, because `tryCollapseToString`
/// (`ComponentSerialization.java:110-113`) succeeds for a styleless literal
/// with no siblings, and the record otherwise.
fn literal_with_style(raw: &str, style: &[NbtSummaryEntry]) -> NbtSummaryValue {
    if style.is_empty() {
        return NbtSummaryValue::string(raw);
    }
    let mut entries = vec![entry("text", NbtSummaryValue::string(raw))];
    entries.extend_from_slice(style);
    NbtSummaryValue::Compound(entries)
}

/// Which of the four 26.1 sign block classes a state belongs to, as the two
/// facts the geometry below needs.
///
/// The class is not read off a name: the registry type tells a sign from a
/// hanging sign (`BlockEntityType.SIGN` versus `HANGING_SIGN`,
/// `BlockEntityType.java:41-96`), and the state's own property set tells the
/// wall-mounted form — `FACING` — from the free-standing one — `ROTATION` —
/// because those are the properties the four `createBlockStateDefinition`
/// bodies declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignRotation {
    /// `StandingSignBlock.getYRotationDegrees`
    /// (`StandingSignBlock.java:67-69`) and
    /// `CeilingHangingSignBlock.getYRotationDegrees`
    /// (`CeilingHangingSignBlock.java:154-156`).
    Segment(i32),
    /// `WallSignBlock.getYRotationDegrees` (`WallSignBlock.java:91-93`) and
    /// `WallHangingSignBlock.getYRotationDegrees`
    /// (`WallHangingSignBlock.java:153-155`).
    Facing(SignFacing),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignFacing {
    North,
    South,
    West,
    East,
}

impl SignFacing {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "north" => Self::North,
            "south" => Self::South,
            "west" => Self::West,
            "east" => Self::East,
            _ => return None,
        })
    }

    /// `Direction.toYRot` (`Direction.java:303-305`): `(data2d & 3) * 90`,
    /// over the horizontal order south, west, north, east
    /// (`Direction.java`'s `data2d` numbering).
    const fn to_y_rot(self) -> f32 {
        match self {
            Self::South => 0.0,
            Self::West => 90.0,
            Self::North => 180.0,
            Self::East => 270.0,
        }
    }
}

fn sign_rotation(properties: &BTreeMap<String, String>) -> Option<SignRotation> {
    if let Some(facing) = properties.get("facing") {
        return SignFacing::parse(facing).map(SignRotation::Facing);
    }
    let rotation = properties.get("rotation")?.parse::<i32>().ok()?;
    Some(SignRotation::Segment(rotation))
}

/// `RotationSegment.convertToDegrees(segment)`
/// (`RotationSegment.java:38-40`) through
/// `SegmentedAnglePrecision(4).toDegrees` (`SegmentedAnglePrecision.java:53-56`
/// with `:20-24`): sixteen segments of 22.5 degrees, normalised into
/// `[-180, 180)`.
fn segment_to_degrees(segment: i32) -> f32 {
    let degrees = (segment & 15) as f32 * (360.0 / 16.0);
    if degrees >= 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

/// `SignBlock.getYRotationDegrees(state)` (`SignBlock.java:160`), dispatched
/// over the four concrete sign blocks.
fn sign_y_rotation_degrees(properties: &BTreeMap<String, String>) -> Option<f32> {
    Some(match sign_rotation(properties)? {
        SignRotation::Segment(segment) => segment_to_degrees(segment),
        SignRotation::Facing(facing) => facing.to_y_rot(),
    })
}

/// `SignBlock.getSignHitboxCenterPosition(state)`'s x and z
/// (`SignBlock.java:162-164`), which is the block centre unless
/// `WallSignBlock` overrides it (`WallSignBlock.java:96-98`) with the centre
/// of its own outline.
///
/// That outline is `Shapes.rotateHorizontal(Block.boxZ(16.0, 4.5, 12.5, 14.0,
/// 16.0))` (`WallSignBlock.java:31`). `Block.boxZ(sizeX, minY, maxY, minZ,
/// maxZ)` is `box(8 - sizeX/2, minY, minZ, 8 + sizeX/2, maxY, maxZ)`
/// (`Block.java:200-203`) over sixteenths (`Block.java:164-166`), so the
/// north form spans x `[0, 1]` and z `[0.875, 1]` and its centre is
/// `(0.5, 0.9375)`. `Shapes.rotateHorizontal` (`Shapes.java:407-419`) turns
/// the north form by 90, 180 and 270 degrees about the block centre for east,
/// south and west.
fn sign_hitbox_center_xz(
    type_is_hanging: bool,
    properties: &BTreeMap<String, String>,
) -> (f64, f64) {
    const BLOCK_CENTER: f64 = 0.5;
    /// The north wall sign's outline centre on the z axis.
    const WALL_SIGN_CENTER_Z: f64 = (14.0 + 16.0) / 2.0 / 16.0;
    if type_is_hanging {
        return (BLOCK_CENTER, BLOCK_CENTER);
    }
    match sign_rotation(properties) {
        Some(SignRotation::Facing(facing)) => match facing {
            SignFacing::North => (BLOCK_CENTER, WALL_SIGN_CENTER_Z),
            SignFacing::South => (BLOCK_CENTER, 1.0 - WALL_SIGN_CENTER_Z),
            SignFacing::West => (WALL_SIGN_CENTER_Z, BLOCK_CENTER),
            SignFacing::East => (1.0 - WALL_SIGN_CENTER_Z, BLOCK_CENTER),
        },
        _ => (BLOCK_CENTER, BLOCK_CENTER),
    }
}

/// `SignBlockEntity.isFacingFrontText(player)`
/// (`SignBlockEntity.java:60-71`).
///
/// `false` for a state that is not a sign block at all, which is the `else`
/// branch vanilla's `instanceof SignBlock` takes (`:68-70`).
#[must_use]
pub fn is_facing_front_text(
    block_state_id: i32,
    type_is_hanging: bool,
    pos: BlockPos,
    player_x: f64,
    player_z: f64,
) -> bool {
    let Some(identity) = vanilla_block_state_identity(block_state_id) else {
        return false;
    };
    let Some(sign_y_rot) = sign_y_rotation_degrees(&identity.properties) else {
        return false;
    };
    let (offset_x, offset_z) = sign_hitbox_center_xz(type_is_hanging, &identity.properties);
    let xd = player_x - (f64::from(pos.x) + offset_x);
    let zd = player_z - (f64::from(pos.z) + offset_z);
    // `:66`: `Mth.atan2(zd, xd)` in degrees, turned a quarter so that the
    // player's bearing is measured the same way a block's yaw is.
    let player_y_rot = (zd.atan2(xd) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
    // `Mth.degreesDifferenceAbs` (`Mth.java`): the absolute wrapped
    // difference, which `:67` compares against ninety degrees.
    degrees_difference_abs(sign_y_rot, player_y_rot) <= 90.0
}

/// `Mth.degreesDifferenceAbs(angleA, angleB)` (`Mth.java:245-247`) through
/// `Mth.degreesDifference` (`Mth.java:241-243`).
fn degrees_difference_abs(a: f32, b: f32) -> f32 {
    wrap_degrees(b - a).abs()
}

/// `Mth.wrapDegrees(angle)` (`Mth.java:215-226`).
fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

/// `ChatFormatting.stripFormatting(input)` (`ChatFormatting.java:107-109`):
/// every `§` followed by one of `0-9A-FK-OR`, case-insensitive, is removed.
#[must_use]
pub fn strip_formatting(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '§' && chars.peek().is_some_and(|next| is_formatting_code(*next)) {
            chars.next();
            continue;
        }
        result.push(character);
    }
    result
}

/// The `[0-9A-FK-OR]` half of `ChatFormatting.STRIP_FORMATTING_PATTERN`
/// (`ChatFormatting.java:48`), matched case-insensitively.
fn is_formatting_code(character: char) -> bool {
    let upper = character.to_ascii_uppercase();
    upper.is_ascii_digit()
        || ('A'..='F').contains(&upper)
        || ('K'..='O').contains(&upper)
        || upper == 'R'
}
