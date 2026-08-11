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
#[cfg(target_os = "macos")]
use std::{ffi::c_void, ptr::NonNull};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetalLayerPresentation {
    pub drawable_width: f64,
    pub drawable_height: f64,
    pub maximum_drawable_count: usize,
}

#[cfg(target_os = "macos")]
#[repr(C)]
struct CoreGraphicsSize {
    width: CoreGraphicsFloat,
    height: CoreGraphicsFloat,
}

#[cfg(all(target_os = "macos", target_pointer_width = "32"))]
type CoreGraphicsFloat = f32;
#[cfg(all(target_os = "macos", target_pointer_width = "64"))]
type CoreGraphicsFloat = f64;

// SAFETY: CoreGraphics CGSize is a C struct containing two CGFloat values.
#[cfg(target_os = "macos")]
unsafe impl objc2::encode::Encode for CoreGraphicsSize {
    const ENCODING: objc2::encode::Encoding = objc2::encode::Encoding::Struct(
        "CGSize",
        &[
            <CoreGraphicsFloat as objc2::encode::Encode>::ENCODING,
            <CoreGraphicsFloat as objc2::encode::Encode>::ENCODING,
        ],
    );
}

/// Reads the CAMetalLayer installed by wgpu from the AppKit view after surface
/// configuration. This is evidence only: it never mutates the layer.
#[cfg(target_os = "macos")]
pub fn metal_layer_presentation(ns_view: NonNull<c_void>) -> Result<MetalLayerPresentation> {
    use anyhow::Context;
    use objc2::{msg_send, runtime::AnyClass, runtime::AnyObject, MainThreadMarker};
    use std::ffi::CStr;

    MainThreadMarker::new().context("Metal layer probe is not on the macOS main thread")?;
    let metal_class = AnyClass::get(
        CStr::from_bytes_with_nul(b"CAMetalLayer\0").expect("static CAMetalLayer class name"),
    )
    .context("CAMetalLayer class is unavailable")?;
    // SAFETY: `ns_view` comes from winit's live AppKit raw window handle and this
    // function is restricted to the main thread for the NSView lookup.
    let view = unsafe { ns_view.cast::<AnyObject>().as_ref() };
    let root_layer: *mut AnyObject = unsafe { msg_send![view, layer] };
    let root_layer = NonNull::new(root_layer).context("AppKit view has no root layer")?;

    // SAFETY: every pointer below is retained by the live NSView layer tree for
    // the duration of this synchronous read. Selectors are standard CALayer,
    // NSArray and CAMetalLayer accessors with the declared ABI types.
    let metal_layer = unsafe {
        let root = root_layer.as_ref();
        let root_is_metal: bool = msg_send![root, isKindOfClass: metal_class];
        if root_is_metal {
            root_layer
        } else {
            let sublayers: *mut AnyObject = msg_send![root, sublayers];
            let sublayers =
                NonNull::new(sublayers).context("AppKit root layer has no sublayers")?;
            let count: usize = msg_send![sublayers.as_ref(), count];
            let mut found = None;
            for index in 0..count {
                let layer: *mut AnyObject = msg_send![sublayers.as_ref(), objectAtIndex: index];
                let Some(layer) = NonNull::new(layer) else {
                    continue;
                };
                let is_metal: bool = msg_send![layer.as_ref(), isKindOfClass: metal_class];
                if is_metal {
                    found = Some(layer);
                    break;
                }
            }
            found.context("wgpu CAMetalLayer is absent from the AppKit layer tree")?
        }
    };
    // SAFETY: `metal_layer` was runtime-checked as CAMetalLayer immediately
    // above, and both queried properties are read-only here.
    let (drawable_size, maximum_drawable_count): (CoreGraphicsSize, usize) = unsafe {
        (
            msg_send![metal_layer.as_ref(), drawableSize],
            msg_send![metal_layer.as_ref(), maximumDrawableCount],
        )
    };
    Ok(MetalLayerPresentation {
        drawable_width: f64::from(drawable_size.width),
        drawable_height: f64::from(drawable_size.height),
        maximum_drawable_count,
    })
}

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

#[cfg(all(test, not(target_os = "macos")))]
mod tests {
    use super::set_application_icon;

    #[test]
    fn non_macos_targets_report_the_platform_mismatch() {
        let err = set_application_icon(b"icns").expect_err("non-macOS apply must fail");
        assert_eq!(
            err.to_string(),
            "macOS application icon apply is unavailable on this target"
        );
    }
}
