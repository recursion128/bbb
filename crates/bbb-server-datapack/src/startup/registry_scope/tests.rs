use super::*;
use crate::tag_loader::LoadedRegistryTags;

type TagSpec<'a> = &'a [(&'a str, &'a [(&'a str, &'a [&'a str])])];

fn tags(entries: TagSpec<'_>) -> TagLoadReport {
    TagLoadReport {
        registries: entries
            .iter()
            .map(|(registry, tags)| LoadedRegistryTags {
                registry_id: (*registry).into(),
                tags: tags
                    .iter()
                    .map(|(id, members)| {
                        (
                            (*id).into(),
                            members.iter().map(|id| (*id).into()).collect(),
                        )
                    })
                    .collect(),
            })
            .collect(),
        warnings: Vec::new(),
    }
}

#[test]
fn frozen_registry_scope_empty_pending_maps_keep_membership_but_nonempty_maps_replace() {
    let previous = tags(&[
        ("minecraft:dialog", &[("test:old", &["test:one"])]),
        ("minecraft:block", &[("test:block", &["minecraft:stone"])]),
    ]);
    let generation = DatapackGeneration {
        registries: Vec::new(),
        worldgen: Default::default(),
    };
    let mut current = tags(&[("minecraft:dialog", &[("test:new", &["test:two"])])]);
    retain_unreloaded_tags(&mut current, &previous, &generation);
    assert!(current
        .registry("minecraft:dialog")
        .unwrap()
        .tag("test:old")
        .is_none());
    assert_eq!(
        current
            .registry("minecraft:dialog")
            .unwrap()
            .tag("test:new"),
        Some(["test:two".to_string()].as_slice())
    );
    assert_eq!(
        current.registry("minecraft:block"),
        previous.registry("minecraft:block")
    );
    let mut empty = TagLoadReport::default();
    retain_unreloaded_tags(&mut empty, &current, &generation);
    assert_eq!(empty, current);
}
