//! `EnchantmentEntityEffect` (`EnchantmentEntityEffect.java:13-52`) and the
//! `TargetedConditionalEffect` lists of `POST_ATTACK`
//! (`EnchantmentEffectComponents.java:46-49`).

use std::sync::Arc;

use crate::explosion::{ExplosionInteraction, ExplosionPresentation};
use crate::random::GameRandom;
use bbb_protocol::packets::{
    LevelParticles, ParticlePayload, SoundEventHolder, Vec3d, WeightedExplosionParticle,
};
use bbb_protocol::{PersistentRegistryResolver, PersistentRegistryTagMembers};
use bbb_registry_codec::FloatProvider;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_datapack::worldgen::{BlockPredicate, BlockStateProvider};
use rustc_hash::FxHashMap;

use crate::enchantment_facts::{
    builtin_enchantment_documents, builtin_enchantment_registry, ENCHANTMENT_REGISTRY,
};
use bbb_registry_codec::level_based_value::LevelBasedValue;

pub const POST_ATTACK_EFFECT: &str = "minecraft:post_attack";
pub const POST_PIERCING_ATTACK_EFFECT: &str = "minecraft:post_piercing_attack";

/// An `EnchantmentEntityEffect` as far as this server applies one.
#[derive(Debug, Clone, PartialEq)]
pub enum EnchantmentEntityEffect {
    /// `AllOf.EntityEffects` (`AllOf.java:31-47`), in list order.
    AllOf(Vec<EnchantmentEntityEffect>),
    /// `ApplyMobEffect` (`ApplyMobEffect.java:21-56`).
    ApplyMobEffect {
        to_apply: HolderSet,
        min_duration: LevelBasedValue,
        max_duration: LevelBasedValue,
        min_amplifier: LevelBasedValue,
        max_amplifier: LevelBasedValue,
    },
    /// `ApplyEntityImpulse` (`ApplyEntityImpulse.java:12-39`).
    ApplyImpulse {
        direction: [f64; 3],
        coordinate_scale: [f64; 3],
        magnitude: LevelBasedValue,
    },
    /// `ApplyExhaustion` (`ApplyExhaustion.java:12-28`).
    ApplyExhaustion(LevelAmount),
    /// `ChangeItemDamage` (`ChangeItemDamage.java:14-33`).
    ChangeItemDamage(LevelAmount),
    /// `DamageEntity` (`DamageEntity.java:15-35`).
    DamageEntity {
        damage_type: String,
        min_damage: LevelBasedValue,
        max_damage: LevelBasedValue,
    },
    Explode(Box<EnchantmentExplosion>),
    /// `Ignite` (`Ignite.java:11-25`).
    Ignite {
        duration: LevelBasedValue,
    },
    /// `PlaySoundEffect` (`PlaySoundEffect.java:18-50`).
    PlaySound {
        sounds: Vec<SoundEventHolder>,
        volume: FloatProvider,
        pitch: FloatProvider,
    },
    /// `ReplaceDisk` (`ReplaceDisk.java:20-60`).
    ReplaceDisk {
        radius: LevelBasedValue,
        height: LevelBasedValue,
        offset: [i32; 3],
        predicate: Option<BlockPredicate>,
        block_state: BlockStateProvider,
        trigger_game_event: Option<String>,
    },
    ReplaceBlock {
        offset: [i32; 3],
        predicate: Option<BlockPredicate>,
        block_state: BlockStateProvider,
        trigger_game_event: Option<String>,
    },
    SetBlockProperties {
        properties: Vec<(String, String)>,
        offset: [i32; 3],
        trigger_game_event: Option<String>,
    },
    /// `SpawnParticlesEffect` (`SpawnParticlesEffect.java:20-141`) of a
    /// simple particle type.
    SpawnParticles(SpawnParticles),
    /// `SummonEntityEffect` (`SummonEntityEffect.java:22-57`).
    SummonEntity {
        entity_types: HolderSet,
        join_team: bool,
    },
    RunFunction(String),
    /// A type or shape this server does not apply. It is refused where it
    /// would run, after its requirements matched, so an effect vanilla's own
    /// requirements never reach stays silent.
    Unmodelled(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentExplosion {
    pub attribute_to_user: bool,
    pub damage_type: Option<String>,
    pub knockback_multiplier: LevelBasedValue,
    pub immune_blocks: Option<Arc<rustc_hash::FxHashSet<i32>>>,
    pub offset: Vec3d,
    pub radius: LevelBasedValue,
    pub create_fire: bool,
    pub interaction: ExplosionInteraction,
    pub presentation: ExplosionPresentation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LevelAmount(pub(crate) LevelBasedValue);

impl LevelAmount {
    #[must_use]
    pub fn calculate(&self, level: i32) -> f32 {
        self.0.calculate(level)
    }
}

/// `RegistryCodecs.homogeneousList`: direct ids or a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderSet {
    Direct(Vec<String>),
    Tag(String),
}

pub(crate) fn sample_float_provider(
    provider: &FloatProvider,
    mut next_float: impl FnMut() -> f32,
) -> f32 {
    match *provider {
        FloatProvider::Constant(value) => value,
        FloatProvider::Uniform {
            min_inclusive,
            max_exclusive,
        } => next_float() * (max_exclusive - min_inclusive) + min_inclusive,
        FloatProvider::ClampedNormal {
            mean,
            deviation,
            min,
            max,
        } => {
            // A Box-Muller standard normal from two uniform draws; only
            // the distribution of `Mth.normal` matters, not its stream.
            let u = f64::from(1.0 - next_float());
            let v = f64::from(next_float());
            let gaussian = (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos();
            (mean + gaussian as f32 * deviation).clamp(min, max)
        }
        FloatProvider::Trapezoid { min, max, plateau } => {
            let range = max - min;
            let plateau_start = (range - plateau) / 2.0;
            let plateau_end = range - plateau_start;
            min + next_float() * plateau_end + next_float() * plateau_start
        }
    }
}

/// `SpawnParticlesEffect.PositionSource` (`SpawnParticlesEffect.java:80-98`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticlePositionSource {
    pub in_bounding_box: bool,
    pub offset: f32,
    pub scale: f32,
}

impl ParticlePositionSource {
    /// `PositionSource.getCoordinate` (`SpawnParticlesEffect.java:95-97`).
    fn coordinate(&self, position: f64, center: f64, span: f32, random: &GameRandom) -> f64 {
        let base = if self.in_bounding_box {
            center + (random.next_double() - 0.5) * f64::from(span * self.scale)
        } else {
            position
        };
        base + f64::from(self.offset)
    }
}

/// `SpawnParticlesEffect.VelocitySource` (`SpawnParticlesEffect.java:128-140`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleVelocitySource {
    pub movement_scale: f32,
    pub base: FloatProvider,
}

impl ParticleVelocitySource {
    fn velocity(&self, movement: f64, random: &GameRandom) -> f64 {
        movement * f64::from(self.movement_scale)
            + f64::from(sample_float_provider(&self.base, || random.next_float()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnParticles {
    pub particle: ParticlePayload,
    pub horizontal_position: ParticlePositionSource,
    pub vertical_position: ParticlePositionSource,
    pub horizontal_velocity: ParticleVelocitySource,
    pub vertical_velocity: ParticleVelocitySource,
    pub speed: FloatProvider,
}

impl SpawnParticles {
    /// `SpawnParticlesEffect.apply` (`SpawnParticlesEffect.java:57-73`): the
    /// `ServerLevel.sendParticles` packet, drawing from the entity's random in
    /// argument order. `dimensions` is `[bbWidth, bbHeight]`.
    pub fn packet(
        &self,
        position: Vec3d,
        dimensions: [f32; 2],
        known_movement: Vec3d,
        random: &GameRandom,
    ) -> LevelParticles {
        let [width, height] = dimensions;
        let x = self
            .horizontal_position
            .coordinate(position.x, position.x, width, random);
        let y = self.vertical_position.coordinate(
            position.y,
            position.y + f64::from(height / 2.0),
            height,
            random,
        );
        let z = self
            .horizontal_position
            .coordinate(position.z, position.z, width, random);
        let x_dist = self.horizontal_velocity.velocity(known_movement.x, random);
        let y_dist = self.vertical_velocity.velocity(known_movement.y, random);
        let z_dist = self.horizontal_velocity.velocity(known_movement.z, random);
        let speed = sample_float_provider(&self.speed, || random.next_float());
        LevelParticles {
            override_limiter: false,
            always_show: false,
            position: Vec3d { x, y, z },
            offset: Vec3d {
                x: f64::from(x_dist as f32),
                y: f64::from(y_dist as f32),
                z: f64::from(z_dist as f32),
            },
            max_speed: speed,
            count: 0,
            particle: self.particle.clone(),
        }
    }
}

/// `EnchantmentTarget` (`EnchantmentTarget.java:6-23`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnchantmentTarget {
    Attacker,
    DamagingEntity,
    Victim,
}

/// One `TargetedConditionalEffect` (`TargetedConditionalEffect.java:13-49`).
#[derive(Debug, Clone, PartialEq)]
pub struct TargetedEntityEffect {
    pub enchanted: EnchantmentTarget,
    pub affected: EnchantmentTarget,
    pub requirements: Option<LootCondition>,
    pub effect: EnchantmentEntityEffect,
}

pub(crate) struct EnchantmentEntityEffects {
    pub(crate) post_attack: Result<Vec<TargetedEntityEffect>, String>,
    pub(crate) post_piercing_attack: Result<Vec<ConditionalEntityEffect>, String>,
    pub(crate) hit_block: Result<Vec<crate::hit_block_effects::HitBlockEffect>, String>,
}

pub(crate) type EnchantmentEntityEffectTable = FxHashMap<String, EnchantmentEntityEffects>;

pub(crate) fn load_enchantment_entity_effects(
    datapacks: Option<&DatapackStartup>,
) -> Arc<EnchantmentEntityEffectTable> {
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let startup = datapacks.or(builtin.as_deref()).unwrap();
    let registry = startup.registries.registry(ENCHANTMENT_REGISTRY).unwrap();
    Arc::new(
        registry
            .elements
            .iter()
            .map(|element| {
                let id = &element.id;
                (
                    id.clone(),
                    EnchantmentEntityEffects {
                        post_attack: post_attack_effects(Some(startup), id),
                        post_piercing_attack: post_piercing_attack_effects(Some(startup), id),
                        hit_block: crate::hit_block_effects::hit_block_effects(Some(startup), id),
                    },
                )
            })
            .collect(),
    )
}

/// The `minecraft:post_attack` list of one element of the loaded
/// `minecraft:enchantment` registry, or of the built-in pack when the runtime
/// wired no generation. An enchantment without the component answers empty.
pub fn post_attack_effects(
    datapacks: Option<&DatapackStartup>,
    enchantment_id: &str,
) -> Result<Vec<TargetedEntityEffect>, String> {
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let datapacks = datapacks.or(builtin.as_deref());
    let lookup = FrozenDatapackRegistryLookup::from_startup(datapacks.unwrap());
    let Some(list) = component(
        enchantment_document(datapacks, enchantment_id)?,
        POST_ATTACK_EFFECT,
    )
    .map_err(|detail| {
        format!("enchantment {enchantment_id} declares {POST_ATTACK_EFFECT} as {detail}")
    })?
    else {
        return Ok(Vec::new());
    };
    list.iter()
        .enumerate()
        .map(|(index, entry)| {
            let target = |field: &str| {
                entry
                    .get(field)
                    .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
                    .as_deref()
                    .and_then(parse_target)
                    .ok_or_else(|| format!("entry {index} without a valid {field}"))
            };
            let enchanted = target("enchanted")?;
            let affected = target("affected")?;
            let effect = entry
                .get("effect")
                .ok_or_else(|| format!("entry {index} without an effect"))
                .and_then(|effect| parse_entity_effect(effect, &lookup))?;
            let requirements = entry
                .get("requirements")
                .map(|value| {
                    LootCondition::decode(
                        bbb_registry_codec::RegistryElementValueRef::Json(value),
                        Some(&lookup),
                    )
                })
                .transpose()
                .map_err(|error| format!("entry {index} requirements: {error}"))?;
            Ok(TargetedEntityEffect {
                enchanted,
                affected,
                requirements,
                effect,
            })
        })
        .collect::<Result<_, String>>()
        .map_err(|detail| format!("enchantment {enchantment_id} {POST_ATTACK_EFFECT}: {detail}"))
}

/// One `ConditionalEffect` (`ConditionalEffect.java:11-30`) of an entity
/// effect.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEntityEffect {
    pub requirements: Option<LootCondition>,
    pub effect: EnchantmentEntityEffect,
}

/// The `minecraft:post_piercing_attack` list of one enchantment, empty
/// without the component.
pub fn post_piercing_attack_effects(
    datapacks: Option<&DatapackStartup>,
    enchantment_id: &str,
) -> Result<Vec<ConditionalEntityEffect>, String> {
    let builtin = datapacks
        .is_none()
        .then(crate::registry_sync::builtin_datapack_startup_shared);
    let datapacks = datapacks.or(builtin.as_deref());
    let lookup = FrozenDatapackRegistryLookup::from_startup(datapacks.unwrap());
    let Some(list) = component(
        enchantment_document(datapacks, enchantment_id)?,
        POST_PIERCING_ATTACK_EFFECT,
    )?
    else {
        return Ok(Vec::new());
    };
    list.iter()
        .map(|entry| {
            Ok(ConditionalEntityEffect {
                requirements: entry
                    .get("requirements")
                    .map(|value| {
                        LootCondition::decode(
                            bbb_registry_codec::RegistryElementValueRef::Json(value),
                            Some(&lookup),
                        )
                    })
                    .transpose()
                    .map_err(|error| format!("requirements: {error}"))?,
                effect: parse_entity_effect(
                    entry.get("effect").ok_or("an entry without an effect")?,
                    &lookup,
                )?,
            })
        })
        .collect::<Result<_, String>>()
        .map_err(|detail| {
            format!("enchantment {enchantment_id} {POST_PIERCING_ATTACK_EFFECT}: {detail}")
        })
}

pub(crate) fn enchantment_document<'a>(
    datapacks: Option<&'a DatapackStartup>,
    enchantment_id: &str,
) -> Result<&'a bbb_registry_codec::RegistryJsonValue, String> {
    match datapacks.and_then(|datapacks| datapacks.registries.registry(ENCHANTMENT_REGISTRY)) {
        Some(registry) => registry
            .elements
            .iter()
            .find(|element| element.id == enchantment_id)
            .and_then(|element| element.json()),
        None => builtin_enchantment_registry()
            .iter()
            .position(|id| id == enchantment_id)
            .map(|index| &builtin_enchantment_documents()[index]),
    }
    .ok_or_else(|| format!("enchantment {enchantment_id} is not in the loaded registry"))
}

/// The list one effect component declares, `None` without it.
pub(crate) fn component<'a>(
    document: &'a bbb_registry_codec::RegistryJsonValue,
    component_id: &str,
) -> Result<Option<&'a [bbb_registry_codec::RegistryJsonValue]>, String> {
    let Some(effects) = document
        .get("effects")
        .and_then(bbb_registry_codec::RegistryJsonValue::as_object)
    else {
        return Ok(None);
    };
    let mut declared = effects.iter().filter(|(name, _)| {
        bbb_protocol::normalize_resource_location((*name).try_to_string().unwrap_or_default())
            .is_ok_and(|id| id == component_id)
    });
    let Some((_, list)) = declared.next() else {
        return Ok(None);
    };
    if declared.next().is_some() {
        return Err("a component declared more than once".to_string());
    }
    list.as_array()
        .map(Some)
        .ok_or_else(|| "a value that is not a list".to_string())
}

