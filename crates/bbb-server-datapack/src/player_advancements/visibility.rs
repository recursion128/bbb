use super::tree::AdvancementTree;

/// `AdvancementVisibilityEvaluator.VISIBILITY_DEPTH` (`:12`).
const VISIBILITY_DEPTH: usize = 2;

/// `AdvancementVisibilityEvaluator.VisibilityRule` (`:79-83`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibilityRule {
    Show,
    Hide,
    NoChange,
}

/// `evaluateVisibility` (`:61-72`): walk the node's whole root tree, reporting
/// each node's visibility to `output`.
pub fn evaluate_visibility(
    tree: &AdvancementTree,
    node: usize,
    is_done: &mut impl FnMut(usize) -> bool,
    output: &mut impl FnMut(usize, bool),
) {
    let root = tree.node(node).root();
    let mut ascendants = vec![VisibilityRule::NoChange; VISIBILITY_DEPTH + 1];
    evaluate_node(tree, root, &mut ascendants, is_done, output);
}

/// `evaluateVisibility` (`:40-59`), which answers whether the node or any
/// descendant is done.
fn evaluate_node(
    tree: &AdvancementTree,
    node: usize,
    ascendants: &mut Vec<VisibilityRule>,
    is_done: &mut impl FnMut(usize) -> bool,
    output: &mut impl FnMut(usize, bool),
) -> bool {
    let is_self_done = is_done(node);
    ascendants.push(visibility_rule(tree, node, is_self_done));
    let mut is_self_or_descendant_done = is_self_done;
    for &child in tree.node(node).children() {
        is_self_or_descendant_done |= evaluate_node(tree, child, ascendants, is_done, output);
    }
    let visible = is_self_or_descendant_done || unfinished_node_is_visible(ascendants);
    ascendants.pop();
    output(node, visible);
    is_self_or_descendant_done
}

/// `evaluateVisibilityRule` (`:14-23`).
fn visibility_rule(tree: &AdvancementTree, node: usize, is_done: bool) -> VisibilityRule {
    match tree.node(node).display() {
        None => VisibilityRule::Hide,
        Some(_) if is_done => VisibilityRule::Show,
        Some(display) if display.hidden => VisibilityRule::Hide,
        Some(_) => VisibilityRule::NoChange,
    }
}

/// `evaluateVisiblityForUnfinishedNode` (`:25-38`): the nearest decided rule
/// within the node's own rule and its two ascendants.
fn unfinished_node_is_visible(ascendants: &[VisibilityRule]) -> bool {
    for depth in 0..=VISIBILITY_DEPTH {
        match ascendants[ascendants.len() - 1 - depth] {
            VisibilityRule::Show => return true,
            VisibilityRule::Hide => return false,
            VisibilityRule::NoChange => {}
        }
    }
    false
}
