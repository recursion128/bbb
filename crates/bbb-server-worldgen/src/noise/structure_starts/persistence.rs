use super::*;
use crate::noise::structure_nbt::{compound, entry, int};

impl NoiseStructureChunkData {
    /// Writes explicit `starts` (Vanilla proto-chunks and forced `/place structure`
    /// starts) as full Vanilla `starts` compounds, while writing compact
    /// `bbb_starts` summaries for natural BBB-generated starts.
    pub fn to_nbt(&self) -> NbtSummaryValue {
        let mut starts = Vec::new();
        macro_rules! push_explicit {
            ($items:expr) => {
                for start in $items.iter() {
                    if self.explicit_starts.contains(start.structure_id()) {
                        starts.push(entry(start.structure_id(), start.to_nbt()));
                    }
                }
            };
        }
        push_explicit!(self.starts);
        push_explicit!(self.buried_treasure_start);
        push_explicit!(self.mineshaft_starts);
        push_explicit!(self.shipwreck_starts);
        push_explicit!(self.ocean_ruin_starts);
        push_explicit!(self.igloo_starts);
        push_explicit!(self.scattered_starts);
        push_explicit!(self.ruined_portal_starts);
        push_explicit!(self.ocean_monument_starts);
        push_explicit!(self.woodland_mansion_starts);
        push_explicit!(self.stronghold_starts);
        push_explicit!(self.fortress_starts);
        push_explicit!(self.nether_fossil_starts);
        push_explicit!(self.end_city_starts);
        starts.sort_by(|left, right| left.name.cmp(&right.name));

        let mut bbb_starts = Vec::new();
        for (id, references) in self.owned_start_references() {
            if self.explicit_starts.contains(&id) {
                continue;
            }
            let pieces = self.raw_start_geometry(&id).unwrap_or_default();
            let mut coords = Vec::with_capacity(pieces.len() * 6);
            for bounds in pieces {
                coords.extend_from_slice(&[
                    bounds.min.x,
                    bounds.min.y,
                    bounds.min.z,
                    bounds.max.x,
                    bounds.max.y,
                    bounds.max.z,
                ]);
            }
            bbb_starts.push(entry(
                &id,
                compound(vec![
                    int("references", references),
                    entry("bounds", NbtSummaryValue::IntArray(coords)),
                ]),
            ));
        }
        let mut fields = vec![
            entry("starts", NbtSummaryValue::Compound(starts)),
            entry(
                "References",
                NbtSummaryValue::Compound(
                    self.references
                        .iter()
                        .map(|(id, positions)| {
                            entry(id, NbtSummaryValue::LongArray(positions.clone()))
                        })
                        .collect(),
                ),
            ),
        ];
        if !bbb_starts.is_empty() {
            fields.push(entry("bbb_starts", NbtSummaryValue::Compound(bbb_starts)));
        }
        compound(fields)
    }
}