fn effect_offset(value: &bbb_registry_codec::RegistryJsonValue) -> Result<[i32; 3], String> {
    let Some(offset) = value.get("offset") else {
        return Ok([0; 3]);
    };
    let axes = offset
        .as_array()
        .filter(|axes| axes.len() == 3)
        .ok_or("an entity effect with an invalid offset")?;
    let axis = |index: usize| {
        axes[index]
            .as_i64()
            .and_then(|axis| i32::try_from(axis).ok())
            .ok_or_else(|| "an entity effect with an invalid offset".to_string())
    };
    Ok([axis(0)?, axis(1)?, axis(2)?])
}

fn parse_target(name: &str) -> Option<EnchantmentTarget> {
    match name {
        "attacker" => Some(EnchantmentTarget::Attacker),
        "damaging_entity" => Some(EnchantmentTarget::DamagingEntity),
        "victim" => Some(EnchantmentTarget::Victim),
        _ => None,
    }
}

fn resource_id(value: &bbb_registry_codec::RegistryJsonValue) -> Option<String> {
    value
        .utf8_string()
        .and_then(|id| bbb_protocol::normalize_resource_location(id).ok())
}

fn holder_set(value: &bbb_registry_codec::RegistryJsonValue) -> Result<HolderSet, String> {
    bbb_server_datapack::loot::HolderSet::decode(bbb_registry_codec::RegistryElementValueRef::Json(
        value,
    ))
    .map(|set| match set {
        bbb_server_datapack::loot::HolderSet::Tag(tag) => HolderSet::Tag(tag),
        bbb_server_datapack::loot::HolderSet::Ids(ids) => HolderSet::Direct(ids),
    })
    .map_err(|error| error.to_string())
}

