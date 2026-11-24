//! Permission levels and what an unpermitted command looks like.

use super::*;

#[test]
fn permission_order_matches_the_vanilla_level_ladder() {
    assert!(CommandPermission::All < CommandPermission::Moderator);
    assert!(CommandPermission::Moderator < CommandPermission::GameMaster);
    assert!(CommandPermission::GameMaster < CommandPermission::Admin);
    assert!(CommandPermission::Admin < CommandPermission::Owner);
    for (index, level) in LEVELS.iter().enumerate() {
        assert_eq!(level.id(), index as u8);
        assert_eq!(CommandPermission::from_id(index as u8), Some(*level));
        assert_eq!(level.entity_event_id(), 24 + index as i8);
    }
    assert_eq!(CommandPermission::from_id(5), None);
}

#[test]
fn an_unpermitted_command_is_indistinguishable_from_an_unknown_one() {
    // `requires()` removes the node from parsing, so the refusal names no
    // command and reads exactly like a misspelling.
    for command in ["say hello", "stop", "ban alice"] {
        let outcome = refused(command, CommandPermission::All);
        assert_eq!(
            translate_key(failure_messages(&outcome)[0]).as_deref(),
            Some("command.unknown.command"),
            "{command} must not leak that it exists"
        );
    }
}
