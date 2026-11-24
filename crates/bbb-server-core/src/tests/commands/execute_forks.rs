//! Sequential forks of one command over a live world: every queued entry runs
//! against the state the entries before it left
//! (`ExecutionContext.runCommandQueue`, ExecutionContext.java:85-110).

use crate::tests::function_tags::function_tag_entries;
use bbb_server_datapack::function::{
    parse_function_source, FunctionElement, FunctionLibrary, FUNCTION_REGISTRY_ID,
};

use super::*;

const OP: u64 = 1;

struct Stage {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    /// The three armor stands at x = 2.5, 4.5 and 6.5, nearest first from `op`
    /// at x = 0.5.
    stands: Vec<Uuid>,
}

fn stage(functions: &[(&str, &str)]) -> Stage {
    stage_with(operator_config(&[("op", 1)]), functions)
}

fn stage_with(config: ServerWorldConfig, functions: &[(&str, &str)]) -> Stage {
    let mut world = ServerWorld::new(config).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, OP, "op", 1);
    let mut moved = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(OP),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Overworld.into(),
            position: Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut moved,
    ));
    if !functions.is_empty() {
        world.replace_function_library(&FunctionLibrary {
            functions: functions
                .iter()
                .map(|(id, source)| FunctionElement {
                    id: (*id).to_string(),
                    source_pack: "test".to_string(),
                    source: parse_function_source(source).expect("a valid line surface"),
                })
                .collect(),
            tags: Some(function_tag_entries(FUNCTION_REGISTRY_ID, &[])),
            warnings: Vec::new(),
            tag_warnings: Vec::new(),
        });
    }
    let stands = [2.5, 4.5, 6.5]
        .into_iter()
        .map(|x| {
            world
                .spawn_stored_entity_for_test(
                    Java26BuiltinDimension::Overworld,
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                    Vec3d { x, y: 64.0, z: 0.5 },
                    crate::entity::EntityData::ArmorStand(
                        crate::entity::ArmorStandData::new().expect("armor stand attributes"),
                    ),
                )
                .1
        })
        .collect();
    let mut stage = Stage {
        sender,
        receiver,
        world,
        stands,
    };
    assert!(stage.run("scoreboard objectives add obj dummy").len() == 1);
    stage
}

impl Stage {
    /// The lines `op` was sent, as `key(arg, ..)`.
    fn run(&mut self, command: &str) -> Vec<String> {
        let tick = chat_command(&self.sender, &self.receiver, &mut self.world, OP, command);
        tick.events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(OP),
                    packet,
                } => shown_chat(&self.world, packet)
                    .map(|shown| translatable_line(shown.component())),
                _ => None,
            })
            .collect()
    }

    fn score(&self, holder: &str) -> Option<i32> {
        self.world
            .scoreboard()
            .score(holder, "obj")
            .map(|score| score.value)
    }

    /// What `execute store result score #probe obj <condition-or-command>`
    /// stored.
    fn measure(&mut self, tail: &str) -> Option<i32> {
        self.run(&format!("execute store result score #probe obj {tail}"));
        self.score("#probe")
    }
}

/// `ExecuteCommand` reads the score its own fork is adding to, so three forks
/// count three and each reports the total it left.
#[test]
fn each_fork_adds_to_the_score_the_fork_before_it_wrote() {
    let mut stage = stage(&[]);
    let lines =
        stage.run("execute as @e[type=armor_stand] run scoreboard players add #fixed obj 1");
    assert_eq!(lines.len(), 3, "{lines:?}");
    for (line, total) in lines.iter().zip(1..) {
        assert!(
            line.starts_with("commands.scoreboard.players.add.success.single(1, ")
                && line.ends_with(&format!(", #fixed, {total})")),
            "{lines:?}"
        );
    }
    assert_eq!(stage.score("#fixed"), Some(3));
}

/// `BuildContexts.execute` (BuildContexts.java:40-119) tests the condition for
/// every source before it schedules the first `ExecuteCommand`, so all three
/// sources pass a test the first fork's write would have failed.
#[test]
fn a_condition_is_tested_for_every_source_before_the_first_fork_runs() {
    let mut stage = stage(&[]);
    stage.run("scoreboard players set #gate obj 0");
    stage.run(
        "execute as @e[type=armor_stand] if score #gate obj matches 0 run scoreboard players add #gate obj 1",
    );
    assert_eq!(stage.score("#gate"), Some(3));
}

