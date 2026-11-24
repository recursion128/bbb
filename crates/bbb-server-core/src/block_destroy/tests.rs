use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{
        vanilla_block_type_for_resource_id, VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_DIRT_ID,
        VANILLA_BLOCK_STONE_ID,
    },
    item_types::vanilla_item_type_for_resource_id,
    packets::{ItemStackSummary, ToolRuleSummary, ToolSummary},
};

use super::*;

fn state(resource_id: &str) -> i32 {
    let block = vanilla_block_type_for_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is a vanilla block"));
    vanilla_block_default_state_id(block.id).expect("every block has a default state")
}

fn default_state(block_type_id: i32) -> i32 {
    vanilla_block_default_state_id(block_type_id).expect("every block has a default state")
}

fn held(resource_id: &str) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            vanilla_item_type_for_resource_id(resource_id)
                .unwrap_or_else(|| panic!("{resource_id} is a vanilla item"))
                .id,
        ),
        count: 1,
        ..ItemStackSummary::empty()
    }
}

/// `BlockBehaviour.getDestroyProgress` (`BlockBehaviour.java:355-363`) is
/// `player.getDestroySpeed(state) / state.getDestroySpeed(..) / modifier`, and
/// every one of the three factors is read out of a generated table here.
///
/// The hardnesses are the `strength(..)` first arguments in `Blocks.java`:
/// stone `1.5F` (`:54-57`), dirt `0.5F` (`:85`), obsidian `50.0F`
/// (`:1266-1272`), oak leaves `0.2F` through `leavesProperties`
/// (`:6958-6971`, `:554`). The tool speeds are the `minecraft:tool` prototypes:
/// a diamond pickaxe declares `speed 8.0` for `#minecraft:mineable/pickaxe`, a
/// wooden one `2.0`, a golden one `12.0`, and shears `15.0` for
/// `#minecraft:leaves`. The divisor is `30` when
/// `Player.hasCorrectToolForDrops` (`Player.java:618-620`) answers true and
/// `100` otherwise (`BlockBehaviour.java:361`).
#[test]
fn a_destroy_progress_is_the_tool_speed_over_the_hardness_over_the_tool_modifier() {
    let progress = |item: &ItemStackSummary, block: &str| {
        block_destroy_progress(item, state(block), facts(true), tags()).unwrap()
    };
    let bare = ItemStackSummary::empty();

    // Stone requires the correct tool (`Blocks.java:56`), so a bare hand takes
    // the 100 divisor and a pickaxe the 30 one.
    assert_eq!(progress(&bare, "minecraft:stone"), 1.0 / 1.5 / 100.0);
    assert_eq!(
        progress(&held("minecraft:wooden_pickaxe"), "minecraft:stone"),
        2.0 / 1.5 / 30.0
    );
    assert_eq!(
        progress(&held("minecraft:diamond_pickaxe"), "minecraft:stone"),
        8.0 / 1.5 / 30.0
    );

    // Dirt declares no `requiresCorrectToolForDrops()` (`Blocks.java:85`), so
    // `hasCorrectToolForDrops` is true whatever the hand holds and the divisor
    // is 30 for a bare hand too.
    assert_eq!(progress(&bare, "minecraft:dirt"), 1.0 / 0.5 / 30.0);

    // The two halves of one tool can be answered by two different rules. A
    // golden pickaxe lists `#minecraft:incorrect_for_gold_tool` first with only
    // `correct_for_drops: false`, then `#minecraft:mineable/pickaxe` with
    // `speed 12.0` and `correct_for_drops: true`; obsidian is in both, so
    // `Tool.getMiningSpeed` (`Tool.java:39-47`) skips the first rule for having
    // no speed and takes 12.0, while `Tool.isCorrectForDrops` (`:49-57`) stops
    // at the first rule and answers false.
    assert_eq!(
        progress(&held("minecraft:golden_pickaxe"), "minecraft:obsidian"),
        12.0 / 50.0 / 100.0
    );
    // A diamond pickaxe is in no `incorrect_for_*` tag that holds obsidian, so
    // the same block takes the 30 divisor.
    assert_eq!(
        progress(&held("minecraft:diamond_pickaxe"), "minecraft:obsidian"),
        8.0 / 50.0 / 30.0
    );

    // Shears declare a speed for `#minecraft:leaves` and no `correct_for_drops`
    // anywhere that covers leaves, so the rule search finds a speed and no
    // flag; leaves need no correct tool either way, so the divisor is 30. The
    // quotient is 2.5, which is what makes `:201-202` insta-mine leaves with
    // shears -- that consequence is asserted where it happens, in
    // `crate::tests::a_tool_fast_enough_breaks_the_block_on_the_swing_that_starts_it`,
    // rather than restated here as a weaker second reading of this same number.
    assert_eq!(
        progress(&held("minecraft:shears"), "minecraft:oak_leaves"),
        15.0 / 0.2 / 30.0
    );

    // A tool whose rules are all silent about the block gets both fallbacks:
    // `Tool.getMiningSpeed` (`:46`) drops to `defaultMiningSpeed`, which is the
    // codec default `1.0F` (`Tool.java:21`) for shears, and
    // `Tool.isCorrectForDrops` (`:56`) answers false, which stone -- a block
    // that does require the correct tool -- turns into the 100 divisor. Shears
    // list cobweb, `#minecraft:leaves`, `#minecraft:wool` and the two vines,
    // and stone is in none of them.
    assert_eq!(
        progress(&held("minecraft:shears"), "minecraft:stone"),
        1.0 / 1.5 / 100.0
    );
}

