use bbb_facts::{
    persistent_registry_facts_26_1::{
        vanilla_persistent_registry_available_26_1, vanilla_persistent_registry_membership_26_1,
        vanilla_persistent_registry_tag_members_26_1,
    },
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};

pub static JAVA_26_1_BUILTIN_REGISTRY_RESOLVER: Java26BuiltinRegistryResolver =
    Java26BuiltinRegistryResolver::new(
        vanilla_persistent_registry_available_26_1,
        vanilla_persistent_registry_membership_26_1,
        vanilla_persistent_registry_tag_members_26_1,
    );

pub struct Java26BuiltinRegistryResolver {
    available: fn(&str) -> bool,
    membership: fn(&str, &str) -> PersistentRegistryMembership,
    tags: fn(&str, &str) -> PersistentRegistryTagMembers,
}

impl Java26BuiltinRegistryResolver {
    pub const fn new(
        available: fn(&str) -> bool,
        membership: fn(&str, &str) -> PersistentRegistryMembership,
        tags: fn(&str, &str) -> PersistentRegistryTagMembers,
    ) -> Self {
        Self {
            available,
            membership,
            tags,
        }
    }
}

impl PersistentRegistryResolver for Java26BuiltinRegistryResolver {
    fn registry_available(&self, registry_id: &str) -> bool {
        (self.available)(registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        (self.membership)(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        (self.tags)(registry_id, tag_id)
    }
}

/// Resolves persistent component references against the built-in 26.1 registries.
pub fn java_26_1_persistent_nbt_context() -> bbb_registry_codec::PersistentNbtDecodeContext<'static>
{
    bbb_registry_codec::PersistentNbtDecodeContext::with_resolver(
        &JAVA_26_1_BUILTIN_REGISTRY_RESOLVER,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMBERSHIP_SAMPLES: [(&str, &str); 5] = [
        ("minecraft:damage_type", "minecraft:generic"),
        ("minecraft:enchantment", "minecraft:sharpness"),
        ("minecraft:sound_event", "minecraft:entity.player.hurt"),
        ("minecraft:block", "minecraft:stone"),
        ("minecraft:mob_effect", "minecraft:speed"),
    ];

    fn membership(registry_id: &str, resource_id: &str) -> PersistentRegistryMembership {
        if MEMBERSHIP_SAMPLES.contains(&(registry_id, resource_id)) {
            PersistentRegistryMembership::Present
        } else if registry_id == "minecraft:unsupported_registry" {
            PersistentRegistryMembership::Unavailable
        } else {
            PersistentRegistryMembership::Absent
        }
    }

    fn tags(registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        match (registry_id, tag_id) {
            ("minecraft:block", "minecraft:mineable/pickaxe") => {
                PersistentRegistryTagMembers::Present(vec![
                    "minecraft:stone".to_string(),
                    "minecraft:deepslate".to_string(),
                ])
            }
            ("minecraft:damage_type", "minecraft:empty") => {
                PersistentRegistryTagMembers::Present(Vec::new())
            }
            ("minecraft:unsupported_registry", _) => PersistentRegistryTagMembers::Unavailable,
            _ => PersistentRegistryTagMembers::Absent,
        }
    }

    fn resolver() -> Java26BuiltinRegistryResolver {
        Java26BuiltinRegistryResolver::new(
            |registry_id| registry_id != "minecraft:unsupported_registry",
            membership,
            tags,
        )
    }

    #[test]
    fn routes_membership_callbacks_for_all_supported_registry_families() {
        let resolver = resolver();

        for (registry_id, resource_id) in MEMBERSHIP_SAMPLES {
            assert_eq!(
                resolver.resource_membership(registry_id, resource_id),
                PersistentRegistryMembership::Present,
                "registry={registry_id}, resource={resource_id}"
            );
        }
    }

    #[test]
    fn preserves_known_unknown_custom_and_unsupported_membership_results() {
        let resolver = resolver();

        assert_eq!(
            resolver.resource_membership("minecraft:block", "minecraft:stone"),
            PersistentRegistryMembership::Present
        );
        assert_eq!(
            resolver.resource_membership("minecraft:block", "minecraft:not_a_block"),
            PersistentRegistryMembership::Absent
        );
        assert_eq!(
            resolver.resource_membership("minecraft:block", "example:custom_block"),
            PersistentRegistryMembership::Absent
        );
        assert_eq!(
            resolver.resource_membership("example:custom_registry", "example:custom_value"),
            PersistentRegistryMembership::Absent
        );
        assert_eq!(
            resolver.resource_membership("minecraft:unsupported_registry", "minecraft:value"),
            PersistentRegistryMembership::Unavailable
        );
    }

    #[test]
    fn preserves_present_empty_absent_and_unavailable_tag_results() {
        let resolver = resolver();

        assert_eq!(
            resolver.tag_members("minecraft:block", "minecraft:mineable/pickaxe"),
            PersistentRegistryTagMembers::Present(vec![
                "minecraft:stone".to_string(),
                "minecraft:deepslate".to_string(),
            ])
        );
        assert_eq!(
            resolver.tag_members("minecraft:damage_type", "minecraft:empty"),
            PersistentRegistryTagMembers::Present(Vec::new())
        );
        assert_eq!(
            resolver.tag_members("minecraft:block", "minecraft:unknown"),
            PersistentRegistryTagMembers::Absent
        );
        assert_eq!(
            resolver.tag_members("minecraft:unsupported_registry", "minecraft:anything"),
            PersistentRegistryTagMembers::Unavailable
        );
    }
}
