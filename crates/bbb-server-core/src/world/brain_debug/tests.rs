use bbb_protocol::nbt_summary::value_field as field;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue as Nbt};
use uuid::Uuid;

use super::*;

fn entry(name: &str, value: Nbt) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn global_pos_value(pos: [i32; 3]) -> Nbt {
    Nbt::Compound(vec![
        entry("dimension", Nbt::String("minecraft:overworld".into())),
        entry("pos", Nbt::IntArray(pos.to_vec())),
    ])
}

fn memory(module: &str, value: Nbt) -> NbtSummaryEntry {
    entry(module, Nbt::Compound(vec![entry("value", value)]))
}

fn uuid_value(uuid: Uuid) -> Nbt {
    let (most, least) = uuid.as_u64_pair();
    Nbt::IntArray(vec![
        (most >> 32) as u32 as i32,
        most as u32 as i32,
        (least >> 32) as u32 as i32,
        least as u32 as i32,
    ])
}

fn staged_uuid(ordinal: u64) -> Uuid {
    Uuid::from_u64_pair(0x6262_6200_0000_0000, ordinal)
}

/// The employed villager `mob_brain_gate` stages: a farmer with a job site, a
/// meeting point, five xp, three bread and one gossip.
fn employed_villager_record() -> Nbt {
    Nbt::Compound(vec![
        entry("Age", Nbt::Int(0)),
        entry(
            "Brain",
            Nbt::Compound(vec![entry(
                "memories",
                Nbt::Compound(vec![
                    memory("minecraft:job_site", global_pos_value([5, 64, 5])),
                    memory("minecraft:meeting_point", global_pos_value([-5, 64, 5])),
                ]),
            )]),
        ),
        entry(
            "VillagerData",
            Nbt::Compound(vec![
                entry("type", Nbt::String("minecraft:plains".into())),
                entry("profession", Nbt::String("minecraft:farmer".into())),
                entry("level", Nbt::Int(2)),
            ]),
        ),
        entry("Xp", Nbt::Int(5)),
        entry(
            "Inventory",
            Nbt::List(vec![Nbt::Compound(vec![
                entry("id", Nbt::String("minecraft:bread".into())),
                entry("count", Nbt::Int(3)),
            ])]),
        ),
        entry(
            "Gossips",
            Nbt::List(vec![Nbt::Compound(vec![
                entry("Target", uuid_value(staged_uuid(9))),
                entry("Type", Nbt::String("minor_positive".into())),
                entry("Value", Nbt::Int(12)),
            ])]),
        ),
    ])
}

fn empty_record() -> Nbt {
    Nbt::Compound(vec![entry("Age", Nbt::Int(0))])
}

fn villager_shape() -> BrainShape {
    brain_shape(VANILLA_ENTITY_TYPE_VILLAGER_ID, false).unwrap()
}

fn packed(record: &Nbt) -> BrainPacked {
    field(record, "Brain").map_or_else(BrainPacked::empty, |brain| {
        BrainPacked::decode(brain).unwrap()
    })
}

#[test]
fn a_villager_names_every_module_its_brain_registers_once_and_sorted() {
    let lines = memory_lines(
        &villager_shape(),
        &packed(&employed_villager_record()),
        6000,
    );
    assert_eq!(
        lines
            .iter()
            .map(|line| line.split_once(": ").unwrap().0)
            .collect::<Vec<_>>(),
        [
            "breed_target",
            "cant_reach_walk_target_since",
            "doors_to_close",
            "golem_detected_recently",
            "heard_bell_time",
            "hiding_place",
            "home",
            "hurt_by",
            "hurt_by_entity",
            "interaction_target",
            "item_pickup_cooldown_ticks",
            "job_site",
            "last_slept",
            "last_woken",
            "look_target",
            "meeting_point",
            "mobs",
            "nearest_bed",
            "nearest_hostile",
            "nearest_players",
            "nearest_visible_player",
            "nearest_visible_targetable_player",
            "nearest_visible_targetable_players",
            "nearest_visible_wanted_item",
            "path",
            "potential_job_site",
            "secondary_job_site",
            "visible_mobs",
            "visible_villager_babies",
            "walk_target",
        ]
    );
}