/// The callback travels with each source (`storeValue`,
/// ExecuteCommand.java:414-425) and fires with the value that fork's command
/// returned.
#[test]
fn a_store_behind_the_forks_keeps_what_each_fork_returned() {
    let mut stage = stage(&[]);
    stage.run(
        "execute as @e[type=armor_stand,sort=nearest] store result score @s obj run scoreboard players add #count obj 1",
    );
    let stands = stage.stands.clone();
    let scores: Vec<_> = stands
        .iter()
        .map(|uuid| stage.score(&uuid.to_string()))
        .collect();
    assert_eq!(scores, [Some(1), Some(2), Some(3)]);
    assert_eq!(stage.score("#count"), Some(3));
}

/// `StorageDataAccessor.getData` (StorageDataAccessor.java:56-58) is a live
/// read, so each append lands on the list the fork before it grew.
#[test]
fn each_fork_appends_to_the_storage_the_fork_before_it_wrote() {
    let mut stage = stage(&[]);
    stage.run(
        "execute as @e[type=armor_stand] run data modify storage test:forks seen append value 1",
    );
    assert_eq!(
        stage.measure("run data get storage test:forks seen"),
        Some(3)
    );
}

/// A tag the first fork wrote takes its entity out of the second fork's
/// `tag=!claimed` selection, so the three forks claim three stands.
#[test]
fn a_tag_one_fork_added_hides_its_entity_from_the_next_forks_selector() {
    let mut stage = stage(&[]);
    let lines = stage.run(
        "execute as @e[type=armor_stand] run tag @e[type=armor_stand,tag=!claimed,limit=1,sort=nearest] add claimed",
    );
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert_eq!(stage.measure("if entity @e[tag=claimed]"), Some(3));
}

/// `storeData` (ExecuteCommand.java:440-456) reads the accessor when the
/// callback fires, which for a function is the body's `return`, after the
/// body's own append already landed.
#[test]
fn a_data_store_reads_the_storage_when_the_body_returns() {
    let mut stage = stage(&[(
        "test:mark",
        "data modify storage test:forks seen append value 1\nreturn 7",
    )]);
    stage.run(
        "execute as @e[type=armor_stand] store result storage test:forks last int 1 run function test:mark",
    );
    assert_eq!(
        stage.measure("run data get storage test:forks seen"),
        Some(3)
    );
    assert_eq!(
        stage.measure("run data get storage test:forks last"),
        Some(7)
    );
}

/// `ContextChain.runExecutable` (ContextChain.java:61-76) swallows a forked
/// command's exception, while the unforked chain reports it.
#[test]
fn a_forked_command_that_throws_reports_nothing() {
    let mut stage = stage(&[]);
    assert_eq!(
        stage.run("execute as @e[type=armor_stand] run scoreboard players get @s obj"),
        Vec::<String>::new()
    );
    let unforked = stage.run("execute run scoreboard players get #nobody obj");
    assert_eq!(unforked.len(), 1, "{unforked:?}");
    assert!(
        unforked[0].starts_with("commands.scoreboard.players.get.null"),
        "{unforked:?}"
    );
}

/// `ReturnFromCommandCustomModifier.apply` (ReturnCommand.java:41-59) queues
/// one continuation over every source, and a returning chain executes only the
/// first of them (BuildContexts.java:109-113).
#[test]
fn a_returning_chain_runs_its_command_for_the_first_source_alone() {
    let mut stage = stage(&[]);
    stage.run(
        "execute as @e[type=armor_stand] run return run scoreboard players add #returned obj 1",
    );
    assert_eq!(stage.score("#returned"), Some(1));
}

/// `scheduleFunctionConditionsAndTest` (ExecuteCommand.java:1057-1113) queues
/// one continuation over the admitted sources, so the stage after the function
/// condition tests all of them before any fork runs.
#[test]
fn a_function_condition_continues_with_every_admitted_source_at_once() {
    let mut stage = stage(&[("test:yes", "return 1")]);
    stage.run("scoreboard players set #gate obj 0");
    stage.run(
        "execute as @e[type=armor_stand] if function test:yes if score #gate obj matches 0 run scoreboard players add #gate obj 1",
    );
    assert_eq!(stage.score("#gate"), Some(3));
}

