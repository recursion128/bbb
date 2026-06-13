use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteDomain {
    BlockStates,
    Biomes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteKind {
    SingleValue,
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PalettedContainerData {
    pub domain: PaletteDomain,
    pub bits_per_entry: u8,
    pub palette_kind: PaletteKind,
    pub palette_global_ids: Vec<i32>,
    pub packed_data: Vec<i64>,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteValue {
    pub global_id: i32,
    pub palette_index: Option<usize>,
}

impl PalettedContainerData {
    pub fn value_at(&self, index: usize) -> Option<PaletteValue> {
        if index >= self.entry_count {
            return None;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => Some(PaletteValue {
                global_id: self.palette_global_ids.first().copied()?,
                palette_index: Some(0),
            }),
            PaletteKind::Local => {
                let palette_index =
                    read_packed_value(&self.packed_data, self.bits_per_entry, index)?;
                let palette_index = usize::try_from(palette_index).ok()?;
                Some(PaletteValue {
                    global_id: *self.palette_global_ids.get(palette_index)?,
                    palette_index: Some(palette_index),
                })
            }
            PaletteKind::Global => Some(PaletteValue {
                global_id: read_packed_value(&self.packed_data, self.bits_per_entry, index)? as i32,
                palette_index: None,
            }),
        }
    }

    pub fn set_value_at(&mut self, index: usize, global_id: i32) -> bool {
        if index >= self.entry_count || global_id < 0 {
            return false;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => {
                if self.palette_global_ids.first().copied() == Some(global_id) {
                    return true;
                }
                self.upgrade_to_global_with(index, global_id)
            }
            PaletteKind::Local => {
                if let Some(palette_index) = self
                    .palette_global_ids
                    .iter()
                    .position(|id| *id == global_id)
                {
                    set_packed_value(
                        &mut self.packed_data,
                        self.bits_per_entry,
                        index,
                        palette_index as u64,
                    )
                } else {
                    self.upgrade_to_global_with(index, global_id)
                }
            }
            PaletteKind::Global => set_packed_value(
                &mut self.packed_data,
                self.bits_per_entry,
                index,
                global_id as u64,
            ),
        }
    }

    fn upgrade_to_global_with(&mut self, index: usize, global_id: i32) -> bool {
        let mut values = Vec::with_capacity(self.entry_count);
        let mut max_value = global_id as u64;
        for entry_index in 0..self.entry_count {
            let Some(value) = self.value_at(entry_index) else {
                return false;
            };
            let Ok(global_id) = u64::try_from(value.global_id) else {
                return false;
            };
            max_value = max_value.max(global_id);
            values.push(global_id);
        }

        values[index] = global_id as u64;
        self.bits_per_entry = bits_needed(max_value);
        self.palette_kind = PaletteKind::Global;
        self.palette_global_ids.clear();
        self.packed_data = pack_values_to_longs(&values, self.bits_per_entry as usize);
        true
    }
}

pub(crate) fn palette_kind(domain: PaletteDomain, bits_per_entry: u8) -> PaletteKind {
    match (domain, bits_per_entry) {
        (_, 0) => PaletteKind::SingleValue,
        (PaletteDomain::BlockStates, 1..=8) => PaletteKind::Local,
        (PaletteDomain::Biomes, 1..=3) => PaletteKind::Local,
        _ => PaletteKind::Global,
    }
}

pub(crate) fn packed_long_len(entry_count: usize, bits_per_entry: usize) -> usize {
    if bits_per_entry == 0 {
        0
    } else {
        let values_per_long = 64 / bits_per_entry;
        entry_count.div_ceil(values_per_long)
    }
}

fn read_packed_value(packed_data: &[i64], bits_per_entry: u8, index: usize) -> Option<u64> {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return None;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return None;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let cell = *packed_data.get(cell_index)? as u64;
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    Some((cell >> bit_index) & mask)
}

fn set_packed_value(packed_data: &mut [i64], bits_per_entry: u8, index: usize, value: u64) -> bool {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return false;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return false;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let Some(cell) = packed_data.get_mut(cell_index) else {
        return false;
    };
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if value & !mask != 0 {
        return false;
    }

    let raw = *cell as u64;
    *cell = ((raw & !(mask << bit_index)) | (value << bit_index)) as i64;
    true
}

fn pack_values_to_longs(values: &[u64], bits_per_entry: usize) -> Vec<i64> {
    if bits_per_entry == 0 {
        return Vec::new();
    }

    let values_per_long = 64 / bits_per_entry;
    if values_per_long == 0 {
        return Vec::new();
    }

    let mut packed = vec![0u64; values.len().div_ceil(values_per_long)];
    let mask = if bits_per_entry == 64 {
        u64::MAX
    } else {
        (1u64 << bits_per_entry) - 1
    };
    for (index, value) in values.iter().copied().enumerate() {
        let cell_index = index / values_per_long;
        let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
        packed[cell_index] |= (value & mask) << bit_index;
    }

    packed.into_iter().map(|value| value as i64).collect()
}

fn bits_needed(max_value: u64) -> u8 {
    (u64::BITS - max_value.leading_zeros()).max(1) as u8
}
