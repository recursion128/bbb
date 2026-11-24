use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};

static PANIC_REPORT_INSTALLED: AtomicBool = AtomicBool::new(false);

/// Sends a panic to the structured log before the default hook prints it.
///
/// A panicking task otherwise leaves nothing in the stream a supervisor
/// collects: the thread dies, the message goes to stderr unstructured, and the
/// process carries on with one fewer connection than it thinks it has.
pub fn install_panic_report() {
    PANIC_REPORT_INSTALLED.store(true, Ordering::Release);
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(
            event = "server.panicked",
            message = panic_message(info.payload()),
            location = info.location().map(ToString::to_string),
            thread = std::thread::current().name(),
        );
        default(info);
    }));
}

pub(crate) fn panic_report_installed() -> bool {
    PANIC_REPORT_INSTALLED.load(Ordering::Acquire)
}

/// What a panic said, out of the two payload shapes `panic!` produces.
///
/// A panic with a formatted message arrives as `String` and a literal one as
/// `&str`; a payload of any other type carries nothing readable, and saying so
/// is better than logging an empty message.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("a panic with no message")
}

#[cfg(test)]
mod tests {
    use super::panic_message;

    #[test]
    fn reads_both_shapes_a_panic_payload_arrives_in() {
        let literal = std::panic::catch_unwind(|| panic!("a literal")).unwrap_err();
        assert_eq!(panic_message(literal.as_ref()), "a literal");

        let formatted = std::panic::catch_unwind(|| panic!("{} formatted", 1)).unwrap_err();
        assert_eq!(panic_message(formatted.as_ref()), "1 formatted");

        let opaque = std::panic::catch_unwind(|| std::panic::panic_any(7u8)).unwrap_err();
        assert_eq!(panic_message(opaque.as_ref()), "a panic with no message");
    }
}
