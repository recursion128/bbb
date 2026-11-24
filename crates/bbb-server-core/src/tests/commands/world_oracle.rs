//! Command parity against the official world-oracle setup save.

use std::collections::BTreeMap;

use super::*;

/// The world every world-oracle scenario is authored inside.
///
/// `scripts/server_world_oracle.py` builds its save inside a copy of an
/// official capture, whose terrain is `FlatLevelGeneratorSettings.getDefault`
/// (FlatLevelGeneratorSettings.java:217-219): one bedrock, then two dirt, then
/// one grass block, stacked from the dimension's own `min_y`. This crate's
/// created save declares a different flat column, so the document is patched
/// back to that default before it is parsed -- otherwise every cell inside a
/// compared box that no setup line writes would be a fixture difference rather
/// than a command one.
///
/// It carries no ticket at all: the save `prepare_world` opens has none either,
/// and the four columns the script writes into are the ones its own
/// `forceload add` lines force.
fn world_oracle_world() -> ServerWorld {
    let mut candidate = restore_candidate_without_tickets(10, None, Vec::new());
    candidate.world_gen_settings = world_oracle_world_gen_settings();
    ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap()
}

/// The created save's own `world_gen_settings.dat`, with the flat preset's
/// default layers put back.
fn world_oracle_world_gen_settings() -> Java26WorldGenSettings {
    let mut document =
        crate::world_creation::created_world_gen_settings_document(&FlatFixture::new().unwrap())
            .unwrap();
    for entry in document.entries_mut() {
        replace_flat_layers(&mut entry.value);
    }
    bbb_server_persistence::world_gen_settings_schema::parse_java_26_1_world_gen_settings(
        document, 0,
    )
    .unwrap()
}

/// Rewrites every `layers` list in a generator document to the default one.
fn replace_flat_layers(value: &mut NbtSummaryValue) {
    match value {
        NbtSummaryValue::Compound(entries) => {
            for entry in entries.iter_mut() {
                if entry.name == "layers" {
                    entry.value = NbtSummaryValue::List(
                        [
                            ("minecraft:bedrock", 1),
                            ("minecraft:dirt", 2),
                            ("minecraft:grass_block", 1),
                        ]
                        .into_iter()
                        .map(|(block, height)| {
                            NbtSummaryValue::Compound(vec![
                                nbt("block", NbtSummaryValue::string(block)),
                                nbt("height", NbtSummaryValue::Int(height)),
                            ])
                        })
                        .collect(),
                    );
                } else {
                    replace_flat_layers(&mut entry.value);
                }
            }
        }
        NbtSummaryValue::List(children) => {
            for child in children.iter_mut() {
                replace_flat_layers(child);
            }
        }
        _ => {}
    }
}

/// One live cell, named the way a region file's palette names it: the block id
/// and, when the state carries any, its properties in `StateDefinition` order.
fn described_block(world: &mut ServerWorld, pos: BlockPos) -> String {
    let state = live_block(world, pos).expect("a loaded cell");
    let identity = bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .expect("a generated state");
    if identity.properties.is_empty() {
        return identity.name.clone();
    }
    let properties = identity
        .properties
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{}[{properties}]", identity.name)
}

/// This server authoring the world oracle's own input save.
///
/// `scripts/server_world_oracle.py` compares two servers over a save neither of
/// them wrote: `prepare_world` boots Mojang's server frozen, types
/// [`WORLD_ORACLE_SETUP`] at its console and stops it, and both sides then open
/// what it saved. Every line of that script is a `/setblock`, a `/fill`, a
/// `/gamerule`, a `/tick freeze` or a `/forceload`, and this server now runs
/// all four -- so it can author the same save itself, starting from a world
/// with no ticket in it.
///
/// The assertion is against the official server's own bytes rather than
/// against what this dispatcher was meant to do: [`WORLD_ORACLE_BOXES`] is the
/// region-file truth read back out of a `prepare_world` run, box for box, cell
/// for cell. It covers what the arguments carry (a plain id, an id with
/// properties, a `minecraft:air` erase), both roots, the neighbour pass a
/// non-strict write runs -- the two redstone lamps are `lit=true` only because
/// `updateNeighboursOnBlockSet` reached them -- and the shape a `/fill` leaves
/// when a later one cuts a trench through it.
#[test]
fn the_world_oracle_setup_script_writes_what_the_official_server_wrote() {
    let (sender, receiver) = mailbox();
    let mut world = world_oracle_world();
    for (index, command) in WORLD_ORACLE_SETUP.iter().enumerate() {
        let tick = block_command(&sender, &receiver, &mut world, index as u64, command);
        let (result, messages) = management_report(&tick);
        assert!(result.is_some(), "{command} ran, but answered {messages:?}");
    }
    // The four columns the script forced are the four it then writes into, and
    // they are simulating because the command filed the ticket -- not because
    // the save declared one.
    assert_eq!(
        world.entity_ticking_chunks(Java26BuiltinDimension::Overworld),
        [(0, 0), (1, 0), (2, 0), (3, 0)]
    );

    for (name, [min_x, min_y, min_z, max_x, max_y, max_z], written) in WORLD_ORACLE_BOXES {
        let written = written
            .iter()
            .map(|(x, y, z, state)| ((*x, *y, *z), *state))
            .collect::<BTreeMap<_, _>>();
        for x in *min_x..=*max_x {
            for y in *min_y..=*max_y {
                for z in *min_z..=*max_z {
                    let expected = written.get(&(x, y, z)).copied().unwrap_or("minecraft:air");
                    assert_eq!(
                        described_block(&mut world, BlockPos { x, y, z }),
                        expected,
                        "{name} at {x} {y} {z}"
                    );
                }
            }
        }
    }
}
