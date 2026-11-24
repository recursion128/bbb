//! Player-name validation (`StringUtil.java:66-68`).

pub const MAX_PLAYER_NAME_UTF16_UNITS: usize = 16;

pub fn valid_player_name(name: &str) -> bool {
    name.len() <= MAX_PLAYER_NAME_UTF16_UNITS && name.bytes().all(|byte| (33..=126).contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_printable_ascii_range_up_to_sixteen_units() {
        assert!(valid_player_name("bbb-client"));
        assert!(valid_player_name("0123456789abcdef"));
        assert!(valid_player_name("!~"));
    }

    #[test]
    fn rejects_names_outside_the_vanilla_filter() {
        assert!(!valid_player_name("with space"));
        assert!(!valid_player_name("del\u{7f}"));
        assert!(!valid_player_name("é"));
        assert!(!valid_player_name("0123456789abcdef g"));
        assert!(!valid_player_name("0123456789abcdefg"));
    }

    #[test]
    fn an_empty_name_passes_the_vanilla_predicate() {
        // `isValidPlayerName` only bounds the length and filters characters, so
        // the empty string passes it; callers that need a non-empty identity
        // (login, the administration surface) bound that separately.
        assert!(valid_player_name(""));
    }
}
