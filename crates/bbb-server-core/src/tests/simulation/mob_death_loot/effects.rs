use super::*;

#[test]
fn saved_active_effects_control_mob_death_drops() {
    let pool = |predicate, item| {
        serde_json::json!({
            "rolls":1,
            "conditions":[{"condition":"minecraft:entity_properties","entity":"this","predicate":predicate}],
            "entries":[{"type":"minecraft:item","name":item}]
        })
    };
    let table = serde_json::json!({"type":"minecraft:entity","pools":[
        pool(serde_json::json!({
            "effects":{"speed":{"amplifier":2,"duration":-1,"ambient":true,"visible":false}},
            "components":{"minecraft:custom_name":"Marked cow"},
            "predicates":{"minecraft:custom_data":{"seal":7},"minecraft:custom_name":{}}
        }), "minecraft:diamond"),
        pool(serde_json::json!({"effects":{"speed":{"amplifier":1}}}), "minecraft:coal"),
        pool(serde_json::json!({"effects":{"speed":{"visible":true}}}), "minecraft:iron_ingot"),
        pool(serde_json::json!({"effects":{"slowness":{}}}), "minecraft:emerald"),
        pool(serde_json::json!({"components":{"minecraft:custom_name":"Other cow"}}), "minecraft:gold_ingot"),
        pool(serde_json::json!({"predicates":{"minecraft:custom_data":{"seal":8},"minecraft:custom_name":{}}}), "minecraft:apple")
    ]});
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        "minecraft:entities/cow",
        &table,
    ));
    let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    load_record(
        &mut world,
        cow,
        vec![
            nbt_entry("CustomName", NbtSummaryValue::String("Marked cow".into())),
            nbt_entry(
                "data",
                NbtSummaryValue::Compound(vec![
                    nbt_entry("seal", NbtSummaryValue::Byte(7)),
                    nbt_entry("extra", NbtSummaryValue::Byte(1)),
                ]),
            ),
            nbt_entry(
                "active_effects",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    nbt_entry("id", NbtSummaryValue::String("minecraft:speed".into())),
                    nbt_entry("duration", NbtSummaryValue::Int(-1)),
                    nbt_entry("amplifier", NbtSummaryValue::Byte(2)),
                    nbt_entry("ambient", NbtSummaryValue::Byte(1)),
                    nbt_entry("show_particles", NbtSummaryValue::Byte(0)),
                    nbt_entry("show_icon", NbtSummaryValue::Byte(1)),
                    nbt_entry(
                        "hidden_effect",
                        NbtSummaryValue::Compound(vec![
                            nbt_entry("duration", NbtSummaryValue::Int(600)),
                            nbt_entry("amplifier", NbtSummaryValue::Byte(1)),
                        ]),
                    ),
                ])]),
            ),
        ],
    );
    kill_stored(&mut world, cow);
    assert_eq!(dropped_ids(&world), ["minecraft:diamond"]);
}
