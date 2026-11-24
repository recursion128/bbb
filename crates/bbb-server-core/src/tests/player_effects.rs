//! The effects a session carries: what reaches its client, what the tick does
//! to its health and food, and the two effects that only work because the
//! health clamps read the player's own attribute map.

use super::*;

use crate::entity::living::mob_effects::ActiveMobEffect;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_GLOWING, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
};
use bbb_protocol::packets::PlayerHealth;

const PLAYER: u64 = 1;

/// A world whose sessions can be hurt: `GameType.updatePlayerAbilities` gives
/// creative and spectator `Abilities.invulnerable` (`GameType.java:63-71`),
/// which `isInvulnerableTo` refuses every source through.
fn survival_config() -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..ServerWorldConfig::default()
    }
}

fn effect(resource_id: &'static str, duration: i32, amplifier: i32) -> ActiveMobEffect {
    ActiveMobEffect::new(resource_id, duration, amplifier).expect("a vanilla effect id")
}

fn joined(world: &mut ServerWorld, sender: &WorldCommandSender, receiver: &WorldCommandReceiver) {
    join_loaded(sender, receiver, world, PLAYER, "alice", 1);
}

fn effect_updates(events: &[WorldEvent]) -> Vec<(i32, i32, i32, bool)> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::UpdateMobEffect(update),
            } if *connection_id == ConnectionId(PLAYER) => Some((
                update.entity_id,
                update.effect_id,
                update.duration_ticks,
                update.flags.blend,
            )),
            _ => None,
        })
        .collect()
}

fn effect_removals(events: &[WorldEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::RemoveMobEffect(remove),
            } if *connection_id == ConnectionId(PLAYER) => Some(remove.effect_id),
            _ => None,
        })
        .collect()
}

fn health_updates(events: &[WorldEvent]) -> Vec<PlayerHealth> {
    events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SetHealth(health),
            } if *connection_id == ConnectionId(PLAYER) => Some(*health),
            _ => None,
        })
        .collect()
}

fn effect_id(resource_id: &str) -> i32 {
    bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(resource_id)
        .expect("a vanilla effect id")
        .id
}

/// `ServerPlayer.onEffectAdded` (`ServerPlayer.java:1636-1638`) is the one send
/// in this server that sets `blend`: a session is told about its own new effect
/// by a packet no passenger send can produce
/// (`LivingEntity.sendEffectToPassengers`, `:1062-1068`, passes `false`).
#[test]
fn a_session_is_told_about_its_own_new_effect_with_blend_set() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    let entity_id = entity_id_of(&world, PLAYER);

    let mut events = Vec::new();
    assert_eq!(
        world.add_player_mob_effect(
            ConnectionId(PLAYER),
            effect("minecraft:speed", 200, 0),
            &mut events
        ),
        Some(true)
    );
    assert_eq!(
        effect_updates(&events),
        [(entity_id, effect_id("minecraft:speed"), 200, true)]
    );
}

/// `LivingEntity.tickEffects` (`:828-846`) counts the instance down and, on the
/// tick it runs out, hands `onEffectsRemoved` the instance
/// (`:837-838`) -- which for a session is a
/// `ClientboundRemoveMobEffectPacket` (`ServerPlayer.java:1655-1661`).
#[test]
fn the_tick_counts_an_effect_down_and_tells_the_session_when_it_expires() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:speed", 3, 0),
        &mut events,
    );

    for remaining in [2, 1] {
        let tick = world.tick(receiver.drain());
        assert!(effect_removals(&tick.events).is_empty(), "{remaining}");
        assert_eq!(
            world
                .player_active_effect_for_test(ConnectionId(PLAYER), "minecraft:speed")
                .map(|effect| effect.duration()),
            Some(remaining)
        );
    }
    let expired = world.tick(receiver.drain());
    assert_eq!(
        effect_removals(&expired.events),
        [effect_id("minecraft:speed")]
    );
    assert_eq!(
        world.player_active_effect_for_test(ConnectionId(PLAYER), "minecraft:speed"),
        None
    );
}

