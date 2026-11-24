use super::*;
use crate::commands::selector_advancements::AdvancementExpectation;

fn read(selector: &str) -> Result<SelectorOptions, SelectorOptionFailure> {
    let letter = selector.chars().nth(1).expect("a selector has a letter");
    read_options(
        &(selector).encode_utf16().collect::<Vec<_>>(),
        SelectorKind::from_letter(letter).expect("the fixtures use published letters"),
    )
}

fn options(selector: &str) -> SelectorOptions {
    read(selector).expect("the fixture is not a refusal")
}

/// The shared prefix parser accepting the whole selector, which is what
/// decides where a selector ends once this reader has walked it.
fn valid_selector(selector: &str) -> bool {
    bbb_protocol::parse_command_entity_selector_prefix(selector)
        .is_some_and(|prefix| prefix.consumed == selector.len())
}

fn refusal_key(selector: &str) -> Option<(String, usize)> {
    let failure = read(selector).err()?;
    let NbtSummaryValue::Compound(entries) = &failure.message else {
        panic!("a refusal is a component");
    };
    let key = entries
        .iter()
        .find(|entry| entry.name == "translate")
        .map(|entry| match &entry.value {
            NbtSummaryValue::String(key) => key.try_to_string().unwrap(),
            other => panic!("a translation key is a string, not {other:?}"),
        })?;
    Some((
        key,
        failure.offset.expect("a selector refusal is positioned"),
    ))
}

fn done(id: &str, expected: bool) -> AdvancementRequirement {
    AdvancementRequirement {
        id: id.to_string(),
        expectation: AdvancementExpectation::Done(expected),
    }
}

fn criteria(id: &str, entries: &[(&str, bool)]) -> AdvancementRequirement {
    AdvancementRequirement {
        id: id.to_string(),
        expectation: AdvancementExpectation::Criteria(
            entries
                .iter()
                .map(|(name, expected)| ((*name).into(), *expected))
                .collect(),
        ),
    }
}

fn bounds<T>(min: Option<T>, max: Option<T>) -> SelectorBounds<T> {
    SelectorBounds { min, max }
}

#[test]
fn a_selector_without_options_reads_as_none_of_them() {
    assert_eq!(options("@e"), SelectorOptions::default());
    assert_eq!(options("@e[]"), SelectorOptions::default());
    assert_eq!(options("@e run say [hi]"), SelectorOptions::default());
}

#[test]
fn the_type_option_reads_its_id_its_tag_and_its_inversion() {
    assert_eq!(
        options("@e[type=cow]").predicates,
        [SelectorPredicate::Type {
            id: "minecraft:cow".to_string(),
            inverted: false
        }]
    );
    assert_eq!(
        options("@e[type=!minecraft:cow]").predicates,
        [SelectorPredicate::Type {
            id: "minecraft:cow".to_string(),
            inverted: true
        }]
    );
    assert_eq!(options("@e[type=!cow,type=!pig]").predicates.len(), 2);
    // `isTag` (EntitySelectorParser.java:338-347) takes the `#` after the
    // inversion, and the tag form never calls `limitToType`.
    assert_eq!(
        options("@e[type=#undead,type=#skeletons,type=zombie]").predicates,
        [
            SelectorPredicate::TypeTag {
                tag: "minecraft:undead".to_string(),
                inverted: false
            },
            SelectorPredicate::TypeTag {
                tag: "minecraft:skeletons".to_string(),
                inverted: false
            },
            SelectorPredicate::Type {
                id: "minecraft:zombie".to_string(),
                inverted: false
            },
        ]
    );
    assert_eq!(
        options("@e[type=!#skeletons]").predicates,
        [SelectorPredicate::TypeTag {
            tag: "minecraft:skeletons".to_string(),
            inverted: true
        }]
    );
    // An inverted tag still sets `typeInverse`
    // (EntitySelectorOptions.java:282-284), so an uninverted type after it is
    // refused.
    assert_eq!(
        refusal_key("@e[type=!#skeletons,type=zombie]"),
        Some(("argument.entity.options.inapplicable".to_string(), 25))
    );
}

#[test]
fn the_tag_option_reads_a_name_the_empty_form_and_the_inversion() {
    assert_eq!(
        options("@e[tag=boss]").predicates,
        [SelectorPredicate::Tag {
            name: "boss".to_string(),
            inverted: false
        }]
    );
    assert_eq!(
        options("@e[tag=!]").predicates,
        [SelectorPredicate::Tag {
            name: String::new(),
            inverted: true
        }]
    );
    assert_eq!(options("@e[tag=a,tag=b,tag=!c]").predicates.len(), 3);
}

