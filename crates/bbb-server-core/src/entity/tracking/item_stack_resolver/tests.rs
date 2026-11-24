use super::*;
use crate::entity::tracking::EntityTracker;

#[test]
fn tracking_registry_view_keeps_empty_registries_and_freezes_tag_status() {
    let ids = BTreeSet::from(["example:empty".into(), "minecraft:enchantment".into()]);
    let tags = Arc::new(RegistryTagSnapshot::new([(
        "example:empty".into(),
        "example:empty_tag".into(),
        Vec::new(),
    )]));
    let tracker =
        EntityTracker::new_with_item_stack_registry_view(ids, BTreeMap::new(), Arc::clone(&tags));
    drop(tags);
    let copy = tracker.clone();
    drop(tracker);
    let resolver = copy.item_stack_resolver();
    assert!(resolver.registry_available("example:empty"));
    assert_eq!(
        resolver.resource_membership("example:empty", "example:missing"),
        PersistentRegistryMembership::Absent
    );
    assert_eq!(
        resolver.tag_members("example:empty", "example:empty_tag"),
        PersistentRegistryTagMembers::Present(Vec::new())
    );
    assert_eq!(
        resolver.tag_members("example:empty", "example:missing"),
        PersistentRegistryTagMembers::Absent
    );
    assert!(resolver.registry_available("minecraft:enchantment"));
    assert_eq!(
        resolver.resource_membership("minecraft:enchantment", "minecraft:sharpness"),
        PersistentRegistryMembership::Absent
    );
    assert!(!resolver.registry_available("example:unknown"));
    assert_eq!(
        resolver.resource_membership("example:unknown", "example:member"),
        PersistentRegistryMembership::Unavailable
    );
    assert!(resolver.registry_available("minecraft:map_decoration_type"));
    assert_eq!(
        resolver.resource_membership("minecraft:map_decoration_type", "minecraft:player"),
        PersistentRegistryMembership::Present
    );
    assert_eq!(
        resolver.holder_id("minecraft:enchantment", "minecraft:sharpness"),
        None
    );
    let publication = &resolver;
    assert!(publication.registry_available("example:empty"));
    assert!(!publication.registry_available("example:unknown"));
    assert!(!&bbb_registry_codec::UnavailableHolderResolver.registry_available("example:empty"));
}

#[test]
fn tracking_registry_view_preserves_wire_ids_and_inferred_legacy_membership() {
    let holders = BTreeMap::from([(("example:custom".into(), "example:member".into()), 37)]);
    let tags = Arc::new(RegistryTagSnapshot::new([]));
    let tracker =
        EntityTracker::new_with_item_stack_registry_view(BTreeSet::new(), holders.clone(), tags);
    let legacy = EntityTracker::new_with_item_stack_holder_ids(holders.clone());
    let bare = TrackingItemStackResolver::new(&holders);
    for resolver in [
        tracker.item_stack_resolver(),
        legacy.item_stack_resolver(),
        bare,
    ] {
        assert!(resolver.registry_available("example:custom"));
        assert_eq!(
            resolver.resource_membership("example:custom", "example:member"),
            PersistentRegistryMembership::Present
        );
        assert_eq!(
            resolver.resource_membership("example:custom", "example:missing"),
            PersistentRegistryMembership::Absent
        );
        assert_eq!(
            resolver.holder_id("example:custom", "example:member"),
            Some(37)
        );
        assert_eq!(
            resolver.holder_resource_id("example:custom", 37),
            Some("example:member".into())
        );
        assert!(resolver.registry_available("minecraft:entity_type"));
        assert_eq!(
            resolver.resource_membership("minecraft:entity_type", "minecraft:pig"),
            PersistentRegistryMembership::Present
        );
    }
}
