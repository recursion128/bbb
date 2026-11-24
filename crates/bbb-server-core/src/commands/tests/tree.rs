//! The published command tree and its executor coverage.

use super::*;

#[test]
fn the_published_tree_matches_every_implemented_node_after_cuts() {
    let tree = usable_command_tree(CommandPermission::Owner);
    // The wire tree is walked beside the derived one from the root down, so
    // every published node is checked against the catalog node it claims to be.
    let mut queue = vec![(0usize, VANILLA_COMMAND_TREE_ROOT)];
    let mut published = 0;
    while let Some((wire, derived)) = queue.pop() {
        published += 1;
        let node = &tree.nodes[wire];
        let source = catalog::node(derived);
        assert_eq!(node.node_type, source.kind);
        assert_eq!(node.executable, source.executable);
        assert_eq!(node.restricted, source.restricted());
        assert_eq!(node.suggestions.as_deref(), source.suggestions);
        match node.node_type {
            CommandNodeType::Root => assert!(node.name.is_none()),
            _ => assert_eq!(node.name.as_deref(), Some(source.name)),
        }
        if let Some(parser) = &node.parser {
            assert_eq!(Some(parser.type_id), source.parser_type_id());
            assert_eq!(parser.template().unwrap(), argument_template(source));
        } else {
            assert_eq!(source.parser, None);
        }
        let children =
            catalog::usable_children(derived, CommandPermission::Owner).collect::<Vec<_>>();
        assert_eq!(node.children.len(), children.len());
        for (wire_child, derived_child) in node.children.iter().zip(children) {
            queue.push((*wire_child as usize, derived_child));
        }
    }
    assert_eq!(published, tree.nodes.len());
    // Every command this server implements is reachable from the root.
    for name in catalog::IMPLEMENTED_COMMANDS {
        assert!(
            tree.nodes[0]
                .children
                .iter()
                .any(|child| tree.nodes[*child as usize].name.as_deref() == Some(*name)),
            "{name} is missing from the published tree"
        );
    }
}

#[test]
fn the_published_tree_redirects_an_alias_at_the_command_it_points_at() {
    let tree = usable_command_tree(CommandPermission::Owner);
    let index_of = |name: &str| {
        tree.nodes[0]
            .children
            .iter()
            .copied()
            .find(|child| tree.nodes[*child as usize].name.as_deref() == Some(name))
            .expect("an implemented command is published")
    };
    for (alias, target) in [
        ("tell", "msg"),
        ("w", "msg"),
        ("tm", "teammsg"),
        ("tp", "teleport"),
    ] {
        let node = &tree.nodes[index_of(alias) as usize];
        assert_eq!(node.redirect, Some(index_of(target)));
        assert!(node.children.is_empty());
    }
}

#[test]
fn the_published_tree_hides_every_node_above_the_sources_permission() {
    let tree = usable_command_tree(CommandPermission::All);
    let names = tree.nodes[0]
        .children
        .iter()
        .map(|child| tree.nodes[*child as usize].name.clone().unwrap())
        .collect::<Vec<_>>();
    // The `PermissionLevel.ALL` commands this server implements.
    assert_eq!(
        names,
        vec![
            "me".to_string(),
            "help".to_string(),
            "list".to_string(),
            "msg".to_string(),
            "tell".to_string(),
            "w".to_string(),
            "random".to_string(),
            "teammsg".to_string(),
            "tm".to_string(),
            "trigger".to_string(),
        ]
        .into_iter()
        .filter(|name| catalog::IMPLEMENTED_COMMANDS.contains(&name.as_str()))
        .collect::<Vec<_>>()
    );
    assert!(tree.nodes.iter().all(|node| !node.restricted));
}

#[test]
fn every_published_tree_survives_its_own_encoder() {
    for permission in LEVELS {
        let tree = usable_command_tree(permission);
        let (packet_id, payload) = encode_play_clientbound_commands(&tree).unwrap();

        assert_eq!(packet_id, ids::play::CLIENTBOUND_COMMANDS);
        assert_eq!(
            decode_play_clientbound(packet_id, &payload).unwrap(),
            PlayClientbound::Commands(tree),
            "{permission:?} tree does not decode back to itself"
        );
    }
}

