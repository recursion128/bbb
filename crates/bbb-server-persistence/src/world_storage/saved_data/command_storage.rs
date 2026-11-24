//! Vanilla command-storage SavedData documents and bounded namespace discovery.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use bbb_protocol::{
    nbt_summary::entry, normalize_resource_location, NbtSummaryEntry, NbtSummaryValue,
};
use bbb_server_types::save::{
    NamedCompound, SaveError, SaveResult, WorldSaveLimits, WorldSavePaths, JAVA_26_1_DATA_VERSION,
};

use crate::atomic_file::io_error;
use crate::level_file::read_gzip_compound;
use crate::schema_support::{compound_or_empty, invalid_schema, require_java_26_1_root};

const FILE_NAME: &str = "command_storage.dat";

pub type Java26CommandStorageContents = BTreeMap<String, Vec<NbtSummaryEntry>>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Java26CommandStorageNamespaces(pub BTreeMap<String, Java26CommandStorageContents>);

impl Java26CommandStorageNamespaces {
    pub fn namespace(&self, namespace: &str) -> Option<&Java26CommandStorageContents> {
        self.0.get(namespace)
    }

    pub fn namespaces(&self) -> impl Iterator<Item = (&str, &Java26CommandStorageContents)> {
        self.0
            .iter()
            .map(|(namespace, contents)| (namespace.as_str(), contents))
    }

    pub fn into_inner(self) -> BTreeMap<String, Java26CommandStorageContents> {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Java26LoadedCommandStorage {
    pub namespaces: Java26CommandStorageNamespaces,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    namespace: String,
    path: PathBuf,
}

/// Loads every command-storage namespace before world publication. The live
/// runtime still tracks which of these prevalidated namespaces Vanilla has
/// lazily touched (`CommandStorage.java:16-102`).
pub(crate) fn load_java_26_1_command_storage(
    paths: &WorldSavePaths,
    limits: WorldSaveLimits,
) -> SaveResult<Java26LoadedCommandStorage> {
    let candidates = discover_candidates(paths)?;
    let mut namespaces = BTreeMap::new();
    for candidate in &candidates {
        let (document, _) = read_gzip_compound(&candidate.path, limits)?;
        namespaces.insert(
            candidate.namespace.clone(),
            parse_document(&candidate.namespace, &document)?,
        );
    }
    Ok(Java26LoadedCommandStorage {
        namespaces: Java26CommandStorageNamespaces(namespaces),
    })
}

pub fn java_26_1_command_storage_document(
    contents: &Java26CommandStorageContents,
) -> NamedCompound {
    let encoded = contents
        .iter()
        .map(|(path, entries)| NbtSummaryEntry {
            name: path.clone().into(),
            value: NbtSummaryValue::Compound(entries.clone()),
        })
        .collect();
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry(
                "data",
                NbtSummaryValue::Compound(vec![entry(
                    "contents",
                    NbtSummaryValue::Compound(encoded),
                )]),
            ),
        ],
    )
}

pub fn command_storage_relative_path(namespace: &str) -> SaveResult<PathBuf> {
    validate_namespace(namespace, "command_storage namespace")?;
    Ok(PathBuf::from("data").join(namespace).join(FILE_NAME))
}

/// `CommandStorage.Container` (`CommandStorage.java:74-102`) reads every
/// compound under `contents`; a key that is not a valid path is skipped.
fn parse_document(
    namespace: &str,
    document: &NamedCompound,
) -> SaveResult<Java26CommandStorageContents> {
    let root = require_java_26_1_root(document, "command_storage")?;
    let contents = compound_or_empty(compound_or_empty(root, "data"), "contents");
    let mut decoded = BTreeMap::new();
    for field in contents {
        let Ok(key) = field.name.try_to_string() else {
            continue;
        };
        let NbtSummaryValue::Compound(entries) = &field.value else {
            continue;
        };
        if validate_path(namespace, &key, "command_storage.data.contents").is_ok() {
            decoded.entry(key).or_insert_with(|| entries.clone());
        }
    }
    Ok(decoded)
}

