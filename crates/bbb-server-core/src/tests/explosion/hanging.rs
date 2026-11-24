use super::*;
use crate::entity::{EntityData, ItemFrameData, LeashKnotData, PaintingData};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
};

const TYPES: [i32; 4] = [
    VANILLA_ENTITY_TYPE_PAINTING_ID,
    VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
];

fn spawn(world: &mut ServerWorld, kind: i32, invulnerable: bool, fixed: bool) -> i32 {
    let attached = BlockPos { x: 2, y: 80, z: 0 };
    let data = match kind {
        VANILLA_ENTITY_TYPE_PAINTING_ID => EntityData::Painting(PaintingData::new(
            attached,
            Direction::North,
            crate::entity::hanging::PaintingVariant::vanilla("minecraft:kebab"),
        )),
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => EntityData::LeashKnot(LeashKnotData::new(attached)),
        _ => {
            let frame = ItemFrameData::new(attached, Direction::North).with_saved_fields(
                plain_stack("minecraft:diamond", 1),
                3,
                1.0,
                fixed,
                false,
            );
            if kind == VANILLA_ENTITY_TYPE_ITEM_FRAME_ID {
                EntityData::ItemFrame(frame)
            } else {
                EntityData::GlowItemFrame(frame)
            }
        }
    };
    let (id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        kind,
        Vec3d {
            x: 2.5,
            y: 80.5,
            z: if kind == VANILLA_ENTITY_TYPE_LEASH_KNOT_ID {
                0.5
            } else {
                0.96875
            },
        },
        data,
    );
    let entity = world.stored_entity_mut_for_test(OVERWORLD, id).unwrap();
    crate::entity::hanging::apply_direction(entity);
    entity.set_invulnerable(invulnerable);
    entity.set_needs_sync(false);
    world.track_stored_chunk_for_test(OVERWORLD, (0, 0));
    world.drain_entity_lifecycle_events_for_test(OVERWORLD);
    id
}

fn request(in_water: bool, affects_blocks: bool) -> PendingExplosion {
    let center = Vec3d {
        x: 0.5,
        y: 80.5,
        z: 0.5,
    };
    PendingExplosion {
        source: Some(ExplosionSource {
            direct: ExplosionSourceEntity {
                custom_name: None,
                entity_id: i32::MAX,
                entity_resource_id: if affects_blocks {
                    "minecraft:tnt"
                } else {
                    "minecraft:wind_charge"
                }
                .into(),
                uuid: Uuid::from_u128(777),
                position: center,
                kind: ExplosionSourceEntityKind::Other,
            },
            indirect: None,
            causing: None,
            in_water,
            wind_charge: !affects_blocks,
        }),
        center,
        radius: 4.0,
        fire: false,
        interaction: if affects_blocks {
            ExplosionInteraction::Block
        } else {
            ExplosionInteraction::None
        },
        damage_source: None,
        damage_profile: ExplosionDamageProfile::Default,
        presentation: ExplosionPresentation::Default,
    }
}

fn assert_drops(world: &ServerWorld, kind: i32) {
    let mut actual: Vec<_> = world
        .stored_items_for_test(OVERWORLD)
        .into_iter()
        .map(|(_, _, resource, count, _, _)| (resource, count))
        .collect();
    actual.sort();
    let mut expected = match kind {
        VANILLA_ENTITY_TYPE_PAINTING_ID => vec![("minecraft:painting".to_string(), 1)],
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID => vec![
            ("minecraft:item_frame".to_string(), 1),
            ("minecraft:diamond".to_string(), 1),
        ],
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID => vec![
            ("minecraft:glow_item_frame".to_string(), 1),
            ("minecraft:diamond".to_string(), 1),
        ],
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => Vec::new(),
        _ => unreachable!(),
    };
    expected.sort();
    assert_eq!(actual, expected);
}

