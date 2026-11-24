//! The `advancements={..}` selector option as a value
//! (EntitySelectorOptions.java:375-454).
//!
//! The option list this map sits in is walked by [`super::selector_options`],
//! which owns the reader primitives; this module owns the map itself and the
//! predicate it registers.

use bbb_protocol::JavaString;
use bbb_server_datapack::player_advancements::{AdvancementTree, PlayerAdvancements};

use super::selector_options::{OptionReader, SelectorOptionFailure};

/// One `<advancement>=<value>` entry of the map (`:383-423`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdvancementRequirement {
    pub id: String,
    pub expectation: AdvancementExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AdvancementExpectation {
    /// `p -> p.isDone() == value` (`:422`).
    Done(bool),
    /// The criterion map at `:388-419`, whose predicate answers true only when
    /// every named criterion matches.
    Criteria(Vec<(JavaString, bool)>),
}

/// The predicate `:433-449` adds, for one player candidate.
///
/// `:434-435` short-circuits anything that is not a `ServerPlayer`, which the
/// caller answers rather than this function.
pub(super) fn player_matches(
    requirements: &[AdvancementRequirement],
    tree: &AdvancementTree,
    player: &PlayerAdvancements,
) -> bool {
    requirements.iter().all(|requirement| {
        // `serverAdvancements.get(entry.getKey())` (`:441-442`): an id no
        // advancement carries refuses the whole predicate.
        let Some(index) = tree.index_of(&requirement.id) else {
            return false;
        };
        let progress = player.progress(index);
        match &requirement.expectation {
            AdvancementExpectation::Done(expected) => {
                progress.is_some_and(|progress| progress.is_done()) == *expected
            }
            AdvancementExpectation::Criteria(criteria) => {
                criteria.iter().all(|(name, expected)| {
                    // `p.getCriterion(name)` (`:412-413`): a criterion the
                    // advancement does not declare refuses the predicate
                    // whichever value was asked for; an unstarted progress
                    // answers each declared one as not done.
                    tree.node(index)
                        .criteria()
                        .iter()
                        .any(|(declared, _)| declared == name)
                        && progress.is_some_and(|progress| {
                            progress
                                .criterion(name)
                                .is_some_and(|criterion| criterion.is_done())
                        }) == *expected
                })
            }
        }
    })
}

/// `advancements` itself (`:376-431`), read off the option list's cursor.
pub(super) fn read_map(
    reader: &mut OptionReader<'_>,
) -> Result<Vec<AdvancementRequirement>, SelectorOptionFailure> {
    reader.expect('{')?;
    reader.skip_whitespace();
    let mut requirements = Vec::new();
    while reader.peek().is_some_and(|next| next != 125) {
        reader.skip_whitespace();
        let id = reader.read_identifier()?;
        reader.skip_whitespace();
        reader.expect('=')?;
        reader.skip_whitespace();
        let expectation = if reader.peek() == Some(123) {
            reader.skip_whitespace();
            let criteria = read_criteria_map(reader)?;
            reader.skip_whitespace();
            AdvancementExpectation::Criteria(criteria)
        } else {
            AdvancementExpectation::Done(reader.read_boolean()?)
        };
        // `expected.put` (`:410`, `:422`) over a `Maps.newHashMap`, so a
        // repeated id keeps the last value rather than both.
        requirements.retain(|existing: &AdvancementRequirement| existing.id != id);
        requirements.push(AdvancementRequirement { id, expectation });
        reader.skip_whitespace();
        if reader.peek() == Some(44) {
            reader.bump();
        }
    }
    reader.expect('}')?;
    Ok(requirements)
}

/// The nested criterion map (`:388-408`), whose keys are unquoted.
fn read_criteria_map(
    reader: &mut OptionReader<'_>,
) -> Result<Vec<(JavaString, bool)>, SelectorOptionFailure> {
    reader.expect('{')?;
    reader.skip_whitespace();
    let mut criteria: Vec<(JavaString, bool)> = Vec::new();
    while reader.peek().is_some_and(|next| next != 125) {
        reader.skip_whitespace();
        let name: JavaString = reader.read_unquoted_java_string();
        reader.skip_whitespace();
        reader.expect('=')?;
        reader.skip_whitespace();
        let expected = reader.read_boolean()?;
        criteria.retain(|(existing, _)| existing != &name);
        criteria.push((name, expected));
        reader.skip_whitespace();
        if reader.peek() == Some(44) {
            reader.bump();
        }
    }
    reader.skip_whitespace();
    reader.expect('}')?;
    Ok(criteria)
}
