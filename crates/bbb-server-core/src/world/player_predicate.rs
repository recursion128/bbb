//! The online-player reads of `PlayerPredicate.matches`
//! (`PlayerPredicate.java:74-137`).

use bbb_protocol::packets::{PlayerInput, Vec3d};
use bbb_server_datapack::loot::{AdvancementTerm, InputPredicate, PlayerTerms};
use bbb_server_types::geometry::{Aabb, Vec3Ops};

use super::pov_clip::{clip_exact, view_vector, ClipFluid, ClipShape};
use super::AdvancementTree;
use super::PlayerState;
use crate::entity::projectile_hit::aabb_clip;
use crate::entity::{Entity, EntityStore};

/// `PlayerPredicate.LOOKING_AT_RANGE` (`PlayerPredicate.java:56`).
const LOOKING_AT_RANGE: f64 = 100.0;

/// The entity `PlayerPredicate.looking_at` tests: the nearest one on the
/// 100-block view ray from the eye (`PlayerPredicate.java:118-127`) that the
/// player has line of sight to (`:130`). Another non-spectator player can be
/// the nearest hit; it blocks the ray but is never answered as the target.
pub(in crate::world) fn player_looking_at<'s>(
    player: &PlayerState,
    store: &'s EntityStore,
    chunks: &crate::chunk::ChunkStore,
) -> Option<&'s Entity> {
    let from = player.eye_position().ok()?;
    let rotation = player.rotation();
    let to = from.plus(view_vector(rotation.x_rot, rotation.y_rot).scale(LOOKING_AT_RANGE));
    let area = Aabb {
        min_x: from.x.min(to.x),
        min_y: from.y.min(to.y),
        min_z: from.z.min(to.z),
        max_x: from.x.max(to.x),
        max_y: from.y.max(to.y),
        max_z: from.z.max(to.z),
    }
    .inflate(1.0, 1.0, 1.0);
    // `None` for the id is another player.
    let mut nearest: Option<(Option<i32>, f64)> = None;
    let mut consider = |entity: Option<i32>, bounds: Aabb| {
        if !bounds.intersects(area) {
            return;
        }
        if let Some(hit) = aabb_clip(bounds, from, to) {
            let distance = from.distance_to_sqr(hit);
            if nearest.is_none_or(|(_, best)| distance < best) {
                nearest = Some((entity, distance));
            }
        }
    };
    for id in store.ids_in_box_sections(area) {
        if id == player.entity_id() {
            continue;
        }
        if let Some(entity) = store
            .get(id)
            .filter(|entity| crate::entity::ender_dragon::state(entity).is_none())
        {
            consider(Some(entity.id()), entity.bounding_box());
        }
    }
    // `Level.getEntities` yields a dragon's parts, which carry its type.
    for dragon in store.dragons() {
        for (_, bounds, _) in crate::entity::ender_dragon::projectile_parts(dragon) {
            consider(Some(dragon.id()), bounds);
        }
    }
    for other in store.players() {
        if other.entity_id() == player.entity_id() || other.is_spectator() || other.avatar_removed {
            continue;
        }
        if let Ok(bounds) = other.bounding_box() {
            consider(None, bounds);
        }
    }
    let entity = store.get(nearest?.0?)?;
    has_line_of_sight(chunks, from, entity).then_some(entity)
}

/// `PlayerPredicate`'s stats, recipes, advancements and input terms
/// (`PlayerPredicate.java:90-135`).
/// `None` for `tree` answers every advancement term false.
pub(in crate::world) fn player_terms_match(
    player: &PlayerState,
    tree: Option<&AdvancementTree>,
    terms: &PlayerTerms,
) -> bool {
    let book = player.inventory.recipe_book();
    terms
        .stats
        .iter()
        .all(|(stat, range)| range.matches(player.cold.stats.value(*stat)))
        && terms
            .recipes
            .iter()
            .all(|(id, present)| book.contains(id) == *present)
        && terms.advancements.iter().all(|(id, term)| {
            tree.is_some_and(|tree| advancement_term_matches(player, tree, id, term))
        })
        && terms
            .input
            .is_none_or(|input| input_matches(input, player.last_client_input))
}

/// `AdvancementDonePredicate` / `AdvancementCriterionsPredicate.test`
/// over `getOrStartProgress` (`PlayerPredicate.java:144-167`).
fn advancement_term_matches(
    player: &PlayerState,
    tree: &AdvancementTree,
    id: &str,
    term: &AdvancementTerm,
) -> bool {
    let Some(index) = tree.index_of(id) else {
        return false;
    };
    let progress = player.cold.advancements.progress(index);
    match term {
        AdvancementTerm::Done(expected) => {
            progress.is_some_and(|progress| progress.is_done()) == *expected
        }
        AdvancementTerm::Criteria(criteria) => {
            criteria.iter().all(|(name, expected)| match progress {
                Some(progress) => progress
                    .criteria()
                    .get(name)
                    .is_some_and(|criterion| criterion.is_done() == *expected),
                None => {
                    !*expected
                        && tree
                            .node(index)
                            .criteria()
                            .iter()
                            .any(|(criterion, _)| name == criterion)
                }
            })
        }
    }
}

/// `LivingEntity.hasLineOfSight(Entity)`: within 128 blocks eye to eye and
/// a `COLLIDER`/`Fluid.NONE` clip that misses.
fn has_line_of_sight(chunks: &crate::chunk::ChunkStore, from: Vec3d, target: &Entity) -> bool {
    let position = target.position();
    let to = Vec3d {
        y: position.y + f64::from(target.eye_height()),
        ..position
    };
    from.distance_to_sqr(to) <= 128.0 * 128.0
        && matches!(
            clip_exact(chunks, from, to, ClipFluid::None, ClipShape::Collider),
            Some(None)
        )
}

/// `InputPredicate.matches` (`InputPredicate.java:30-38`).
fn input_matches(predicate: InputPredicate, input: PlayerInput) -> bool {
    let flag = |term: Option<bool>, value: bool| term.is_none_or(|term| term == value);
    flag(predicate.forward, input.forward)
        && flag(predicate.backward, input.backward)
        && flag(predicate.left, input.left)
        && flag(predicate.right, input.right)
        && flag(predicate.jump, input.jump)
        && flag(predicate.sneak, input.shift)
        && flag(predicate.sprint, input.sprint)
}
