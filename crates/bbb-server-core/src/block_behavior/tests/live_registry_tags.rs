use super::native_world::NativeWorld;
use super::*;

const POS: BlockPos = BlockPos { x: 3, y: 70, z: -2 };

fn snapshot(tags: &[(&str, &str, &[&str])]) -> RegistryTagSnapshot {
    RegistryTagSnapshot::new(tags.iter().map(|(registry, tag, members)| {
        (
            (*registry).to_string(),
            (*tag).to_string(),
            members.iter().map(|member| (*member).to_string()).collect(),
        )
    }))
}

#[test]
fn snow_layer_shape_update_uses_replaced_support_tag() {
    let snow = with_property(default_state("minecraft:snow"), "layers", "1").unwrap();
    let mut world = NativeWorld::new();
    world.set(below(POS), stone());
    world.set(POS, snow);
    world
        .world
        .install_level_registry_tags(std::sync::Arc::new(snapshot(&[(
            "minecraft:block",
            "minecraft:cannot_support_snow_layer",
            &[],
        )])));
    world.shape_update(POS, Direction::Down);
    assert_eq!(world.state(POS), snow);

    world
        .world
        .install_level_registry_tags(std::sync::Arc::new(snapshot(&[(
            "minecraft:block",
            "minecraft:cannot_support_snow_layer",
            &["minecraft:stone"],
        )])));
    world.shape_update(POS, Direction::Down);
    assert_eq!(world.state(POS), air());
}

#[test]
fn waterlogged_small_dripleaf_survival_observes_the_drained_cell() {
    let waterlogged = with_property(
        default_state("minecraft:small_dripleaf"),
        "waterlogged",
        "true",
    )
    .unwrap();
    let mut world = NativeWorld::new();
    world.set(POS, waterlogged);
    world.set(below(POS), default_state("minecraft:dirt"));
    world
        .world
        .install_level_registry_tags(std::sync::Arc::new(snapshot(&[
            ("minecraft:block", "minecraft:supports_small_dripleaf", &[]),
            (
                "minecraft:block",
                "minecraft:supports_vegetation",
                &["minecraft:dirt"],
            ),
        ])));

    assert!(
        world.with_level(|level| crate::block_behavior::bucket_pickup::pickup_block(
            level,
            POS,
            waterlogged,
            false
        )
        .is_some())
    );
    assert_eq!(world.state(POS), air());
}

#[test]
fn soul_fire_on_place_needs_a_soul_fire_base_block() {
    let soul_fire = default_state("minecraft:soul_fire");
    let mut world = NativeWorld::new();
    world.set(below(POS), stone());
    world
        .world
        .install_level_registry_tags(std::sync::Arc::new(snapshot(&[(
            "minecraft:block",
            "minecraft:soul_fire_base_blocks",
            &["minecraft:stone"],
        )])));
    world.world.set_block(
        bbb_server_types::save::Java26BuiltinDimension::Overworld,
        POS,
        soul_fire,
        18,
    );
    assert_eq!(world.state(POS), soul_fire);

    world
        .world
        .install_level_registry_tags(std::sync::Arc::new(snapshot(&[(
            "minecraft:block",
            "minecraft:soul_fire_base_blocks",
            &[],
        )])));
    world.with_level(|level| {
        crate::block_behavior::fire::soul_fire_on_place(level, POS, soul_fire, air())
    });
    assert_eq!(world.state(POS), air());
}
