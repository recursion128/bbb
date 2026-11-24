use lz4_flex::block::{compress_into, decompress_into, get_maximum_output_size};
use twox_hash::xxhash32::Hasher;

const MAGIC: &[u8; 8] = b"LZ4Block";
const HEADER_LEN: usize = 21;
const METHOD_RAW: u8 = 0x10;
const METHOD_COMPRESSED: u8 = 0x20;
const METHOD_MASK: u8 = 0xf0;
const LEVEL_BASE: u32 = 10;
const WRITER_LEVEL_NIBBLE: u8 = 6;
const WRITER_TOKEN_RAW: u8 = METHOD_RAW | WRITER_LEVEL_NIBBLE;
const WRITER_TOKEN_COMPRESSED: u8 = METHOD_COMPRESSED | WRITER_LEVEL_NIBBLE;
const BLOCK_SIZE: usize = 1 << (LEVEL_BASE + WRITER_LEVEL_NIBBLE as u32);
const CHECKSUM_SEED: u32 = 0x9747_b28c;
const JAVA_CHECKSUM_MASK: u32 = 0x0fff_ffff;

pub fn compress_java_block(input: &[u8]) -> Result<Vec<u8>, String> {
    let block_count = input.len().div_ceil(BLOCK_SIZE);
    let header_bytes = block_count
        .checked_add(1)
        .and_then(|count| count.checked_mul(HEADER_LEN))
        .ok_or_else(|| "LZ4 block stream size overflow".to_owned())?;
    let capacity = input
        .len()
        .checked_add(header_bytes)
        .ok_or_else(|| "LZ4 block stream size overflow".to_owned())?;
    let mut output = Vec::with_capacity(capacity);
    let mut scratch = if input.is_empty() {
        Vec::new()
    } else {
        vec![0_u8; get_maximum_output_size(BLOCK_SIZE.min(input.len()))]
    };

    for block in input.chunks(BLOCK_SIZE) {
        let compressed_len = compress_into(block, &mut scratch)
            .map_err(|err| format!("LZ4 block compression failed: {err}"))?;
        let (token, payload) = if compressed_len < block.len() {
            (WRITER_TOKEN_COMPRESSED, &scratch[..compressed_len])
        } else {
            (WRITER_TOKEN_RAW, block)
        };
        write_header(
            &mut output,
            token,
            payload.len(),
            block.len(),
            java_checksum(block),
        )?;
        output.extend_from_slice(payload);
    }

    write_header(&mut output, WRITER_TOKEN_RAW, 0, 0, 0)?;
    Ok(output)
}

