//! One section's 4×4×4 biome paletted container.

use bbb_protocol::codec::{Decoder, Encoder};

use crate::chunk_ticks::ChunkError;

const SIDE: usize = 4;
const VOLUME: usize = SIDE * SIDE * SIDE;
const MAX_LOCAL_BITS: u8 = 3;

fn index(x: usize, y: usize, z: usize) -> usize {
    // Strategy.getIndex, Strategy.java:69-70.
    (y << 2 | z) << 2 | x
}

fn ceil_log2(value: usize) -> u8 {
    if value <= 1 {
        0
    } else {
        (usize::BITS - (value - 1).leading_zeros()) as u8
    }
}

fn values_per_word(bits: u8) -> usize {
    64 / usize::from(bits)
}

fn word_count(bits: u8) -> usize {
    VOLUME.div_ceil(values_per_word(bits))
}

fn read_cell(data: &[u64], bits: u8, entry: usize) -> usize {
    let per_word = values_per_word(bits);
    let shift = (entry % per_word) * usize::from(bits);
    ((data[entry / per_word] >> shift) & ((1_u64 << bits) - 1)) as usize
}

fn write_cell(data: &mut [u64], bits: u8, entry: usize, value: usize) {
    let per_word = values_per_word(bits);
    let word = entry / per_word;
    let shift = (entry % per_word) * usize::from(bits);
    let mask = (1_u64 << bits) - 1;
    data[word] = data[word] & !(mask << shift) | (value as u64 & mask) << shift;
}

fn repack(data: &[u64], bits: u8, new_bits: u8) -> Vec<u64> {
    let mut packed = vec![0; word_count(new_bits)];
    for entry in 0..VOLUME {
        write_cell(&mut packed, new_bits, entry, read_cell(data, bits, entry));
    }
    packed
}

fn read_u8(decoder: &mut Decoder<'_>, field: &str) -> Result<u8, ChunkError> {
    decoder
        .read_u8()
        .map_err(|error| ChunkError(format!("failed to decode {field}: {error}")))
}

fn read_var_i32(decoder: &mut Decoder<'_>, field: &str) -> Result<i32, ChunkError> {
    decoder
        .read_var_i32()
        .map_err(|error| ChunkError(format!("failed to decode {field}: {error}")))
}

fn read_i64(decoder: &mut Decoder<'_>, field: &str) -> Result<i64, ChunkError> {
    decoder
        .read_i64()
        .map_err(|error| ChunkError(format!("failed to decode {field}: {error}")))
}

/// Vanilla's biome `PalettedContainer`: 64 cells indexed `(y, z, x)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionBiomes {
    Single(i32),
    Indirect {
        bits: u8,
        palette: Vec<i32>,
        data: Vec<u64>,
    },
}

