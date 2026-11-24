use crate::{PackResourceStack, PackRoots, ResourceLocation};

const END_POEM: &str = "minecraft:texts/end.txt";
const CREDITS: &str = "minecraft:texts/credits.json";
const POSTCREDITS: &str = "minecraft:texts/postcredits.txt";

/// Selected-pack raw documents used by Vanilla's poem-and-credits screen.
///
/// Individual missing, unreadable, or invalid UTF-8 documents are omitted,
/// matching `WinScreen.wrapCreditsIO`. Admission failures remain recorded in
/// the shared resource-data generation budget and reject the candidate when
/// the reload barrier calls `ensure_resource_data_admitted`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WinScreenTextResources {
    pub end_poem: Option<String>,
    pub credits_json: Option<String>,
    pub postcredits: Option<String>,
}

impl WinScreenTextResources {
    pub fn load(roots: &PackRoots) -> Self {
        Self::load_from_stack(&roots.resource_stack())
    }

    fn load_from_stack(resources: &PackResourceStack) -> Self {
        Self {
            end_poem: read_optional(resources, END_POEM, "end poem"),
            credits_json: read_optional(resources, CREDITS, "credits"),
            postcredits: read_optional(resources, POSTCREDITS, "postcredits"),
        }
    }
}

fn read_optional(resources: &PackResourceStack, id: &str, label: &str) -> Option<String> {
    let location = ResourceLocation::parse(id).expect("static win-screen resource id is valid");
    let resource = resources.get_resource(&location)?;
    resources.read_text_resource(&resource, label).ok()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::resource_budget::ResourceDataGenerationBudget;

    use super::*;

    #[test]
    fn win_screen_text_resources_follow_pack_precedence_and_omit_bad_documents() {
        let root = unique_temp_dir("win-screen-text");
        let vanilla = root.join("vanilla");
        let overlay = root.join("overlay");
        write(&vanilla, "end.txt", b"vanilla poem");
        write(&vanilla, "credits.json", br#"[{"section":"vanilla"}]"#);
        write(&vanilla, "postcredits.txt", b"vanilla postcredits");
        write(&overlay, "end.txt", b"overlay poem");
        write(&overlay, "credits.json", &[0xff]);

        let resources = PackResourceStack::from_roots([vanilla, overlay]);
        let loaded = WinScreenTextResources::load_from_stack(&resources);

        assert_eq!(loaded.end_poem.as_deref(), Some("overlay poem"));
        assert_eq!(loaded.credits_json, None);
        assert_eq!(loaded.postcredits.as_deref(), Some("vanilla postcredits"));
        assert_eq!(
            resources.resource_data_memory_observation().text_documents,
            2
        );
        resources.ensure_resource_data_admitted().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn win_screen_text_budget_exhaustion_rejects_the_generation() {
        let root = unique_temp_dir("win-screen-budget");
        let vanilla = root.join("vanilla");
        write(&vanilla, "end.txt", b"four");
        let budget = ResourceDataGenerationBudget::new(3);
        let resources = PackResourceStack::from_roots_with_data_budget([vanilla], budget);

        let loaded = WinScreenTextResources::load_from_stack(&resources);

        assert_eq!(loaded.end_poem, None);
        assert!(resources.ensure_resource_data_admitted().is_err());
        assert_eq!(resources.resource_data_memory_observation().rejections, 1);
        fs::remove_dir_all(root).unwrap();
    }

    fn write(root: &Path, name: &str, contents: &[u8]) {
        let path = root.join("assets/minecraft/texts").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-{label}-{}-{nanos}", std::process::id()))
    }
}
