use std::collections::{BTreeMap, BTreeSet};

use crate::advancement::{Advancement, AdvancementSet};
use bbb_protocol::{content_registry::ContentRegistry, JavaString};
use bbb_registry_codec::RegistryJsonValue;

use super::display::{self, AdvancementDisplay};
use super::layout;
use super::requirements::AdvancementRequirements;

/// `AdvancementNode` (`AdvancementNode.java:8-17`) with its parent link
/// resolved to an index into the owning [`AdvancementTree`].
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementNode {
    id: String,
    parent: Option<usize>,
    children: Vec<usize>,
    root: usize,
    display: Option<AdvancementDisplay>,
    criteria: Vec<(JavaString, String)>,
    requirements: AdvancementRequirements,
    sends_telemetry_event: bool,
}

impl AdvancementNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    pub fn children(&self) -> &[usize] {
        &self.children
    }

    /// `root` (`AdvancementNode.java:31-46`).
    pub fn root(&self) -> usize {
        self.root
    }

    /// `Advancement.isRoot` (`Advancement.java:101-103`).
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    pub fn display(&self) -> Option<&AdvancementDisplay> {
        self.display.as_ref()
    }

    /// The criteria map's keys with each criterion's trigger id, the pair
    /// `Criterion.CODEC` dispatches on (`Criterion.java:8-10`).
    pub fn criteria(&self) -> &[(JavaString, String)] {
        &self.criteria
    }

    pub fn requirements(&self) -> &AdvancementRequirements {
        &self.requirements
    }

    /// `Advancement.sendsTelemetryEvent` (`Advancement.java:46`).
    pub fn sends_telemetry_event(&self) -> bool {
        self.sends_telemetry_event
    }
}

/// One advancement as `Advancement.write` puts it on the wire
/// (`Advancement.java:83-88`), which is what
/// `ClientboundUpdateAdvancementsPacket`'s added set carries.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementSummary {
    pub id: String,
    pub parent: Option<String>,
    pub display: Option<AdvancementDisplay>,
    pub requirements: AdvancementRequirements,
    pub sends_telemetry_event: bool,
}

/// One advancement `AdvancementTree.addAll` kept out of the runtime tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancementTreeWarning {
    pub id: String,
    pub message: String,
}

/// `AdvancementTree` (`AdvancementTree.java:16-20`) over one loaded datapack
/// generation: the parent/child graph `PlayerAdvancements` walks, laid out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdvancementTree {
    nodes: Vec<AdvancementNode>,
    by_id: BTreeMap<String, usize>,
    roots: Vec<usize>,
    warnings: Vec<AdvancementTreeWarning>,
}

impl AdvancementTree {
    /// `AdvancementTree.addAll` (`:54-93`) over the loaded set, which the
    /// datapack loader has already pruned of orphans.
    ///
    /// A definition the rest of `Advancement.CODEC` refuses -- a requirement
    /// grid that does not name exactly its criteria (`Advancement.java:53`,
    /// `:67-69`) or a malformed `display` -- never reaches vanilla's tree and
    /// its children become orphans. These fields remain source-backed until
    /// this tree validates them, so rejection cascades here.
    pub fn from_loaded(loaded: &AdvancementSet) -> Self {
        Self::from_loaded_with_content_registry(loaded, ContentRegistry::vanilla())
    }

    /// Content-registry-aware form of [`Self::from_loaded`].
    pub fn from_loaded_with_content_registry(
        loaded: &AdvancementSet,
        content_registry: &ContentRegistry,
    ) -> Self {
        let mut warnings = Vec::new();
        let mut decoded = BTreeMap::new();
        let mut rejected = BTreeSet::new();
        for element in &loaded.advancements {
            match decode_advancement(&element.advancement, content_registry) {
                Ok(advancement) => {
                    decoded.insert(element.id.as_str(), advancement);
                }
                Err(message) => {
                    rejected.insert(element.id.as_str());
                    warnings.push(AdvancementTreeWarning {
                        id: element.id.clone(),
                        message,
                    });
                }
            }
        }
        loop {
            let cascade: Vec<&str> = loaded
                .advancements
                .iter()
                .filter(|element| !rejected.contains(element.id.as_str()))
                .filter(|element| {
                    element
                        .advancement
                        .parent
                        .as_deref()
                        .is_some_and(|parent| rejected.contains(parent))
                })
                .map(|element| element.id.as_str())
                .collect();
            if cascade.is_empty() {
                break;
            }
            for id in cascade {
                rejected.insert(id);
                warnings.push(AdvancementTreeWarning {
                    id: id.to_string(),
                    message: "parent left the advancement tree".to_string(),
                });
            }
        }

        let mut tree = Self {
            warnings,
            ..Self::default()
        };
        // `addAll`'s `removeIf(this::tryInsert)` rounds: a node joins once its
        // parent is in, which is what makes the parent index resolvable here.
        let mut pending: Vec<_> = loaded
            .advancements
            .iter()
            .filter(|element| !rejected.contains(element.id.as_str()))
            .collect();
        while !pending.is_empty() {
            let mut deferred = Vec::with_capacity(pending.len());
            let mut inserted = false;
            for element in pending {
                let parent = match element.advancement.parent.as_deref() {
                    Some(parent) => match tree.by_id.get(parent) {
                        Some(index) => Some(*index),
                        None => {
                            deferred.push(element);
                            continue;
                        }
                    },
                    None => None,
                };
                let index = tree.nodes.len();
                let root = parent.map_or(index, |parent| tree.nodes[parent].root);
                if let Some(parent) = parent {
                    tree.nodes[parent].children.push(index);
                } else {
                    tree.roots.push(index);
                }
                let advancement = &decoded[element.id.as_str()];
                tree.nodes.push(AdvancementNode {
                    id: element.id.clone(),
                    parent,
                    children: Vec::new(),
                    root,
                    display: advancement.display.clone(),
                    criteria: advancement.criteria.clone(),
                    requirements: advancement.requirements.clone(),
                    sends_telemetry_event: advancement.sends_telemetry_event,
                });
                tree.by_id.insert(element.id.clone(), index);
                inserted = true;
            }
            if !inserted {
                break;
            }
            pending = deferred;
        }
        tree.position_roots();
        tree
    }

