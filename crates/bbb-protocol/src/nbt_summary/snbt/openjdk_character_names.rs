/// Resolves the exact character-name surface used by OpenJDK 26
/// `Character.codePointOf`. The Unicode table is only a candidate index:
/// `unicode_names2::character` deliberately implements loose UAX44 matching,
/// while Java accepts only a trimmed, case-insensitive exact name. This returns
/// a codepoint rather than a Rust `char` because Java's generated block names
/// include surrogate codepoints.
pub(super) fn codepoint_by_name(raw_name: &str) -> Option<u32> {
    let name = raw_name.trim().to_ascii_uppercase();
    if let Some(codepoint) = constructed_codepoint(&name) {
        return Some(codepoint);
    }
    if let Some(codepoint) = openjdk_control_codepoint(&name) {
        return Some(codepoint);
    }
    let character = unicode_names2::character(&name)?;
    let codepoint = character as u32;
    if constructed_block_name(codepoint).is_some() {
        return None;
    }
    if character.is_control() {
        return (openjdk_control_name(codepoint) == Some(name.as_str())).then_some(codepoint);
    }
    let canonical_name = unicode_names2::name(character)?.to_string();
    (canonical_name == name).then_some(codepoint)
}

fn constructed_codepoint(name: &str) -> Option<u32> {
    let (_, raw_codepoint) = name.rsplit_once(' ')?;
    let codepoint = u32::from_str_radix(raw_codepoint, 16).ok()?;
    let expected_prefix = constructed_block_name(codepoint)?;
    (name == format!("{expected_prefix} {codepoint:X}")).then_some(codepoint)
}

fn openjdk_control_codepoint(name: &str) -> Option<u32> {
    (0..=0x009f).find(|&codepoint| openjdk_control_name(codepoint) == Some(name))
}

fn constructed_block_name(codepoint: u32) -> Option<&'static str> {
    Some(match codepoint {
        0x3400..=0x4dbf => "CJK UNIFIED IDEOGRAPHS EXTENSION A",
        0x4e00..=0x9fff => "CJK UNIFIED IDEOGRAPHS",
        0xac00..=0xd7a3 => "HANGUL SYLLABLES",
        0xd800..=0xdb7f => "HIGH SURROGATES",
        0xdb80..=0xdbff => "HIGH PRIVATE USE SURROGATES",
        0xdc00..=0xdfff => "LOW SURROGATES",
        0xe000..=0xf8ff => "PRIVATE USE AREA",
        0x17000..=0x187ff => "TANGUT",
        0x18d00..=0x18d1e => "TANGUT SUPPLEMENT",
        0x20000..=0x2a6df => "CJK UNIFIED IDEOGRAPHS EXTENSION B",
        0x2a700..=0x2b73f => "CJK UNIFIED IDEOGRAPHS EXTENSION C",
        0x2b740..=0x2b81d => "CJK UNIFIED IDEOGRAPHS EXTENSION D",
        0x2b820..=0x2cead => "CJK UNIFIED IDEOGRAPHS EXTENSION E",
        0x2ceb0..=0x2ebe0 => "CJK UNIFIED IDEOGRAPHS EXTENSION F",
        0x2ebf0..=0x2ee5d => "CJK UNIFIED IDEOGRAPHS EXTENSION I",
        0x30000..=0x3134a => "CJK UNIFIED IDEOGRAPHS EXTENSION G",
        0x31350..=0x323af => "CJK UNIFIED IDEOGRAPHS EXTENSION H",
        0x323b0..=0x33479 => "CJK UNIFIED IDEOGRAPHS EXTENSION J",
        0xf0000..=0xffffd => "SUPPLEMENTARY PRIVATE USE AREA A",
        0x100000..=0x10fffd => "SUPPLEMENTARY PRIVATE USE AREA B",
        _ => return None,
    })
}

