//! The option value `ParticleTypes.STREAM_CODEC` writes after the type id
//! (`ParticleTypes.java:158-159`), one variant per option class.

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::content_registry::ContentRegistry;
use crate::item_types::VANILLA_ITEM_AIR_ID;
use crate::particle_types::VanillaParticleCodecFamily;

use super::super::chunks::{decode_block_pos, encode_block_pos, BlockPos};
use super::super::data_components::{
    decode_data_component_patch_summary_with_registry,
    encode_data_component_patch_summary_with_framing_and_registry, ComponentValueFraming,
    ItemStackTemplateSummary,
};
use super::{particle_codec_family, ParticlePayload, Vec3d};

/// `PositionSourceType.BLOCK`, the first `POSITION_SOURCE_TYPE` registration
/// (`PositionSourceType.java:10`).
pub const VANILLA_POSITION_SOURCE_TYPE_BLOCK_ID: i32 = 0;
/// `PositionSourceType.ENTITY` (`PositionSourceType.java:11`).
pub const VANILLA_POSITION_SOURCE_TYPE_ENTITY_ID: i32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum ParticleOptions {
    /// `SimpleParticleType` writes nothing (`SimpleParticleType.java:9`).
    Simple,
    /// `BlockParticleOption.java:23-25`.
    Block { state_id: i32 },
    /// `PowerParticleOption.java:17-19`.
    Power { power: f32 },
    /// `DustParticleOptions.java:21-23`.
    Dust { color: i32, scale: f32 },
    /// `DustColorTransitionOptions.java:23-31`.
    DustColorTransition {
        from_color: i32,
        to_color: i32,
        scale: f32,
    },
    /// `SpellParticleOption.java:27-31`.
    Spell { color: i32, power: f32 },
    /// `ColorParticleOption.java:18-20`.
    Color { color: i32 },
    /// `SculkChargeParticleOptions.java:14-16`.
    SculkCharge { roll: f32 },
    /// `ItemParticleOption.java:17-19` over `ItemStackTemplate.STREAM_CODEC`
    /// (`ItemStackTemplate.java:30-38`).
    Item(Box<ItemStackTemplateSummary>),
    /// `VibrationParticleOption.java:23-29` with a block position source, the
    /// only kind its codec admits (`:14-15`).
    Vibration {
        destination: BlockPos,
        arrival_in_ticks: i32,
    },
    /// The stream codec carries the full `PositionSource.STREAM_CODEC`, so an
    /// `EntityPositionSource` (`EntityPositionSource.java:27-33`) reaches the
    /// client even though the map codec refuses it.
    VibrationEntity {
        entity_id: i32,
        y_offset: f32,
        arrival_in_ticks: i32,
    },
    /// `TrailParticleOption.java:20-28`.
    Trail {
        target: Vec3d,
        color: i32,
        duration: i32,
    },
    /// `ShriekParticleOption.java:14-16`.
    Shriek { delay: i32 },
}

impl ParticleOptions {
    pub fn codec_family(&self) -> VanillaParticleCodecFamily {
        match self {
            Self::Simple => VanillaParticleCodecFamily::Simple,
            Self::Block { .. } => VanillaParticleCodecFamily::Block,
            Self::Power { .. } => VanillaParticleCodecFamily::Power,
            Self::Dust { .. } => VanillaParticleCodecFamily::Dust,
            Self::DustColorTransition { .. } => VanillaParticleCodecFamily::DustColorTransition,
            Self::Spell { .. } => VanillaParticleCodecFamily::Spell,
            Self::Color { .. } => VanillaParticleCodecFamily::Color,
            Self::SculkCharge { .. } => VanillaParticleCodecFamily::SculkCharge,
            Self::Item(_) => VanillaParticleCodecFamily::Item,
            Self::Vibration { .. } | Self::VibrationEntity { .. } => {
                VanillaParticleCodecFamily::Vibration
            }
            Self::Trail { .. } => VanillaParticleCodecFamily::Trail,
            Self::Shriek { .. } => VanillaParticleCodecFamily::Shriek,
        }
    }

    /// The payload for `particle_type_id`, whose registered option class must
    /// be this value's.
    pub fn payload(&self, particle_type_id: i32) -> Result<ParticlePayload> {
        self.payload_with_registry(particle_type_id, ContentRegistry::vanilla())
    }

