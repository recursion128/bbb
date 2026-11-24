//! Damage-source predicates over actual source values.

use bbb_server_datapack::loot::{DamageSourcePredicate, EntityPredicate};

use super::entity_predicate::entity_predicate_matches;
use super::{unsupported_field, Eval, LootEntity, LootRuntimeReason};

pub(super) fn damage_source_properties(
    eval: &mut Eval<'_>,
    predicate: Option<&DamageSourcePredicate>,
    path: &str,
) -> super::Result<bool> {
    if eval.context.origin().is_none() || !eval.context.damage_source().present() {
        return Ok(false);
    }
    let Some(predicate) = predicate else {
        return Ok(true);
    };
    if predicate.tags.is_empty()
        && predicate.direct_entity.is_none()
        && predicate.source_entity.is_none()
        && predicate.is_direct.is_none()
    {
        return Ok(true);
    }
    let super::LootDamageSource::Present(facts) = eval.context.damage_source() else {
        return Ok(false);
    };
    if !predicate.tags.is_empty() {
        let tags = facts.tags.ok_or_else(|| {
            eval.err(
                path,
                unsupported_field(
                    "damage_source_properties",
                    "predicate.tags has no damage-type tag authority here",
                ),
            )
        })?;
        for (tag, expected) in &predicate.tags {
            let member = tags
                .and_then(|tags| tags.tag(tag))
                .is_some_and(|members| members.iter().any(|member| member == facts.damage_type));
            if member != *expected {
                return Ok(false);
            }
        }
    }
    for (entity_predicate, entity, term) in [
        (&predicate.direct_entity, facts.direct, "direct_entity"),
        (
            &predicate.source_entity,
            Some(facts.causing),
            "source_entity",
        ),
    ] {
        if let Some(entity_predicate) = entity_predicate {
            if !entity_predicate_slot(eval, entity_predicate, entity, path, term)? {
                return Ok(false);
            }
        }
    }
    match predicate.is_direct {
        Some(expected) => facts
            .is_direct
            .map(|actual| actual == expected)
            .ok_or_else(|| {
                eval.err(
                    path,
                    unsupported_field(
                        "damage_source_properties",
                        "predicate.is_direct has no source directness here",
                    ),
                )
            }),
        None => Ok(true),
    }
}

fn entity_predicate_slot(
    eval: &Eval<'_>,
    predicate: &EntityPredicate,
    entity: Option<LootEntity<'_>>,
    path: &str,
    term: &'static str,
) -> super::Result<bool> {
    let entity = entity.ok_or_else(|| {
        eval.err(
            path,
            unsupported_field(
                "damage_source_properties",
                format!("predicate.{term} has no source entity slot here"),
            ),
        )
    })?;
    if !entity.present() {
        return Ok(false);
    }
    eval.at(
        path,
        entity_predicate_matches(predicate, entity, eval.entity_predicate_world()).map_err(
            |reason| match reason {
                LootRuntimeReason::UnsupportedField { detail, .. } => unsupported_field(
                    "damage_source_properties",
                    format!("predicate.{term}.{detail}"),
                ),
                reason => reason,
            },
        ),
    )
}

#[cfg(test)]
mod tests;