fn level_value(
    value: &bbb_registry_codec::RegistryJsonValue,
    field: &str,
) -> Result<LevelBasedValue, String> {
    value
        .get(field)
        .ok_or_else(|| format!("missing {field}"))
        .and_then(LevelBasedValue::parse)
}

fn float_provider(value: &bbb_registry_codec::RegistryJsonValue) -> Option<FloatProvider> {
    FloatProvider::decode_source(bbb_registry_codec::RegistryElementValueRef::Json(value)).ok()
}

fn particle_payload(
    value: &bbb_registry_codec::RegistryJsonValue,
    lookup: &FrozenDatapackRegistryLookup<'_>,
) -> Result<ParticlePayload, String> {
    let particle = bbb_registry_codec::registry_codec::environment_source::particles::particle_nbt_from_source(
        bbb_registry_codec::RegistryElementValueRef::Json(value),
        bbb_registry_codec::RegistryElementSource::ServerDatapackJson,
        Default::default(),
    ).map_err(|error| error.to_string())?;
    let resource_id = bbb_registry_codec::RegistryElementValueRef::Nbt(&particle)
        .field("type")
        .and_then(|value| value.utf8_string())
        .ok_or("particle without a type")?;
    bbb_server_datapack::particle_options::particle_payload_from_nbt(
        &resource_id,
        &particle,
        lookup,
    )
}

