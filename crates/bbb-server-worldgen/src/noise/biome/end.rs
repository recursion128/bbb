use bbb_server_datapack::dynamic_registry::DatapackGeneration;

use super::{refuse, ClimateSampler, NoiseBiomeError, BIOME_REGISTRY};

const END_BIOME_ORDER: [&str; 5] = [
    "minecraft:the_end",
    "minecraft:end_highlands",
    "minecraft:end_midlands",
    "minecraft:small_end_islands",
    "minecraft:end_barrens",
];

/// `TheEndBiomeSource` holder binding and biome selection (`TheEndBiomeSource.java:13-81`).
pub struct EndBiomeSource {
    end: i32,
    highlands: i32,
    midlands: i32,
    islands: i32,
    barrens: i32,
    registry_len: usize,
}

impl EndBiomeSource {
    pub const fn biome_order() -> &'static [&'static str] {
        &END_BIOME_ORDER
    }

    pub fn from_registry(generation: &DatapackGeneration) -> Result<Self, NoiseBiomeError> {
        let biomes = generation
            .registry(BIOME_REGISTRY)
            .ok_or_else(|| refuse(BIOME_REGISTRY, "registry is absent"))?;
        let holder = |id: &str| {
            biomes
                .elements
                .iter()
                .position(|element| element.id == id)
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| {
                    refuse(
                        format!("{BIOME_REGISTRY}.{id}"),
                        "required The End biome holder is absent",
                    )
                })
        };
        Ok(Self {
            end: holder("minecraft:the_end")?,
            highlands: holder("minecraft:end_highlands")?,
            midlands: holder("minecraft:end_midlands")?,
            islands: holder("minecraft:small_end_islands")?,
            barrens: holder("minecraft:end_barrens")?,
            registry_len: biomes.elements.len(),
        })
    }

    pub const fn registry_len(&self) -> usize {
        self.registry_len
    }

    pub fn sample(
        &self,
        climate: &mut impl ClimateSampler,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
    ) -> i32 {
        let block_x = quart_x.wrapping_mul(4);
        let block_y = quart_y.wrapping_mul(4);
        let block_z = quart_z.wrapping_mul(4);
        let section_x = block_x >> 4;
        let section_z = block_z >> 4;
        if i64::from(section_x) * i64::from(section_x) + i64::from(section_z) * i64::from(section_z)
            <= 4096
        {
            return self.end;
        }

        let weird_x = section_x.wrapping_mul(2).wrapping_add(1).wrapping_mul(8);
        let weird_z = section_z.wrapping_mul(2).wrapping_add(1).wrapping_mul(8);
        let erosion = climate
            .sample_erosion(weird_x, block_y, weird_z)
            .expect("The End biome source requires a compiled erosion density");
        self.outer_biome(erosion)
    }

    fn outer_biome(&self, erosion: f64) -> i32 {
        if erosion > 0.25 {
            self.highlands
        } else if erosion >= -0.0625 {
            self.midlands
        } else if erosion < -0.21875 {
            self.islands
        } else {
            self.barrens
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::noise::biome::NoiseBiomeSource;
    use crate::noise::density::NoiseDensitySource;
    use bbb_server_datapack::dynamic_registry::{
        DatapackGeneration, LoadedDynamicRegistry, RawRegistryElement,
    };
    use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

    use super::*;

    fn element(id: &str) -> RawRegistryElement {
        let value = json!({});
        RawRegistryElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            source_json: serde_json::to_vec(&value).unwrap().into(),
            source: Some(
                bbb_registry_codec::decode_direct_registry_json(
                    &serde_json::to_vec(&(value)).unwrap(),
                )
                .unwrap(),
            ),
            codec_value: None,
        }
    }

    fn generation(ids: &[&str]) -> DatapackGeneration {
        DatapackGeneration {
            registries: vec![LoadedDynamicRegistry {
                registry_id: BIOME_REGISTRY.to_string(),
                elements: ids.iter().map(|id| element(id)).collect(),
            }],
            worldgen: Default::default(),
        }
    }

    #[test]
    fn holders_and_all_outer_threshold_edges_are_bound_exactly() {
        let source = EndBiomeSource::from_registry(&generation(&[
            "minecraft:the_end",
            "minecraft:end_highlands",
            "minecraft:end_midlands",
            "minecraft:small_end_islands",
            "minecraft:end_barrens",
        ]))
        .unwrap();
        assert_eq!(source.registry_len(), 5);
        assert_eq!(source.outer_biome(0.250_000_000_1), 1);
        assert_eq!(source.outer_biome(0.25), 2);
        assert_eq!(source.outer_biome(-0.0625), 2);
        assert_eq!(source.outer_biome(-0.062_500_000_1), 4);
        assert_eq!(source.outer_biome(-0.21875), 4);
        assert_eq!(source.outer_biome(-0.218_750_000_1), 3);
    }

    #[test]
    fn built_in_end_keeps_the_center_and_covers_four_outer_biomes() {
        let startup = DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .unwrap();
        let source = EndBiomeSource::from_registry(&startup.registries).unwrap();
        let density_zero =
            NoiseDensitySource::from_registry_with_climate(&startup.registries, "minecraft:end", 0)
                .unwrap();
        let density_42 = NoiseDensitySource::from_registry_with_climate(
            &startup.registries,
            "minecraft:end",
            42,
        )
        .unwrap();
        let owner = NoiseBiomeSource::end_from_registry(&startup.registries).unwrap();
        assert_eq!(owner.registry_len(), source.registry_len());
        let mut search = owner.create_search();
        assert_eq!(
            owner.sample(&density_zero, 0, 0, 0, &mut search),
            source.end
        );
        let outer = [
            source.highlands,
            source.midlands,
            source.islands,
            source.barrens,
        ];
        let mut seen = Vec::new();
        for density in [&density_zero, &density_42] {
            for (quart_x, quart_z) in [(0, 0), (257, 0), (0, -256)] {
                let mut climate = density;
                assert_eq!(source.sample(&mut climate, quart_x, 0, quart_z), source.end);
            }
            for quart_x in (-1_600..=-1_200).step_by(8) {
                for quart_z in (-1_600..=-1_200).step_by(8) {
                    let mut climate = density;
                    let biome = source.sample(&mut climate, quart_x, 0, quart_z);
                    assert!(outer.contains(&biome));
                    if !seen.contains(&biome) {
                        seen.push(biome);
                    }
                }
            }
        }
        assert_eq!(seen.len(), 4, "every outer End biome appears");
    }
}
