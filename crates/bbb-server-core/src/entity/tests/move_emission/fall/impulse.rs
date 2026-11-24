use super::*;
use crate::entity::living::FallImpulseContext;

fn set_context(entity: &mut Entity, grace: i32, height: Option<f64>) -> FallImpulseContext {
    let context = FallImpulseContext {
        reset_grace_time: grace,
        impact_position: height.map(|height| Vec3d {
            x: 100.0,
            y: entity.position().y + height,
            z: -100.0,
        }),
    };
    entity.living_mut().unwrap().fall_impulse = context;
    context
}

#[test]
fn stored_impulse_fall_reduces_damage_below_the_impact_height() {
    for stand in [false] {
        for grace in [0, 4] {
            for height in [-2.0, 2.0] {
                let (mut store, mut level, mut body) = fall_provider_body(stand);
                set_context(&mut body, grace, Some(height));
                let health = body.living().unwrap().health();
                provider_fall(
                    &mut body,
                    &mut store,
                    &mut level,
                    8.0,
                    1.0,
                    "minecraft:fall",
                );
                assert_eq!(body.living().unwrap().health(), health);

                assert!(game_events(&level).is_empty());
            }
            let (mut store, mut level, mut body) = fall_provider_body(stand);
            set_context(&mut body, grace, None);
            provider_fall(
                &mut body,
                &mut store,
                &mut level,
                2.0,
                1.0,
                "minecraft:fall",
            );
        }
    }
}

#[test]
fn stored_impulse_fall_positive_damage_preserves_health_when_hurt_is_refused() {
    for stand in [false, true] {
        for grace in [4] {
            for invulnerable in [false, true] {
                let (mut store, mut level, mut body) = fall_provider_body(stand);
                set_context(&mut body, grace, Some(8.0));
                body.set_invulnerable(invulnerable);
                let health = body.living().unwrap().health();
                provider_fall(
                    &mut body,
                    &mut store,
                    &mut level,
                    12.0,
                    1.0,
                    "minecraft:fall",
                );

                assert_eq!(
                    body.living().unwrap().health(),
                    health - if stand || invulnerable { 0.0 } else { 5.0 }
                );
                assert!(!queued_sounds(&mut store).is_empty());
            }
        }
    }
}

#[test]
fn stored_impulse_fall_effective_distance_is_handed_to_child_with_its_own_impact() {
    for stand in [false] {
        for (parent_height, child_height, parent_damage, child_damage) in [
            (5.0, 10.0, 2.0, 2.0),
            (8.0, 4.0, 5.0, 1.0),
            (2.0, 10.0, 0.0, 0.0),
        ] {
            let (store, mut level, body) = fall_provider_body(stand);
            let (mut store, mut body) = with_fall_passenger(store, body);
            let child = store.passengers(body.id())[0];
            set_context(&mut body, 4, Some(parent_height));
            set_context(store.get_mut(child).unwrap(), -3, Some(child_height));
            let health = body.living().unwrap().health();
            provider_fall(
                &mut body,
                &mut store,
                &mut level,
                12.0,
                1.0,
                "minecraft:fall",
            );
            assert_eq!(
                body.living().unwrap().health(),
                health - if stand { 0.0 } else { parent_damage }
            );
            assert_eq!(
                store.get(child).unwrap().living().unwrap().health(),
                20.0 - child_damage
            );
        }
    }
}

#[test]
fn stored_impulse_fall_horse_and_llama_use_the_original_distance() {
    for name in ["minecraft:horse", "minecraft:llama"] {
        {
            let grace = 4;
            let (store, mut level, mut body) = fall_subtype_body(name);
            standard_fall_attributes(&mut body);
            let (mut store, mut body) = with_fall_passenger(store, body);
            let child = store.passengers(body.id())[0];
            set_context(&mut body, grace, Some(-2.0));
            set_context(store.get_mut(child).unwrap(), 4, Some(2.0));
            let health = body.living().unwrap().health();
            provider_fall(
                &mut body,
                &mut store,
                &mut level,
                8.0,
                1.0,
                "minecraft:fall",
            );

            assert_eq!(body.living().unwrap().health(), health - 5.0);
            assert_eq!(store.get(child).unwrap().living().unwrap().health(), 20.0);
        }
    }
}

#[test]
fn stored_impulse_fall_breeze_prefix_and_creeper_suffix_keep_original_distance() {
    for name in ["minecraft:breeze", "minecraft:creeper"] {
        let (mut store, mut level, mut body) = fall_subtype_body(name);
        set_context(&mut body, 4, Some(0.0));
        provider_fall(
            &mut body,
            &mut store,
            &mut level,
            5.0,
            1.0,
            "minecraft:fall",
        );

        if name == "minecraft:breeze" {
            let sounds = queued_sounds(&mut store);
            assert_eq!(
                sounds
                    .iter()
                    .find(|s| s.registry_id == sound_id("minecraft:entity.breeze.land"))
                    .unwrap()
                    .registry_id,
                sound_id("minecraft:entity.breeze.land")
            );
        } else {
            let EntityData::GeneralLiving(data) = body.data() else {
                unreachable!()
            };
            assert_eq!(data.creeper_retained().unwrap().swell(), 7);
        }
    }
}
