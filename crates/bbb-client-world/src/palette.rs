pub use bbb_render_types::{
    packed_long_len, palette_kind, PaletteDomain, PaletteKind, PaletteValue, PalettedContainerData,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_needed(max_value: u64) -> u8 {
        (u64::BITS - max_value.leading_zeros()).max(1) as u8
    }

    fn pack_values_to_longs(values: &[u64], bits_per_entry: usize) -> Vec<i64> {
        let values_per_long = 64 / bits_per_entry;
        let mut packed = vec![0u64; values.len().div_ceil(values_per_long)];
        let mask = (1u64 << bits_per_entry) - 1;
        for (index, value) in values.iter().copied().enumerate() {
            let cell_index = index / values_per_long;
            let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
            packed[cell_index] |= (value & mask) << bit_index;
        }
        packed.into_iter().map(|value| value as i64).collect()
    }

    #[test]
    fn palette_updates_retain_compact_storage_until_domain_capacity() {
        for (domain, local_bits) in [(PaletteDomain::BlockStates, 8), (PaletteDomain::Biomes, 3)] {
            let capacity = 1usize << local_bits;
            let entry_count = capacity + 3;
            let mut expected = vec![13_332; entry_count];
            let mut container = PalettedContainerData {
                domain,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![13_332],
                packed_data: Vec::new(),
                entry_count,
            };

            assert!(container.set_value_at(entry_count - 1, 27_047));
            expected[entry_count - 1] = 27_047;
            assert_eq!(container.palette_kind, PaletteKind::Local);
            assert_eq!(container.packed_data.len(), entry_count.div_ceil(64));
            assert_eq!(
                (0..entry_count)
                    .map(|index| container.value_at(index).unwrap().global_id)
                    .collect::<Vec<_>>(),
                expected,
            );

            for palette_index in 2..capacity {
                let index = palette_index - 2;
                let global_id = 100 + palette_index as i32;
                assert!(container.set_value_at(index, global_id));
                expected[index] = global_id;
                assert_eq!(container.palette_kind, PaletteKind::Local);
                assert_eq!(
                    container.packed_data.len(),
                    packed_long_len(entry_count, bits_needed(palette_index as u64) as usize),
                );
                assert_eq!(
                    (0..entry_count)
                        .map(|index| container.value_at(index).unwrap().global_id)
                        .collect::<Vec<_>>(),
                    expected,
                );
            }

            assert!(container.set_value_at(capacity - 2, 100 + capacity as i32));
            expected[capacity - 2] = 100 + capacity as i32;
            assert_eq!(container.palette_kind, PaletteKind::Global);
            assert_eq!(
                container.packed_data.len(),
                packed_long_len(entry_count, 15)
            );
            assert_eq!(
                (0..entry_count)
                    .map(|index| container.value_at(index).unwrap().global_id)
                    .collect::<Vec<_>>(),
                expected,
            );
        }
    }

    #[test]
    fn global_palette_set_value_grows_bits_for_higher_state_ids() {
        let mut container = PalettedContainerData {
            domain: PaletteDomain::BlockStates,
            bits_per_entry: 14,
            palette_kind: PaletteKind::Global,
            palette_global_ids: Vec::new(),
            packed_data: pack_values_to_longs(&[0, 13_332, 9], 14),
            entry_count: 3,
        };

        assert!(container.set_value_at(2, 27_047));

        assert_eq!(container.palette_kind, PaletteKind::Global);
        assert_eq!(container.bits_per_entry, 15);
        assert_eq!(container.value_at(0).unwrap().global_id, 0);
        assert_eq!(container.value_at(1).unwrap().global_id, 13_332);
        assert_eq!(container.value_at(2).unwrap().global_id, 27_047);
    }
}
