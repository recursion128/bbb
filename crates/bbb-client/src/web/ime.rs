//! Browser CJK IME composition proxy and DevTools shortcut passthrough.

use winit::keyboard::{KeyCode, PhysicalKey};

#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, cell::RefCell, sync::Arc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{closure::Closure, JsCast};
#[cfg(target_arch = "wasm32")]
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::ElementState,
};

#[cfg(target_arch = "wasm32")]
use crate::simulation::commands::{CommandQueue, LoopWake, SimulationCommand};

/// Returns `true` for standard browser DevTools key chords (`F12`,
/// `Ctrl+Shift+I`, and `Cmd+Option+I`) so DOM `keydown`/`keyup` listeners never
/// swallow them with `preventDefault()`.
pub(crate) fn is_browser_devtools_shortcut(
    key: Option<&str>,
    code: Option<&str>,
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
) -> bool {
    let is_f12 = !ctrl && !alt && !meta && !shift && (key == Some("F12") || code == Some("F12"));
    if is_f12 {
        return true;
    }
    let key_is_i = key.is_some_and(|k| k.eq_ignore_ascii_case("i")) || code == Some("KeyI");
    let is_ctrl_shift_i = ctrl && shift && !alt && !meta && key_is_i;
    let is_cmd_option_i = meta && alt && !ctrl && key_is_i;
    is_ctrl_shift_i || is_cmd_option_i
}

pub(crate) fn dom_code_to_physical_key(code: &str) -> Option<PhysicalKey> {
    let key_code = match code {
        "Backquote" => KeyCode::Backquote,
        "Backslash" => KeyCode::Backslash,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Comma" => KeyCode::Comma,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "Equal" => KeyCode::Equal,
        "IntlBackslash" => KeyCode::IntlBackslash,
        "IntlRo" => KeyCode::IntlRo,
        "IntlYen" => KeyCode::IntlYen,
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Minus" => KeyCode::Minus,
        "Period" => KeyCode::Period,
        "Quote" => KeyCode::Quote,
        "Semicolon" => KeyCode::Semicolon,
        "Slash" => KeyCode::Slash,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "Backspace" => KeyCode::Backspace,
        "CapsLock" => KeyCode::CapsLock,
        "ContextMenu" => KeyCode::ContextMenu,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "Enter" => KeyCode::Enter,
        "MetaLeft" => KeyCode::SuperLeft,
        "MetaRight" => KeyCode::SuperRight,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        "Delete" => KeyCode::Delete,
        "End" => KeyCode::End,
        "Home" => KeyCode::Home,
        "Insert" => KeyCode::Insert,
        "PageDown" => KeyCode::PageDown,
        "PageUp" => KeyCode::PageUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "NumLock" => KeyCode::NumLock,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadDecimal" => KeyCode::NumpadDecimal,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "NumpadEqual" => KeyCode::NumpadEqual,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "Escape" => KeyCode::Escape,
        "PrintScreen" => KeyCode::PrintScreen,
        "ScrollLock" => KeyCode::ScrollLock,
        "Pause" => KeyCode::Pause,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        _ => return None,
    };
    Some(PhysicalKey::Code(key_code))
}