fn assert_death_events(world: &ServerWorld, id: i32, kind: i32) {
    let events: Vec<_> = world
        .positioned_game_events_for_test(OVERWORLD)
        .into_iter()
        .map(|event| event.recorded)
        .filter(|event| {
            matches!(
                event.game_event,
                VanillaGameEvent::ENTITY_DIE
                    | VanillaGameEvent::BLOCK_CHANGE
                    | VanillaGameEvent::ENTITY_DAMAGE
            )
        })
        .map(|event| (event.game_event, event.source_entity_id))
        .collect();
    let mut expected = vec![(VanillaGameEvent::ENTITY_DIE, Some(id))];
    if matches!(
        kind,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID | VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    ) {
        expected.push((VanillaGameEvent::BLOCK_CHANGE, None));
    }
    assert_eq!(events, expected);
}

#[test]
fn invulnerable_hurt_refusal_still_pushes_and_breaks_all_hanging_types() {
    for kind in TYPES {
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let id = spawn(&mut world, kind, true, false);
        world.explode(OVERWORLD, request(false, true), &mut Vec::new());
        assert!(
            world.stored_entity_for_test(OVERWORLD, id).is_none(),
            "type {kind} survives a nonzero push"
        );
        assert_drops(&world, kind);
        assert_death_events(&world, id, kind);
    }
}

#[test]
fn fixed_frames_preserve_body_velocity_sync_and_contents() {
    for kind in [
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
    ] {
        for invulnerable in [false, true] {
            let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
            let id = spawn(&mut world, kind, invulnerable, true);
            world
                .stored_entity_mut_for_test(OVERWORLD, id)
                .unwrap()
                .set_delta_movement(Vec3d {
                    x: 0.125,
                    y: -0.25,
                    z: 0.5,
                });
            let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            let position = body.position();
            let bounds = body.bounding_box();
            let delta = body.delta_movement();
            world.explode(OVERWORLD, request(false, true), &mut Vec::new());
            let body = world
                .stored_entity_for_test(OVERWORLD, id)
                .expect("Fixed frame stays alive");
            assert_eq!(body.position(), position);
            assert_eq!(body.bounding_box(), bounds);
            assert_eq!(body.delta_movement(), delta);
            assert!(!body.needs_sync());
            let (EntityData::ItemFrame(frame) | EntityData::GlowItemFrame(frame)) = body.data()
            else {
                unreachable!()
            };
            assert!(frame.fixed());
            assert_eq!(frame.rotation(), 3);
            assert_eq!(frame.item(), plain_stack("minecraft:diamond", 1).as_ref());
            assert!(world.stored_items_for_test(OVERWORLD).is_empty());
            assert!(world
                .drain_entity_lifecycle_events_for_test(OVERWORLD)
                .is_empty());
            assert!(world
                .positioned_game_events_for_test(OVERWORLD)
                .iter()
                .all(|event| event.recorded.game_event == VanillaGameEvent::EXPLODE));
        }
    }
}

#[test]
fn underwater_and_non_blocklike_explosions_skip_hurt_and_push_for_all_hanging() {
    for kind in TYPES {
        for (in_water, affects_blocks) in [(true, true), (false, false)] {
            let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
            let id = spawn(&mut world, kind, false, false);
            let before = world.stored_entity_for_test(OVERWORLD, id).unwrap().clone();
            let saved_before = world
                .stored_entity_command_data_for_test(OVERWORLD, id)
                .unwrap();
            world.explode(
                OVERWORLD,
                request(in_water, affects_blocks),
                &mut Vec::new(),
            );
            let body = world.stored_entity_for_test(OVERWORLD, id).unwrap();
            assert_eq!(body.position(), before.position());
            assert_eq!(body.delta_movement(), before.delta_movement());
            assert!(!body.needs_sync());
            assert_eq!(
                world
                    .stored_entity_command_data_for_test(OVERWORLD, id)
                    .unwrap(),
                saved_before
            );
            assert!(world.stored_items_for_test(OVERWORLD).is_empty());
            assert!(world
                .drain_entity_lifecycle_events_for_test(OVERWORLD)
                .is_empty());
            assert!(world
                .positioned_game_events_for_test(OVERWORLD)
                .iter()
                .all(|event| event.recorded.game_event == VanillaGameEvent::EXPLODE));
        }
    }
}
