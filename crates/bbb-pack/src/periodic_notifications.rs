use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{
    resources::{PackResource, PackResourceStack, ResourceLocation},
    roots::PackRoots,
};

const REGIONAL_COMPLIANCIES: &str = "minecraft:regional_compliancies.json";
const MAX_NOTIFICATION_GROUPS: u64 = 256;
const MAX_PERIODIC_NOTIFICATIONS: u64 = 1_024;
const RETAINED_NOTIFICATION_GROUP_BYTES: u64 = 128;
const RETAINED_NOTIFICATION_BYTES: u64 = 256;

/// Immutable CPU preparation for Vanilla's `PeriodicNotificationManager`.
///
/// Selection by the host ISO3 country and timer ownership remain native-client
/// concerns. Missing or malformed data produces the same empty preparation as
/// Vanilla; security-budget rejection still rejects the enclosing generation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeriodicNotificationCatalog {
    generation: u64,
    source_pack_id: Option<String>,
    groups: BTreeMap<String, Vec<PeriodicNotification>>,
    load_error: Option<String>,
}

impl PeriodicNotificationCatalog {
    pub fn load(roots: &PackRoots) -> Result<Self> {
        let generation = roots
            .client_resource_generation()
            .map_or(0, |generation| generation.generation());
        Self::load_resource_stack(generation, &roots.resource_stack())
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn source_pack_id(&self) -> Option<&str> {
        self.source_pack_id.as_deref()
    }

    pub fn groups(&self) -> &BTreeMap<String, Vec<PeriodicNotification>> {
        &self.groups
    }

    pub fn notifications_for_iso3(&self, country_iso3: &str) -> &[PeriodicNotification] {
        self.groups
            .get(country_iso3)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    fn load_resource_stack(generation: u64, stack: &PackResourceStack) -> Result<Self> {
        let location = ResourceLocation::parse(REGIONAL_COMPLIANCIES)?;
        let Some(resource) = stack.get_resource(&location) else {
            return Ok(Self {
                generation,
                load_error: Some(format!("missing {REGIONAL_COMPLIANCIES}")),
                ..Self::default()
            });
        };
        let source_pack_id = Some(resource.pack_id.clone());
        let rejections_before = stack.resource_data_memory_observation().rejections;
        let prepared = prepare_notifications(stack, &resource);
        let (groups, load_error) = match prepared {
            Ok(groups) => (groups, None),
            Err(error) => {
                if stack.resource_data_memory_observation().rejections > rejections_before {
                    return Err(error);
                }
                (BTreeMap::new(), Some(format!("{error:#}")))
            }
        };
        stack.ensure_resource_data_admitted()?;
        Ok(Self {
            generation,
            source_pack_id,
            groups,
            load_error,
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PeriodicNotification {
    #[serde(default)]
    pub delay: i64,
    pub period: i64,
    pub title: String,
    pub message: String,
}

fn prepare_notifications(
    stack: &PackResourceStack,
    resource: &PackResource,
) -> Result<BTreeMap<String, Vec<PeriodicNotification>>> {
    let groups: BTreeMap<String, Vec<PeriodicNotification>> = stack
        .read_json_resource(resource, "regional compliancies")
        .with_context(|| format!("load {}", resource.location.id()))?;
    stack.admit_resource_collection_count(
        u64::try_from(groups.len()).context("periodic notification group count overflow")?,
        MAX_NOTIFICATION_GROUPS,
        "periodic notification groups",
    )?;
    let notification_count = groups.values().try_fold(0_u64, |count, notifications| {
        count
            .checked_add(
                u64::try_from(notifications.len())
                    .context("periodic notification count overflow")?,
            )
            .context("periodic notification count overflow")
    })?;
    stack.admit_resource_collection_count(
        notification_count,
        MAX_PERIODIC_NOTIFICATIONS,
        "periodic notifications",
    )?;
    let retained_bytes = u64::try_from(groups.len())
        .context("retained notification group count overflow")?
        .checked_mul(RETAINED_NOTIFICATION_GROUP_BYTES)
        .and_then(|bytes| {
            notification_count
                .checked_mul(RETAINED_NOTIFICATION_BYTES)
                .and_then(|notifications| bytes.checked_add(notifications))
        })
        .context("retained periodic notification byte count overflow")?;
    stack.reserve_derived_resource_bytes(retained_bytes, "retained periodic notifications")?;
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::*;

    #[test]
    fn catalog_uses_selected_pack_precedence_and_default_delay() {
        let root = unique_temp_dir("periodic-precedence");
        let vanilla = root.join("vanilla");
        let overlay = root.join("overlay");
        write_json(
            &asset(&vanilla),
            &json!({"USA":[{"period":60,"title":"vanilla","message":"message"}]}),
        );
        write_json(
            &asset(&overlay),
            &json!({"USA":[{"delay":7,"period":15,"title":"overlay","message":"message"}]}),
        );

        let catalog =
            PeriodicNotificationCatalog::load(&fixture_roots(&root, &vanilla, [overlay])).unwrap();
        assert_eq!(catalog.source_pack_id(), Some("unmanaged/1"));
        assert_eq!(
            catalog.notifications_for_iso3("USA"),
            [PeriodicNotification {
                delay: 7,
                period: 15,
                title: "overlay".to_string(),
                message: "message".to_string(),
            }]
        );
        assert!(catalog.notifications_for_iso3("CAN").is_empty());
        assert!(catalog.load_error().is_none());

        let default_delay =
            PeriodicNotificationCatalog::load(&fixture_roots(&root, &vanilla, std::iter::empty()))
                .unwrap();
        assert_eq!(default_delay.notifications_for_iso3("USA")[0].delay, 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn filter_and_malformed_resource_fall_back_to_empty() {
        let root = unique_temp_dir("periodic-fallback");
        let vanilla = root.join("vanilla");
        let filter = root.join("filter");
        write_json(
            &asset(&vanilla),
            &json!({"USA":[{"period":60,"title":"title","message":"message"}]}),
        );
        write_text(
            &filter.join("pack.mcmeta"),
            r#"{"filter":{"block":[{"namespace":"minecraft","path":"regional_compliancies\\.json"}]}}"#,
        );
        let filtered =
            PeriodicNotificationCatalog::load(&fixture_roots(&root, &vanilla, [filter])).unwrap();
        assert!(filtered.is_empty());
        assert!(filtered.load_error().unwrap().contains("missing"));

        write_text(&asset(&vanilla), "{not-json");
        let malformed =
            PeriodicNotificationCatalog::load(&fixture_roots(&root, &vanilla, std::iter::empty()))
                .unwrap();
        assert!(malformed.is_empty());
        assert_eq!(malformed.source_pack_id(), Some("unmanaged/0"));
        assert!(malformed.load_error().unwrap().contains("load"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_rejects_group_count_over_security_budget() {
        let root = unique_temp_dir("periodic-budget");
        let vanilla = root.join("vanilla");
        let groups = (0..=MAX_NOTIFICATION_GROUPS)
            .map(|index| (format!("C{index}"), serde_json::Value::Array(Vec::new())))
            .collect::<serde_json::Map<_, _>>();
        write_json(&asset(&vanilla), &serde_json::Value::Object(groups));
        let error =
            PeriodicNotificationCatalog::load(&fixture_roots(&root, &vanilla, std::iter::empty()))
                .unwrap_err()
                .to_string();
        assert!(error.contains("256 entry limit"), "{error}");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn catalog_carries_selected_resource_generation() {
        let root = unique_temp_dir("periodic-generation");
        std::fs::create_dir_all(&root).unwrap();
        let roots = fixture_roots(&root, &root.join("vanilla"), std::iter::empty());
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots.with_client_resource_generation(47, selected).unwrap();
        let catalog = PeriodicNotificationCatalog::load(&roots).unwrap();
        assert_eq!(catalog.generation(), 47);
        assert!(catalog.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires local vanilla 26.1 sources"]
    fn loads_local_vanilla_regional_compliancies() {
        let catalog =
            PeriodicNotificationCatalog::load(&PackRoots::from_environment().unwrap()).unwrap();
        assert_eq!(catalog.groups().len(), 1);
        assert_eq!(catalog.notifications_for_iso3("KOR").len(), 2);
        assert_eq!(catalog.notifications_for_iso3("KOR")[0].delay, 1_440);
        assert_eq!(catalog.notifications_for_iso3("KOR")[1].delay, 0);
        assert!(catalog.load_error().is_none());
    }

    fn fixture_roots(
        root: &Path,
        vanilla: &Path,
        overlays: impl IntoIterator<Item = PathBuf>,
    ) -> PackRoots {
        let mut packs = vec![vanilla.to_path_buf()];
        packs.extend(overlays);
        PackRoots::from_paths(
            root.to_path_buf(),
            root.join("empty-vanilla"),
            root.join("empty-vanilla/assets/minecraft"),
        )
        .with_resource_pack_dirs(packs)
    }

    fn asset(root: &Path) -> PathBuf {
        root.join("assets/minecraft/regional_compliancies.json")
    }

    fn write_json(path: &Path, value: &serde_json::Value) {
        write_text(path, &serde_json::to_string(value).unwrap());
    }

    fn write_text(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
    }
}
