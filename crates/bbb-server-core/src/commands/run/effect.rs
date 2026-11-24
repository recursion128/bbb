//! `/effect` (`EffectCommands.java:33-239`).

use crate::entity::living::mob_effects::{is_instantaneous, ActiveMobEffect};
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_protocol::NbtSummaryValue;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{translatable, CommandCx, CommandSource};

use super::{
    integer_argument, resource_argument, select_damage_entity_states, source_entity, CommandRun,
    Outcome,
};
use crate::commands::types::CommandDamageEntityRef;

/// `EffectCommands.giveEffect` (`EffectCommands.java:139-191`).
pub(super) fn give_effect(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    seconds: Option<i32>,
) -> Outcome {
    let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
        unreachable!("targets is an entity argument");
    };
    let (targets, success, static_id, duration, amplifier, particles) =
        cx.world.with_entity_reads(|reads| {
            let targets = select_damage_entity_states(selector, source, &cx.origin, reads)?;
            if targets.is_empty() {
                return Err(translatable("argument.entity.notfound.entity", Vec::new()));
            }
            let effect_id = resource_argument(parse, "effect");
            let static_id = vanilla_mob_effect_type_for_resource_id(effect_id)
                .expect("the resource argument only resolves registry ids")
                .resource_id;
            let amplifier = parse
                .argument("amplifier")
                .map(|_| integer_argument(parse, "amplifier"))
                .unwrap_or(0);
            let particles = parse
                .argument("hideParticles")
                .map(|_| !boolean_argument(parse, "hideParticles"))
                .unwrap_or(true);
            // InstantenousMobEffect.java:9-11, MobEffects.java:46-47, :88; seconds
            // rules are EffectCommands.java:150-162.
            let duration = give_duration(seconds, is_instantaneous(static_id));
            if ActiveMobEffect::give(static_id, duration, amplifier, particles).is_none() {
                return Err(translatable("commands.effect.give.failed", Vec::new()));
            }
            let display = effect_display_name(static_id);
            let seconds_arg = NbtSummaryValue::Int(duration / 20);
            let success = if targets.len() == 1 {
                translatable(
                    "commands.effect.give.success.single",
                    vec![display, targets[0].display_name(reads), seconds_arg],
                )
            } else {
                translatable(
                    "commands.effect.give.success.multiple",
                    vec![display, target_count(&targets), seconds_arg],
                )
            };
            let targets = targets
                .iter()
                .map(|target| target.attribute_target())
                .collect::<Vec<_>>();
            Ok::<_, NbtSummaryValue>((targets, success, static_id, duration, amplifier, particles))
        })?;
    let mut count = 0;
    for target in &targets {
        let effect = ActiveMobEffect::give(static_id, duration, amplifier, particles)
            .expect("the effect was validated above");
        count += i32::from(
            cx.world
                .give_command_effect(source, target, effect, cx.events),
        );
    }
    if count == 0 {
        return Err(translatable("commands.effect.give.failed", Vec::new()));
    }
    run.succeed(success, true);
    Ok(count)
}

/// `EffectCommands.clearEffects` / `clearEffect` (`EffectCommands.java:193-238`).
pub(super) fn clear_effects(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    self_target: bool,
) -> Outcome {
    let (targets, success, static_id) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let targets = if self_target {
            vec![source_entity(source, reads)?]
        } else {
            let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
                unreachable!("targets is an entity argument");
            };
            let found = select_damage_entity_states(selector, source, at, reads)?;
            if found.is_empty() {
                return Err(translatable("argument.entity.notfound.entity", Vec::new()));
            }
            found
        };
        let effect_id = parse
            .argument("effect")
            .map(|_| resource_argument(parse, "effect"));
        let static_id = effect_id.map(|id| {
            vanilla_mob_effect_type_for_resource_id(id)
                .expect("the resource argument only resolves registry ids")
                .resource_id
        });

        let success = match static_id {
            Some(id) if targets.len() == 1 => translatable(
                "commands.effect.clear.specific.success.single",
                vec![effect_display_name(id), targets[0].display_name(reads)],
            ),
            Some(id) => translatable(
                "commands.effect.clear.specific.success.multiple",
                vec![effect_display_name(id), target_count(&targets)],
            ),
            None if targets.len() == 1 => translatable(
                "commands.effect.clear.everything.success.single",
                vec![targets[0].display_name(reads)],
            ),
            None => translatable(
                "commands.effect.clear.everything.success.multiple",
                vec![target_count(&targets)],
            ),
        };
        let targets = targets
            .iter()
            .map(|target| target.attribute_target())
            .collect::<Vec<_>>();
        Ok::<_, NbtSummaryValue>((targets, success, static_id))
    })?;
    let mut count = 0;
    for target in &targets {
        count += i32::from(cx.world.clear_command_effect(target, static_id, cx.events));
    }
    if count == 0 {
        return Err(translatable(
            if static_id.is_some() {
                "commands.effect.clear.specific.failed"
            } else {
                "commands.effect.clear.everything.failed"
            },
            Vec::new(),
        ));
    }
    run.succeed(success, true);
    Ok(count)
}

fn target_count(targets: &[CommandDamageEntityRef<'_>]) -> NbtSummaryValue {
    NbtSummaryValue::Int(i32::try_from(targets.len()).unwrap_or(i32::MAX))
}

/// `EffectCommands.java:150-162`.
fn give_duration(seconds: Option<i32>, instantaneous: bool) -> i32 {
    match seconds {
        Some(seconds) if instantaneous => seconds,
        Some(-1) => -1,
        Some(seconds) => seconds * 20,
        None if instantaneous => 1,
        None => 600,
    }
}

/// `MobEffect.getDisplayName` (`MobEffect.java:124-126`) through
/// `Util.makeDescriptionId("effect", key)` (`:112-116`).
fn effect_display_name(resource_id: &str) -> NbtSummaryValue {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    translatable(format!("effect.{namespace}.{path}"), Vec::new())
}

fn boolean_argument(parse: &ParseResult, name: &str) -> bool {
    match parse.argument(name) {
        Some(ArgumentValue::Boolean(value)) => *value,
        other => unreachable!("{name} is a boolean argument, not {other:?}"),
    }
}
