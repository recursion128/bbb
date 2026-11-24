//! The loot terms that read `ATTACKING_ENTITY` or `LAST_DAMAGE_PLAYER`. An
//! attacker the slot does not carry is enchantment level 0, not a refusal.

use bbb_protocol::packets::ItemStackSummary;
use bbb_registry_codec::level_based_value::LevelBasedValue;

use super::{stack_count, unsupported_field, Eval, LootContextParam, LootEntity, NumberProvider};

/// A presence test: a slot holding an entity the caller cannot show passes.
pub(super) fn killed_by_player(context: &super::LootContext<'_>) -> bool {
    context.entity(LootContextParam::LastDamagePlayer).present()
}

/// `EnchantedCountIncreaseFunction.run`: level 0 returns before the count
/// provider is evaluated.
pub(super) fn enchanted_count_increase(
    eval: &mut Eval<'_>,
    enchantment: &str,
    count: &NumberProvider,
    limit: i32,
    mut stack: ItemStackSummary,
    path: &str,
) -> super::Result<ItemStackSummary> {
    let level = attacker_enchantment_level(eval, enchantment, path)?;
    if level == 0 {
        return Ok(stack);
    }
    let addition = level as f32 * eval.number_float(count, &format!("{path}.count"))?;
    stack.count = stack_count(&stack) + super::java_round(addition);
    if limit > 0 && stack_count(&stack) > 0 && stack.count > limit {
        stack.count = limit;
    }
    Ok(stack)
}

/// The draw is spent whatever the level is.
pub(super) fn random_chance_with_enchanted_bonus(
    eval: &mut Eval<'_>,
    unenchanted_chance: f32,
    enchanted_chance: &LevelBasedValue,
    enchantment: &str,
    path: &str,
) -> super::Result<bool> {
    let level = attacker_enchantment_level(eval, enchantment, path)?;
    let chance = if level > 0 {
        enchanted_chance.calculate(level)
    } else {
        unenchanted_chance
    };
    Ok(eval.random.next_float() < chance)
}

/// An `Opaque` attacker is one the caller has but cannot show, so its level
/// is refused rather than guessed.
fn attacker_enchantment_level(
    eval: &Eval<'_>,
    enchantment: &str,
    path: &str,
) -> super::Result<i32> {
    match eval.context.entity(LootContextParam::AttackingEntity) {
        LootEntity::Absent => Ok(0),
        LootEntity::Opaque => Err(eval.err(
            path,
            unsupported_field(
                "attacker enchantment level",
                "the attacking entity is carried without a view of it",
            ),
        )),
        entity => entity.enchantment_level(enchantment).ok_or_else(|| {
            eval.err(
                path,
                unsupported_field(
                    "attacker enchantment level",
                    format!("no view of the attacking entity resolves {enchantment}"),
                ),
            )
        }),
    }
}

#[cfg(test)]
mod tests;