#[cfg(target_arch = "wasm32")]
fn is_non_printable_navigation_or_action_key(code: &str) -> bool {
    matches!(
        code,
        "Escape"
            | "Enter"
            | "NumpadEnter"
            | "Backspace"
            | "Delete"
            | "Tab"
            | "ArrowUp"
            | "ArrowDown"
            | "ArrowLeft"
            | "ArrowRight"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
            | "F1"
            | "F2"
            | "F3"
            | "F4"
            | "F5"
            | "F6"
            | "F7"
            | "F8"
            | "F9"
            | "F10"
            | "F11"
    )
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_IME_ALLOWED: Cell<bool> = const { Cell::new(false) };
    static WEB_IME_COMPOSING: Cell<bool> = const { Cell::new(false) };
    static WEB_IME_INPUT: RefCell<Option<web_sys::HtmlInputElement>> = const { RefCell::new(None) };
    static WEB_CANVAS_EL: RefCell<Option<web_sys::HtmlCanvasElement>> = const { RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn ime_proxy_owns_focus() -> bool {
    if !WEB_IME_ALLOWED.with(Cell::get) {
        return false;
    }
    let Some(active) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element())
    else {
        return false;
    };
    WEB_IME_INPUT.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|input| &active == AsRef::<web_sys::Element>::as_ref(input))
    })
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_web_ime_allowed(allowed: bool) {
    let previous = WEB_IME_ALLOWED.with(|c| c.replace(allowed));
    if !allowed {
        WEB_IME_COMPOSING.with(|c| c.set(false));
    }
    WEB_IME_INPUT.with(|slot| {
        let Some(input) = slot.borrow().clone() else {
            return;
        };
        input.set_value("");
        if allowed {
            input.set_disabled(false);
            let _ = input.style().set_property("pointer-events", "none");
            let _ = input.focus();
        } else {
            input.set_disabled(true);
            if previous {
                let _ = input.blur();
            }
            WEB_CANVAS_EL.with(|canvas_slot| {
                if let Some(canvas) = canvas_slot.borrow().as_ref() {
                    let _ = canvas.focus();
                }
            });
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn pack_controls_own_focus() -> bool {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return false;
    };
    let Some(active) = document.active_element() else {
        return false;
    };
    active
        .closest("#bbb-pack-controls")
        .ok()
        .flatten()
        .is_some()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn ensure_web_ime_focus_if_allowed() {
    if !WEB_IME_ALLOWED.with(Cell::get) || super::dom_has_pointer_lock() {
        return;
    }
    if !super::document_has_focus() || ime_proxy_owns_focus() || pack_controls_own_focus() {
        return;
    }
    WEB_IME_INPUT.with(|slot| {
        if let Some(input) = slot.borrow().as_ref() {
            input.set_disabled(false);
            let _ = input.focus();
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_web_ime_cursor_area(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale_factor: f64,
) {
    let dpr = scale_factor.max(1.0);
    let css_x = (f64::from(position.x) / dpr).max(0.0);
    let css_y = (f64::from(position.y) / dpr).max(0.0);
    let css_w = (f64::from(size.width) / dpr).max(1.0);
    let css_h = (f64::from(size.height) / dpr).max(12.0);
    WEB_IME_INPUT.with(|slot| {
        let Some(input) = slot.borrow().clone() else {
            return;
        };
        let style = input.style();
        let _ = style.set_property("left", &format!("{css_x:.1}px"));
        let _ = style.set_property("top", &format!("{css_y:.1}px"));
        let _ = style.set_property("width", &format!("{css_w:.1}px"));
        let _ = style.set_property("height", &format!("{css_h:.1}px"));
    });
}

#[cfg(target_arch = "wasm32")]
pub(super) fn install_web_ime_bridge(
    document: &web_sys::Document,
    canvas: &web_sys::HtmlCanvasElement,
    commands: CommandQueue,
    loop_wake: Arc<LoopWake>,
) {
    let Ok(el) = document.create_element("input") else {
        return;
    };
    let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else {
        return;
    };
    input.set_id("bbb-ime-input");
    input.set_type("text");
    input.set_disabled(true);
    let _ = input.set_attribute("autocomplete", "off");
    let _ = input.set_attribute("autocorrect", "off");
    let _ = input.set_attribute("autocapitalize", "off");
    let _ = input.set_attribute("spellcheck", "false");
    let style = input.style();
    let _ = style.set_property("position", "fixed");
    let _ = style.set_property("left", "0px");
    let _ = style.set_property("top", "0px");
    let _ = style.set_property("width", "1px");
    let _ = style.set_property("height", "12px");
    let _ = style.set_property("opacity", "0");
    let _ = style.set_property("padding", "0");
    let _ = style.set_property("margin", "0");
    let _ = style.set_property("border", "0");
    let _ = style.set_property("outline", "none");
    let _ = style.set_property("background", "transparent");
    let _ = style.set_property("color", "transparent");
    let _ = style.set_property("pointer-events", "none");
    let _ = style.set_property("z-index", "-1");
    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
    }

    WEB_CANVAS_EL.with(|slot| *slot.borrow_mut() = Some(canvas.clone()));
    WEB_IME_INPUT.with(|slot| *slot.borrow_mut() = Some(input.clone()));

    let comp_start_cmds = commands.clone();
    let on_composition_start =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            WEB_IME_COMPOSING.with(|c| c.set(true));
            let data = super::js_string_prop(&ev, "data").unwrap_or_default();
            let end = data.len();
            comp_start_cmds.push(SimulationCommand::ImePreedit {
                text: data,
                cursor_range: Some((end, end)),
            });
        });
    let _ = input.add_event_listener_with_callback(
        "compositionstart",
        on_composition_start.as_ref().unchecked_ref(),
    );
    on_composition_start.forget();

    let comp_update_cmds = commands.clone();
    let on_composition_update =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            WEB_IME_COMPOSING.with(|c| c.set(true));
            let data = super::js_string_prop(&ev, "data").unwrap_or_default();
            let end = data.len();
            comp_update_cmds.push(SimulationCommand::ImePreedit {
                text: data,
                cursor_range: Some((end, end)),
            });
        });
    let _ = input.add_event_listener_with_callback(
        "compositionupdate",
        on_composition_update.as_ref().unchecked_ref(),
    );
    on_composition_update.forget();

    let comp_end_cmds = commands.clone();
    let comp_end_input = input.clone();
    let on_composition_end =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
            WEB_IME_COMPOSING.with(|c| c.set(false));
            let committed = super::js_string_prop(&ev, "data")
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| comp_end_input.value());
            comp_end_input.set_value("");
            comp_end_cmds.push(SimulationCommand::ImePreedit {
                text: String::new(),
                cursor_range: None,
            });
            if !committed.is_empty() {
                comp_end_cmds.push(SimulationCommand::ImeCommit(committed));
            }
        });
    let _ = input.add_event_listener_with_callback(
        "compositionend",
        on_composition_end.as_ref().unchecked_ref(),
    );
    on_composition_end.forget();

    let input_event_cmds = commands.clone();
    let input_el_for_input = input.clone();
    let on_input = Closure::<dyn FnMut(web_sys::Event)>::new(move |ev: web_sys::Event| {
        let composing =
            WEB_IME_COMPOSING.with(Cell::get) || super::js_bool_prop(&ev, "isComposing");
        if composing {
            return;
        }
        let val = input_el_for_input.value();
        if !val.is_empty() {
            input_el_for_input.set_value("");
            input_event_cmds.push(SimulationCommand::ImeCommit(val));
        }
    });
    let _ = input.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
    on_input.forget();

    let keydown_cmds = commands.clone();
    let on_keydown =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let ctrl = ev.ctrl_key();
            let shift = ev.shift_key();
            let alt = ev.alt_key();
            let meta = ev.meta_key();
            let key = ev.key();
            let code = ev.code();
            if is_browser_devtools_shortcut(Some(&key), Some(&code), ctrl, shift, alt, meta) {
                return;
            }
            if WEB_IME_COMPOSING.with(Cell::get) || ev.is_composing() || ev.key_code() == 229 {
                return;
            }
            let is_shortcut = (ctrl || meta) && !alt;
            let is_paste = is_shortcut && !shift && code == "KeyV";
            super::set_shortcut_modifier_state(is_shortcut, is_paste);
            if is_paste {
                return;
            }
            if is_shortcut || is_non_printable_navigation_or_action_key(&code) {
                ev.prevent_default();
            }
            if let Some(physical_key) = dom_code_to_physical_key(&code) {
                if physical_key == PhysicalKey::Code(KeyCode::F9) && ev.repeat() {
                    return;
                }
                keydown_cmds.push(SimulationCommand::Key {
                    physical_key,
                    state: ElementState::Pressed,
                    clipboard_text: None,
                });
            }
        });
    let _ = input.add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
    on_keydown.forget();

    let keyup_cmds = commands.clone();
    let keyup_wake = Arc::clone(&loop_wake);
    let on_keyup =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            let ctrl = ev.ctrl_key();
            let shift = ev.shift_key();
            let alt = ev.alt_key();
            let meta = ev.meta_key();
            let key = ev.key();
            let code = ev.code();
            if is_browser_devtools_shortcut(Some(&key), Some(&code), ctrl, shift, alt, meta) {
                return;
            }
            super::on_dom_keyup_paste_check(&code, (ctrl || meta) && !alt, &keyup_wake);
            if WEB_IME_COMPOSING.with(Cell::get) || ev.is_composing() || ev.key_code() == 229 {
                return;
            }
            if let Some(physical_key) = dom_code_to_physical_key(&code) {
                keyup_cmds.push(SimulationCommand::Key {
                    physical_key,
                    state: ElementState::Released,
                    clipboard_text: None,
                });
            }
        });
    let _ = input.add_event_listener_with_callback("keyup", on_keyup.as_ref().unchecked_ref());
    on_keyup.forget();

    let blur_cmds = commands;
    let on_blur = Closure::<dyn FnMut()>::new(move || {
        WEB_IME_COMPOSING.with(|c| c.set(false));
        let active_is_canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .is_some_and(|active| {
                WEB_CANVAS_EL.with(|c| {
                    c.borrow()
                        .as_ref()
                        .is_some_and(|canvas| &active == AsRef::<web_sys::Element>::as_ref(canvas))
                })
            });
        if !active_is_canvas && !super::document_has_focus() {
            blur_cmds.push(SimulationCommand::FocusChanged(false));
        }
    });
    let _ = input.add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref());
    on_blur.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devtools_shortcut_filter_matches_f12_ctrl_shift_i_and_cmd_option_i_only() {
        assert!(is_browser_devtools_shortcut(
            Some("F12"),
            Some("F12"),
            false,
            false,
            false,
            false
        ));
        assert!(is_browser_devtools_shortcut(
            Some("I"),
            Some("KeyI"),
            true,
            true,
            false,
            false
        ));
        assert!(is_browser_devtools_shortcut(
            Some("i"),
            Some("KeyI"),
            true,
            true,
            false,
            false
        ));
        assert!(is_browser_devtools_shortcut(
            Some("I"),
            Some("KeyI"),
            false,
            false,
            true,
            true
        ));
        // macOS Option+I produces a dead-key glyph for `event.key`, so `code == "KeyI"` covers it.
        assert!(is_browser_devtools_shortcut(
            Some("Dead"),
            Some("KeyI"),
            false,
            false,
            true,
            true
        ));

        // Gameplay and debug keys must never be treated as DevTools shortcuts.
        assert!(!is_browser_devtools_shortcut(
            Some("F3"),
            Some("F3"),
            false,
            false,
            false,
            false
        ));
        assert!(!is_browser_devtools_shortcut(
            Some("i"),
            Some("KeyI"),
            true,
            false,
            false,
            false
        ));
        assert!(!is_browser_devtools_shortcut(
            Some("I"),
            Some("KeyI"),
            false,
            true,
            false,
            false
        ));
        assert!(!is_browser_devtools_shortcut(
            Some("F11"),
            Some("F11"),
            false,
            false,
            false,
            false
        ));
    }

    #[test]
    fn dom_code_mapping_covers_chat_and_navigation_keys() {
        assert_eq!(
            dom_code_to_physical_key("KeyT"),
            Some(PhysicalKey::Code(KeyCode::KeyT))
        );
        assert_eq!(
            dom_code_to_physical_key("Enter"),
            Some(PhysicalKey::Code(KeyCode::Enter))
        );
        assert_eq!(
            dom_code_to_physical_key("Escape"),
            Some(PhysicalKey::Code(KeyCode::Escape))
        );
        assert_eq!(
            dom_code_to_physical_key("Backspace"),
            Some(PhysicalKey::Code(KeyCode::Backspace))
        );
        assert!(dom_code_to_physical_key("UnknownCustomKey").is_none());
    }
}