/// The management surface drains the same queue and reports every line and
/// the last result the sequence produced.
#[test]
fn the_management_surface_reports_the_whole_fork_sequence() {
    let mut stage = stage(&[]);
    let tick = block_command(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        900,
        "execute as @e[type=armor_stand] run scoreboard players add #managed obj 2",
    );
    let (result, messages) = management_report(&tick);
    assert_eq!(result, Some(6));
    assert_eq!(messages.len(), 3, "{messages:?}");
    assert!(messages[2].ends_with(", 6)"), "{messages:?}");
    assert_eq!(stage.score("#managed"), Some(6));
}

/// `DifficultyCommand.setDifficulty` (DifficultyCommand.java:32-41) refuses
/// the difficulty the fork before it set, and the forked chain swallows the
/// refusal, so only the first fork reports.
#[test]
fn a_setting_the_first_fork_wrote_refuses_the_forks_after_it() {
    let mut stage = stage(&[]);
    stage.run("difficulty easy");
    let lines = stage.run("execute as @e[type=armor_stand] run difficulty hard");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("commands.difficulty.success("),
        "{lines:?}"
    );
    assert_eq!(stage.measure("run difficulty"), Some(3));
}

/// `mergeData` (DataCommands.java:373-387) refuses a merge that changes
/// nothing, which is what every fork after the first one leaves the entity.
#[test]
fn an_entity_merge_the_first_fork_landed_leaves_the_rest_unchanged() {
    let mut stage = stage(&[]);
    let lines = stage.run(
        "execute as @e[type=armor_stand] run data merge entity @e[type=armor_stand,sort=nearest,limit=1] {Small:1b}",
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("commands.data.entity.modified("),
        "{lines:?}"
    );
    assert_eq!(
        stage.measure("run data get entity @e[type=armor_stand,sort=nearest,limit=1] Small"),
        Some(1)
    );
}

/// The same refusal over a block entity the forks share.
#[test]
fn a_block_merge_the_first_fork_landed_leaves_the_rest_unchanged() {
    let mut stage = stage(&[]);
    assert_eq!(
        stage.run("setblock 1 64 3 chest{Items:[{Slot:0b,id:\"minecraft:stone\",count:1}]}"),
        ["commands.setblock.success(1, 64, 3)"]
    );
    let lines = stage.run(
        "execute as @e[type=armor_stand] run data merge block 1 64 3 {Items:[{Slot:1b,id:\"minecraft:dirt\",count:2}]}",
    );
    assert_eq!(lines, ["commands.data.block.modified(1, 64, 3)"]);
    assert_eq!(
        stage.measure("run data get block 1 64 3 Items[0].count"),
        Some(2)
    );
}

/// `sendSuccess` reads `acceptsSuccess` (CommandSourceStack.java:465) for
/// every fork, including the ones the world ran after the first.
#[test]
fn a_silenced_success_stays_silent_on_every_fork() {
    let mut stage = stage(&[]);
    stage.run("gamerule send_command_feedback false");
    assert_eq!(
        stage.run("execute as @e[type=armor_stand] run tag @s add quiet"),
        Vec::<String>::new()
    );
    stage.run("gamerule send_command_feedback true");
    assert_eq!(stage.measure("if entity @e[tag=quiet]"), Some(3));
}

/// `sendChatMessage` asks `this.silent` alone (CommandSourceStack.java:442-451),
/// so `/msg`'s outgoing copy survives `send_command_feedback false` on every
/// fork, not only the first.
#[test]
fn a_whisper_reports_its_outgoing_copy_on_every_fork() {
    let mut stage = stage(&[]);
    stage.run("gamerule send_command_feedback false");
    let lines = stage.run("execute as @e[type=armor_stand] run msg op hello");
    let keys: Vec<_> = lines
        .iter()
        .map(|line| line.split('(').next().unwrap_or_default())
        .collect();
    assert_eq!(
        keys,
        [
            "commands.message.display.outgoing",
            "commands.message.display.incoming",
        ]
        .repeat(3),
        "{lines:?}"
    );
}

