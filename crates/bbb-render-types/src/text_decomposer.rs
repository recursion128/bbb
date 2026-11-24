#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyTextFormat {
    Color(u32),
    Obfuscated,
    Bold,
    Strikethrough,
    Underlined,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceTextChar {
    pub ch: char,
    pub utf16_index: usize,
    pub scalar_index: usize,
}

pub fn visit_legacy_text<S: Clone>(
    text: &str,
    style: S,
    reset: &S,
    apply: impl FnMut(&mut S, LegacyTextFormat),
    output: impl FnMut(SourceTextChar, &S) -> bool,
) -> bool {
    visit_legacy_utf16(
        &text.encode_utf16().collect::<Vec<_>>(),
        style,
        reset,
        apply,
        output,
    )
}

// StringDecomposer.java:84-128: codes consume one UTF-16 unit, even for unknown formats.
pub fn visit_legacy_utf16<S: Clone>(
    text: &[u16],
    mut style: S,
    reset: &S,
    mut apply: impl FnMut(&mut S, LegacyTextFormat),
    mut output: impl FnMut(SourceTextChar, &S) -> bool,
) -> bool {
    // Paired units share a source scalar index even when formatting consumes the high unit.
    let mut scalar_count = 0usize;
    let units = text
        .iter()
        .copied()
        .enumerate()
        .map(|(i, unit)| {
            let paired_low = (0xdc00..=0xdfff).contains(&unit)
                && i > 0
                && (0xd800..=0xdbff).contains(&text[i - 1]);
            let index = if paired_low {
                scalar_count - 1
            } else {
                let index = scalar_count;
                scalar_count += 1;
                index
            };
            (unit, index)
        })
        .collect::<Vec<_>>();
    let mut i = 0;
    while i < units.len() {
        let (unit, scalar_index) = units[i];
        if unit == 0xa7 {
            let Some(&(code, _)) = units.get(i + 1) else {
                break;
            };
            if let Some(code) = char::from_u32(u32::from(code)) {
                if code.eq_ignore_ascii_case(&'r') {
                    style = reset.clone();
                } else if let Some(format) = legacy_format(code) {
                    apply(&mut style, format);
                }
            }
            i += 2;
            continue;
        }
        let utf16_index = i;
        let ch = if (0xd800..=0xdbff).contains(&unit) {
            if let Some(&(low, _)) = units
                .get(i + 1)
                .filter(|(low, _)| (0xdc00..=0xdfff).contains(low))
            {
                i += 1;
                char::from_u32(
                    0x10000 + ((u32::from(unit) - 0xd800) << 10) + u32::from(low) - 0xdc00,
                )
                .unwrap()
            } else {
                char::REPLACEMENT_CHARACTER
            }
        } else {
            char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER)
        };
        if !output(
            SourceTextChar {
                ch,
                utf16_index,
                scalar_index,
            },
            &style,
        ) {
            return false;
        }
        i += 1;
    }
    true
}

fn legacy_format(code: char) -> Option<LegacyTextFormat> {
    use LegacyTextFormat::*;
    // ChatFormatting.java:129-138: Unicode lowercase includes the Kelvin sign.
    Some(match code.to_lowercase().next().unwrap_or(code) {
        '0' => Color(0x000000),
        '1' => Color(0x0000aa),
        '2' => Color(0x00aa00),
        '3' => Color(0x00aaaa),
        '4' => Color(0xaa0000),
        '5' => Color(0xaa00aa),
        '6' => Color(0xffaa00),
        '7' => Color(0xaaaaaa),
        '8' => Color(0x555555),
        '9' => Color(0x5555ff),
        'a' => Color(0x55ff55),
        'b' => Color(0x55ffff),
        'c' => Color(0xff5555),
        'd' => Color(0xff55ff),
        'e' => Color(0xffff55),
        'f' => Color(0xffffff),
        'k' => Obfuscated,
        'l' => Bold,
        'm' => Strikethrough,
        'n' => Underlined,
        'o' => Italic,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_text_raw_utf16_retains_orphan_units_and_part_local_indices() {
        let mut chars = Vec::new();
        visit_legacy_utf16(
            &[0xd800, 65, 0xdc00, 0xa7, 0xd83d, 0xde00],
            (),
            &(),
            |_, _| {},
            |source, _| {
                chars.push(source);
                true
            },
        );
        assert_eq!(
            chars,
            [
                SourceTextChar {
                    ch: '�',
                    utf16_index: 0,
                    scalar_index: 0
                },
                SourceTextChar {
                    ch: 'A',
                    utf16_index: 1,
                    scalar_index: 1
                },
                SourceTextChar {
                    ch: '�',
                    utf16_index: 2,
                    scalar_index: 2
                },
                SourceTextChar {
                    ch: '�',
                    utf16_index: 5,
                    scalar_index: 4
                },
            ]
        );
        let mut tail = Vec::new();
        visit_legacy_utf16(
            &[0xde00],
            (),
            &(),
            |_, _| {},
            |source, _| {
                tail.push(source);
                true
            },
        );
        assert_eq!(
            tail,
            [SourceTextChar {
                ch: '�',
                utf16_index: 0,
                scalar_index: 0
            }]
        );
    }

    #[test]
    fn legacy_text_consumes_utf16_units_and_preserves_source_indices() {
        let mut chars = Vec::new();
        assert!(visit_legacy_text(
            "😀§😀X§?Y§",
            (),
            &(),
            |_, _| {},
            |ch, _| {
                chars.push(ch);
                true
            }
        ));
        assert_eq!(
            chars,
            vec![
                SourceTextChar {
                    ch: '😀',
                    utf16_index: 0,
                    scalar_index: 0
                },
                SourceTextChar {
                    ch: '�',
                    utf16_index: 4,
                    scalar_index: 2
                },
                SourceTextChar {
                    ch: 'X',
                    utf16_index: 5,
                    scalar_index: 3
                },
                SourceTextChar {
                    ch: 'Y',
                    utf16_index: 8,
                    scalar_index: 6
                },
            ]
        );
    }

    #[test]
    fn legacy_text_reset_restores_complete_metadata_and_stops_at_sink() {
        let mut seen = Vec::new();
        assert!(!visit_legacy_text(
            "§KA§RBignored",
            ("current", None),
            &("reset", None),
            |style, format| style.1 = Some(format),
            |ch, style| {
                seen.push((ch.ch, *style));
                ch.ch != 'B'
            }
        ));
        assert_eq!(
            seen,
            [
                ('A', ("current", Some(LegacyTextFormat::Obfuscated))),
                ('B', ("reset", None))
            ]
        );
    }

    #[test]
    fn legacy_text_never_joins_codes_across_parts() {
        let mut seen = String::new();
        for text in ["§", "aX"] {
            visit_legacy_text(
                text,
                (),
                &(),
                |_, _| {},
                |ch, _| {
                    seen.push(ch.ch);
                    true
                },
            );
        }
        assert_eq!(seen, "aX");
    }
}
