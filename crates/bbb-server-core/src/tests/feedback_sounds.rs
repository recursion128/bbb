//! Server-broadcast feedback sounds the vanilla client never plays itself.

use super::*;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

const ALICE: u64 = 1;

fn world_with_alice() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        game_type: GameType::Survival,
        datapacks: datapack_config().datapacks,
        ..operator_config(&[("alice", 1)])
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, ALICE, "alice", 1);
    world.tick(receiver.drain());
    (sender, receiver, world)
}

fn spawn_zombie(world: &mut ServerWorld) -> uuid::Uuid {
    let at = world.publication().players[0].position;
    let mut data = crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    data.set_no_ai(true);
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: at.x + 2.0,
                ..at
            },
            crate::entity::EntityData::GeneralLiving(data),
        )
        .1
}

fn sounds(tick: &TickResult) -> Vec<(&'static str, SoundSource, f32)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(ALICE),
                packet: ClientboundPlayPacket::Sound(sound),
            } => match sound.sound {
                SoundEventHolder::Reference { registry_id } => Some((
                    vanilla_sound_event_resource_id(registry_id)?,
                    sound.source,
                    sound.volume,
                )),
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    command: &str,
) -> Vec<(&'static str, SoundSource, f32)> {
    sounds(&chat_command(sender, receiver, world, ALICE, command))
}

/// `LivingEntity.playSecondaryHurtSound` (LivingEntity.java:1322-1327).
#[test]
fn thorns_damage_plays_the_thorns_hit_for_mobs_and_players() {
    let (sender, receiver, mut world) = world_with_alice();
    let zombie = spawn_zombie(&mut world);
    let hit = ("minecraft:enchant.thorns.hit", SoundSource::Hostile, 1.0);
    let played = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {zombie} 1 minecraft:thorns"),
    );
    assert!(played.contains(&hit), "{played:?}");
    let played = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {zombie} 1 minecraft:generic"),
    );
    assert!(!played.iter().any(|sound| sound.0 == hit.0), "{played:?}");
    let played = command(
        &sender,
        &receiver,
        &mut world,
        "damage alice 1 minecraft:thorns",
    );
    assert!(
        played.contains(&("minecraft:enchant.thorns.hit", SoundSource::Players, 1.0)),
        "{played:?}"
    );
}

/// `MobEffect.onEffectAdded` (MobEffect.java:98-100): the omens play their
/// sound when first added, not when an active one is refreshed.
#[test]
fn omen_effects_play_their_sound_when_first_added() {
    let (sender, receiver, mut world) = world_with_alice();
    let zombie = spawn_zombie(&mut world);
    let bad_omen = (
        "minecraft:event.mob_effect.bad_omen",
        SoundSource::Players,
        1.0,
    );
    let played = command(
        &sender,
        &receiver,
        &mut world,
        "effect give alice minecraft:bad_omen 60",
    );
    assert!(played.contains(&bad_omen), "{played:?}");
    let played = command(
        &sender,
        &receiver,
        &mut world,
        "effect give alice minecraft:bad_omen 120",
    );
    assert!(!played.contains(&bad_omen), "{played:?}");
    let played = command(
        &sender,
        &receiver,
        &mut world,
        &format!("effect give {zombie} minecraft:trial_omen 60"),
    );
    assert!(
        played.contains(&(
            "minecraft:event.mob_effect.trial_omen",
            SoundSource::Hostile,
            1.0
        )),
        "{played:?}"
    );
}

/// `FireworkRocketEntity.tick` (FireworkRocketEntity.java:168-170).
#[test]
fn a_firework_rocket_plays_its_launch_sound_once() {
    let (sender, receiver, mut world) = world_with_alice();
    let launch = (
        "minecraft:entity.firework_rocket.launch",
        SoundSource::Ambient,
        3.0,
    );
    let mut played = command(
        &sender,
        &receiver,
        &mut world,
        "summon minecraft:firework_rocket ~ ~1 ~",
    );
    for _ in 0..5 {
        played.extend(sounds(&world.tick(Vec::new())));
    }
    assert_eq!(
        played.iter().filter(|sound| **sound == launch).count(),
        1,
        "{played:?}"
    );
}