#[test]
fn the_limit_and_sort_options_read_the_values_the_selector_carries() {
    assert_eq!(options("@e[limit=3]").max_results, Some(3));
    for (name, order) in [
        ("nearest", SelectorOrder::Nearest),
        ("furthest", SelectorOrder::Furthest),
        ("random", SelectorOrder::Random),
        ("arbitrary", SelectorOrder::Arbitrary),
    ] {
        assert_eq!(options(&format!("@e[sort={name}]")).order, Some(order));
    }
}

#[test]
fn the_name_team_and_predicate_options_read_their_words() {
    assert_eq!(
        options("@e[name=\"Bob Smith\",team=!,predicate=!pack:check]").predicates,
        [
            SelectorPredicate::Name {
                name: "Bob Smith".into(),
                inverted: false
            },
            SelectorPredicate::Team {
                name: String::new(),
                inverted: true
            },
            SelectorPredicate::Predicate {
                id: "pack:check".to_string(),
                inverted: true
            },
        ]
    );
    // Only the uninverted form sets `hasNameEquals` / `hasTeamEquals`.
    assert_eq!(options("@e[name=!a,name=!b]").predicates.len(), 2);
    assert_eq!(options("@e[team=!a,team=b]").predicates.len(), 2);
}

#[test]
fn the_position_options_read_their_values_and_limit_the_world() {
    let read = options("@e[x=1,y=2.5,z=-3,dx=4,dy=-5,dz=.5]");
    assert_eq!((read.x, read.y, read.z), (Some(1.0), Some(2.5), Some(-3.0)));
    assert_eq!(
        (read.delta_x, read.delta_y, read.delta_z),
        (Some(4.0), Some(-5.0), Some(0.5))
    );
    assert!(read.world_limited);
    let read = options("@e[distance=..5]");
    assert_eq!(read.distance, Some(bounds(None, Some(5.0))));
    assert!(read.world_limited);
    // `level`, the rotations and every predicate option leave the flag alone.
    let read = options("@a[level=3..,x_rotation=-90..0,y_rotation=45]");
    assert_eq!(read.level, Some(bounds(Some(3), None)));
    assert_eq!(read.rot_x, Some(bounds(Some(-90.0), Some(0.0))));
    assert_eq!(read.rot_y, Some(bounds(Some(45.0), Some(45.0))));
    assert!(!read.world_limited);
    // `FloatDegrees.fromReader` (MinMaxBounds.java:260-265) has no swapped
    // check: the wrapped range is `createRotationPredicate`'s to read.
    assert_eq!(
        options("@e[y_rotation=90..-90]").rot_y,
        Some(bounds(Some(90.0), Some(-90.0)))
    );
}

#[test]
fn the_gamemode_nbt_and_scores_options_read_their_payloads() {
    assert_eq!(
        options("@a[gamemode=!creative,gamemode=!spectator]").predicates,
        [
            SelectorPredicate::GameMode {
                game_type: GameType::Creative,
                inverted: true
            },
            SelectorPredicate::GameMode {
                game_type: GameType::Spectator,
                inverted: true
            },
        ]
    );
    let predicates = options("@e[nbt=!{Health:20f}]").predicates;
    let [SelectorPredicate::Nbt { tag, inverted }] = predicates.as_slice() else {
        panic!("the option reads one nbt predicate");
    };
    assert!(inverted);
    assert!(matches!(tag, NbtSummaryValue::Compound(entries) if entries.len() == 1));
    // `expected.put` over a `HashMap` (EntitySelectorOptions.java:341) keeps
    // the last bound for a repeated objective.
    assert_eq!(
        options("@e[scores={kills=1..,deaths=..0,kills=2}]").predicates,
        [SelectorPredicate::Scores(vec![
            ("deaths".to_string(), bounds(None, Some(0))),
            ("kills".to_string(), bounds(Some(2), Some(2))),
        ])]
    );
    // `:349`: an empty map adds no predicate, although it still counts as the
    // one `scores` the selector may carry.
    assert_eq!(options("@e[scores={}]").predicates, []);
    assert_eq!(
        refusal_key("@e[scores={},scores={}]"),
        Some(("argument.entity.options.inapplicable".to_string(), 19))
    );
}

