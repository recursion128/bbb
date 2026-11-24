//! Sculk charge cursor saved values and NBT codec.
use crate::block_queries::{direction_from_serialized_name, direction_serialized_name};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
/// `SculkSpreader.MAX_CHARGE` (`SculkSpreader.java:38`).
pub const MAX_CHARGE: i32 = 1000;
/// `SculkSpreader.MAX_CURSORS` (`SculkSpreader.java:40`).
pub const MAX_CURSORS: usize = 32;
/// `Codec.intRange(0, 1)` (`SculkSpreader.java:205`).
const MAX_DECAY_DELAY: i32 = 1;
fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
/// One `SculkSpreader.ChargeCursor.CODEC` entry
/// (`SculkSpreader.java:201-210`) as the codec carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SculkCursorFacts {
    /// `pos`, required and `BlockPos.CODEC`-shaped.
    pub pos: BlockPos,
    /// `charge`, `intRange(0, 1000)` defaulting to 0 (`:204`).
    pub charge: i32,
    /// `decay_delay`, `intRange(0, 1)` defaulting to 1 (`:205`).
    pub decay_delay: i32,
    /// `update_delay`, `intRange(0, MAX)` defaulting to 0 (`:206`).
    pub update_delay: i32,
    /// `facings` (`:207`), a direction set in `Direction` ordinal order; `None`
    /// is the field's own absence, which the codec writes for a null set.
    pub facings: Option<Vec<Direction>>,
}

/// `SculkSpreader.ChargeCursor.CODEC`'s encode side
/// (`SculkSpreader.java:201-210`).
#[must_use]
pub fn cursor_nbt(facts: &SculkCursorFacts) -> NbtSummaryValue {
    let mut entries = vec![
        entry(
            "pos",
            NbtSummaryValue::IntArray(vec![facts.pos.x, facts.pos.y, facts.pos.z]),
        ),
        entry("charge", NbtSummaryValue::Int(facts.charge)),
        entry("decay_delay", NbtSummaryValue::Int(facts.decay_delay)),
        entry("update_delay", NbtSummaryValue::Int(facts.update_delay)),
    ];
    if let Some(facings) = &facts.facings {
        entries.push(entry(
            "facings",
            NbtSummaryValue::List(
                facings
                    .iter()
                    .map(|direction| NbtSummaryValue::string(direction_serialized_name(*direction)))
                    .collect(),
            ),
        ));
    }
    NbtSummaryValue::Compound(entries)
}

/// `SculkSpreader.ChargeCursor.CODEC`'s decode side
/// (`SculkSpreader.java:201-210`), `None` for an entry the codec rejects.
#[must_use]
pub fn parse_cursor_nbt(value: &NbtSummaryValue) -> Option<SculkCursorFacts> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let pos = match codec_field(entries, "pos")? {
        NbtSummaryValue::IntArray(values) if values.len() == 3 => BlockPos {
            x: values[0],
            y: values[1],
            z: values[2],
        },
        _ => return None,
    };
    let charge = codec_int_field(entries, "charge", 0)?;
    if !(0..=MAX_CHARGE).contains(&charge) {
        return None;
    }
    let decay_delay = codec_int_field(entries, "decay_delay", 1)?;
    if !(0..=MAX_DECAY_DELAY).contains(&decay_delay) {
        return None;
    }
    let update_delay = codec_int_field(entries, "update_delay", 0)?;
    if update_delay < 0 {
        return None;
    }
    Some(SculkCursorFacts {
        pos,
        charge,
        decay_delay,
        update_delay,
        // `lenientOptionalFieldOf` (`:207`): a value the direction-set codec
        // cannot read is the absent field, not a failed entry.
        facings: parse_facings(codec_field(entries, "facings")),
    })
}

/// `SculkSpreader.load`'s `sizeLimitedListOf(32)` (`SculkSpreader.java:110`):
/// a list longer than the bound, or one entry the codec rejects, fails the
/// whole read, which `load`'s `orElse(List.of())` then answers empty.
#[must_use]
pub fn parse_cursors_nbt(values: &[NbtSummaryValue]) -> Option<Vec<SculkCursorFacts>> {
    if values.len() > MAX_CURSORS {
        return None;
    }
    values.iter().map(parse_cursor_nbt).collect()
}