#[test]
fn a_frog_names_its_own_roster_and_no_villager_module() {
    let shape = brain_shape(VANILLA_ENTITY_TYPE_FROG_ID, false).unwrap();
    let lines = memory_lines(&shape, &BrainPacked::empty(), 6000);
    assert_eq!(
        lines,
        [
            "attack_target: -",
            "breed_target: -",
            "cant_reach_walk_target_since: -",
            "hurt_by: -",
            "hurt_by_entity: -",
            "is_in_water: -",
            "is_panicking: -",
            "is_pregnant: -",
            "is_tempted: -",
            "long_jump_cooling_down: -",
            "long_jump_mid_jump: -",
            "look_target: -",
            "mobs: -",
            "nearest_attackable: -",
            "path: -",
            "temptation_cooldown_ticks: -",
            "tempting_player: -",
            "unreachable_tongue_targets: -",
            "visible_mobs: -",
            "walk_target: -",
        ]
    );
}

#[test]
fn a_saved_memory_the_type_never_registers_is_not_a_line() {
    let record = Nbt::Compound(vec![entry(
        "Brain",
        Nbt::Compound(vec![entry(
            "memories",
            Nbt::Compound(vec![memory(
                "minecraft:liked_noteblock",
                global_pos_value([1, 2, 3]),
            )]),
        )]),
    )]);
    let lines = memory_lines(&villager_shape(), &packed(&record), 0);
    assert_eq!(lines.len(), VILLAGER_MEMORY_MODULES.len());
    assert!(lines.iter().all(|line| line.ends_with(": -")), "{lines:?}");
}

#[test]
fn a_villager_takes_the_scheduled_activity_only_where_its_memory_backs_it() {
    let shape = villager_shape();
    let employed = packed(&employed_villager_record());
    assert_eq!(
        active_activities(&shape, &employed, Some("minecraft:work")),
        ["core", "work"]
    );
    assert_eq!(
        active_activities(&shape, &BrainPacked::empty(), Some("minecraft:work")),
        ["core", "idle"]
    );
    assert_eq!(
        active_activities(&shape, &employed, Some("minecraft:rest")),
        ["core", "rest"]
    );
    // `Activity.PLAY` is registered for a baby alone (`Villager.java:143-154`).
    assert_eq!(
        active_activities(&shape, &employed, Some("minecraft:play")),
        ["core", "idle"]
    );
    assert_eq!(
        active_activities(
            &brain_shape(VANILLA_ENTITY_TYPE_VILLAGER_ID, true).unwrap(),
            &employed,
            Some("minecraft:play")
        ),
        ["core", "play"]
    );
    assert_eq!(active_activities(&shape, &employed, None), ["core", "idle"]);
}

#[test]
fn a_frog_keeps_the_activity_its_brain_was_built_with() {
    let shape = brain_shape(VANILLA_ENTITY_TYPE_FROG_ID, false).unwrap();
    assert_eq!(
        active_activities(&shape, &BrainPacked::empty(), Some("minecraft:work")),
        ["core", "idle"]
    );
}

