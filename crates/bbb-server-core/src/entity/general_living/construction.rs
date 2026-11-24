use bbb_protocol::nbt_summary::entry;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

pub(super) fn extend_opaque_constructor_defaults(
    resource_id: &str,
    opaque_fields: &mut Vec<NbtSummaryEntry>,
) {
    match resource_id {
        "minecraft:bogged" => opaque_fields.push(entry("sheared", NbtSummaryValue::Byte(0))),
        "minecraft:ender_dragon" => opaque_fields.extend([
            entry("DragonPhase", NbtSummaryValue::Int(10)),
            entry("DragonDeathTime", NbtSummaryValue::Int(0)),
        ]),
        "minecraft:iron_golem" => {
            opaque_fields.push(entry("PlayerCreated", NbtSummaryValue::Byte(0)))
        }
        "minecraft:mannequin" => opaque_fields.extend([
            entry("profile", NbtSummaryValue::Compound(Vec::new())),
            entry("hidden_layers", NbtSummaryValue::List(Vec::new())),
            entry("main_hand", NbtSummaryValue::string("right")),
            entry("pose", NbtSummaryValue::string("standing")),
            entry("immovable", NbtSummaryValue::Byte(0)),
        ]),
        "minecraft:mooshroom" => opaque_fields.push(entry("Type", NbtSummaryValue::string("red"))),
        "minecraft:snow_golem" => opaque_fields.push(entry("Pumpkin", NbtSummaryValue::Byte(1))),
        _ => {}
    }
}
