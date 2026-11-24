// Netty 4.2.7.Final ByteBufUtil consumes the unit after a high surrogate even
// when it is not a low surrogate.
pub(super) fn encode_units(value: &[u16], mut emit: impl FnMut(&[u8])) {
    let mut units = value.iter().copied();
    while let Some(unit) = units.next() {
        let scalar = if (0xd800..=0xdbff).contains(&unit) {
            let Some(next) = units.next() else {
                emit(b"?");
                break;
            };
            if !(0xdc00..=0xdfff).contains(&next) {
                emit(&[
                    b'?',
                    if (0xd800..=0xdbff).contains(&next) {
                        b'?'
                    } else {
                        next as u8
                    },
                ]);
                continue;
            }
            0x10000 + (((unit as u32 - 0xd800) << 10) | (next as u32 - 0xdc00))
        } else if (0xdc00..=0xdfff).contains(&unit) {
            emit(b"?");
            continue;
        } else {
            unit as u32
        };
        let mut encoded = [0; 4];
        emit(
            char::from_u32(scalar)
                .unwrap()
                .encode_utf8(&mut encoded)
                .as_bytes(),
        );
    }
}

pub(super) fn decode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len());
    scan(bytes, |value| output.push(value));
    output
}

pub(super) fn decode_units(bytes: &[u8]) -> Vec<u16> {
    let mut output = Vec::with_capacity(bytes.len());
    scan(bytes, |value| {
        let mut units = [0; 2];
        output.extend_from_slice(value.encode_utf16(&mut units));
    });
    output
}

fn scan(bytes: &[u8], mut emit: impl FnMut(char)) {
    let mut offset = 0;
    while let Some(&first) = bytes.get(offset) {
        if first < 0x80 {
            emit(char::from(first));
            offset += 1;
            continue;
        }
        let (width, mut value) = match first {
            0xc2..=0xdf => (2, u32::from(first & 0x1f)),
            0xe0..=0xef => (3, u32::from(first & 0x0f)),
            0xf0..=0xf4 => (4, u32::from(first & 0x07)),
            _ => {
                emit(char::REPLACEMENT_CHARACTER);
                offset += 1;
                continue;
            }
        };
        let mut consumed = 1;
        while consumed < width {
            let Some(&next) = bytes.get(offset + consumed) else {
                break;
            };
            if !(0x80..=0xbf).contains(&next)
                || (consumed == 1
                    && ((first == 0xe0 && next < 0xa0)
                        || (first == 0xf0 && next < 0x90)
                        || (first == 0xf4 && next > 0x8f)))
            {
                break;
            }
            value = (value << 6) | u32::from(next & 0x3f);
            consumed += 1;
        }
        // JDK String.decodeUTF8_UTF16 consumes the whole surrogate sequence as one error.
        emit(if consumed == width {
            char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER)
        } else {
            char::REPLACEMENT_CHARACTER
        });
        offset += consumed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_utf8_matches_netty_heap_and_direct_oracle() {
        let mut hash = 0xcbf29ce484222325u64;
        let mut check = |bytes: &[u8]| {
            let units = decode_units(bytes);
            assert_eq!(units, decode(bytes).encode_utf16().collect::<Vec<_>>());
            for byte in (units.len() as u32)
                .to_le_bytes()
                .into_iter()
                .chain(units.into_iter().flat_map(u16::to_le_bytes))
            {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        };
        for first in 0..=255 {
            check(&[first]);
            for second in 0..=255 {
                check(&[first, second]);
            }
        }
        let alphabet = [
            0, 0x41, 0x7f, 0x80, 0x8f, 0x90, 0x9f, 0xa0, 0xbf, 0xc0, 0xc1, 0xc2, 0xdf, 0xe0, 0xed,
            0xef, 0xf0, 0xf4, 0xf5, 0xff,
        ];
        for len in 0..=4 {
            for mut index in 0..alphabet.len().pow(len as u32) {
                let mut bytes = vec![0; len];
                for byte in &mut bytes {
                    *byte = alphabet[index % alphabet.len()];
                    index /= alphabet.len();
                }
                check(&bytes);
            }
        }
        assert_eq!(hash, 0x605336a5aa11fe53);
    }
    #[test]
    fn java_utf8_bounded_reader_checks_bytes_then_java_units() {
        use crate::codec::{Decoder, Encoder, ProtocolError};
        for (bytes, limit, expected) in [
            (vec![0xed, 0xa0, 0x80], 1, "�"),
            (vec![0xed, 0xa0], 1, "�"),
            (vec![0xe0, 0x80, 0x80], 3, "���"),
            (vec![0xf0, 0x90, 0x80], 1, "�"),
            (vec![0xf0, 0x90, 0x80, 0x80], 2, "𐀀"),
        ] {
            let mut wire = Encoder::new();
            wire.write_var_i32(bytes.len() as i32);
            wire.write_bytes(&bytes);
            let wire = wire.into_inner();
            assert_eq!(Decoder::new(&wire).read_string(limit).unwrap(), expected);
            assert_eq!(
                Decoder::new(&wire).read_java_string(limit).unwrap(),
                expected
            );
        }
        let mut wire = Encoder::new();
        wire.write_var_i32(4);
        let wire = wire.into_inner();
        assert!(matches!(
            Decoder::new(&wire).read_string(1),
            Err(ProtocolError::StringTooLong { actual: 4, max: 3 })
        ));
        assert!(matches!(
            Decoder::new(&wire).read_java_string(1),
            Err(ProtocolError::StringTooLong { actual: 4, max: 3 })
        ));
        let mut wire = Encoder::new();
        wire.write_string("😀");
        wire.write_u8(99);
        let wire = wire.into_inner();
        let mut reader = Decoder::new(&wire);
        assert!(matches!(
            reader.read_string(1),
            Err(ProtocolError::StringTooLong { actual: 4, max: 3 })
        ));
        let mut wire = Encoder::new();
        wire.write_string("😀😀");
        wire.write_u8(99);
        let wire = wire.into_inner();
        let mut reader = Decoder::new(&wire);
        assert!(matches!(
            reader.read_string(3),
            Err(ProtocolError::StringTooLong { actual: 4, max: 3 })
        ));
        assert_eq!(reader.read_u8().unwrap(), 99);
        let mut reader = Decoder::new(&wire);
        assert!(matches!(
            reader.read_java_string(3),
            Err(ProtocolError::StringTooLong { actual: 4, max: 3 })
        ));
        assert_eq!(reader.read_u8().unwrap(), 99);
    }
}