pub(crate) fn parse_entity_effect(
    value: &bbb_registry_codec::RegistryJsonValue,
    lookup: &FrozenDatapackRegistryLookup<'_>,
) -> Result<EnchantmentEntityEffect, String> {
    let effect_type = value
        .get("type")
        .and_then(resource_id)
        .ok_or_else(|| "an entity effect without a type".to_string())?;
    let unmodelled = |detail: &str| {
        Ok(EnchantmentEntityEffect::Unmodelled(format!(
            "{effect_type} {detail}"
        )))
    };
    match effect_type.as_str() {
        "minecraft:all_of" => value
            .get("effects")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .ok_or_else(|| "all_of without an effects list".to_string())?
            .iter()
            .map(|effect| parse_entity_effect(effect, lookup))
            .collect::<Result<Vec<_>, _>>()
            .map(EnchantmentEntityEffect::AllOf),
        "minecraft:run_function" => Ok(EnchantmentEntityEffect::RunFunction(
            value
                .get("function")
                .and_then(resource_id)
                .ok_or("run_function without a function")?,
        )),
        "minecraft:replace_disk" | "minecraft:replace_block" => {
            let provider = value
                .get("block_state")
                .ok_or("a block replacement without its retained block_state codec")?;
            let block_state = BlockStateProvider::decode(provider)
                .map_err(|error| format!("block_state: {error}"))?;
            let predicate = value
                .get("predicate")
                .filter(|value| !value.is_null())
                .map(|value| BlockPredicate::decode(value).map_err(|error| error.to_string()))
                .transpose()?;
            let offset = effect_offset(value)?;
            let trigger_game_event = value.get("trigger_game_event").and_then(resource_id);
            if effect_type == "minecraft:replace_block" {
                return Ok(EnchantmentEntityEffect::ReplaceBlock {
                    offset,
                    predicate,
                    block_state,
                    trigger_game_event,
                });
            }
            Ok(EnchantmentEntityEffect::ReplaceDisk {
                radius: level_value(value, "radius")?,
                height: level_value(value, "height")?,
                offset,
                predicate,
                block_state,
                trigger_game_event: value.get("trigger_game_event").and_then(resource_id),
            })
        }
        "minecraft:set_block_properties" => {
            let properties = value
                .get("properties")
                .and_then(bbb_registry_codec::RegistryJsonValue::as_object)
                .ok_or("set_block_properties without properties")?
                .iter()
                .map(|(name, value)| {
                    Ok((
                        name.try_to_string()
                            .map_err(|_| "invalid block property name")?,
                        value
                            .utf8_string()
                            .ok_or("a block property without a string value")?,
                    ))
                })
                .collect::<Result<_, String>>()?;
            Ok(EnchantmentEntityEffect::SetBlockProperties {
                properties,
                offset: effect_offset(value)?,
                trigger_game_event: value.get("trigger_game_event").and_then(resource_id),
            })
        }
        "minecraft:spawn_particles" => {
            let particle_id = value
                .get("particle")
                .and_then(|particle| particle.get("type"))
                .and_then(resource_id)
                .ok_or("spawn_particles without a particle type")?;
            // `SpawnParticlesEffect.particle` is a full `ParticleOptions`
            // (`SpawnParticlesEffect.java:30`), read from the retained codec form.
            let particle = match value.get("particle") {
                Some(options) => match particle_payload(options, lookup) {
                    Ok(particle) => particle,
                    Err(detail) => return unmodelled(&format!("particle: {detail}")),
                },
                None => match bbb_protocol::particle_types::vanilla_particle_type_for_resource_id(
                    &particle_id,
                )
                .filter(|entry| {
                    entry.codec_family
                        == bbb_protocol::particle_types::VanillaParticleCodecFamily::Simple
                }) {
                    Some(entry) => ParticlePayload {
                        particle_type_id: entry.id,
                        raw_options: Vec::new(),
                    },
                    None => return unmodelled("without the retained codec form of its particle"),
                },
            };
            let position = |field: &str| -> Result<ParticlePositionSource, String> {
                let source = value
                    .get(field)
                    .ok_or_else(|| format!("spawn_particles without {field}"))?;
                let number = |name: &str, default: f32| {
                    source
                        .get(name)
                        .map_or(Some(default), |value| {
                            value.as_f64().map(|value| value as f32)
                        })
                        .ok_or_else(|| format!("{field}.{name} is not a number"))
                };
                let in_bounding_box = match source
                    .get("type")
                    .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
                    .as_deref()
                {
                    Some("entity_position") => false,
                    Some("in_bounding_box") => true,
                    _ => return Err(format!("{field} with an unknown type")),
                };
                Ok(ParticlePositionSource {
                    in_bounding_box,
                    offset: number("offset", 0.0)?,
                    scale: number("scale", 1.0)?,
                })
            };
            let velocity = |field: &str| -> Result<Option<ParticleVelocitySource>, String> {
                let source = value
                    .get(field)
                    .ok_or_else(|| format!("spawn_particles without {field}"))?;
                let movement_scale = source
                    .get("movement_scale")
                    .map_or(Some(0.0), |value| value.as_f64().map(|value| value as f32))
                    .ok_or_else(|| format!("{field}.movement_scale is not a number"))?;
                let base = match source.get("base").filter(|value| !value.is_null()) {
                    Some(base) => float_provider(base),
                    None => Some(FloatProvider::Constant(0.0)),
                };
                Ok(base.map(|base| ParticleVelocitySource {
                    movement_scale,
                    base,
                }))
            };
            let speed = match value.get("speed").filter(|value| !value.is_null()) {
                Some(speed) => float_provider(speed),
                None => Some(FloatProvider::Constant(0.0)),
            };
            let (Some(horizontal_velocity), Some(vertical_velocity), Some(speed)) = (
                velocity("horizontal_velocity")?,
                velocity("vertical_velocity")?,
                speed,
            ) else {
                return unmodelled("with a float provider this server does not sample");
            };
            Ok(EnchantmentEntityEffect::SpawnParticles(SpawnParticles {
                particle,
                horizontal_position: position("horizontal_position")?,
                vertical_position: position("vertical_position")?,
                horizontal_velocity,
                vertical_velocity,
                speed,
            }))
        }
        "minecraft:apply_mob_effect" => Ok(EnchantmentEntityEffect::ApplyMobEffect {
            to_apply: holder_set(value.get("to_apply").ok_or("missing to_apply")?)?,
            min_duration: level_value(value, "min_duration")?,
            max_duration: level_value(value, "max_duration")?,
            min_amplifier: level_value(value, "min_amplifier")?,
            max_amplifier: level_value(value, "max_amplifier")?,
        }),
        "minecraft:apply_impulse" => {
            let vector = |field: &str| -> Result<[f64; 3], String> {
                let axes = value
                    .get(field)
                    .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
                    .filter(|axes| axes.len() == 3)
                    .ok_or_else(|| format!("apply_impulse without a {field}"))?;
                let axis = |index: usize| {
                    axes[index]
                        .as_f64()
                        .ok_or_else(|| format!("apply_impulse {field} is not numeric"))
                };
                Ok([axis(0)?, axis(1)?, axis(2)?])
            };
            Ok(EnchantmentEntityEffect::ApplyImpulse {
                direction: vector("direction")?,
                coordinate_scale: vector("coordinate_scale")?,
                magnitude: level_value(value, "magnitude")?,
            })
        }
        "minecraft:apply_exhaustion" => level_value(value, "amount")
            .map(|amount| EnchantmentEntityEffect::ApplyExhaustion(LevelAmount(amount))),
        "minecraft:change_item_damage" => value
            .get("amount")
            .ok_or_else(|| "change_item_damage without an amount".to_string())
            .and_then(LevelBasedValue::parse)
            .map(|amount| EnchantmentEntityEffect::ChangeItemDamage(LevelAmount(amount))),
        "minecraft:damage_entity" => Ok(EnchantmentEntityEffect::DamageEntity {
            damage_type: value
                .get("damage_type")
                .and_then(resource_id)
                .ok_or("damage_entity without a damage_type")?,
            min_damage: level_value(value, "min_damage")?,
            max_damage: level_value(value, "max_damage")?,
        }),
        "minecraft:explode" => {
            let flag = |field: &str| {
                value
                    .get(field)
                    .and_then(bbb_registry_codec::RegistryJsonValue::as_bool)
                    .unwrap_or(false)
            };
            let offset = match value.get("offset") {
                Some(offset) => {
                    let axes = offset
                        .as_array()
                        .filter(|axes| axes.len() == 3)
                        .ok_or("explode with an invalid offset")?;
                    let axis =
                        |index: usize| axes[index].as_f64().ok_or("explode with an invalid offset");
                    Vec3d {
                        x: axis(0)?,
                        y: axis(1)?,
                        z: axis(2)?,
                    }
                }
                None => Vec3d::default(),
            };
            let interaction = match value
                .get("block_interaction")
                .and_then(bbb_registry_codec::RegistryJsonValue::utf8_string)
                .as_deref()
            {
                Some("none") => ExplosionInteraction::None,
                Some("block") => ExplosionInteraction::Block,
                Some("mob") => ExplosionInteraction::Mob,
                Some("tnt") => ExplosionInteraction::Tnt,
                Some("trigger") => ExplosionInteraction::Trigger,
                _ => return Err("explode without a block interaction".into()),
            };
            let immune_blocks = value
                .get("immune_blocks")
                .map(|value| {
                    let names = match holder_set(value)? {
                        HolderSet::Direct(names) => names,
                        HolderSet::Tag(tag) => match lookup.tag_members("minecraft:block", &tag) {
                            PersistentRegistryTagMembers::Present(names) => names,
                            _ => {
                                return Err(format!(
                                    "explode immune block tag {tag} is unavailable"
                                ))
                            }
                        },
                    };
                    names
                        .iter()
                        .map(|name| {
                            bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
                                .map(|block| block.id)
                                .ok_or_else(|| format!("unknown immune block {name}"))
                        })
                        .collect::<Result<rustc_hash::FxHashSet<_>, String>>()
                        .map(Arc::new)
                })
                .transpose()?;
            let particle = |field: &str| {
                particle_payload(
                    value
                        .get(field)
                        .ok_or_else(|| format!("explode without retained {field}"))?,
                    lookup,
                )
            };
            let block_particles = match value.get("block_particles") {
                Some(bbb_registry_codec::RegistryJsonValue::Array(entries)) => entries
                    .iter()
                    .map(|entry| {
                        let field = |name| entry.get(name);
                        let number = |name| {
                            field(name)
                                .and_then(|value| {
                                    bbb_registry_codec::RegistryElementValueRef::Json(value)
                                        .codec_f32()
                                })
                                .unwrap_or(1.0)
                        };
                        let weight = field("weight")
                            .and_then(|value| {
                                bbb_registry_codec::RegistryElementValueRef::Json(value).codec_i32()
                            })
                            .ok_or("explode particle without a weight")?;
                        Ok(WeightedExplosionParticle {
                            particle: particle_payload(
                                field("particle").ok_or("explode block particle missing")?,
                                lookup,
                            )?,
                            scaling: number("scaling"),
                            speed: number("speed"),
                            weight,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                _ => Vec::new(),
            };
            let sound =
                bbb_registry_codec::registry_codec::sound_variants::decode_sound_event_holder(
                    bbb_registry_codec::RegistryElementValueRef::Json(
                        value.get("sound").ok_or("explode without retained sound")?,
                    ),
                    ENCHANTMENT_REGISTRY,
                    "minecraft:explode",
                    "sound",
                )
                .map_err(|error| error.to_string())?;
            let sound = crate::entity::sound_variants::sound_holder(&sound)
                .ok_or("explode sound is not published")?;
            // ExplodeEffect.java:61-81.
            Ok(EnchantmentEntityEffect::Explode(Box::new(
                EnchantmentExplosion {
                    attribute_to_user: flag("attribute_to_user"),
                    damage_type: value.get("damage_type").and_then(resource_id),
                    knockback_multiplier: value
                        .get("knockback_multiplier")
                        .map(LevelBasedValue::parse)
                        .transpose()?
                        .unwrap_or(LevelBasedValue::Constant(1.0)),
                    immune_blocks,
                    offset,
                    radius: level_value(value, "radius")?,
                    create_fire: flag("create_fire"),
                    interaction,
                    presentation: ExplosionPresentation::Custom {
                        small_particle: particle("small_particle")?,
                        large_particle: particle("large_particle")?,
                        block_particles,
                        sound,
                    },
                },
            )))
        }
        "minecraft:ignite" => Ok(EnchantmentEntityEffect::Ignite {
            duration: level_value(value, "duration")?,
        }),
        "minecraft:play_sound" => {
            let sound = value
                .get("sound")
                .ok_or("play_sound without retained sound")?;
            let sounds = match sound {
                bbb_registry_codec::RegistryJsonValue::Array(list) => list.as_slice(),
                _ => std::slice::from_ref(sound),
            }
            .iter()
            .map(|sound| {
                let sound =
                    bbb_registry_codec::registry_codec::sound_variants::decode_sound_event_holder(
                        bbb_registry_codec::RegistryElementValueRef::Json(sound),
                        ENCHANTMENT_REGISTRY,
                        "minecraft:play_sound",
                        "sound",
                    )
                    .map_err(|error| error.to_string())?;
                crate::entity::sound_variants::sound_holder(&sound)
                    .ok_or_else(|| "play_sound sound is not published".to_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
            let (Some(volume), Some(pitch)) = (
                value.get("volume").and_then(float_provider),
                value.get("pitch").and_then(float_provider),
            ) else {
                return unmodelled("with a float provider this server does not sample");
            };
            Ok(EnchantmentEntityEffect::PlaySound {
                sounds,
                volume,
                pitch,
            })
        }
        "minecraft:summon_entity" => Ok(EnchantmentEntityEffect::SummonEntity {
            entity_types: holder_set(value.get("entity").ok_or("missing entity")?)?,
            join_team: value
                .get("join_team")
                .and_then(bbb_registry_codec::RegistryJsonValue::as_bool)
                .unwrap_or(false),
        }),
        _ => Ok(EnchantmentEntityEffect::Unmodelled(effect_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_post_attack_entries_parse_into_applied_effects() {
        let thorns = post_attack_effects(None, "minecraft:thorns").expect("thorns parses");
        assert_eq!(thorns.len(), 1);
        assert_eq!(thorns[0].enchanted, EnchantmentTarget::Victim);
        assert_eq!(thorns[0].affected, EnchantmentTarget::Attacker);
        let EnchantmentEntityEffect::AllOf(effects) = &thorns[0].effect else {
            panic!("thorns is all_of");
        };
        assert!(matches!(
            &effects[0],
            EnchantmentEntityEffect::DamageEntity { damage_type, .. } if damage_type == "minecraft:thorns"
        ));
        for id in [
            "minecraft:fire_aspect",
            "minecraft:bane_of_arthropods",
            "minecraft:channeling",
            "minecraft:wind_burst",
        ] {
            let effects = post_attack_effects(None, id).expect("the entry parses");
            let mut stack = vec![effects[0].effect.clone()];
            while let Some(effect) = stack.pop() {
                match effect {
                    EnchantmentEntityEffect::AllOf(inner) => stack.extend(inner),
                    EnchantmentEntityEffect::Unmodelled(detail) => panic!("{id}: {detail}"),
                    _ => {}
                }
            }
        }
        assert_eq!(
            post_attack_effects(None, "minecraft:sharpness"),
            Ok(Vec::new())
        );
    }
}
