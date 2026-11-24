//! `TreeNodePosition` (`net/minecraft/advancements/TreeNodePosition.java`): the
//! Buchheim walk that gives every displayed advancement under a displayed root
//! the `DisplayInfo` location the client draws its tree from.

use super::tree::AdvancementTree;

/// One `TreeNodePosition` (`TreeNodePosition.java:8-19`), with every link an
/// index into the arena the walk allocates. Vanilla's `mod` field is spelled
/// `modifier` here.
struct Position {
    node: usize,
    parent: Option<usize>,
    previous_sibling: Option<usize>,
    child_index: i32,
    children: Vec<usize>,
    ancestor: usize,
    thread: Option<usize>,
    x: i32,
    y: f32,
    modifier: f32,
    change: f32,
    shift: f32,
}

struct Layout<'a> {
    tree: &'a AdvancementTree,
    positions: Vec<Position>,
}

impl<'a> Layout<'a> {
    /// The constructor (`:21-44`), which builds the whole displayed subtree.
    fn insert(
        &mut self,
        node: usize,
        parent: Option<usize>,
        previous_sibling: Option<usize>,
        child_index: i32,
        depth: i32,
    ) -> usize {
        let index = self.positions.len();
        self.positions.push(Position {
            node,
            parent,
            previous_sibling,
            child_index,
            children: Vec::new(),
            ancestor: index,
            thread: None,
            x: depth,
            y: -1.0,
            modifier: 0.0,
            change: 0.0,
            shift: 0.0,
        });
        let tree = self.tree;
        let mut previous = None;
        for child in tree.node(node).children() {
            previous = self.add_child(*child, index, previous);
        }
        index
    }

    /// `addChild` (`:46-57`): an advancement with no display is not positioned,
    /// and its children attach to the nearest displayed ancestor instead.
    fn add_child(
        &mut self,
        node: usize,
        parent: usize,
        mut previous: Option<usize>,
    ) -> Option<usize> {
        let tree = self.tree;
        if tree.node(node).display().is_some() {
            let child_index = self.positions[parent].children.len() as i32 + 1;
            let depth = self.positions[parent].x + 1;
            let child = self.insert(node, Some(parent), previous, child_index, depth);
            self.positions[parent].children.push(child);
            return Some(child);
        }
        for grandchild in tree.node(node).children() {
            previous = self.add_child(*grandchild, parent, previous);
        }
        previous
    }

    /// `firstWalk` (`:59-83`).
    fn first_walk(&mut self, index: usize) {
        if self.positions[index].children.is_empty() {
            self.positions[index].y = match self.positions[index].previous_sibling {
                Some(previous) => self.positions[previous].y + 1.0,
                None => 0.0,
            };
            return;
        }
        let mut default_ancestor = None;
        for slot in 0..self.positions[index].children.len() {
            let child = self.positions[index].children[slot];
            self.first_walk(child);
            default_ancestor = Some(self.apportion(child, default_ancestor.unwrap_or(child)));
        }
        self.execute_shifts(index);
        let first = self.positions[index].children[0];
        let last = *self.positions[index]
            .children
            .last()
            .expect("the child list was checked non-empty");
        let midpoint = (self.positions[first].y + self.positions[last].y) / 2.0;
        match self.positions[index].previous_sibling {
            Some(previous) => {
                let y = self.positions[previous].y + 1.0;
                self.positions[index].y = y;
                self.positions[index].modifier = y - midpoint;
            }
            None => self.positions[index].y = midpoint,
        }
    }

    /// `secondWalk` (`:85-97`), answering the lowest position it reached.
    fn second_walk(&mut self, index: usize, modifier_sum: f32, depth: i32, mut min: f32) -> f32 {
        self.positions[index].y += modifier_sum;
        self.positions[index].x = depth;
        if self.positions[index].y < min {
            min = self.positions[index].y;
        }
        let modifier = self.positions[index].modifier;
        for slot in 0..self.positions[index].children.len() {
            let child = self.positions[index].children[slot];
            min = self.second_walk(child, modifier_sum + modifier, depth + 1, min);
        }
        min
    }

    /// `thirdWalk` (`:99-105`).
    fn third_walk(&mut self, index: usize, offset: f32) {
        self.positions[index].y += offset;
        for slot in 0..self.positions[index].children.len() {
            let child = self.positions[index].children[slot];
            self.third_walk(child, offset);
        }
    }

    /// `executeShifts` (`:107-118`).
    fn execute_shifts(&mut self, index: usize) {
        let mut shift = 0.0;
        let mut change = 0.0;
        for slot in (0..self.positions[index].children.len()).rev() {
            let child = self.positions[index].children[slot];
            self.positions[child].y += shift;
            self.positions[child].modifier += shift;
            change += self.positions[child].change;
            shift += self.positions[child].shift + change;
        }
    }

    /// `previousOrThread` (`:120-126`).
    fn previous_or_thread(&self, index: usize) -> Option<usize> {
        self.positions[index]
            .thread
            .or_else(|| self.positions[index].children.first().copied())
    }

