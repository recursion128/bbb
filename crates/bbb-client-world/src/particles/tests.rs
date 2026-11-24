use super::*;

/// Air everywhere above a solid floor at `y = floor`.
struct FloorLevel {
    floor: f64,
}

impl ParticleLevel for FloorLevel {
    fn entity_position(&self, _id: i32) -> Option<[f64; 3]> {
        None
    }

    fn clip_movement(
        &self,
        position: [f64; 3],
        movement: [f64; 3],
        _half_width: f64,
        _height: f64,
    ) -> [f64; 3] {
        let mut clipped = movement;
        clipped[1] = clipped[1].max(self.floor - position[1]);
        clipped
    }

    fn block_fluid_surface(&self, _position: [f64; 3]) -> ParticleBlockFluidSurfaceSample {
        ParticleBlockFluidSurfaceSample::default()
    }
}

fn count(engine: &ParticleEngine, kind: ParticleKind) -> usize {
    engine
        .instances()
        .filter(|instance| instance.kind == kind)
        .count()
}

fn tick(engine: &mut ParticleEngine, level: &FloorLevel) {
    engine.advance(1, level, ParticleTickContext::default());
}

/// `DripParticle.FallAndLandParticle.postMoveUpdate`: a falling water drop that
/// hits the ground is replaced by a `splash` particle.
#[test]
fn falling_water_lands_as_splash() {
    let level = FloorLevel { floor: 64.0 };
    let mut engine = ParticleEngine::default();
    engine.spawn(ParticleSpawn::new(
        ParticleKind::FALLING_WATER,
        [0.5, 64.6, 0.5],
        [0.0; 3],
    ));
    tick(&mut engine, &level);
    assert_eq!(count(&engine, ParticleKind::FALLING_WATER), 1);
    for _ in 0..40 {
        tick(&mut engine, &level);
        if count(&engine, ParticleKind::FALLING_WATER) == 0 {
            break;
        }
    }
    assert_eq!(count(&engine, ParticleKind::FALLING_WATER), 0);
    assert_eq!(count(&engine, ParticleKind::SPLASH), 1);
}

/// `FireworkParticles.SparkParticle.tick`: a trail spark leaves a half-life
/// copy of itself every other tick during the first half of its life.
#[test]
fn firework_trail_spark_leaves_half_life_children() {
    let level = FloorLevel { floor: -1000.0 };
    let mut engine = ParticleEngine::default();
    engine.spawn(
        ParticleSpawn::new(ParticleKind::FIREWORK, [0.0, 100.0, 0.0], [0.0; 3]).with_params(
            ParticleParams::FireworkSpark {
                color: [1.0, 0.0, 0.0, 1.0],
                fade_to: None,
                trail: true,
                twinkle: false,
                half_lifetime: false,
            },
        ),
    );
    tick(&mut engine, &level);
    let parent_lifetime = engine.instances().next().unwrap().lifetime_ticks;
    for _ in 0..4 {
        tick(&mut engine, &level);
    }
    let children: Vec<_> = engine.instances().skip(1).collect();
    assert_eq!(children.len(), 2);
    for child in children {
        assert_eq!(child.kind, ParticleKind::FIREWORK);
        assert!(child.age_ticks >= child.lifetime_ticks / 2);
    }
    assert!(parent_lifetime >= 8);
}

fn item_pickup_spawn() -> ParticleSpawn {
    let pickup = ItemPickupParticle {
        source_entity_id: 1,
        item: Some((1, 1, 0)),
        component_patch: None,
        age_ticks: 0.0,
        bob_offset: 0.0,
        light: [1.0, 1.0],
        experience_orb_icon: None,
        projectile_model: None,
        y_rot: 0.0,
        x_rot: 0.0,
        target: [0.0, 1.0, 0.0],
        target_source: ParticleEntityTargetSource {
            entity_id: 2,
            y_offset: 0.8,
        },
    };
    ParticleSpawn::new(ParticleKind::ITEM_PICKUP, [2.0, 0.0, 0.0], [0.0; 3])
        .with_params(ParticleParams::ItemPickup(Box::new(pickup)))
}

/// `ItemPickupParticle.LIFE_TIME = 3`.
#[test]
fn item_pickup_lives_three_ticks() {
    let level = FloorLevel { floor: -1000.0 };
    let mut engine = ParticleEngine::default();
    engine.spawn(item_pickup_spawn());
    tick(&mut engine, &level);
    tick(&mut engine, &level);
    tick(&mut engine, &level);
    assert_eq!(engine.len(), 1);
    tick(&mut engine, &level);
    assert!(engine.is_empty());
}

#[test]
fn full_quad_group_evicts_its_oldest_quad_and_keeps_the_pickup() {
    let level = FloorLevel { floor: -1000.0 };
    let mut engine = ParticleEngine::default();
    engine.spawn(ParticleSpawn::new(ParticleKind::SMOKE, [0.0; 3], [0.0; 3]));
    tick(&mut engine, &level);
    engine.spawn(item_pickup_spawn());
    engine.spawn(ParticleSpawn::new(ParticleKind::FLAME, [0.0; 3], [0.0; 3]));
    for _ in 0..MAX_PARTICLES_PER_GROUP {
        engine.spawn(ParticleSpawn::new(
            ParticleKind::BLOCK_MARKER,
            [0.0; 3],
            [0.0; 3],
        ));
    }
    assert_eq!(engine.pending_len(), MAX_PARTICLES_PER_GROUP + 1);
    tick(&mut engine, &level);

    assert_eq!(count(&engine, ParticleKind::SMOKE), 0);
    assert_eq!(count(&engine, ParticleKind::FLAME), 0);
    assert_eq!(
        count(&engine, ParticleKind::BLOCK_MARKER),
        MAX_PARTICLES_PER_GROUP
    );
    let pickup = engine
        .instances_in_group(ParticleRenderGroup::ItemPickup)
        .next()
        .unwrap();
    assert_eq!(pickup.item_pickup_position(0.0), Some([2.0, 0.0, 0.0]));
}

#[test]
fn spore_limit_rejects_a_burst_before_it_can_evict_other_quads() {
    let level = FloorLevel { floor: -1000.0 };
    let mut engine = ParticleEngine::default();
    let spore = || ParticleSpawn::new(ParticleKind::SPORE_BLOSSOM_AIR, [0.0; 3], [0.0; 3]);
    engine.spawn(ParticleSpawn::new(ParticleKind::SMOKE, [0.0; 3], [0.0; 3]));
    for _ in 0..MAX_PARTICLES_PER_GROUP {
        engine.spawn(spore());
    }
    let limit = ParticleLimitDescriptor::SporeBlossom.limit();
    assert_eq!(engine.pending_len(), limit + 1);
    tick(&mut engine, &level);
    assert_eq!(count(&engine, ParticleKind::SMOKE), 1);
    assert_eq!(count(&engine, ParticleKind::SPORE_BLOSSOM_AIR), limit);
    engine.spawn(spore());
    assert_eq!(engine.pending_len(), 0);
}

/// A resource reload swaps the sprite table and drops every particle.
#[test]
fn new_sprite_table_clears_particles() {
    let level = FloorLevel { floor: 0.0 };
    let mut engine = ParticleEngine::default();
    let drip = || ParticleSpawn::new(ParticleKind::FALLING_WATER, [0.5, 64.6, 0.5], [0.0; 3]);
    engine.spawn(drip());
    tick(&mut engine, &level);
    engine.spawn(drip());
    engine.set_sprites(ParticleSpriteTable::default());
    tick(&mut engine, &level);
    assert!(engine.is_empty());
}