#[test]
fn the_advancements_map_is_read_as_its_entries() {
    assert_eq!(
        options("@a[advancements={minecraft:story/root=true}]").predicates,
        [SelectorPredicate::Advancements(vec![done(
            "minecraft:story/root",
            true
        )])]
    );
    assert_eq!(
        options("@a[advancements={story/root={crafting_table=true,other=false}}]").predicates,
        [SelectorPredicate::Advancements(vec![criteria(
            "minecraft:story/root",
            &[("crafting_table", true), ("other", false)]
        )])]
    );
    assert_eq!(
        options("@a[\"advancements\"={minecraft:test='true'}]").predicates,
        [SelectorPredicate::Advancements(vec![done(
            "minecraft:test",
            true
        )])]
    );
    assert_eq!(
        options("@a[advancements={minecraft:test=true,minecraft:test=false}]").predicates,
        [SelectorPredicate::Advancements(vec![done(
            "minecraft:test",
            false
        )])]
    );
    assert_eq!(options("@e[advancements={}]"), SelectorOptions::default());
}

#[test]
fn whitespace_and_trailing_commas_are_tolerated_the_way_the_loops_are() {
    let selector = "@e[ advancements = { minecraft:test = { a=true b=false,},}]";
    assert!(valid_selector(selector));
    assert_eq!(
        options(selector).predicates,
        [SelectorPredicate::Advancements(vec![criteria(
            "minecraft:test",
            &[("a", true), ("b", false)]
        )])]
    );
    assert_eq!(
        options("@e[ limit = 2 , sort = nearest ]").max_results,
        Some(2)
    );
    assert_eq!(
        options("@e[ scores = { a = 1 b = 2 } ]").predicates.len(),
        1
    );
}

#[test]
fn every_registered_option_is_read_where_the_shared_parser_accepts_it() {
    for selector in [
        "@e[name=alice]",
        "@e[distance=..5]",
        "@a[level=1..2]",
        "@e[x=1,y=2,z=3,dx=1,dy=1,dz=1]",
        "@e[x_rotation=..0,y_rotation=-45..45]",
        "@e[gamemode=creative]",
        "@e[team=red]",
        "@e[type=#minecraft:undead]",
        "@e[nbt={Health:1f,Tags:[\"a\"]}]",
        "@e[scores={objective=1}]",
        "@e[predicate=minecraft:test]",
    ] {
        assert!(valid_selector(selector), "{selector}");
        assert!(read(selector).is_ok(), "{selector}");
    }
}

