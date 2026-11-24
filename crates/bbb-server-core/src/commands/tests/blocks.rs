//! The block position, block state and block predicate arguments.

use super::*;

/// `BlockPosArgument.listSuggestions` (BlockPosArgument.java:66-80) reaches the
/// same helper `Vec3Argument` does, so it offers the same triple -- but through
/// its own `Commands.createValidator(this::parse)` (`:79`), which is the
/// whole-number reader.
#[test]
fn the_block_position_argument_suggests_a_whole_number_triple() {
    let completions = completions_for(
        "setblock ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["~", "~ ~", "~ ~ ~"]);
    let completions = completions_for(
        "setblock ^",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["^ ^", "^ ^ ^"]);
    // The validator is the whole thing that separates the two arguments.
    // `WorldCoordinate.parseInt` (WorldCoordinate.java:41-59) reads a double
    // after a `~` and an int without one (`:53`), so an absolute decimal
    // stands as a `minecraft:vec3` and is refused as a `minecraft:block_pos`:
    // the candidates it would have carried are dropped here and offered there.
    let completions = completions_for(
        "setblock 0.5",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), [] as [&str; 0]);
    let completions = completions_for(
        "damage alice 1 minecraft:generic at 0.5",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["0.5 ~", "0.5 ~ ~"]);
    // A relative decimal is legal on both, because `parseInt` reads a double
    // once `isRelative` consumed the `~`.
    let completions = completions_for(
        "setblock ~0.5",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["~0.5 ~", "~0.5 ~ ~"]);
}