pub fn decompress_java_block(input: &[u8], max_output: usize) -> Result<Vec<u8>, String> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    let mut block_index = 0usize;

    loop {
        let header_end = cursor.checked_add(HEADER_LEN).ok_or_else(|| {
            format!("LZ4 block {block_index} header offset overflow at byte {cursor}")
        })?;
        let header = input.get(cursor..header_end).ok_or_else(|| {
            format!(
                "truncated LZ4 block {block_index} header at byte {cursor}: need {HEADER_LEN} bytes"
            )
        })?;
        cursor = header_end;

        if &header[..MAGIC.len()] != MAGIC {
            return Err(format!("invalid LZ4 block {block_index} magic"));
        }

        let token = header[8];
        let method = token & METHOD_MASK;
        if !matches!(method, METHOD_RAW | METHOD_COMPRESSED) {
            return Err(format!(
                "invalid LZ4 block {block_index} compression method 0x{method:02x}"
            ));
        }
        let level = LEVEL_BASE + u32::from(token & 0x0f);
        let block_capacity = 1usize
            .checked_shl(level)
            .ok_or_else(|| format!("LZ4 block {block_index} level {level} is unsupported"))?;

        let compressed_len = read_java_len(&header[9..13], block_index, "compressed")?;
        let original_len = read_java_len(&header[13..17], block_index, "original")?;
        let expected_checksum = u32::from_le_bytes(
            header[17..21]
                .try_into()
                .expect("fixed-width checksum header slice"),
        );

        if compressed_len == 0 || original_len == 0 {
            if compressed_len != 0 || original_len != 0 || expected_checksum != 0 {
                return Err(format!(
                    "invalid LZ4 block {block_index} empty block fields"
                ));
            }
            if cursor != input.len() {
                return Err(format!(
                    "trailing bytes after LZ4 terminator: {} bytes",
                    input.len() - cursor
                ));
            }
            return Ok(output);
        }

        if original_len > block_capacity {
            return Err(format!(
                "LZ4 block {block_index} original length {original_len} exceeds level capacity {block_capacity}"
            ));
        }
        match method {
            METHOD_RAW if compressed_len != original_len => {
                return Err(format!(
                    "LZ4 raw block {block_index} length mismatch: compressed {compressed_len}, original {original_len}"
                ));
            }
            _ => {}
        }

        let output_end = output
            .len()
            .checked_add(original_len)
            .ok_or_else(|| format!("LZ4 block {block_index} cumulative output length overflow"))?;
        if output_end > max_output {
            return Err(format!(
                "LZ4 block {block_index} exceeds output budget {max_output}: would produce {output_end} bytes"
            ));
        }

        let payload_end = cursor.checked_add(compressed_len).ok_or_else(|| {
            format!("LZ4 block {block_index} payload offset overflow at byte {cursor}")
        })?;
        let payload = input.get(cursor..payload_end).ok_or_else(|| {
            format!(
                "truncated LZ4 block {block_index} payload at byte {cursor}: need {compressed_len} bytes"
            )
        })?;
        cursor = payload_end;

        let block_start = output.len();
        if method == METHOD_RAW {
            output.extend_from_slice(payload);
        } else {
            output.resize(output_end, 0);
            let decoded_len = decompress_into(payload, &mut output[block_start..])
                .map_err(|error| format!("invalid compressed LZ4 block {block_index}: {error}"))?;
            if decoded_len != original_len {
                return Err(format!(
                    "LZ4 block {block_index} decoded length mismatch: expected {original_len}, got {decoded_len}"
                ));
            }
        }

        let actual_checksum = java_checksum(&output[block_start..output_end]);
        if actual_checksum != expected_checksum {
            return Err(format!(
                "LZ4 block {block_index} checksum mismatch: expected {expected_checksum:#010x}, got {actual_checksum:#010x}"
            ));
        }
        block_index += 1;
    }
}

fn write_header(
    output: &mut Vec<u8>,
    token: u8,
    compressed_len: usize,
    original_len: usize,
    checksum: u32,
) -> Result<(), String> {
    let compressed_len = u32::try_from(compressed_len)
        .map_err(|_| "LZ4 compressed block length exceeds Java u32 field".to_owned())?;
    let original_len = u32::try_from(original_len)
        .map_err(|_| "LZ4 original block length exceeds Java u32 field".to_owned())?;
    output.extend_from_slice(MAGIC);
    output.push(token);
    output.extend_from_slice(&compressed_len.to_le_bytes());
    output.extend_from_slice(&original_len.to_le_bytes());
    output.extend_from_slice(&checksum.to_le_bytes());
    Ok(())
}

fn read_java_len(bytes: &[u8], block_index: usize, field_name: &str) -> Result<usize, String> {
    let value = i32::from_le_bytes(
        bytes
            .try_into()
            .expect("fixed-width LZ4 block length header slice"),
    );
    usize::try_from(value)
        .map_err(|_| format!("LZ4 block {block_index} has negative {field_name} length {value}"))
}

