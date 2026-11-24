use super::*;

impl NoiseStructureChunkData {
    /// `StructureStart.java:126-139`: whether a valid start can still be referenced.
    pub fn can_add_reference(&self, structure_id: &str) -> bool {
        macro_rules! check_start {
            ($starts:expr) => {
                if let Some(start) = $starts
                    .iter()
                    .find(|start| start.structure_id() == structure_id)
                {
                    return start.references() < 1;
                }
            };
        }
        check_start!(self.starts);
        check_start!(self.buried_treasure_start);
        check_start!(self.mineshaft_starts);
        check_start!(self.shipwreck_starts);
        check_start!(self.ocean_ruin_starts);
        check_start!(self.igloo_starts);
        check_start!(self.scattered_starts);
        check_start!(self.ruined_portal_starts);
        check_start!(self.ocean_monument_starts);
        check_start!(self.woodland_mansion_starts);
        check_start!(self.stronghold_starts);
        check_start!(self.fortress_starts);
        check_start!(self.nether_fossil_starts);
        check_start!(self.end_city_starts);
        if let Some(restored) = self.restored_bounds.get(structure_id) {
            return restored.references < 1;
        }
        false
    }

    /// `StructureStart.java:126-139`: a valid start can be referenced once.
    pub fn try_add_reference(&mut self, structure_id: &str) -> bool {
        macro_rules! try_start {
            ($starts:expr) => {
                if let Some(start) = $starts
                    .iter_mut()
                    .find(|start| start.structure_id() == structure_id)
                {
                    return start.try_add_reference();
                }
            };
        }
        try_start!(self.starts);
        try_start!(self.buried_treasure_start);
        try_start!(self.mineshaft_starts);
        try_start!(self.shipwreck_starts);
        try_start!(self.ocean_ruin_starts);
        try_start!(self.igloo_starts);
        try_start!(self.scattered_starts);
        try_start!(self.ruined_portal_starts);
        try_start!(self.ocean_monument_starts);
        try_start!(self.woodland_mansion_starts);
        try_start!(self.stronghold_starts);
        try_start!(self.fortress_starts);
        try_start!(self.nether_fossil_starts);
        try_start!(self.end_city_starts);
        if let Some(restored) = self.restored_bounds.get_mut(structure_id) {
            return restored.try_add_reference();
        }
        false
    }

    pub fn has_valid_start(&self, structure_id: &str) -> bool {
        self.starts
            .iter()
            .any(|start| start.structure_id() == structure_id)
            || self
                .buried_treasure_start
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .mineshaft_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .shipwreck_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .ocean_ruin_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .igloo_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .scattered_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .ruined_portal_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .ocean_monument_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .woodland_mansion_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .stronghold_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .fortress_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .nether_fossil_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self
                .end_city_starts
                .iter()
                .any(|start| start.structure_id() == structure_id)
            || self.restored_bounds.contains_key(structure_id)
    }

    pub fn references(&self) -> &BTreeMap<String, Vec<i64>> {
        &self.references
    }

    pub fn exclusive_nonempty_referenced_structure(&self) -> Option<&str> {
        let mut exclusive = None;
        for (structure_id, positions) in &self.references {
            if positions.is_empty() {
                continue;
            }
            match exclusive {
                None => exclusive = Some(structure_id.as_str()),
                Some(existing) if existing == structure_id => {}
                Some(_) => return None,
            }
        }
        exclusive
    }

    pub fn referenced_source_chunks(&self) -> BTreeSet<[i32; 2]> {
        self.references
            .values()
            .flatten()
            .map(|&packed| unpack_chunk(packed))
            .collect()
    }
}

impl NoiseStructureChunkData {
    /// Assembles the jigsaw References from the starts of the 17x17 source
    /// window around the target (`ChunkGenerator.java:598-627`).
    pub fn from_complete_reference_window(
        chunk_x: i32,
        chunk_z: i32,
        owned_starts: impl IntoIterator<Item = JigsawStructureStart>,
        mut starts_at: impl FnMut(i32, i32) -> Vec<JigsawStructureStart>,
    ) -> Self {
        let mut nearby = Vec::new();
        for source_x in chunk_x - 8..=chunk_x + 8 {
            for source_z in chunk_z - 8..=chunk_z + 8 {
                nearby.extend(starts_at(source_x, source_z));
            }
        }
        Self::from_supplied_starts(chunk_x, chunk_z, owned_starts, nearby.iter())
    }

