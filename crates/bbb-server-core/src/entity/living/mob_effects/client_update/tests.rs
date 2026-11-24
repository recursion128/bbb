use bbb_protocol::mob_effect_types::{
    VANILLA_MOB_EFFECT_INVISIBILITY_ID, VANILLA_MOB_EFFECT_SPEED_ID,
};

use super::*;

#[test]
fn update_packet_carries_the_vanilla_flag_bits_and_registry_id() {
    // `ClientboundUpdateMobEffectPacket` (`ClientboundUpdateMobEffectPacket.java:25-48`)
    // over `MobEffect.STREAM_CODEC`'s registry id (`:52`, `:60`).
    let effect = ActiveMobEffect::new("minecraft:speed", 200, 2).unwrap();
    let packet = update_mob_effect_packet(7, &effect, false);

    assert_eq!(packet.entity_id, 7);
    assert_eq!(packet.effect_id, VANILLA_MOB_EFFECT_SPEED_ID);
    assert_eq!(packet.amplifier, 2);
    assert_eq!(packet.duration_ticks, 200);
    // `ActiveMobEffect::new` leaves ambient false and both show flags true, so
    // FLAG_VISIBLE | FLAG_SHOW_ICON.
    assert_eq!(packet.flags.raw, 2 | 4);
    assert!(!packet.flags.ambient);
    assert!(packet.flags.visible);
    assert!(packet.flags.show_icon);
    assert!(!packet.flags.blend);
}

#[test]
fn update_packet_sets_every_flag_the_instance_asks_for() {
    let effect =
        ActiveMobEffect::from_saved("minecraft:invisibility", 40, 0, true, true, true).unwrap();
    let packet = update_mob_effect_packet(1, &effect, true);

    assert_eq!(packet.effect_id, VANILLA_MOB_EFFECT_INVISIBILITY_ID);
    assert_eq!(packet.flags.raw, 1 | 2 | 4 | 8);
    assert!(packet.flags.ambient);
    assert!(packet.flags.blend);
}

#[test]
fn update_packet_clears_the_flags_a_hidden_ambient_effect_does_not_set() {
    // `show_icon` defaults to `show_particles` in the details codec, so an
    // invisible instance sets neither FLAG_VISIBLE nor FLAG_SHOW_ICON.
    let effect =
        ActiveMobEffect::from_saved("minecraft:speed", 40, 0, false, false, false).unwrap();
    let packet = update_mob_effect_packet(1, &effect, false);

    assert_eq!(packet.flags.raw, 0);
    assert!(!packet.flags.visible);
    assert!(!packet.flags.show_icon);
}

#[test]
fn update_packet_forwards_an_infinite_duration_unchanged() {
    // `getDuration` is written straight to the wire (`:29`, `:62`); the client
    // reads `-1` back as the infinite sentinel.
    let effect = ActiveMobEffect::from_saved("minecraft:speed", -1, 0, false, true, true).unwrap();

    assert_eq!(
        update_mob_effect_packet(3, &effect, false).duration_ticks,
        -1
    );
}

#[test]
fn remove_packet_names_the_entity_and_the_effect() {
    let effect = ActiveMobEffect::new("minecraft:speed", 200, 0).unwrap();
    let packet = remove_mob_effect_packet(9, &effect);

    assert_eq!(packet.entity_id, 9);
    assert_eq!(packet.effect_id, VANILLA_MOB_EFFECT_SPEED_ID);
}

#[test]
fn update_packet_keeps_the_visible_and_icon_bits_distinct() {
    // FLAG_VISIBLE is `isVisible` and FLAG_SHOW_ICON is `showIcon`
    // (`ClientboundUpdateMobEffectPacket.java:35-41`), so an instance that hides
    // its particles while keeping its icon sets only the latter.
    let effect = ActiveMobEffect::from_saved("minecraft:speed", 40, 0, false, false, true).unwrap();
    let packet = update_mob_effect_packet(1, &effect, false);

    assert_eq!(packet.flags.raw, 4);
    assert!(!packet.flags.visible);
    assert!(packet.flags.show_icon);
}
