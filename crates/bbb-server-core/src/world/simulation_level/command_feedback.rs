//! Which `CommandSourceStack` channel one feedback line travelled.

use super::*;

/// Which `CommandSourceStack` channel one feedback line travelled, which is
/// what decides whether `send_command_feedback` may swallow it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::world) enum CommandFeedbackChannel {
    /// `CommandSourceStack.sendSuccess` (`CommandSourceStack.java:464-477`),
    /// whose gate is `this.source.acceptsSuccess()` (`:465`).
    Success,
    /// `CommandSourceStack.sendChatMessage` (`:442-451`), `sendFailure`
    /// (`:495-499`) and `sendSystemMessage` (`:453-462`). None of the three
    /// reads `acceptsSuccess`: `sendChatMessage` reads `this.silent` alone
    /// (`:443`), `sendFailure` reads `acceptsFailure` (`:496`), which a player
    /// answers true to unconditionally (`ServerPlayer.java:346-348`), and
    /// `sendSystemMessage` reads `this.silent` alone as well (`:454`).
    Ungated,
}

// CommandSourceStack.java:442-499: only sendSuccess reads acceptsSuccess.
pub(in crate::world) fn command_feedback_channel(
    feedback: &CommandFeedback,
) -> CommandFeedbackChannel {
    match feedback.kind {
        CommandFeedbackKind::Failure | CommandFeedbackKind::System | CommandFeedbackKind::Chat => {
            CommandFeedbackChannel::Ungated
        }
        CommandFeedbackKind::Success => CommandFeedbackChannel::Success,
    }
}
