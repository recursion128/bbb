use super::*;

pub(super) fn structure_in_tag(
    runtime: &MinecartSpawnerRuntime<'_>,
    pos: BlockPos,
    tag: &str,
) -> bool {
    let Some(id) = bbb_registry_codec::registry_codec::RegistryResourceIdentifier::parse(tag)
    else {
        return false;
    };
    structure(
        runtime,
        pos,
        &HolderSetReference {
            references: vec![HolderReference::Tag(id)],
        },
    )
}

fn structure(
    runtime: &MinecartSpawnerRuntime<'_>,
    pos: BlockPos,
    structures: &HolderSetReference,
) -> bool {
    runtime
        .chunks
        .natural_spawn_structure_target_at(pos)
        .is_some_and(|target| {
            target.referenced_piece_contains(
                pos,
                |id| {
                    holder_set_contains(
                        runtime.chunks.registry_tags(),
                        "minecraft:worldgen/structure",
                        structures,
                        id,
                    )
                },
                |x, z| runtime.chunks.structure_component(x, z),
            )
        })
}

pub(super) fn prepare<'registry>(
    runtime: &MinecartSpawnerRuntime<'registry>,
    pos: BlockPos,
    biome: &str,
) -> Result<SpawnContext<'registry>, String> {
    runtime
        .datapacks
        .and_then(|packs| packs.registries.registry("minecraft:cat_variant"))
        .ok_or("cat variant registry missing")?;
    let moon_brightness = runtime
        .spawn_environment
        .moon_brightness(runtime.chunks, pos)
        .ok_or("cat missing moon context")?;
    let mut context = runtime
        .finalize_context(pos, "minecraft:cat")
        .ok_or("cat missing moon/difficulty context")?;
    context.biome = Some(biome.to_owned());
    context.variants = select_animal_variants(
        runtime.datapacks,
        runtime.registries,
        "minecraft:cat_variant",
        |condition| match condition {
            SpawnCondition::Structure { structures } => structure(runtime, pos, structures),
            _ => spawn_condition_matches(runtime, pos, biome, moon_brightness, condition),
        },
    );
    context.variants.extend(sound_variant_candidates(
        runtime.datapacks,
        runtime.registries,
        "minecraft:cat_sound_variant",
    )?);
    Ok(context)
}
