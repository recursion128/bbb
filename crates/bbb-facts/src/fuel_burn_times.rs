//! Vanilla 26.1 furnace fuel burn durations shared by server datapacks and client inspection.

/// The `baseUnit` the two-argument `vanillaBurnTimes` passes
/// (`FuelValues.java:38-40`), which is `BURN_TIME_STANDARD`
/// (`AbstractFurnaceBlockEntity.java:53`).
pub const VANILLA_FUEL_BASE_UNIT: i32 = 200;

/// One row's declared duration as the Java integer expression
/// `offset + base_unit * multiplier / divisor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BurnTime {
    offset: i32,
    multiplier: i32,
    divisor: i32,
}

impl BurnTime {
    pub const fn new(offset: i32, multiplier: i32, divisor: i32) -> Self {
        Self {
            offset,
            multiplier,
            divisor,
        }
    }

    pub fn ticks(self, base_unit: i32) -> i32 {
        self.offset + base_unit * self.multiplier / self.divisor
    }
}

/// One `FuelValues.Builder` call, in the order `vanillaBurnTimes` makes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelRow {
    /// `Builder.add(ItemLike, int)` (`FuelValues.java:140-144`); a `Blocks`
    /// argument names the item its `asItem` returns.
    Item(&'static str, BurnTime),
    /// `Builder.add(TagKey, int)` (`FuelValues.java:131-138`), which walks the
    /// tag's holders in their bound order.
    Tag(&'static str, BurnTime),
    /// `Builder.remove(TagKey)` (`FuelValues.java:126-129`).
    RemoveTag(&'static str),
}

const fn unit(multiplier: i32) -> BurnTime {
    BurnTime::new(0, multiplier, 1)
}

const fn fraction(multiplier: i32, divisor: i32) -> BurnTime {
    BurnTime::new(0, multiplier, divisor)
}

/// `FuelValues.vanillaBurnTimes` (`FuelValues.java:42-110`), row for row.
pub const VANILLA_BURN_TIME_ROWS: &[FuelRow] = &[
    FuelRow::Item("minecraft:lava_bucket", unit(100)),
    FuelRow::Item("minecraft:coal_block", unit(8 * 10)),
    FuelRow::Item("minecraft:blaze_rod", unit(12)),
    FuelRow::Item("minecraft:coal", unit(8)),
    FuelRow::Item("minecraft:charcoal", unit(8)),
    FuelRow::Tag("minecraft:logs", fraction(3, 2)),
    FuelRow::Tag("minecraft:bamboo_blocks", fraction(3, 2)),
    FuelRow::Tag("minecraft:planks", fraction(3, 2)),
    FuelRow::Item("minecraft:bamboo_mosaic", fraction(3, 2)),
    FuelRow::Tag("minecraft:wooden_stairs", fraction(3, 2)),
    FuelRow::Item("minecraft:bamboo_mosaic_stairs", fraction(3, 2)),
    FuelRow::Tag("minecraft:wooden_slabs", fraction(3, 4)),
    FuelRow::Item("minecraft:bamboo_mosaic_slab", fraction(3, 4)),
    FuelRow::Tag("minecraft:wooden_trapdoors", fraction(3, 2)),
    FuelRow::Tag("minecraft:wooden_pressure_plates", fraction(3, 2)),
    FuelRow::Tag("minecraft:wooden_shelves", fraction(3, 2)),
    FuelRow::Tag("minecraft:wooden_fences", fraction(3, 2)),
    FuelRow::Tag("minecraft:fence_gates", fraction(3, 2)),
    FuelRow::Item("minecraft:note_block", fraction(3, 2)),
    FuelRow::Item("minecraft:bookshelf", fraction(3, 2)),
    FuelRow::Item("minecraft:chiseled_bookshelf", fraction(3, 2)),
    FuelRow::Item("minecraft:lectern", fraction(3, 2)),
    FuelRow::Item("minecraft:jukebox", fraction(3, 2)),
    FuelRow::Item("minecraft:chest", fraction(3, 2)),
    FuelRow::Item("minecraft:trapped_chest", fraction(3, 2)),
    FuelRow::Item("minecraft:crafting_table", fraction(3, 2)),
    FuelRow::Item("minecraft:daylight_detector", fraction(3, 2)),
    FuelRow::Tag("minecraft:banners", fraction(3, 2)),
    FuelRow::Item("minecraft:bow", fraction(3, 2)),
    FuelRow::Item("minecraft:fishing_rod", fraction(3, 2)),
    FuelRow::Item("minecraft:ladder", fraction(3, 2)),
    FuelRow::Tag("minecraft:signs", unit(1)),
    FuelRow::Tag("minecraft:hanging_signs", unit(4)),
    FuelRow::Item("minecraft:wooden_shovel", unit(1)),
    FuelRow::Item("minecraft:wooden_sword", unit(1)),
    FuelRow::Item("minecraft:wooden_spear", unit(1)),
    FuelRow::Item("minecraft:wooden_hoe", unit(1)),
    FuelRow::Item("minecraft:wooden_axe", unit(1)),
    FuelRow::Item("minecraft:wooden_pickaxe", unit(1)),
    FuelRow::Tag("minecraft:wooden_doors", unit(1)),
    FuelRow::Tag("minecraft:boats", unit(6)),
    FuelRow::Tag("minecraft:wool", fraction(1, 2)),
    FuelRow::Tag("minecraft:wooden_buttons", fraction(1, 2)),
    FuelRow::Item("minecraft:stick", fraction(1, 2)),
    FuelRow::Tag("minecraft:saplings", fraction(1, 2)),
    FuelRow::Item("minecraft:bowl", fraction(1, 2)),
    FuelRow::Tag("minecraft:wool_carpets", BurnTime::new(1, 1, 3)),
    FuelRow::Item("minecraft:dried_kelp_block", BurnTime::new(1, 20, 1)),
    FuelRow::Item("minecraft:crossbow", fraction(3, 2)),
    FuelRow::Item("minecraft:bamboo", fraction(1, 4)),
    FuelRow::Item("minecraft:dead_bush", fraction(1, 2)),
    FuelRow::Item("minecraft:short_dry_grass", fraction(1, 2)),
    FuelRow::Item("minecraft:tall_dry_grass", fraction(1, 2)),
    FuelRow::Item("minecraft:scaffolding", fraction(1, 4)),
    FuelRow::Item("minecraft:loom", fraction(3, 2)),
    FuelRow::Item("minecraft:barrel", fraction(3, 2)),
    FuelRow::Item("minecraft:cartography_table", fraction(3, 2)),
    FuelRow::Item("minecraft:fletching_table", fraction(3, 2)),
    FuelRow::Item("minecraft:smithing_table", fraction(3, 2)),
    FuelRow::Item("minecraft:composter", fraction(3, 2)),
    FuelRow::Item("minecraft:azalea", fraction(1, 2)),
    FuelRow::Item("minecraft:flowering_azalea", fraction(1, 2)),
    FuelRow::Item("minecraft:mangrove_roots", fraction(3, 2)),
    FuelRow::Item("minecraft:leaf_litter", fraction(1, 2)),
    FuelRow::RemoveTag("minecraft:non_flammable_wood"),
];