/// `BlockStateArgument.listSuggestions` (BlockStateArgument.java:40-42) offers
/// the block ids `BlockStateParser.suggestItem` (BlockStateParser.java:344-346)
/// collects, filtered by `filterResources` like every other registry list.
#[test]
fn the_block_state_argument_suggests_the_block_ids_it_would_accept() {
    let completions = completions_for(
        "setblock 0 0 0 waxed_oxidized_c",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(
        texts(&completions),
        [
            "minecraft:waxed_oxidized_chiseled_copper",
            "minecraft:waxed_oxidized_copper",
            "minecraft:waxed_oxidized_copper_bars",
            "minecraft:waxed_oxidized_copper_bulb",
            "minecraft:waxed_oxidized_copper_chain",
            "minecraft:waxed_oxidized_copper_chest",
            "minecraft:waxed_oxidized_copper_door",
            "minecraft:waxed_oxidized_copper_golem_statue",
            "minecraft:waxed_oxidized_copper_grate",
            "minecraft:waxed_oxidized_copper_lantern",
            "minecraft:waxed_oxidized_copper_trapdoor",
            "minecraft:waxed_oxidized_cut_copper",
            "minecraft:waxed_oxidized_cut_copper_slab",
            "minecraft:waxed_oxidized_cut_copper_stairs",
        ]
    );
    // `filterResources` matches on either half while no colon is written, and
    // on the whole id once one is (SharedSuggestionProvider.java:93-107).
    let completions = completions_for(
        "setblock 0 0 0 minecraft:jigs",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), ["minecraft:jigsaw"]);
    // A `#` under a block state never reaches `readTag`'s `expect`
    // (BlockStateParser.java:366-368), so the id list is still the one on
    // offer -- and no id matches a remainder carrying a `#`.
    let completions = completions_for(
        "setblock 0 0 0 #mineable",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(texts(&completions), [] as [&str; 0]);
}

/// Every stage `BlockStateParser.parse` (BlockStateParser.java:143-170) and
/// `readProperties` (`:380-431`) install after the id.
///
/// `fillSuggestions` (`:128-141`) re-bases the builder onto the cursor its own
/// parse stopped at (`:140`), so each answer below carries that cursor as its
/// start rather than the argument's.
#[test]
fn the_block_state_argument_offers_every_stage_after_the_id() {
    // `setblock 0 0 0 ` is fifteen characters, so every start below is that
    // much plus an offset into the argument.
    let ask = |remaining: &str| {
        let completions = completions_for(
            &format!("setblock 0 0 0 {remaining}"),
            CommandPermission::Owner,
            CommandTestWorld::default(),
        );
        (completions.start, completions.length, texts(&completions))
    };

    // `suggestOpenPropertiesOrNbt` (`:326-338`) offers the `[` for a block
    // whose `StateDefinition` is not empty and the `{` for one with a block
    // entity, and nothing at all on a non-empty remainder.
    assert_eq!(ask("chest"), stage(20, 0, &["[", "{"]));
    assert_eq!(ask("oak_log"), stage(22, 0, &["["]));
    assert_eq!(ask("stone"), stage(0, 0, &[]));
    assert_eq!(ask("chest]"), stage(0, 0, &[]));
    // `suggestPropertyNameOrEnd` (`:172-178`) and `suggestPropertyName`
    // (`:188-198`), which spells each name with the `=` that has to follow it
    // and drops the ones the argument already named.
    assert_eq!(ask("oak_log["), stage(23, 0, &["]", "axis="]));
    assert_eq!(ask("oak_log[a"), stage(23, 1, &["axis="]));
    assert_eq!(
        ask("chest[facing=north,"),
        stage(34, 0, &["type=", "waterlogged="])
    );
    assert_eq!(ask("chest[facing=north,t"), stage(34, 1, &["type="]));
    // `suggestEquals` (`:239-245`).
    assert_eq!(ask("oak_log[axis"), stage(27, 0, &["="]));
    // The value list `readProperties` installs at `:408`, which is
    // `addSuggestions` (`:259-269`) over every value the property declares --
    // unfiltered, an `IntegerProperty`'s in numeric order.
    assert_eq!(ask("oak_log[axis="), stage(28, 0, &["x", "y", "z"]));
    assert_eq!(
        ask("chest[facing="),
        stage(28, 0, &["east", "north", "south", "west"])
    );
    assert_eq!(
        ask("redstone_wire[power="),
        stage(
            35,
            0,
            &[
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
                "15"
            ]
        )
    );
    // `suggestNextPropertyOrEnd` (`:247-257`): the `,` only while some property
    // is still unnamed, and both only on an empty remainder.
    assert_eq!(ask("chest[facing=north"), stage(33, 0, &[",", "]"]));
    assert_eq!(
        ask("chest[facing=north,type=left,waterlogged=true"),
        stage(60, 0, &["]"])
    );
    // Whitespace is skipped on both sides of the pair (`:386`, `:400`, `:407`,
    // `:412`), so the stage after one stands where the reader stopped.
    assert_eq!(ask("chest[ facing = north "), stage(37, 0, &[",", "]"]));
    // `suggestOpenNbt` (`:215-221`) once the list closed, and `SUGGEST_NOTHING`
    // (`:167`) once a `{` opened.
    assert_eq!(ask("chest[facing=north]"), stage(34, 0, &["{"]));
    assert_eq!(ask("oak_log[axis=x]"), stage(0, 0, &[]));
    assert_eq!(ask("chest{"), stage(0, 0, &[]));
}

/// `BlockPredicateArgument.listSuggestions` (BlockPredicateArgument.java:59-61)
/// asks `fillSuggestions` for the testing form, whose first stage is
/// `suggestBlockIdOrTag` (BlockStateParser.java:348-351) -- the tag ids *and*
/// the block ids on one builder.
#[test]
fn the_block_predicate_argument_suggests_the_tag_ids_beside_the_block_ids() {
    let filter = |remaining: &str| {
        let completions = completions_for(
            &format!("fill 0 0 0 1 1 1 stone replace {remaining}"),
            CommandPermission::Owner,
            CommandTestWorld::default(),
        );
        texts(&completions)
    };

    // A `#` reaches the tag half alone, matched with the `#` stripped off
    // (`filterResources`, SharedSuggestionProvider.java:109-121).
    assert_eq!(filter("#wooden_d"), ["#minecraft:wooden_doors"]);
    assert_eq!(
        filter("#mineable"),
        [
            "#minecraft:mineable/axe",
            "#minecraft:mineable/hoe",
            "#minecraft:mineable/pickaxe",
            "#minecraft:mineable/shovel",
        ]
    );
    // `matchesSubStr` (SharedSuggestionProvider.java:271-283) restarts at every
    // `_`, so a tag whose path only ends in the pattern is offered as well.
    // `#logs` itself would not reach this stage at all: it is a tag that
    // resolves, so `readTag` leaves the cursor past it.
    assert_eq!(
        filter("#log"),
        [
            "#minecraft:acacia_logs",
            "#minecraft:birch_logs",
            "#minecraft:cherry_logs",
            "#minecraft:dark_oak_logs",
            "#minecraft:jungle_logs",
            "#minecraft:logs",
            "#minecraft:logs_that_burn",
            "#minecraft:mangrove_logs",
            "#minecraft:mangrove_logs_can_grow_through",
            "#minecraft:oak_logs",
            "#minecraft:overworld_natural_logs",
            "#minecraft:pale_oak_logs",
            "#minecraft:spruce_logs",
        ]
    );
    // A namespace written out matches the whole id rather than either half.
    assert_eq!(
        filter("#minecraft:log"),
        ["#minecraft:logs", "#minecraft:logs_that_burn"]
    );

    // Without a `#` the tag half shares no common prefix with `#` and is
    // dropped whole (`:115-119`), leaving the same block ids a block state
    // offers for the same remainder.
    let state = completions_for(
        "setblock 0 0 0 waxed_oxidized_c",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(filter("waxed_oxidized_c"), texts(&state));
    assert!(!filter("waxed_oxidized_c").is_empty());

    // `Identifier.read` (Identifier.java:189-199) answers `minecraft:` for an
    // empty run, which no tag holds, so a bare `#` rewinds onto itself and the
    // whole tag list is what is left to offer.
    let hash = filter("#");
    assert_eq!(hash.len(), 248);
    assert!(hash.iter().all(|text| text.starts_with('#')));

    // An empty remainder offers every tag and every block id (`:112-113`).
    let everything = filter("");
    let (tags, blocks): (Vec<String>, Vec<String>) = everything
        .into_iter()
        .partition(|text| text.starts_with('#'));
    assert_eq!(tags.len(), 248);
    assert!(tags.contains(&"#minecraft:mineable/pickaxe".to_string()));
    let states = completions_for(
        "setblock 0 0 0 ",
        CommandPermission::Owner,
        CommandTestWorld::default(),
    );
    assert_eq!(blocks, texts(&states));

    // Every tag the enumeration offers is one the argument's own reader
    // resolves, which is what keeps the two halves of `BlockPredicateArgument`
    // -- the completion and the parse -- naming the same set.
    for tag in &tags {
        read_block_predicate(
            &(tag).encode_utf16().collect::<Vec<_>>(),
            0,
            crate::registry_tags::RegistryTagSnapshot::builtin(),
        )
        .unwrap_or_else(|_| panic!("{tag} must parse"));
    }
}

/// The stages after a `#tag`, which `readVagueProperties`
/// (BlockStateParser.java:433-484) installs instead: no definition to resolve a
/// name or a value against, so every member of the tag answers for itself.
#[test]
fn the_block_predicate_argument_offers_every_stage_after_the_id() {
    // `fill 0 0 0 1 1 1 stone replace ` is thirty-one characters.
    let ask = |remaining: &str| {
        let completions = completions_for(
            &format!("fill 0 0 0 1 1 1 stone replace {remaining}"),
            CommandPermission::Owner,
            CommandTestWorld::default(),
        );
        (completions.start, completions.length, texts(&completions))
    };

    // A block reaches the same stages a block state does (`:158-163`).
    assert_eq!(
        ask("chest["),
        stage(37, 0, &["]", "facing=", "type=", "waterlogged="])
    );
    // `suggestOpenVaguePropertiesOrNbt` (`:300-324`) asks the whole tag: the
    // `[` while any member declares a property and the `{` while any member has
    // a block entity.
    assert_eq!(ask("#minecraft:logs"), stage(46, 0, &["["]));
    assert_eq!(ask("#minecraft:beds"), stage(46, 0, &["[", "{"]));
    // `suggestVaguePropertyNameOrEnd` (`:180-186`) over `suggestVaguePropertyName`
    // (`:200-213`), which collects every member's own definition.
    assert_eq!(ask("#minecraft:logs["), stage(47, 0, &["]", "axis="]));
    assert_eq!(
        ask("#minecraft:beds["),
        stage(47, 0, &["]", "facing=", "occupied=", "part="])
    );
    // A missing `=` rewinds to the key here (`:449-452`) where the block branch
    // reports where the reader stopped and offers the `=` itself.
    assert_eq!(ask("#minecraft:logs[axis"), stage(47, 4, &["axis="]));
    // `suggestVaguePropertyValue` (`:271-298`) stays installed through the value
    // and the separator after it, so it offers the `]` itself -- and the `,`
    // only while some member still has a property the argument has not named.
    assert_eq!(
        ask("#minecraft:logs[axis="),
        stage(52, 0, &["]", "x", "y", "z"])
    );
    assert_eq!(
        ask("#minecraft:logs[axis=x"),
        stage(52, 1, &["]", "y", "z"])
    );
    assert_eq!(
        ask("#minecraft:beds[facing=north"),
        stage(54, 5, &[",", "]", "east", "south", "west"])
    );
    assert_eq!(
        ask("#minecraft:cauldrons[level="),
        stage(58, 0, &["1", "2", "3", "]"])
    );
    // An `IntegerSuggestion` (`addSuggestions`, `:261-262`) has no
    // equal-to-the-remainder test (SuggestionsBuilder.java:72-75), so the `1`
    // is offered back where the `north` above was dropped.
    assert_eq!(
        ask("#minecraft:cauldrons[level=1"),
        stage(58, 1, &["1", "2", "3", "]"])
    );
    // `suggestOpenNbt` (`:215-221`) reads the tag's members too (`:228-234`).
    assert_eq!(ask("#minecraft:logs[axis=x]"), stage(0, 0, &[]));
    assert_eq!(ask("#minecraft:beds[facing=north]"), stage(60, 0, &["{"]));
    assert_eq!(ask("#minecraft:beds{"), stage(0, 0, &[]));
}

/// The answer one stage is expected to carry: its start, the length of the
/// range it rewrites, and the candidate texts in the order `Suggestions.create`
/// sorts them (Suggestions.java:100-101).
fn stage(start: i32, length: i32, texts: &[&str]) -> (i32, i32, Vec<String>) {
    (
        start,
        length,
        texts.iter().map(|text| (*text).to_string()).collect(),
    )
}

/// `WorldCoordinates.parseInt` (WorldCoordinates.java:39-57) through
/// `BlockPosArgument.parse` (BlockPosArgument.java:61-63).
#[test]
fn a_block_position_reads_whole_numbers_and_never_centres_them() {
    let (end, value) =
        read_block_pos(&("0 -60 3").encode_utf16().collect::<Vec<_>>(), 0).expect("three axes");
    assert_eq!(end, "0 -60 3".len());
    // `/setblock 0 -60 3` names the block at the origin corner: unlike
    // `minecraft:vec3`, `WorldCoordinate.parseInt` (WorldCoordinate.java:41-59)
    // has no `centerCorrect` branch to add the half.
    assert_eq!(
        block_pos_of(&value, [0.0, 0.0, 0.0], [0.0, 0.0]),
        BlockPos { x: 0, y: -60, z: 3 }
    );
    // `:52-56`: a `~` with no number reads none and stands for the source axis,
    // and the whole coordinate is floored afterwards.
    let (_, relative) = read_block_pos(&("~ ~1 ~-2").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("three relative axes");
    assert_eq!(
        block_pos_of(&relative, [10.7, 64.0, -3.2], [0.0, 0.0]),
        BlockPos {
            x: 10,
            y: 65,
            z: -6
        }
    );
    // `:53`: only the relative form reads a double, so a fractional absolute
    // axis reaches `Integer.parseInt` and is refused there.
    assert_eq!(
        reader_failure(read_block_pos(
            &("0.5 0 0").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("parsing.int.invalid")
    );
    let (_, fractional) = read_block_pos(&("~0.5 ~ ~").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("a relative double");
    assert_eq!(
        block_pos_of(&fractional, [0.0, 0.0, 0.0], [0.0, 0.0]),
        BlockPos { x: 0, y: 0, z: 0 }
    );
    // `:46-48` names the int key, not the double one `Vec3Argument` reports.
    assert_eq!(
        reader_failure(read_block_pos(
            &("0 0 ").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos.missing.int")
    );
    // `WorldCoordinates.parseInt` rewinds to the argument before
    // `ERROR_NOT_COMPLETE` (WorldCoordinates.java:50, `:54`).
    assert_eq!(
        reader_failure(read_block_pos(
            &("0 0").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos3d.incomplete")
    );
    // `WorldCoordinate.parseInt` (WorldCoordinate.java:42-44): one axis may not
    // switch form.
    assert_eq!(
        reader_failure(read_block_pos(
            &("0 ^ 0").encode_utf16().collect::<Vec<_>>(),
            0
        ))
        .as_deref(),
        Some("argument.pos.mixed")
    );
    // The local form is `LocalCoordinates.parse`, shared with `minecraft:vec3`.
    let (end, local) =
        read_block_pos(&("^ ^ ^1").encode_utf16().collect::<Vec<_>>(), 0).expect("a local offset");
    assert_eq!(end, "^ ^ ^1".len());
    assert!(matches!(
        local,
        ArgumentValue::BlockPos(super::parse::Coordinates::Local { .. })
    ));
}

fn block_pos_of(value: &ArgumentValue, position: [f64; 3], rotation: [f32; 2]) -> BlockPos {
    let ArgumentValue::BlockPos(coordinates) = value else {
        panic!("a block position argument reads as coordinates");
    };
    let [x, y, z] = coordinates.position(&CommandOrigin {
        position,
        rotation,
        ..crate::world::ServerWorld::console_origin()
    });
    BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    }
}

/// `BlockStateParser.readBlock` (BlockStateParser.java:354-363) and
/// `readProperties` (`:380-431`).
#[test]
fn a_block_state_starts_at_the_default_state_and_writes_the_properties_it_names() {
    let (end, value) = read_block_state(&("minecraft:stone").encode_utf16().collect::<Vec<_>>(), 0)
        .expect("a block");
    assert_eq!(end, "minecraft:stone".len());
    let ArgumentValue::BlockState(input) = value else {
        panic!("a block-state argument reads as a block input");
    };
    // `readBlock` (`:362`) starts at `block.defaultBlockState()`, and nothing
    // was written on top of it.
    assert!(input.defined_properties.is_empty());
    assert_eq!(
        block_state_name(input.state).as_deref(),
        Some("minecraft:stone")
    );

    let (_, value) = read_block_state(
        &("oak_leaves[distance=1,persistent=true]")
            .encode_utf16()
            .collect::<Vec<_>>(),
        0,
    )
    .expect("two properties");
    let ArgumentValue::BlockState(input) = value else {
        panic!("a block-state argument reads as a block input");
    };
    // `setValue` (`:490-499`) writes each pair into the state as it reads it,
    // and `parseForBlock` (`:98`) keeps the key set beside it -- which is what
    // `BlockInput.overwriteWithDefinedProperties` (BlockInput.java:103-113)
    // later puts back.
    assert_eq!(input.defined_properties, ["distance", "persistent"]);
    assert_eq!(
        block_state_property(input.state, "distance").as_deref(),
        Some("1")
    );
    assert_eq!(
        block_state_property(input.state, "persistent").as_deref(),
        Some("true")
    );
}

/// The five refusals `BlockStateParser` raises for a block state, each at the
/// position vanilla's reader stands at when it throws.
#[test]
fn a_block_state_reports_the_refusal_its_own_parser_raises() {
    for (command, key, cursor) in [
        // `readBlock` rewinds to the start of the name
        // (BlockStateParser.java:358-359).
        ("minecraft:not_a_block", "argument.block.id.invalid", 0),
        // `:390-393`, at the start of the key.
        ("stone[nope=1]", "argument.block.property.unknown", 6),
        // `:395-398`, at the start of the repeated key.
        (
            "oak_leaves[distance=1,distance=2]",
            "argument.block.property.duplicate",
            22,
        ),
        // `setValue` (`:496-497`) rewinds to the start of the value.
        (
            "oak_leaves[distance=9]",
            "argument.block.property.invalid",
            20,
        ),
        // `:402-404` reports where the reader stopped, not at the key.
        (
            "oak_leaves[distance]",
            "argument.block.property.novalue",
            19,
        ),
        // `:426-430`, at the end of the input.
        (
            "oak_leaves[distance=1",
            "argument.block.property.unclosed",
            21,
        ),
        // `:413-417`, at the stray character.
        (
            "oak_leaves[distance=1 persistent=true]",
            "argument.block.property.unclosed",
            22,
        ),
        // `readTag` (`:366-368`) refuses a tag before it reads one, with the
        // reader still on the `#`.
        ("#minecraft:logs", "argument.block.tag.disallowed", 0),
    ] {
        let failure =
            read_block_state(&(command).encode_utf16().collect::<Vec<_>>(), 0).expect_err(command);
        assert_eq!(
            translate_key(&failure.message).as_deref(),
            Some(key),
            "{command:?}"
        );
        assert_eq!(failure.cursor, Some(cursor), "{command:?}");
    }
}

/// `BlockPredicateArgument.parse` (BlockPredicateArgument.java:46-52): the same
/// parser with `forTesting` set, which is the one thing that lets a `#tag`
/// through.
#[test]
fn a_block_predicate_reads_the_tag_a_block_state_refuses() {
    let (end, value) = read_block_predicate(
        &("#minecraft:logs").encode_utf16().collect::<Vec<_>>(),
        0,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
    )
    .expect("a tag");
    assert_eq!(end, "#minecraft:logs".len());
    assert_eq!(
        value,
        ArgumentValue::BlockPredicate(super::parse::CommandBlockPredicate::Tag {
            id: "minecraft:logs".to_string(),
            vague_properties: std::collections::BTreeMap::new(),
            block_entity_data: None,
        })
    );
    // `readVagueProperties` (BlockStateParser.java:433-484) resolves neither
    // the name nor the
    // value, because a tag has no `StateDefinition` to resolve them against --
    // `TagPredicate.test` (BlockPredicateArgument.java:126-140) does that per
    // candidate state instead.
    let (_, value) = read_block_predicate(
        &("#minecraft:logs[axis=diagonal]")
            .encode_utf16()
            .collect::<Vec<_>>(),
        0,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
    )
    .expect("a vague property");
    assert_eq!(
        value,
        ArgumentValue::BlockPredicate(super::parse::CommandBlockPredicate::Tag {
            id: "minecraft:logs".to_string(),
            vague_properties: std::collections::BTreeMap::from([(
                "axis".into(),
                "diagonal".into()
            )]),
            block_entity_data: None,
        })
    );
    // `orElseThrow` (BlockStateParser.java:374-377) rewinds to the `#`.
    let failure = read_block_predicate(
        &("#minecraft:not_a_tag").encode_utf16().collect::<Vec<_>>(),
        0,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
    )
    .expect_err("an unknown tag");
    assert_eq!(
        translate_key(&failure.message).as_deref(),
        // The plural prefix is the tag refusal's own
        // (BlockStateParser.java:58-60); every other
        // key in that class is singular.
        Some("arguments.block.tag.unknown")
    );
    assert_eq!(failure.cursor, Some(0));
    // The block branch still resolves its properties exactly, because that is
    // `parseForTesting`'s left arm (BlockStateParser.java:119-121).
    let (_, value) = read_block_predicate(
        &("stone").encode_utf16().collect::<Vec<_>>(),
        0,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
    )
    .expect("a block");
    assert_eq!(
        value,
        ArgumentValue::BlockPredicate(super::parse::CommandBlockPredicate::Block(
            CommandBlockInput {
                state: read_block_state_id("stone"),
                defined_properties: Vec::new(),
                block_entity_data: None,
            }
        ))
    );
}

/// `readNbt` (BlockStateParser.java:486-488) at `:166-169` is the last leg of
/// both readers, and what it reads is `BlockInput.tag`: the compound travels
/// beside the state the argument resolved to.
#[test]
fn a_block_state_reads_the_block_entity_compound_the_placement_carries() {
    let compound = |entries: &[(&str, NbtSummaryValue)]| {
        Some(NbtSummaryValue::Compound(
            entries
                .iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: (*name).into(),
                    value: value.clone(),
                })
                .collect(),
        ))
    };
    for (command, properties, data) in [
        (
            "setblock 0 64 0 chest{lock:'k'}",
            Vec::new(),
            compound(&[("lock", NbtSummaryValue::string("k"))]),
        ),
        // Both halves at once: the compound is read after the property list
        // (`:160-163` then `:166-169`).
        (
            "setblock 0 64 0 chest[facing=north]{Lock:'k'}",
            vec!["facing"],
            compound(&[("Lock", NbtSummaryValue::string("k"))]),
        ),
        (
            "setblock 0 64 0 chest{Items:[{Slot:0b}]}",
            Vec::new(),
            compound(&[(
                "Items",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "Slot".into(),
                    value: NbtSummaryValue::Byte(0),
                }])]),
            )]),
        ),
    ] {
        let run = dispatched(
            command,
            CommandTestWorld::default().with_block(BlockPos { x: 0, y: 64, z: 0 }, 0),
        );
        assert_eq!(keys(&run), ["commands.setblock.success"], "{command}");
        let (_, value) = read_block_state(
            &(command).encode_utf16().collect::<Vec<_>>(),
            "setblock 0 64 0 ".len(),
        )
        .expect("a block");
        let ArgumentValue::BlockState(block) = value else {
            panic!("{command}: a block-state argument reads as a block input");
        };
        assert_eq!(block.defined_properties, properties, "{command}");
        assert_eq!(block.block_entity_data, data, "{command}");
    }
}

fn read_block_state_id(command: &str) -> i32 {
    let (_, value) =
        read_block_state(&(command).encode_utf16().collect::<Vec<_>>(), 0).expect("a block");
    let ArgumentValue::BlockState(input) = value else {
        panic!("a block-state argument reads as a block input");
    };
    input.state
}

fn block_state_name(state: i32) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .map(|identity| identity.name.clone())
}

fn block_state_property(state: i32, property: &str) -> Option<String> {
    bbb_protocol::block_state_types::vanilla_block_state_identity(state)
        .and_then(|identity| identity.properties.get(property).cloned())
}

fn two_blocks(first: &str, second: &str) -> CommandTestWorld {
    CommandTestWorld::default()
        .with_block(BlockPos { x: 0, y: 63, z: 0 }, read_block_state_id("dirt"))
        .with_block(BlockPos { x: 1, y: 63, z: 0 }, read_block_state_id("dirt"))
        .with_block(BlockPos { x: 0, y: 64, z: 0 }, read_block_state_id(first))
        .with_block(BlockPos { x: 1, y: 64, z: 0 }, read_block_state_id(second))
}

#[test]
fn tag_filters_resolve_integer_spellings_through_the_candidate_property() {
    for (filter, admitted) in [
        ("stage=00", true),
        ("stage=7", false),
        ("stage=-1", false),
        ("stage=0x0", false),
        ("stage=", false),
    ] {
        let mut fixture = two_blocks("oak_sapling[stage=0]", "oak_sapling[stage=1]");
        let run = ran(
            &format!("fill 0 64 0 1 64 0 stone replace #minecraft:saplings[{filter}]"),
            &mut fixture,
        );
        assert_eq!(run.result, admitted.then_some(1), "{filter}");
        assert_eq!(
            keys(&run),
            [if admitted {
                "commands.fill.success"
            } else {
                "commands.fill.failed"
            }],
            "{filter}"
        );
    }
    let mut fixture = two_blocks("oak_sapling[stage=0]", "oak_sapling[stage=1]");
    let run = ran(
        "clone 0 64 0 1 64 0 8 64 0 filtered #minecraft:saplings[stage=+1]",
        &mut fixture,
    );
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.clone.success"]);
    assert_eq!(
        fixture.execute_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x: 9, y: 64, z: 0 }
        ),
        Some(read_block_state_id("oak_sapling[stage=1]"))
    );
}

