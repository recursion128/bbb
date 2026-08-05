//! Resolved key bindings from the generated `KeyMapping` catalog.
//!
//! The catalog carries `category` and `sort_order` as data, but the vanilla
//! consumers of `KeyMapping.compareTo`, `same`, `Category.label` and the
//! `Category` sort registry are all key-binds-screen concerns — conflict
//! highlighting and list ordering in an options UI this client declares a
//! non-goal — so no runtime consumer exists for them here.

use super::key_mappings_26_1::{
    VanillaKeyMapping, VanillaKeyType, VANILLA_KEYSYM_KEY_CODES, VANILLA_KEY_MAPPINGS,
    VANILLA_MOUSE_BUTTONS, VANILLA_NAMED_KEYS,
};
use super::ClientOptions;
use winit::{event::MouseButton, keyboard::KeyCode};

/// The key one binding currently resolves to, either from an `options.txt`
/// `key_<name>` line or from the mapping's own default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoundKey {
    pub(crate) key_type: VanillaKeyType,
    pub(crate) value: i32,
}

/// What the window actually has to deliver for one binding to count as held.
/// A binding whose key winit cannot report — an unbound mapping, or a device
/// key with no physical equivalent — stays `Unbound` and never fires.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum BoundInput {
    #[default]
    Unbound,
    Key(KeyCode),
    Mouse(MouseButton),
}

impl BoundKey {
    #[cfg(test)]
    pub(crate) fn is_unbound(self) -> bool {
        self.key_type == VanillaKeyType::Keysym && self.value == -1
    }

    pub(crate) fn input(self) -> BoundInput {
        match self.key_type {
            VanillaKeyType::Keysym => VANILLA_KEYSYM_KEY_CODES
                .iter()
                .find(|(value, _)| *value == self.value)
                .map(|(_, code)| BoundInput::Key(*code))
                .unwrap_or_default(),
            VanillaKeyType::Mouse => VANILLA_MOUSE_BUTTONS
                .iter()
                .find(|(value, _)| *value == self.value)
                .map(|(_, button)| BoundInput::Mouse(*button))
                .unwrap_or_default(),
        }
    }
}

/// Every binding resolved once against the stored options, so the input owner
/// asks for `key.forward` rather than hard-coding the key it happens to sit on.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyBindings {
    resolved: [BoundInput; VANILLA_KEY_MAPPINGS.len()],
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut resolved = [BoundInput::Unbound; VANILLA_KEY_MAPPINGS.len()];
        for (slot, mapping) in resolved.iter_mut().zip(VANILLA_KEY_MAPPINGS.iter()) {
            *slot = BoundKey {
                key_type: mapping.default_type,
                value: mapping.default_value,
            }
            .input();
        }
        Self { resolved }
    }
}

impl KeyBindings {
    pub(crate) fn from_options(options: &ClientOptions) -> Self {
        let mut resolved = [BoundInput::Unbound; VANILLA_KEY_MAPPINGS.len()];
        for (slot, mapping) in resolved.iter_mut().zip(VANILLA_KEY_MAPPINGS.iter()) {
            *slot = bound_key(options, mapping).input();
        }
        Self { resolved }
    }

