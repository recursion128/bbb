use bbb_protocol::{component_literal_nbt, packets::CommandNodeType};

use super::{ArgumentValue, CommandRun, Outcome, ParseResult};
use crate::commands::{catalog, parse, translatable, CommandPermission, CommandSource};
use crate::world::ServerWorld;

pub(super) fn help(
    run: &mut CommandRun,
    command_parse: &ParseResult,
    source: &CommandSource,
    world: &ServerWorld,
) -> Outcome {
    // HelpCommand.java:17-51.
    let (node, prefix) = match command_parse.argument("command") {
        None => (
            bbb_protocol::packets::VANILLA_COMMAND_TREE_ROOT,
            bbb_protocol::JavaString::from("/"),
        ),
        Some(ArgumentValue::Text(command)) => {
            let parsed = parse::parse(command.units(), source.permission, world);
            let Some(last) = parsed.nodes.last() else {
                return Err(translatable("commands.help.failed", Vec::new()));
            };
            (
                last.index,
                std::iter::once(u16::from(b'/'))
                    .chain(command.units().iter().copied())
                    .chain([u16::from(b' ')])
                    .collect::<Vec<_>>()
                    .into(),
            )
        }
        _ => unreachable!("help command is a greedy brigadier string"),
    };

    let usage = smart_usage(node, source.permission);
    for line in &usage {
        let mut text = prefix.clone();
        text.push_str(line);
        run.succeed(component_literal_nbt(text), false);
    }
    Ok(i32::try_from(usage.len()).expect("the command tree fits in i32"))
}

fn smart_usage(node: u16, permission: CommandPermission) -> Vec<String> {
    let optional = catalog::node(node).executable;
    catalog::usable_children(node, permission)
        .map(|child| usage(child, permission, optional, false))
        .collect()
}

fn usage(node: u16, permission: CommandPermission, optional: bool, deep: bool) -> String {
    let current = catalog::node(node);
    let own_usage = usage_text(node);
    let own_usage = if optional {
        format!("[{own_usage}]")
    } else {
        own_usage
    };

    if deep {
        return own_usage;
    }

    if let Some(redirect) = catalog::redirect_target(node) {
        let redirect = if redirect == bbb_protocol::packets::VANILLA_COMMAND_TREE_ROOT {
            "...".to_string()
        } else {
            format!("->{}", usage_text(redirect))
        };
        return format!("{own_usage} {redirect}");
    }

    let child_optional = current.executable;
    let children = catalog::usable_children(node, permission).collect::<Vec<_>>();
    if children.len() == 1 {
        let child = usage(children[0], permission, child_optional, child_optional);
        return format!("{own_usage} {child}");
    }
    if children.len() > 1 {
        let mut recursive = Vec::new();
        for child in &children {
            let child_usage = usage(*child, permission, child_optional, true);
            if !recursive.contains(&child_usage) {
                recursive.push(child_usage);
            }
        }
        if recursive.len() == 1 {
            let child = &recursive[0];
            return if child_optional {
                format!("{own_usage} [{child}]")
            } else {
                format!("{own_usage} {child}")
            };
        }
        if recursive.len() > 1 {
            let alternatives = children
                .iter()
                .map(|child| usage_text(*child))
                .collect::<Vec<_>>()
                .join("|");
            let (open, close) = if child_optional {
                ('[', ']')
            } else {
                ('(', ')')
            };
            return format!("{own_usage} {open}{alternatives}{close}");
        }
    }

    own_usage
}

fn usage_text(node: u16) -> String {
    let node = catalog::node(node);
    match node.kind {
        CommandNodeType::Argument => format!("<{}>", node.name),
        CommandNodeType::Literal => node.name.to_string(),
        CommandNodeType::Root => String::new(),
    }
}
