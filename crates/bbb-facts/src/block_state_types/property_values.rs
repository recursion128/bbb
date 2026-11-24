use super::{property_facts_26_1::java_bmp_decimal_digit, VanillaBlockPropertyKind};

pub fn vanilla_block_property_value_rank(
    kind: &VanillaBlockPropertyKind,
    value: &str,
) -> Option<i32> {
    property_value_rank(
        kind,
        |token| token == value,
        || java_integer(value.encode_utf16()),
    )
}

pub fn vanilla_block_property_value_rank_utf16(
    kind: &VanillaBlockPropertyKind,
    value: &[u16],
) -> Option<i32> {
    property_value_rank(
        kind,
        |token| token.encode_utf16().eq(value.iter().copied()),
        || java_integer(value.iter().copied()),
    )
}

fn property_value_rank(
    kind: &VanillaBlockPropertyKind,
    equals: impl Fn(&str) -> bool,
    integer: impl FnOnce() -> Option<i32>,
) -> Option<i32> {
    match kind {
        VanillaBlockPropertyKind::Boolean if equals("false") => Some(0),
        VanillaBlockPropertyKind::Boolean if equals("true") => Some(1),
        VanillaBlockPropertyKind::Boolean => None,
        VanillaBlockPropertyKind::Integer { min, max } => {
            let rank = integer()?;
            (*min <= rank && rank <= *max).then_some(rank)
        }
        VanillaBlockPropertyKind::Enum { values } => values
            .iter()
            .find_map(|(token, rank)| equals(token).then_some(*rank)),
    }
}

fn java_integer(mut units: impl Iterator<Item = u16>) -> Option<i32> {
    let first = units.next()?;
    let negative = first == u16::from(b'-');
    let first_digit = if negative || first == u16::from(b'+') {
        units.next()?
    } else {
        first
    };
    // Integer.parseInt accumulates negatively so MIN_VALUE stays representable.
    let mut result = 0_i32;
    for code in std::iter::once(first_digit).chain(units) {
        let digit = if (u16::from(b'0')..=u16::from(b'9')).contains(&code) {
            i32::from(code - u16::from(b'0'))
        } else {
            java_bmp_decimal_digit(code)?
        };
        result = result.checked_mul(10)?.checked_sub(digit)?;
    }
    if negative {
        Some(result)
    } else {
        result.checked_neg()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_state_types::{
        vanilla_block_has_property, vanilla_block_property_kind, vanilla_block_property_values,
        vanilla_block_state_identities, vanilla_block_state_property_matches_utf16,
    };

    #[test]
    fn property_facts_cover_every_state_and_canonical_value() {
        for state in vanilla_block_state_identities() {
            for (property, value) in &state.properties {
                let kind = vanilla_block_property_kind(&state.name, property).unwrap();
                let rank = vanilla_block_property_value_rank(kind, value);
                assert!(rank.is_some());
                assert_eq!(
                    vanilla_block_property_value_rank_utf16(
                        kind,
                        &value.encode_utf16().collect::<Vec<_>>()
                    ),
                    rank,
                );
                assert!(vanilla_block_state_property_matches_utf16(
                    state.id,
                    &property.encode_utf16().collect::<Vec<_>>(),
                    &value.encode_utf16().collect::<Vec<_>>(),
                ));
            }
        }
        for (block, property, kind) in
            super::super::property_facts_26_1::vanilla_block_property_kinds()
        {
            assert!(vanilla_block_has_property(block, property));
            assert_eq!(vanilla_block_property_kind(block, property), Some(kind));
        }
        assert!(vanilla_block_property_kind("minecraft:stone", "missing").is_none());
        assert!(vanilla_block_property_kind("minecraft:missing", "age").is_none());
    }

    #[test]
    fn property_facts_preserve_comparable_order_instead_of_possible_value_order() {
        let kind = vanilla_block_property_kind("minecraft:pointed_dripstone", "vertical_direction")
            .unwrap();
        assert_eq!(
            vanilla_block_property_values("minecraft:pointed_dripstone", "vertical_direction")
                .unwrap(),
            &["up", "down"]
        );
        assert_eq!(vanilla_block_property_value_rank(kind, "down"), Some(0));
        assert_eq!(vanilla_block_property_value_rank(kind, "up"), Some(1));
        assert_eq!(vanilla_block_property_value_rank(kind, "north"), None);
        assert_eq!(
            vanilla_block_property_value_rank(&VanillaBlockPropertyKind::Boolean, "false"),
            Some(0)
        );
        assert_eq!(
            vanilla_block_property_value_rank(&VanillaBlockPropertyKind::Boolean, "true"),
            Some(1)
        );
        assert_eq!(
            vanilla_block_property_value_rank(&VanillaBlockPropertyKind::Boolean, "TRUE"),
            None
        );
    }

    #[test]
    fn property_integer_values_follow_java_utf16_digits_signs_and_bounds() {
        let kind = vanilla_block_property_kind("minecraft:wheat", "age").unwrap();
        assert_eq!(*kind, VanillaBlockPropertyKind::Integer { min: 0, max: 7 });
        for value in ["7", "+007", "٧", "７", "०७"] {
            assert_eq!(
                vanilla_block_property_value_rank(kind, value),
                Some(7),
                "{value}"
            );
            assert_eq!(
                vanilla_block_property_value_rank_utf16(
                    kind,
                    &value.encode_utf16().collect::<Vec<_>>()
                ),
                Some(7),
                "{value}"
            );
        }
        assert_eq!(vanilla_block_property_value_rank(kind, "-0"), Some(0));
        for value in [
            "",
            "+",
            "-",
            " 7",
            "7 ",
            "8",
            "-1",
            "²",
            "𝟟",
            "−7",
            "2147483648",
            "-2147483649",
        ] {
            assert_eq!(
                vanilla_block_property_value_rank(kind, value),
                None,
                "{value}"
            );
            assert_eq!(
                vanilla_block_property_value_rank_utf16(
                    kind,
                    &value.encode_utf16().collect::<Vec<_>>()
                ),
                None,
                "{value}"
            );
        }
        let all = VanillaBlockPropertyKind::Integer {
            min: i32::MIN,
            max: i32::MAX,
        };
        assert_eq!(
            vanilla_block_property_value_rank(&all, "-2147483648"),
            Some(i32::MIN)
        );
        assert_eq!(
            vanilla_block_property_value_rank(&all, "+2147483647"),
            Some(i32::MAX)
        );
    }
}
