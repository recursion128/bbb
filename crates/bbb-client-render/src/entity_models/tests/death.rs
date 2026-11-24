use super::*;

#[test]
fn death_fall_factor_matches_vanilla_setup_rotations() {
    // Vanilla LivingEntityRenderer.setupRotations:
    //   fall = (deathTime - 1) / 20 * 1.6; fall = sqrt(fall); if fall > 1 fall = 1.
    // state.deathTime is always >= 1 while dying (entity.deathTime >= 1 + partial).
    assert_eq!(death_fall_factor(1.0), 0.0);
    assert!((death_fall_factor(11.0) - (0.8_f32).sqrt()).abs() < 1e-6);
    // deathTime 20: (19/20)*1.6 = 1.52, sqrt > 1, clamped to 1.0.
    assert_eq!(death_fall_factor(20.0), 1.0);
    // Beyond the clamp point it stays at 1.0.
    assert_eq!(death_fall_factor(40.0), 1.0);
    // The factor is monotonic across the unclamped range.
    assert!(death_fall_factor(5.0) < death_fall_factor(10.0));
    assert!(death_fall_factor(10.0) < death_fall_factor(15.0));
}

#[test]
fn entity_flip_degrees_match_vanilla_renderer_overrides() {
    // SpiderRenderer (and the cave spider that extends it), EndermiteRenderer,
    // and SilverfishRenderer override getFlipDegrees to 180; other living
    // renderers keep the base 90.
    assert_eq!(entity_flip_degrees(EntityModelKind::Spider), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::CaveSpider), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Endermite), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Silverfish), 180.0);
    assert_eq!(entity_flip_degrees(EntityModelKind::Creeper), 90.0);
    assert_eq!(
        entity_flip_degrees(EntityModelKind::Zombie { baby: false }),
        90.0
    );
    assert_eq!(entity_flip_degrees(EntityModelKind::Ravager), 90.0);
}
