//! The two `Item` subclasses that hold a use without consuming anything:
//! `SpyglassItem` (`SpyglassItem.java:11-51`) and `InstrumentItem`
//! (`InstrumentItem.java:30-66`).

use super::*;

fn entity_sounds(tick: &TickResult, viewer: u64) -> Vec<(&'static str, f32)> {
    packets_to(tick, viewer)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SoundEntity(sound) => match sound.sound {
                SoundEventHolder::Reference { registry_id } => {
                    vanilla_sound_event_resource_id(registry_id).map(|id| (id, sound.volume))
                }
                SoundEventHolder::Direct { .. } => None,
            },
            _ => None,
        })
        .collect()
}

fn cooldown_durations(tick: &TickResult) -> Vec<i32> {
    packets_to(tick, ALICE)
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::Cooldown(cooldown) => Some(cooldown.duration),
            _ => None,
        })
        .collect()
}

/// `SpyglassItem.use` plays its sound for everyone but the user and counts
/// the use; letting go runs `releaseUsing`'s stop sound.
#[test]
fn a_spyglass_is_held_until_released() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:spyglass", 1));
    let alice = fixture.alice;

    let raised = fixture.use_item(InteractionHand::MainHand);
    assert_eq!(sounds(&raised, BOB), ["minecraft:item.spyglass.use"]);
    assert!(sounds(&raised, ALICE).is_empty());
    assert_eq!(living_flags(&raised, BOB, alice), [1]);
    assert_eq!(fixture.use_state().2, 1199);
    assert_eq!(fixture.item_used("minecraft:spyglass"), 1);

    fixture.idle();
    let released = fixture.action(PlayerActionKind::ReleaseUseItem);
    assert_eq!(
        sounds(&released, BOB),
        ["minecraft:item.spyglass.stop_using"]
    );
    assert!(sounds(&released, ALICE).is_empty());
    assert_eq!(living_flags(&released, BOB, alice), [0]);
    assert!(entity_events(&released, ALICE, alice).is_empty());
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);
}

/// `USE_DURATION` is a minute: the last tick finishes the use through
/// `finishUsingItem`, which stops it with the same sound.
#[test]
fn a_spyglass_held_for_a_minute_finishes_its_use() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:spyglass", 1));
    let alice = fixture.alice;
    fixture.use_item(InteractionHand::MainHand);
    for _ in 1..1199 {
        fixture.idle();
    }
    assert_eq!(fixture.use_state().2, 1);

    let finished = fixture.idle();
    assert_eq!(entity_events(&finished, ALICE, alice), [9]);
    assert_eq!(
        sounds(&finished, BOB),
        ["minecraft:item.spyglass.stop_using"]
    );
    assert!(!fixture.use_state().0);
    assert_eq!(living_flags(&fixture.idle(), BOB, alice), [0]);
}

/// `InstrumentItem.use`: the horn's instrument is `ponder_goat_horn`
/// (seven seconds, 256 blocks), played at `range / 16` volume to everyone
/// else, held for its duration and cooled down for as long.
#[test]
fn a_goat_horn_plays_to_others_and_cools_down_for_its_duration() {
    let mut fixture = Fixture::new(GameType::Survival);
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:goat_horn", 1));
    let alice = fixture.alice;

    let played = fixture.use_item(InteractionHand::MainHand);
    assert_eq!(
        entity_sounds(&played, BOB),
        [("minecraft:item.goat_horn.sound.0", 16.0)]
    );
    assert!(entity_sounds(&played, ALICE).is_empty());
    assert_eq!(cooldown_durations(&played), [140]);
    assert_eq!(living_flags(&played, BOB, alice), [1]);
    assert_eq!(fixture.use_state().2, 139);
    assert_eq!(fixture.item_used("minecraft:goat_horn"), 1);

    for _ in 1..139 {
        fixture.idle();
    }
    let finished = fixture.idle();
    assert_eq!(entity_events(&finished, ALICE, alice), [9]);
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.slot(HOTBAR_SLOT).count, 1);

    // The cooldown runs out on the use's own last tick, so the horn plays
    // again straight away.
    let again = fixture.use_item(InteractionHand::MainHand);
    assert_eq!(
        entity_sounds(&again, BOB),
        [("minecraft:item.goat_horn.sound.0", 16.0)]
    );
}

/// `ServerPlayerGameMode.useItem`'s cooldown gate (`:308-310`): a horn
/// still cooling down is not played.
#[test]
fn a_cooling_goat_horn_does_not_play() {
    let mut fixture = Fixture::with_config(GameType::Survival, ServerWorldConfig::default());
    fixture.hold(HOTBAR_MENU_SLOT, stack("minecraft:goat_horn", 1));
    fixture.use_item(InteractionHand::MainHand);
    fixture.action(PlayerActionKind::ReleaseUseItem);

    let refused = fixture.use_item(InteractionHand::MainHand);
    assert!(entity_sounds(&refused, BOB).is_empty());
    assert!(!fixture.use_state().0);
    assert_eq!(fixture.item_used("minecraft:goat_horn"), 1);
}