#[test]
fn the_published_argument_properties_are_the_ones_the_catalog_recorded() {
    let tree = usable_command_tree(CommandPermission::Owner);
    let message = vanilla_command_lookup(&["say", "message"]).expect("say takes a message");
    let count = tree
        .nodes
        .iter()
        .filter(|node| node.node_type == CommandNodeType::Argument)
        .count();
    assert!(count > 0);
    // The singleton parsers carry no property payload at all, while the ones
    // the catalog gave properties carry exactly the bytes their template
    // encodes.
    assert_eq!(
        argument_template(catalog::node(message)),
        bbb_protocol::packets::CommandArgumentTemplate::Unit
    );
    let targets = vanilla_command_lookup(&["msg", "targets"]).expect("msg takes targets");
    assert_eq!(
        argument_template(catalog::node(targets)),
        bbb_protocol::packets::CommandArgumentTemplate::Entity {
            single: false,
            players_only: true,
        }
    );
    let value = vanilla_command_lookup(&["gamerule", "random_tick_speed", "value"])
        .expect("the node exists");
    assert_eq!(
        argument_template(catalog::node(value)),
        bbb_protocol::packets::CommandArgumentTemplate::Integer {
            min: Some(0),
            max: None,
        }
    );
}

#[test]
fn the_published_flag_byte_carries_the_two_bits_no_builder_sets() {
    // `createEntry` (ClientboundCommandsPacket.java:146-180) ors 1 for a
    // literal (:179), 2 for an argument (:172), 4 for an executable node
    // (:155-157), 8 for a redirect (:148-150), 16 for a custom suggestion
    // provider (:173-175) and 32 for a restricted one (:159-161).
    let tree = usable_command_tree(CommandPermission::Owner);
    let index_of = |children: &[i32], name: &str| {
        children
            .iter()
            .copied()
            .find(|child| tree.nodes[*child as usize].name.as_deref() == Some(name))
            .expect("the published tree carries the node")
    };
    let roots = tree.nodes[0].children.clone();

    // `tell` redirects at `msg` and carries nothing else.
    assert_eq!(tree.nodes[index_of(&roots, "tell") as usize].flags, 1 | 8);
    // `ban` needs `LEVEL_ADMINS`, so it is restricted and nothing more.
    let ban = index_of(&roots, "ban");
    assert_eq!(tree.nodes[ban as usize].flags, 1 | 32);
    // Its `targets` argument runs the command and has no suggestion provider of
    // its own.
    let ban_targets = index_of(&tree.nodes[ban as usize].children, "targets");
    assert_eq!(tree.nodes[ban_targets as usize].flags, 2 | 4);
    assert_eq!(tree.nodes[ban_targets as usize].suggestions, None);
    // `/op`'s does have one, which is the only difference between the two.
    let op = index_of(&roots, "op");
    let op_targets = index_of(&tree.nodes[op as usize].children, "targets");
    assert_eq!(tree.nodes[op_targets as usize].flags, 2 | 4 | 16);
    assert_eq!(
        tree.nodes[op_targets as usize].suggestions.as_deref(),
        Some("minecraft:ask_server")
    );
}