/// `LivingEntity.updateInvisibilityStatus` (`LivingEntity.java:870-878`) over a
/// session: the effect list drives the invisibility bit of the one shared-flags
/// byte this server owns for a player, and the level tracker publishes it where
/// `ServerEntity.sendDirtyEntityData` publishes a stored entity's
/// (`ServerEntity.java:91`).
///
/// `GameType.updatePlayerAbilities` and the rest of `updateDirtyEffects` are
/// not this test's subject: the byte below is the only row either session
/// carries.
#[test]
fn an_invisible_session_publishes_the_shared_flag_its_effect_drives() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    let entity_id = entity_id_of(&world, PLAYER);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:invisibility", 200, 0),
        &mut events,
    );
    let applied = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&applied.events, entity_id),
        [VANILLA_ENTITY_SHARED_FLAG_INVISIBLE],
        "the tracker publishes the byte `updateInvisibilityStatus` wrote"
    );

    world.remove_player_mob_effect(ConnectionId(PLAYER), "minecraft:invisibility", &mut events);
    let cleared = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&cleared.events, entity_id),
        [0],
        "the empty effect list writes false"
    );
}

/// `ServerPlayer.updateInvisibilityStatus` (`ServerPlayer.java:1903-1910`): a
/// spectator is invisible whatever its effects, re-read when `setGameMode`
/// calls `updateEffectVisibility` (`:1754`).
#[test]
fn a_spectator_session_is_invisible_until_it_leaves_spectator() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(&[("alice", 1)])
    });
    joined(&mut world, &sender, &receiver);
    let entity_id = entity_id_of(&world, PLAYER);

    let spectating = chat_command(&sender, &receiver, &mut world, PLAYER, "gamemode spectator");
    assert_eq!(
        shared_flag_bytes(&spectating.events, entity_id),
        [VANILLA_ENTITY_SHARED_FLAG_INVISIBLE]
    );
    let surviving = chat_command(&sender, &receiver, &mut world, PLAYER, "gamemode survival");
    assert_eq!(shared_flag_bytes(&surviving.events, entity_id), [0]);
}

/// The `DATA_SHARED_FLAGS_ID` bytes one tick published for `entity_id`.
fn shared_flag_bytes(events: &[WorldEvent], entity_id: i32) -> Vec<i8> {
    let mut bytes = Vec::new();
    for event in events {
        if let WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(update),
            ..
        } = event
        {
            if update.id != entity_id {
                continue;
            }
            for value in &update.values {
                if value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID {
                    let EntityDataValueKind::Byte(byte) = value.value else {
                        panic!("the shared-flags row is a byte")
                    };
                    bytes.push(byte);
                }
            }
        }
    }
    bytes
}

/// `MobEffects.HEALTH_BOOST` carries a `max_health` modifier of
/// `4.0 * (amplifier + 1)` (`MobEffects.java:78-82`), and `setHealth` clamps
/// against `getMaxHealth()` (`LivingEntity.java:1146-1148`, `:1954-1956`) --
/// so the raised cap is only real if the clamp reads the player's own attribute
/// map rather than the registry base. When the effect expires,
/// `onAttributeUpdated` drags the health back down (`:1112-1117`).
#[test]
fn health_boost_raises_the_cap_and_expiring_it_lowers_the_health_again() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    assert_eq!(world.player_max_health_for_test(ConnectionId(PLAYER)), 20.0);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:health_boost", 2, 0),
        &mut events,
    );
    assert_eq!(world.player_max_health_for_test(ConnectionId(PLAYER)), 24.0);

    world.set_player_health_for_test(ConnectionId(PLAYER), 24.0);
    assert_eq!(world.player_health_for_test(ConnectionId(PLAYER)), 24.0);

    world.tick(receiver.drain());
    let expired = world.tick(receiver.drain());
    assert_eq!(
        effect_removals(&expired.events),
        [effect_id("minecraft:health_boost")]
    );
    assert_eq!(world.player_max_health_for_test(ConnectionId(PLAYER)), 20.0);
    assert_eq!(world.player_health_for_test(ConnectionId(PLAYER)), 20.0);
}

