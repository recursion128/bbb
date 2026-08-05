use super::key_mappings_26_1::{
    VanillaKeyMapping, VanillaKeyType, VANILLA_KEY_MAPPINGS, VANILLA_NAMED_KEYS,
};
use super::ClientOptions;

/// The key one binding currently resolves to, either from an `options.txt`
/// `key_<name>` line or from the mapping's own default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundKey {
    pub(crate) key_type: VanillaKeyType,
    pub(crate) value: i32,
}

impl BoundKey {
    #[cfg(test)]
    pub(crate) fn is_unbound(self) -> bool {
        self.key_type == VanillaKeyType::Keysym && self.value == -1
    }
}

#[cfg(test)]
pub(crate) fn vanilla_key_mapping(name: &str) -> Option<&'static VanillaKeyMapping> {
    VANILLA_KEY_MAPPINGS
        .iter()
        .find(|mapping| mapping.name == name)
}

/// `InputConstants.getKey(String)`, restricted to the names the static
/// initializer registers. An unknown name leaves the binding untouched, the
/// same way Vanilla's lookup refuses to invent a key.
pub(crate) fn named_key(name: &str) -> Option<BoundKey> {
    VANILLA_NAMED_KEYS
        .iter()
        .find(|key| key.name == name)
        .map(|key| BoundKey {
            key_type: key.key_type,
            value: key.value,
        })
}

/// Every binding whose stored key differs from the mapping's own default.
pub(crate) fn rebound_keys(options: &ClientOptions) -> Vec<(&'static str, BoundKey)> {
    VANILLA_KEY_MAPPINGS
        .iter()
        .filter_map(|mapping| {
            let bound = bound_key(options, mapping);
            (bound
                != BoundKey {
                    key_type: mapping.default_type,
                    value: mapping.default_value,
                })
            .then_some((mapping.name, bound))
        })
        .collect()
}

pub(crate) fn bound_key(options: &ClientOptions, mapping: &VanillaKeyMapping) -> BoundKey {
    let default = BoundKey {
        key_type: mapping.default_type,
        value: mapping.default_value,
    };
    options
        .dynamic_raw(&format!("key_{}", mapping.name))
        .and_then(named_key)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_options::ClientOptionHost;

    #[test]
    fn the_catalog_covers_every_binding_default_with_a_named_key() {
        assert_eq!(VANILLA_KEY_MAPPINGS.len(), 60);
        for mapping in &VANILLA_KEY_MAPPINGS {
            let default = BoundKey {
                key_type: mapping.default_type,
                value: mapping.default_value,
            };
            assert!(
                VANILLA_NAMED_KEYS
                    .iter()
                    .any(|key| key.key_type == default.key_type && key.value == default.value),
                "{} defaults to an unnamed key",
                mapping.name
            );
        }
        let attack = vanilla_key_mapping("key.attack").expect("key.attack is catalogued");
        assert_eq!(attack.default_type, VanillaKeyType::Mouse);
        assert_eq!(attack.default_value, 0);
        assert!(attack.toggle);
        let smooth_camera =
            vanilla_key_mapping("key.smoothCamera").expect("key.smoothCamera is catalogued");
        assert!(BoundKey {
            key_type: smooth_camera.default_type,
            value: smooth_camera.default_value,
        }
        .is_unbound());
    }

    #[test]
    fn a_stored_binding_overrides_the_default_and_a_bad_name_does_not() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        let forward = vanilla_key_mapping("key.forward").expect("key.forward is catalogued");
        assert_eq!(
            bound_key(&options, forward),
            BoundKey {
                key_type: VanillaKeyType::Keysym,
                value: 87,
            }
        );

        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.up");
        assert_eq!(
            bound_key(&options, forward),
            BoundKey {
                key_type: VanillaKeyType::Keysym,
                value: 265,
            }
        );

        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.nonsense");
        assert_eq!(
            bound_key(&options, forward),
            BoundKey {
                key_type: VanillaKeyType::Keysym,
                value: 87,
            }
        );

        let use_item = vanilla_key_mapping("key.use").expect("key.use is catalogued");
        options.set_dynamic_raw_for_test("key_key.use", "key.mouse.middle");
        assert_eq!(
            bound_key(&options, use_item),
            BoundKey {
                key_type: VanillaKeyType::Mouse,
                value: 2,
            }
        );
    }
}
