//! The argument properties the derived tree records, read back as templates.
//!
//! The catalog stores `ArgumentTypeInfo.serializeToJson` output — the same
//! projection `ClientboundCommandsPacket` writes — as the JSON text the probe
//! read off the official server. Both halves of this module's callers need it:
//! the tree publisher turns it back into the property bytes the wire carries,
//! and the parser reads the bounds out of it. Converting once here keeps the
//! properties a single derived fact rather than two hand-copied ones.

use std::sync::OnceLock;

use bbb_protocol::packets::{
    vanilla_command_node, CommandArgumentTemplate, CommandNodeType, CommandStringMode,
    VanillaCommandNode, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID, VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID, VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID, VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID,
};
use rustc_hash::FxHashMap;

fn cached_templates() -> &'static FxHashMap<usize, CommandArgumentTemplate> {
    static CACHE: OnceLock<FxHashMap<usize, CommandArgumentTemplate>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut map = FxHashMap::default();
        let mut index = 0u16;
        while let Some(node) = vanilla_command_node(index) {
            if node.kind == CommandNodeType::Argument && node.parser_type_id().is_some() {
                if let Some(template) = try_compute_argument_template(node) {
                    map.insert(std::ptr::from_ref(node) as usize, template);
                }
            }
            let Some(next) = index.checked_add(1) else {
                break;
            };
            index = next;
        }
        map
    })
}

/// The template behind an argument node of the derived tree.
///
/// Panics for a node this server publishes but cannot describe, which a test
/// over the whole published tree rules out: an argument this server offers but
/// could not serialize would be a node the client could not read back.
pub(super) fn argument_template(node: &VanillaCommandNode) -> CommandArgumentTemplate {
    if let Some(cached) = cached_templates().get(&(std::ptr::from_ref(node) as usize)) {
        return cached.clone();
    }
    compute_argument_template(node)
}