/// `MobEffects.ABSORPTION` carries a `max_absorption` modifier
/// (`MobEffects.java:83-87`) and `AbsorptionMobEffect.onEffectStarted` raises
/// the current amount to `4 * (amplifier + 1)` (`AbsorptionMobEffect.java:12-24`).
/// With the cap read from the registry base of zero, `setAbsorptionAmount`'s
/// clamp made the whole effect a no-op.
#[test]
fn absorption_gives_a_session_absorption_that_takes_the_hit() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:absorption", 200, 0),
        &mut events,
    );
    assert_eq!(
        world.player_absorption_for_test(ConnectionId(PLAYER)),
        4.0,
        "the effect's own start raised it"
    );

    world.hurt_connected_player_for_test(ConnectionId(PLAYER), 3.0, &mut events);
    // `LivingEntity.actuallyHurt` (`:1925-1931`) spends the absorption first.
    assert_eq!(world.player_absorption_for_test(ConnectionId(PLAYER)), 1.0);
    assert_eq!(world.player_health_for_test(ConnectionId(PLAYER)), 20.0);
}

/// `SaturationMobEffect.applyEffectTick` feeds the player
/// (`SaturationMobEffect.java:13-19`), and its `InstantenousMobEffect` base
/// applies on any remaining duration (`InstantenousMobEffect.java:14-16`).
#[test]
fn saturation_feeds_the_session_on_the_tick_it_applies() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    world.set_player_food_for_test(ConnectionId(PLAYER), 10, 0.0);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:saturation", 1, 0),
        &mut events,
    );
    let tick = world.tick(receiver.drain());

    // `FoodData.eat(amplification + 1, 1.0F)` (`FoodData.java:24-26`).
    let health = health_updates(&tick.events);
    assert_eq!(health.len(), 1, "{health:?}");
    assert_eq!(health[0].food, 11);
    assert!(health[0].saturation > 0.0, "{health:?}");
    // Instantaneous, so the one tick is also its last.
    assert_eq!(
        effect_removals(&tick.events),
        [effect_id("minecraft:saturation")]
    );
}

/// `PoisonMobEffect.applyEffectTick` (`PoisonMobEffect.java:14-26`) hurts every
/// twenty-five ticks at amplifier zero and never below one health, and the hurt
/// reaches the session through the same pipeline `/damage` does.
#[test]
fn poison_hurts_the_session_on_its_own_interval() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);

    // Below eighteen food neither regeneration branch of `FoodData.tick` runs
    // (`FoodData.java:45`, `:53`), so what poison takes stays taken.
    world.set_player_food_for_test(ConnectionId(PLAYER), 10, 0.0);

    let mut events = Vec::new();
    assert_eq!(
        world.add_player_mob_effect(
            ConnectionId(PLAYER),
            effect("minecraft:poison", 60, 0),
            &mut events,
        ),
        Some(true)
    );
    for _ in 0..60 {
        world.tick(receiver.drain());
    }
    // The instance applies before it counts down (`MobEffectInstance.java:228-236`),
    // so the twenty-five-tick interval lands on the remaining durations 50 and
    // 25: two hits of one, and the `getHealth() > 1.0F` floor is never near.
    assert_eq!(world.player_health_for_test(ConnectionId(PLAYER)), 18.0);
    assert_eq!(
        world.player_active_effect_for_test(ConnectionId(PLAYER), "minecraft:poison"),
        None,
        "sixty ticks is the whole duration"
    );
}

