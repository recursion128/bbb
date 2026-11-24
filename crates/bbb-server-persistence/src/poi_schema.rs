//! Java 26.1 point-of-interest region entries.

use std::collections::HashSet;

use bbb_facts::poi_types::vanilla_poi_type_for_resource_id;
use bbb_protocol::{
    nbt_summary::entry, normalize_resource_location, NbtSummaryEntry, NbtSummaryValue,
};

use super::schema_support::{
    compound_or_empty, field, int_value, invalid_schema, require_java_26_1_root,
};
use bbb_server_types::save::{NamedCompound, SaveResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiChunkData {
    raw: NamedCompound,
    chunk_x: i32,
    chunk_z: i32,
    sections: Vec<Java26PoiSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiSection {
    raw: Vec<NbtSummaryEntry>,
    y: i32,
    valid: bool,
    records: Vec<Java26PoiRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiRecord {
    raw: Vec<NbtSummaryEntry>,
    pos: [i32; 3],
    type_id: String,
    free_tickets: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiRecordEdit {
    pub pos: [i32; 3],
    pub type_id: String,
    pub free_tickets: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26PoiSectionEdit {
    pub y: i32,
    pub valid: bool,
    pub records: Vec<Java26PoiRecordEdit>,
}

impl Java26PoiChunkData {
    pub fn parse(
        raw: NamedCompound,
        expected_x: i32,
        expected_z: i32,
        min_section_y: i32,
        section_count: usize,
    ) -> SaveResult<Self> {
        let entries = require_java_26_1_root(&raw, "poi chunk")?;
        let max_section_y = i64::from(min_section_y) + section_count as i64;
        let mut sections: Vec<Java26PoiSection> = Vec::new();
        for section_entry in compound_or_empty(entries, "Sections") {
            let Some(section_y) = section_entry
                .name
                .try_to_string()
                .ok()
                .and_then(|raw| canonical_section_y(&raw))
            else {
                continue;
            };
            if i64::from(section_y) < i64::from(min_section_y)
                || i64::from(section_y) >= max_section_y
                || sections.iter().any(|section| section.y == section_y)
            {
                continue;
            }
            let NbtSummaryValue::Compound(compound) = &section_entry.value else {
                continue;
            };
            // PoiSection.Packed.CODEC (`PoiSection.java:150-156`).
            let Some(NbtSummaryValue::List(record_values)) = field(compound, "Records") else {
                continue;
            };
            let mut records: Vec<Java26PoiRecord> = Vec::with_capacity(record_values.len());
            for value in record_values {
                let NbtSummaryValue::Compound(record) = value else {
                    continue;
                };
                let Some(record) = parse_record(record, expected_x, expected_z, section_y) else {
                    continue;
                };
                // PoiSection.add (`PoiSection.java:64-80`): a second record at a
                // position replaces the first unless it has the same type.
                match records
                    .iter_mut()
                    .find(|previous| previous.pos == record.pos)
                {
                    Some(previous) if previous.type_id != record.type_id => *previous = record,
                    Some(_) => {}
                    None => records.push(record),
                }
            }
            sections.push(Java26PoiSection {
                raw: compound.to_vec(),
                y: section_y,
                valid: field(compound, "Valid")
                    .and_then(int_value)
                    .is_some_and(|value| value as i8 != 0),
                records,
            });
        }
        sections.sort_by_key(|section| section.y);

        Ok(Self {
            raw,
            chunk_x: expected_x,
            chunk_z: expected_z,
            sections,
        })
    }

    pub fn raw(&self) -> &NamedCompound {
        &self.raw
    }
    pub fn into_raw(self) -> NamedCompound {
        self.raw
    }
    pub fn chunk_x(&self) -> i32 {
        self.chunk_x
    }
    pub fn chunk_z(&self) -> i32 {
        self.chunk_z
    }
    pub fn sections(&self) -> &[Java26PoiSection] {
        &self.sections
    }
    pub fn records(&self) -> impl Iterator<Item = &Java26PoiRecord> {
        self.sections
            .iter()
            .flat_map(|section| section.records.iter())
    }

    pub fn empty(
        chunk_x: i32,
        chunk_z: i32,
        min_section_y: i32,
        section_count: usize,
    ) -> SaveResult<Self> {
        Self::parse(
            NamedCompound::new(
                "",
                vec![
                    entry(
                        "DataVersion",
                        NbtSummaryValue::Int(bbb_server_types::save::JAVA_26_1_DATA_VERSION),
                    ),
                    entry("Sections", NbtSummaryValue::Compound(Vec::new())),
                ],
            ),
            chunk_x,
            chunk_z,
            min_section_y,
            section_count,
        )
    }

    pub fn raw_with_section_edits(
        &self,
        edits: &[Java26PoiSectionEdit],
        min_section_y: i32,
        section_count: usize,
    ) -> SaveResult<NamedCompound> {
        let mut root = self.raw.entries().to_vec();
        let mut sections = compound_or_empty(&root, "Sections").to_vec();
        let mut seen = HashSet::new();
        for edit in edits {
            if !seen.insert(edit.y)
                || i64::from(edit.y) < i64::from(min_section_y)
                || i64::from(edit.y) >= i64::from(min_section_y) + section_count as i64
            {
                return Err(invalid_schema(
                    "poi section edit",
                    "duplicate or out-of-height section",
                ));
            }
            let old = self.sections.iter().find(|section| section.y == edit.y);
            let mut fields = old.map_or_else(Vec::new, |section| section.raw.clone());
            let records = edit
                .records
                .iter()
                .map(|record| {
                    let mut fields = old
                        .and_then(|section| {
                            section.records.iter().find(|previous| {
                                previous.pos == record.pos && previous.type_id == record.type_id
                            })
                        })
                        .map_or_else(Vec::new, |previous| previous.raw.clone());
                    replace(
                        &mut fields,
                        "pos",
                        NbtSummaryValue::IntArray(record.pos.to_vec()),
                    );
                    replace(
                        &mut fields,
                        "type",
                        NbtSummaryValue::string(record.type_id.clone()),
                    );
                    replace(
                        &mut fields,
                        "free_tickets",
                        NbtSummaryValue::Int(record.free_tickets),
                    );
                    NbtSummaryValue::Compound(fields)
                })
                .collect();
            replace(
                &mut fields,
                "Valid",
                NbtSummaryValue::Byte(i8::from(edit.valid)),
            );
            replace(&mut fields, "Records", NbtSummaryValue::List(records));
            replace(
                &mut sections,
                &edit.y.to_string(),
                NbtSummaryValue::Compound(fields),
            );
        }
        replace(&mut root, "Sections", NbtSummaryValue::Compound(sections));
        let raw = NamedCompound::new(self.raw.root_name().clone(), root);
        require_java_26_1_root(&raw, "poi chunk")?;
        Ok(raw)
    }
}

fn replace(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(entry) = entries.iter_mut().find(|entry| entry.name == name) {
        entry.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

impl Java26PoiSection {
    pub fn raw(&self) -> &[NbtSummaryEntry] {
        &self.raw
    }
    pub fn y(&self) -> i32 {
        self.y
    }
    pub fn valid(&self) -> bool {
        self.valid
    }
    pub fn records(&self) -> &[Java26PoiRecord] {
        &self.records
    }
}

impl Java26PoiRecord {
    pub fn raw(&self) -> &[NbtSummaryEntry] {
        &self.raw
    }
    pub fn pos(&self) -> [i32; 3] {
        self.pos
    }
    pub fn type_id(&self) -> &str {
        &self.type_id
    }
    pub fn free_tickets(&self) -> i32 {
        self.free_tickets
    }
}

/// A record the codec cannot decode is dropped; one outside its section is
/// keyed by its section-relative position in Vanilla, so it is dropped here.
fn parse_record(
    entries: &[NbtSummaryEntry],
    expected_x: i32,
    expected_z: i32,
    section_y: i32,
) -> Option<Java26PoiRecord> {
    let Some(NbtSummaryValue::IntArray(pos)) = field(entries, "pos") else {
        return None;
    };
    let pos: [i32; 3] = pos.as_slice().try_into().ok()?;
    if pos[0].div_euclid(16) != expected_x
        || pos[2].div_euclid(16) != expected_z
        || pos[1].div_euclid(16) != section_y
    {
        return None;
    }
    let Some(NbtSummaryValue::String(type_id)) = field(entries, "type") else {
        return None;
    };
    let type_id = type_id
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    vanilla_poi_type_for_resource_id(&type_id)?;
    Some(Java26PoiRecord {
        raw: entries.to_vec(),
        pos,
        type_id,
        // PoiRecord.java:89-97: decoding failures default to zero.
        free_tickets: field(entries, "free_tickets")
            .and_then(int_value)
            .unwrap_or(0),
    })
}

fn canonical_section_y(name: &str) -> Option<i32> {
    let value = name.parse::<i32>().ok()?;
    (value.to_string() == name).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_server_types::save::JAVA_26_1_DATA_VERSION;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn record(pos: [i32; 3]) -> NbtSummaryValue {
        record_with_free_tickets(pos, 0)
    }

    fn record_with_free_tickets(pos: [i32; 3], free_tickets: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("pos", NbtSummaryValue::IntArray(pos.to_vec())),
            entry("type", NbtSummaryValue::string("minecraft:home")),
            entry("free_tickets", NbtSummaryValue::Int(free_tickets)),
            entry("future_record", NbtSummaryValue::Long(11)),
        ])
    }

    fn document(section_name: &str, section: Vec<NbtSummaryEntry>) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry(
                    "Sections",
                    NbtSummaryValue::Compound(vec![entry(
                        section_name,
                        NbtSummaryValue::Compound(section),
                    )]),
                ),
                entry("future_root", NbtSummaryValue::string("kept")),
            ],
        )
    }

    #[test]
    fn projects_sorts_sections_and_preserves_unknown_payload() {
        let raw = document(
            "-4",
            vec![
                entry("Records", NbtSummaryValue::List(vec![record([-1, -64, 0])])),
                entry("future_section", NbtSummaryValue::Int(4)),
            ],
        );
        let parsed = Java26PoiChunkData::parse(raw.clone(), -1, 0, -4, 24).unwrap();
        assert_eq!(parsed.chunk_x(), -1);
        assert_eq!(parsed.chunk_z(), 0);
        assert_eq!(parsed.sections()[0].y(), -4);
        assert!(!parsed.sections()[0].valid());
        let projected = parsed.records().next().unwrap();
        assert_eq!(projected.pos(), [-1, -64, 0]);
        assert_eq!(projected.type_id(), "minecraft:home");
        assert_eq!(projected.free_tickets(), 0);
        assert_eq!(parsed.into_raw(), raw);
    }

    #[test]
    fn accepts_explicit_valid_and_free_tickets() {
        let mut value = match record([1, 4, 1]) {
            NbtSummaryValue::Compound(v) => v,
            _ => unreachable!(),
        };
        value[1].value = NbtSummaryValue::string("minecraft:meeting");
        value
            .iter_mut()
            .find(|entry| entry.name == "free_tickets")
            .expect("record fixture has free_tickets")
            .value = NbtSummaryValue::Int(2);
        let parsed = Java26PoiChunkData::parse(
            document(
                "0",
                vec![
                    entry("Valid", NbtSummaryValue::Byte(1)),
                    entry(
                        "Records",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(value)]),
                    ),
                ],
            ),
            0,
            0,
            -4,
            24,
        )
        .unwrap();
        assert!(parsed.sections()[0].valid());
        assert_eq!(parsed.records().next().unwrap().free_tickets(), 2);
    }

    #[test]
    fn preserves_but_does_not_project_noncanonical_or_out_of_range_sections() {
        let raw = NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry(
                    "Sections",
                    NbtSummaryValue::Compound(vec![
                        entry("+0", NbtSummaryValue::Int(1)),
                        entry("00", NbtSummaryValue::string("ignored")),
                        entry("-0", NbtSummaryValue::Long(2)),
                        entry("24", NbtSummaryValue::Byte(3)),
                        entry("not-y", NbtSummaryValue::List(Vec::new())),
                        entry(
                            "0",
                            NbtSummaryValue::Compound(vec![entry(
                                "Records",
                                NbtSummaryValue::List(Vec::new()),
                            )]),
                        ),
                    ]),
                ),
            ],
        );
        let parsed = Java26PoiChunkData::parse(raw.clone(), 0, 0, -4, 24).unwrap();
        assert_eq!(parsed.sections().len(), 1);
        assert_eq!(parsed.sections()[0].y(), 0);
        assert_eq!(parsed.into_raw(), raw);
    }
    #[test]
    fn poi_section_edits_preserve_unknown_identity_and_empty_owned_sections() {
        let raw = document(
            "0",
            vec![
                entry("future_section", NbtSummaryValue::Int(7)),
                entry("Records", NbtSummaryValue::List(vec![record([1, 4, 1])])),
            ],
        );
        let parsed = Java26PoiChunkData::parse(raw, 0, 0, -4, 24).unwrap();
        let mut edit = Java26PoiSectionEdit {
            y: 0,
            valid: true,
            records: vec![Java26PoiRecordEdit {
                pos: [1, 4, 1],
                type_id: "minecraft:home".into(),
                free_tickets: 1,
            }],
        };
        let updated = Java26PoiChunkData::parse(
            parsed
                .raw_with_section_edits(&[edit.clone()], -4, 24)
                .unwrap(),
            0,
            0,
            -4,
            24,
        )
        .unwrap();
        assert!(updated
            .records()
            .next()
            .unwrap()
            .raw()
            .iter()
            .any(|entry| entry.name == "future_record"));
        assert!(updated
            .raw()
            .entries()
            .iter()
            .any(|entry| entry.name == "future_root"));
        assert!(updated.sections()[0]
            .raw()
            .iter()
            .any(|entry| entry.name == "future_section"));
        edit.records[0].type_id = "minecraft:meeting".into();
        let replaced = Java26PoiChunkData::parse(
            updated
                .raw_with_section_edits(&[edit.clone()], -4, 24)
                .unwrap(),
            0,
            0,
            -4,
            24,
        )
        .unwrap();
        assert!(!replaced
            .records()
            .next()
            .unwrap()
            .raw()
            .iter()
            .any(|entry| entry.name == "future_record"));
        edit.records.clear();
        let empty = Java26PoiChunkData::parse(
            replaced
                .raw_with_section_edits(&[edit.clone()], -4, 24)
                .unwrap(),
            0,
            0,
            -4,
            24,
        )
        .unwrap();
        assert_eq!(empty.sections().len(), 1);
        assert!(empty.sections()[0].valid());
        assert_eq!(empty.records().count(), 0);
        assert!(empty
            .raw_with_section_edits(&[edit.clone(), edit], -4, 24)
            .is_err());
    }
}
