//! The macOS system calls bbb makes outside winit.
//!
//! These live in their own crate for one reason: `bbb-native` cannot be
//! type-checked for `aarch64-apple-darwin` from a Linux host, because its
//! dependency closure reaches third-party build scripts that need an Apple
//! toolchain (`ring` compiles C, `objc_exception` compiles Objective-C). Every
//! `#[cfg(target_os = "macos")]` block left inside `bbb-native` is therefore
//! code no gate on this repository's dev host ever compiles. This crate's
//! closure is pure Rust, so `scripts/cargo-dev.sh cross-check` really does
//! compile the AppKit calls below on Linux, and they cannot rot silently.
//!
//! Keep it that way: add dependencies here only if they cross-compile without
//! an Apple toolchain.

use anyhow::Result;

/// Sets the running application's Dock icon from ICNS bytes.
///
/// Vanilla's `Window` hands the macOS icon to GLFW, which forwards it to
/// `-[NSApplication setApplicationIconImage:]`; winit has no equivalent
/// setter, so bbb makes the AppKit call itself.
///
/// Must be called from the macOS main thread — AppKit's `NSApplication` is
/// main-thread-only, and the call fails attributably rather than aborting when
/// it is not.
#[cfg(target_os = "macos")]
pub fn set_application_icon(icns_bytes: &[u8]) -> Result<()> {
    use anyhow::Context;
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let marker =
        MainThreadMarker::new().context("window icon apply is not on the macOS main thread")?;
    let data = NSData::with_bytes(icns_bytes);
    let image = NSImage::initWithData(NSImage::alloc(), &data)
        .context("AppKit rejected the Vanilla ICNS window icon")?;
    let application = NSApplication::sharedApplication(marker);
    // SAFETY: AppKit documents a non-null NSImage for this setter; `image` is retained for the
    // duration of the call and AppKit retains the application icon as needed.
    unsafe { application.setApplicationIconImage(Some(&image)) };
    Ok(())
}

/// Non-macOS targets keep the same entry point so callers need no `cfg` around
/// the call itself, and report the platform mismatch instead of silently
/// succeeding.
#[cfg(not(target_os = "macos"))]
pub fn set_application_icon(_icns_bytes: &[u8]) -> Result<()> {
    anyhow::bail!("macOS application icon apply is unavailable on this target")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn non_macos_targets_report_the_platform_mismatch() {
        let err = set_application_icon(b"icns").expect_err("non-macOS apply must fail");
        assert_eq!(
            err.to_string(),
            "macOS application icon apply is unavailable on this target"
        );
    }
}