/// A recipe grant settles in the world after its fork ran. A forked chain
/// swallows the `ERROR_GIVE_FAILED` (RecipeCommand.java:72-92) of every fork
/// that finds the recipe already known, while each fork's store still reads
/// that fork's own answer.
#[test]
fn a_live_result_settles_per_fork_and_a_forked_refusal_stays_silent() {
    let mut config = operator_config(&[("op", 1)]);
    config.datapacks = Some(Arc::new(
        bbb_server_datapack::startup::DatapackStartup::load(
            &bbb_server_datapack::startup::DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: DatapackLoadLimits::default(),
            },
        )
        .unwrap(),
    ));
    let mut stage = stage_with(config, &[]);
    let lines = stage.run(
        "execute as @e[type=armor_stand,sort=nearest] store success score @s obj run recipe give op minecraft:flint_and_steel",
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("commands.recipe.give.success.single("),
        "{lines:?}"
    );
    let stands = stage.stands.clone();
    let scores: Vec<_> = stands
        .iter()
        .map(|uuid| stage.score(&uuid.to_string()))
        .collect();
    assert_eq!(scores, [Some(1), Some(0), Some(0)]);
    assert_eq!(
        stage.run("execute run recipe give op minecraft:flint_and_steel"),
        ["commands.recipe.give.failed"]
    );
}

/// A custom executor's refusal goes through the same `handleError`
/// (CustomCommandExecutor.java:20-32): `runGuarded` reports the schedule before
/// the macro fails to instantiate (FunctionCommand.java:235-265), and only the
/// unforked call reports the failure behind it.
#[test]
fn a_forked_function_that_cannot_instantiate_reports_only_its_schedule() {
    let mut stage = stage(&[("test:macro", "$say $(missing)")]);
    let keys = |lines: Vec<String>| {
        lines
            .into_iter()
            .map(|line| line.split('(').next().unwrap_or_default().to_string())
            .collect::<Vec<_>>()
    };
    let forked =
        stage.run("execute as @e[type=armor_stand] run function test:macro with storage test:args");
    assert_eq!(
        keys(forked),
        ["commands.function.scheduled.single"].repeat(3)
    );
    assert_eq!(
        keys(stage.run("function test:macro with storage test:args")),
        [
            "commands.function.scheduled.single",
            "commands.function.instantiationFailure"
        ]
    );
}

