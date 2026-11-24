//! `StructureTemplate.placeInWorld` loading a template's block entities
//! through `level.registryAccess()` (`StructureTemplate.java:305-306`).

use std::sync::Arc;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::chunk::generation_publication::GenerationItemPublication;
use crate::chunk::noise_generator::test_support::{
    fresh_overworld_program, BuiltinEnchantmentPublication,
};
use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use bbb_server_worldgen::noise::woodland_mansion::WoodlandMansionStructureStart;

use super::with_feature_region;

/// `minecraft:woodland_mansion/1x2_a6.nbt` puts an enchanted iron axe in slot
/// 10 of its chest.
const AXE_ROOM: &str = "1x2_a6";

/// Places the first mansion with an axe room into each chunk around that room
/// and returns the chest records found there.
fn axe_room_chest_records(
    publication: Option<GenerationItemPublication>,
) -> Vec<Vec<NbtSummaryEntry>> {
    let (seed, _, starts) = super::find_mansion(|start| {
        start
            .pieces()
            .iter()
            .any(|piece| piece.template().ends_with(AXE_ROOM))
    });
    let program: Arc<NoiseFinalDensityProgram> = fresh_overworld_program(true, seed);
    if let Some(publication) = publication {
        assert!(program.publish_item_registry(publication));
    }
    let room = starts[0]
        .pieces()
        .iter()
        .find(|piece| piece.template().ends_with(AXE_ROOM))
        .unwrap()
        .template_position();
    let source_chunk = starts[0].start_chunk();
    let mut records = Vec::new();
    for x in (room.x >> 4) - 1..=(room.x >> 4) + 1 {
        for z in (room.z >> 4) - 1..=(room.z >> 4) + 1 {
            let target = [x, z];
            let invocation = program
                .woodland_mansion_structure_decoration_invocation(target)
                .unwrap();
            let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
                source_chunk[0], source_chunk[1], [], [],
            ).with_woodland_mansion_starts(starts.clone());
            let mut cache = super::structure_cache(target, source);
            let store = with_feature_region(&program, target, |region, _source| {
                program
                    .structure_generation_context()?
                    .place_referenced_structure::<WoodlandMansionStructureStart>(
                        &mut cache,
                        &invocation,
                        region,
                    )
            });
            records.extend(super::block_entity_records(&store, target, &program));
        }
    }
    records
}

fn efficiency_level(record: &[NbtSummaryEntry]) -> Option<i32> {
    let NbtSummaryValue::List(items) = &record.iter().find(|entry| entry.name == "Items")?.value
    else {
        return None;
    };
    items.iter().find_map(|item| {
        let NbtSummaryValue::Compound(stack) = item else {
            return None;
        };
        let NbtSummaryValue::Compound(components) =
            &stack.iter().find(|entry| entry.name == "components")?.value
        else {
            return None;
        };
        let NbtSummaryValue::Compound(enchantments) = &components
            .iter()
            .find(|entry| entry.name == "minecraft:enchantments")?
            .value
        else {
            return None;
        };
        match enchantments
            .iter()
            .find(|entry| entry.name == "minecraft:efficiency")?
            .value
        {
            NbtSummaryValue::Int(level) => Some(level),
            _ => None,
        }
    })
}

#[test]
fn a_generated_mansion_chest_keeps_the_enchantment_the_level_publishes() {
    let records = axe_room_chest_records(Some(GenerationItemPublication::new(Arc::new(
        BuiltinEnchantmentPublication,
    ))));
    assert!(records
        .iter()
        .any(|record| efficiency_level(record) == Some(1)));
}
