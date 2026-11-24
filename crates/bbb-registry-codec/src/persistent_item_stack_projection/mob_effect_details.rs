//! Shared persistent MobEffectInstance.Details projection.

use crate::registry_codec::RegistryElementValueRef;
use bbb_protocol::packets::MobEffectDetailsSummary;

use super::encoding::{ComponentEncoder, ComponentScalar as Scalar};
use super::error::{invalid_persistent_component, Result};

pub(super) fn project_details(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
    max_hidden_depth: usize,
) -> Result<MobEffectDetailsSummary> {
    project_details_at_depth(type_id, resource_id, value, 0, max_hidden_depth)
}

fn project_details_at_depth(
    type_id: i32,
    resource_id: &'static str,
    value: RegistryElementValueRef<'_>,
    depth: usize,
    max_hidden_depth: usize,
) -> Result<MobEffectDetailsSummary> {
    let invalid = |detail| invalid_persistent_component(type_id, resource_id, detail);
    if depth > max_hidden_depth {
        return Err(invalid(
            "hidden effect depth exceeds codec limit".to_string(),
        ));
    }
    let int = |name: &str, default: i32| -> Result<i32> {
        value
            .field(name)
            .map(|value| {
                value
                    .codec_i32()
                    .ok_or_else(|| invalid(format!("{name} is not numeric")))
            })
            .transpose()
            .map(|value| value.unwrap_or(default))
    };
    let flag = |name: &str, default: bool| -> Result<bool> {
        value
            .field(name)
            .map(|value| {
                value
                    .as_bool()
                    .ok_or_else(|| invalid(format!("{name} is not boolean")))
            })
            .transpose()
            .map(|value| value.unwrap_or(default))
    };
    let particles = flag("show_particles", true)?;
    let hidden_effect = value
        .field("hidden_effect")
        .map(|value| {
            if !value.is_compound() {
                return Err(invalid("hidden_effect is not a compound".to_string()));
            }
            project_details_at_depth(type_id, resource_id, value, depth + 1, max_hidden_depth)
                .map(Box::new)
        })
        .transpose()?;
    Ok(MobEffectDetailsSummary {
        amplifier: i32::from((int("amplifier", 0)? as i8) as u8),
        duration: int("duration", 0)?,
        ambient: flag("ambient", false)?,
        show_particles: particles,
        show_icon: flag("show_icon", particles)?,
        hidden_effect,
    })
}

#[allow(clippy::too_many_arguments, reason = "one field per codec field")]
pub(super) fn append_details<E: ComponentEncoder>(
    encoder: &E,
    fields: &mut E::Fields,
    amplifier: i32,
    duration: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
    hidden: Option<&MobEffectDetailsSummary>,
    depth: usize,
    max_hidden_depth: usize,
) -> std::result::Result<(), E::Error> {
    encoder.check(
        depth <= max_hidden_depth,
        "hidden effect depth exceeds codec limit",
    )?;
    encoder.check(
        (0..=255).contains(&amplifier),
        "effect amplifier outside 0..=255",
    )?;
    if amplifier != 0 {
        encoder.scalar(fields, "amplifier", Scalar::Byte(amplifier as i8));
    }
    if duration != 0 {
        encoder.scalar(fields, "duration", Scalar::Int(duration));
    }
    if ambient {
        encoder.scalar(fields, "ambient", Scalar::Boolean(true));
    }
    if !show_particles {
        encoder.scalar(fields, "show_particles", Scalar::Boolean(false));
    }
    encoder.scalar(fields, "show_icon", Scalar::Boolean(show_icon));
    if let Some(hidden) = hidden {
        let mut nested = encoder.fields(0);
        append_details(
            encoder,
            &mut nested,
            hidden.amplifier,
            hidden.duration,
            hidden.ambient,
            hidden.show_particles,
            hidden.show_icon,
            hidden.hidden_effect.as_deref(),
            depth + 1,
            max_hidden_depth,
        )?;
        encoder.field(fields, "hidden_effect", encoder.record(nested));
    }
    Ok(())
}