#[test]
fn team_messages_reach_online_teammates_from_players_and_stored_entities() {
    let mut stage = stage(&[]);
    join_loaded(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        2,
        "bob",
        2,
    );
    join_loaded(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        3,
        "outside",
        3,
    );
    let stored = stage.stands[0];
    stage.run("team add red");
    for member in [
        "op".to_string(),
        "bob".to_string(),
        "offline".to_string(),
        stored.to_string(),
    ] {
        let lines = stage.run(&format!("team join red {member}"));
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with("commands.team.join.success")),
            "{lines:?}"
        );
    }
    stage.run(&format!(
        r#"data merge entity {stored} {{CustomName:{{text:"Messenger"}}}}"#
    ));
    stage.run("team modify red color red");
    stage.run(r#"team modify red displayName {text:"Red crew"}"#);
    stage.run(r#"team modify red prefix {text:"Crew-"}"#);
    for (command, outgoing) in [
        (
            "execute store result score #probe obj run teammsg hello @a[team=red] @z".to_string(),
            true,
        ),
        (
            format!(
                "execute store result score #probe obj as {stored} run tm hello @a[team=red] @z"
            ),
            false,
        ),
    ] {
        enqueue(
            &stage.sender,
            WorldCommand::Play {
                connection_id: ConnectionId(OP),
                packet: PlayServerbound::ChatCommand { command },
            },
        )
        .unwrap();
        let tick = stage.world.tick(stage.receiver.drain());
        assert_eq!(stage.score("#probe"), Some(2), "{:?}", tick.events);
        let messages = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id,
                    packet,
                } => shown_chat(&stage.world, packet).map(|shown| (*connection_id, shown)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(messages.len(), 2, "{messages:?}");
        for (recipient, message) in messages {
            assert!(recipient == ConnectionId(OP) || recipient == ConnectionId(2));
            let NbtSummaryValue::Compound(fields) = message.component() else {
                panic!("chat component");
            };
            let field = |name: &str| {
                &fields
                    .iter()
                    .find(|field| field.name == name)
                    .unwrap()
                    .value
            };
            let key = if outgoing && recipient == ConnectionId(OP) {
                "chat.type.team.sent"
            } else {
                "chat.type.team.text"
            };
            assert_eq!(field("translate"), &NbtSummaryValue::String(key.into()));
            let NbtSummaryValue::List(arguments) = field("with") else {
                panic!("chat arguments");
            };
            assert_eq!(arguments.len(), 3);
            let body =
                bbb_protocol::RichTextComponent::try_from_component_nbt(arguments[2].clone())
                    .unwrap();
            assert!(body.text().contains("Crew-op"), "{}", body.text());
            assert!(body.text().contains("Crew-bob"), "{}", body.text());
            assert!(body.text().ends_with(" @z"), "{}", body.text());
            let label =
                bbb_protocol::RichTextComponent::try_from_component_nbt(arguments[0].clone())
                    .unwrap();
            assert!(label.text().contains("Red crew"));
            let team = bbb_protocol::component_node_style_from_nbt_value(&arguments[0]);
            assert_eq!(
                team.color_name,
                Some(bbb_protocol::chat_formattings::ChatFormatting::Red)
            );
            assert_eq!(
                team.click_event,
                Some(bbb_protocol::ComponentClickEvent::SuggestCommand {
                    command: "/teammsg ".into()
                })
            );
            assert!(
                matches!(team.hover_event, Some(bbb_protocol::ComponentHoverEvent::ShowText { component: Some(NbtSummaryValue::Compound(ref fields)), .. })
                if fields.iter().any(|field| field.name == "translate" && field.value == NbtSummaryValue::String("chat.type.team.hover".into())))
            );
            let sender =
                bbb_protocol::RichTextComponent::try_from_component_nbt(arguments[1].clone())
                    .unwrap();
            assert!(
                sender.text().contains(if outgoing {
                    "Crew-op"
                } else {
                    "Crew-Messenger"
                }),
                "{}",
                sender.text()
            );
            assert!(sender.styled().iter().any(|run| matches!(
                run.style.hover_event,
                Some(bbb_protocol::ComponentHoverEvent::ShowEntity { .. })
            )));
        }
    }
}

#[test]
fn dynamic_command_text_reads_live_world_and_resolves_each_recipient() {
    let mut stage = stage(&[]);
    join_loaded(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        2,
        "bob",
        2,
    );
    stage.run("scoreboard players set op obj 7");
    stage.run("scoreboard players set bob obj 9");
    stage.run(r#"data merge storage test:text {word:"word",rich:{n:4,s:"leaf"},template:{text:"",extra:[]}}"#);
    stage.run(&format!(
        r#"data merge entity {} {{CustomName:{{text:"Named Stand"}}}}"#,
        stage.stands[0]
    ));
    stage.run(r#"setblock 1 64 1 chest{Items:[{Slot:0b,id:"minecraft:diamond",count:3}]}"#);
    for entry in [
        r#"{score:{name:"*",objective:"obj"}}"#,
        r#"{text:"|"}"#,
        r#"{selector:"@a",separator:{text:" / ",color:"gold"}}"#,
        r#"{text:"|"}"#,
        r#"{nbt:"word",storage:"test:text",plain:true}"#,
        r#"{text:"|"}"#,
        r#"{nbt:"rich",storage:"test:text"}"#,
        r#"{text:"|"}"#,
        r#"{nbt:"CustomName",entity:"@e[type=armor_stand,sort=nearest,limit=1]",interpret:true}"#,
        r#"{text:"|"}"#,
        r#"{nbt:"Items[0].count",block:"1 64 1",plain:true}"#,
    ] {
        stage.run(&format!(
            "data modify storage test:text template.extra append value {entry}"
        ));
    }
    stage.run(r#"data modify storage test:text template.hover_event set value {action:"show_text",value:{score:{name:"*",objective:"obj"}}}"#);
    let tellraw = chat_command(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        r#"tellraw @a {nbt:"template",storage:"test:text",interpret:true}"#,
    );
    let messages = tellraw
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SystemChat(chat),
            } => Some((*connection_id, &chat.content)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 2, "{:?}", tellraw.events);
    for (recipient, message) in messages {
        let score = if recipient == ConnectionId(OP) {
            "7"
        } else {
            "9"
        };
        assert!(
            message
                .text()
                .starts_with(&format!("{score}|op / bob|\"word\"|")),
            "{}",
            message.text()
        );
        assert!(message.text().contains("n: 4"), "{}", message.text());
        assert!(message.text().contains("s: \"leaf\""), "{}", message.text());
        assert!(
            message.text().ends_with("|Named Stand|3"),
            "{}",
            message.text()
        );
        assert!(message.styled().iter().any(|run| run.text == "4"
            && run.style.color_name == Some(bbb_protocol::chat_formattings::ChatFormatting::Gold)));
        assert!(message.styled().iter().any(|run| run.text == "leaf"
            && run.style.color_name
                == Some(bbb_protocol::chat_formattings::ChatFormatting::Green)));
        let style = bbb_protocol::component_node_style_from_nbt_value(message.component());
        let Some(bbb_protocol::ComponentHoverEvent::ShowText {
            component: Some(hover),
            ..
        }) = style.hover_event
        else {
            panic!("resolved hover text");
        };
        assert_eq!(
            bbb_protocol::RichTextComponent::try_from_component_nbt(hover)
                .unwrap()
                .text(),
            score
        );
    }
    let title = chat_command(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        OP,
        r#"title @a title {score:{name:"*",objective:"obj"}}"#,
    );
    let titles = title
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::SetTitleText(title),
            } => Some((*connection_id, title.content.text().to_string())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        titles,
        [
            (ConnectionId(OP), "7".into()),
            (ConnectionId(2), "9".into())
        ]
    );
    stage.run(r#"team add dynamic {selector:"@s"}"#);
    stage.run(r#"team modify dynamic prefix {score:{name:"*",objective:"obj"}}"#);
    let team = stage.world.scoreboard().team("dynamic").unwrap();
    assert_eq!(
        bbb_protocol::RichTextComponent::try_from_component_nbt(team.display_name.clone())
            .unwrap()
            .text(),
        "op"
    );
    assert_eq!(
        bbb_protocol::RichTextComponent::try_from_component_nbt(
            team.player_prefix.clone().unwrap()
        )
        .unwrap()
        .text(),
        "7"
    );
    stage.run(r#"scoreboard objectives add named dummy {selector:"@s"}"#);
    stage.run(r#"scoreboard players display name op obj {selector:"@s"}"#);
    assert_eq!(
        bbb_protocol::RichTextComponent::try_from_component_nbt(
            stage
                .world
                .scoreboard()
                .objective("named")
                .unwrap()
                .display_name
                .clone()
        )
        .unwrap()
        .text(),
        "op"
    );
    assert_eq!(
        bbb_protocol::RichTextComponent::try_from_component_nbt(
            stage
                .world
                .scoreboard()
                .score("op", "obj")
                .unwrap()
                .display
                .clone()
                .unwrap()
        )
        .unwrap()
        .text(),
        "op"
    );
    stage.run(r#"bossbar add test:dynamic {selector:"@s"}"#);
    assert_eq!(
        stage.world.command_boss_bars_snapshot_for_test()[0]
            .name
            .text(),
        "op"
    );
    stage.run("team join dynamic op");
    for command in ["say hello @p @z", "me greets @p", "msg bob hello @p"] {
        let tick = chat_command(
            &stage.sender,
            &stage.receiver,
            &mut stage.world,
            OP,
            command,
        );
        let messages = tick
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send { packet, .. } => shown_chat(&stage.world, packet),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!messages.is_empty(), "{command}");
        assert!(
            messages
                .iter()
                .all(|message| !message.text().contains("@p")),
            "{command}: {messages:?}"
        );
        assert!(
            messages.iter().all(|message| message.text().contains("op")),
            "{command}: {messages:?}"
        );
    }
    let unprivileged = chat_command(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        2,
        "msg op literal @p @z",
    );
    assert!(unprivileged.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(OP), packet }
        if shown_chat(&stage.world, packet)
            .is_some_and(|shown| shown.text().contains("literal @p @z")))));
}

#[test]
fn execute_on_forks_vehicle_passengers_and_controller_without_moving_the_origin() {
    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    let mut stage = stage(&[]);
    let (boat, passenger) = stage
        .world
        .spawn_boat_with_stored_passenger_for_test(
            DIM,
            Vec3d {
                x: 8.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .expect("a real boat passenger tree");
    assert!(stage
        .world
        .attach_external_player_for_test(ConnectionId(OP), boat));
    let boat_uuid = stage
        .world
        .stored_entity_for_test(DIM, boat)
        .unwrap()
        .uuid();
    let passenger_uuid = stage
        .world
        .stored_entity_for_test(DIM, passenger)
        .unwrap()
        .uuid();
    stage.run(&format!(
        "execute as {boat_uuid} on passengers run scoreboard players add @s obj 1"
    ));
    assert_eq!(stage.score("op"), Some(1));
    assert_eq!(stage.score(&passenger_uuid.to_string()), Some(1));
    stage.run(&format!(
        "execute as {boat_uuid} on passengers on vehicle run scoreboard players add @s obj 1"
    ));
    assert_eq!(stage.score(&boat_uuid.to_string()), Some(2));
    stage.run(&format!(
        "execute as {boat_uuid} on controller run scoreboard players add @s obj 1"
    ));
    assert_eq!(stage.score("op"), Some(2));
    stage.run("forceload add 30 30");
    stage.run(&format!("execute positioned 30 70 30 as {boat_uuid} on passengers run setblock ~ ~ ~ minecraft:gold_block"));
    assert_eq!(
        stage.measure("positioned 30 70 30 if block ~ ~ ~ minecraft:gold_block"),
        Some(1)
    );
    let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{vehicle:{type:'minecraft:oak_boat'}}}";
    stage.run("ride op dismount");
    assert_eq!(
        stage
            .world
            .external_player_vehicle_state_for_test(ConnectionId(OP))
            .2,
        None
    );
    for (tail, expected) in [
        (format!("if predicate {predicate}"), 0),
        (format!("unless predicate {predicate}"), 1),
    ] {
        stage.run("scoreboard players set #probe obj -7");
        assert_eq!(stage.score("#probe"), Some(-7));
        assert_eq!(stage.measure(&tail), Some(expected));
    }
    stage.run(&format!("ride op mount {boat_uuid}"));
    assert_eq!(
        stage
            .world
            .external_player_vehicle_state_for_test(ConnectionId(OP))
            .2,
        Some(boat)
    );
    for origin in ["", "in minecraft:the_nether "] {
        stage.run("scoreboard players set #probe obj -7");
        assert_eq!(stage.score("#probe"), Some(-7));
        assert_eq!(
            stage.measure(&format!("{origin}if predicate {predicate}")),
            Some(1)
        );
    }
    assert_eq!(
        stage
            .world
            .external_player_vehicle_state_for_test(ConnectionId(OP))
            .2,
        Some(boat)
    );
    stage.run("ride op dismount");
    assert_eq!(
        stage
            .world
            .external_player_vehicle_state_for_test(ConnectionId(OP))
            .2,
        None
    );
    for (tail, expected) in [
        (format!("if predicate {predicate}"), 0),
        (format!("unless predicate {predicate}"), 1),
    ] {
        stage.run("scoreboard players set #probe obj -7");
        assert_eq!(stage.score("#probe"), Some(-7));
        assert_eq!(stage.measure(&tail), Some(expected));
    }
}

#[test]
fn execute_on_reads_saved_owner_and_item_origin_and_skips_empty_relations() {
    use crate::entity::{EntityData, GeneralLivingData};
    use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_WOLF_ID};
    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    let mut stage = stage(&[]);
    stage.run("forceload add 0 0");
    let (wolf, wolf_uuid) = stage.world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 3.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_WOLF_ID).unwrap()),
    );
    let NbtSummaryValue::Compound(mut record) = stage
        .world
        .stored_entity_command_data_for_test(DIM, wolf)
        .unwrap()
    else {
        panic!("wolf record");
    };
    record.retain(|entry| entry.name != "Owner" && entry.name != "NoAI");
    record.push(NbtSummaryEntry {
        name: "Owner".into(),
        value: crate::entity::save::uuid_int_array(Uuid::from_u128(1)),
    });
    record.push(NbtSummaryEntry {
        name: "NoAI".into(),
        value: NbtSummaryValue::Byte(1),
    });
    record.push(NbtSummaryEntry {
        name: "leash".into(),
        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "UUID".into(),
            value: crate::entity::save::uuid_int_array(Uuid::from_u128(1)),
        }]),
    });
    stage
        .world
        .load_stored_entity_record_for_test(DIM, wolf, &record);
    let item = crate::entity::item::ItemEntityData::new(
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
        0.0,
    )
    .with_saved_fields(5, 0, 32767, Some(Uuid::from_u128(1)), None);
    let (_, item_uuid) = stage.world.spawn_stored_entity_for_test(
        DIM,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 3.5,
        },
        EntityData::Item(item),
    );
    stage.run(&format!(
        "execute in minecraft:the_nether as {wolf_uuid} on owner if dimension minecraft:the_nether run scoreboard players add @s obj 2"
    ));
    stage.run(&format!(
        "execute as {item_uuid} on origin run scoreboard players add @s obj 3"
    ));
    for _ in 0..100 {
        stage.run(&format!(
            "execute as {wolf_uuid} on leasher run scoreboard players add @s obj 1"
        ));
        if stage.score("op") == Some(6) {
            break;
        }
    }
    assert_eq!(
        stage.score("op"),
        Some(6),
        "leash={:?}",
        crate::entity::leash::leash_data(stage.world.stored_entity_for_test(DIM, wolf).unwrap())
    );
    stage.run(&format!(
        "execute as {} on owner run scoreboard players add #empty obj 1",
        stage.stands[0]
    ));
    stage.run("execute on vehicle run scoreboard players add #empty obj 1");
    assert_eq!(stage.score("#empty"), None);
}