/// The `-1.0F` early return (`BlockBehaviour.java:356-359`), which is the whole
/// reason bedrock cannot be mined rather than merely being slow.
#[test]
fn an_unbreakable_block_never_reaches_the_division() {
    let bedrock = default_state(VANILLA_BLOCK_BEDROCK_ID);
    assert_eq!(
        vanilla_block_state_destroy_speed(bedrock),
        Some(-1.0),
        "`Blocks.java:275-283` is `strength(-1.0F, 3600000.0F)`"
    );
    for item in [
        ItemStackSummary::empty(),
        held("minecraft:diamond_pickaxe"),
        held("minecraft:netherite_pickaxe"),
    ] {
        assert_eq!(
            block_destroy_progress(&item, bedrock, facts(true), tags()).unwrap(),
            0.0
        );
    }
}

/// `Player.getDestroySpeed`'s last modifier (`Player.java:611-613`).
#[test]
fn mining_off_the_ground_costs_four_fifths_of_the_progress() {
    let pickaxe = held("minecraft:diamond_pickaxe");
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let grounded = block_destroy_progress(&pickaxe, stone, facts(true), tags()).unwrap();
    let airborne = block_destroy_progress(&pickaxe, stone, facts(false), tags()).unwrap();
    assert_eq!(airborne, grounded / 5.0);
}

