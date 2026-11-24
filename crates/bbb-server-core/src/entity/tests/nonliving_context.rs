use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::level_event::VanillaGameEvent;

#[derive(Clone, Copy, Debug)]
enum Body {
    Item,
    Orb,
    Tnt,
}

impl Body {
    fn spawn(self, store: &mut EntityStore) -> i32 {
        let (kind, data) =
            match self {
                Self::Item => {
                    (
                        VANILLA_ENTITY_TYPE_ITEM_ID,
                        EntityData::Item(
                            ItemEntityData::new(stack_of("minecraft:dirt", 1), 0.0)
                                .with_saved_fields(DEFAULT_HEALTH, 10, 20, None, None),
                        ),
                    )
                }
                Self::Orb => (
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
                    EntityData::ExperienceOrb(ExperienceOrbData::from_saved(5, 10, 3, 1)),
                ),
                Self::Tnt => (
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID,
                    EntityData::Tnt(crate::entity::TntData::with_saved_fields(
                        2,
                        state_id("minecraft:tnt"),
                        None,
                        4.0,
                    )),
                ),
            };
        let id = store.spawn(
            kind,
            Vec3d {
                x: 0.5,
                y: 65.25,
                z: 0.5,
            },
            data,
        );
        let entity = store.get_mut(id).unwrap();
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -2.0,
            z: 0.0,
        });
        entity.fall_distance = 2.0;
        entity.portal_cooldown = 3;
        id
    }

    fn assert_suffix(self, entity: &crate::entity::Entity) {
        match (self, entity.data()) {
            (Self::Item, EntityData::Item(item)) => {
                assert_eq!(item.age(), 11);
                assert_eq!(item.pickup_delay(), 19);
                assert_eq!(item.item(), &stack_of("minecraft:dirt", 1));
            }
            (Self::Orb, EntityData::ExperienceOrb(orb)) => assert_eq!(orb.age(), 11),
            (Self::Tnt, EntityData::Tnt(tnt)) => assert_eq!(tnt.fuse(), 1),
            _ => unreachable!(),
        }
    }
}

#[test]
fn nonliving_context_actual_ticks_land_emit_hit_ground_and_keep_subtype_suffixes() {
    for kind in [Body::Item, Body::Orb, Body::Tnt] {
        let mut store = EntityStore::new(17, EntityIdCounter::new());
        let id = kind.spawn(&mut store);
        let mut level = NativeLevel::new();
        assert!(matches!(
            level.with_level(|ctx| crate::entity::tick_one_entity(&mut store, ctx, id)),
            crate::entity::EntityTickOutcome::Complete
        ));
        let body = store.get(id).unwrap();
        kind.assert_suffix(body);
        assert_eq!(body.position().y, 64.0, "{kind:?}");
        assert!(body.on_ground());
        assert_eq!(body.fall_distance(), 0.0);
        assert!(body.is_supported_by(pos(0, 63, 0)));
        assert!(level
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(
                |event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND
                    && event.recorded.pos == pos(0, 64, 0)
                    && event.recorded.source_entity_id == Some(id)
                    && event.recorded.affected_state == Some(state_id("minecraft:stone"))
            ));
    }
}
