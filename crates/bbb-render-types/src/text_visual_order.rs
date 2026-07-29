use std::{char::decode_utf16, ops::Range};

use unicode_bidi::{utf16::BidiInfo, BidiClass, Level};
use unicode_character_database::bidi_mirroring_glyph::BIDI_MIRRORING_GLYPH;

const IRRELEVANT: u16 = 4;
const LAM_TYPE: u16 = 16;
const ALEF_TYPE: u16 = 32;
const LINK_R: u16 = 1;
const LINK_L: u16 = 2;
const LINK_MASK: usize = 3;
const LAMALEF_SPACE_SUB: u16 = 0xffff;

// ICU4J ArabicShaping's isolated/initial/medial/final selection table.
const SHAPE_TABLE: [[[u16; 4]; 4]; 4] = [
    [[0, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 3], [0, 1, 0, 1]],
    [[0, 0, 2, 2], [0, 0, 1, 2], [0, 1, 1, 2], [0, 1, 1, 3]],
    [[0, 0, 0, 0], [0, 0, 0, 0], [0, 1, 0, 3], [0, 1, 0, 3]],
    [[0, 0, 1, 2], [0, 0, 1, 2], [0, 1, 1, 2], [0, 1, 1, 3]],
];

const TASHKEEL_FORM_OFFSETS: [u16; 8] = [0, 2, 4, 6, 8, 10, 12, 14];

/// Applies vanilla 26.1's formatted visual-order operation to one logical text value.
///
/// The metadata value models `SubStringSource`'s UTF-16-indexed style. It is repeated for
/// supplementary codepoints, follows visual runs, and is retained across Arabic Lam-Alef
/// compaction exactly as vanilla retains its original style vector.
pub fn visual_order<T: Clone + Eq>(
    logical: &[(char, T)],
    default_right_to_left: bool,
) -> Vec<(char, T)> {
    if logical.is_empty() {
        return Vec::new();
    }

    let mut logical_units = Vec::new();
    let mut metadata = Vec::new();
    for (ch, value) in logical {
        let mut encoded = [0; 2];
        let units = ch.encode_utf16(&mut encoded);
        logical_units.extend_from_slice(units);
        metadata.extend(std::iter::repeat(value.clone()).take(units.len()));
    }

    let shaped = shape_arabic_icu8(&logical_units);
    if shaped.len() > metadata.len() {
        return logical.to_vec();
    }
    metadata.truncate(shaped.len());

    let auto = BidiInfo::new(&shaped, None);
    let mut output = Vec::with_capacity(logical.len());
    for paragraph in &auto.paragraphs {
        let classes = &auto.original_classes[paragraph.range.clone()];
        let level = paragraph_level(classes, paragraph.level, default_right_to_left);
        let paragraph_units = &shaped[paragraph.range.clone()];
        let paragraph_metadata = &metadata[paragraph.range.clone()];
        let bidi = unicode_bidi::utf16::ParagraphBidiInfo::new(paragraph_units, Some(level));
        let (levels, runs) = bidi.visual_runs(0..paragraph_units.len());
        for run in runs {
            emit_visual_run(
                paragraph_units,
                paragraph_metadata,
                run.clone(),
                levels[run.start].is_rtl(),
                &mut output,
            );
        }
    }
    output
}

fn paragraph_level(classes: &[BidiClass], auto_level: Level, default_right_to_left: bool) -> Level {
    if !default_right_to_left || first_strong_class(classes).is_some() {
        auto_level
    } else {
        Level::rtl()
    }
}

fn first_strong_class(classes: &[BidiClass]) -> Option<BidiClass> {
    let mut isolate_depth = 0usize;
    for class in classes {
        match class {
            BidiClass::LRI | BidiClass::RLI | BidiClass::FSI => {
                isolate_depth = isolate_depth.saturating_add(1);
            }
            BidiClass::PDI if isolate_depth > 0 => isolate_depth -= 1,
            BidiClass::B if isolate_depth == 0 => break,
            BidiClass::L | BidiClass::R | BidiClass::AL if isolate_depth == 0 => {
                return Some(*class);
            }
            _ => {}
        }
    }
    None
}