    /// `ServerAdvancementManager.apply` (`ServerAdvancementManager.java:44-48`)
    /// positions each root that has a display, once per reload.
    ///
    /// Vanilla keeps a node's children in a `ReferenceOpenHashSet`
    /// (`AdvancementNode.java:11`), so the sibling order the walk lays out is
    /// identity-hash order and the coordinates it produces differ between runs
    /// of the same server. This tree keeps children in insertion order instead,
    /// which is one of the orders vanilla itself can produce.
    fn position_roots(&mut self) {
        for root in self.roots.clone() {
            if self.nodes[root].display.is_none() {
                continue;
            }
            for (index, x, y) in layout::run(self, root) {
                if let Some(display) = self.nodes[index].display.as_mut() {
                    display.x = x;
                    display.y = y;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// `get(Identifier)` (`:112-114`).
    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    pub fn node(&self, index: usize) -> &AdvancementNode {
        &self.nodes[index]
    }

    /// `roots` (`:104-106`).
    pub fn roots(&self) -> &[usize] {
        &self.roots
    }

    /// The node as the wire carries it, with the parent link named again.
    pub fn summary(&self, index: usize) -> AdvancementSummary {
        let node = self.node(index);
        AdvancementSummary {
            id: node.id.clone(),
            parent: node.parent.map(|parent| self.nodes[parent].id.clone()),
            display: node.display.clone(),
            requirements: node.requirements.clone(),
            sends_telemetry_event: node.sends_telemetry_event,
        }
    }

    pub fn warnings(&self) -> &[AdvancementTreeWarning] {
        &self.warnings
    }
}

/// The fields of `Advancement.CODEC` (`Advancement.java:41-52`) this tree
/// keeps, all decoded before the advancement may join it.
struct DecodedAdvancement {
    display: Option<AdvancementDisplay>,
    criteria: Vec<(JavaString, String)>,
    requirements: AdvancementRequirements,
    sends_telemetry_event: bool,
}

fn decode_advancement(
    advancement: &Advancement,
    content_registry: &ContentRegistry,
) -> Result<DecodedAdvancement, String> {
    let names: BTreeSet<&JavaString> = advancement
        .criteria
        .iter()
        .map(|criterion| &criterion.name)
        .collect();
    Ok(DecodedAdvancement {
        display: advancement
            .display
            .as_ref()
            .map(|display| display::decode_with_content_registry(display, content_registry))
            .transpose()?,
        criteria: advancement
            .criteria
            .iter()
            .map(|criterion| (criterion.name.clone(), criterion.trigger.clone()))
            .collect(),
        requirements: decode_requirements(advancement.requirements.as_ref(), &names)?,
        sends_telemetry_event: advancement.sends_telemetry_event,
    })
}

/// `Advancement.CODEC`'s requirement arm (`Advancement.java:45`, `:49`): the
/// declared grid when there is one, `allOf(criteria)` when there is not, and
/// either way validated against the criterion names.
fn decode_requirements(
    declared: Option<&RegistryJsonValue>,
    criteria: &BTreeSet<&JavaString>,
) -> Result<AdvancementRequirements, String> {
    let requirements = match declared {
        Some(value) => AdvancementRequirements::parse(value)?,
        None => AdvancementRequirements::all_of(criteria.iter().map(|name| (*name).clone())),
    };
    requirements.validate(criteria)?;
    Ok(requirements)
}
