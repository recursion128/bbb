//! `BeehiveBlockEntity` (`BeehiveBlockEntity.java:51-433`).
//!
//! Occupants and `flower_pos` round-trip as opaque extra NBT.

use bbb_protocol::nbt_summary::codec_int_value;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::RegistryElementValueRef;

use super::components::{ComponentMapBuilder, ComponentMapReader};
use super::{entry, unique_field};

const TAG_BEES: &str = "bees";
const TAG_FLOWER_POS: &str = "flower_pos";

#[derive(Debug, Clone, PartialEq)]
pub struct BeehiveReleaseEntry {
    pub saved: NbtSummaryValue,
    pub entity_resource_id: String,
    pub entity_data: Vec<NbtSummaryEntry>,
    pub ticks_in_hive: i32,
    pub min_ticks_in_hive: i32,
}

fn occupant_field<'a>(
    entries: &'a [NbtSummaryEntry],
    name: &str,
) -> Result<&'a NbtSummaryValue, String> {
    let mut fields = entries.iter().filter(|entry| entry.name == name);
    let Some(first) = fields.next() else {
        return Err(format!("beehive occupant lacks {name}"));
    };
    if fields.next().is_some() {
        return Err(format!("beehive occupant repeats {name}"));
    }
    Ok(&first.value)
}

fn is_valid_occupant(saved: &NbtSummaryValue) -> bool {
    let NbtSummaryValue::Compound(entries) = saved else {
        return false;
    };
    let Ok(NbtSummaryValue::Compound(entity_data)) = occupant_field(entries, "entity_data") else {
        return false;
    };
    let Ok(NbtSummaryValue::String(entity_resource_id)) = occupant_field(entity_data, "id") else {
        return false;
    };
    if entity_resource_id.try_to_string().is_err() {
        return false;
    }
    if occupant_field(entries, "ticks_in_hive")
        .ok()
        .and_then(codec_int_value)
        .is_none()
    {
        return false;
    }
    occupant_field(entries, "min_ticks_in_hive")
        .ok()
        .and_then(codec_int_value)
        .is_some()
}

fn validate_occupant_list(values: &[NbtSummaryValue]) -> Vec<NbtSummaryValue> {
    if values.iter().all(is_valid_occupant) {
        values.to_vec()
    } else {
        Vec::new()
    }
}

fn release_entry(saved: &NbtSummaryValue) -> Result<BeehiveReleaseEntry, String> {
    let NbtSummaryValue::Compound(entries) = saved else {
        return Err("beehive occupant is not a compound".into());
    };
    let NbtSummaryValue::Compound(entity_data) = occupant_field(entries, "entity_data")? else {
        return Err("beehive occupant entity_data is not a compound".into());
    };
    let NbtSummaryValue::String(entity_resource_id) = occupant_field(entity_data, "id")? else {
        return Err("beehive occupant entity_data id is not a string".into());
    };
    let ticks_in_hive = codec_int_value(occupant_field(entries, "ticks_in_hive")?)
        .ok_or("beehive occupant ticks_in_hive is not numeric")?;
    let min_ticks_in_hive = codec_int_value(occupant_field(entries, "min_ticks_in_hive")?)
        .ok_or("beehive occupant min_ticks_in_hive is not numeric")?;
    Ok(BeehiveReleaseEntry {
        saved: saved.clone(),
        entity_resource_id: entity_resource_id
            .try_to_string()
            .map_err(|_| "beehive occupant entity id is not Unicode")?,
        entity_data: entity_data.clone(),
        ticks_in_hive,
        min_ticks_in_hive,
    })
}

fn block_pos(value: &NbtSummaryValue) -> Result<BlockPos, String> {
    let coords = RegistryElementValueRef::Nbt(value);
    if coords.list_len() != Some(3) {
        return Err("beehive flower_pos must have three coordinates".into());
    }
    let coordinate = |index| {
        coords
            .list_element(index)
            .unwrap()
            .codec_i32()
            .ok_or("beehive flower_pos list is not numeric")
    };
    Ok(BlockPos {
        x: coordinate(0)?,
        y: coordinate(1)?,
        z: coordinate(2)?,
    })
}

/// Saved hive occupants and their residence time.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BeehiveBlockEntity {
    bees: Vec<NbtSummaryValue>,
    flower_pos: Option<NbtSummaryValue>,
}

impl BeehiveBlockEntity {
    pub(crate) fn has_space(&self) -> bool {
        self.bees.len() < 3
    }