fn java_checksum(bytes: &[u8]) -> u32 {
    // lz4-java 1.10.1 exposes StreamingXXHash32 through java.util.zip.Checksum,
    // whose adapter intentionally retains only the low 28 bits.
    Hasher::oneshot(CHECKSUM_SEED, bytes) & JAVA_CHECKSUM_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(token: u8, compressed_len: u32, original_len: u32, checksum: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(token);
        bytes.extend_from_slice(&compressed_len.to_le_bytes());
        bytes.extend_from_slice(&original_len.to_le_bytes());
        bytes.extend_from_slice(&checksum.to_le_bytes());
        bytes
    }

    #[test]
    fn empty_stream_is_complete_java_terminator() {
        let encoded = compress_java_block(&[]).expect("encode empty stream");

        assert_eq!(encoded, header(0x16, 0, 0, 0));
        assert_eq!(
            decompress_java_block(&encoded, 0).expect("decode empty stream"),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn reader_accepts_every_java_level_nibble() {
        for nibble in 0..=0x0f {
            let encoded = header(METHOD_RAW | nibble, 0, 0, 0);
            assert_eq!(
                decompress_java_block(&encoded, 0).expect("decode valid level nibble"),
                Vec::<u8>::new(),
                "level nibble {nibble}"
            );
        }
    }

    #[test]
    fn short_incompressible_block_is_stored_raw() {
        let input = [0xa5];
        let encoded = compress_java_block(&input).expect("encode raw block");

        assert_eq!(encoded[8], 0x16);
        assert_eq!(u32::from_le_bytes(encoded[9..13].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(encoded[13..17].try_into().unwrap()), 1);
        assert_eq!(encoded[HEADER_LEN], input[0]);
        assert_eq!(
            decompress_java_block(&encoded, input.len()).expect("decode raw block"),
            input
        );
    }

    #[test]
    fn repetitive_block_is_compressed() {
        let input = vec![b'z'; 4096];
        let encoded = compress_java_block(&input).expect("encode compressed block");

        assert_eq!(encoded[8], 0x26);
        assert!(
            u32::from_le_bytes(encoded[9..13].try_into().unwrap())
                < u32::from_le_bytes(encoded[13..17].try_into().unwrap())
        );
        assert_eq!(
            decompress_java_block(&encoded, input.len()).expect("decode compressed block"),
            input
        );
    }

    #[test]
    fn java_1_10_1_writer_fixture_decodes() {
        let input = decode_hex(
            "6262622d6a6176612d32362e312d6c7a342d626c6f636b2d666978747572652d6262622d6a6176612d32362e312d6c7a342d626c6f636b2d66697874757265",
        );
        let encoded = decode_hex(
            "4c5a34426c6f636b262b0000003f000000df28b50aff116262622d6a6176612d32362e312d6c7a342d626c6f636b2d666978747572652d2000075078747572654c5a34426c6f636b16000000000000000000000000",
        );

        assert_eq!(decompress_java_block(&encoded, input.len()).unwrap(), input);
    }

    #[test]
    fn multiblock_stream_round_trips() {
        let input: Vec<u8> = (0..(BLOCK_SIZE * 2 + 173))
            .map(|index| ((index * 31 + index / 251) & 0xff) as u8)
            .collect();
        let encoded = compress_java_block(&input).expect("encode multiblock stream");

        let mut cursor = 0;
        let mut blocks = 0;
        loop {
            let compressed_len =
                u32::from_le_bytes(encoded[cursor + 9..cursor + 13].try_into().unwrap()) as usize;
            let original_len =
                u32::from_le_bytes(encoded[cursor + 13..cursor + 17].try_into().unwrap());
            cursor += HEADER_LEN;
            if original_len == 0 {
                break;
            }
            blocks += 1;
            cursor += compressed_len;
        }

        assert_eq!(blocks, 3);
        assert_eq!(cursor, encoded.len());
        assert_eq!(
            decompress_java_block(&encoded, input.len()).expect("decode multiblock stream"),
            input
        );
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|digits| {
                let text = std::str::from_utf8(digits).unwrap();
                u8::from_str_radix(text, 16).unwrap()
            })
            .collect()
    }
}