#[test]
fn the_poi_sets_are_the_three_memories_that_back_them() {
    let shape = villager_shape();
    let record = employed_villager_record();
    assert_eq!(
        known_block_positions(
            &shape,
            None,
            &packed(&record),
            &[
                MemoryModuleType::JobSite,
                MemoryModuleType::Home,
                MemoryModuleType::MeetingPoint
            ],
        ),
        [
            BlockPos { x: -5, y: 64, z: 5 },
            BlockPos { x: 5, y: 64, z: 5 },
        ]
    );
    assert!(known_block_positions(
        &shape,
        None,
        &packed(&record),
        &[MemoryModuleType::PotentialJobSite],
    )
    .is_empty());
    let mut live = Brain::new();
    live.register_memory(MemoryModuleType::JobSite);
    live.register_memory(MemoryModuleType::MeetingPoint);
    live.set_memory(
        MemoryModuleType::JobSite,
        Some(MemoryValue::GlobalPos(crate::entity::ai::MemoryGlobalPos {
            dimension: "minecraft:overworld".to_owned(),
            pos: BlockPos { x: 8, y: 70, z: 9 },
        })),
    );
    assert_eq!(
        known_block_positions(
            &shape,
            Some(&live),
            &packed(&record),
            &[MemoryModuleType::JobSite, MemoryModuleType::MeetingPoint],
        ),
        [BlockPos { x: 8, y: 70, z: 9 }],
    );
}

#[test]
fn a_creaking_names_its_own_roster_and_its_three_activities() {
    let shape = brain_shape(VANILLA_ENTITY_TYPE_CREAKING_ID, false).unwrap();
    let lines = memory_lines(&shape, &BrainPacked::empty(), 6000);
    assert_eq!(
        lines,
        [
            "attack_cooling_down: -",
            "attack_target: -",
            "cant_reach_walk_target_since: -",
            "look_target: -",
            "mobs: -",
            "nearest_players: -",
            "nearest_visible_player: -",
            "nearest_visible_targetable_player: -",
            "nearest_visible_targetable_players: -",
            "path: -",
            "visible_mobs: -",
            "walk_target: -",
        ]
    );
    // `CreakingAi.getActivities` (`CreakingAi.java:75-77`) carries `CORE`,
    // `IDLE` and `FIGHT`, whose condition is the `ATTACK_TARGET`
    // `ActivityData.create` was handed (`:57-67`). `CreakingAi.updateActivity`
    // is no schedule, so the projection answers the activity the brain was
    // built with whatever the record holds -- the record a live creaking
    // publishes is the other half of this (`tests::simulation::creaking_freeze`).
    let fighting = Nbt::Compound(vec![entry(
        "Brain",
        Nbt::Compound(vec![entry(
            "memories",
            Nbt::Compound(vec![memory(
                "minecraft:attack_target",
                uuid_value(staged_uuid(3)),
            )]),
        )]),
    )]);
    for record in [&empty_record(), &fighting] {
        assert_eq!(
            active_activities(&shape, &packed(record), None),
            ["core", "idle"]
        );
    }
}

#[test]
fn live_memory_lines_keep_registered_slots_and_the_line_budget() {
    use crate::entity::ai::MemoryModuleType;

    let mut brain = Brain::new();
    for memory_type in [
        MemoryModuleType::InteractionTarget,
        MemoryModuleType::NearestLivingEntities,
        MemoryModuleType::HurtByEntity,
        MemoryModuleType::WalkTarget,
    ] {
        brain.register_memory(memory_type);
    }
    brain.set_memory(
        MemoryModuleType::InteractionTarget,
        Some(MemoryValue::EntityRef(7)),
    );
    brain.set_memory(
        MemoryModuleType::HurtByEntity,
        Some(MemoryValue::EntityRef(8)),
    );
    brain.set_memory(
        MemoryModuleType::NearestLivingEntities,
        Some(MemoryValue::EntityList((0..1000).collect())),
    );
    let lines = live_memory_lines(&brain, 6000);
    assert_eq!(lines.len(), 4);
    assert!(lines
        .iter()
        .any(|line| line.starts_with("interaction_target:") && line.contains('7')));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("hurt_by_entity:") && line.contains('8')));
    assert!(lines.iter().any(|line| line == "walk_target: -"));
    assert!(lines
        .iter()
        .all(|line| line.encode_utf16().count() <= MEMORY_LINE_LIMIT));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("mobs:") && line.ends_with("...")));
    let unicode = "🧠".repeat(200);
    let line = memory_line(format_args!("{unicode}"));
    assert_eq!(line.encode_utf16().count(), MEMORY_LINE_LIMIT);
    assert!(line.ends_with("..."));
}
