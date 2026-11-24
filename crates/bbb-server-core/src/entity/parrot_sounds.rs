//! Live and shoulder-resident parrots share ambient and imitation sounds.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID;
use bbb_protocol::packets::SoundSource;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::store::EntityBodyReadView;
use super::{Entity, EntityStore, PendingEntitySound};
use crate::entity_runtime_facts_26_1::VANILLA_PARROT_IMITATION_SOUNDS;
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

fn imitation_sound(entity_type: i32) -> Option<&'static str> {
    VANILLA_PARROT_IMITATION_SOUNDS
        .binary_search_by_key(&entity_type, |entry| entry.0)
        .ok()
        .map(|index| VANILLA_PARROT_IMITATION_SOUNDS[index].1)
}

// `Parrot.getAmbient` (`Parrot.java:331-338`).
pub(crate) fn ambient(random: &GameRandom, difficulty: Difficulty) -> &'static str {
    if difficulty != Difficulty::Peaceful && random.next_int(1000) == 0 {
        VANILLA_PARROT_IMITATION_SOUNDS
            [random.next_int(VANILLA_PARROT_IMITATION_SOUNDS.len() as i32) as usize]
            .1
    } else {
        "minecraft:entity.parrot.ambient"
    }
}

fn sound(
    source: &Entity,
    resource: &'static str,
    category: SoundSource,
    volume: f32,
    random: &GameRandom,
) -> PendingEntitySound {
    PendingEntitySound {
        position: source.position(),
        resource_id: resource.to_owned(),
        source: category,
        volume,
        // `Parrot.getPitch` (`Parrot.java:375-377`).
        pitch: (random.next_float() - random.next_float()) * 0.2 + 1.0,
    }
}

// `Parrot.imitateNearbyMobs` (`Parrot.java:232-247`).
fn imitate(
    source: &Entity,
    may_imitate: bool,
    bodies: EntityBodyReadView<'_>,
    category: SoundSource,
    random: &GameRandom,
) -> Option<PendingEntitySound> {
    if !may_imitate || random.next_int(2) != 0 {
        return None;
    }
    let area = source.bounding_box().inflate(20.0, 20.0, 20.0);
    let mut candidates = bodies.ids_in_box_sections(area);
    candidates.retain(|id| {
        bodies.get(*id).is_some_and(|entity| {
            entity.bounding_box().intersects(area)
                && imitation_sound(entity.entity_type_id()).is_some()
        })
    });
    if candidates.is_empty() {
        return None;
    }
    let selected = candidates[random.next_int(candidates.len() as i32) as usize];
    let selected = bodies.get(selected)?;
    if selected.is_silent() {
        return None;
    }
    Some(sound(
        source,
        imitation_sound(selected.entity_type_id())?,
        category,
        0.7,
        random,
    ))
}

pub(crate) fn tick(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if entity.entity_type_id() != VANILLA_ENTITY_TYPE_PARROT_ID {
        return;
    }
    let alive = entity.is_alive_with_health();
    let ambient_due = store
        .mob_ai_mut(entity.id())
        .is_some_and(|runtime| runtime.tick_mob_ambient_sound(alive));
    let random = level.level_random();
    if ambient_due && !entity.is_silent() {
        store.play_entity_sound(sound(
            entity,
            ambient(random, level.difficulty()),
            SoundSource::Neutral,
            1.0,
            random,
        ));
    }
    // `Parrot.aiStep` (`Parrot.java:196-198`).
    if alive && random.next_int(400) == 0 {
        let bodies = EntityBodyReadView::new(store, Some(entity), level.detached_bodies());
        let sound = imitate(
            entity,
            !entity.is_silent(),
            bodies,
            SoundSource::Neutral,
            random,
        );
        if let Some(sound) = sound {
            store.play_entity_sound(sound);
        }
    }
}

pub(crate) fn shoulder_sound(
    entries: &[NbtSummaryEntry],
    player: &Entity,
    may_imitate: bool,
    bodies: EntityBodyReadView<'_>,
    random: &GameRandom,
    difficulty: Difficulty,
) -> Option<PendingEntitySound> {
    // `ServerPlayer.playShoulderEntityAmbientSound` (`ServerPlayer.java:766-784`).
    if entries.is_empty()
        || super::save::optional_bool(entries, "Silent").unwrap_or(false)
        || random.next_int(200) != 0
    {
        return None;
    }
    let id = super::save::optional_field(entries, "id")?;
    if !matches!(id, NbtSummaryValue::String(id) if id == "minecraft:parrot" || id == "parrot") {
        return None;
    }
    Some(
        imitate(player, may_imitate, bodies, SoundSource::Players, random).unwrap_or_else(|| {
            sound(
                player,
                ambient(random, difficulty),
                SoundSource::Players,
                1.0,
                random,
            )
        }),
    )
}