    /// `nextOrThread` (`:128-134`).
    fn next_or_thread(&self, index: usize) -> Option<usize> {
        self.positions[index]
            .thread
            .or_else(|| self.positions[index].children.last().copied())
    }

    /// `apportion` (`:136-182`), where `inside`/`outside` are vanilla's
    /// `vi*`/`vo*` contour cursors and their running modifier sums.
    fn apportion(&mut self, index: usize, mut default_ancestor: usize) -> usize {
        let Some(previous_sibling) = self.positions[index].previous_sibling else {
            return default_ancestor;
        };
        let parent = self.positions[index]
            .parent
            .expect("a node with a previous sibling has a parent");
        let mut inside_right = index;
        let mut outside_right = index;
        let mut inside_left = previous_sibling;
        let mut outside_left = self.positions[parent].children[0];
        let mut inside_right_sum = self.positions[index].modifier;
        let mut outside_right_sum = self.positions[index].modifier;
        let mut inside_left_sum = self.positions[inside_left].modifier;
        let mut outside_left_sum = self.positions[outside_left].modifier;
        while let (Some(next), Some(previous)) = (
            self.next_or_thread(inside_left),
            self.previous_or_thread(inside_right),
        ) {
            inside_left = next;
            inside_right = previous;
            // The outer cursors walk the same two contours as the inner ones one
            // level at a time, which is why vanilla dereferences them straight
            // after advancing them (`:152-155`).
            outside_left = self
                .previous_or_thread(outside_left)
                .expect("the left outer contour is as deep as the left inner one");
            outside_right = self
                .next_or_thread(outside_right)
                .expect("the right outer contour is as deep as the right inner one");
            self.positions[outside_right].ancestor = index;
            let shift = self.positions[inside_left].y + inside_left_sum
                - (self.positions[inside_right].y + inside_right_sum)
                + 1.0;
            if shift > 0.0 {
                let ancestor = self.ancestor_of(inside_left, index, default_ancestor);
                self.move_subtree(ancestor, index, shift);
                inside_right_sum += shift;
                outside_right_sum += shift;
            }
            inside_left_sum += self.positions[inside_left].modifier;
            inside_right_sum += self.positions[inside_right].modifier;
            outside_left_sum += self.positions[outside_left].modifier;
            outside_right_sum += self.positions[outside_right].modifier;
        }
        if self.next_or_thread(inside_left).is_some()
            && self.next_or_thread(outside_right).is_none()
        {
            self.positions[outside_right].thread = self.next_or_thread(inside_left);
            self.positions[outside_right].modifier += inside_left_sum - outside_right_sum;
        } else {
            if self.previous_or_thread(inside_right).is_some()
                && self.previous_or_thread(outside_left).is_none()
            {
                self.positions[outside_left].thread = self.previous_or_thread(inside_right);
                self.positions[outside_left].modifier += inside_right_sum - outside_left_sum;
            }
            default_ancestor = index;
        }
        default_ancestor
    }

    /// `moveSubtree` (`:184-194`).
    fn move_subtree(&mut self, left: usize, right: usize, shift: f32) {
        let subtrees =
            (self.positions[right].child_index - self.positions[left].child_index) as f32;
        if subtrees != 0.0 {
            self.positions[right].change -= shift / subtrees;
            self.positions[left].change += shift / subtrees;
        }
        self.positions[right].shift += shift;
        self.positions[right].y += shift;
        self.positions[right].modifier += shift;
    }

    /// `getAncestor` (`:196-198`), whose `contains` is reference identity.
    fn ancestor_of(&self, index: usize, other: usize, default_ancestor: usize) -> usize {
        let ancestor = self.positions[index].ancestor;
        let parent = self.positions[other]
            .parent
            .expect("apportion only reaches this for a node with a previous sibling");
        if self.positions[parent].children.contains(&ancestor) {
            ancestor
        } else {
            default_ancestor
        }
    }
}

/// `TreeNodePosition.run` (`:209-222`) over one root the caller has already
/// checked has a display, answering the `setLocation` (`DisplayInfo.java:61-64`)
/// each positioned node is owed.
///
/// `finalizePosition` (`:200-207`) descends the same child links the arena was
/// built from, so every entry it reaches is one entry of the arena.
pub(super) fn run(tree: &AdvancementTree, root: usize) -> Vec<(usize, f32, f32)> {
    let mut layout = Layout {
        tree,
        positions: Vec::new(),
    };
    let root_position = layout.insert(root, None, None, 1, 0);
    layout.first_walk(root_position);
    let start = layout.positions[root_position].y;
    let min = layout.second_walk(root_position, 0.0, 0, start);
    if min < 0.0 {
        layout.third_walk(root_position, -min);
    }
    layout
        .positions
        .iter()
        .map(|position| (position.node, position.x as f32, position.y))
        .collect()
}