/// `DIRECTION_SET` (`SculkSpreader.java:200`): `Direction.CODEC.listOf()` read
/// into an `EnumSet`, so duplicates collapse and the set's order is ordinal.
fn parse_facings(value: Option<&NbtSummaryValue>) -> Option<Vec<Direction>> {
    let Some(NbtSummaryValue::List(values)) = value else {
        return None;
    };
    let mut facings = Vec::new();
    for value in values {
        let NbtSummaryValue::String(name) = value else {
            return None;
        };
        let direction = name.with_utf8(direction_from_serialized_name).flatten()?;
        if !facings.contains(&direction) {
            facings.push(direction);
        }
    }
    facings.sort_by_key(|direction| direction.id());
    Some(facings)
}

fn codec_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn codec_int_field(entries: &[NbtSummaryEntry], name: &str, default: i32) -> Option<i32> {
    match codec_field(entries, name) {
        None => Some(default),
        Some(NbtSummaryValue::Int(value)) => Some(*value),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// `SculkSpreader.ChargeCursor.CODEC` (`SculkSpreader.java:201-210`): the
    /// field order, the defaults, the `intRange` bounds, the direction-set
    /// shape and `load`'s `sizeLimitedListOf(32)`.
    #[test]
    fn the_cursor_codec_round_trips_the_vanilla_shape() {
        let facts = SculkCursorFacts {
            pos: BlockPos { x: 1, y: 2, z: 3 },
            charge: 7,
            decay_delay: 0,
            update_delay: 5,
            facings: Some(vec![Direction::Down, Direction::East]),
        };
        let value = cursor_nbt(&facts);
        assert_eq!(
            value,
            NbtSummaryValue::Compound(vec![
                entry("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
                entry("charge", NbtSummaryValue::Int(7)),
                entry("decay_delay", NbtSummaryValue::Int(0)),
                entry("update_delay", NbtSummaryValue::Int(5)),
                entry(
                    "facings",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::string("down"),
                        NbtSummaryValue::string("east"),
                    ])
                ),
            ])
        );
        assert_eq!(parse_cursor_nbt(&value), Some(facts.clone()));
        // `facings` is the one field the codec omits while null.
        let without = SculkCursorFacts {
            facings: None,
            ..facts.clone()
        };
        let value = cursor_nbt(&without);
        let NbtSummaryValue::Compound(entries) = &value else {
            panic!("a cursor is a compound");
        };
        assert_eq!(entries.len(), 4);
        assert_eq!(parse_cursor_nbt(&value), Some(without));

        // `availableFaces` answers an empty set for a cell that is not a
        // multiface (`MultifaceBlock.java:70-73`), which is the set a cursor
        // standing on plain sculk carries. The codec writes it as a present
        // empty list, and the two readings stay apart downstream (`:174-179`,
        // `SculkBehaviour.java:18-24`), so the shape has to keep them apart.
        let emptied = SculkCursorFacts {
            facings: Some(Vec::new()),
            ..facts
        };
        let value = cursor_nbt(&emptied);
        let NbtSummaryValue::Compound(entries) = &value else {
            panic!("a cursor is a compound");
        };
        assert_eq!(
            entries
                .iter()
                .find(|entry| entry.name == "facings")
                .expect("the field is written")
                .value,
            NbtSummaryValue::List(Vec::new())
        );
        assert_eq!(parse_cursor_nbt(&value), Some(emptied));

        // `charge`, `decay_delay` and `update_delay` all have `orElse`
        // defaults (`:204-206`).
        assert_eq!(
            parse_cursor_nbt(&NbtSummaryValue::Compound(vec![entry(
                "pos",
                NbtSummaryValue::IntArray(vec![1, 2, 3])
            )])),
            Some(SculkCursorFacts {
                pos: BlockPos { x: 1, y: 2, z: 3 },
                charge: 0,
                decay_delay: 1,
                update_delay: 0,
                facings: None,
            })
        );
    }
}
