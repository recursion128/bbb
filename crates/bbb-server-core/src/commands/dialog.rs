//! `ResourceOrIdArgument.dialog` (`ResourceOrIdArgument.java:111-117`).

use bbb_protocol::{
    packets::{DialogHolder, ShowDialog},
    scan_command_resource_or_id_utf16, NbtSummaryValue, ScannedCommandResourceOrIdUtf16,
};
use bbb_registry_codec::{
    decode_exact_synchronized_registry_element_with_content_registry, RegistryElementData,
};

use super::parse::ParseFailure;
use super::translatable;
use crate::world::ServerWorld;

/// The registry `ResourceOrIdArgument.DialogArgument` names
/// (`ResourceOrIdArgument.java:144-147`).
const DIALOG_REGISTRY: &str = "minecraft:dialog";

/// `ResourceOrIdArgument.parse` (`ResourceOrIdArgument.java:123-132`) for
/// `Registries.DIALOG`: a name resolves against the live registry and an
/// inline value is decoded by `Dialog.DIRECT_CODEC` while the argument parses.
pub(super) fn read_dialog(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, ShowDialog), ParseFailure> {
    let scanned = scan_command_resource_or_id_utf16(command, cursor)
        .map_err(|_| failed_to_parse(cursor, "expected an id or an inline SNBT value"))?;
    match scanned {
        ScannedCommandResourceOrIdUtf16::Reference { end, resource_id } => {
            // `ReferenceResult.parse` (`ResourceOrIdArgument.java:184-196`).
            let Some(registry_id) = world
                .registry_ids("minecraft:dialog")
                .iter()
                .position(|id| id == &resource_id)
                .and_then(|index| i32::try_from(index).ok())
            else {
                return Err(ParseFailure {
                    message: translatable(
                        "argument.resource_or_id.no_such_element",
                        vec![
                            NbtSummaryValue::string(resource_id),
                            NbtSummaryValue::string(DIALOG_REGISTRY),
                        ],
                    ),
                    cursor: Some(cursor),
                });
            };
            Ok((
                end,
                ShowDialog {
                    dialog: DialogHolder::Reference { registry_id },
                },
            ))
        }
        // `InlineResult.parse` (`ResourceOrIdArgument.java:150-163`).
        ScannedCommandResourceOrIdUtf16::Inline { end, value } => {
            let data = RegistryElementData::from_network_nbt(value);
            let decoded = decode_exact_synchronized_registry_element_with_content_registry(
                DIALOG_REGISTRY,
                &data,
                world.plugins().content_registry(),
            )
            .map_err(|error| failed_to_parse(cursor, error.to_string()))?;
            let dialog = decoded
                .projected_network_nbt()
                .map_err(|error| failed_to_parse(cursor, error.to_string()))?;
            Ok((
                end,
                ShowDialog {
                    dialog: DialogHolder::Direct { dialog },
                },
            ))
        }
    }
}

/// `ERROR_FAILED_TO_PARSE` (`ResourceOrIdArgument.java:46-48`).
fn failed_to_parse(cursor: usize, message: impl Into<String>) -> ParseFailure {
    ParseFailure {
        message: translatable(
            "argument.resource_or_id.failed_to_parse",
            vec![NbtSummaryValue::String(message.into().into())],
        ),
        cursor: Some(cursor),
    }
}