/// A value the parser behind `index` accepts, so a generated command reaches
/// the node that carries the executor.
fn sample_token(index: u16) -> String {
    let node = catalog::node(index);
    if node.kind != CommandNodeType::Argument {
        return node.name.to_string();
    }
    match node.parser {
        Some("brigadier:bool") => "true".to_string(),
        // The bound the tree itself publishes, not a value assumed to be
        // inside every one of them: `gamerule minecraft:max_minecart_speed`
        // takes `{"min":1,"max":1000}`, so zero is refused there.
        Some("brigadier:integer") => {
            let CommandArgumentTemplate::Integer { min, max } = argument_template(node) else {
                unreachable!("the integer parser carries an integer template");
            };
            min.unwrap_or(0).min(max.unwrap_or(i32::MAX)).to_string()
        }
        // The float bound the tree publishes, the same way the integer arm
        // above reads its own: `/tick rate` takes `1.0..=10000.0`
        // (TickCommand.java:28).
        Some("brigadier:float") => {
            let CommandArgumentTemplate::Float { min, max } = argument_template(node) else {
                unreachable!("the float parser carries a float template");
            };
            let value = min.unwrap_or(0.0).min(max.unwrap_or(f32::MAX));
            format!("{value}")
        }
        Some("brigadier:double") => {
            let CommandArgumentTemplate::Double { min, max } = argument_template(node) else {
                unreachable!("the double parser carries a double template");
            };
            let value = min.unwrap_or(0.0).min(max.unwrap_or(f64::MAX));
            format!("{value}")
        }
        // `TimeArgument` reads a number and then a unit
        // (TimeArgument.java:44-51); `t` is the tick unit (TimeArgument.java:81),
        // and every published `<time>` node takes `time(1)`
        // (TickCommand.java:38, :47) so one tick is inside the bound.
        Some("minecraft:time") => "1t".to_string(),
        Some("minecraft:entity") | Some("minecraft:game_profile") => "alice".to_string(),
        Some("brigadier:string") | Some("minecraft:message") => "text".to_string(),
        // `GameType.getName()` for the first value, which is the one
        // `GameModeArgument.EXAMPLES` also leads with (GameModeArgument.java:21).
        Some("minecraft:gamemode") => "survival".to_string(),
        // `/damage` samples `generic` (`DamageCommand.java:34`); `/attribute`
        // samples `max_health` (`Attributes.java:58-59`); `/enchant` samples
        // `sharpness` (`EnchantCommand.java:46`).
        Some("minecraft:resource") => match argument_template(node) {
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::ATTRIBUTE_REGISTRY =>
            {
                "minecraft:max_health".to_string()
            }
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::ENCHANTMENT_REGISTRY =>
            {
                "minecraft:sharpness".to_string()
            }
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::MOB_EFFECT_REGISTRY =>
            {
                "minecraft:speed".to_string()
            }
            CommandArgumentTemplate::Registry { registry }
                if registry == "minecraft:worldgen/biome" =>
            {
                "minecraft:plains".to_string()
            }
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::ENTITY_TYPE_REGISTRY =>
            {
                "minecraft:pig".to_string()
            }
            // `/time of` samples the clock every built-in dimension type but
            // the Nether declares (`WorldClocks.java:9`), and `query` the
            // timeline that clock drives.
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::WORLD_CLOCK_REGISTRY =>
            {
                "minecraft:overworld".to_string()
            }
            CommandArgumentTemplate::Registry { registry }
                if registry == catalog::TIMELINE_REGISTRY =>
            {
                "minecraft:day".to_string()
            }
            _ => GENERIC_DAMAGE_TYPE.to_string(),
        },
        // `Vec3Argument.EXAMPLES` leads with `0 0 0` (Vec3Argument.java:21).
        Some("minecraft:vec3") => "0 0 0".to_string(),
        // `Vec2Argument.EXAMPLES` leads with `0 0` (Vec2Argument.java:22).
        Some("minecraft:vec2") => "0 0".to_string(),
        // `RotationArgument.EXAMPLES` leads with `0 0`
        // (RotationArgument.java:14).
        Some("minecraft:rotation") => "0 0".to_string(),
        // `EntityAnchorArgument.EXAMPLES` leads with `eyes`
        // (EntityAnchorArgument.java:25).
        Some("minecraft:entity_anchor") => "eyes".to_string(),
        // `SwizzleArgument.EXAMPLES` leads with `xyz` (SwizzleArgument.java:16).
        Some("minecraft:swizzle") => "xyz".to_string(),
        // `StringRepresentableArgument.EXAMPLES` (`:51-56`) over
        // `HeightmapTypeArgument.keptTypes` leads with `world_surface`.
        Some("minecraft:heightmap") => "world_surface".to_string(),
        // `ResourceOrTagArgument.EXAMPLES` leads with `foo`
        // (ResourceOrTagArgument.java:33), which no registry holds. `/locate
        // poi` samples the first `PoiTypes.bootstrap` entry (PoiTypes.java:124).
        Some("minecraft:resource_or_tag") => match argument_template(node) {
            CommandArgumentTemplate::Registry { registry }
                if registry == "minecraft:point_of_interest_type" =>
            {
                "minecraft:armorer".to_string()
            }
            _ => "minecraft:plains".to_string(),
        },
        // `ResourceOrTagKeyArgument.EXAMPLES` leads with `foo`
        // (ResourceOrTagKeyArgument.java:29); the key resolves when the
        // command runs (`:40-49`), so the sample reaches the executor.
        Some("minecraft:resource_or_tag_key") => "foo".to_string(),
        // `StringRepresentableArgument` over `Rotation` and `Mirror`
        // (TemplateRotationArgument.java:9, TemplateMirrorArgument.java:9);
        // both lead with `none` (Rotation.java:18, Mirror.java:11).
        Some("minecraft:template_rotation") | Some("minecraft:template_mirror") => {
            "none".to_string()
        }
        // `FunctionArgument.EXAMPLES` leads with `foo` (FunctionArgument.java:19).
        // No function names this sample, so the sample reaches the executor and
        // is refused there -- which is the point: the refusal is the executor's
        // (`arguments.function.unknown`) rather than a missing arm.
        Some("minecraft:function") => "foo".to_string(),
        // `CompoundTagArgument.EXAMPLES` leads with `{}` (CompoundTagArgument.java:13).
        Some("minecraft:nbt_compound_tag") => "{}".to_string(),
        Some("minecraft:nbt_tag") => "0".to_string(),
        Some("minecraft:nbt_path") => "foo".to_string(),
        // `ComponentArgument.EXAMPLES` leads with `"hello world"`
        // (ComponentArgument.java:24).
        Some("minecraft:component") => "\"hello world\"".to_string(),
        // `BlockPosArgument.EXAMPLES` leads with `0 0 0`
        // (BlockPosArgument.java:23). No column is loaded in the world a
        // sample runs against, so the executor refuses it with
        // `argument.pos.unloaded` -- which is the point: the refusal is
        // `getLoadedBlockPos`'s rather than a missing arm.
        Some("minecraft:block_pos") => "0 0 0".to_string(),
        // `ColumnPosArgument.EXAMPLES` leads with `0 0`
        // (ColumnPosArgument.java:22). The forced set of the world a
        // sample runs against is empty, so `add` reaches the executor and
        // `remove`/`query` are refused there -- which is the point: the refusal
        // is `ForceLoadCommand`'s rather than a missing arm.
        Some("minecraft:column_pos") => "0 0".to_string(),
        // `BlockStateArgument.EXAMPLES` leads with `stone`
        // (BlockStateArgument.java:19) and `BlockPredicateArgument.EXAMPLES`
        // with the same name (BlockPredicateArgument.java:31).
        Some("minecraft:block_state") | Some("minecraft:block_predicate") => "stone".to_string(),
        // `DimensionArgument.EXAMPLES` leads with `minecraft:overworld`
        // (DimensionArgument.java:24-26).
        Some("minecraft:dimension") => "minecraft:overworld".to_string(),
        // `IdentifierArgument.EXAMPLES` leads with `foo`
        // (IdentifierArgument.java:13). The id names no registry, so the
        // sample reaches the executor as it is.
        Some("minecraft:resource_location") => "foo".to_string(),
        // `ResourceKeyArgument.EXAMPLES` leads with `foo`
        // (ResourceKeyArgument.java:36). Existence is checked when the
        // command runs (`:105-113`), so the sample reaches the executor.
        Some("minecraft:resource_key") => "foo".to_string(),
        // `RangeArgument.Ints.EXAMPLES` leads with `0..5`
        // (RangeArgument.java:39). A single number is both bounds and
        // `randomSample` refuses a zero span, so the sample has to be a
        // genuine range for the executor to be the thing that runs.
        Some("minecraft:int_range") => "0..5".to_string(),
        // `RangeArgument.Floats.EXAMPLES` leads with `0..5.2`
        // (RangeArgument.java:22).
        Some("minecraft:float_range") => "0..5.2".to_string(),
        // `ItemArgument.EXAMPLES` leads with `stick` (ItemArgument.java:15).
        Some("minecraft:item_stack") => "stick".to_string(),
        // `SlotArgument.EXAMPLES` leads with `container.5` (SlotArgument.java:21).
        Some("minecraft:item_slot") => "container.5".to_string(),
        Some("minecraft:item_slots") => "container.*".to_string(),
        Some("minecraft:item_predicate") => "*".to_string(),
        Some("minecraft:loot_predicate") => "{condition:'minecraft:all_of',terms:[]}".to_string(),
        // `ResourceOrIdArgument.EXAMPLES` leads with `foo`
        // (ResourceOrIdArgument.java:45), which is the reference spelling and
        // is refused against an empty `minecraft:item_modifier` registry. The
        // inline spelling is the one that reaches the executor, so the sample
        // is the smallest function this server evaluates.
        Some("minecraft:loot_modifier") => "{function:'minecraft:set_count',count:1}".to_string(),
        // `ResourceOrIdArgument.EXAMPLES` leads with `foo`
        // (ResourceOrIdArgument.java:45). A name is refused against an empty
        // loot-table registry; the inline chest table is the one that reaches
        // execute (`LootCommand.java:467-474`).
        Some("minecraft:loot_table") => {
            "{type:'minecraft:chest',pools:[],random_sequence:'minecraft:commands/loot'}"
                .to_string()
        }
        // `ResourceOrIdArgument.EXAMPLES` leads with `foo`
        // (ResourceOrIdArgument.java:45). A name is refused against an empty
        // dialog registry; the inline notice is the one that reaches execute.
        Some("minecraft:dialog") => "{type:'minecraft:notice',title:'Hello'}".to_string(),
        // `ParticleArgument.EXAMPLES` includes a bare name
        // (ParticleArgument.java:28). `heart` is Empty, so the sample reaches
        // the executor instead of `particle.invalidOptions`.
        Some("minecraft:particle") => "heart".to_string(),
        // `ObjectiveArgument.EXAMPLES` leads with `foo`
        // (ObjectiveArgument.java:20). No scoreboard is loaded here, so the
        // sample reaches the executor and is refused there with
        // `arguments.objective.notFound` -- which is the point.
        Some("minecraft:objective") => "foo".to_string(),
        // `ObjectiveCriteriaArgument.EXAMPLES` leads with `foo`
        // (ObjectiveCriteriaArgument.java:24), which names no criterion and so
        // never reaches the executor. `dummy` is the first `registerCustom`
        // (ObjectiveCriteria.java:23).
        Some("minecraft:objective_criteria") => "dummy".to_string(),
        // `ScoreboardSlotArgument.EXAMPLES` leads with `sidebar`
        // (ScoreboardSlotArgument.java:19).
        Some("minecraft:scoreboard_slot") => "sidebar".to_string(),
        // `ScoreHolderArgument.EXAMPLES` leads with `Player`
        // (ScoreHolderArgument.java:45), which is a name-only holder here.
        Some("minecraft:score_holder") => "Player".to_string(),
        // `OperationArgument.EXAMPLES` leads with `=` (OperationArgument.java:20).
        Some("minecraft:operation") => "=".to_string(),
        // `StyleArgument.EXAMPLES` ends with `{}` (StyleArgument.java:20), the
        // one of the three that carries no field.
        Some("minecraft:style") => "{}".to_string(),
        // `TeamArgument.EXAMPLES` leads with `foo` (TeamArgument.java:20). No
        // scoreboard is loaded here, so the sample reaches the executor and
        // is refused there with `team.notFound`.
        Some("minecraft:team") => "foo".to_string(),
        // `ColorArgument.EXAMPLES` leads with `red` (ColorArgument.java:19).
        Some("minecraft:color") => "red".to_string(),
        // `HexColorArgument.EXAMPLES` leads with `F00` (:23).
        Some("minecraft:hex_color") => "F00".to_string(),
        other => panic!("no sample value for the parser {other:?}"),
    }
}

