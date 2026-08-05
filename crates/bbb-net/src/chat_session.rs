//! Outbound chat preparation for the offline-only client.
//!
//! Online mode is a declared non-goal, so there is no profile key pair and no
//! local signed-message chain: signable command arguments keep the vanilla
//! signed-command packet shape with an empty signature list, and plain chat
//! messages carry no signature — exactly what a vanilla client without a chat
//! session sends. The last-seen acknowledgement pipeline is independent of
//! signing and stays in `bbb-world`.

use anyhow::{ensure, Result};
use bbb_protocol::packets::{ArgumentSignatures, ChatCommand, ChatCommandSigned, ChatMessage};

use crate::types::{OutboundChatCommand, OutboundChatCommandSigning, OutboundChatMessage};

#[derive(Debug)]
pub(crate) enum PreparedChatCommand {
    Unsigned(ChatCommand),
    Signed(ChatCommandSigned),
}

pub(crate) fn prepare_outbound_chat_command(
    request: OutboundChatCommand,
) -> Result<PreparedChatCommand> {
    let Some(OutboundChatCommandSigning {
        timestamp_millis,
        salt,
        arguments,
        last_seen,
    }) = request.signing
    else {
        return Ok(PreparedChatCommand::Unsigned(ChatCommand {
            command: request.command,
        }));
    };
    ensure!(
        !arguments.is_empty(),
        "signed outbound command has no signable arguments"
    );

    Ok(PreparedChatCommand::Signed(ChatCommandSigned {
        command: request.command,
        timestamp_millis,
        salt,
        argument_signatures: ArgumentSignatures {
            entries: Vec::new(),
        },
        last_seen_messages: last_seen.update,
    }))
}

pub(crate) fn prepare_outbound_chat_message(request: OutboundChatMessage) -> Result<ChatMessage> {
    Ok(ChatMessage {
        message: request.message,
        timestamp_millis: request.timestamp_millis,
        salt: request.salt,
        signature: None,
        last_seen_messages: request.last_seen.update,
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::packets::{LastSeenMessagesUpdate, MessageSignature};
    use bbb_world::{OutboundLastSeenMessages, SignableCommandArgument};

    use super::*;

    #[test]
    fn outbound_requests_are_unsigned_without_a_session() {
        let last_seen = OutboundLastSeenMessages {
            update: LastSeenMessagesUpdate {
                offset: 3,
                acknowledged: 5,
                checksum: 7,
            },
            signatures: vec![MessageSignature {
                bytes: vec![0x11; 256],
            }],
        };
        let message = prepare_outbound_chat_message(OutboundChatMessage {
            message: "hello".to_string(),
            timestamp_millis: 2_000,
            salt: 9,
            last_seen: last_seen.clone(),
        })
        .unwrap();
        assert_eq!(message.signature, None);
        assert_eq!(message.last_seen_messages, last_seen.update);

        let command = prepare_outbound_chat_command(OutboundChatCommand::signed(
            "say hello",
            3_000,
            10,
            vec![SignableCommandArgument {
                name: "message".to_string(),
                value: "hello".to_string(),
            }],
            last_seen.clone(),
        ))
        .unwrap();
        let PreparedChatCommand::Signed(command) = command else {
            panic!("signable command must retain the signed-command packet shape");
        };
        assert!(command.argument_signatures.entries.is_empty());
        assert_eq!(command.last_seen_messages, last_seen.update);

        let command =
            prepare_outbound_chat_command(OutboundChatCommand::unsigned("time set day")).unwrap();
        let PreparedChatCommand::Unsigned(command) = command else {
            panic!("plain command must use the unsigned command packet");
        };
        assert_eq!(command.command, "time set day");
    }
}
