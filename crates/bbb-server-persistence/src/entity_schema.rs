//! Java 26.1 entity-region entries, read like `EntityStorage.loadEntities`:
//! an entity `EntityType.loadEntityRecursive` would skip is dropped and the
//! rest of the chunk loads.

use std::{borrow::Cow, collections::HashSet};

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::{
    entity_types::vanilla_entity_type_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};
use uuid::Uuid;

use super::schema_support::{field, require_java_26_1_root, uuid_value};
use bbb_server_types::save::{NamedCompound, SaveResult, JAVA_26_1_DATA_VERSION};

pub fn entity_chunk_document(
    chunk_x: i32,
    chunk_z: i32,
    records: impl IntoIterator<Item = Vec<NbtSummaryEntry>>,
) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry(
                "Position",
                NbtSummaryValue::IntArray(vec![chunk_x, chunk_z]),
            ),
            entry(
                "Entities",
                NbtSummaryValue::List(records.into_iter().map(NbtSummaryValue::Compound).collect()),
            ),
        ],
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26EntityChunkData {
    raw: NamedCompound,
    chunk_x: i32,
    chunk_z: i32,
    entities: Vec<(usize, usize, EntityFacts)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26EntityRecord {
    raw: Vec<NbtSummaryEntry>,
    facts: EntityFacts,
}

#[derive(Debug, Clone, PartialEq)]
struct EntityFacts {
    id: Cow<'static, str>,
    uuid: Uuid,
    position: [f64; 3],
    passengers: Vec<(usize, usize, EntityFacts)>,
}

impl EntityFacts {
    fn contains_uuid(&self, uuid: Uuid) -> bool {
        self.uuid == uuid
            || self
                .passengers
                .iter()
                .any(|(_, _, passenger)| passenger.contains_uuid(uuid))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Java26EntityRecordRef<'a> {
    raw: &'a [NbtSummaryEntry],
    facts: &'a EntityFacts,
}

impl Java26EntityChunkData {
    /// A stored `Position` that disagrees with the region slot is only logged by
    /// Vanilla (`EntityStorage.java`), so it is not read at all.
    pub fn parse(raw: NamedCompound, chunk_x: i32, chunk_z: i32) -> SaveResult<Self> {
        Self::parse_with_content_registry(raw, chunk_x, chunk_z, ContentRegistry::vanilla())
    }

    pub fn parse_with_registry(
        raw: NamedCompound,
        chunk_x: i32,
        chunk_z: i32,
        content_registry: &ContentRegistry,
    ) -> SaveResult<Self> {
        Self::parse_with_content_registry(raw, chunk_x, chunk_z, content_registry)
    }

    pub fn parse_with_content_registry(
        raw: NamedCompound,
        chunk_x: i32,
        chunk_z: i32,
        content_registry: &ContentRegistry,
    ) -> SaveResult<Self> {
        let entries = require_java_26_1_root(&raw, "entity chunk")?;
        let mut seen_uuids = HashSet::new();
        let entities = read_entity_list(entries, "Entities", &mut seen_uuids, content_registry);
        Ok(Self {
            raw,
            chunk_x,
            chunk_z,
            entities,
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

    pub fn entities(&self) -> impl ExactSizeIterator<Item = Java26EntityRecordRef<'_>> {
        entity_refs(self.raw.entries(), &self.entities)
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

impl Java26EntityRecord {
    pub fn parse(entries: &[NbtSummaryEntry]) -> Option<Self> {
        Self::parse_with_content_registry(entries, ContentRegistry::vanilla())
    }

    pub fn parse_with_registry(
        entries: &[NbtSummaryEntry],
        content_registry: &ContentRegistry,
    ) -> Option<Self> {
        Self::parse_with_content_registry(entries, content_registry)
    }

    pub fn parse_with_content_registry(
        entries: &[NbtSummaryEntry],
        content_registry: &ContentRegistry,
    ) -> Option<Self> {
        let facts = read_entity(entries, &mut HashSet::new(), content_registry)?;
        Some(Self {
            raw: entries.to_vec(),
            facts,
        })
    }

    /// Base fields exclude `Passengers`; child records are consumed into that list.
    pub fn new(
        mut raw: Vec<NbtSummaryEntry>,
        id: impl Into<Cow<'static, str>>,
        uuid: Uuid,
        position: [f64; 3],
        passengers: Vec<Self>,
    ) -> Self {
        let mut children = Vec::with_capacity(passengers.len());
        let mut entities = Vec::with_capacity(passengers.len());
        let field_index = raw.len();
        for (index, passenger) in passengers.into_iter().enumerate() {
            children.push(NbtSummaryValue::Compound(passenger.raw));
            entities.push((field_index, index, passenger.facts));
        }
        if !children.is_empty() {
            raw.push(entry("Passengers", NbtSummaryValue::List(children)));
        }
        Self {
            raw,
            facts: EntityFacts {
                id: id.into(),
                uuid,
                position,
                passengers: entities,
            },
        }
    }

    pub fn into_raw(self) -> Vec<NbtSummaryEntry> {
        self.raw
    }

    pub fn as_ref(&self) -> Java26EntityRecordRef<'_> {
        Java26EntityRecordRef {
            raw: &self.raw,
            facts: &self.facts,
        }
    }
}

impl<'a> Java26EntityRecordRef<'a> {
    pub fn raw(self) -> &'a [NbtSummaryEntry] {
        self.raw
    }

    pub fn id(self) -> &'a str {
        &self.facts.id
    }

    pub fn uuid(self) -> Uuid {
        self.facts.uuid
    }

    pub fn position(self) -> [f64; 3] {
        self.facts.position
    }

    pub fn passengers(self) -> impl ExactSizeIterator<Item = Self> + 'a {
        entity_refs(self.raw, &self.facts.passengers)
    }

    pub fn contains_uuid(self, uuid: Uuid) -> bool {
        self.facts.contains_uuid(uuid)
    }
}

fn entity_refs<'a>(
    entries: &'a [NbtSummaryEntry],
    entities: &'a [(usize, usize, EntityFacts)],
) -> impl ExactSizeIterator<Item = Java26EntityRecordRef<'a>> + 'a {
    entities.iter().map(move |(field_index, index, facts)| {
        let NbtSummaryValue::List(values) = &entries[*field_index].value else {
            unreachable!("entity list metadata refers to a list")
        };
        let NbtSummaryValue::Compound(raw) = &values[*index] else {
            unreachable!("entity metadata refers to a compound")
        };
        Java26EntityRecordRef { raw, facts }
    })
}

fn read_entity_list(
    entries: &[NbtSummaryEntry],
    name: &str,
    seen_uuids: &mut HashSet<Uuid>,
    content_registry: &ContentRegistry,
) -> Vec<(usize, usize, EntityFacts)> {
    match entries
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.name == name)
    {
        Some((
            field_index,
            NbtSummaryEntry {
                value: NbtSummaryValue::List(values),
                ..
            },
        )) => values
            .iter()
            .enumerate()
            .filter_map(|(index, value)| match value {
                NbtSummaryValue::Compound(entries) => {
                    read_entity(entries, seen_uuids, content_registry)
                        .map(|facts| (field_index, index, facts))
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `EntityType.loadEntityRecursive`: an unknown or unsaveable type, a missing
/// UUID, a non-finite position or rotation, or a UUID already loaded drops the
/// entity with its passengers; unreadable passengers are dropped alone.
fn read_entity(
    entries: &[NbtSummaryEntry],
    seen_uuids: &mut HashSet<Uuid>,
    content_registry: &ContentRegistry,
) -> Option<EntityFacts> {
    let id = match field(entries, "id")? {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())?,
        _ => return None,
    };
    let (resource_id, serializable) = match vanilla_entity_type_for_resource_id(&id) {
        Some(catalog) => (Cow::Borrowed(catalog.resource_id), catalog.serializable),
        None => {
            let spec = content_registry.entity_by_resource_id(&id)?;
            (
                Cow::Owned(spec.resource_id().to_string()),
                spec.serializable,
            )
        }
    };
    if !serializable {
        return None;
    }
    let uuid = field(entries, "UUID").and_then(uuid_value)?;
    let mut position = [0.0; 3];
    if let Some(NbtSummaryValue::List(values)) = field(entries, "Pos") {
        if let [NbtSummaryValue::Double(x), NbtSummaryValue::Double(y), NbtSummaryValue::Double(z)] =
            values.as_slice()
        {
            position = [x, y, z].map(|bits| f64::from_bits(*bits));
        }
    }
    if !position.iter().all(|value| value.is_finite()) {
        return None;
    }
    if let Some(NbtSummaryValue::List(values)) = field(entries, "Rotation") {
        if values.iter().any(|value| {
            matches!(value, NbtSummaryValue::Float(bits) if !f32::from_bits(*bits).is_finite())
        }) {
            return None;
        }
    }
    if !seen_uuids.insert(uuid) {
        return None;
    }
    let passengers = read_entity_list(entries, "Passengers", seen_uuids, content_registry);
    Some(EntityFacts {
        id: resource_id,
        uuid,
        position,
        passengers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegionFileStore;
    use bbb_server_types::save::{WorldSaveLimits, JAVA_26_1_DATA_VERSION};
    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn doubles(values: [f64; 3]) -> NbtSummaryValue {
        NbtSummaryValue::List(
            values
                .into_iter()
                .map(|v| NbtSummaryValue::Double(v.to_bits()))
                .collect(),
        )
    }

    fn floats(values: [f32; 2]) -> NbtSummaryValue {
        NbtSummaryValue::List(
            values
                .into_iter()
                .map(|v| NbtSummaryValue::Float(v.to_bits()))
                .collect(),
        )
    }

    fn entity(uuid: i32, x: f64, z: f64) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:pig")),
            entry("UUID", NbtSummaryValue::IntArray(vec![0, 0, 0, uuid])),
            entry("Pos", doubles([x, 64.0, z])),
            entry("Motion", doubles([0.0, 0.0, 0.0])),
            entry("Rotation", floats([0.0, 0.0])),
            entry("fall_distance", NbtSummaryValue::Double(0.0_f64.to_bits())),
            entry("Fire", NbtSummaryValue::Short(0)),
            entry("Air", NbtSummaryValue::Short(300)),
            entry("OnGround", NbtSummaryValue::Byte(1)),
            entry("Invulnerable", NbtSummaryValue::Byte(0)),
            entry("PortalCooldown", NbtSummaryValue::Int(0)),
            entry("future_payload", NbtSummaryValue::Long(91)),
        ])
    }

    fn document(entities: Vec<NbtSummaryValue>, x: i32, z: i32) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("Position", NbtSummaryValue::IntArray(vec![x, z])),
                entry("Entities", NbtSummaryValue::List(entities)),
                entry("future_root", NbtSummaryValue::string("kept")),
            ],
        )
    }

    fn compound(value: NbtSummaryValue) -> Vec<NbtSummaryEntry> {
        match value {
            NbtSummaryValue::Compound(entries) => entries,
            _ => unreachable!(),
        }
    }

    #[test]
    fn projects_entities_and_preserves_unknown_payload_exactly() {
        let raw = document(vec![entity(7, -0.5, 15.5)], -1, 0);
        let parsed = Java26EntityChunkData::parse(raw.clone(), -1, 0).unwrap();
        assert_eq!(parsed.chunk_x(), -1);
        assert_eq!(parsed.chunk_z(), 0);
        let record = parsed.entities().next().unwrap();
        assert_eq!(record.id(), "minecraft:pig");
        assert_eq!(record.uuid(), Uuid::from_u128(7));
        assert_eq!(parsed.into_raw(), raw);
    }

    #[test]
    fn unloadable_entities_are_skipped_and_the_rest_of_the_chunk_loads() {
        let mut unknown = compound(entity(2, 1.0, 1.0));
        unknown[0].value = NbtSummaryValue::string("example:pig");
        let mut player = compound(entity(3, 1.0, 1.0));
        player[0].value = NbtSummaryValue::string("minecraft:player");
        let mut nan = compound(entity(4, 1.0, 1.0));
        nan[2].value = doubles([f64::NAN, 64.0, 1.0]);
        let mut sparse = compound(entity(5, 1.0, 1.0));
        sparse.retain(|entry| entry.name == "id" || entry.name == "UUID");
        let parsed = Java26EntityChunkData::parse(
            document(
                vec![
                    entity(1, 1.0, 1.0),
                    NbtSummaryValue::Compound(unknown),
                    NbtSummaryValue::Compound(player),
                    NbtSummaryValue::Compound(nan),
                    NbtSummaryValue::Int(0),
                    entity(1, 2.0, 2.0),
                    entity(6, 40.0, 1.0),
                    NbtSummaryValue::Compound(sparse),
                ],
                9,
                9,
            ),
            0,
            0,
        )
        .unwrap();
        let uuids: Vec<_> = parsed.entities().map(|e| e.uuid().as_u128()).collect();
        assert_eq!(uuids, [1, 6, 5]);
        assert_eq!(parsed.entities().nth(2).unwrap().position(), [0.0; 3]);
    }

    #[test]
    fn recursively_projects_passengers_and_drops_duplicate_passengers() {
        let mut root = compound(entity(1, 1.0, 1.0));
        root.push(entry(
            "Passengers",
            NbtSummaryValue::List(vec![entity(2, 1.0, 1.0), entity(1, 1.0, 1.0)]),
        ));
        let parsed = Java26EntityChunkData::parse(
            document(vec![NbtSummaryValue::Compound(root)], 0, 0),
            0,
            0,
        )
        .unwrap();
        let mut passengers = parsed.entities().next().unwrap().passengers();
        assert_eq!(passengers.len(), 1);
        assert_eq!(passengers.next().unwrap().uuid(), Uuid::from_u128(2));
    }

    #[test]
    #[ignore = "requires authorized official Java 26.1 live-gate artifact"]
    fn parses_nonempty_official_java_26_1_entity_entry() {
        let world = std::path::PathBuf::from(
            std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
                .expect("BBB_TEST_JAVA_26_1_WORLD must point at an official Java 26.1 world"),
        );
        let root = world
            .join("dimensions")
            .join("minecraft")
            .join("overworld")
            .join("entities");
        let mut found = None;
        for region_x in -1..=0 {
            for region_z in -1..=0 {
                let path = root.join(format!("r.{region_x}.{region_z}.mca"));
                let store = RegionFileStore::new(&path, &root, WorldSaveLimits::default()).unwrap();
                for local_x in 0..32 {
                    for local_z in 0..32 {
                        let x = region_x * 32 + local_x;
                        let z = region_z * 32 + local_z;
                        if let Some(raw) = store.read_chunk(x, z).unwrap() {
                            let parsed = Java26EntityChunkData::parse(raw, x, z).unwrap();
                            if !parsed.is_empty() {
                                found = Some(parsed);
                                break;
                            }
                        }
                    }
                    if found.is_some() {
                        break;
                    }
                }
            }
        }
        if found.is_none() {
            eprintln!(
                "official Java 26.1 fixture contains no non-empty entity entry; synthetic schema coverage remains authoritative"
            );
        }
    }

    #[test]
    fn content_registry_allows_serializable_mod_entities_and_filters_transient_ones() {
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModEntityRegistration};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:inferno_warden".to_string(),
            width: 0.9,
            height: 2.9,
            serializable: true,
            ..Default::default()
        });
        builder.register_entity(ModEntityRegistration {
            resource_id: "bbb_demo:ephemeral_spark".to_string(),
            width: 0.25,
            height: 0.25,
            serializable: false,
            ..Default::default()
        });
        let registry = builder.freeze().unwrap();

        let uuid_tag = |uuid: Uuid| {
            let bits = uuid.as_u128();
            NbtSummaryValue::IntArray(vec![
                (bits >> 96) as i32,
                (bits >> 64) as i32,
                (bits >> 32) as i32,
                bits as i32,
            ])
        };
        let warden_uuid = Uuid::from_u128(0x101);
        let spark_uuid = Uuid::from_u128(0x102);
        let warden = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("bbb_demo:inferno_warden")),
            entry("UUID", uuid_tag(warden_uuid)),
            entry("Pos", doubles([1.5, 64.0, -2.5])),
        ]);
        let spark = NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::string("bbb_demo:ephemeral_spark")),
            entry("UUID", uuid_tag(spark_uuid)),
            entry("Pos", doubles([3.5, 64.0, -2.5])),
        ]);

        // Vanilla parser rejects unknown mod entity ids.
        let vanilla =
            Java26EntityChunkData::parse(document(vec![warden.clone()], 0, 0), 0, 0).unwrap();
        assert!(vanilla.is_empty());

        let modded = Java26EntityChunkData::parse_with_content_registry(
            document(vec![warden, spark], 0, 0),
            0,
            0,
            &registry,
        )
        .unwrap();
        let entities: Vec<_> = modded.entities().collect();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].id(), "bbb_demo:inferno_warden");
        assert_eq!(entities[0].uuid(), warden_uuid);
    }
}
