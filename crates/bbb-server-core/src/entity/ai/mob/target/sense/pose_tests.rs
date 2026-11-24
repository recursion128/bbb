use super::*;
use crate::connection::ConnectionId;
use crate::entity::ai::mob::target::TargetRef;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::Vec3d;

use crate::entity::ai::mob::target::goals::{NearestAttackableTargetGoal, NearestVariant};
use crate::entity::ai::mob::target::selector::TargetSelector;
use crate::entity::ai::{mob::MobAiContext, Goal};
use crate::random::GameRandom;
fn selected(
    store: &EntityStore,
    id: i32,
    level: &LevelCtx<'_, '_>,
    class: &'static str,
) -> Option<TargetRef> {
    let body = store.get(id).unwrap();
    let mut world = store.mob_ai(id).unwrap().target_view(body, store, level);
    let eye_y = world.body().eye_y();
    let mut context = MobAiContext::from_living_body(
        id,
        20,
        body.position(),
        body.bounding_box(),
        eye_y,
        0.0,
        0.0,
        true,
        false,
        false,
        body.living().unwrap(),
        GameRandom::with_seed(41),
    );
    let mut goal = NearestAttackableTargetGoal::new(
        NearestVariant::VindicatorJohnny,
        class,
        0,
        true,
        TargetSelector::Attackable,
    );
    if goal.can_use(&mut context, &mut world) {
        goal.start(&mut context, &mut world);
    }
    context.target.target
}

#[test]
fn living_search_uses_current_scaled_bbox_while_player_search_uses_feet_distance() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "target", 1000);
    let connection = ConnectionId(1);
    native.world.set_player_mode_and_abilities_for_test(
        connection,
        bbb_protocol::game_types::GameType::Survival,
        bbb_protocol::packets::PlayerAbilities::for_game_type(
            bbb_protocol::game_types::GameType::Survival,
        ),
    );
    native.world.set_player_position_for_test(
        connection,
        Vec3d {
            x: 1.9,
            y: 64.0,
            z: 0.5,
        },
    );
    native.world.set_player_attribute_base_for_test(
        connection,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
        0.1,
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
        true,
    );
    native.set(
        bbb_protocol::packets::BlockPos { x: 1, y: 64, z: 0 },
        crate::entity::tests::native_level::state("minecraft:water"),
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(connection),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
    );
    let session = native.world.nearby_players_for_test(DIMENSION)[0];
    let ids = crate::entity::EntityIdCounter::new();
    for _ in 0..session.entity_id {
        ids.next_id();
    }
    let mut store = EntityStore::new(41, ids);
    store.set_difficulty(Difficulty::Normal);
    let body = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .unwrap();
    let actor = store.get_mut(body).unwrap();
    actor
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
        .unwrap()
        .set_base_value(1.0);
    assert!(actor.set_custom_name(Some(
        bbb_protocol::RichTextComponent::try_from_component_nbt(NbtSummaryValue::String(
            "Johnny".into()
        ))
        .unwrap()
    )));
    let expected = Some(TargetRef {
        entity_id: session.entity_id,
        uuid: session.uuid,
    });
    native.with_level(|level| {
        assert_eq!(selected(&store, body, level, "LivingEntity"), None);
        assert_eq!(selected(&store, body, level, "Player"), expected);
        assert_eq!(selected(&store, body, level, "ServerPlayer"), expected);
    });
    native.world.set_player_attribute_base_for_test(
        connection,
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
        1.0,
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
        false,
    );
    native.set(
        bbb_protocol::packets::BlockPos { x: 1, y: 64, z: 0 },
        crate::entity::tests::native_level::state("minecraft:air"),
    );
    native.world.set_player_shared_flag_for_test(
        connection,
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        false,
    );
    native.world.tick(receiver.drain());
    assert_eq!(
        native.world.player_pose_id_for_test(connection),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID
    );
    native.with_level(|level| {
        for class in ["LivingEntity", "Player", "ServerPlayer"] {
            assert_eq!(selected(&store, body, level, class), expected);
        }
    });
    native.world.set_player_position_for_test(
        connection,
        Vec3d {
            x: 3.0,
            y: 64.0,
            z: 0.5,
        },
    );
    assert_eq!(
        native.with_level(|level| selected(&store, body, level, "Player")),
        None
    );
}