    pub(crate) fn add_occupant(
        &mut self,
        record: Vec<NbtSummaryEntry>,
        nectar: bool,
        flower: Option<BlockPos>,
        replace_flower: bool,
    ) -> bool {
        if !self.has_space() {
            return false;
        }
        self.bees.push(NbtSummaryValue::Compound(vec![
            entry("entity_data", NbtSummaryValue::Compound(record)),
            entry("ticks_in_hive", NbtSummaryValue::Int(0)),
            entry(
                "min_ticks_in_hive",
                NbtSummaryValue::Int(if nectar { 2400 } else { 600 }),
            ),
        ]));
        if let Some(pos) = flower.filter(|_| self.flower_pos.is_none() || replace_flower) {
            self.flower_pos = Some(NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]));
        }
        true
    }

    /// `BeeData.tick` (`BeehiveBlockEntity.java:347-352`).
    pub(crate) fn server_tick(&mut self) -> bool {
        let mut ready = false;
        for saved in &mut self.bees {
            let NbtSummaryValue::Compound(entries) = saved else {
                continue;
            };
            let Some(minimum) = occupant_field(entries, "min_ticks_in_hive")
                .ok()
                .and_then(codec_int_value)
            else {
                continue;
            };
            let Some(ticks) = entries
                .iter_mut()
                .find(|entry| entry.name == "ticks_in_hive")
            else {
                continue;
            };
            let Some(current) = codec_int_value(&ticks.value) else {
                continue;
            };
            let next = current.saturating_add(1);
            ready |= next > minimum;
            ticks.value = NbtSummaryValue::Int(next);
        }
        ready
    }

    // BeehiveBlockEntity.java:310-315 preserves each occupant and does not touch savedFlowerPos.
    pub(super) fn apply_implicit_components(
        &mut self,
        components: &mut ComponentMapReader<'_>,
    ) -> bbb_server_types::Result<()> {
        self.bees = match components.read("minecraft:bees")? {
            Some(NbtSummaryValue::List(bees)) => validate_occupant_list(&bees),
            _ => Vec::new(),
        };
        Ok(())
    }

    /// `BeehiveBlockEntity.collectImplicitComponents` (`:318-321`): the
    /// occupants as `Bees.CODEC` writes them, the same list the save holds.
    pub(super) fn collect_implicit_components(&self, components: &mut ComponentMapBuilder<'_>) {
        components.set(
            "minecraft:bees",
            Some(NbtSummaryValue::List(self.bees.clone())),
        );
    }

    /// `BeehiveBlockEntity.Occupant.create` (`BeehiveBlockEntity.java:397-399`).
    pub fn store_bee(&mut self, ticks_in_hive: i32) {
        let bee = NbtSummaryValue::Compound(vec![
            entry(
                "entity_data",
                NbtSummaryValue::Compound(vec![entry(
                    "id",
                    NbtSummaryValue::string("minecraft:bee"),
                )]),
            ),
            entry("ticks_in_hive", NbtSummaryValue::Int(ticks_in_hive)),
            entry("min_ticks_in_hive", NbtSummaryValue::Int(600)),
        ]);
        self.bees.push(bee);
    }

    pub(crate) fn bees(&self) -> &[NbtSummaryValue] {
        &self.bees
    }

    pub fn release_entry_at(&self, index: usize) -> Option<Result<BeehiveReleaseEntry, String>> {
        self.bees.get(index).map(release_entry)
    }

    pub fn release_entries(&self) -> Vec<Result<BeehiveReleaseEntry, String>> {
        let mut entries = Vec::new();
        for bee in &self.bees {
            entries.push(release_entry(bee));
        }
        entries
    }

    /// Removes occupants whose corresponding flag in `released_flags` is true in a single O(N) pass.
    pub fn retain_unreleased(&mut self, released_flags: &[bool]) -> bool {
        if self.bees.len() != released_flags.len() {
            return false;
        }
        let mut index = 0usize;
        let mut removed_any = false;
        self.bees.retain(|_| {
            let remove = released_flags[index];
            index += 1;
            removed_any |= remove;
            !remove
        });
        removed_any
    }

    pub fn remove_released(&mut self, saved: &NbtSummaryValue) -> bool {
        let Some(index) = self.bees.iter().position(|candidate| candidate == saved) else {
            return false;
        };
        self.bees.remove(index);
        true
    }

    pub fn saved_flower_pos(&self) -> Result<Option<BlockPos>, String> {
        self.flower_pos.as_ref().map(block_pos).transpose()
    }

    /// `BeehiveBlockEntity.isEmpty` (`:117-119`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bees.is_empty()
    }

    pub fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = vec![entry(TAG_BEES, NbtSummaryValue::List(self.bees.clone()))];
        if let Some(flower_pos) = &self.flower_pos {
            entries.push(entry(TAG_FLOWER_POS, flower_pos.clone()));
        }
        entries
    }

    /// `BeehiveBlockEntity.loadAdditional` (`BeehiveBlockEntity.java:295-300`):
    /// `Occupant.LIST_CODEC` drops the whole list if any occupant fails validation.
    pub fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.bees = match unique_field(record, TAG_BEES) {
            Some(NbtSummaryValue::List(values)) => validate_occupant_list(values),
            _ => Vec::new(),
        };
        self.flower_pos = unique_field(record, TAG_FLOWER_POS).cloned();
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::VANILLA_BLOCK_BEE_NEST_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::block_entity::new_block_entity;

    fn field(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn worldgen_bees_append_in_order_and_round_trip() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_BEE_NEST_ID).unwrap();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        let mut built = new_block_entity(pos, state).unwrap();
        let payload = built.beehive_mut().unwrap();
        payload.store_bee(17);
        payload.store_bee(23);
        assert_eq!(payload.bees().len(), 2);
        let saved = built
            .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
            .expect("fixture codec succeeds");
        let loaded = crate::block_entity::load_test_fixture(pos, state, &saved)
            .expect("fixture codec succeeds")
            .unwrap();
        assert_eq!(
            loaded
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver(),)
                .expect("fixture codec succeeds"),
            saved
        );
        assert!(loaded.chest().is_none());
    }

    #[test]
    fn saved_occupants_parse_in_order_and_drop_malformed_lists() {
        let state = vanilla_block_default_state_id(VANILLA_BLOCK_BEE_NEST_ID).unwrap();
        let pos = BlockPos { x: 1, y: 2, z: 3 };
        let first = NbtSummaryValue::Compound(vec![
            field(
                "entity_data",
                NbtSummaryValue::Compound(vec![
                    field("id", NbtSummaryValue::String("minecraft:bee".into())),
                    field("Age", NbtSummaryValue::Int(-37)),
                ]),
            ),
            field("ticks_in_hive", NbtSummaryValue::Int(17)),
            field("min_ticks_in_hive", NbtSummaryValue::Int(600)),
        ]);
        let second = NbtSummaryValue::Compound(vec![
            field(
                "entity_data",
                NbtSummaryValue::Compound(vec![
                    field("id", NbtSummaryValue::String("minecraft:bee".into())),
                    field("Age", NbtSummaryValue::Int(-12)),
                ]),
            ),
            field("ticks_in_hive", NbtSummaryValue::Int(42)),
            field("min_ticks_in_hive", NbtSummaryValue::Int(600)),
        ]);
        let invalid =
            NbtSummaryValue::Compound(vec![field("ticks_in_hive", NbtSummaryValue::Int(3))]);

        // Malformed list is dropped wholesale per `Occupant.LIST_CODEC` (`BeehiveBlockEntity.java:298`).
        let mut malformed_hive = new_block_entity(pos, state).unwrap();
        malformed_hive.beehive_mut().unwrap().load_additional(&[
            field(
                "bees",
                NbtSummaryValue::List(vec![first.clone(), invalid, first.clone()]),
            ),
            field("flower_pos", NbtSummaryValue::IntArray(vec![7, 8, 9])),
        ]);
        assert!(malformed_hive.beehive().unwrap().is_empty());

        let saved = vec![
            field(
                "bees",
                NbtSummaryValue::List(vec![first.clone(), second.clone(), first.clone()]),
            ),
            field("flower_pos", NbtSummaryValue::IntArray(vec![7, 8, 9])),
        ];
        let mut hive = new_block_entity(pos, state).unwrap();
        hive.beehive_mut().unwrap().load_additional(&saved);
        let reloaded = crate::block_entity::load_test_fixture(
            pos,
            state,
            &hive
                .save_with_full_metadata_with_resolver(crate::block_entity::test_item_resolver())
                .expect("fixture codec succeeds"),
        )
        .expect("fixture codec succeeds")
        .unwrap();
        let hive = reloaded.beehive().unwrap();
        let entries = hive.release_entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0].as_ref().unwrap().entity_resource_id,
            "minecraft:bee"
        );
        assert_eq!(entries[0].as_ref().unwrap().ticks_in_hive, 17);
        assert_eq!(entries[1].as_ref().unwrap().ticks_in_hive, 42);
        assert_eq!(
            hive.saved_flower_pos().unwrap(),
            Some(BlockPos { x: 7, y: 8, z: 9 })
        );
        let mut hive = reloaded;
        let hive = hive.beehive_mut().unwrap();
        assert!(hive.retain_unreleased(&[true, false, false]));
        assert_eq!(hive.bees(), &[second, first]);
    }
}
