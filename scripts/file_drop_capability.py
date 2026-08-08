"""Derive the Vanilla/winit failed-file-drop capability contract.

This is blocker evidence, not an implementation-status table.  The report is
rebuilt from the verified Vanilla workspace, Cargo.lock, the resolved winit
source package, and BBB's window-event adapter.  If any side gains a usable
batch/failure channel the derivation fails closed so the open gap must be
re-audited instead of silently retaining a stale external-blocker ruling.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib


VANILLA_VERSION = "26.1"
WINIT_VERSION = "0.29.15"
MECHANISM_IDS = ("hud-frame-extract-order", "platform-window-lifecycle")
OWNER_IDS = (
    "method:net.minecraft.client.gui.components.toasts.SystemToast#"
    "onFileDropFailure(final Minecraft minecraft,final int count)",
)
REPRODUCE_COMMAND = (
    "scripts/python scripts/generate-file-drop-capability.py --vanilla-root "
    '"$(scripts/vanilla path 26.1 --kind root)" --check'
)

MOUSE_HANDLER = "net/minecraft/client/MouseHandler.java"
SYSTEM_TOAST = "net/minecraft/client/gui/components/toasts/SystemToast.java"
WINIT_EVENT = "src/event.rs"
WINIT_WINDOWS_DROP = "src/platform_impl/windows/drop_handler.rs"
WINIT_MACOS_DROP = "src/platform_impl/macos/window_delegate.rs"
WINIT_X11_PROCESSOR = "src/platform_impl/linux/x11/event_processor.rs"
WINIT_X11_DND = "src/platform_impl/linux/x11/dnd.rs"
WINIT_WAYLAND_ROOT = "src/platform_impl/linux/wayland"

GENERATOR_INPUT_PATHS = (
    "Cargo.lock",
    "scripts/file_drop_capability.py",
    "scripts/generate-file-drop-capability.py",
)


class FileDropCapabilityError(RuntimeError):
    """The pinned sources no longer prove the recorded capability ruling."""


def sha256_file(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def _line(text: str, pattern: str, context: str) -> int:
    match = re.search(pattern, text, re.MULTILINE | re.DOTALL)
    if match is None:
        raise FileDropCapabilityError(f"{context}: required source contract is absent")
    return text.count("\n", 0, match.start()) + 1


def _forbid(text: str, pattern: str, context: str) -> None:
    if re.search(pattern, text, re.MULTILINE | re.DOTALL | re.IGNORECASE):
        raise FileDropCapabilityError(f"{context}: source now exposes a failed-drop channel")


def _receipt(root: Path, relative: str, anchors: dict[str, str]) -> dict[str, object]:
    path = root / relative
    if not path.is_file():
        raise FileDropCapabilityError(f"source receipt is absent: {relative}")
    text = path.read_text(encoding="utf-8")
    return {
        "anchors": [
            {"line": _line(text, pattern, f"{relative}#{name}"), "symbol": name}
            for name, pattern in anchors.items()
        ],
        "path": relative,
        "sha256": sha256_file(path),
    }


def _rust_enum_body(text: str, declaration: str) -> str:
    start = text.find(declaration)
    if start < 0:
        raise FileDropCapabilityError(f"missing Rust declaration: {declaration}")
    opening = text.find("{", start)
    if opening < 0:
        raise FileDropCapabilityError(f"missing Rust body: {declaration}")
    depth = 0
    for index in range(opening, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[opening + 1 : index]
    raise FileDropCapabilityError(f"unbalanced Rust body: {declaration}")


def analyze_public_event(text: str) -> dict[str, object]:
    """Prove that winit publishes successes one-by-one and no failure receipt."""
    body = _rust_enum_body(text, "pub enum WindowEvent")
    if len(re.findall(r"\bDroppedFile\s*\(\s*PathBuf\s*\)", body)) != 1:
        raise FileDropCapabilityError("winit WindowEvent::DroppedFile shape changed")
    if "this event will be emitted for each file\n    /// separately" not in body:
        raise FileDropCapabilityError("winit multi-file delivery contract changed")
    _forbid(
        body,
        r"(?:drop(?:ped)?[^\n,{}()]{0,60}fail|fail[^\n,{}()]{0,60}drop)",
        "winit WindowEvent",
    )
    return {
        "batch_identity_exposed": False,
        "failed_path_count_exposed": False,
        "success_event": "DroppedFile(PathBuf)",
        "success_delivery": "one-event-per-file",
    }


def analyze_x11(event_text: str, dnd_text: str) -> dict[str, object]:
    """Prove X11 parse errors are retained internally and not emitted publicly."""
    error_variants = re.findall(
        r"^\s{4}(EmptyData|InvalidUtf8|HostnameSpecified|UnexpectedProtocol|UnresolvablePath)(?:\([^\n]+\))?,?$",
        _rust_enum_body(dnd_text, "pub enum DndDataParseError"),
        re.MULTILINE,
    )
    expected = [
        "EmptyData",
        "InvalidUtf8",
        "HostnameSpecified",
        "UnexpectedProtocol",
        "UnresolvablePath",
    ]
    if error_variants != expected:
        raise FileDropCapabilityError(
            f"winit X11 DND parse-error surface changed: {error_variants!r}"
        )
    if "pub result: Option<Result<Vec<PathBuf>, DndDataParseError>>" not in dnd_text:
        raise FileDropCapabilityError("winit X11 DND result shape changed")
    if "if let Some(Ok(ref path_list)) = self.dnd.result" not in event_text:
        raise FileDropCapabilityError("winit X11 drop success gate changed")
    if "self.dnd.result = Some(parse_result);" not in event_text:
        raise FileDropCapabilityError("winit X11 parse result retention changed")
    drop_block = event_text[
        event_text.index("if xev.message_type == atoms[XdndDrop]") :
        event_text.index("if xev.message_type == atoms[XdndLeave]")
    ]
    _forbid(
        drop_block,
        r"(?:Some\(Err|Err\s*\(|failed_count|failure_count)",
        "winit X11 drop dispatch",
    )
    return {
        "batch_identity_exposed": False,
        "failed_path_count_exposed": False,
        "parse_error_delivery": "no-WindowEvent",
        "parse_error_variants": error_variants,
        "success_delivery": "DroppedFile-per-path-only-on-Ok-batch",
    }


def _tree_receipt(root: Path, relative: str) -> dict[str, object]:
    tree = root / relative
    paths = sorted(path for path in tree.rglob("*.rs") if path.is_file())
    if not paths:
        raise FileDropCapabilityError(f"empty winit source tree: {relative}")
    rows = []
    combined = hashlib.sha256()
    for path in paths:
        name = path.relative_to(root).as_posix()
        digest = sha256_file(path)
        combined.update(f"{name}\t{digest}\n".encode())
        rows.append(path.read_text(encoding="utf-8"))
    joined = "\n".join(rows)
    if re.search(r"\b(?:DroppedFile|HoveredFile|HoveredFileCancelled)\b", joined):
        raise FileDropCapabilityError("winit Wayland now exposes file-drop events")
    return {
        "file_count": len(paths),
        "path": relative,
        "searched_events": ["DroppedFile", "HoveredFile", "HoveredFileCancelled"],
        "sha256_tree": combined.hexdigest(),
    }


def _winit_lock_record(cargo_lock: Path) -> dict[str, str]:
    value = tomllib.loads(cargo_lock.read_text(encoding="utf-8"))
    records = [
        item
        for item in value.get("package", [])
        if item.get("name") == "winit" and item.get("version") == WINIT_VERSION
    ]
    if len(records) != 1:
        raise FileDropCapabilityError(
            f"Cargo.lock must contain exactly one winit {WINIT_VERSION} package"
        )
    checksum = records[0].get("checksum")
    if not isinstance(checksum, str) or not re.fullmatch(r"[0-9a-f]{64}", checksum):
        raise FileDropCapabilityError("Cargo.lock winit registry checksum is absent")
    return {"name": "winit", "registry_checksum": checksum, "version": WINIT_VERSION}


def resolve_winit_root(repo_root: Path) -> Path:
    result = subprocess.run(
        ["cargo", "metadata", "--format-version=1", "--locked", "--offline"],
        cwd=repo_root,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise FileDropCapabilityError(
            "cargo metadata could not resolve the pinned winit source: "
            + result.stderr.strip()
        )
    metadata = json.loads(result.stdout)
    packages = [
        item
        for item in metadata.get("packages", [])
        if item.get("name") == "winit" and item.get("version") == WINIT_VERSION
    ]
    if len(packages) != 1:
        raise FileDropCapabilityError(
            f"cargo metadata must resolve exactly one winit {WINIT_VERSION} package"
        )
    return Path(packages[0]["manifest_path"]).resolve().parent


def build_report(
    repo_root: Path, vanilla_root: Path, winit_root: Path
) -> dict[str, object]:
    dependency = _winit_lock_record(repo_root / "Cargo.lock")
    winit_manifest = tomllib.loads((winit_root / "Cargo.toml").read_text(encoding="utf-8"))
    package = winit_manifest.get("package", {})
    if package.get("name") != "winit" or package.get("version") != WINIT_VERSION:
        raise FileDropCapabilityError("resolved winit manifest does not match Cargo.lock")

    mouse_text = (vanilla_root / MOUSE_HANDLER).read_text(encoding="utf-8")
    toast_text = (vanilla_root / SYSTEM_TOAST).read_text(encoding="utf-8")
    mouse_receipt = _receipt(
        vanilla_root,
        MOUSE_HANDLER,
        {
            "onDrop(validPaths, failedCount)": r"private void onDrop\([^\n]+failedCount\)",
            "InvalidPathException increments failedCount": r"catch \(InvalidPathException e\) \{\s*failedCount\+\+;",
            "all-invalid batch is not dispatched": r"if \(!names\.isEmpty\(\)\) \{\s*int finalFailedCount",
            "failure toast requires positive count": r"if \(failedCount > 0\) \{\s*SystemToast\.onFileDropFailure",
        },
    )
    toast_receipt = _receipt(
        vanilla_root,
        SYSTEM_TOAST,
        {
            "onFileDropFailure": r"public static void onFileDropFailure\([^\n]+int count\)",
            "FILE_DROP_FAILURE identity": r"SystemToast\.SystemToastId\.FILE_DROP_FAILURE",
            "localized failed-count detail": r'Component\.translatable\("gui\.fileDropFailure\.detail", count\)',
        },
    )
    for required in ("Paths.get(name)", "names.add(Paths.get(name))"):
        if required not in mouse_text:
            raise FileDropCapabilityError(f"Vanilla MouseHandler contract changed: {required}")
    for required in ("gui.fileDropFailure.title", "gui.fileDropFailure.detail"):
        if required not in toast_text:
            raise FileDropCapabilityError(f"Vanilla SystemToast contract changed: {required}")

    event_text = (winit_root / WINIT_EVENT).read_text(encoding="utf-8")
    windows_text = (winit_root / WINIT_WINDOWS_DROP).read_text(encoding="utf-8")
    macos_text = (winit_root / WINIT_MACOS_DROP).read_text(encoding="utf-8")
    x11_event_text = (winit_root / WINIT_X11_PROCESSOR).read_text(encoding="utf-8")
    x11_dnd_text = (winit_root / WINIT_X11_DND).read_text(encoding="utf-8")
    public_contract = analyze_public_event(event_text)
    x11_contract = analyze_x11(x11_event_text, x11_dnd_text)

    windows_receipt = _receipt(
        winit_root,
        WINIT_WINDOWS_DROP,
        {
            "success-only callback": r"F: Fn\(PathBuf\)",
            "one event per converted filename": r"event: DroppedFile\(filename\)",
            "non-file and GetData failures return None": r"get_data_result == DV_E_FORMATETC[\s\S]+?None\s*\}\s*else",
        },
    )
    _forbid(
        windows_text,
        r"(?:failed_count|failure_count|DroppedFileFail)",
        "winit Windows drop backend",
    )
    macos_receipt = _receipt(
        winit_root,
        WINIT_MACOS_DROP,
        {
            "one event per NSString": r"filenames\.into_iter\(\)\.for_each\(\|file\|",
            "NSString converts directly to PathBuf": r"PathBuf::from\(file\.to_string\(\)\)",
            "success-only public event": r"WindowEvent::DroppedFile\(path\)",
        },
    )
    _forbid(
        macos_text,
        r"(?:failed_count|failure_count|DroppedFileFail)",
        "winit macOS drop backend",
    )
    x11_processor_receipt = _receipt(
        winit_root,
        WINIT_X11_PROCESSOR,
        {
            "drop dispatch accepts only Ok": r"if let Some\(Ok\(ref path_list\)\) = self\.dnd\.result",
            "success-only public event": r"event: WindowEvent::DroppedFile\(path\.clone\(\)\)",
            "parse result retained internally": r"self\.dnd\.result = Some\(parse_result\);",
        },
    )
    x11_dnd_receipt = _receipt(
        winit_root,
        WINIT_X11_DND,
        {
            "fallible batch result": r"Option<Result<Vec<PathBuf>, DndDataParseError>>",
            "parse_data": r"pub fn parse_data\([^\n]+Result<Vec<PathBuf>, DndDataParseError>",
            "invalid UTF-8 is representable internally": r"InvalidUtf8\(Utf8Error\)",
        },
    )

    bbb_main = "crates/bbb-native/src/main.rs"
    bbb_receipt = _receipt(
        repo_root,
        bbb_main,
        {
            "window-event adapter": r"fn enqueue_simulation_window_event\(",
            "failed-drop capability blocker": r"failed-file-drop capability receipt",
        },
    )
    main_text = (repo_root / bbb_main).read_text(encoding="utf-8")
    adapter_body_start = main_text.index("fn enqueue_simulation_window_event(")
    adapter_body_end = main_text.index("\n}\n", adapter_body_start) + 2
    adapter_body = main_text[adapter_body_start:adapter_body_end]
    if "WindowEvent::DroppedFile" in adapter_body:
        raise FileDropCapabilityError(
            "BBB window adapter now consumes DroppedFile; re-audit the blocker"
        )

    generator_inputs = [
        {"path": relative, "sha256": sha256_file(repo_root / relative)}
        for relative in GENERATOR_INPUT_PATHS
    ]
    return {
        "bbb_event_surface": {
            "adapter": {
                "failed_path_count_consumed": False,
                "source": bbb_receipt,
                "successful_path_event_consumed": False,
            },
            "backends": {
                "linux-wayland": {
                    "contract": "no file-drop WindowEvent implementation",
                    "source_tree": _tree_receipt(winit_root, WINIT_WAYLAND_ROOT),
                },
                "linux-x11": {
                    "contract": x11_contract,
                    "sources": [x11_processor_receipt, x11_dnd_receipt],
                },
                "macos": {
                    "contract": {
                        "batch_identity_exposed": False,
                        "failed_path_count_exposed": False,
                        "success_delivery": "DroppedFile-per-NSString",
                    },
                    "source": macos_receipt,
                },
                "windows": {
                    "contract": {
                        "batch_identity_exposed": False,
                        "failed_path_count_exposed": False,
                        "success_delivery": "DroppedFile-per-PathBuf",
                    },
                    "source": windows_receipt,
                },
            },
            "dependency": dependency,
            "public_event_contract": public_contract,
            "public_event_source": _receipt(
                winit_root,
                WINIT_EVENT,
                {
                    "WindowEvent": r"pub enum WindowEvent",
                    "DroppedFile(PathBuf)": r"DroppedFile\(PathBuf\)",
                    "one event per file": r"event will be emitted for each file\s+/// separately",
                },
            ),
        },
        "blocker": {
            "classification": "external-dependency-event-surface",
            "final_conclusion": (
                "The pinned winit public event stream cannot reproduce Vanilla's per-drop-batch "
                "valid-path list plus exact failed-path count; X11 parse failures are discarded "
                "before WindowEvent delivery and no backend exposes a batch identity or failure count."
            ),
            "missing_condition": (
                "A supported platform adapter must expose one drop batch's valid paths and exact "
                "failed path-conversion count before winit discards or flattens that information."
            ),
            "reopen_condition": (
                "winit or an owned platform adapter exposes drop batch identity plus exact failed "
                "path count, or the supported-platform scope changes."
            ),
            "reproduce_command": REPRODUCE_COMMAND,
        },
        "generated_by": "scripts/generate-file-drop-capability.py",
        "generator_inputs": generator_inputs,
        "mechanism_ids": list(MECHANISM_IDS),
        "owner_ids": list(OWNER_IDS),
        "schema_version": 1,
        "status": "blocked-external",
        "valid": True,
        "vanilla_contract": {
            "all_invalid_batch_dispatches_callback": False,
            "callback_state": ["valid_paths", "failed_path_count"],
            "failure_toast_condition": "valid_paths is non-empty and failed_path_count > 0",
            "sources": [mouse_receipt, toast_receipt],
            "toast_identity": "FILE_DROP_FAILURE",
            "toast_translation_keys": [
                "gui.fileDropFailure.title",
                "gui.fileDropFailure.detail",
            ],
        },
        "vanilla_version": VANILLA_VERSION,
    }


def render_report(repo_root: Path, vanilla_root: Path, winit_root: Path) -> str:
    return json.dumps(
        build_report(repo_root, vanilla_root, winit_root),
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    ) + "\n"
