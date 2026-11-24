//! Mob-specific step, swim, activity and shake sounds heard by a session.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::console_command;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
    VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_WARPED_FUNGUS_ID;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::sound_event_types::vanilla_sound_event_resource_id;

/// A session fixture that records every sound and entity event Alice receives.
struct Arena {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    sounds: Vec<&'static str>,
    events: Vec<(i32, i8)>,
}

impl Arena {
    fn new(spectator: bool) -> Self {
        let (sender, receiver, world) = fixture();
        let mut arena = Self {
            sender,
            receiver,
            world,
            sounds: Vec::new(),
            events: Vec::new(),
        };
        if spectator {
            arena.command("gamemode spectator alice");
        }
        arena
    }

    fn command(&mut self, text: &str) {
        let operation = self.world.game_time() as u64 + 10000;
        let output = console_command(
            &self.sender,
            &self.receiver,
            &mut self.world,
            operation,
            text,
        );
        let outcome = admin_outcome(&output);
        let Ok(AdminResult::Command(report)) = &outcome.result else {
            panic!("{text}: {outcome:?}");
        };
        assert!(report.result.is_some(), "{text}: {report:?}");
        self.record(&output);
    }

    fn record(&mut self, output: &TickResult) {
        for event in &output.events {
            let WorldEvent::Send {
                connection_id: ALICE,
                packet,
            } = event
            else {
                continue;
            };
            match packet {
                ClientboundPlayPacket::KeepAlive { id } => {
                    enqueue(
                        &self.sender,
                        WorldCommand::Play {
                            connection_id: ALICE,
                            packet: PlayServerbound::KeepAlive { id: *id },
                        },
                    )
                    .unwrap();
                }
                ClientboundPlayPacket::Sound(sound) => {
                    if let SoundEventHolder::Reference { registry_id } = sound.sound {
                        self.sounds
                            .extend(vanilla_sound_event_resource_id(registry_id));
                    }
                }
                ClientboundPlayPacket::EntityEvent(packet) => {
                    self.events.push((packet.entity_id, packet.event_id))
                }
                _ => {}
            }
        }
    }

    fn mob(&mut self, kind: i32, position: Vec3d, nbt: &str) -> i32 {
        let (id, uuid) = spawn(&mut self.world, kind, position);
        if !nbt.is_empty() {
            self.command(&format!("data merge entity {uuid} {nbt}"));
        }
        id
    }

    fn tick(&mut self) {
        let output = self.world.tick(self.receiver.drain());
        self.record(&output);
    }

    /// Pushes the body horizontally, keeping its fall so it stays grounded.
    fn push(&mut self, id: i32, x: f64) {
        let body = self
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap();
        let delta = body.delta_movement();
        body.set_delta_movement(Vec3d { x, ..delta });
    }

    /// Ticks until `sound` has been heard, at most `limit` times.
    fn hears(&mut self, sound: &str, limit: usize) -> bool {
        for _ in 0..limit {
            if self.sounds.contains(&sound) {
                return true;
            }
            self.tick();
        }
        self.sounds.contains(&sound)
    }
}

/// `Spider.playStepSound` (`Spider.java:108-110`) and `AbstractFish.getSwimSound`
/// (`AbstractFish.java:155-157`) replace the block step and generic swim sounds.
#[test]
fn walking_and_swimming_mobs_use_their_own_movement_sounds() {
    let mut arena = Arena::new(true);
    arena.command("fill 0 65 12 15 67 15 minecraft:water");
    let spider = arena.mob(
        VANILLA_ENTITY_TYPE_SPIDER_ID,
        Vec3d {
            x: 2.5,
            y: 65.0,
            z: 4.5,
        },
        "",
    );
    let cod = arena.mob(
        VANILLA_ENTITY_TYPE_COD_ID,
        Vec3d {
            x: 2.5,
            y: 66.0,
            z: 13.5,
        },
        "",
    );
    for _ in 0..25 {
        arena.push(spider, 0.4);
        arena.push(cod, 0.4);
        arena.tick();
    }
    let heard = &arena.sounds;
    assert!(heard.contains(&"minecraft:entity.spider.step"), "{heard:?}");
    assert!(heard.contains(&"minecraft:entity.fish.swim"), "{heard:?}");
    assert!(!heard.contains(&"minecraft:block.stone.step"), "{heard:?}");
    assert!(
        !heard.contains(&"minecraft:entity.generic.swim"),
        "{heard:?}"
    );
}

