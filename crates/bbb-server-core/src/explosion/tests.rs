use super::*;

const ORACLE_CENTER: Vec3d = Vec3d {
    x: 0.5,
    y: 64.061_25,
    z: 0.5,
};

#[test]
fn block_resistance_uses_builtin_facts() {
    assert_eq!(
        vanilla_block_explosion_resistance(
            bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                "minecraft:stone",
                &Default::default()
            )
            .unwrap()
        ),
        Some(6.0)
    );
    assert_eq!(vanilla_block_explosion_resistance(0), Some(0.0));
    assert_eq!(vanilla_block_explosion_resistance(-1), None);
    assert_eq!(vanilla_block_explosion_resistance(i32::MAX), None);
}

#[test]
fn fluid_explosion_resistance_matches_vanilla() {
    // `FluidState.getExplosionResistance()`: 100 for water and lava, 0 for the
    // empty fluid. Water and lava source states from the generated catalog.
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &[("level".to_string(), "0".to_string())]
            .into_iter()
            .collect(),
    )
    .expect("water source state");
    let lava = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:lava",
        &[("level".to_string(), "0".to_string())]
            .into_iter()
            .collect(),
    )
    .expect("lava source state");
    let stone = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone",
        &Default::default(),
    )
    .expect("stone state");
    assert_eq!(fluid_explosion_resistance(water), 100.0);
    assert_eq!(fluid_explosion_resistance(lava), 100.0);
    assert_eq!(fluid_explosion_resistance(stone), 0.0);
}

#[test]
fn is_small_follows_radius_and_block_interaction() {
    assert!(is_small(1.5, BlockInteraction::Destroy));
    assert!(!is_small(4.0, BlockInteraction::Destroy));
    // A KEEP interaction is always small, whatever the radius.
    assert!(is_small(8.0, BlockInteraction::Keep));
}

#[test]
fn tnt_interaction_maps_to_destroy_with_decay_under_default_rules() {
    let rules = ExplosionGameRules::default();
    assert_eq!(
        ExplosionInteraction::Tnt.block_interaction(rules),
        BlockInteraction::DestroyWithDecay
    );
    let no_decay = ExplosionGameRules {
        tnt_explosion_drop_decay: false,
        ..rules
    };
    assert_eq!(
        ExplosionInteraction::Tnt.block_interaction(no_decay),
        BlockInteraction::Destroy
    );
    // MOB with mobGriefing off keeps blocks.
    let no_grief = ExplosionGameRules {
        mob_griefing: false,
        ..rules
    };
    assert_eq!(
        ExplosionInteraction::Mob.block_interaction(no_grief),
        BlockInteraction::Keep
    );
}

#[test]
fn hurt_entities_box_and_wind_charge_griefing() {
    assert!(hurt_entities_box(ORACLE_CENTER, 0.0).is_none());
    let bb = hurt_entities_box(ORACLE_CENTER, 4.0).expect("a radius-four box");
    // floor(0.5 - 8 - 1) = -9, floor(0.5 + 8 + 1) = 9.
    assert_eq!(bb.min_x, -9.0);
    assert_eq!(bb.max_x, 9.0);

    // `shouldAffectBlocklikeEntities`: with mobGriefing on, everything but a
    // wind charge affects block-like entities.
    let tnt = Some(ExplosionSource {
        direct: ExplosionSourceEntity {
            custom_name: None,
            entity_id: 1,
            entity_resource_id: "minecraft:tnt".to_string(),
            uuid: uuid::Uuid::from_u128(1),
            position: ORACLE_CENTER,
            kind: ExplosionSourceEntityKind::Other,
        },
        indirect: None,
        causing: None,
        in_water: false,
        wind_charge: false,
    });
    assert!(should_affect_blocklike_entities(
        true,
        tnt.as_ref(),
        BlockInteraction::DestroyWithDecay
    ));
    let wind = Some(ExplosionSource {
        direct: ExplosionSourceEntity {
            custom_name: None,
            entity_id: 2,
            entity_resource_id: "minecraft:wind_charge".to_string(),
            uuid: uuid::Uuid::from_u128(2),
            position: ORACLE_CENTER,
            kind: ExplosionSourceEntityKind::Other,
        },
        indirect: None,
        causing: None,
        in_water: false,
        wind_charge: true,
    });
    assert!(!should_affect_blocklike_entities(
        true,
        wind.as_ref(),
        BlockInteraction::DestroyWithDecay
    ));
}

#[test]
fn explosion_source_keeps_mob_and_living_non_player_as_orthogonal_facts() {
    let source = |kind| ExplosionSourceEntity {
        custom_name: None,
        entity_id: 1,
        entity_resource_id: "minecraft:test".to_string(),
        uuid: uuid::Uuid::from_u128(1),
        position: ORACLE_CENTER,
        kind,
    };

    assert!(source(ExplosionSourceEntityKind::Mob).is_mob());
    assert!(source(ExplosionSourceEntityKind::Mob).is_living_non_player());
    assert!(!source(ExplosionSourceEntityKind::LivingOther).is_mob());
    assert!(source(ExplosionSourceEntityKind::LivingOther).is_living_non_player());
    assert!(!source(ExplosionSourceEntityKind::Other).is_living_non_player());
}