fn openjdk_control_name(codepoint: u32) -> Option<&'static str> {
    Some(match codepoint {
        0x0000 => "NULL",
        0x0001 => "START OF HEADING",
        0x0002 => "START OF TEXT",
        0x0003 => "END OF TEXT",
        0x0004 => "END OF TRANSMISSION",
        0x0005 => "ENQUIRY",
        0x0006 => "ACKNOWLEDGE",
        0x0007 => "BEL",
        0x0008 => "BACKSPACE",
        0x0009 => "CHARACTER TABULATION",
        0x000a => "LINE FEED (LF)",
        0x000b => "LINE TABULATION",
        0x000c => "FORM FEED (FF)",
        0x000d => "CARRIAGE RETURN (CR)",
        0x000e => "SHIFT OUT",
        0x000f => "SHIFT IN",
        0x0010 => "DATA LINK ESCAPE",
        0x0011 => "DEVICE CONTROL ONE",
        0x0012 => "DEVICE CONTROL TWO",
        0x0013 => "DEVICE CONTROL THREE",
        0x0014 => "DEVICE CONTROL FOUR",
        0x0015 => "NEGATIVE ACKNOWLEDGE",
        0x0016 => "SYNCHRONOUS IDLE",
        0x0017 => "END OF TRANSMISSION BLOCK",
        0x0018 => "CANCEL",
        0x0019 => "END OF MEDIUM",
        0x001a => "SUBSTITUTE",
        0x001b => "ESCAPE",
        0x001c => "INFORMATION SEPARATOR FOUR",
        0x001d => "INFORMATION SEPARATOR THREE",
        0x001e => "INFORMATION SEPARATOR TWO",
        0x001f => "INFORMATION SEPARATOR ONE",
        0x007f => "DELETE",
        0x0080 => "PADDING CHARACTER",
        0x0081 => "HIGH OCTET PRESET",
        0x0082 => "BREAK PERMITTED HERE",
        0x0083 => "NO BREAK HERE",
        0x0085 => "NEXT LINE (NEL)",
        0x0086 => "START OF SELECTED AREA",
        0x0087 => "END OF SELECTED AREA",
        0x0088 => "CHARACTER TABULATION SET",
        0x0089 => "CHARACTER TABULATION WITH JUSTIFICATION",
        0x008a => "LINE TABULATION SET",
        0x008b => "PARTIAL LINE FORWARD",
        0x008c => "PARTIAL LINE BACKWARD",
        0x008d => "REVERSE LINE FEED",
        0x008e => "SINGLE SHIFT TWO",
        0x008f => "SINGLE SHIFT THREE",
        0x0090 => "DEVICE CONTROL STRING",
        0x0091 => "PRIVATE USE ONE",
        0x0092 => "PRIVATE USE TWO",
        0x0093 => "SET TRANSMIT STATE",
        0x0094 => "CANCEL CHARACTER",
        0x0095 => "MESSAGE WAITING",
        0x0096 => "START OF GUARDED AREA",
        0x0097 => "END OF GUARDED AREA",
        0x0098 => "START OF STRING",
        0x0099 => "SINGLE GRAPHIC CHARACTER INTRODUCER",
        0x009a => "SINGLE CHARACTER INTRODUCER",
        0x009b => "CONTROL SEQUENCE INTRODUCER",
        0x009c => "STRING TERMINATOR",
        0x009d => "OPERATING SYSTEM COMMAND",
        0x009e => "PRIVACY MESSAGE",
        0x009f => "APPLICATION PROGRAM COMMAND",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_unicode_names_are_case_insensitive_but_not_loosely_matched() {
        assert_eq!(
            codepoint_by_name("LATIN CAPITAL LETTER A"),
            Some('A' as u32)
        );
        assert_eq!(
            codepoint_by_name(" latin capital letter a "),
            Some('A' as u32)
        );
        assert_eq!(codepoint_by_name("LATINCAPITALLETTERA"), None);
        assert_eq!(codepoint_by_name("LATIN-CAPITAL LETTER A"), None);
    }

    #[test]
    fn openjdk_control_names_exclude_other_unicode_aliases() {
        assert_eq!(codepoint_by_name("BEL"), Some(0x0007));
        assert_eq!(codepoint_by_name("BACKSPACE"), Some(0x0008));
        assert_eq!(codepoint_by_name("PADDING CHARACTER"), Some(0x0080));
        assert_eq!(codepoint_by_name("BELL"), Some('🔔' as u32));
        assert_eq!(codepoint_by_name("BS"), None);
        assert_eq!(codepoint_by_name("INDEX"), None);
    }

    #[test]
    fn openjdk_constructed_names_use_block_and_uppercase_hex() {
        assert_eq!(
            codepoint_by_name("HANGUL SYLLABLES AC00"),
            Some('가' as u32)
        );
        assert_eq!(
            codepoint_by_name("CJK UNIFIED IDEOGRAPHS 4E00"),
            Some('一' as u32)
        );
        assert_eq!(codepoint_by_name("TANGUT 17000"), Some(0x17000));
        assert_eq!(codepoint_by_name("PRIVATE USE AREA E000"), Some(0xe000));
        assert_eq!(codepoint_by_name("HIGH SURROGATES D800"), Some(0xd800));
        assert_eq!(codepoint_by_name("LOW SURROGATES DFFF"), Some(0xdfff));
        assert_eq!(codepoint_by_name("HANGUL SYLLABLE GA"), None);
        assert_eq!(codepoint_by_name("CJK UNIFIED IDEOGRAPH-4E00"), None);
        assert_eq!(codepoint_by_name("PRIVATE USE AREA 0E000"), None);
    }
}
