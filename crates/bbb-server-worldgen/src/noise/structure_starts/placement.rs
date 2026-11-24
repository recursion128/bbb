//! Borrowed FEATURES placement of canonical structure starts.

use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::startup::DatapackStartup;

use super::*;
use crate::noise::mineshaft::MineshaftPlacementLevel;

pub trait PlaceableStart: Sized {
    fn find_mut<'a>(data: &'a mut NoiseStructureChunkData, id: &str) -> Option<&'a mut Self>;

    fn place<L: MineshaftPlacementLevel>(
        &mut self,
        startup: &DatapackStartup,
        level: &mut L,
        target: [i32; 2],
        bounds: JigsawBoundingBox,
        random: &mut dyn RandomSource,
    );
}

macro_rules! placeable_starts {
    ($($ty:ty => $field:ident,
        |$start:ident, $startup:ident, $level:ident, $target:ident, $bounds:ident, $random:ident|
        $body:block;)*) => {$(
        impl PlaceableStart for $ty {
            fn find_mut<'a>(data: &'a mut NoiseStructureChunkData, id: &str) -> Option<&'a mut Self> {
                data.$field.iter_mut().find(|start| start.structure_id() == id)
            }

            fn place<L: MineshaftPlacementLevel>(
                &mut self,
                $startup: &DatapackStartup,
                $level: &mut L,
                $target: [i32; 2],
                $bounds: JigsawBoundingBox,
                $random: &mut dyn RandomSource,
            ) {
                let $start = self;
                $body
            }
        }
    )*};
}

placeable_starts! {
    JigsawStructureStart => starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(startup, level, bounds, random);
        };
    BuriedTreasureStructureStart => buried_treasure_start,
        |start, _startup, level, _target, _bounds, random| {
            start.place_in_chunk(level, random.next_long());
        };
    MineshaftStructureStart => mineshaft_starts,
        |start, _startup, level, target, _bounds, random| {
            start.place_in_chunk(level, random, target);
        };
    ShipwreckStructureStart => shipwreck_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    OceanRuinStructureStart => ocean_ruin_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    IglooStructureStart => igloo_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    EndCityStructureStart => end_city_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    WoodlandMansionStructureStart => woodland_mansion_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    RuinedPortalStructureStart => ruined_portal_starts,
        |start, startup, level, _target, bounds, random| {
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
    ScatteredFeatureStructureStart => scattered_starts,
        |start, _startup, level, _target, bounds, random| {
            start.place_in_chunk(level, bounds, random);
        };
    OceanMonumentStructureStart => ocean_monument_starts,
        |start, _startup, level, _target, bounds, random| {
            start.place_in_chunk(level, bounds, random);
        };
    StrongholdStructureStart => stronghold_starts,
        |start, _startup, level, _target, bounds, random| {
            start.place_in_chunk(level, bounds, random);
        };
    FortressStructureStart => fortress_starts,
        |start, _startup, level, _target, bounds, random| {
            let bounds = JigsawBoundingBox {
                min: BlockPos { y: i32::MIN, ..bounds.min },
                max: BlockPos { y: i32::MAX, ..bounds.max },
            };
            start.place_in_chunk(level, bounds, random);
        };
    NetherFossilStructureStart => nether_fossil_starts,
        |start, startup, level, _target, bounds, random| {
            let bounds = JigsawBoundingBox {
                min: BlockPos { y: i32::MIN, ..bounds.min },
                max: BlockPos { y: i32::MAX, ..bounds.max },
            };
            start.place_in_chunk(&startup.structures, level, bounds, random);
        };
}