fn try_compute_argument_template(node: &VanillaCommandNode) -> Option<CommandArgumentTemplate> {
    let type_id = node.parser_type_id()?;
    let properties = node
        .properties
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .unwrap_or(serde_json::Value::Null);
    match type_id {
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID => {
            Some(CommandArgumentTemplate::Integer {
                min: number(&properties, "min").map(|value| value as i32),
                max: number(&properties, "max").map(|value| value as i32),
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID => Some(CommandArgumentTemplate::Long {
            min: number(&properties, "min").map(|value| value as i64),
            max: number(&properties, "max").map(|value| value as i64),
        }),
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID => Some(CommandArgumentTemplate::Float {
            min: number(&properties, "min").map(|value| value as f32),
            max: number(&properties, "max").map(|value| value as f32),
        }),
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID => {
            Some(CommandArgumentTemplate::Double {
                min: number(&properties, "min"),
                max: number(&properties, "max"),
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID => {
            let mode = match text(&properties, "type")? {
                "word" => CommandStringMode::SingleWord,
                "phrase" => CommandStringMode::QuotablePhrase,
                "greedy" => CommandStringMode::GreedyPhrase,
                _ => return None,
            };
            Some(CommandArgumentTemplate::String { mode })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID => Some(CommandArgumentTemplate::Entity {
            single: text(&properties, "amount") == Some("single"),
            players_only: text(&properties, "type") == Some("players"),
        }),
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID => {
            Some(CommandArgumentTemplate::Registry {
                registry: text(&properties, "registry")?.to_string(),
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID => {
            Some(CommandArgumentTemplate::ScoreHolder {
                multiple: text(&properties, "amount") == Some("multiple"),
            })
        }
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => Some(CommandArgumentTemplate::Time {
            minimum: number(&properties, "min")? as i32,
        }),
        _ if node.properties.is_none() => Some(CommandArgumentTemplate::Unit),
        _ => None,
    }
}

fn compute_argument_template(node: &VanillaCommandNode) -> CommandArgumentTemplate {
    let type_id = node
        .parser_type_id()
        .expect("only argument nodes carry properties");
    let properties = node
        .properties
        .map(|json| {
            serde_json::from_str::<serde_json::Value>(json)
                .expect("the derived tree records serializeToJson output")
        })
        .unwrap_or(serde_json::Value::Null);

    match type_id {
        // `IntegerArgumentInfo.serializeToJson` writes the bounds it holds and
        // omits the ones left at the type's extremes.
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_INTEGER_ID => CommandArgumentTemplate::Integer {
            min: number(&properties, "min").map(|value| value as i32),
            max: number(&properties, "max").map(|value| value as i32),
        },
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_LONG_ID => CommandArgumentTemplate::Long {
            min: number(&properties, "min").map(|value| value as i64),
            max: number(&properties, "max").map(|value| value as i64),
        },
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_FLOAT_ID => CommandArgumentTemplate::Float {
            min: number(&properties, "min").map(|value| value as f32),
            max: number(&properties, "max").map(|value| value as f32),
        },
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_DOUBLE_ID => CommandArgumentTemplate::Double {
            min: number(&properties, "min"),
            max: number(&properties, "max"),
        },
        // `StringArgumentSerializer.serializeToJson` names the three
        // `StringType`s `word`, `phrase` and `greedy`.
        VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID => CommandArgumentTemplate::String {
            mode: match text(&properties, "type") {
                Some("word") => CommandStringMode::SingleWord,
                Some("phrase") => CommandStringMode::QuotablePhrase,
                Some("greedy") => CommandStringMode::GreedyPhrase,
                other => panic!("unknown brigadier string type {other:?}"),
            },
        },
        // `EntityArgument.Info.serializeToJson` writes `amount` as `single` or
        // `multiple` and `type` as `players` or `entities`.
        VANILLA_COMMAND_ARGUMENT_TYPE_ENTITY_ID => CommandArgumentTemplate::Entity {
            single: text(&properties, "amount") == Some("single"),
            players_only: text(&properties, "type") == Some("players"),
        },
        // `ResourceArgument.Info.serializeToJson` (ResourceArgument.java:140-142)
        // writes the registry the argument resolves against and nothing else.
        // The key/tag and selector forms use the same property and wire key
        // (`ResourceOrTagKeyArgument.java:80-90`, `ResourceSelectorArgument.java:96-106`).
        VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_KEY_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_OR_TAG_KEY_ID
        | VANILLA_COMMAND_ARGUMENT_TYPE_RESOURCE_SELECTOR_ID => CommandArgumentTemplate::Registry {
            registry: text(&properties, "registry")
                .expect("a resource argument always serializes its registry")
                .to_string(),
        },
        // `ScoreHolderArgument.Info.serializeToJson`
        // (ScoreHolderArgument.java:187-189) writes `amount` as `single` or
        // `multiple` and nothing else.
        VANILLA_COMMAND_ARGUMENT_TYPE_SCORE_HOLDER_ID => CommandArgumentTemplate::ScoreHolder {
            multiple: text(&properties, "amount") == Some("multiple"),
        },
        // `TimeArgument.Info.serializeToJson` (TimeArgument.java:95-97).
        VANILLA_COMMAND_ARGUMENT_TYPE_TIME_ID => CommandArgumentTemplate::Time {
            minimum: number(&properties, "min")
                .map(|value| value as i32)
                .expect("a time argument always serializes its minimum"),
        },
        // Every other parser this server publishes is a
        // `SingletonArgumentInfo`, which serializes to nothing at all.
        _ => {
            assert!(
                node.properties.is_none(),
                "argument parser {type_id} carries properties this server cannot read"
            );
            CommandArgumentTemplate::Unit
        }
    }
}

fn number(properties: &serde_json::Value, key: &str) -> Option<f64> {
    properties.get(key)?.as_f64()
}

fn text<'a>(properties: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    properties.get(key)?.as_str()
}
