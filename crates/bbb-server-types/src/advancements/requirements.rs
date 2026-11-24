use std::collections::BTreeSet;

use bbb_protocol::JavaString;
use bbb_registry_codec::RegistryJsonValue;

/// `AdvancementRequirements` (`AdvancementRequirements.java:13-18`): the
/// conjunction of criterion-name disjunctions an advancement is done by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdvancementRequirements {
    groups: Vec<Vec<JavaString>>,
}

impl AdvancementRequirements {
    pub fn new(groups: Vec<Vec<JavaString>>) -> Self {
        Self { groups }
    }

    /// `allOf` (`:28-30`), which `Advancement.CODEC` applies over the criterion
    /// names when the definition declares no `requirements` (`Advancement.java:49`).
    pub fn all_of<I>(criteria: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<JavaString>,
    {
        Self {
            groups: criteria.into_iter().map(|name| vec![name.into()]).collect(),
        }
    }

    pub fn groups(&self) -> &[Vec<JavaString>] {
        &self.groups
    }

    /// `size` (`:36-38`), the denominator of the progress text.
    pub fn size(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// `names` (`:107-115`).
    pub fn names(&self) -> BTreeSet<&JavaString> {
        self.groups.iter().flatten().collect()
    }

    /// `test` (`:40-52`): every group needs one satisfied name, and an empty
    /// requirement list is never satisfied.
    pub fn test(&self, mut predicate: impl FnMut(&JavaString) -> bool) -> bool {
        !self.groups.is_empty()
            && self
                .groups
                .iter()
                .all(|group| group.iter().any(&mut predicate))
    }

    /// `count` (`:54-64`).
    pub fn count(&self, mut predicate: impl FnMut(&JavaString) -> bool) -> usize {
        self.groups
            .iter()
            .filter(|group| group.iter().any(&mut predicate))
            .count()
    }

    /// `validate` (`:76-96`), whose failure takes the whole advancement out of
    /// `Advancement.CODEC` (`Advancement.java:53`, `:67-69`).
    pub fn validate(&self, expected: &BTreeSet<&JavaString>) -> Result<(), String> {
        let mut referenced = BTreeSet::new();
        for group in &self.groups {
            if group.is_empty() && expected.is_empty() {
                return Err("Requirement entry cannot be empty".to_string());
            }
            referenced.extend(group.iter());
        }
        if &referenced == expected {
            return Ok(());
        }
        let missing: Vec<&JavaString> = expected.difference(&referenced).copied().collect();
        let unknown: Vec<&JavaString> = referenced.difference(expected).copied().collect();
        Err(format!(
            "Advancement completion requirements did not exactly match specified criteria. Missing: {missing:?}. Unknown: {unknown:?}"
        ))
    }

    /// `AdvancementRequirements.CODEC` (`:14-17`): a list of lists of strings.
    pub fn parse(value: &RegistryJsonValue) -> Result<Self, String> {
        let outer = value
            .as_array()
            .ok_or_else(|| "requirements must be a list".to_string())?;
        let mut groups = Vec::with_capacity(outer.len());
        for group in outer {
            let names = group
                .as_array()
                .ok_or_else(|| "a requirement entry must be a list".to_string())?;
            groups.push(
                names
                    .iter()
                    .map(|name| match name {
                        RegistryJsonValue::String(name) => Ok(name.clone()),
                        _ => Err("a requirement name must be a string".to_string()),
                    })
                    .collect::<Result<Vec<JavaString>, String>>()?,
            );
        }
        Ok(Self { groups })
    }
}
