use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{ItemRegistryCatalog, PackRoots, TagCatalog};

const FREEZE_IMMUNE_WEARABLES_TAG: &str = "minecraft:freeze_immune_wearables";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeImmuneWearableCatalog {
    item_ids: BTreeSet<String>,
}

impl FreezeImmuneWearableCatalog {
    pub fn load(roots: &PackRoots, registry: &ItemRegistryCatalog) -> Result<Self> {
        let item_tags = roots
            .load_tag_catalog("item")
            .context("load item tags for freeze immune wearables")?;
        Self::from_item_tags(registry, &item_tags)
    }

    fn from_item_tags(registry: &ItemRegistryCatalog, item_tags: &TagCatalog) -> Result<Self> {
        let Some(values) = item_tags.values(FREEZE_IMMUNE_WEARABLES_TAG) else {
            bail!("missing {FREEZE_IMMUNE_WEARABLES_TAG} item tag");
        };
        let item_ids = values
            .iter()
            .filter(|item_id| registry.protocol_id(item_id).is_some())
            .cloned()
            .collect::<BTreeSet<_>>();
        if item_ids.is_empty() {
            bail!("{FREEZE_IMMUNE_WEARABLES_TAG} item tag did not contain registry items");
        }
        Ok(Self { item_ids })
    }

    pub fn item_ids(&self) -> &BTreeSet<String> {
        &self.item_ids
    }

    pub fn protocol_ids(&self, registry: &ItemRegistryCatalog) -> BTreeSet<i32> {
        self.item_ids
            .iter()
            .filter_map(|item_id| registry.protocol_id(item_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use crate::TagDefinition;

    #[test]
    fn freeze_immune_wearables_resolve_item_tag_to_protocol_ids() {
        let registry = ItemRegistryCatalog::from_items_java_source(
            r#"public class Items {
                public static final Item LEATHER_BOOTS = registerItem("leather_boots");
                public static final Item LEATHER_LEGGINGS = registerItem("leather_leggings");
                public static final Item STONE = registerItem("stone");
            }"#,
            &BTreeMap::new(),
        )
        .unwrap();
        let item_tags = item_tag_catalog(vec![
            "minecraft:leather_boots".to_string(),
            "minecraft:leather_leggings".to_string(),
            "minecraft:not_in_registry".to_string(),
        ]);

        let catalog = FreezeImmuneWearableCatalog::from_item_tags(&registry, &item_tags).unwrap();

        assert!(catalog.item_ids().contains("minecraft:leather_boots"));
        assert!(catalog.item_ids().contains("minecraft:leather_leggings"));
        assert!(!catalog.item_ids().contains("minecraft:not_in_registry"));
        assert_eq!(
            catalog.protocol_ids(&registry),
            BTreeSet::from([
                registry.protocol_id("minecraft:leather_boots").unwrap(),
                registry.protocol_id("minecraft:leather_leggings").unwrap(),
            ])
        );
    }

    fn item_tag_catalog(values: Vec<String>) -> TagCatalog {
        TagCatalog {
            registry_path: "item".to_string(),
            tags: BTreeMap::from([(
                FREEZE_IMMUNE_WEARABLES_TAG.to_string(),
                TagDefinition {
                    id: FREEZE_IMMUNE_WEARABLES_TAG.to_string(),
                    values,
                },
            )]),
        }
    }
}