    pub(crate) fn input(&self, name: &str) -> BoundInput {
        VANILLA_KEY_MAPPINGS
            .iter()
            .position(|mapping| mapping.name == name)
            .map(|index| self.resolved[index])
            .unwrap_or_default()
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
/// `KeyMapping.createNameSupplier`: a keybind component for a registered
/// mapping shows the mapping's *currently bound* key, not the mapping's own
/// name. Bindings resolve once from `options.txt` at startup (there is no
/// key-binds screen), so the id→bound-key-name table is process-immutable.
/// The display text is the bound key's `key.keyboard.*`/`key.mouse.*`
/// language entry — vanilla's non-printable fallback path; the
/// `glfwGetKeyName` layout query has no winit equivalent.
static BOUND_KEY_DISPLAY_KEYS: std::sync::OnceLock<
    std::collections::HashMap<&'static str, &'static str>,
> = std::sync::OnceLock::new();

pub(crate) fn install_bound_key_display_keys(options: &ClientOptions) {
    let _ = BOUND_KEY_DISPLAY_KEYS.set(bound_key_display_keys(options));
}

pub(crate) fn bound_key_display_key(mapping_name: &str) -> Option<&'static str> {
    BOUND_KEY_DISPLAY_KEYS.get()?.get(mapping_name).copied()
}

pub(crate) fn bound_key_display_keys(
    options: &ClientOptions,
) -> std::collections::HashMap<&'static str, &'static str> {
    VANILLA_KEY_MAPPINGS
        .iter()
        .map(|mapping| {
            let bound = bound_key(options, mapping);
            let name = VANILLA_NAMED_KEYS
                .iter()
                .find(|named| named.key_type == bound.key_type && named.value == bound.value)
                .map(|named| named.name)
                .unwrap_or("key.keyboard.unknown");
            (mapping.name, name)
        })
        .collect()
}

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
    fn every_binding_resolves_to_a_key_the_window_can_report() {
        let bindings = KeyBindings::default();
        assert_eq!(
            bindings.input("key.forward"),
            BoundInput::Key(KeyCode::KeyW)
        );
        assert_eq!(
            bindings.input("key.sneak"),
            BoundInput::Key(KeyCode::ShiftLeft)
        );
        assert_eq!(
            bindings.input("key.hotbar.9"),
            BoundInput::Key(KeyCode::Digit9)
        );
        assert_eq!(
            bindings.input("key.attack"),
            BoundInput::Mouse(MouseButton::Left)
        );
        assert_eq!(
            bindings.input("key.use"),
            BoundInput::Mouse(MouseButton::Right)
        );
        assert_eq!(
            bindings.input("key.pickItem"),
            BoundInput::Mouse(MouseButton::Middle)
        );
        assert_eq!(bindings.input("key.smoothCamera"), BoundInput::Unbound);
        assert_eq!(bindings.input("key.nonexistent"), BoundInput::Unbound);

        for mapping in &VANILLA_KEY_MAPPINGS {
            let bound = BoundKey {
                key_type: mapping.default_type,
                value: mapping.default_value,
            };
            assert_eq!(
                bound.input() == BoundInput::Unbound,
                bound.is_unbound(),
                "{} resolves inconsistently",
                mapping.name
            );
        }
    }

    #[test]
    fn a_rebound_key_moves_the_resolved_input() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.up");
        options.set_dynamic_raw_for_test("key_key.attack", "key.keyboard.z");
        options.set_dynamic_raw_for_test("key_key.jump", "key.mouse.middle");

        let bindings = KeyBindings::from_options(&options);
        assert_eq!(
            bindings.input("key.forward"),
            BoundInput::Key(KeyCode::ArrowUp)
        );
        assert_eq!(bindings.input("key.attack"), BoundInput::Key(KeyCode::KeyZ));
        assert_eq!(
            bindings.input("key.jump"),
            BoundInput::Mouse(MouseButton::Middle)
        );
        assert_eq!(bindings.input("key.back"), BoundInput::Key(KeyCode::KeyS));
    }

    #[test]
    fn rebound_keys_lists_only_bindings_that_left_their_default() {
        let mut options = ClientOptions::defaults(ClientOptionHost::default());
        assert!(rebound_keys(&options).is_empty());

        options.set_dynamic_raw_for_test("key_key.forward", "key.keyboard.up");
        options.set_dynamic_raw_for_test("key_key.jump", "key.keyboard.space");
        options.set_dynamic_raw_for_test("key_key.sneak", "key.keyboard.bogus");
        let rebound = rebound_keys(&options);
        assert_eq!(
            rebound,
            vec![(
                "key.forward",
                BoundKey {
                    key_type: VanillaKeyType::Keysym,
                    value: 265,
                }
            )]
        );
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