/// `LivingEntity.addAdditionalSaveData` writes the packed attribute map
/// (`LivingEntity.java:731`) and then the active effects (`:732-734`);
/// `readAdditionalSaveData` applies the first (`:780`) and files the second in
/// *without* re-adding its modifiers (`:783-789`). The modifiers come back
/// through `attributes` because `MobEffect.addAttributeModifiers` filed them
/// permanently (`MobEffect.java:167-175`) and `AttributeInstance.pack` writes
/// exactly the permanent ones (`AttributeInstance.java:186`) -- which is why
/// the two fields only make sense travelling together.
#[test]
fn an_effect_and_the_cap_it_raised_both_survive_a_save_and_a_restart() {
    let root =
        std::env::temp_dir().join(format!("bbb-client-world-effects-{}", std::process::id()));
    let storage = created_fixture_save(&root, "bbb-effects");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(survival_config(), &candidate).unwrap();
    joined(&mut world, &sender, &receiver);
    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:health_boost", 400, 0),
        &mut events,
    );
    world.set_player_health_for_test(ConnectionId(PLAYER), 24.0);
    let packed_before_save = world
        .player_attributes_for_test(ConnectionId(PLAYER))
        .pack();

    let request = world
        .save_request(1_786_000_001_000, 201, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (_, saved) = reloaded
        .load_player(profile("alice", 1).uuid)
        .unwrap()
        .expect("the session's save wrote its playerdata");
    assert_eq!(saved.active_effects().len(), 1);
    assert_eq!(
        crate::entity::PackedAttribute::from_saved_list(saved.attributes()),
        packed_before_save,
    );
    // Without it the health would come back clamped to the registry base of
    // twenty (`:791`).
    assert_eq!(
        saved
            .damage(bbb_server_datapack::damage::player_max_health())
            .health,
        20.0
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(survival_config(), &reloaded).unwrap();
    let (rejoined, _) = join_loaded_ticks(&sender, &receiver, &mut world, PLAYER, "alice", 1);
    assert_eq!(world.player_max_health_for_test(ConnectionId(PLAYER)), 24.0);
    assert_eq!(world.player_health_for_test(ConnectionId(PLAYER)), 24.0);
    let restored_effect = world
        .player_active_effect_for_test(ConnectionId(PLAYER), "minecraft:health_boost")
        .expect("the restored session carries the saved effect");
    assert_eq!(restored_effect.amplifier(), 0);
    // It is live rather than a restored constant: the two ticks the join ran
    // took the saved four hundred down.
    assert_eq!(restored_effect.duration(), 398);
    // `PlayerList.placeNewPlayer` (`PlayerList.java:214`) announces it before
    // either of those ticks, with `blend` false because this is not
    // `onEffectAdded`.
    assert_eq!(
        effect_updates(&rejoined.events),
        [(
            world.player_entity_id_for_test(ConnectionId(PLAYER)),
            effect_id("minecraft:health_boost"),
            400,
            false,
        )]
    );

    storage.close().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

/// `LivingEntity.updateGlowingStatus` (`LivingEntity.java:891-896`), the second
/// callback `updateDirtyEffects` runs behind the one `effectsDirty` gate
/// (`:862-868`): `isCurrentlyGlowing` adds `hasEffect(GLOWING)` to the
/// inherited tag off the client (`:3876-3878`, `Entity.java:2668-2670`), and
/// the bit it writes is shared flag 6 of the same byte the level tracker
/// publishes.
#[test]
fn a_glowing_session_publishes_the_shared_flag_its_effect_drives() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    let entity_id = entity_id_of(&world, PLAYER);

    let mut events = Vec::new();
    world.add_player_mob_effect(
        ConnectionId(PLAYER),
        effect("minecraft:glowing", 200, 0),
        &mut events,
    );
    let applied = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&applied.events, entity_id),
        [VANILLA_ENTITY_SHARED_FLAG_GLOWING],
        "the tracker publishes the byte `updateGlowingStatus` wrote"
    );

    world.remove_player_mob_effect(ConnectionId(PLAYER), "minecraft:glowing", &mut events);
    let cleared = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&cleared.events, entity_id),
        [0],
        "losing the effect clears the bit again"
    );
}

