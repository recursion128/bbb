//! The command tree a session is told about.
//!
//! `Commands.sendCommands` (Commands.java:411-417) hands the client the
//! dispatcher root filtered through `fillUsableCommands` (:419-437): every
//! child the source `canUse` is rebuilt into a fresh tree and every child it
//! may not use is left out altogether, so an unpermitted command is absent
//! rather than flagged — the same rule `super::parse` applies when it walks
//! `catalog::usable_children` instead of the whole child list, which is what
//! makes an unpermitted command unparseable rather than refused.
//!
//! The shape is the derived tree's, node for node: the subtree under every root
//! literal this server implements is published exactly as the probed official
//! server built it, including the argument parsers, their serialized
//! properties, the suggestion providers and the redirects. The only thing this
//! module decides is which root literals appear at all, which is
//! `catalog::IMPLEMENTED_COMMANDS`.

use std::sync::OnceLock;

use bbb_protocol::packets::{
    CommandArgumentParser, CommandNode, CommandNodeType, Commands, VANILLA_COMMAND_TREE_ROOT,
};
use rustc_hash::FxHashMap;

use super::properties::argument_template;
use super::{catalog, CommandPermission};

fn permission_slot(permission: CommandPermission) -> usize {
    match permission {
        CommandPermission::All => 0,
        CommandPermission::Moderator => 1,
        CommandPermission::GameMaster => 2,
        CommandPermission::Admin => 3,
        CommandPermission::Owner => 4,
    }
}

/// The tree `Commands.sendCommands` would build for a source holding
/// `permission`.
pub fn usable_command_tree(permission: CommandPermission) -> Commands {
    static CACHED: [OnceLock<Commands>; 5] = [
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
        OnceLock::new(),
    ];
    CACHED[permission_slot(permission)]
        .get_or_init(|| build_usable_command_tree(permission))
        .clone()
}

fn build_usable_command_tree(permission: CommandPermission) -> Commands {
    // `enumerateNodes` (ClientboundCommandsPacket.java:82-98) numbers the tree
    // breadth-first from the root, so the root is index 0 and a node is
    // numbered before any of its own children.
    let mut order = vec![VANILLA_COMMAND_TREE_ROOT];
    let mut wire_index = FxHashMap::default();
    wire_index.insert(VANILLA_COMMAND_TREE_ROOT, 0i32);
    let mut cursor = 0;
    while cursor < order.len() {
        for child in catalog::usable_children(order[cursor], permission) {
            wire_index.insert(child, order.len() as i32);
            order.push(child);
        }
        cursor += 1;
    }

    let nodes = order
        .iter()
        .map(|index| wire_node(*index, permission, &wire_index))
        .collect();
    Commands {
        nodes,
        root_index: 0,
    }
}

fn wire_node(
    index: u16,
    permission: CommandPermission,
    wire_index: &FxHashMap<u16, i32>,
) -> CommandNode {
    let derived = catalog::node(index);
    let children = catalog::usable_children(index, permission)
        .filter_map(|child| wire_index.get(&child).copied())
        .collect::<Vec<_>>();
    let mut node = match derived.kind {
        CommandNodeType::Root => CommandNode::root(),
        CommandNodeType::Literal => CommandNode::literal(derived.name),
        CommandNodeType::Argument => CommandNode::argument(
            derived.name,
            CommandArgumentParser::from_template(
                derived
                    .parser_type_id()
                    .expect("an argument node names a registered parser"),
                &argument_template(derived),
            )
            .expect("the derived properties describe the parser they belong to"),
        ),
    }
    .with_children(children);

    // The redirect and the suggestion provider are fields rather than builder
    // steps, so setting them leaves the flag byte behind; it is reprojected once
    // below, after everything the node carries is on it.
    if let Some(redirect) = catalog::redirect_target(index) {
        if let Some(target) = wire_index.get(&redirect) {
            node.redirect = Some(*target);
        }
    }
    if let Some(suggestions) = derived.suggestions {
        node.suggestions = Some(suggestions.to_string());
    }
    if derived.executable {
        node = node.executes();
    }
    // `NodeInspector.isRestricted` (Commands.java:182-185) asks whether the
    // node's own requirement rejects a source holding no permission at all,
    // which the derived tree records per node.
    if derived.restricted() {
        node = node.restrict();
    }
    node.flags = projected_flags(&node);
    node
}

/// The flag byte [`CommandNode`]'s own fields project to.
///
/// `createEntry` (ClientboundCommandsPacket.java:146-180) derives every bit of
/// that byte from the node in front of it, and `bbb_protocol` owns the same
/// projection: each builder [`CommandNode`] offers rebuilds the byte, and its
/// encoder refuses a node whose stored byte disagrees with its fields. The
/// redirect target (:148-150) and the suggestion provider (:173-175) are the two
/// the builders do not cover, being plain fields — so rather than spelling their
/// bits a second time here, the byte is read back off the projection that owns
/// them. `executes()` is what reruns it; the one bit that call forces on is the
/// same bit it forces on a bare probe, so it can be taken back off a node that
/// does not carry a command.
fn projected_flags(node: &CommandNode) -> u8 {
    let probe = CommandNode::literal("probe");
    let executable_flag = probe.clone().executes().flags ^ probe.flags;
    let rebuilt = node.clone().executes().flags;
    if node.executable {
        rebuilt
    } else {
        rebuilt & !executable_flag
    }
}