/// `ItemStack.get(DataComponents.TOOL)` composes the prototype with the patch,
/// so a patch decides on its own once it names the component.
#[test]
fn a_component_patch_replaces_the_prototype_tool_it_names() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let bare_hand =
        block_destroy_progress(&ItemStackSummary::empty(), stone, facts(true), tags()).unwrap();

    // Removing the component leaves the pickaxe mining like a bare hand.
    let mut disarmed = held("minecraft:diamond_pickaxe");
    disarmed
        .component_patch
        .removed_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    assert_eq!(
        block_destroy_progress(&disarmed, stone, facts(true), tags()).unwrap(),
        bare_hand
    );

    // Adding one turns a stick into a pickaxe. The rule names the tag, which is
    // the `ByteBufCodecs.holderSet` form `ToolRuleSummary` keeps apart from the
    // inline id list.
    let mut sharpened = held("minecraft:stick");
    sharpened
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID);
    sharpened.component_patch.tool = Some(ToolSummary {
        rules: vec![ToolRuleSummary {
            block_registry_ids: Vec::new(),
            block_tag: Some("minecraft:mineable/pickaxe".to_string()),
            speed_bits: Some(8.0_f32.to_bits()),
            correct_for_drops: Some(true),
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    assert_eq!(
        block_destroy_progress(&sharpened, stone, facts(true), tags()).unwrap(),
        8.0 / 1.5 / 30.0
    );
    // The inline-id form of the same rule reaches the same answer, and a block
    // the list does not name falls back to `defaultMiningSpeed`.
    let mut listed = sharpened.clone();
    listed.component_patch.tool = Some(ToolSummary {
        rules: vec![ToolRuleSummary {
            block_registry_ids: vec![VANILLA_BLOCK_STONE_ID],
            block_tag: None,
            speed_bits: Some(8.0_f32.to_bits()),
            correct_for_drops: Some(true),
        }],
        // A `defaultMiningSpeed` that is not the codec default, so the block the
        // list does not name shows which of the two the search fell back to.
        default_mining_speed_bits: 3.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    assert_eq!(
        block_destroy_progress(&listed, stone, facts(true), tags()).unwrap(),
        8.0 / 1.5 / 30.0
    );
    assert_eq!(
        block_destroy_progress(
            &listed,
            default_state(VANILLA_BLOCK_DIRT_ID),
            facts(true),
            tags()
        )
        .unwrap(),
        3.0 / 0.5 / 30.0
    );
}

/// `ServerPlayerGameMode.tick`'s active arm (`ServerPlayerGameMode.java:120-128`)
/// and `incrementDestroyProgress` (`:132-142`): the stage climbs with the ticks
/// spent and is broadcast only when it moves.
#[test]
fn the_active_arm_broadcasts_a_stage_only_when_it_changes() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let pickaxe = held("minecraft:diamond_pickaxe");
    let mut progress = DestroyProgress::default();
    progress.begin_progress_window();
    // One tick's worth of progress is 8/1.5/30 = 0.1777…, so the stage is
    // `(int)(0.1777… * 10) = 1` on the first tick and moves every tick after.
    let step = block_destroy_progress(&pickaxe, stone, facts(true), tags()).unwrap();
    assert_eq!(
        progress.start(BlockPos { x: 1, y: 2, z: 3 }, step),
        (None, 1)
    );

    let mut stages = Vec::new();
    for _ in 0..6 {
        match progress
            .tick(Some(stone), &pickaxe, facts(true), tags())
            .unwrap()
        {
            DestroyTickStep::Progress { stage, .. } => stages.push(stage),
            other => panic!("expected progress, got {other:?}"),
        }
    }
    // Ticks 1..6 give (n+1) * 0.1777…, so the stages are 3, 5, 7, 8, 10, 12 and
    // every one of them differs from the last.
    assert_eq!(
        stages,
        vec![Some(3), Some(5), Some(7), Some(8), Some(10), Some(12)]
    );

    // A hand slow enough to leave the stage where it is says nothing: a bare
    // hand on stone earns 1/1.5/100 = 0.00666… a tick, so the first ten ticks
    // all round to stage 0.
    let mut slow = DestroyProgress::default();
    slow.begin_progress_window();
    assert_eq!(
        slow.start(
            BlockPos { x: 1, y: 2, z: 3 },
            block_destroy_progress(&ItemStackSummary::empty(), stone, facts(true), tags()).unwrap()
        ),
        (None, 0)
    );
    for _ in 0..10 {
        assert_eq!(
            slow.tick(Some(stone), &ItemStackSummary::empty(), facts(true), tags())
                .unwrap(),
            DestroyTickStep::Progress {
                pos: BlockPos { x: 1, y: 2, z: 3 },
                stage: None,
                destroy: false,
            }
        );
    }
}

/// `ServerPlayerGameMode.tick`'s `blockState.isAir()` arm (`:122-125`): the
/// session stops destroying and the stage it broadcast is cleared.
#[test]
fn an_active_break_whose_block_vanished_clears_its_stage_once() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let pickaxe = held("minecraft:diamond_pickaxe");
    let pos = BlockPos { x: 4, y: 5, z: 6 };
    let mut progress = DestroyProgress::default();
    progress.begin_progress_window();
    progress.start(
        pos,
        block_destroy_progress(&pickaxe, stone, facts(true), tags()).unwrap(),
    );
    assert_eq!(progress.tick_target(), Some(pos));

    assert_eq!(
        progress.tick(None, &pickaxe, facts(true), tags()).unwrap(),
        DestroyTickStep::Cleared { pos }
    );
    assert_eq!(progress.tick_target(), None);
    assert_eq!(
        progress.tick(None, &pickaxe, facts(true), tags()).unwrap(),
        DestroyTickStep::Idle
    );
}

/// The STOP branch's 0.7 criterion (`ServerPlayerGameMode.java:218-234`):
/// `getDestroyProgress * (ticksSpentDestroying + 1)`.
///
/// A bare hand on stone earns 1/1.5/100 = 0.006666667 a tick, so it needs
/// `ceil(0.7 / 0.006666667) - 1 = 104` ticks between START and STOP; one tick
/// short is the delayed branch instead.
#[test]
fn the_stop_criterion_is_seven_tenths_of_what_the_ticks_earned() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let bare = ItemStackSummary::empty();
    let per_tick = block_destroy_progress(&bare, stone, facts(true), tags()).unwrap();
    let needed = (STOP_DESTROY_PROGRESS_THRESHOLD / per_tick).ceil() as i32 - 1;
    assert_eq!(needed, 104);

    let stop_after = |ticks: i32| {
        let mut progress = DestroyProgress::default();
        progress.begin_progress_window();
        progress.start(BlockPos { x: 0, y: 0, z: 0 }, per_tick);
        for _ in 0..ticks {
            progress
                .tick(Some(stone), &bare, facts(true), tags())
                .unwrap();
        }
        let mut progress_for_stop = progress.clone();
        (
            progress_for_stop
                .stop(stone, &bare, facts(true), tags())
                .unwrap(),
            progress_for_stop,
        )
    };

    assert_eq!(stop_after(needed).0, StopOutcome::Destroy);
    let (outcome, mut delayed) = stop_after(needed - 1);
    assert_eq!(outcome, StopOutcome::Delayed);

    // The delayed arm (`:109-119`) finishes the break on the first tick whose
    // `gameTicks` makes `getDestroyProgress * (gameTicks + 1)` reach 1.0, which
    // is `ceil(1.0 / per_tick) - 1 = 149`; the 103 ticks the interrupted break
    // already spent count towards it, because `delayedTickStart` is the same
    // `destroyProgressStart` (`:233`).
    let ticks_to_full = (1.0 / per_tick).ceil() as i32 - 1;
    assert_eq!(ticks_to_full, 149);
    for _ in 0..(ticks_to_full - (needed - 1) - 1) {
        assert!(matches!(
            delayed
                .tick(Some(stone), &bare, facts(true), tags())
                .unwrap(),
            DestroyTickStep::Progress { destroy: false, .. }
        ));
    }
    assert!(matches!(
        delayed
            .tick(Some(stone), &bare, facts(true), tags())
            .unwrap(),
        DestroyTickStep::Progress { destroy: true, .. }
    ));
    // `:116` clears the flag, so the next tick has nothing to look at.
    assert_eq!(delayed.tick_target(), None);
}

/// `handleBlockBreakAction`'s STOP branch keeps a delayed break rather than
/// replacing it (`:229`), and ABORT never looks at one (`:239-249`).
#[test]
fn a_delayed_break_survives_a_second_stop_and_an_abort() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let bare = ItemStackSummary::empty();
    let first = BlockPos { x: 1, y: 1, z: 1 };
    let second = BlockPos { x: 2, y: 1, z: 1 };
    let mut progress = DestroyProgress::default();
    progress.begin_progress_window();
    progress.start(
        first,
        block_destroy_progress(&bare, stone, facts(true), tags()).unwrap(),
    );
    assert_eq!(
        progress.stop(stone, &bare, facts(true), tags()).unwrap(),
        StopOutcome::Delayed
    );
    assert_eq!(progress.tick_target(), Some(first));

    // A second break that stops early cannot take the delayed slot.
    progress.begin_progress_window();
    progress.start(
        second,
        block_destroy_progress(&bare, stone, facts(true), tags()).unwrap(),
    );
    assert_eq!(
        progress.stop(stone, &bare, facts(true), tags()).unwrap(),
        StopOutcome::Kept
    );
    assert_eq!(progress.tick_target(), Some(first));

    // An abort of the second position clears two stages -- the one the session
    // was on and the one the packet named -- and leaves the delayed break.
    progress.begin_progress_window();
    progress.start(
        second,
        block_destroy_progress(&bare, stone, facts(true), tags()).unwrap(),
    );
    assert_eq!(
        progress.abort(first),
        AbortOutcome {
            mismatched: Some(second),
            pos: first,
        }
    );
    assert_eq!(progress.tick_target(), Some(first));
    // A matched abort clears one.
    progress.begin_progress_window();
    progress.start(
        second,
        block_destroy_progress(&bare, stone, facts(true), tags()).unwrap(),
    );
    assert_eq!(
        progress.abort(second),
        AbortOutcome {
            mismatched: None,
            pos: second,
        }
    );
}

/// The else arm's interrupted-break answer (`ServerPlayerGameMode.java:204-207`):
/// starting a second break while one is live names the first position so the
/// caller can put its block back on the client.
#[test]
fn starting_a_second_break_names_the_one_it_interrupted() {
    let stone = default_state(VANILLA_BLOCK_STONE_ID);
    let bare = ItemStackSummary::empty();
    let first = BlockPos { x: 7, y: 8, z: 9 };
    let second = BlockPos { x: 7, y: 8, z: 10 };
    let step = block_destroy_progress(&bare, stone, facts(true), tags()).unwrap();
    let mut progress = DestroyProgress::default();
    assert_eq!(progress.start(first, step), (None, 0));
    assert_eq!(progress.start(second, step), (Some(first), 0));
    assert_eq!(progress.destroy_pos(), second);
}

/// A session with the player's base attributes and no effects.
fn facts(on_ground: bool) -> PlayerDestroySpeedFacts {
    let base = |id| bbb_server_types::interaction::player_attribute_value(id).unwrap();
    PlayerDestroySpeedFacts {
        mining_efficiency: base(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MINING_EFFICIENCY_ID,
        ),
        block_break_speed: base(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BLOCK_BREAK_SPEED_ID,
        ),
        submerged_mining_speed: base(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SUBMERGED_MINING_SPEED_ID,
        ),
        dig_speed_amplification: None,
        mining_fatigue_amplifier: None,
        eye_in_water: false,
        on_ground,
    }
}

fn tags() -> &'static crate::registry_tags::RegistryTagSnapshot {
    crate::registry_tags::RegistryTagSnapshot::builtin()
}