#[test]
fn tag_filter_enum_and_boolean_spellings_remain_exact() {
    for (first, second, filter, admitted) in [
        (
            "oak_log[axis=x]",
            "oak_log[axis=y]",
            "#minecraft:logs[axis=x]",
            true,
        ),
        (
            "oak_log[axis=x]",
            "oak_log[axis=y]",
            "#minecraft:logs[axis=X]",
            false,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[lit=true]",
            true,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[lit=TRUE]",
            false,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[waterlogged=true]",
            true,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[candles=01]",
            true,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[candles=+1]",
            true,
        ),
        (
            "candle[candles=1,lit=true,waterlogged=false]",
            "candle[candles=2,lit=false,waterlogged=true]",
            "#minecraft:candles[candles=5]",
            false,
        ),
    ] {
        let run = ran(
            &format!("fill 0 64 0 1 64 0 stone replace {filter}"),
            two_blocks(first, second),
        );
        assert_eq!(run.result, admitted.then_some(1), "{filter}");
        assert_eq!(
            keys(&run),
            [if admitted {
                "commands.fill.success"
            } else {
                "commands.fill.failed"
            }],
            "{filter}"
        );
    }
}

#[test]
fn clone_refuses_overlap_limit_unloaded_regions_and_debug_destination() {
    for (command, fixture, key) in [
        (
            "clone 0 64 0 2 64 2 1 64 1",
            two_blocks("stone", "stone"),
            "commands.clone.overlap",
        ),
        (
            "clone 0 64 0 1 64 0 8 64 0",
            two_blocks("stone", "stone").with_rules(&[(
                "minecraft:max_block_modifications",
                GameRuleValue::Integer(1),
            )]),
            "commands.clone.toobig",
        ),
        (
            "clone 0 64 0 32 64 0 64 64 0",
            two_blocks("stone", "stone"),
            "argument.pos.unloaded",
        ),
        (
            "clone from minecraft:overworld 0 64 0 0 64 0 to minecraft:the_nether 8 64 0",
            two_blocks("stone", "stone")
                .with_level(level_key(Java26BuiltinDimension::Nether))
                .with_block(BlockPos { x: 8, y: 64, z: 0 }, 0)
                .with_debug_level(level_key(Java26BuiltinDimension::Nether)),
            "commands.clone.failed",
        ),
    ] {
        let run = ran(command, fixture);
        assert_eq!(run.result, None, "{command}");
        assert_eq!(keys(&run), [key], "{command}");
    }
}

#[test]
fn clone_refuses_an_empty_mask_and_a_moderator_source() {
    let run = ran(
        "clone 0 64 0 0 64 0 8 64 0 masked",
        two_blocks("air", "air"),
    );
    assert_eq!(run.result, None);
    assert_eq!(keys(&run), ["commands.clone.failed"]);
    assert_eq!(
        keys(&refused(
            "clone 0 64 0 0 64 0 8 64 0",
            CommandPermission::Moderator
        )),
        ["command.unknown.command", CURSOR_CONTEXT]
    );
}
