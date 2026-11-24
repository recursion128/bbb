use bbb_client_world::{RegistryPacketEntry, WorldStore, DIMENSION_TYPE_REGISTRY};

pub(super) fn install_piglins_zombify_dimension(world: &mut WorldStore, value: bool) {
    let mut out = vec![10];
    for (name, value) in [
        ("has_skylight", true),
        ("has_ceiling", false),
        ("has_ender_dragon_fight", false),
    ] {
        out.push(1);
        write_test_nbt_string(&mut out, name);
        out.push(u8::from(value));
    }
    out.push(6);
    write_test_nbt_string(&mut out, "coordinate_scale");
    out.extend(1.0_f64.to_be_bytes());
    for (name, value) in [
        ("min_y", -64_i32),
        ("height", 384),
        ("logical_height", 384),
        ("monster_spawn_light_level", 0),
        ("monster_spawn_block_light_limit", 0),
    ] {
        out.push(3);
        write_test_nbt_string(&mut out, name);
        out.extend(value.to_be_bytes());
    }
    out.push(8);
    write_test_nbt_string(&mut out, "infiniburn");
    write_test_nbt_string(&mut out, "#minecraft:infiniburn_overworld");
    out.push(5);
    write_test_nbt_string(&mut out, "ambient_light");
    out.extend(0.0_f32.to_be_bytes());
    out.push(10);
    write_test_nbt_string(&mut out, "attributes");
    out.push(1);
    write_test_nbt_string(&mut out, "minecraft:gameplay/piglins_zombify");
    out.push(i8::from(value) as u8);
    out.push(0);
    out.push(0);
    world.record_registry_entries(
        DIMENSION_TYPE_REGISTRY,
        0,
        vec![RegistryPacketEntry::with_raw_data("example:custom", out)],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
}

pub(super) fn crossbow_charge_time_enchantment_nbt(seconds: f32) -> Vec<u8> {
    let mut out = vec![10];
    for (name, value) in [
        ("description", "Patient Loading"),
        ("supported_items", "minecraft:crossbow"),
    ] {
        out.push(8);
        write_test_nbt_string(&mut out, name);
        write_test_nbt_string(&mut out, value);
    }
    for (name, value) in [("weight", 1_i32), ("max_level", 3), ("anvil_cost", 0)] {
        out.push(3);
        write_test_nbt_string(&mut out, name);
        out.extend(value.to_be_bytes());
    }
    for name in ["min_cost", "max_cost"] {
        out.push(10);
        write_test_nbt_string(&mut out, name);
        for field in ["base", "per_level_above_first"] {
            out.push(3);
            write_test_nbt_string(&mut out, field);
            out.extend(0_i32.to_be_bytes());
        }
        out.push(0);
    }
    out.push(9);
    write_test_nbt_string(&mut out, "slots");
    out.push(8);
    out.extend(1_i32.to_be_bytes());
    write_test_nbt_string(&mut out, "mainhand");
    out.push(10);
    write_test_nbt_string(&mut out, "effects");
    out.push(10);
    write_test_nbt_string(&mut out, "minecraft:crossbow_charge_time");
    out.push(8);
    write_test_nbt_string(&mut out, "type");
    write_test_nbt_string(&mut out, "minecraft:set");
    out.push(5);
    write_test_nbt_string(&mut out, "value");
    out.extend(seconds.to_be_bytes());
    out.extend([0, 0, 0]);
    out
}

fn write_test_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