/// Both callbacks at once, which is what the one gate means: a session holding
/// invisibility and glowing together carries both bits of the one byte.
#[test]
fn one_dirty_gate_settles_both_bits_of_the_sessions_byte() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    let entity_id = entity_id_of(&world, PLAYER);

    let mut events = Vec::new();
    for resource_id in ["minecraft:invisibility", "minecraft:glowing"] {
        world.add_player_mob_effect(
            ConnectionId(PLAYER),
            effect(resource_id, 200, 0),
            &mut events,
        );
    }
    let applied = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&applied.events, entity_id),
        [VANILLA_ENTITY_SHARED_FLAG_INVISIBLE | VANILLA_ENTITY_SHARED_FLAG_GLOWING]
    );

    world.remove_player_mob_effect(ConnectionId(PLAYER), "minecraft:glowing", &mut events);
    let half = world.tick(receiver.drain());
    assert_eq!(
        shared_flag_bytes(&half.events, entity_id),
        [VANILLA_ENTITY_SHARED_FLAG_INVISIBLE],
        "the other callback's bit survives"
    );
}

/// The `potion` a persistent stack's `minecraft:potion_contents` names.
fn named_potion(value: &NbtSummaryValue) -> Option<String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    for entry in entries {
        if entry.name == "potion" {
            if let NbtSummaryValue::String(name) = &entry.value {
                return Some(name.try_to_string().unwrap());
            }
        }
        if let Some(found) = named_potion(&entry.value) {
            return Some(found);
        }
    }
    None
}

/// The potion of the one thrown splash potion the level holds, if it has been
/// spawned yet.
fn thrown_splash_potion(world: &ServerWorld) -> Option<String> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            match world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?
                .data()
            {
                crate::entity::EntityData::SplashPotion(data) => {
                    named_potion(&data.item().write(&world.item_stack_resolver()).ok()??)
                }
                _ => None,
            }
        })
}

/// The potion a witch nine blocks east of the session throws at it, with the
/// session already holding `held`.
///
/// Nine blocks is both arms of the choice at once: `dist >= 8.0` opens the
/// slowness branch and `distanceToSqr <= 121.0` keeps `Witch.aiStep` off the
/// swiftness drink that would suppress the throw (`Witch.java:142-146`).
fn witch_potion_thrown_at_a_session(held: &[&'static str]) -> String {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_config());
    joined(&mut world, &sender, &receiver);
    let mut events = Vec::new();
    for resource_id in held {
        world.add_player_mob_effect(
            ConnectionId(PLAYER),
            effect(resource_id, 600, 0),
            &mut events,
        );
    }
    let spot = world.publication().players[0].position;
    let witch_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITCH_ID;
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        witch_type,
        Vec3d {
            x: spot.x + 9.0,
            y: spot.y,
            z: spot.z,
        },
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(witch_type).expect("witch attributes"),
        ),
    );
    for _ in 0..240 {
        world.tick(receiver.drain());
        if let Some(potion) = thrown_splash_potion(&world) {
            return potion;
        }
    }
    panic!("the witch never threw a potion at the session");
}

/// `Witch.performRangedAttack`'s three `hasEffect` reads over a session
/// (`Witch.java:237-242`): the potion is the first of slowness, poison,
/// weakness the target is not already carrying, so what the session's own
/// effect table holds moves which projectile the witch spawns.
#[test]
fn a_witch_picks_its_potion_off_the_sessions_own_effect_table() {
    assert_eq!(
        witch_potion_thrown_at_a_session(&[]),
        "minecraft:slowness",
        "nine blocks out, a session carrying nothing draws the first branch"
    );
    assert_eq!(
        witch_potion_thrown_at_a_session(&["minecraft:slowness"]),
        "minecraft:poison",
        "the session's own slowness closes that branch and health >= 8 opens the next"
    );
    assert_eq!(
        witch_potion_thrown_at_a_session(&["minecraft:slowness", "minecraft:poison"]),
        "minecraft:harming",
        "with both closed, nine blocks is too far for the weakness branch"
    );
}
