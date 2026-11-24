//! `EvokerSummonSpellGoal.performSpellCasting` (`Evoker.java:244-264`): the
//! vexes join the evoker's team and their placement names the evoker.

use super::melee::{fixture, PLAYER_POSITION};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_VEX_ID};

#[test]
fn summoned_vexes_join_the_evokers_team() {
    let (sender, receiver, mut world) = fixture();
    let (evoker, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 6.0,
            ..PLAYER_POSITION
        },
    );
    for (id, command) in [
        (20, "team add coven".to_owned()),
        (21, format!("team join coven {uuid}")),
    ] {
        let tick = console_command(&sender, &receiver, &mut world, id, &command);
        assert!(console_command_answer(&tick).0.is_some(), "{command}");
    }
    let mut vex = None;
    for _ in 0..200 {
        let tick = world.tick(receiver.drain());
        vex = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                connection_id: super::melee::ALICE,
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.entity_type_id == VANILLA_ENTITY_TYPE_VEX_ID => Some(add.uuid),
            _ => None,
        });
        if vex.is_some() {
            break;
        }
    }
    let vex = vex.expect("the evoker summons vexes");
    assert_eq!(
        world
            .scoreboard()
            .players_team(&vex.to_string())
            .map(|team| &team.name),
        Some(&bbb_protocol::JavaString::from("coven"))
    );
    assert!(world
        .game_events(DIMENSION)
        .0
        .iter()
        .any(
            |event| event.game_event == crate::level_event::VanillaGameEvent::ENTITY_PLACE
                && event.source_entity_id == Some(evoker)
        ));
}

/// `SpellcasterIllager.java:47`: a cast in progress is written as `SpellTicks`.
#[test]
fn a_cast_in_progress_is_saved_as_spell_ticks() {
    let (_sender, receiver, mut world) = fixture();
    let (evoker, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        Vec3d {
            x: PLAYER_POSITION.x + 6.0,
            ..PLAYER_POSITION
        },
    );
    let spell_ticks = |world: &ServerWorld| {
        let NbtSummaryValue::Compound(fields) = world
            .stored_entity_command_data_for_test(DIMENSION, evoker)
            .unwrap()
        else {
            panic!("compound record")
        };
        fields
            .into_iter()
            .find(|field| field.name == "SpellTicks")
            .map(|field| field.value)
    };
    let casting = (0..200).any(|_| {
        world.tick(receiver.drain());
        matches!(spell_ticks(&world), Some(NbtSummaryValue::Int(ticks)) if ticks > 0)
    });
    assert!(casting, "the evoker starts a cast and saves its countdown");
}
