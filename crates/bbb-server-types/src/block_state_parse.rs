//! `BlockStateParser` (BlockStateParser.java) over the generated block
//! catalog: the block-state and block-predicate readers that command arguments
//! and structure `final_state` strings share.

use std::collections::BTreeMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_kind,
    vanilla_block_property_value_rank_utf16, vanilla_block_property_values,
    VanillaBlockPropertyKind,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{
    read_command_string_value_utf16, scan_command_resource_location_utf16,
    scan_command_whitespace_utf16, CommandSyntaxErrorKind, JavaString, NbtSummaryValue,
};

mod suggest;

pub use suggest::{BlockArgumentCandidate, BlockArgumentCandidates, BlockArgumentSuggestions};
use suggest::{SuggestionProvider, SuggestionTrace};

/// Synthetic integer property name for multi-state custom mod blocks
/// (`0..spec.state_count`), matching chunk palette persistence.
pub const MOD_STATE_OFFSET_PROPERTY: &str = "bbb_state_offset";

/// `BlockInput` (BlockInput.java:22-32) as this crate spells a block state: the
/// global state id the argument resolved to, the names of the properties the
/// argument wrote out, and the block-entity compound it typed beside them.
///
/// The values behind those names are not carried beside them because the state
/// already holds them -- `BlockStateParser.setValue`
/// (BlockStateParser.java:490-499) writes each one into the state as it reads
/// it -- so `overwriteWithDefinedProperties` (BlockInput.java:103-113) reads
/// them back off [`Self::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStateInput {
    pub state: i32,
    pub defined_properties: Vec<String>,
    /// `BlockInput.tag` (BlockInput.java:26, read at `:78-98` and `:54-59`),
    /// which `readNbt` (BlockStateParser.java:486-488) read as one
    /// `TagParser.parseCompoundAsArgument` (TagParser.java:83-86). Always a
    /// [`NbtSummaryValue::Compound`].
    pub block_entity_data: Option<NbtSummaryValue>,
}

/// `BlockPredicateArgument.Result` (BlockPredicateArgument.java:105-107) in the
/// two shapes `parse` (`:46-52`) builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockPredicate {
    /// `BlockPredicateArgument.BlockPredicate` (`:68-103`): a state and the
    /// property names that have to agree with it.
    Block(BlockStateInput),
    /// `BlockPredicateArgument.TagPredicate` (`:109-154`): a block tag and the
    /// vague `key=value` pairs, which are matched by name against whatever
    /// state the tag admits.
    Tag {
        /// The tag the argument named, already qualified. The members are
        /// looked up again when the predicate is tested, the way vanilla's
        /// `HolderSet` resolves through the registry it was taken from.
        id: String,
        vague_properties: BTreeMap<JavaString, JavaString>,
        /// `TagPredicate.nbt` (`:111`, read at `:142-147`), read from the same
        /// place a block state's is (`:166-169`).
        block_entity_data: Option<NbtSummaryValue>,
    },
}

/// The loaded `minecraft:block` tags a block predicate reads `#tag` against
/// (`BlockStateParser.java:372-377` looks the tag up in the parser's registry).
pub trait BlockTagLookup {
    /// The tag's block type ids, or `None` when the registry has no such tag.
    fn block_tag_members(&self, tag_id: &str) -> Option<Vec<i32>>;
}

/// A refusal: the translation key vanilla's reader throws with, its literal
/// arguments, and the cursor the reader stands at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStateParseError {
    pub key: &'static str,
    pub arguments: Vec<JavaString>,
    pub cursor: usize,
}

