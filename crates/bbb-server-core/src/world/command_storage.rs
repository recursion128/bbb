use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{normalize_resource_location, NbtSummaryValue};
use bbb_server_persistence::{Java26CommandStorageContents, Java26CommandStorageNamespaces};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ServerCommandStorageError {
    InvalidResourceId(String),
    ValueMustBeCompound,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ServerCommandStorage {
    namespaces: BTreeMap<String, Java26CommandStorageContents>,
    loaded_namespaces: RefCell<BTreeSet<String>>,
    dirty_namespaces: BTreeSet<String>,
}

impl ServerCommandStorage {
    pub(crate) fn from_persisted(persisted: Java26CommandStorageNamespaces) -> Self {
        Self {
            namespaces: persisted.into_inner(),
            loaded_namespaces: RefCell::new(BTreeSet::new()),
            dirty_namespaces: BTreeSet::new(),
        }
    }

    pub(crate) fn get(
        &self,
        resource_id: &str,
    ) -> Result<NbtSummaryValue, ServerCommandStorageError> {
        let (namespace, path) = split_resource_id(resource_id)?;
        self.loaded_namespaces
            .borrow_mut()
            .insert(namespace.to_string());
        Ok(NbtSummaryValue::Compound(
            self.namespaces
                .get(namespace)
                .and_then(|contents| contents.get(path))
                .cloned()
                .unwrap_or_default(),
        ))
    }

    /// `CommandStorage.get` (CommandStorage.java:53-57): `getContainer`
    /// publishes the namespace to [`Self::keys`] on the read itself.
    pub(crate) fn read(&self, resource_id: &str) -> Option<NbtSummaryValue> {
        let (namespace, path) = split_resource_id(resource_id).ok()?;
        self.loaded_namespaces
            .borrow_mut()
            .insert(namespace.to_string());
        self.namespaces
            .get(namespace)
            .and_then(|contents| contents.get(path))
            .map(|entries| NbtSummaryValue::Compound(entries.clone()))
    }

    pub(crate) fn set(
        &mut self,
        resource_id: &str,
        value: NbtSummaryValue,
    ) -> Result<bool, ServerCommandStorageError> {
        let (namespace, path) = split_resource_id(resource_id)?;
        let NbtSummaryValue::Compound(entries) = value else {
            return Err(ServerCommandStorageError::ValueMustBeCompound);
        };
        self.loaded_namespaces
            .get_mut()
            .insert(namespace.to_string());
        let contents = self.namespaces.entry(namespace.to_string()).or_default();
        let changed = if entries.is_empty() {
            contents.remove(path).is_some()
        } else if contents.get(path) == Some(&entries) {
            false
        } else {
            contents.insert(path.to_string(), entries);
            true
        };
        if changed {
            self.dirty_namespaces.insert(namespace.to_string());
        }
        Ok(changed)
    }

    pub(crate) fn keys(&self) -> std::vec::IntoIter<String> {
        self.loaded_namespaces
            .borrow()
            .iter()
            .flat_map(|namespace| {
                self.namespaces
                    .get(namespace)
                    .into_iter()
                    .flat_map(|contents| contents.keys())
                    .map(move |path| format!("{namespace}:{path}"))
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub(crate) fn persistent_snapshot(&self) -> BTreeMap<String, Java26CommandStorageContents> {
        self.dirty_namespaces
            .iter()
            .map(|namespace| {
                (
                    namespace.clone(),
                    self.namespaces.get(namespace).cloned().unwrap_or_default(),
                )
            })
            .collect()
    }

    pub(crate) fn mark_persisted(
        &mut self,
        snapshot: &BTreeMap<String, Java26CommandStorageContents>,
    ) {
        for (namespace, saved_contents) in snapshot {
            if self.namespaces.get(namespace) == Some(saved_contents) {
                self.dirty_namespaces.remove(namespace);
            }
        }
    }
}

fn split_resource_id(resource_id: &str) -> Result<(&str, &str), ServerCommandStorageError> {
    let normalized = normalize_resource_location(resource_id.to_string())
        .map_err(|_| ServerCommandStorageError::InvalidResourceId(resource_id.to_string()))?;
    if normalized != resource_id {
        return Err(ServerCommandStorageError::InvalidResourceId(
            resource_id.to_string(),
        ));
    }
    let (namespace, path) = resource_id
        .split_once(':')
        .expect("normalized resource ids contain a namespace");
    if namespace.is_empty() || path.is_empty() {
        return Err(ServerCommandStorageError::InvalidResourceId(
            resource_id.to_string(),
        ));
    }
    Ok((namespace, path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::NbtSummaryEntry;

    fn compound(value: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "value".into(),
            value: NbtSummaryValue::Int(value),
        }])
    }

    #[test]
    fn persisted_namespaces_are_invisible_until_lazily_touched() {
        let persisted = Java26CommandStorageNamespaces(BTreeMap::from([
            (
                "example".to_string(),
                BTreeMap::from([(
                    "visible_later".to_string(),
                    match compound(1) {
                        NbtSummaryValue::Compound(entries) => entries,
                        _ => unreachable!(),
                    },
                )]),
            ),
            (
                "untouched".to_string(),
                BTreeMap::from([("hidden".to_string(), Vec::new())]),
            ),
        ]));
        let storage = ServerCommandStorage::from_persisted(persisted);
        assert_eq!(storage.keys().collect::<Vec<_>>(), Vec::<String>::new());

        assert_eq!(storage.read("example:visible_later"), Some(compound(1)));
        assert_eq!(
            storage.keys().collect::<Vec<_>>(),
            vec!["example:visible_later"]
        );
        assert!(storage.persistent_snapshot().is_empty());
    }

    #[test]
    fn empty_compounds_delete_keys_and_dirty_only_changed_namespaces() {
        let mut storage = ServerCommandStorage::default();
        assert!(storage.set("example:key", compound(2)).unwrap());
        assert!(!storage.set("example:key", compound(2)).unwrap());
        assert_eq!(storage.get("example:key").unwrap(), compound(2));
        assert!(storage
            .set("example:key", NbtSummaryValue::Compound(Vec::new()))
            .unwrap());
        assert_eq!(
            storage.get("example:key").unwrap(),
            NbtSummaryValue::Compound(Vec::new())
        );
        assert_eq!(
            storage.persistent_snapshot(),
            BTreeMap::from([("example".to_string(), BTreeMap::new())])
        );
        let snapshot = storage.persistent_snapshot();
        storage.mark_persisted(&snapshot);
        assert!(storage.persistent_snapshot().is_empty());
    }

    #[test]
    fn save_acknowledgement_does_not_clear_a_newer_namespace_mutation() {
        let mut storage = ServerCommandStorage::default();
        storage.set("example:key", compound(1)).unwrap();
        let snapshot = storage.persistent_snapshot();
        storage.set("example:key", compound(2)).unwrap();

        storage.mark_persisted(&snapshot);

        assert_eq!(
            storage.persistent_snapshot(),
            BTreeMap::from([(
                "example".to_string(),
                match compound(2) {
                    NbtSummaryValue::Compound(entries) => {
                        BTreeMap::from([("key".to_string(), entries)])
                    }
                    _ => unreachable!(),
                },
            )])
        );
    }
}
