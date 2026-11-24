//! Template mob NBT fixtures shared with the chunk-writer tests above this crate.

use bbb_protocol::NbtSummaryValue;

use super::entry;

pub fn villager_nbt() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("id", NbtSummaryValue::string("minecraft:villager")),
        entry("Health", NbtSummaryValue::Float(20.0_f32.to_bits())),
        entry("PersistenceRequired", NbtSummaryValue::Byte(1)),
        entry("CanPickUpLoot", NbtSummaryValue::Byte(1)),
        entry("Age", NbtSummaryValue::Int(0)),
        entry("ForcedAge", NbtSummaryValue::Int(0)),
        entry("Xp", NbtSummaryValue::Int(0)),
        entry(
            "VillagerData",
            NbtSummaryValue::Compound(vec![
                entry("type", NbtSummaryValue::string("minecraft:plains")),
                entry("profession", NbtSummaryValue::string("minecraft:cleric")),
                entry("level", NbtSummaryValue::Int(1)),
            ]),
        ),
        entry(
            "Offers",
            NbtSummaryValue::Compound(vec![entry(
                "Recipes",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    entry("maxUses", NbtSummaryValue::Int(7)),
                    entry("uses", NbtSummaryValue::Int(0)),
                    entry("rewardExp", NbtSummaryValue::Byte(1)),
                    entry(
                        "buy",
                        NbtSummaryValue::Compound(vec![
                            entry("id", NbtSummaryValue::string("minecraft:rotten_flesh")),
                            entry("count", NbtSummaryValue::Int(36)),
                        ]),
                    ),
                    entry(
                        "sell",
                        NbtSummaryValue::Compound(vec![
                            entry("id", NbtSummaryValue::string("minecraft:emerald")),
                            entry("count", NbtSummaryValue::Int(1)),
                        ]),
                    ),
                ])]),
            )]),
        ),
    ])
}
