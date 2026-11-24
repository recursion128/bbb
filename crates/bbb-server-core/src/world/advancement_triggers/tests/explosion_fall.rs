use super::*;
use crate::entity::hurting_projectile::wind_charge::WindChargeData;
use crate::entity::hurting_projectile::HurtingProjectileState;
use bbb_protocol::packets::MovePlayer;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const START: Vec3d = Vec3d {
    x: 8.5,
    y: 100.0,
    z: 8.5,
};

fn packet(world: &mut ServerWorld, y: f64, on_ground: bool) -> Vec<WorldEvent> {
    let position = Vec3d { y, ..START };
    let result = world.tick(vec![crate::mailbox::CommandEnvelope::for_test(
        world.last_command_sequence + 1,
        crate::mailbox::WorldProducerClass::Network,
        WorldCommand::Play {
            connection_id: CONNECTION,
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: None,
                on_ground,
                horizontal_collision: false,
            }),
        },
    )]);
    assert!(!result.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close { .. }
            | WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
    )));
    assert_eq!(world.levels[CONNECTION].position(), position);
    result.events
}

fn launch(world: &mut ServerWorld) -> i32 {
    let position = world.levels[CONNECTION].position();
    let owner = world.levels[CONNECTION].profile.uuid;
    let (charge, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        Vec3d {
            y: position.y + 0.5,
            ..position
        },
        EntityData::WindCharge(WindChargeData::new(
            HurtingProjectileState::new(Some(owner), 0.0),
            0,
        )),
    );
    let body = world.stored_entity_mut_for_test(DIMENSION, charge).unwrap();
    assert!(body.add_tag("launch"));
    let events = command(world, &format!(
        "data merge entity {uuid} {{data:{{seal:7b,nested:{{kept:1b,extra:2b}}}},CustomName:{{text:\"launch\"}}}}"
    ));
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::SystemChat(chat) }
            if chat.content.text().contains("commands.data.entity.modified")
    )));
    assert_eq!(
        world
            .stored_entity_for_test(DIMENSION, charge)
            .unwrap()
            .delta_movement(),
        Vec3d::default()
    );
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(DIMENSION, charge)
        .unwrap()
    else {
        unreachable!()
    };
    assert!(saved.iter().any(|field| field.name == "Tags"
        && field.value == NbtSummaryValue::List(vec![NbtSummaryValue::String("launch".into())])));
    let entry = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    for (name, expected) in [
        (
            "data",
            NbtSummaryValue::Compound(vec![
                entry("seal", NbtSummaryValue::Byte(7)),
                entry(
                    "nested",
                    NbtSummaryValue::Compound(vec![
                        entry("kept", NbtSummaryValue::Byte(1)),
                        entry("extra", NbtSummaryValue::Byte(2)),
                    ]),
                ),
            ]),
        ),
        (
            "CustomName",
            NbtSummaryValue::Compound(vec![entry(
                "text",
                NbtSummaryValue::String("launch".into()),
            )]),
        ),
    ] {
        assert!(saved.iter().any(|field| field.name == name
            && bbb_server_types::block_state_parse::block_entity_data_matches(
                Some(&expected),
                Some(&field.value)
            )));
    }
    world
        .stored_entity_mut_for_test(DIMENSION, charge)
        .unwrap()
        .set_delta_movement(Vec3d {
            x: 0.0,
            y: -1.0,
            z: 0.0,
        });
    let output = world.tick(Vec::new());
    assert!(world.stored_entity_for_test(DIMENSION, charge).is_none());
    assert!(output.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: CONNECTION,
            packet: ClientboundPlayPacket::Explode(_),
        }
    )));
    charge
}