    pub fn payload_with_registry(
        &self,
        particle_type_id: i32,
        registry: &ContentRegistry,
    ) -> Result<ParticlePayload> {
        let family = particle_codec_family(particle_type_id, registry)?;
        if family != self.codec_family() {
            return Err(ProtocolError::InvalidData(format!(
                "particle type id {particle_type_id} takes {family:?} options, not {:?}",
                self.codec_family()
            )));
        }
        let mut out = Encoder::new();
        match self {
            Self::Simple => {}
            Self::Block { state_id } => out.write_var_i32(*state_id),
            Self::Power { power } => out.write_f32(*power),
            Self::Dust { color, scale } => {
                out.write_i32(*color);
                out.write_f32(*scale);
            }
            Self::DustColorTransition {
                from_color,
                to_color,
                scale,
            } => {
                out.write_i32(*from_color);
                out.write_i32(*to_color);
                out.write_f32(*scale);
            }
            Self::Spell { color, power } => {
                out.write_i32(*color);
                out.write_f32(*power);
            }
            Self::Color { color } => out.write_i32(*color),
            Self::SculkCharge { roll } => out.write_f32(*roll),
            Self::Item(template) => {
                // The record constructor refuses both (`ItemStackTemplate.java:52-60`).
                if template.item_id == VANILLA_ITEM_AIR_ID || template.count == 0 {
                    return Err(ProtocolError::InvalidData(
                        "item particle template must be non-empty".to_string(),
                    ));
                }
                out.write_var_i32(template.item_id);
                out.write_var_i32(template.count);
                encode_data_component_patch_summary_with_framing_and_registry(
                    &mut out,
                    &template.component_patch,
                    ComponentValueFraming::Direct,
                    registry,
                )?;
            }
            Self::Vibration {
                destination,
                arrival_in_ticks,
            } => {
                out.write_var_i32(VANILLA_POSITION_SOURCE_TYPE_BLOCK_ID);
                out.write_i64(encode_block_pos(*destination));
                out.write_var_i32(*arrival_in_ticks);
            }
            Self::VibrationEntity {
                entity_id,
                y_offset,
                arrival_in_ticks,
            } => {
                out.write_var_i32(VANILLA_POSITION_SOURCE_TYPE_ENTITY_ID);
                out.write_var_i32(*entity_id);
                out.write_f32(*y_offset);
                out.write_var_i32(*arrival_in_ticks);
            }
            Self::Trail {
                target,
                color,
                duration,
            } => {
                out.write_f64(target.x);
                out.write_f64(target.y);
                out.write_f64(target.z);
                out.write_i32(*color);
                out.write_var_i32(*duration);
            }
            Self::Shriek { delay } => out.write_var_i32(*delay),
        }
        Ok(ParticlePayload {
            particle_type_id,
            raw_options: out.into_inner(),
        })
    }

    /// Reads the option value `payload` writes for `particle_type_id`.
    pub fn decode(particle_type_id: i32, raw_options: &[u8]) -> Result<Self> {
        Self::decode_with_registry(particle_type_id, raw_options, ContentRegistry::vanilla())
    }

    pub fn decode_with_registry(
        particle_type_id: i32,
        raw_options: &[u8],
        registry: &ContentRegistry,
    ) -> Result<Self> {
        let family = particle_codec_family(particle_type_id, registry)?;
        let mut input = Decoder::new(raw_options);
        let options = match family {
            VanillaParticleCodecFamily::Simple => Self::Simple,
            VanillaParticleCodecFamily::Block => Self::Block {
                state_id: input.read_var_i32()?,
            },
            VanillaParticleCodecFamily::Power => Self::Power {
                power: input.read_f32()?,
            },
            VanillaParticleCodecFamily::Dust => Self::Dust {
                color: input.read_i32()?,
                scale: input.read_f32()?,
            },
            VanillaParticleCodecFamily::DustColorTransition => Self::DustColorTransition {
                from_color: input.read_i32()?,
                to_color: input.read_i32()?,
                scale: input.read_f32()?,
            },
            VanillaParticleCodecFamily::Spell => Self::Spell {
                color: input.read_i32()?,
                power: input.read_f32()?,
            },
            VanillaParticleCodecFamily::Color => Self::Color {
                color: input.read_i32()?,
            },
            VanillaParticleCodecFamily::SculkCharge => Self::SculkCharge {
                roll: input.read_f32()?,
            },
            VanillaParticleCodecFamily::Item => Self::Item(Box::new(ItemStackTemplateSummary {
                item_id: input.read_var_i32()?,
                count: input.read_var_i32()?,
                component_patch: decode_data_component_patch_summary_with_registry(
                    &mut input, registry,
                )?
                .into(),
            })),
            VanillaParticleCodecFamily::Vibration => match input.read_var_i32()? {
                VANILLA_POSITION_SOURCE_TYPE_BLOCK_ID => Self::Vibration {
                    destination: decode_block_pos(input.read_i64()?),
                    arrival_in_ticks: input.read_var_i32()?,
                },
                VANILLA_POSITION_SOURCE_TYPE_ENTITY_ID => Self::VibrationEntity {
                    entity_id: input.read_var_i32()?,
                    y_offset: input.read_f32()?,
                    arrival_in_ticks: input.read_var_i32()?,
                },
                other => {
                    return Err(ProtocolError::InvalidData(format!(
                        "unknown position source type id {other}"
                    )))
                }
            },
            VanillaParticleCodecFamily::Trail => Self::Trail {
                target: Vec3d {
                    x: input.read_f64()?,
                    y: input.read_f64()?,
                    z: input.read_f64()?,
                },
                color: input.read_i32()?,
                duration: input.read_var_i32()?,
            },
            VanillaParticleCodecFamily::Shriek => Self::Shriek {
                delay: input.read_var_i32()?,
            },
        };
        if !input.is_empty() {
            return Err(ProtocolError::InvalidData(format!(
                "particle type id {particle_type_id} options have {} trailing byte(s)",
                input.remaining_len()
            )));
        }
        Ok(options)
    }
}
