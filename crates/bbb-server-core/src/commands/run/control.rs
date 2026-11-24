//! `/return`.

use std::sync::Arc;

use crate::commands::execution::{ExecutionControl, ExecutionSource};
use crate::commands::parse::{ArgumentValue, ParseResult};

/// `ReturnCommand.ReturnValueCustomExecutor.run` (ReturnCommand.java:62-70):
/// the source's own callback (`:65`) before the frame's (`:67`), which is how
/// an enclosing `/execute store` sees the value.
pub(super) fn return_value(
    parse: &ParseResult,
    source: &Arc<ExecutionSource>,
    control: &mut ExecutionControl<'_, '_>,
) {
    let Some(ArgumentValue::Integer(value)) = parse.argument("value") else {
        unreachable!("value is an integer argument");
    };
    control.source_result(source, true, *value);
    control.return_success(*value);
    control.discard_current_frame();
}

/// `ReturnCommand.ReturnFailCustomExecutor.run` (ReturnCommand.java:32-39).
pub(super) fn return_fail(source: &Arc<ExecutionSource>, control: &mut ExecutionControl<'_, '_>) {
    control.source_result(source, false, 0);
    control.return_failure();
    control.discard_current_frame();
}
