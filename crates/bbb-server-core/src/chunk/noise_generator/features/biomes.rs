use super::*;

pub(super) fn neighborhood_biomes<'a>(
    generation: &DatapackGeneration,
    chunks: impl Iterator<Item = &'a ChunkColumn>,
    path: &str,
) -> SaveResult<Vec<String>> {
    let registry = generation
        .registry(BIOME_REGISTRY)
        .ok_or_else(|| invalid_schema(path, "the biome registry is absent"))?;
    // Collect distinct present biome ids per section without scanning uniform sections.
    let mut raw_ids = Vec::new();
    for chunk in chunks {
        for section in 0..chunk.section_count() {
            chunk
                .section_biomes(section)
                .visit_present_biomes(|raw_id| {
                    if !raw_ids.contains(&raw_id) {
                        raw_ids.push(raw_id);
                    }
                });
        }
    }
    let mut biome_ids = BTreeSet::new();
    for raw_id in raw_ids {
        let element = usize::try_from(raw_id)
            .ok()
            .and_then(|id| registry.elements.get(id))
            .ok_or_else(|| {
                invalid_schema(
                    path,
                    format!("generated biome registry id {raw_id} is not published"),
                )
            })?;
        biome_ids.insert(element.id.clone());
    }
    Ok(biome_ids.into_iter().collect())
}
