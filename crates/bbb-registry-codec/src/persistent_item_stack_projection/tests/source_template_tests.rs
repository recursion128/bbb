use std::collections::BTreeMap;

use bbb_facts::{
    mob_effect_types::vanilla_mob_effect_type_for_resource_id,
    potion_types::vanilla_potion_for_resource_id, PersistentRegistryMembership,
    PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_protocol::packets::ConsumeEffectSummary;

use crate::registry_codec::RegistryReferenceLookup;
use crate::{decode_direct_registry_json, item_stack_template_from_source, RegistryElementSource};

use super::super::PersistentItemStackHolderResolver;

#[derive(Default)]
struct Lookup(BTreeMap<String, i32>);

impl RegistryReferenceLookup for Lookup {
    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        match registry {
            "minecraft:banner_pattern" => self.0.contains_key(resource_id),
            "minecraft:mob_effect" => {
                vanilla_mob_effect_type_for_resource_id(resource_id).is_some()
            }
            "minecraft:potion" => vanilla_potion_for_resource_id(resource_id).is_some(),
            _ => false,
        }
    }

    fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
        false
    }
}

impl PersistentRegistryResolver for Lookup {
    fn resource_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if self.contains_element(registry, resource_id) {
            PersistentRegistryMembership::Present
        } else {
            bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1(
                registry,
                resource_id,
            )
        }
    }

    fn tag_members(&self, _registry: &str, _tag: &str) -> PersistentRegistryTagMembers {
        PersistentRegistryTagMembers::Absent
    }
}

impl PersistentItemStackHolderResolver for Lookup {
    fn holder_id(&self, registry: &str, resource_id: &str) -> Option<i32> {
        match registry {
            "minecraft:banner_pattern" => self.0.get(resource_id).copied(),
            "minecraft:mob_effect" => {
                vanilla_mob_effect_type_for_resource_id(resource_id).map(|entry| entry.id)
            }
            "minecraft:potion" => vanilla_potion_for_resource_id(resource_id).map(|entry| entry.id),
            _ => None,
        }
    }
}

fn from_json(
    raw: &[u8],
    lookup: &Lookup,
) -> Result<bbb_protocol::packets::ItemStackTemplateSummary, crate::RegistryCodecError> {
    let decoded = decode_direct_registry_json(raw).unwrap();
    item_stack_template_from_source(
        decoded.root(),
        RegistryElementSource::ServerDatapackJson,
        lookup,
    )
}

#[test]
fn source_template_retains_potion_and_generic_hidden_effects() {
    let raw = br#"{"id":"potion","components":{"potion_contents":{"custom_name":"x","custom_effects":[{"id":"haste","amplifier":1,"duration":200,"hidden_effect":{"duration":40}}]},"consumable":{"on_consume_effects":[{"type":"apply_effects","effects":[{"id":"haste","hidden_effect":{"duration":40}}]}]}}}"#;
    let template = from_json(raw, &Lookup::default()).unwrap();
    assert_eq!(
        template
            .component_patch
            .potion_contents
            .custom_name
            .as_ref()
            .unwrap(),
        "x"
    );
    assert_eq!(
        template
            .component_patch
            .potion_contents
            .custom_effects
            .len(),
        1
    );
    assert_eq!(
        template.component_patch.potion_contents.custom_effects[0]
            .hidden_effect
            .as_ref()
            .unwrap()
            .duration,
        40,
    );
    let [ConsumeEffectSummary::ApplyEffects { effects, .. }] =
        template.component_patch.consumable_effects.as_slice()
    else {
        panic!("one generic apply effect");
    };
    assert_eq!(effects[0].hidden_effect.as_ref().unwrap().duration, 40);
}