/// The block-state prefix accepted by `BlockStateParser.parseForBlock`
/// (`BlockStateParser.java:88-103`). Jigsaw replacement deliberately uses
/// only the parsed state, so a following block-entity compound is validated
/// but not applied.
pub fn parse_block_state_prefix(input: &str) -> Option<i32> {
    parse_block_state_prefix_with_registry(input, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`parse_block_state_prefix`].
pub fn parse_block_state_prefix_with_registry(
    input: &str,
    registry: &ContentRegistry,
) -> Option<i32> {
    let (_, block) = read_block_state_with_registry(input, 0, registry).ok()?;
    Some(block.state)
}

/// `BlockStateParser.fillSuggestions` (BlockStateParser.java:128-141) over the
/// text an argument starts at: the same parse, and then the provider it left
/// installed applied to a builder re-based onto the cursor it stopped at
/// (`:140`).
///
/// `tags` is present for the block-predicate half, as everywhere else here.
pub fn block_argument_suggestions(
    text: &str,
    tags: Option<&dyn BlockTagLookup>,
) -> BlockArgumentSuggestions {
    block_argument_suggestions_with_registry(text, tags, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`block_argument_suggestions`].
pub fn block_argument_suggestions_with_registry(
    text: &str,
    tags: Option<&dyn BlockTagLookup>,
    registry: &ContentRegistry,
) -> BlockArgumentSuggestions {
    let mut trace = SuggestionTrace::default();
    let start = JavaString::with_utf16(text, |units| {
        match read_block_argument(units, 0, tags, registry, &mut trace) {
            Ok((end, _)) => end,
            // `:136-138` swallows the refusal and reads the cursor it was left at,
            // which is what every rewind in this file already reproduces.
            Err(refusal) => refusal.cursor,
        }
    });
    let start = byte_cursor(text, start);
    BlockArgumentSuggestions {
        start,
        candidates: trace.candidates(&text[start..], registry),
    }
}

/// `BlockStateArgument.parse` (BlockStateArgument.java:30-33): the block
/// branch of the shared parser, with tags refused.
pub fn read_block_state(
    command: &str,
    cursor: usize,
) -> Result<(usize, BlockStateInput), BlockStateParseError> {
    read_block_state_with_registry(command, cursor, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`read_block_state`].
pub fn read_block_state_with_registry(
    command: &str,
    cursor: usize,
    registry: &ContentRegistry,
) -> Result<(usize, BlockStateInput), BlockStateParseError> {
    read_utf8(command, cursor, |units, cursor| {
        read_block_state_utf16_with_registry(units, cursor, registry)
    })
}

/// Reads a block state with absolute UTF-16 cursors.
pub fn read_block_state_utf16(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, BlockStateInput), BlockStateParseError> {
    read_block_state_utf16_with_registry(command, cursor, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`read_block_state_utf16`].
pub fn read_block_state_utf16_with_registry(
    command: &[u16],
    cursor: usize,
    registry: &ContentRegistry,
) -> Result<(usize, BlockStateInput), BlockStateParseError> {
    let (end, block) = read_block_argument(
        command,
        cursor,
        None,
        registry,
        &mut SuggestionTrace::default(),
    )?;
    match block {
        BlockPredicate::Block(input) => Ok((end, input)),
        BlockPredicate::Tag { .. } => {
            unreachable!("a block-state argument refuses a tag before it reads one")
        }
    }
}

/// `BlockPredicateArgument.parse` (BlockPredicateArgument.java:46-52): the same
/// parser with `forTesting` set, so a `#tag` is read as well as a block.
pub fn read_block_predicate(
    command: &str,
    cursor: usize,
    tags: &dyn BlockTagLookup,
) -> Result<(usize, BlockPredicate), BlockStateParseError> {
    read_block_predicate_with_registry(command, cursor, tags, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`read_block_predicate`].
pub fn read_block_predicate_with_registry(
    command: &str,
    cursor: usize,
    tags: &dyn BlockTagLookup,
    registry: &ContentRegistry,
) -> Result<(usize, BlockPredicate), BlockStateParseError> {
    read_utf8(command, cursor, |units, cursor| {
        read_block_predicate_utf16_with_registry(units, cursor, tags, registry)
    })
}

/// Reads a block predicate with absolute UTF-16 cursors and raw vague properties.
pub fn read_block_predicate_utf16(
    command: &[u16],
    cursor: usize,
    tags: &dyn BlockTagLookup,
) -> Result<(usize, BlockPredicate), BlockStateParseError> {
    read_block_predicate_utf16_with_registry(command, cursor, tags, ContentRegistry::vanilla())
}

/// `ContentRegistry`-aware variant of [`read_block_predicate_utf16`].
pub fn read_block_predicate_utf16_with_registry(
    command: &[u16],
    cursor: usize,
    tags: &dyn BlockTagLookup,
    registry: &ContentRegistry,
) -> Result<(usize, BlockPredicate), BlockStateParseError> {
    read_block_argument(
        command,
        cursor,
        Some(tags),
        registry,
        &mut SuggestionTrace::default(),
    )
}

fn read_utf8<T>(
    command: &str,
    cursor: usize,
    read: impl FnOnce(&[u16], usize) -> Result<(usize, T), BlockStateParseError>,
) -> Result<(usize, T), BlockStateParseError> {
    let start = command[..cursor].encode_utf16().count();
    JavaString::with_utf16(command, |units| read(units, start))
        .map(|(end, value)| (byte_cursor(command, end), value))
        .map_err(|mut failure| {
            failure.cursor = byte_cursor(command, failure.cursor);
            failure
        })
}

fn byte_cursor(input: &str, units: usize) -> usize {
    let mut cursor = 0;
    for (byte, character) in input.char_indices() {
        if cursor >= units {
            return byte;
        }
        cursor += character.len_utf16();
    }
    input.len()
}

/// `BlockStateParser.parse` (BlockStateParser.java:143-170).
///
/// `tags` is present exactly when vanilla's `forTesting` is set, the one thing
/// the two callers disagree about: a predicate may name a tag (`:150-156`) and
/// a block state may not (`:366-368`).
///
/// The block-entity compound both close with (`:166-169`) is read here, after
/// whichever leg read the id and, for a block, its property list. The two
/// command arguments and both their suggestion paths all pass `allowNbt` true
/// (BlockStateArgument.java:31, `:41`, BlockPredicateArgument.java:47, `:60`),
/// as do the two structure `final_state` readers, so there is no arm that
/// would leave it unread.
fn read_block_argument(
    command: &[u16],
    cursor: usize,
    tags: Option<&dyn BlockTagLookup>,
    registry: &ContentRegistry,
    trace: &mut SuggestionTrace,
) -> Result<(usize, BlockPredicate), BlockStateParseError> {
    // `:144-148`.
    trace.provider = if tags.is_some() {
        SuggestionProvider::BlockIdOrTag
    } else {
        SuggestionProvider::Item
    };
    if command.get(cursor) == Some(&u16::from(b'#')) {
        let (end, id) = read_block_tag(command, cursor, tags, trace)?;
        // `:152`.
        trace.provider = SuggestionProvider::OpenVaguePropertiesOrNbt;
        let (end, vague_properties) = if command.get(end) == Some(&u16::from(b'[')) {
            let read = read_vague_block_properties(command, end, &id, trace)?;
            // `:155`.
            trace.provider = SuggestionProvider::OpenNbt;
            read
        } else {
            (end, std::collections::BTreeMap::new())
        };
        let (end, block_entity_data) = read_block_entity_data(command, end, trace)?;
        return Ok((
            end,
            BlockPredicate::Tag {
                id,
                vague_properties,
                block_entity_data,
            },
        ));
    }
    let (end, name, state) = read_block_name(command, cursor, registry, trace)?;
    // `:159`.
    trace.provider = SuggestionProvider::OpenPropertiesOrNbt;
    if command.get(end) != Some(&u16::from(b'[')) {
        let (end, block_entity_data) = read_block_entity_data(command, end, trace)?;
        return Ok((
            end,
            BlockPredicate::Block(BlockStateInput {
                state,
                defined_properties: Vec::new(),
                block_entity_data,
            }),
        ));
    }
    let (end, input) = read_block_properties(command, end, &name, state, registry, trace)?;
    // `:162`.
    trace.provider = SuggestionProvider::OpenNbt;
    let (end, block_entity_data) = read_block_entity_data(command, end, trace)?;
    Ok((
        end,
        BlockPredicate::Block(BlockStateInput {
            block_entity_data,
            ..input
        }),
    ))
}

/// `BlockStateParser.parse`'s block-entity arm (`:166-169`) and `readNbt`
/// (`:486-488`), which is `TagParser.parseCompoundAsArgument`
/// (TagParser.java:83-86): one compound read from where the argument left a
/// `{`, and nothing at all where it left none.
///
/// `Grammar.parseForCommands` (Grammar.java:29-56) throws its own farthest
/// failure, so the key and the cursor the reader carries out are what the player
/// reads. A bound this repository adds instead of Vanilla has no such key and
/// keeps refusing the compound where the `{` stands.
fn read_block_entity_data(
    command: &[u16],
    cursor: usize,
    trace: &mut SuggestionTrace,
) -> Result<(usize, Option<NbtSummaryValue>), BlockStateParseError> {
    if command.get(cursor) != Some(&u16::from(b'{')) {
        return Ok((cursor, None));
    }
    // `:167`, before the compound is read rather than after.
    trace.provider = SuggestionProvider::Nothing;
    match bbb_protocol::parse_snbt_compound_prefix_utf16(&command[cursor..]) {
        Ok((consumed, value)) => Ok((cursor + consumed, Some(value))),
        Err(failure) => Err(match failure.key {
            Some(key) => BlockStateParseError {
                key,
                arguments: failure
                    .arguments
                    .into_iter()
                    .map(JavaString::from)
                    .collect(),
                cursor: cursor + failure.cursor,
            },
            None => BlockStateParseError {
                key: "argument.nbt.expected.value",
                arguments: Vec::new(),
                cursor,
            },
        }),
    }
}

/// `BlockStateParser.readBlock` (BlockStateParser.java:354-363): an
/// `Identifier`, then the block it names and that block's default state.
///
/// `orElseThrow` puts the cursor back at the start of the name (`:358`) before
/// building the refusal, so an unknown block is reported at the name rather
/// than after it.
fn read_block_name(
    command: &[u16],
    cursor: usize,
    registry: &ContentRegistry,
    trace: &mut SuggestionTrace,
) -> Result<(usize, String, i32), BlockStateParseError> {
    let (end, resource_id) =
        scan_command_resource_location_utf16(command, cursor).map_err(|_| {
            // `Identifier.read` (Identifier.java:189-199) rewinds to the start of
            // the greedy run before reporting a malformed id.
            BlockStateParseError {
                key: "argument.id.invalid",
                arguments: Vec::new(),
                cursor,
            }
        })?;
    let Some(state) = vanilla_block_type_for_resource_id(&resource_id)
        .and_then(|entry| vanilla_block_default_state_id(entry.id))
        .or_else(|| {
            registry
                .block_by_resource_id(&resource_id)
                .map(|spec| spec.first_state_id)
        })
    else {
        return Err(BlockStateParseError {
            key: "argument.block.id.invalid",
            arguments: vec![resource_id.clone().into()],
            cursor,
        });
    };
    // `:361-362`, so only a block the registry answered for.
    trace.block = Some((resource_id.clone(), state));
    Ok((end, resource_id, state))
}

/// `BlockStateParser.readTag` (BlockStateParser.java:365-378).
///
/// The unknown-tag key is `arguments.block.tag.unknown` (`:58-60`), with the
/// plural prefix every other refusal in that class does without; both spellings
/// are in `en_us.json` and only this one is the tag's.
fn read_block_tag(
    command: &[u16],
    cursor: usize,
    tags: Option<&dyn BlockTagLookup>,
    trace: &mut SuggestionTrace,
) -> Result<(usize, String), BlockStateParseError> {
    let Some(tags) = tags else {
        // `:366-368`, thrown with the reader still on the `#` and before
        // `:372` narrows the candidates to the tag half.
        return Err(BlockStateParseError {
            key: "argument.block.tag.disallowed",
            arguments: Vec::new(),
            cursor,
        });
    };
    // `:372`, after `expect('#')` and before the id is read.
    trace.provider = SuggestionProvider::Tag;
    let (end, resource_id) =
        scan_command_resource_location_utf16(command, cursor + 1).map_err(|_| {
            BlockStateParseError {
                key: "argument.id.invalid",
                arguments: Vec::new(),
                cursor: cursor + 1,
            }
        })?;
    let Some(members) = tags.block_tag_members(&resource_id) else {
        // `orElseThrow` (`:374-377`) rewinds to the `#` itself, which `start`
        // was taken before `expect('#')` consumed it.
        return Err(BlockStateParseError {
            key: "arguments.block.tag.unknown",
            arguments: vec![resource_id.clone().into()],
            cursor,
        });
    };
    trace.tag = Some(members);
    Ok((end, resource_id))
}

/// `BlockStateParser.readProperties` (BlockStateParser.java:380-431).
///
/// Every refusal names the position vanilla's reader stands at when it throws,
/// which is not the same place for all five: an unknown or duplicate property
/// rewinds to the start of the key (`:391`, `:396`), an invalid value rewinds
/// to the start of the value (`:496`), and a missing `=` and an unclosed list
/// report from wherever the reader stopped.
fn read_block_properties(
    command: &[u16],
    start: usize,
    name: &str,
    default_state: i32,
    registry: &ContentRegistry,
    trace: &mut SuggestionTrace,
) -> Result<(usize, BlockStateInput), BlockStateParseError> {
    let unclosed = |cursor: usize| BlockStateParseError {
        key: "argument.block.property.unclosed",
        arguments: Vec::new(),
        cursor,
    };
    let mod_block = registry.block_by_resource_id(name);
    let mut state = default_state;
    // `:381`, past the `[`.
    let mut cursor = start + 1;
    // `:382`.
    trace.provider = SuggestionProvider::PropertyNameOrEnd;
    skip_whitespace(command, &mut cursor);
    while command
        .get(cursor)
        .is_some_and(|unit| *unit != u16::from(b']'))
    {
        skip_whitespace(command, &mut cursor);
        let key_start = cursor;
        let (after_key, key) = read_quotable_value(command, cursor)?;
        cursor = after_key;
        enum PropertyTarget {
            Vanilla(&'static [&'static str]),
            ModStateOffset {
                first_state_id: i32,
                state_count: u16,
            },
        }
        let Some((property, target)) = key
            .with_utf8(|key| {
                if let Some(values) = vanilla_block_property_values(name, key) {
                    return Some((key.to_owned(), PropertyTarget::Vanilla(values)));
                }
                if key == MOD_STATE_OFFSET_PROPERTY {
                    if let Some(spec) = mod_block {
                        if spec.state_count > 1 {
                            return Some((
                                key.to_owned(),
                                PropertyTarget::ModStateOffset {
                                    first_state_id: spec.first_state_id,
                                    state_count: spec.state_count,
                                },
                            ));
                        }
                    }
                }
                None
            })
            .flatten()
        else {
            // `:390-393`.
            return Err(BlockStateParseError {
                key: "argument.block.property.unknown",
                arguments: vec![name.into(), key.clone()],
                cursor: key_start,
            });
        };
        if trace.defined.contains(&property) {
            // `:395-398`, whose translatable takes the property first and the
            // block second (`:46-48`) where the unknown one takes them the
            // other way round.
            return Err(BlockStateParseError {
                key: "argument.block.property.duplicate",
                arguments: vec![key.clone(), name.into()],
                cursor: key_start,
            });
        }
        skip_whitespace(command, &mut cursor);
        // `:401`.
        trace.provider = SuggestionProvider::Equals;
        if command.get(cursor) != Some(&u16::from(b'=')) {
            // `:402-404`, reported where the reader stopped rather than at the
            // key, and with the property first (`:52-54`).
            return Err(BlockStateParseError {
                key: "argument.block.property.novalue",
                arguments: vec![key.clone(), name.into()],
                cursor,
            });
        }
        cursor += 1;
        skip_whitespace(command, &mut cursor);
        // `:408`.
        trace.provider = SuggestionProvider::PropertyValue(key);
        let value_start = cursor;
        let (after_value, value) = read_quotable_value(command, cursor)?;
        cursor = after_value;
        let Some(next) = (match target {
            PropertyTarget::Vanilla(values) => {
                block_state_with_property(state, name, &property, values, value.units())
            }
            PropertyTarget::ModStateOffset {
                first_state_id,
                state_count,
            } => vanilla_block_property_value_rank_utf16(
                &VanillaBlockPropertyKind::Integer {
                    min: 0,
                    max: i32::from(state_count) - 1,
                },
                value.units(),
            )
            .and_then(|offset| first_state_id.checked_add(offset)),
        }) else {
            // `setValue` (`:490-499`), whose translatable takes the block, the
            // value and then the property name (`:49-51`).
            return Err(BlockStateParseError {
                key: "argument.block.property.invalid",
                arguments: vec![name.into(), value, property.into()],
                cursor: value_start,
            });
        };
        state = next;
        trace.defined.push(property);
        // `:411`.
        trace.provider = SuggestionProvider::NextPropertyOrEnd;
        skip_whitespace(command, &mut cursor);
        if cursor >= command.len() {
            break;
        }
        // `:413-423`: a comma opens the next pair, a `]` ends the list and
        // anything else is the unclosed refusal.
        if command.get(cursor) != Some(&u16::from(b',')) {
            if command.get(cursor) != Some(&u16::from(b']')) {
                return Err(unclosed(cursor));
            }
            break;
        }
        cursor += 1;
        // `:422`, which drops the `]` the list-end provider offered.
        trace.provider = SuggestionProvider::PropertyName;
    }
    if cursor >= command.len() {
        // `:426-430`.
        return Err(unclosed(cursor));
    }
    Ok((
        cursor + 1,
        BlockStateInput {
            state,
            defined_properties: trace.defined.clone(),
            block_entity_data: None,
        },
    ))
}

/// `BlockStateParser.readVagueProperties` (BlockStateParser.java:433-484).
///
/// The differences from [`read_block_properties`] are all in what is *not*
/// checked: a tag has no `StateDefinition`, so neither the property name nor
/// its value is resolved and only the duplicate check survives. The two
/// refusals it does keep are also positioned differently -- a missing `=`
/// rewinds to the key here (`:450`) where the exact form reports where the
/// reader stopped, and an unterminated list rewinds to the last value read
/// (`:478-480`) when one was.
fn read_vague_block_properties(
    command: &[u16],
    start: usize,
    name: &str,
    trace: &mut SuggestionTrace,
) -> Result<(usize, BTreeMap<JavaString, JavaString>), BlockStateParseError> {
    let mut cursor = start + 1;
    let mut value_start: Option<usize> = None;
    // `:435`.
    trace.provider = SuggestionProvider::VaguePropertyNameOrEnd;
    skip_whitespace(command, &mut cursor);
    while command
        .get(cursor)
        .is_some_and(|unit| *unit != u16::from(b']'))
    {
        skip_whitespace(command, &mut cursor);
        let key_start = cursor;
        let (after_key, key) = read_quotable_value(command, cursor)?;
        cursor = after_key;
        if trace.vague.contains_key(&key) {
            return Err(BlockStateParseError {
                key: "argument.block.property.duplicate",
                arguments: vec![key.clone(), name.into()],
                cursor: key_start,
            });
        }
        skip_whitespace(command, &mut cursor);
        // No `suggestEquals` here: the vague loop (`:449-452`) never installs
        // one, so a missing `=` still answers with the property names.
        if command.get(cursor) != Some(&u16::from(b'=')) {
            return Err(BlockStateParseError {
                key: "argument.block.property.novalue",
                arguments: vec![key.clone(), name.into()],
                cursor: key_start,
            });
        }
        cursor += 1;
        skip_whitespace(command, &mut cursor);
        // `:456`, and left installed through the value and the `,`/`]` that
        // follows it -- which is why that provider offers both of them.
        trace.provider = SuggestionProvider::VaguePropertyValue(key.clone());
        value_start = Some(cursor);
        let (after_value, value) = read_quotable_value(command, cursor)?;
        cursor = after_value;
        trace.vague.insert(key, value);
        skip_whitespace(command, &mut cursor);
        if cursor >= command.len() {
            break;
        }
        value_start = None;
        if command.get(cursor) != Some(&u16::from(b',')) {
            if command.get(cursor) != Some(&u16::from(b']')) {
                return Err(BlockStateParseError {
                    key: "argument.block.property.unclosed",
                    arguments: Vec::new(),
                    cursor,
                });
            }
            break;
        }
        cursor += 1;
        // `:471`.
        trace.provider = SuggestionProvider::VaguePropertyName;
    }
    if cursor >= command.len() {
        return Err(BlockStateParseError {
            key: "argument.block.property.unclosed",
            arguments: Vec::new(),
            cursor: value_start.unwrap_or(cursor),
        });
    }
    Ok((cursor + 1, trace.vague.clone()))
}

/// `BlockState.setValue(property, value)` over the generated state registry:
/// the state whose properties are this one's with `property` set to `raw`, or
/// `None` when the block does not accept that value.
///
/// `Property.getValue(raw)` (BlockStateParser.java:491) is what refuses, and
/// for an integer property that is not membership in the definition's value
/// list: `IntegerProperty.getValue` (`IntegerProperty.java:52-59`) parses the
/// text and range-checks the result, so `stage=00`, `stage=+0` and a quoted
/// non-ASCII digit name the listed value they spell.
fn block_state_with_property(
    state: i32,
    block: &str,
    property: &str,
    values: &[&str],
    raw: &[u16],
) -> Option<i32> {
    let canonical = values
        .iter()
        .copied()
        .find(|value| value.encode_utf16().eq(raw.iter().copied()))
        .or_else(|| {
            let kind @ VanillaBlockPropertyKind::Integer { .. } =
                vanilla_block_property_kind(block, property)?
            else {
                return None;
            };
            let rank = vanilla_block_property_value_rank_utf16(kind, raw)?;
            values
                .iter()
                .copied()
                .find(|value| value.parse::<i32>().ok() == Some(rank))
        })?;
    bbb_protocol::block_state_types::vanilla_block_state_with_property(state, property, canonical)
}

/// `TagPredicate.test` (BlockPredicateArgument.java:126-140): resolve the raw
/// value through the candidate's property and compare it with the carried value.
pub fn block_predicate_property_matches(state: i32, property: &[u16], raw: &[u16]) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_property_matches_utf16(
        state, property, raw,
    )
}

/// `ContentRegistry`-aware variant of [`block_predicate_property_matches`].
pub fn block_predicate_property_matches_with_registry(
    state: i32,
    property: &[u16],
    raw: &[u16],
    registry: &ContentRegistry,
) -> bool {
    if bbb_protocol::block_state_types::vanilla_block_state_property_matches_utf16(
        state, property, raw,
    ) {
        return true;
    }
    let Some((spec, actual_offset)) = registry.block_by_state_id(state) else {
        return false;
    };
    if spec.state_count <= 1
        || !MOD_STATE_OFFSET_PROPERTY
            .encode_utf16()
            .eq(property.iter().copied())
    {
        return false;
    }
    vanilla_block_property_value_rank_utf16(
        &VanillaBlockPropertyKind::Integer {
            min: 0,
            max: i32::from(spec.state_count) - 1,
        },
        raw,
    ) == Some(i32::from(actual_offset))
}

/// The block-entity half of every block-predicate test: `NbtUtils.compareNbt`
/// (NbtUtils.java:65-125) with `partialListMatches` true, which is the call
/// `BlockInput.test` (BlockInput.java:59), `BlockPredicate.test`
/// (BlockPredicateArgument.java:96) and `TagPredicate.test` (`:147`) all make
/// against `BlockEntity.saveWithFullMetadata` (`:59`, `:96`, `:147`).
///
/// `expected` is the compound the argument typed and `actual` the block
/// entity's own record: a key the record does not carry is a mismatch, a key
/// the argument did not name is not, and a list matches when every element it
/// asks for is somewhere in the record's list. An absent `expected` is
/// vanilla's null tag, which every record satisfies.
#[must_use]
pub fn block_entity_data_matches(
    expected: Option<&NbtSummaryValue>,
    actual: Option<&NbtSummaryValue>,
) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    let Some(actual) = actual else {
        return false;
    };
    nbt_matches(expected, actual)
}

/// `compareNbt(expected, actual, true)` (NbtUtils.java:65-125) without the
/// null arms, which [`block_entity_data_matches`] takes.
///
/// `:78-80` is an equality test on the tag's *class*, so two tags of different
/// kinds never match however their text reads; the compound arm (`:82-95`)
/// asks only for the keys `expected` names, and the list arm (`:96-121`) only
/// for the elements. `expected == actual` is the last arm, which is every
/// scalar and every list whose `partialListMatches` is off.
fn nbt_matches(expected: &NbtSummaryValue, actual: &NbtSummaryValue) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            actual.len() >= expected.len()
                && expected.iter().all(|entry| {
                    actual
                        .iter()
                        .find(|held| held.name == entry.name)
                        .is_some_and(|held| nbt_matches(&entry.value, &held.value))
                })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual)) => {
            if expected.is_empty() {
                return actual.is_empty();
            }
            actual.len() >= expected.len()
                && expected
                    .iter()
                    .all(|wanted| actual.iter().any(|held| nbt_matches(wanted, held)))
        }
        _ => expected == actual,
    }
}

