use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use bbb_pack::{PackRoots, ParticleDefinitionCatalog, ParticleSpriteCatalog};
use bbb_protocol::packets::{LevelParticles, Vec3d};
use bbb_renderer::{ParticleSpawnBatch, ParticleSpawnCommand};

use crate::particle_registry::{vanilla_particle_type, ParticleTypeInfo};

pub(crate) trait ParticleEventSink {
    fn spawn_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch;
}

pub(crate) struct NativeParticleRuntime {
    resolver: ParticleCommandResolver,
}

impl NativeParticleRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        Ok(Self {
            resolver: ParticleCommandResolver::new(
                roots
                    .load_particle_definition_catalog()
                    .context("load particle definition catalog")?,
                roots
                    .load_particle_sprite_catalog()
                    .context("load particle sprite catalog")?,
            ),
        })
    }
}

impl ParticleEventSink for NativeParticleRuntime {
    fn spawn_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
        self.resolver.resolve_level_particles(packet)
    }
}

#[derive(Debug, Clone)]
struct ParticleCommandResolver {
    definitions: ParticleDefinitionCatalog,
    sprites: ParticleSpriteCatalog,
    random: LegacyRandom,
}

impl ParticleCommandResolver {
    fn new(definitions: ParticleDefinitionCatalog, sprites: ParticleSpriteCatalog) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(default_particle_seed()),
        }
    }

    #[cfg(test)]
    fn with_seed(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        seed: i64,
    ) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(seed),
        }
    }

    fn resolve_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
        if packet.count < 0 {
            return ParticleSpawnBatch::default();
        }

        let Some(particle_type) = vanilla_particle_type(packet.particle.particle_type_id) else {
            return ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };
        let Some(definition) = self.definitions.definition(particle_type.name) else {
            return ParticleSpawnBatch {
                missing_definition_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };

        let sprite_ids = definition.textures.clone();
        let missing_sprite_count = sprite_ids
            .iter()
            .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
            .count();
        let override_limiter = particle_type.override_limiter || packet.override_limiter;
        let raw_options_len = packet.particle.raw_options.len();
        let command_count = if packet.count == 0 {
            1
        } else {
            packet.count as usize
        };
        let mut commands = Vec::with_capacity(command_count);

        if packet.count == 0 {
            commands.push(self.command(
                packet,
                particle_type,
                &sprite_ids,
                packet.position,
                Vec3d {
                    x: packet.offset.x * f64::from(packet.max_speed),
                    y: packet.offset.y * f64::from(packet.max_speed),
                    z: packet.offset.z * f64::from(packet.max_speed),
                },
                override_limiter,
                raw_options_len,
            ));
        } else {
            for _ in 0..packet.count {
                let position = Vec3d {
                    x: packet.position.x + self.random.next_gaussian() * packet.offset.x,
                    y: packet.position.y + self.random.next_gaussian() * packet.offset.y,
                    z: packet.position.z + self.random.next_gaussian() * packet.offset.z,
                };
                let velocity = Vec3d {
                    x: self.random.next_gaussian() * f64::from(packet.max_speed),
                    y: self.random.next_gaussian() * f64::from(packet.max_speed),
                    z: self.random.next_gaussian() * f64::from(packet.max_speed),
                };
                commands.push(self.command(
                    packet,
                    particle_type,
                    &sprite_ids,
                    position,
                    velocity,
                    override_limiter,
                    raw_options_len,
                ));
            }
        }

        ParticleSpawnBatch {
            commands,
            missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    fn command(
        &self,
        packet: &LevelParticles,
        particle_type: ParticleTypeInfo,
        sprite_ids: &[String],
        position: Vec3d,
        velocity: Vec3d,
        override_limiter: bool,
        raw_options_len: usize,
    ) -> ParticleSpawnCommand {
        ParticleSpawnCommand {
            particle_type_id: packet.particle.particle_type_id,
            particle_id: particle_type.name.to_string(),
            sprite_ids: sprite_ids.to_vec(),
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            override_limiter,
            always_show: packet.always_show,
            raw_options_len,
        }
    }
}

fn default_particle_seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0)
}

const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;