fn sample_command(index: u16) -> String {
    let mut path = Vec::new();
    let mut current = Some(index);
    while let Some(node) = current {
        if node == VANILLA_COMMAND_TREE_ROOT {
            break;
        }
        path.push(sample_token(node));
        current = catalog::node(node).parent;
    }
    path.reverse();
    path.join(" ")
}

#[test]
fn every_executable_node_this_server_publishes_has_an_executor() {
    let players = [user("alice", 1)];
    let mut fixture = CommandTestWorld::with_builtin_data(&players);
    let mut stack = vec![VANILLA_COMMAND_TREE_ROOT];
    let mut executed = 0;
    while let Some(node) = stack.pop() {
        stack.extend(catalog::usable_children(node, CommandPermission::Owner));
        if node == VANILLA_COMMAND_TREE_ROOT || !catalog::node(node).executable {
            continue;
        }
        let command = sample_command(node);
        assert_eq!(
            parse::parse(
                &command.encode_utf16().collect::<Vec<_>>(),
                CommandPermission::Owner,
                fixture.world()
            )
            .executable_node(),
            Some(node),
            "{command:?}"
        );
        fixture.run(&command, &source("alice", CommandPermission::Owner));
        executed += 1;
    }
    assert!(executed > 200, "only {executed} executable nodes were run");
}
