//! Bell resonance through a real block interaction and the world tick.

use super::*;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_GLOWING;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const BELL: BlockPos = BlockPos { x: 0, y: 64, z: 1 };

fn ring(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    assert!(world.set_block(
        OVERWORLD,
        BELL,
        bell_state(),
        crate::fluid::SET_BLOCK_UPDATE_ALL
    ));
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BELL,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.0,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        }),
    )
}

fn spawn(world: &mut ServerWorld, kind: &str, distance: f64) -> i32 {
    let type_id = bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(kind)
        .expect("vanilla mob");
    let position = Vec3d {
        x: 0.5 + distance,
        y: 64.5,
        z: 1.5,
    };
    let (id, _) = world.spawn_stored_entity_for_test(
        OVERWORLD,
        type_id,
        position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(type_id).unwrap(),
        ),
    );
    world.load_stored_entity_record_for_test(
        OVERWORLD,
        id,
        &[
            NbtSummaryEntry {
                name: "NoAI".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "NoGravity".into(),
                value: NbtSummaryValue::Byte(1),
            },
        ],
    );
    world
        .stored_entity_mut_for_test(OVERWORLD, id)
        .unwrap()
        .set_pos(position);
    id
}

fn glowing(world: &ServerWorld, id: i32) -> bool {
    world
        .stored_entity_for_test(OVERWORLD, id)
        .unwrap()
        .living()
        .unwrap()
        .active_effect("minecraft:glowing")
        .is_some()
}

fn resonates(tick: &TickResult) -> bool {
    sounds_sent(tick).iter().any(|sound| {
        sound.sound
            == SoundEventHolder::Reference {
                registry_id: vanilla_sound_event_registry_id("minecraft:block.bell.resonate")
                    .unwrap(),
            }
            && sound.source == SoundSource::Blocks
            && sound.volume == 1.0
            && sound.pitch == 1.0
    })
}

#[test]
fn ringing_reveals_nearby_raiders_and_the_effect_expires() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let near: Vec<_> = [
        "evoker",
        "pillager",
        "ravager",
        "vindicator",
        "illusioner",
        "witch",
    ]
    .into_iter()
    .enumerate()
    .map(|(n, kind)| {
        spawn(
            &mut world,
            &format!("minecraft:{kind}"),
            6.0 + n as f64 * 2.0,
        )
    })
    .collect();
    let far = spawn(&mut world, "minecraft:pillager", 40.0);
    let outside = spawn(&mut world, "minecraft:pillager", 48.0);
    let cow = spawn(&mut world, "minecraft:cow", 5.0);
    let mut heard = resonates(&ring(&sender, &receiver, &mut world));
    let mut metadata_glowed = false;
    for _ in 0..55 {
        let tick = world.tick(receiver.drain());
        heard |= resonates(&tick);
        metadata_glowed |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
                if near.contains(&update.id) && update.values.iter().any(|value| {
                    value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                        && matches!(value.value, EntityDataValueKind::Byte(byte) if byte & VANILLA_ENTITY_SHARED_FLAG_GLOWING != 0)
                })
        ));
    }
    assert!(heard);
    assert!(
        metadata_glowed,
        "tracked raiders publish the glowing shared flag"
    );
    for id in near.iter().copied().chain([far]) {
        assert!(glowing(&world, id), "raider {id}");
        let entity = world.stored_entity_for_test(OVERWORLD, id).unwrap();
        let effect = entity
            .living()
            .unwrap()
            .active_effect("minecraft:glowing")
            .unwrap();
        assert_eq!(effect.amplifier(), 0);
        assert!(effect.duration() <= 60);
    }
    assert!(!glowing(&world, outside));
    assert!(!glowing(&world, cow));
    for _ in 0..65 {
        world.tick(receiver.drain());
    }
    for id in near.into_iter().chain([far]) {
        assert!(!glowing(&world, id));
        assert_eq!(
            world
                .stored_entity_for_test(OVERWORLD, id)
                .unwrap()
                .shared_flags()
                & VANILLA_ENTITY_SHARED_FLAG_GLOWING,
            0
        );
    }
}

#[test]
fn raiders_outside_hearing_range_do_not_start_resonance() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let raider = spawn(&mut world, "minecraft:pillager", 32.0);
    spawn(&mut world, "minecraft:cow", 5.0);
    assert!(!resonates(&ring(&sender, &receiver, &mut world)));
    for _ in 0..60 {
        assert!(!resonates(&world.tick(receiver.drain())));
    }
    assert!(!glowing(&world, raider));
}

#[test]
fn datapack_raiders_tag_can_reveal_an_online_player() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.install_level_registry_tags(Arc::new(crate::registry_tags::RegistryTagSnapshot::new([
        (
            "minecraft:entity_type".into(),
            "minecraft:raiders".into(),
            vec!["minecraft:player".into()],
        ),
    ])));
    ring(&sender, &receiver, &mut world);
    let mut received_effect = false;
    for _ in 0..55 {
        let tick = world.tick(receiver.drain());
        received_effect |= tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::UpdateMobEffect(effect) }
                if effect.effect_id == bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id("minecraft:glowing").unwrap().id
                    && effect.duration_ticks == 60 && effect.amplifier == 0
        ));
    }
    assert!(
        received_effect,
        "tag-selected sessions receive their effect packet"
    );
}