impl SectionBiomes {
    pub fn single(biome: i32, registry_len: usize) -> Result<Self, ChunkError> {
        validate_biome_id(biome, registry_len)?;
        Ok(Self::Single(biome))
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> i32 {
        let entry = index(x.min(3), y.min(3), z.min(3));
        match self {
            Self::Single(biome) => *biome,
            Self::Indirect {
                bits,
                palette,
                data,
            } => palette[read_cell(data, *bits, entry)],
        }
    }

    pub fn set(
        &mut self,
        x: usize,
        y: usize,
        z: usize,
        biome: i32,
        registry_len: usize,
    ) -> Result<i32, ChunkError> {
        validate_biome_id(biome, registry_len)?;
        let entry = index(x.min(3), y.min(3), z.min(3));
        match self {
            Self::Single(current) => {
                let previous = *current;
                if biome != previous {
                    let mut data = vec![0; word_count(1)];
                    write_cell(&mut data, 1, entry, 1);
                    *self = Self::Indirect {
                        bits: 1,
                        palette: vec![previous, biome],
                        data,
                    };
                }
                Ok(previous)
            }
            Self::Indirect {
                bits,
                palette,
                data,
            } => {
                let previous = palette[read_cell(data, *bits, entry)];
                if previous == biome {
                    return Ok(previous);
                }
                let palette_index = match palette.iter().position(|candidate| *candidate == biome) {
                    Some(index) => index,
                    None => {
                        palette.push(biome);
                        let needed = ceil_log2(palette.len());
                        if needed > *bits {
                            *data = repack(data, *bits, needed);
                            *bits = needed;
                        }
                        palette.len() - 1
                    }
                };
                write_cell(data, *bits, entry, palette_index);
                Ok(previous)
            }
        }
    }

    pub fn palette(&self) -> &[i32] {
        match self {
            Self::Single(biome) => std::slice::from_ref(biome),
            Self::Indirect { palette, .. } => palette,
        }
    }

    /// Visits each distinct biome id actually present in at least one cell of this section.
    pub fn visit_present_biomes(&self, mut visit: impl FnMut(i32)) {
        match self {
            Self::Single(biome) => visit(*biome),
            Self::Indirect {
                bits,
                palette,
                data,
            } => {
                if palette.len() <= 64 {
                    let mut seen = 0_u64;
                    let mut remaining = palette.len();
                    for entry in 0..VOLUME {
                        let idx = read_cell(data, *bits, entry);
                        let bit = 1_u64 << idx;
                        if seen & bit == 0 {
                            seen |= bit;
                            visit(palette[idx]);
                            remaining -= 1;
                            if remaining == 0 {
                                break;
                            }
                        }
                    }
                } else {
                    let mut seen = vec![false; palette.len()];
                    for entry in 0..VOLUME {
                        let idx = read_cell(data, *bits, entry);
                        if !seen[idx] {
                            seen[idx] = true;
                            visit(palette[idx]);
                        }
                    }
                }
            }
        }
    }

    pub fn encode(&self, out: &mut Encoder, registry_len: usize) {
        match self {
            Self::Single(biome) => {
                debug_assert!(validate_biome_id(*biome, registry_len).is_ok());
                out.write_u8(0);
                out.write_var_i32(*biome);
            }
            Self::Indirect {
                bits,
                palette,
                data,
            } if palette.len() <= 1 << MAX_LOCAL_BITS => {
                out.write_u8(*bits);
                out.write_var_i32(palette.len() as i32);
                for biome in palette {
                    debug_assert!(validate_biome_id(*biome, registry_len).is_ok());
                    out.write_var_i32(*biome);
                }
                for word in data {
                    out.write_i64(*word as i64);
                }
            }
            Self::Indirect {
                bits,
                palette,
                data,
            } => {
                let global_bits = ceil_log2(registry_len);
                debug_assert!(global_bits >= 4);
                out.write_u8(global_bits);
                let mut global = vec![0; word_count(global_bits)];
                for entry in 0..VOLUME {
                    let biome = palette[read_cell(data, *bits, entry)];
                    debug_assert!(validate_biome_id(biome, registry_len).is_ok());
                    write_cell(&mut global, global_bits, entry, biome as usize);
                }
                for word in global {
                    out.write_i64(word as i64);
                }
            }
        }
    }

    pub fn decode(decoder: &mut Decoder<'_>, registry_len: usize) -> Result<Self, ChunkError> {
        if registry_len == 0 {
            return Err(ChunkError("biome registry must not be empty".to_string()));
        }
        let wire_bits = read_u8(decoder, "biome bits")?;
        if wire_bits == 0 {
            let biome = read_var_i32(decoder, "single-value biome palette")?;
            return Self::single(biome, registry_len);
        }
        if wire_bits > MAX_LOCAL_BITS {
            let global_bits = ceil_log2(registry_len);
            if global_bits == 0 {
                return Err(ChunkError(
                    "global biome palette is impossible for a one-entry registry".to_string(),
                ));
            }
            let mut global = Vec::with_capacity(word_count(global_bits));
            for _ in 0..word_count(global_bits) {
                global.push(read_i64(decoder, "biome data word")? as u64);
            }
            let mut palette = Vec::new();
            let mut indices = [0_usize; VOLUME];
            for (entry, stored) in indices.iter_mut().enumerate() {
                let biome = read_cell(&global, global_bits, entry) as i32;
                validate_biome_id(biome, registry_len)?;
                *stored = match palette.iter().position(|candidate| *candidate == biome) {
                    Some(index) => index,
                    None => {
                        palette.push(biome);
                        palette.len() - 1
                    }
                };
            }
            if palette.len() == 1 {
                return Ok(Self::Single(palette[0]));
            }
            let bits = ceil_log2(palette.len());
            let mut data = vec![0; word_count(bits)];
            for (entry, palette_index) in indices.into_iter().enumerate() {
                write_cell(&mut data, bits, entry, palette_index);
            }
            return Ok(Self::Indirect {
                bits,
                palette,
                data,
            });
        }

        let bits = wire_bits;
        let palette_len = read_var_i32(decoder, "biome palette length")?;
        let max_palette_len = 1_i32 << bits;
        if !(1..=max_palette_len).contains(&palette_len) {
            return Err(ChunkError(format!(
                "biome palette length {palette_len} outside 1..={max_palette_len} for {bits} bits"
            )));
        }
        let mut palette = Vec::with_capacity(palette_len as usize);
        for _ in 0..palette_len {
            let biome = read_var_i32(decoder, "biome palette entry")?;
            validate_biome_id(biome, registry_len)?;
            if palette.contains(&biome) {
                return Err(ChunkError(format!("duplicate biome palette entry {biome}")));
            }
            palette.push(biome);
        }
        let mut data = Vec::with_capacity(word_count(bits));
        for _ in 0..word_count(bits) {
            data.push(read_i64(decoder, "biome data word")? as u64);
        }
        for entry in 0..VOLUME {
            let palette_index = read_cell(&data, bits, entry);
            if palette_index >= palette.len() {
                return Err(ChunkError(format!(
                    "biome palette index {palette_index} out of range for palette of {}",
                    palette.len()
                )));
            }
        }
        Ok(Self::Indirect {
            bits,
            palette,
            data,
        })
    }
}

fn validate_biome_id(biome: i32, registry_len: usize) -> Result<(), ChunkError> {
    if biome < 0 || biome as usize >= registry_len {
        Err(ChunkError(format!(
            "global biome id {biome} outside registry of {registry_len} entries"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(source: &SectionBiomes, registry_len: usize) -> (SectionBiomes, Vec<u8>) {
        let mut encoder = Encoder::new();
        source.encode(&mut encoder, registry_len);
        let bytes = encoder.into_inner();
        let mut decoder = Decoder::new(&bytes);
        let decoded = SectionBiomes::decode(&mut decoder, registry_len).unwrap();
        assert!(decoder.is_empty());
        (decoded, bytes)
    }

    #[test]
    fn local_palette_grows_through_all_three_wire_widths() {
        let mut biomes = SectionBiomes::single(0, 65).unwrap();
        for biome in 1..8 {
            let entry = biome as usize;
            biomes
                .set(entry & 3, entry >> 4, (entry >> 2) & 3, biome, 65)
                .unwrap();
            let expected_bits = ceil_log2((biome + 1) as usize);
            let SectionBiomes::Indirect { bits, .. } = &biomes else {
                panic!("a diverging cell creates a local palette")
            };
            assert_eq!(*bits, expected_bits);
            let (decoded, bytes) = round_trip(&biomes, 65);
            assert_eq!(bytes[0], expected_bits);
            for entry in 0..64 {
                let (x, y, z) = (entry & 3, entry >> 4, (entry >> 2) & 3);
                assert_eq!(decoded.get(x, y, z), biomes.get(x, y, z));
            }
        }
    }

    #[test]
    fn ninth_distinct_biome_switches_to_registry_global_wire() {
        let mut biomes = SectionBiomes::single(0, 65).unwrap();
        for biome in 1..9 {
            let entry = biome as usize;
            biomes
                .set(entry & 3, entry >> 4, (entry >> 2) & 3, biome, 65)
                .unwrap();
        }
        let (decoded, bytes) = round_trip(&biomes, 65);
        assert_eq!(bytes[0], 7, "65 published biomes require seven global bits");
        assert_eq!(bytes.len(), 1 + 8 * 8);
        for entry in 0..64 {
            let (x, y, z) = (entry & 3, entry >> 4, (entry >> 2) & 3);
            assert_eq!(decoded.get(x, y, z), biomes.get(x, y, z));
        }
    }

    #[test]
    fn storage_index_is_y_then_z_then_x_across_word_boundaries() {
        let mut biomes = SectionBiomes::single(0, 65).unwrap();
        for (entry, biome) in [(0, 1), (20, 2), (21, 3), (42, 4), (63, 5)] {
            biomes
                .set(entry & 3, entry >> 4, (entry >> 2) & 3, biome, 65)
                .unwrap();
        }
        let (decoded, _) = round_trip(&biomes, 65);
        for (entry, biome) in [(0, 1), (20, 2), (21, 3), (42, 4), (63, 5)] {
            assert_eq!(decoded.get(entry & 3, entry >> 4, (entry >> 2) & 3), biome);
        }
    }

    #[test]
    fn decode_rejects_unknown_ids_duplicate_entries_and_bad_indices() {
        assert!(SectionBiomes::decode(&mut Decoder::new(&[]), 0)
            .unwrap_err()
            .to_string()
            .contains("must not be empty"));

        let mut unknown = Encoder::new();
        unknown.write_u8(0);
        unknown.write_var_i32(65);
        assert!(
            SectionBiomes::decode(&mut Decoder::new(&unknown.into_inner()), 65)
                .unwrap_err()
                .to_string()
                .contains("outside registry")
        );

        let mut duplicate = Encoder::new();
        duplicate.write_u8(1);
        duplicate.write_var_i32(2);
        duplicate.write_var_i32(1);
        duplicate.write_var_i32(1);
        duplicate.write_i64(0);
        assert!(
            SectionBiomes::decode(&mut Decoder::new(&duplicate.into_inner()), 65)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );

        let mut bad_index = Encoder::new();
        bad_index.write_u8(2);
        bad_index.write_var_i32(2);
        bad_index.write_var_i32(0);
        bad_index.write_var_i32(1);
        bad_index.write_i64(3);
        for _ in 1..word_count(2) {
            bad_index.write_i64(0);
        }
        assert!(
            SectionBiomes::decode(&mut Decoder::new(&bad_index.into_inner()), 65)
                .unwrap_err()
                .to_string()
                .contains("out of range")
        );
    }
}
