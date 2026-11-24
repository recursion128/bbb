use super::*;

fn tick_health_loss(native: &mut NativeLevel, resource_id: &str, no_ai: bool) -> f32 {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = living_at(&mut store, resource_id, FEET);
    let mut body = store.detach(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut body.data else {
        panic!("living body");
    };
    data.set_no_ai(no_ai);
    let before = body.living().unwrap().health();
    assert!(native
        .with_level(|level| { crate::entity::general_living::tick(&mut body, &mut store, level) }));
    before - body.living().unwrap().health()
}

#[test]
fn water_sensitive_bodies_take_water_and_rain_damage_even_with_no_ai() {
    let mut water = sloping_pool("minecraft:water");
    let mut rain = rainy_level();
    for resource_id in [
        "minecraft:blaze",
        "minecraft:enderman",
        "minecraft:snow_golem",
        "minecraft:strider",
    ] {
        for no_ai in [false, true] {
            assert_eq!(
                tick_health_loss(&mut water, resource_id, no_ai),
                1.0,
                "{resource_id}"
            );
            assert_eq!(
                tick_health_loss(&mut rain, resource_id, no_ai),
                1.0,
                "{resource_id}"
            );
        }
    }
    assert_eq!(tick_health_loss(&mut water, "minecraft:pig", true), 0.0);
    assert_eq!(tick_health_loss(&mut rain, "minecraft:pig", true), 0.0);
}

#[test]
fn water_sensitive_bodies_are_protected_by_a_roof_or_dry_weather() {
    let mut sheltered = rainy_level();
    sheltered.set(pos(0, 68, 0), state_id("minecraft:stone"));
    assert!(!sheltered.with_level(|level| level.is_raining_at(pos(0, 67, 0))));
    let mut dry = NativeLevel::new();
    for resource_id in [
        "minecraft:blaze",
        "minecraft:enderman",
        "minecraft:snow_golem",
        "minecraft:strider",
    ] {
        assert_eq!(
            tick_health_loss(&mut sheltered, resource_id, true),
            0.0,
            "{resource_id}"
        );
        assert_eq!(
            tick_health_loss(&mut dry, resource_id, true),
            0.0,
            "{resource_id}"
        );
    }
}