#[test]
fn each_option_raises_its_own_refusal_where_vanilla_does() {
    for (selector, key, offset) in [
        ("@e[nosuchoption=1]", "argument.entity.options.unknown", 3),
        ("@a[type=cow]", "argument.entity.options.inapplicable", 7),
        ("@s[limit=1]", "argument.entity.options.inapplicable", 8),
        (
            "@s[sort=nearest]",
            "argument.entity.options.inapplicable",
            7,
        ),
        (
            "@e[limit=1,limit=2]",
            "argument.entity.options.inapplicable",
            16,
        ),
        (
            "@e[sort=nearest,sort=random]",
            "argument.entity.options.inapplicable",
            20,
        ),
        (
            "@e[type=cow,type=pig]",
            "argument.entity.options.inapplicable",
            16,
        ),
        (
            "@e[type=zombie,type=#undead]",
            "argument.entity.options.inapplicable",
            19,
        ),
        (
            "@e[type=!cow,type=pig]",
            "argument.entity.options.inapplicable",
            18,
        ),
        (
            "@e[advancements={},advancements={}]",
            "argument.entity.options.inapplicable",
            31,
        ),
        ("@e[limit]", "argument.entity.options.valueless", 3),
        ("@e[limit=0]", "argument.entity.options.limit.toosmall", 9),
        (
            "@e[sort=sideways]",
            "argument.entity.options.sort.irreversible",
            8,
        ),
        (
            "@e[type=minecraft:nosuch]",
            "argument.entity.options.type.invalid",
            8,
        ),
        ("@e[type=!Cow]", "argument.entity.options.type.invalid", 8),
        ("@e[type=a:b:c]", "argument.id.invalid", 8),
        ("@e[limit=1 2]", "argument.entity.options.unterminated", 11),
        ("@e[limit=1", "argument.entity.options.unterminated", 10),
        // `register("name")` (EntitySelectorOptions.java:90-106): the repeat
        // `canUse` reports past the key, the inversion check from the value.
        (
            "@e[name=a,name=b]",
            "argument.entity.options.inapplicable",
            14,
        ),
        (
            "@e[name=!a,name=b]",
            "argument.entity.options.inapplicable",
            16,
        ),
        ("@e[name=\"a]", "parsing.quote.expected.end", 11),
        // `register("distance")` (`:107-117`) and `MinMaxBounds`.
        (
            "@e[distance=-1]",
            "argument.entity.options.distance.negative",
            12,
        ),
        (
            "@e[distance=..5,distance=1]",
            "argument.entity.options.inapplicable",
            24,
        ),
        ("@e[distance=5..1]", "argument.range.swapped", 12),
        ("@e[distance=]", "argument.range.empty", 12),
        ("@e[distance=1-2]", "parsing.double.invalid", 12),
        // `register("level")` (`:118-128`).
        (
            "@a[level=-1..]",
            "argument.entity.options.level.negative",
            9,
        ),
        (
            "@a[level=1,level=2]",
            "argument.entity.options.inapplicable",
            16,
        ),
        // `x`..`dz` (`:129-152`) over `StringReader.readDouble`.
        ("@e[x=1,x=2]", "argument.entity.options.inapplicable", 8),
        ("@e[dz=1,dz=2]", "argument.entity.options.inapplicable", 10),
        ("@e[x=abc]", "parsing.double.expected", 5),
        // `x_rotation` / `y_rotation` (`:153-164`) over `FloatDegrees`.
        (
            "@e[x_rotation=1,x_rotation=2]",
            "argument.entity.options.inapplicable",
            26,
        ),
        ("@e[x_rotation=a]", "argument.range.empty", 14),
        ("@e[y_rotation=1-2]", "parsing.float.invalid", 14),
        // `register("gamemode")` (`:193-249`).
        (
            "@a[gamemode=creative,gamemode=survival]",
            "argument.entity.options.inapplicable",
            29,
        ),
        (
            "@a[gamemode=!creative,gamemode=survival]",
            "argument.entity.options.inapplicable",
            31,
        ),
        (
            "@a[gamemode=flying]",
            "argument.entity.options.mode.invalid",
            12,
        ),
        (
            "@a[gamemode=!flying]",
            "argument.entity.options.mode.invalid",
            12,
        ),
        // `register("team")` (`:250-263`).
        (
            "@e[team=red,team=blue]",
            "argument.entity.options.inapplicable",
            16,
        ),
        // `register("nbt")` (`:310-327`) over `TagParser`.
        ("@e[nbt=1]", "argument.nbt.expected.compound", 8),
        // `register("scores")` (`:328-374`).
        ("@e[scores=1]", "parsing.expected", 10),
        ("@e[scores={a}]", "parsing.expected", 12),
        ("@e[scores={a=}]", "argument.range.empty", 13),
        ("@e[scores={a=2..1}]", "argument.range.swapped", 13),
        // `register("advancements")` (`:375-454`).
        ("@e[advancements={Bad=true}]", "parsing.expected", 17),
        (
            "@e[advancements={story/root=maybe}]",
            "parsing.bool.invalid",
            28,
        ),
        (
            "@e[advancements={story/root=}]",
            "parsing.bool.expected",
            28,
        ),
        ("@e[advancements={a:b:c=true}]", "argument.id.invalid", 17),
        // `register("predicate")` (`:455-483`).
        ("@e[predicate=a:b:c]", "argument.id.invalid", 13),
    ] {
        assert_eq!(
            refusal_key(selector),
            Some((key.to_string(), offset)),
            "{selector}"
        );
    }
}

#[test]
fn every_refusal_names_a_selector_the_shared_parser_also_refuses() {
    for selector in [
        "@e[nosuchoption=1]",
        "@a[type=cow]",
        "@s[limit=1]",
        "@e[limit=1,limit=2]",
        "@e[type=!cow,type=pig]",
        "@e[type=zombie,type=#undead]",
        "@e[limit]",
        "@e[limit=0]",
        "@e[sort=sideways]",
        "@e[type=minecraft:nosuch]",
        "@e[limit=1 2]",
        "@e[limit=1",
        "@e[name=a,name=b]",
        "@e[name=!a,name=b]",
        "@e[distance=-1]",
        "@e[distance=..5,distance=1]",
        "@e[distance=5..1]",
        "@a[level=-1..]",
        "@e[x=1,x=2]",
        "@e[x_rotation=a]",
        "@a[gamemode=!creative,gamemode=survival]",
        "@a[gamemode=flying]",
        "@e[team=red,team=blue]",
        "@e[nbt=1]",
        "@e[scores={a}]",
        "@e[scores={},scores={}]",
        "@e[advancements={story/root=maybe}]",
        "@e[predicate=a:b:c]",
    ] {
        assert!(read(selector).is_err(), "{selector}");
        assert!(!valid_selector(selector), "{selector}");
    }
}