fn discover_candidates(paths: &WorldSavePaths) -> SaveResult<Vec<Candidate>> {
    let directory = paths.saved_data_dir();
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error("listing SavedData namespaces", &directory, source)),
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|source| io_error("listing SavedData namespaces", &directory, source))?;
        let namespace_path = entry.path();
        let file_type = entry.file_type().map_err(|source| {
            io_error(
                "reading SavedData namespace entry type",
                &namespace_path,
                source,
            )
        })?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let Some(namespace) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let path = namespace_path.join(FILE_NAME);
        let file_type = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata.file_type(),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(io_error(
                    "inspecting command-storage document",
                    &path,
                    source,
                ));
            }
        };
        // Vanilla only ever asks for valid namespaces.
        if validate_namespace(&namespace, "command_storage namespace directory").is_err() {
            continue;
        }
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "command-storage candidate must be a regular file and not a symbolic link",
            });
        }
        candidates.push(Candidate { namespace, path });
    }
    candidates.sort_by(|left, right| left.namespace.cmp(&right.namespace));
    Ok(candidates)
}

fn validate_namespace(namespace: &str, path: &str) -> SaveResult<()> {
    if namespace.is_empty()
        || normalize_resource_location(format!("{namespace}:value"))
            .ok()
            .as_deref()
            != Some(&format!("{namespace}:value"))
    {
        return Err(invalid_schema(path, "invalid resource namespace"));
    }
    Ok(())
}

fn validate_path(namespace: &str, value: &str, path: &str) -> SaveResult<()> {
    let resource_id = format!("{namespace}:{value}");
    if value.is_empty()
        || normalize_resource_location(resource_id.clone())
            .ok()
            .as_deref()
            != Some(resource_id.as_str())
    {
        return Err(invalid_schema(
            path,
            format!("invalid resource path `{value}`"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_file::encode_gzip_named_compound;

    fn scratch(name: &str) -> WorldSavePaths {
        let root =
            std::env::temp_dir().join(format!("bbb-command-storage-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        WorldSavePaths::new(root).expect("scratch path")
    }

    fn write_namespace(
        paths: &WorldSavePaths,
        namespace: &str,
        contents: Java26CommandStorageContents,
    ) {
        let relative = command_storage_relative_path(namespace).expect("valid namespace");
        let path = paths.root().join(relative);
        fs::create_dir_all(path.parent().expect("file has parent")).expect("create parent");
        let bytes = encode_gzip_named_compound(
            &path,
            &java_26_1_command_storage_document(&contents),
            WorldSaveLimits::default(),
        )
        .expect("gzip encodes");
        fs::write(path, bytes).expect("write fixture");
    }

    #[test]
    fn namespace_documents_round_trip_in_deterministic_order() {
        let paths = scratch("round-trip");
        let contents = BTreeMap::from([
            (
                "z/path".to_string(),
                vec![entry("v", NbtSummaryValue::Int(2))],
            ),
            ("a".to_string(), vec![entry("v", NbtSummaryValue::Int(1))]),
        ]);
        write_namespace(&paths, "example", contents.clone());

        let loaded = load_java_26_1_command_storage(&paths, WorldSaveLimits::default())
            .expect("namespace loads");
        assert_eq!(loaded.namespaces.namespace("example"), Some(&contents));
        let _ = fs::remove_dir_all(paths.root());
    }

    #[test]
    fn absent_storage_is_empty() {
        let paths = scratch("candidate-change");
        fs::create_dir_all(paths.root()).expect("create root");
        let loaded = load_java_26_1_command_storage(&paths, WorldSaveLimits::default())
            .expect("absence is empty");
        assert_eq!(loaded.namespaces, Java26CommandStorageNamespaces::default());
        let _ = fs::remove_dir_all(paths.root());
    }
}