#[derive(Debug, Clone)]
struct LegacyRandom {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl LegacyRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
            next_gaussian: None,
        }
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let v1 = 2.0 * self.next_f64() - 1.0;
            let v2 = 2.0 * self.next_f64() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                self.next_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }

    fn next_f64(&mut self) -> f64 {
        let high = (self.next_bits(26) as u64) << 27;
        let low = self.next_bits(27) as u64;
        (high + low) as f64 / ((1_u64 << 53) as f64)
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn legacy_random_gaussian_matches_java_samples() {
        let mut random = LegacyRandom::new(0);
        assert_close(random.next_gaussian(), 0.8025330637390305);
        assert_close(random.next_gaussian(), -0.9015460884175122);
        assert_close(random.next_gaussian(), 2.080920790428163);
    }

    #[test]
    fn count_zero_emits_single_spawn_with_offset_velocity() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 0));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.unknown_particle_type_count, 0);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, 4);
        assert_eq!(command.particle_id, "minecraft:cloud");
        assert_eq!(
            command.sprite_ids,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:generic_6".to_string(),
            ]
        );
        assert_eq!(command.position, [10.0, 64.5, -3.25]);
        assert_close(command.velocity[0], 0.15);
        assert_close(command.velocity[1], 0.30);
        assert_close(command.velocity[2], 0.45);
        assert!(command.override_limiter);
        assert!(command.always_show);
        assert_eq!(command.raw_options_len, 2);
    }

    #[test]
    fn positive_count_emits_deterministic_gaussian_scatter() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 2));

        assert_eq!(batch.len(), 2);
        let first = &batch.commands[0];
        assert_close(first.position[0], 10.080253306373904);
        assert_close(first.position[1], 64.3196907823165);
        assert_close(first.position[2], -2.625723762871551);
        assert_close(first.velocity[0], 1.1456561526547341);
        assert_close(first.velocity[1], 1.4768617993237692);
        assert_close(first.velocity[2], -2.525118388151014);
    }

    #[test]
    fn negative_count_emits_no_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, -1));

        assert!(batch.is_empty());
    }

    #[test]
    fn missing_definition_records_diagnostic_without_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(47, 1));

        assert!(batch.commands.is_empty());
        assert_eq!(batch.missing_definition_count, 1);
        assert_eq!(batch.unknown_particle_type_count, 0);
    }

    #[test]
    fn unknown_particle_type_records_diagnostic_without_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(999, 1));

        assert!(batch.commands.is_empty());
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.unknown_particle_type_count, 1);
    }

    #[test]
    fn missing_sprite_records_diagnostic_without_dropping_spawn_command() {
        let mut resolver = test_resolver_with_cloud_textures(
            0,
            &["minecraft:generic_7", "minecraft:missing_particle"],
            &["generic_7"],
        );
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 1));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.missing_sprite_count, 1);
        assert_eq!(
            batch.commands[0].sprite_ids,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:missing_particle".to_string(),
            ]
        );
    }

    fn test_resolver(seed: i64) -> ParticleCommandResolver {
        test_resolver_with_cloud_textures(
            seed,
            &["minecraft:generic_7", "minecraft:generic_6"],
            &["generic_7", "generic_6", "flame"],
        )
    }

    fn test_resolver_with_cloud_textures(
        seed: i64,
        cloud_textures: &[&str],
        particle_textures: &[&str],
    ) -> ParticleCommandResolver {
        let root = unique_temp_dir("particle-runtime");
        let assets_dir = assets_dir(&root);
        write_particle_atlas(&assets_dir);
        for texture in particle_textures {
            write_test_png(
                &assets_dir
                    .join("textures")
                    .join("particle")
                    .join(format!("{texture}.png")),
                8,
                8,
            );
        }
        write_json(
            &particle_dir(&root).join("cloud.json"),
            &particle_definition_json(cloud_textures),
        );
        write_json(
            &particle_dir(&root).join("flame.json"),
            r#"{
              "textures": [
                "minecraft:flame"
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_definition_catalog()
            .unwrap();
        let sprites = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_sprite_catalog()
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        ParticleCommandResolver::with_seed(catalog, sprites, seed)
    }

    fn level_particles_packet(particle_type_id: i32, count: i32) -> LevelParticles {
        LevelParticles {
            override_limiter: true,
            always_show: true,
            position: Vec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: Vec3d {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
            max_speed: 1.5,
            count,
            particle: bbb_protocol::packets::ParticlePayload {
                particle_type_id,
                raw_options: vec![0xaa, 0xbb],
            },
        }
    }

    fn particle_dir(root: &Path) -> PathBuf {
        assets_dir(root).join("particles")
    }

    fn assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn particle_definition_json(textures: &[&str]) -> String {
        let textures = textures
            .iter()
            .map(|texture| format!("\"{texture}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(r#"{{ "textures": [{textures}] }}"#)
    }

    fn write_particle_atlas(assets_dir: &Path) {
        write_json(
            &assets_dir.join("atlases").join("particles.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "particle"
                }
              ]
            }"#,
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut image = image::RgbaImage::new(width, height);
        for (index, pixel) in image.pixels_mut().enumerate() {
            let shade = (index % 255) as u8;
            *pixel = image::Rgba([shade, 255 - shade, 64, 255]);
        }
        image.save(path).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}"))
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }
}