#[test]
fn execute_on_interaction_target_and_attacker_bind_the_recorded_players() {
    use crate::entity::{interaction, EntityData};
    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
    let mut stage = stage(&[]);
    join_loaded(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        2,
        "visitor",
        2,
    );
    let (id, uuid) = stage.world.spawn_stored_entity_for_test(
        DIM,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_INTERACTION_ID,
        Vec3d {
            x: 4.5,
            y: 64.0,
            z: 3.5,
        },
        EntityData::Interaction(interaction::InteractionData::new()),
    );
    let body = stage.world.stored_entity_mut_for_test(DIM, id).unwrap();
    interaction::interact(body, Uuid::from_u128(1), 0);
    interaction::skip_attack_interaction(body, Uuid::from_u128(2), 0);
    stage.run(&format!(
        "execute as {uuid} on target run scoreboard players add @s obj 2"
    ));
    stage.run(&format!(
        "execute as {uuid} on attacker run scoreboard players add @s obj 3"
    ));
    assert_eq!(stage.score("op"), Some(2));
    assert_eq!(stage.score("visitor"), Some(3));
}

#[test]
fn execute_summon_binds_each_new_entity_from_existing_forks() {
    let mut stage = stage(&[]);
    let lines = stage.run(
        "execute as @e[type=armor_stand] at @s summon minecraft:pig run scoreboard players add @s obj 1",
    );
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines
        .iter()
        .all(|line| !line.starts_with("commands.summon.success")));
    let reads = stage.world.entity_reads();
    let pigs: Vec<_> = reads
        .stored_entities()
        .filter(|entry| {
            entry.entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID
        })
        .collect();
    assert_eq!(pigs.len(), 3);
    let mut positions: Vec<_> = pigs
        .iter()
        .map(|entry| {
            assert_eq!(stage.score(&entry.entity.uuid().to_string()), Some(1));
            entry.entity.position().x
        })
        .collect();
    positions.sort_by(f64::total_cmp);
    assert_eq!(positions, [2.5, 4.5, 6.5]);
}

