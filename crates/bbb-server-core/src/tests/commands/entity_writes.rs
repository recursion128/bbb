//! `/damage` and `/effect`, whose answer is the live entity's.

use super::*;

struct Session {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
}

impl Session {
    fn new() -> Self {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            ..operator_config(&[("alice", 1)])
        })
        .unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.tick(receiver.drain());
        Self {
            world,
            sender,
            receiver,
        }
    }

    /// The translation keys of the lines `command` sent alice.
    fn run(&mut self, command: &str) -> Vec<String> {
        let tick = chat_command(&self.sender, &self.receiver, &mut self.world, 1, command);
        system_chats(&tick)
            .into_iter()
            .map(|(_, line)| line)
            .collect()
    }

    fn answered(&mut self, command: &str, key: &str) {
        let lines = self.run(command);
        assert!(
            lines.iter().any(|line| line.contains(key)),
            "{command}: {lines:?}"
        );
    }
}

/// `Player.hurtServer` (Player.java:684-686): a creative player refuses every
/// source outside `minecraft:bypasses_invulnerability`, which `/damage` reports
/// as `commands.damage.invulnerable` (DamageCommand.java:108); `out_of_world`
/// is in the tag and lands.
#[test]
fn damage_answers_with_what_the_live_player_took() {
    let mut session = Session::new();
    let health = |session: &Session| session.world.player_health_for_test(ConnectionId(1));
    session.answered("damage alice 6", "commands.damage.success");
    assert_eq!(health(&session), 14.0);

    session.run("gamemode creative alice");
    for _ in 0..11 {
        session.world.tick(Vec::new());
    }
    let before = health(&session);
    session.answered("damage alice 6", "commands.damage.invulnerable");
    assert!(health(&session) >= before);
    session.answered(
        "damage alice 6 minecraft:out_of_world",
        "commands.damage.success",
    );
    assert!(health(&session) <= before - 5.0);
}

/// `LivingEntity.addEffect` refuses an instance that would not outlast or
/// outrank the one held (`MobEffectInstance.update`), and `removeAllEffects`
/// / `removeEffect` answer false with nothing to remove
/// (`EffectCommands.java:196-226`).
#[test]
fn effect_answers_with_what_the_live_player_changed() {
    let mut session = Session::new();
    session.answered(
        "effect give alice speed 30",
        "commands.effect.give.success.single",
    );
    session.answered("effect give alice speed 10", "commands.effect.give.failed");
    session.answered(
        "effect clear alice speed",
        "commands.effect.clear.specific.success.single",
    );
    session.answered(
        "effect clear alice speed",
        "commands.effect.clear.specific.failed",
    );
    session.answered(
        "effect clear alice",
        "commands.effect.clear.everything.failed",
    );
}