fn emit_visual_run<T: Clone + Eq>(
    units: &[u16],
    metadata: &[T],
    run: Range<usize>,
    reverse: bool,
    output: &mut Vec<(char, T)>,
) {
    if run.is_empty() {
        return;
    }

    let mut parts = Vec::new();
    let mut part_start = run.start;
    for index in (run.start + 1)..run.end {
        if metadata[index] != metadata[part_start] {
            parts.push(part_start..index);
            part_start = index;
        }
    }
    parts.push(part_start..run.end);

    if reverse {
        for part in parts.into_iter().rev() {
            let value = metadata[part.start].clone();
            let chars = decode_utf16(units[part].iter().copied())
                .map(|result| result.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect::<Vec<_>>();
            output.extend(
                chars
                    .into_iter()
                    .rev()
                    .map(|ch| (mirrored(ch), value.clone())),
            );
        }
    } else {
        for part in parts {
            let value = metadata[part.start].clone();
            output.extend(
                decode_utf16(units[part].iter().copied())
                    .map(|result| (result.unwrap_or(char::REPLACEMENT_CHARACTER), value.clone())),
            );
        }
    }
}

fn mirrored(ch: char) -> char {
    let codepoint = ch as u32;
    let Ok(index) = BIDI_MIRRORING_GLYPH.binary_search_by_key(&codepoint, |entry| entry.0) else {
        return ch;
    };
    char::from_u32(BIDI_MIRRORING_GLYPH[index].1).unwrap_or(ch)
}

// This is a narrow behavioral implementation of ICU4J 77.1 `ArabicShaping(8)`.
// Option 8 means logical input, Lam-Alef resize, letter shaping, and no digit
// conversion. The link facts and selection order come from Unicode ICU's
// ArabicShaping.java:
// https://github.com/unicode-org/icu/blob/release-77-1/icu4j/main/core/src/main/java/com/ibm/icu/text/ArabicShaping.java
fn shape_arabic_icu8(input: &[u16]) -> Vec<u16> {
    if input.is_empty() {
        return Vec::new();
    }

    // ICU's logical mode shapes a temporary visual-RTL buffer, then reverses it back.
    let mut dest = input.iter().rev().copied().collect::<Vec<_>>();
    normalize_presentation_forms(&mut dest);

    let mut lamalef_found = false;
    let mut index = dest.len() as isize - 1;
    let mut current_link = arabic_link(dest[index as usize]);
    let mut next_link = 0;
    let mut previous_link = 0;
    let mut last_link = 0;
    let mut last_position = index as usize;
    let mut next_position = -2isize;

    while index >= 0 {
        let current_position = index as usize;
        if current_link & 0xff00 != 0 || is_tashkeel(dest[current_position]) {
            let mut scan = index - 1;
            loop {
                if scan == -1 {
                    next_link = 0;
                    next_position = isize::MAX;
                    break;
                }
                next_link = arabic_link(dest[scan as usize]);
                if next_link & IRRELEVANT == 0 {
                    next_position = scan;
                    break;
                }
                scan -= 1;
            }

            if current_link & ALEF_TYPE != 0 && last_link & LAM_TYPE != 0 {
                let lamalef = normalized_lamalef(dest[current_position]);
                if lamalef != 0 {
                    lamalef_found = true;
                    dest[current_position] = LAMALEF_SPACE_SUB;
                    dest[last_position] = lamalef;
                    index = last_position as isize;
                }
                last_link = previous_link;
                current_link = arabic_link(lamalef);
            }

            let position = index as usize;
            let special = special_char(dest[position]);
            let mut shape = SHAPE_TABLE[(next_link & LINK_MASK as u16) as usize]
                [(last_link & LINK_MASK as u16) as usize]
                [(current_link & LINK_MASK as u16) as usize];
            if special == 1 {
                shape &= 1;
            } else if special == 2 {
                if last_link & LINK_L != 0
                    && next_link & LINK_R != 0
                    && dest[position] != 0x064c
                    && dest[position] != 0x064d
                    && !(next_link & ALEF_TYPE == ALEF_TYPE && last_link & LAM_TYPE == LAM_TYPE)
                {
                    shape = 1;
                } else {
                    shape = 0;
                }
            }

            if special == 2 {
                dest[position] =
                    0xfe70 + TASHKEEL_FORM_OFFSETS[(dest[position] - 0x064b) as usize] + shape;
            } else {
                dest[position] = 0xfe70 + (current_link >> 8) + shape;
            }
        }

        if current_link & IRRELEVANT == 0 {
            previous_link = last_link;
            last_link = current_link;
            last_position = index as usize;
        }

        index -= 1;
        if index == next_position {
            current_link = next_link;
            next_position = -2;
        } else if index != -1 {
            current_link = arabic_link(dest[index as usize]);
        }
    }

    if lamalef_found {
        dest.retain(|unit| *unit != LAMALEF_SPACE_SUB);
    }
    dest.reverse();
    dest
}

fn normalize_presentation_forms(units: &mut [u16]) {
    for unit in units {
        *unit = match *unit {
            0xfe70..=0xfe7f => 0x064b + (*unit - 0xfe70) / 2,
            0xfe80 => 0x0621,
            0xfe81..=0xfe82 => 0x0622,
            0xfe83..=0xfe84 => 0x0623,
            0xfe85..=0xfe86 => 0x0624,
            0xfe87..=0xfe88 => 0x0625,
            0xfe89..=0xfe8c => 0x0626,
            0xfe8d..=0xfe8e => 0x0627,
            0xfe8f..=0xfe92 => 0x0628,
            0xfe93..=0xfe94 => 0x0629,
            0xfe95..=0xfe98 => 0x062a,
            0xfe99..=0xfe9c => 0x062b,
            0xfe9d..=0xfea0 => 0x062c,
            0xfea1..=0xfea4 => 0x062d,
            0xfea5..=0xfea8 => 0x062e,
            0xfea9..=0xfeaa => 0x062f,
            0xfeab..=0xfeac => 0x0630,
            0xfead..=0xfeae => 0x0631,
            0xfeaf..=0xfeb0 => 0x0632,
            0xfeb1..=0xfeb4 => 0x0633,
            0xfeb5..=0xfeb8 => 0x0634,
            0xfeb9..=0xfebc => 0x0635,
            0xfebd..=0xfec0 => 0x0636,
            0xfec1..=0xfec4 => 0x0637,
            0xfec5..=0xfec8 => 0x0638,
            0xfec9..=0xfecc => 0x0639,
            0xfecd..=0xfed0 => 0x063a,
            0xfed1..=0xfed4 => 0x0641,
            0xfed5..=0xfed8 => 0x0642,
            0xfed9..=0xfedc => 0x0643,
            0xfedd..=0xfee0 => 0x0644,
            0xfee1..=0xfee4 => 0x0645,
            0xfee5..=0xfee8 => 0x0646,
            0xfee9..=0xfeec => 0x0647,
            0xfeed..=0xfeee => 0x0648,
            0xfeef..=0xfef0 => 0x0649,
            0xfef1..=0xfef4 => 0x064a,
            0xfef5..=0xfef6 => 0x065c,
            0xfef7..=0xfef8 => 0x065d,
            0xfef9..=0xfefa => 0x065e,
            0xfefb..=0xfefc => 0x065f,
            unit => unit,
        };
    }
}

fn normalized_lamalef(unit: u16) -> u16 {
    match unit {
        0x0622 => 0x065c,
        0x0623 => 0x065d,
        0x0625 => 0x065e,
        0x0627 => 0x065f,
        _ => 0,
    }
}

fn special_char(unit: u16) -> u16 {
    if (0x0622..=0x0625).contains(&unit)
        || unit == 0x0627
        || (0x062f..=0x0632).contains(&unit)
        || (0x0648..=0x0649).contains(&unit)
        || unit == 0x0629
    {
        1
    } else if (0x064b..=0x0652).contains(&unit) {
        2
    } else if (0x0653..=0x0655).contains(&unit)
        || unit == 0x0670
        || (0xfe70..=0xfe7f).contains(&unit)
    {
        3
    } else {
        0
    }
}

fn is_tashkeel(unit: u16) -> bool {
    (0x064b..=0x0652).contains(&unit)
}

fn form_link(right: bool, left: bool, form_offset: u16) -> u16 {
    u16::from(right) * LINK_R + u16::from(left) * LINK_L + form_offset * 256
}

fn arabic_link(unit: u16) -> u16 {
    match unit {
        0x0622 => form_link(true, false, 0x11) | ALEF_TYPE,
        0x0623 => form_link(true, false, 0x13) | ALEF_TYPE,
        0x0624 => form_link(true, false, 0x15),
        0x0625 => form_link(true, false, 0x17) | ALEF_TYPE,
        0x0626 => form_link(true, true, 0x19),
        0x0627 => form_link(true, false, 0x1d) | ALEF_TYPE,
        0x0628 => form_link(true, true, 0x1f),
        0x0629 => form_link(true, false, 0x23),
        0x062a => form_link(true, true, 0x25),
        0x062b => form_link(true, true, 0x29),
        0x062c => form_link(true, true, 0x2d),
        0x062d => form_link(true, true, 0x31),
        0x062e => form_link(true, true, 0x35),
        0x062f => form_link(true, false, 0x39),
        0x0630 => form_link(true, false, 0x3b),
        0x0631 => form_link(true, false, 0x3d),
        0x0632 => form_link(true, false, 0x3f),
        0x0633 => form_link(true, true, 0x41),
        0x0634 => form_link(true, true, 0x45),
        0x0635 => form_link(true, true, 0x49),
        0x0636 => form_link(true, true, 0x4d),
        0x0637 => form_link(true, true, 0x51),
        0x0638 => form_link(true, true, 0x55),
        0x0639 => form_link(true, true, 0x59),
        0x063a => form_link(true, true, 0x5d),
        0x0640 => LINK_R | LINK_L,
        0x0641 => form_link(true, true, 0x61),
        0x0642 => form_link(true, true, 0x65),
        0x0643 => form_link(true, true, 0x69),
        0x0644 => form_link(true, true, 0x6d) | LAM_TYPE,
        0x0645 => form_link(true, true, 0x71),
        0x0646 => form_link(true, true, 0x75),
        0x0647 => form_link(true, true, 0x79),
        0x0648 => form_link(true, false, 0x7d),
        0x0649 => form_link(true, false, 0x7f),
        0x064a => form_link(true, true, 0x81),
        0x064b..=0x0655 | 0x0670 | 0x206d..=0x206f => IRRELEVANT,
        0x065c => form_link(true, false, 0x85),
        0x065d => form_link(true, false, 0x87),
        0x065e => form_link(true, false, 0x89),
        0x065f => form_link(true, false, 0x8b),
        0x0672 | 0x0673 | 0x0675 => LINK_R | ALEF_TYPE,
        0x0676 | 0x0677 => LINK_R,
        0x0678..=0x0687 => LINK_R | LINK_L,
        0x0688..=0x0699 => LINK_R,
        0x069a..=0x06bf => LINK_R | LINK_L,
        0x06c0 => LINK_R,
        0x06c1 => LINK_R | LINK_L,
        0x06c2..=0x06cb => LINK_R,
        0x06cc => LINK_R | LINK_L,
        0x06cd => LINK_R,
        0x06ce..=0x06d1 => LINK_R | LINK_L,
        0x06d2..=0x06d3 => LINK_R,
        0x200d => LINK_R | LINK_L,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaped_units(input: &str) -> Vec<u16> {
        shape_arabic_icu8(&input.encode_utf16().collect::<Vec<_>>())
    }

    fn visual_string(input: &str, default_right_to_left: bool) -> String {
        visual_order(
            &input.chars().map(|ch| (ch, ())).collect::<Vec<_>>(),
            default_right_to_left,
        )
        .into_iter()
        .map(|(ch, ())| ch)
        .collect()
    }

    #[test]
    fn arabic_shaping_matches_icu4j_77_1_fixed_vectors() {
        assert_eq!(
            shaped_units("hello (123)"),
            "hello (123)".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(shaped_units("سلام"), vec![0xfeb3, 0xfefc, 0xfee1]);
        assert_eq!(shaped_units("لا"), vec![0xfefb]);
        assert_eq!(shaped_units("لَا"), vec![0xfefb, 0xfe76]);
        assert_eq!(
            shaped_units("سلام دنیا"),
            vec![0xfeb3, 0xfefc, 0xfee1, 0x20, 0xfea9, 0xfee7, 0x06cc, 0xfe8e]
        );
        assert_eq!(shaped_units("ب\u{200d}ب"), vec![0xfe91, 0x200d, 0xfe90]);
        assert_eq!(shaped_units("ب\u{0656}ب"), vec![0xfe8f, 0x0656, 0xfe8f]);
        assert_eq!(shaped_units("\u{0679}"), vec![0x0679]);
    }

    #[test]
    fn arabic_shaping_matches_icu4j_77_1_bmp_context_digest() {
        let mut digest = 0xcbf29ce484222325u64;
        let mut put = |value: u16| {
            digest ^= u64::from(value);
            digest = digest.wrapping_mul(0x100000001b3);
        };
        for unit in 0..=u16::MAX {
            for context in [
                vec![unit],
                vec![0x0628, unit],
                vec![unit, 0x0628],
                vec![0x0628, unit, 0x0628],
                vec![0x0644, unit],
                vec![unit, 0x0627],
            ] {
                let shaped = shape_arabic_icu8(&context);
                put(shaped.len() as u16);
                for shaped_unit in shaped {
                    put(shaped_unit);
                }
            }
        }
        assert_eq!(digest, 0x3f41d9018ab93e71);
    }

    #[test]
    fn visual_order_matches_icu_mixed_direction_and_mirroring_vectors() {
        assert_eq!(visual_string("hello (123)", false), "hello (123)");
        assert_eq!(visual_string("אבג abc", false), "abc גבא");
        assert_eq!(visual_string("אבג abc", true), "abc גבא");
        assert_eq!(visual_string("سلام", true), "\u{fee1}\u{fefc}\u{feb3}");
        assert_eq!(
            visual_string("مرحبا (abc) 123", true),
            "123 (abc) \u{fe8e}\u{fe92}\u{fea3}\u{feae}\u{fee3}"
        );
        assert_eq!(visual_string("אבג (123)", false), "(123) גבא");
        assert_eq!(visual_string("123", false), "123");
        assert_eq!(visual_string("123", true), "123");
        assert_eq!(visual_string("123-", false), "123-");
        assert_eq!(visual_string("123-", true), "-123");
        assert_eq!(visual_string(" - 123", true), "123 - ");
        assert_eq!(
            visual_string("\u{2067}abc\u{2069} - 123", true),
            "123 - \u{2069}abc\u{2067}"
        );
        assert_eq!(
            visual_string("\u{2066}abc\u{2069} אב", false),
            "בא \u{2069}abc\u{2066}"
        );
        assert_eq!(visual_string("א[ב]", false), "[ב]א");
        assert_eq!(visual_string("א\u{0301}ב", false), "ב\u{0301}א");
    }

    #[test]
    fn visual_order_carries_utf16_metadata_like_vanilla_substring_source() {
        let logical = [('א', "rtl"), ('ב', "rtl"), ('c', "ltr"), ('d', "ltr")];
        assert_eq!(
            visual_order(&logical, false),
            vec![('c', "ltr"), ('d', "ltr"), ('ב', "rtl"), ('א', "rtl")]
        );

        // Arabic shaping compacts Lam-Alef but vanilla keeps the original UTF-16 style vector.
        // The following `x` therefore inherits the metadata at original index 1.
        let compacted = [('ل', "lam"), ('ا', "alef"), ('x', "tail")];
        assert_eq!(
            visual_order(&compacted, true),
            vec![('x', "alef"), ('\u{fefb}', "lam")]
        );
    }

    #[test]
    fn visual_order_keeps_supplementary_codepoints_intact() {
        let logical = [('א', 1), ('😀', 2), ('ב', 3)];
        let visual = visual_order(&logical, false);
        assert_eq!(visual, vec![('ב', 3), ('😀', 2), ('א', 1)]);
    }
}