#[test]
fn execute_summon_retains_origin_rotation_and_anchor_and_reports_failure() {
    let mut stage = stage(&[("bbb:summoned", "tag @s add born\nexecute at @s run setblock ~ ~ ~ minecraft:gold_block\nsetblock ^ ^ ^1 minecraft:diamond_block")]);
    stage.run("forceload add 20 20");
    let lines = stage.run("execute positioned 20.5 70.3 20.5 rotated 90 0 anchored eyes summon minecraft:armor_stand run function bbb:summoned");
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("commands.summon.success")),
        "{lines:?}"
    );
    assert_eq!(
        stage.measure("if block 20 70 20 minecraft:gold_block"),
        Some(1)
    );
    assert_eq!(
        stage.measure("if block 19 72 20 minecraft:diamond_block"),
        Some(1)
    );
    {
        let reads = stage.world.entity_reads();
        let entity = reads
            .stored_entities()
            .find(|entry| entry.entity.entity_tags().iter().any(|tag| tag == "born"))
            .expect("the summoned executor received its tag");
        assert_eq!(entity.entity.y_rot(), 0.0);
    }
    for (resource_id, score) in [
        ("minecraft:acacia_boat", "#boat"),
        ("minecraft:leash_knot", "#leash"),
    ] {
        stage.run(&format!(
            "execute summon {resource_id} run scoreboard players add {score} obj 1"
        ));
        assert_eq!(stage.score(score), Some(1));
    }
    assert_eq!(
        stage.run("execute summon minecraft:item run scoreboard players add #failed obj 1"),
        ["commands.summon.failed"]
    );
    assert_eq!(stage.score("#failed"), None);
}