fn skip_whitespace(command: &[u16], cursor: &mut usize) {
    *cursor = scan_command_whitespace_utf16(command, *cursor);
}

fn read_quotable_value(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, JavaString), BlockStateParseError> {
    read_command_string_value_utf16(command, cursor).map_err(|failure| {
        let (key, arguments) = match failure.kind {
            CommandSyntaxErrorKind::InvalidEscape => (
                "parsing.quote.escape",
                vec![JavaString::from(vec![command[failure.cursor]])],
            ),
            CommandSyntaxErrorKind::ExpectedQuoteEnd => ("parsing.quote.expected.end", Vec::new()),
            _ => unreachable!("a string reader only refuses escapes and unterminated quotes"),
        };
        BlockStateParseError {
            key,
            arguments,
            cursor: failure.cursor,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_state_types::vanilla_block_state_identity;
    use bbb_protocol::NbtSummaryEntry;

    struct ShippedTags;

    impl BlockTagLookup for ShippedTags {
        fn block_tag_members(&self, tag_id: &str) -> Option<Vec<i32>> {
            crate::fluid::declared_block_type_ids_in_tag(tag_id)
                .map(|members| members.into_iter().collect())
        }
    }

    fn accepted(command: &str) -> BlockStateInput {
        let (end, input) = read_block_state(command, 0)
            .unwrap_or_else(|refusal| panic!("{command}: {}", refusal.key));
        assert_eq!(end, command.len(), "{command}");
        input
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn property_of(state: i32, property: &str) -> String {
        vanilla_block_state_identity(state)
            .and_then(|identity| identity.properties.get(property).cloned())
            .unwrap_or_else(|| panic!("the state carries {property}"))
    }

    /// `setValue` (`:490-499`) accepts what the definition's own
    /// `Property.getValue` answers, and for an integer property that is
    /// `Integer.parseInt` followed by the range check
    /// (`IntegerProperty.java:52-59`), not membership in the value list.
    #[test]
    fn an_integer_property_accepts_every_spelling_parse_int_reads() {
        for (command, property, expected) in [
            // The listed spelling itself, as the control.
            ("oak_sapling[stage=0]", "stage", "0"),
            ("oak_sapling[stage=1]", "stage", "1"),
            ("oak_sapling[stage=+01]", "stage", "1"),
            ("oak_sapling[stage='١']", "stage", "1"),
            ("wheat[age=7]", "age", "7"),
            ("oak_leaves[distance=7]", "distance", "7"),
        ] {
            let input = accepted(command);
            assert_eq!(input.defined_properties, [property], "{command}");
            assert_eq!(property_of(input.state, property), expected, "{command}");
        }

        // A spelled integer beside a literal name still lands in one state.
        let input = accepted("cocoa[age=00,facing=north]");
        assert_eq!(input.defined_properties, ["age", "facing"]);
        assert_eq!(property_of(input.state, "age"), "0");
        assert_eq!(property_of(input.state, "facing"), "north");
    }

    /// `readNbt` (BlockStateParser.java:486-488) is the last thing `parse`
    /// (`:166-169`) does, so the compound is read after whichever leg read the
    /// id -- and, for a block, after its property list.
    #[test]
    fn a_block_state_reads_the_block_entity_compound_after_its_properties() {
        // Both halves of the argument at once (`:160-163` then `:166-169`).
        let input = accepted("chest[facing=north]{Lock:'k'}");
        assert_eq!(input.defined_properties, ["facing"]);
        assert_eq!(property_of(input.state, "facing"), "north");
        assert_eq!(
            input.block_entity_data,
            Some(NbtSummaryValue::Compound(vec![entry(
                "Lock",
                NbtSummaryValue::string("k")
            )]))
        );

        // `:159-164`: with no `[` the compound follows the id, and the reader
        // reads it whatever the block is -- `stone` has no block entity and is
        // not refused for it; `BlockInput.place` is where that shows.
        let input = accepted("chest{lock:{items:\"minecraft:stick\"}}");
        assert!(input.defined_properties.is_empty());
        assert!(matches!(
            input.block_entity_data,
            Some(NbtSummaryValue::Compound(_))
        ));
        let stone = accepted("stone{Lock:'k'}");
        assert_eq!(stone.state, accepted("stone").state);
        assert!(stone.block_entity_data.is_some());

        // The tag leg reads it from the same place (`:150-156`).
        let (end, predicate) =
            read_block_predicate("#minecraft:planks{Lock:'k'}", 0, &ShippedTags).expect("a tag");
        assert_eq!(end, "#minecraft:planks{Lock:'k'}".len());
        let BlockPredicate::Tag {
            block_entity_data, ..
        } = predicate
        else {
            panic!("a `#tag` is a `TagPredicate`");
        };
        assert!(block_entity_data.is_some());

        let command = "😀 chest[facing=north]{'😀':'𐐀'} tail";
        let start = "😀 ".len();
        let (end, block) = read_block_state(command, start).expect("a byte cursor at the block");
        assert_eq!(&command[end..], " tail");
        let units: Vec<_> = command.encode_utf16().collect();
        let (raw_end, raw_block) = read_block_state_utf16(&units, 3).expect("a UTF-16 cursor");
        assert_eq!(raw_block, block);
        assert_eq!(raw_end, command[..end].encode_utf16().count());

        let mut raw: Vec<_> = "ignored #minecraft:planks[".encode_utf16().collect();
        let mut expected = BTreeMap::new();
        for (index, unit) in [0xd800, 0xd801, 0xfffd].into_iter().enumerate() {
            if index != 0 {
                raw.push(u16::from(b','));
            }
            raw.extend([34, unit, 34, 61, 34, unit, 34]);
            expected.insert(JavaString::from(vec![unit]), JavaString::from(vec![unit]));
        }
        raw.extend("]{v:'".encode_utf16());
        raw.extend([0xd800, 0xd801, 0xfffd]);
        raw.extend("'} tail".encode_utf16());
        let (end, predicate) = read_block_predicate_utf16(&raw, 8, &ShippedTags).expect("raw text");
        assert_eq!(&raw[end..], " tail".encode_utf16().collect::<Vec<_>>());
        let BlockPredicate::Tag {
            vague_properties,
            block_entity_data,
            ..
        } = predicate
        else {
            panic!("a tag predicate");
        };
        assert_eq!(vague_properties, expected);
        assert_eq!(
            block_entity_data,
            Some(compound(vec![entry(
                "v",
                NbtSummaryValue::string_from_utf16(vec![0xd800, 0xd801, 0xfffd]),
            )]))
        );

        for (prefix, key, argument) in [
            ("stone[\"", "argument.block.property.unknown", 1),
            ("oak_log[axis=\"", "argument.block.property.invalid", 1),
            ("oak_log[axis=\"\\", "parsing.quote.escape", 0),
        ] {
            let mut raw: Vec<_> = prefix.encode_utf16().collect();
            let cursor = raw.len();
            raw.extend([0xd800, 34, 93]);
            let refusal = read_block_state_utf16(&raw, 0).expect_err("invalid raw property");
            assert_eq!(refusal.key, key);
            assert_eq!(refusal.arguments[argument].units(), &[0xd800]);
            assert_eq!(
                refusal.cursor,
                cursor - usize::from(key != "parsing.quote.escape")
            );
        }

        let text = "#minecraft:planks[\"😀\"=x] trailing";
        let suggestions = block_argument_suggestions(text, Some(&ShippedTags));
        assert_eq!(&text[suggestions.start..], " trailing");
    }

    /// `NbtUtils.compareNbt` (NbtUtils.java:65-125) as every block-predicate
    /// test uses it: the argument's compound against the block entity's own
    /// record, key by key and element by element.
    #[test]
    fn block_entity_data_matches_the_way_compare_nbt_does() {
        let record = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:chest")),
            entry("lock", NbtSummaryValue::string("k")),
            entry(
                "Items",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![entry("Slot", NbtSummaryValue::Byte(0))]),
                    NbtSummaryValue::Compound(vec![entry("Slot", NbtSummaryValue::Byte(1))]),
                ]),
            ),
        ]);

        // The null tag is every record (`:70-72`), and a record against a null
        // tag is none (`:74-76`).
        assert!(block_entity_data_matches(None, Some(&record)));
        assert!(block_entity_data_matches(None, None));
        assert!(!block_entity_data_matches(Some(&record), None));

        // A subset of the keys is enough (`:82-95`), and their order is not
        // read.
        let subset = NbtSummaryValue::Compound(vec![entry("lock", NbtSummaryValue::string("k"))]);
        assert!(block_entity_data_matches(Some(&subset), Some(&record)));
        assert!(block_entity_data_matches(Some(&record), Some(&record)));
        // A key the record does not carry, and a value that differs, both
        // refuse.
        for expected in [
            compound(vec![entry("Lock", NbtSummaryValue::string("k"))]),
            compound(vec![entry("lock", NbtSummaryValue::string("other"))]),
            compound(vec![entry("lock", NbtSummaryValue::Byte(1))]),
        ] {
            assert!(!block_entity_data_matches(Some(&expected), Some(&record)));
        }

        // A list matches when every element it names is in the record's list
        // (`:96-121`), whatever the order, and an empty one only against an
        // empty list (`:98-100`).
        let one_slot = compound(vec![entry(
            "Items",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                "Slot",
                NbtSummaryValue::Byte(1),
            )])]),
        )]);
        assert!(block_entity_data_matches(Some(&one_slot), Some(&record)));
        let missing_slot = compound(vec![entry(
            "Items",
            NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                "Slot",
                NbtSummaryValue::Byte(7),
            )])]),
        )]);
        assert!(!block_entity_data_matches(
            Some(&missing_slot),
            Some(&record)
        ));
        let empty_list = compound(vec![entry("Items", NbtSummaryValue::List(Vec::new()))]);
        assert!(!block_entity_data_matches(Some(&empty_list), Some(&record)));
        assert!(block_entity_data_matches(
            Some(&empty_list),
            Some(&compound(vec![entry(
                "Items",
                NbtSummaryValue::List(Vec::new())
            )]))
        ));
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    /// `TagPredicate.test` (`BlockPredicateArgument.java:126-140`) resolves the
    /// vague value through the candidate state's own `Property.getValue`
    /// (`:132`) and compares that to the state (`:137`), so the integer rule
    /// that decides `setValue` decides the filter too.
    #[test]
    fn a_tag_predicate_value_is_resolved_by_the_candidates_own_property() {
        let matches = |state, property: &str, raw: &str| {
            JavaString::with_utf16(property, |property| {
                JavaString::with_utf16(raw, |raw| {
                    block_predicate_property_matches(state, property, raw)
                })
            })
        };
        let stage_zero = accepted("oak_sapling[stage=0]").state;
        let stage_one = accepted("oak_sapling[stage=1]").state;
        for raw in ["0", "00", "+0", "٠"] {
            assert!(matches(stage_zero, "stage", raw), "stage=0 admits {raw:?}");
            assert!(!matches(stage_one, "stage", raw), "stage=1 refuses {raw:?}");
        }
        for raw in ["1", "+01", "１"] {
            assert!(matches(stage_one, "stage", raw), "stage=1 admits {raw:?}");
        }
        // Outside `stage`'s 0..1, or not an integer `parseInt` reads.
        let raw = "7";
        assert!(
            !matches(stage_zero, "stage", raw),
            "stage=0 refuses {raw:?}"
        );
        // A property the candidate's own block does not declare (`:127-130`).
        assert!(!matches(stage_zero, "axis", "x"));

        // `EnumProperty.getValue` (`EnumProperty.java:48-50`) and
        // `BooleanProperty.getValue` (`BooleanProperty.java:25-31`) answer by
        // exact name whatever the state carries, while their integer-shaped
        // neighbours go through the same rule.
        let log_x = accepted("oak_log[axis=x]").state;
        assert!(matches(log_x, "axis", "x"));
        assert!(!matches(log_x, "axis", "X"));
        let candle = accepted("candle[candles=1,lit=true,waterlogged=false]").state;
        assert!(matches(candle, "lit", "true"));
        assert!(!matches(candle, "lit", "TRUE"));
        assert!(matches(candle, "waterlogged", "false"));
        assert!(matches(candle, "candles", "01"));
        assert!(!matches(candle, "candles", "5"));
    }
}