    /// Loads every STRUCTURE_STARTS chunk in Vanilla's dependency window and
    /// adds the target References for every start that intersects it
    /// (`ChunkPyramid.java:9-19`, `ChunkGenerator.java:598-627`).
    pub fn references_from_complete_source_window<E>(
        target_starts: &Self,
        mut source_at: impl FnMut(i32, i32) -> Result<Self, E>,
    ) -> Result<(Self, BTreeMap<[i32; 2], Self>), E> {
        let [chunk_x, chunk_z] = target_starts.chunk;
        let mut sources = BTreeMap::new();
        for source_x in chunk_x - 8..=chunk_x + 8 {
            for source_z in chunk_z - 8..=chunk_z + 8 {
                sources.insert([source_x, source_z], source_at(source_x, source_z)?);
            }
        }
        let jigsaw = Self::from_complete_reference_window(
            chunk_x,
            chunk_z,
            target_starts.starts.iter().cloned(),
            |x, z| sources[&[x, z]].starts.clone(),
        );
        let mut target = Self {
            starts: jigsaw.starts,
            references: jigsaw.references,
            ..target_starts.clone()
        };
        for (&[source_x, source_z], source) in &sources {
            let mut reference = |structure_id: &str| {
                target
                    .references
                    .entry(structure_id.to_string())
                    .or_default()
                    .push(pack_chunk(source_x, source_z));
            };
            macro_rules! reference_intersecting {
                ($($starts:expr),*) => {$(
                    for start in $starts.iter() {
                        if start.intersects_chunk(chunk_x, chunk_z) {
                            reference(start.structure_id());
                        }
                    }
                )*};
            }
            reference_intersecting!(
                source.buried_treasure_start,
                source.mineshaft_starts,
                source.shipwreck_starts,
                source.ocean_ruin_starts,
                source.igloo_starts,
                source.scattered_starts,
                source.ruined_portal_starts,
                source.ocean_monument_starts,
                source.woodland_mansion_starts,
                source.stronghold_starts,
                source.fortress_starts,
                source.nether_fossil_starts,
                source.end_city_starts
            );
            for (id, restored) in &source.restored_bounds {
                if restored.intersects_chunk(chunk_x, chunk_z) {
                    reference(id);
                }
            }
        }
        for positions in target.references.values_mut() {
            positions.sort_by_key(|packed| unpack_chunk(*packed));
            positions.dedup();
        }
        Ok((target, sources))
    }

    /// Borrows every STRUCTURE_STARTS chunk in Vanilla's 17x17 dependency window
    /// without cloning the 289 window components.
    pub fn references_from_complete_source_window_borrowed<'a, E>(
        target_starts: &Self,
        mut source_at: impl FnMut(i32, i32) -> Result<&'a Self, E>,
    ) -> Result<Self, E> {
        let [chunk_x, chunk_z] = target_starts.chunk;
        let mut sources = Vec::with_capacity(17 * 17);
        for source_x in chunk_x - 8..=chunk_x + 8 {
            for source_z in chunk_z - 8..=chunk_z + 8 {
                sources.push(([source_x, source_z], source_at(source_x, source_z)?));
            }
        }
        let jigsaw = Self::from_supplied_starts(
            chunk_x,
            chunk_z,
            target_starts.starts.iter().cloned(),
            sources.iter().flat_map(|(_, source)| source.starts.iter()),
        );
        let mut target = Self {
            starts: jigsaw.starts,
            references: jigsaw.references,
            ..target_starts.clone()
        };
        for &([source_x, source_z], source) in &sources {
            let mut reference = |structure_id: &str| {
                target
                    .references
                    .entry(structure_id.to_string())
                    .or_default()
                    .push(pack_chunk(source_x, source_z));
            };
            macro_rules! reference_intersecting {
                ($($starts:expr),*) => {$(
                    for start in $starts.iter() {
                        if start.intersects_chunk(chunk_x, chunk_z) {
                            reference(start.structure_id());
                        }
                    }
                )*};
            }
            reference_intersecting!(
                source.buried_treasure_start,
                source.mineshaft_starts,
                source.shipwreck_starts,
                source.ocean_ruin_starts,
                source.igloo_starts,
                source.scattered_starts,
                source.ruined_portal_starts,
                source.ocean_monument_starts,
                source.woodland_mansion_starts,
                source.stronghold_starts,
                source.fortress_starts,
                source.nether_fossil_starts,
                source.end_city_starts
            );
            for (id, restored) in &source.restored_bounds {
                if restored.intersects_chunk(chunk_x, chunk_z) {
                    reference(id);
                }
            }
        }
        for positions in target.references.values_mut() {
            positions.sort_by_key(|packed| unpack_chunk(*packed));
            positions.dedup();
        }
        Ok(target)
    }
}