/// `Wolf.aiStep`/`tick` (`Wolf.java:299-358`): leaving water starts a shake with
/// entity event 8 and the shake sound; getting wet again cancels it with 56.
#[test]
fn a_wet_wolf_shakes_dry_and_rewetting_cancels_the_shake() {
    let mut arena = Arena::new(true);
    arena.command("fill 7 65 7 9 66 9 minecraft:stone");
    arena.command("setblock 8 66 8 minecraft:air");
    arena.command("setblock 8 65 8 minecraft:water");
    let wolf = arena.mob(
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 8.5,
        },
        "",
    );
    for _ in 0..5 {
        arena.tick();
    }
    arena.command("setblock 8 65 8 minecraft:air");
    arena.events.clear();
    arena.sounds.clear();
    for _ in 0..10 {
        arena.tick();
    }
    assert!(arena.events.contains(&(wolf, 8)), "{:?}", arena.events);
    assert!(
        !arena.events.contains(&(wolf, 56)),
        "a dry wolf keeps shaking"
    );
    assert!(arena.sounds.contains(&"minecraft:entity.wolf.shake"));
    arena.command("setblock 8 65 8 minecraft:water");
    arena.tick();
    assert!(arena.events.contains(&(wolf, 56)), "{:?}", arena.events);
}

/// `PiglinAi.updateActivity` plays the new activity's sound
/// (`PiglinAi.java:313-315`, `:620-621`), and a fighting brute growls
/// (`PiglinBruteAi.java:150-160`).
#[test]
fn piglins_growl_when_they_start_fighting_and_brutes_growl_while_fighting() {
    let mut arena = Arena::new(false);
    arena.command("effect give alice minecraft:resistance infinite 4 true");
    arena.mob(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
        "{IsImmuneToZombification:1b}",
    );
    assert!(arena.hears("minecraft:entity.piglin.angry", 100));
    arena.mob(
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        },
        "{IsImmuneToZombification:1b}",
    );
    assert!(arena.hears("minecraft:entity.piglin_brute.angry", 40));
}

/// `HoglinAi.updateActivity` (`HoglinAi.java:120-127`) and `Zoglin.updateActivity`
/// (`Zoglin.java:214-223`) growl on entering a fight.
#[test]
fn hoglins_and_zoglins_growl_when_they_start_fighting() {
    let mut arena = Arena::new(false);
    arena.command("effect give alice minecraft:resistance infinite 4 true");
    arena.mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
        "{IsImmuneToZombification:1b}",
    );
    assert!(arena.hears("minecraft:entity.hoglin.angry", 40));
    arena.mob(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        },
        "",
    );
    assert!(arena.hears("minecraft:entity.zoglin.angry", 40));
}

/// `Piglin` admiring a picked-up gold ingot (`PiglinAi.java:626-627`).
#[test]
fn a_piglin_admiring_gold_plays_its_admiring_sound() {
    let mut arena = Arena::new(true);
    arena.mob(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
        "{IsImmuneToZombification:1b,CanPickUpLoot:1b}",
    );
    arena.command("summon minecraft:item 5.5 65 8.5 {Item:{id:\"minecraft:gold_ingot\",count:1}}");
    assert!(arena.hears("minecraft:entity.piglin.admiring_item", 200));
}

/// `Strider.tick` (`Strider.java:288-293`): a tempted strider is happy.
#[test]
fn a_tempted_strider_sounds_happy() {
    let mut arena = Arena::new(false);
    arena.world.set_inventory_slot_for_test(
        ALICE,
        0,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_WARPED_FUNGUS_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    arena.mob(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        Vec3d {
            x: 4.5,
            y: 65.0,
            z: 8.5,
        },
        "",
    );
    assert!(arena.hears("minecraft:entity.strider.happy", 1500));
}