#[test]
fn explosion_fall_measures_the_launch_to_first_descent_and_matches_the_retained_cause() {
    let conditions = json!({
        "player":{"type":"minecraft:player"},
        "start_position":{"position":{"y":{"min":99,"max":101}}},
        "distance":{"y":{"min":7}},
        "cause":[this_entity_properties(json!({"type":"minecraft:wind_charge",
            "nbt":"{Tags:[\"launch\"]}",
            "components":{"minecraft:custom_name":{"text":"launch"},
                "minecraft:custom_data":{"seal":7,"nested":{"kept":1,"extra":2}}},
            "predicates":{"minecraft:custom_data":"{seal:7b,nested:{kept:1b}}"}}))]
    });
    let mut wrong_cause = conditions.clone();
    wrong_cause["cause"][0]["predicate"]["type"] = json!("minecraft:breeze_wind_charge");
    let mut wrong_nbt = conditions.clone();
    wrong_nbt["cause"][0]["predicate"]["nbt"] = json!("{Tags:[\"other\"]}");
    let mut wrong_name = conditions.clone();
    wrong_name["cause"][0]["predicate"]["components"]["minecraft:custom_name"] =
        json!({"text":"other"});
    let mut wrong_data = conditions.clone();
    wrong_data["cause"][0]["predicate"]["components"]["minecraft:custom_data"]["seal"] = json!(8);
    let mut wrong_partial = conditions.clone();
    wrong_partial["cause"][0]["predicate"]["predicates"]["minecraft:custom_data"] =
        json!("{seal:8b,nested:{kept:1b}}");
    let mut wrong_start = conditions.clone();
    wrong_start["start_position"]["position"]["y"] = json!({"min":200});
    let mut wrong_player = conditions.clone();
    wrong_player["player"]["type"] = json!("minecraft:cow");
    let mut too_far = conditions.clone();
    too_far["distance"]["y"] = json!({"min":20});
    let mut mixed_tnt = conditions.clone();
    mixed_tnt["cause"][0]["predicate"] = json!({"type":"minecraft:tnt","nbt":"{Tags:[\"mixed\"]}"});
    let mut wrong_mixed_nbt = mixed_tnt.clone();
    wrong_mixed_nbt["cause"][0]["predicate"]["nbt"] = json!("{Tags:[\"other\"]}");
    let definitions = [
        ("test:flight", conditions, 7),
        ("test:wrong_cause", wrong_cause, 7),
        ("test:wrong_nbt", wrong_nbt, 7),
        ("test:wrong_name", wrong_name, 0),
        ("test:wrong_data", wrong_data, 0),
        ("test:wrong_partial", wrong_partial, 0),
        ("test:wrong_start", wrong_start, 7),
        ("test:wrong_player", wrong_player, 7),
        ("test:too_far", too_far, 7),
        ("test:mixed_tnt", mixed_tnt, 0),
        ("test:wrong_mixed_nbt", wrong_mixed_nbt, 0),
    ]
    .map(|(id, conditions, reward)| {
        (id, json!({
            "criteria":{"flight":{"trigger":"minecraft:fall_after_explosion","conditions":conditions}},
            "rewards":{"experience":reward}
        }))
    });
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup(&definitions)),
        ..crate::tests::operator_config(&[("rocketeer", 1)])
    })
    .unwrap();
    let (sender, receiver) = crate::mailbox::world_command_mailbox(Default::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "rocketeer", 1);
    world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (0, 0));
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id("minecraft:stone")
            .unwrap()
            .id,
    )
    .unwrap();
    for x in 7..=9 {
        for z in 7..=9 {
            world.set_raw_block_state_for_test(DIMENSION, BlockPos { x, y: 99, z }, stone);
        }
    }
    world.set_player_position_for_test(CONNECTION, START);
    for (attempt, height) in [5, 9, 9].into_iter().enumerate() {
        let charge = launch(&mut world);
        for y in 1..=height {
            packet(&mut world, START.y + f64::from(y), false);
        }
        assert!(world.stored_entity_for_test(DIMENSION, charge).is_none());
        let events = packet(&mut world, START.y + f64::from(height) - 0.5, false);
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, "test:flight"),
            height >= 7
        );
        assert_eq!(xp(&world), if height >= 7 { 7 } else { 0 });
        if attempt == 1 {
            assert!(events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
                    if *connection_id == CONNECTION && update.progress.iter().any(|progress|
                        progress.id == "test:flight" && progress.criteria.iter().any(|criterion|
                            criterion.name == "flight" && criterion.obtained_epoch_millis.is_some()))
            )));
        }
        let written = world.player_advancements_to_data(CONNECTION);
        assert_eq!(
            written.progress.get("test:flight").is_some_and(|progress| {
                progress
                    .criteria()
                    .iter()
                    .any(|(name, criterion)| name == "flight" && criterion.is_done())
            }),
            height >= 7
        );
        for id in [
            "test:wrong_cause",
            "test:wrong_nbt",
            "test:wrong_name",
            "test:wrong_data",
            "test:wrong_partial",
            "test:wrong_start",
            "test:wrong_player",
            "test:too_far",
            "test:mixed_tnt",
            "test:wrong_mixed_nbt",
        ] {
            assert!(!world.player_advancement_is_done(CONNECTION, id), "{id}");
            assert!(!events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
                    if update.progress.iter().any(|progress| progress.id == id
                        && progress.criteria.iter().any(|criterion| criterion.obtained_epoch_millis.is_some()))
            )), "{id}");
            assert!(
                written.progress.get(id).is_none_or(|progress| progress
                    .criteria()
                    .iter()
                    .all(|(_, criterion)| !criterion.is_done())),
                "{id}"
            );
        }
        packet(&mut world, START.y, true);
    }

    let charge = launch(&mut world);
    for y in 1..=9 {
        packet(&mut world, START.y + f64::from(y), false);
    }
    assert!(world.stored_entity_for_test(DIMENSION, charge).is_none());
    assert_eq!(
        world.levels[CONNECTION].cold.fall_impulse.impact_position,
        Some(START)
    );
    assert!(world.levels[CONNECTION]
        .block_contact
        .starting_to_fall_position
        .is_none());
    assert_eq!(world.levels[CONNECTION].block_contact.fall_distance, 0.0);
    assert!(world.levels[CONNECTION].is_alive());
    assert!(!world.levels[CONNECTION].abilities.flying);

    let (tnt, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID,
        Vec3d {
            x: START.x + 5.0,
            y: START.y + 9.0,
            ..START
        },
        EntityData::Tnt(crate::entity::TntData::with_saved_fields(
            1,
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID,
            )
            .unwrap(),
            None,
            3.0,
        )),
    );
    let body = world.stored_entity_mut_for_test(DIMENSION, tnt).unwrap();
    assert!(body.add_tag("mixed"));
    body.set_no_gravity(true);
    let NbtSummaryValue::Compound(saved) = world
        .stored_entity_command_data_for_test(DIMENSION, tnt)
        .unwrap()
    else {
        unreachable!()
    };
    for (name, expected) in [
        (
            "Tags",
            NbtSummaryValue::List(vec![NbtSummaryValue::String("mixed".into())]),
        ),
        ("fuse", NbtSummaryValue::Short(1)),
        ("explosion_power", NbtSummaryValue::Float(3.0_f32.to_bits())),
        ("NoGravity", NbtSummaryValue::Byte(1)),
    ] {
        assert_eq!(
            saved
                .iter()
                .find(|field| field.name == name)
                .map(|field| &field.value),
            Some(&expected),
            "{name}"
        );
    }
    let mut explosion_events = Vec::new();
    for _ in 0..20 {
        explosion_events.extend(world.tick(Vec::new()).events);
        if world.stored_entity_for_test(DIMENSION, tnt).is_none() {
            break;
        }
    }
    assert!(world.stored_entity_for_test(DIMENSION, tnt).is_none());
    assert!(world
        .live_entity_id_for_uuid_for_test(DIMENSION, uuid)
        .is_none());
    assert!(explosion_events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: CONNECTION,
            packet: ClientboundPlayPacket::Explode(explosion),
        } if explosion.radius == 3.0
    )));
    let player = &world.levels[CONNECTION];
    assert!(player.is_alive());
    assert!(!player.abilities.flying);
    assert_eq!(player.cold.fall_impulse.impact_position, Some(START));
    assert!(player.block_contact.starting_to_fall_position.is_none());
    assert_eq!(player.block_contact.fall_distance, 0.0);
    let (cause, _) = player.cold.impulse_explosion_cause.as_ref().unwrap();
    assert_eq!(cause.entity_id, tnt);
    assert_eq!(cause.uuid, uuid);
    assert_eq!(cause.entity_resource_id, "minecraft:tnt");
    for id in ["test:mixed_tnt", "test:wrong_mixed_nbt"] {
        assert!(!world.player_advancement_is_done(CONNECTION, id), "{id}");
    }

    let events = packet(&mut world, START.y + 8.5, false);
    let written = world.player_advancements_to_data(CONNECTION);
    for (id, expected) in [("test:mixed_tnt", true), ("test:wrong_mixed_nbt", false)] {
        assert_eq!(
            world.player_advancement_is_done(CONNECTION, id),
            expected,
            "{id}"
        );
        assert_eq!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::UpdateAdvancements(update) }
                if update.progress.iter().any(|progress| progress.id == id
                    && progress.criteria.iter().any(|criterion|
                        criterion.name == "flight" && criterion.obtained_epoch_millis.is_some()))
        )), expected, "{id}");
        assert_eq!(
            written.progress.get(id).is_some_and(|progress| progress
                .criteria()
                .iter()
                .any(|(name, criterion)| name == "flight" && criterion.is_done())),
            expected,
            "{id}"
        );
    }
    assert_eq!(xp(&world), 7);
}
