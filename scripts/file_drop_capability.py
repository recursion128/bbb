"""Derive the Vanilla/winit failed-file-drop scope contract.

This is mechanism evidence, not a second implementation-status table.  The
report is rebuilt from the verified Vanilla workspace, its exact Java 25
LWJGL/GLFW artifacts, Cargo.lock, the resolved winit source package, and BBB's
window-event adapter.  It proves whether the failed-path branch or a valid-path
consumer is observable in the declared Linux/Wayland surface.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
import zipfile


VANILLA_VERSION = "26.1"
WINIT_VERSION = "0.30.13"
LWJGL_VERSION = "3.4.1"
LWJGL_COMMIT = "b800ccffab14396fc529ddb6c931b7c5c5226763"
GLFW_COMMIT = "9352d8fe93cd443be18157abe81f16500549aec0"
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
SCREEN = "net/minecraft/client/gui/screens/Screen.java"
PACK_SELECTION_SCREEN = "net/minecraft/client/gui/screens/packs/PackSelectionScreen.java"
WINIT_EVENT = "src/event.rs"
WINIT_WINDOWS_DROP = "src/platform_impl/windows/drop_handler.rs"
WINIT_MACOS_DROP = "src/platform_impl/macos/window_delegate.rs"
WINIT_X11_PROCESSOR = "src/platform_impl/linux/x11/event_processor.rs"
WINIT_X11_DND = "src/platform_impl/linux/x11/dnd.rs"
WINIT_WAYLAND_ROOT = "src/platform_impl/linux/wayland"
BBB_MAIN = "crates/bbb-native/src/main.rs"

UPSTREAM_SOURCE_RECEIPTS = (
    {
        "owner": "GLFW::_glfwParseUriList",
        "repository": "https://github.com/glfw/glfw",
        "revision": GLFW_COMMIT,
        "path": "src/init.c",
        "sha256": "b6ff7576ff61505c06043da7a8a14e4e1f15c30b07c7953e2a838f25ed138893",
    },
    {
        "owner": "GLFW::dataDeviceHandleDrop",
        "repository": "https://github.com/glfw/glfw",
        "revision": GLFW_COMMIT,
        "path": "src/wl_window.c",
        "sha256": "0371bbf004408002563cd03055d3949fe6aff56112f430ad1fe8d019e52aceae",
    },
    {
        "owner": "LWJGL::GLFWDropCallback.getName",
        "repository": "https://github.com/LWJGL/lwjgl3",
        "revision": LWJGL_COMMIT,
        "path": "modules/lwjgl/glfw/src/generated/java/org/lwjgl/glfw/GLFWDropCallback.java",
        "sha256": "8f7bde750d1b99785f4e0fa40831a790baacedf7a08be329be0d9dfed87cf81d",
    },
    {
        "owner": "LWJGL::MemoryUtil.memUTF8",
        "repository": "https://github.com/LWJGL/lwjgl3",
        "revision": LWJGL_COMMIT,
        "path": "modules/lwjgl/core25/src/main/java/org/lwjgl/system/MemoryUtil.java",
        "sha256": "dd76c861ea8c8a25ee0ba6c0f3cd041ec8a07a4f1bc473f30e10a9e845603159",
    },
    {
        "owner": "LWJGL::MultiReleaseTextDecoding.decodeUTF8",
        "repository": "https://github.com/LWJGL/lwjgl3",
        "revision": LWJGL_COMMIT,
        "path": "modules/lwjgl/core11/src/main/java/org/lwjgl/system/MultiReleaseTextDecoding.java",
        "sha256": "b3bf1c0594bb7c264787df50cfaace52bd9f6ba9ee3026049d7e60d08da5d5b0",
    },
    {
        "owner": "OpenJDK::UnixPath",
        "repository": "https://github.com/openjdk/jdk25u",
        "revision": "78770bfaefd23ae77ec4f8ddd769c1c2d9c282df",
        "release": "jdk-25.0.1+8",
        "path": "src/java.base/unix/classes/sun/nio/fs/UnixPath.java",
        "sha256": "515557ab1e13000656f13a6dc157b73894c720f4f990547766b8c0fc48f688c2",
    },
)

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


def sha1_file(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha1").hexdigest()


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


def _on_files_drop_occurrences(vanilla_root: Path) -> list[dict[str, object]]:
    rows = []
    for path in sorted(vanilla_root.rglob("*.java")):
        count = path.read_text(encoding="utf-8").count("onFilesDrop(")
        if count:
            rows.append(
                {
                    "count": count,
                    "path": path.relative_to(vanilla_root).as_posix(),
                    "sha256": sha256_file(path),
                }
            )
    expected_paths = (MOUSE_HANDLER, PACK_SELECTION_SCREEN, SCREEN)
    expected = [
        {
            "count": 1,
            "path": relative,
            "sha256": sha256_file(vanilla_root / relative),
        }
        for relative in expected_paths
    ]
    if rows != sorted(expected, key=lambda row: row["path"]):
        raise FileDropCapabilityError(f"Vanilla onFilesDrop surface changed: {rows!r}")
    return rows


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
    rustc = subprocess.run(
        ["rustc", "-vV"],
        cwd=repo_root,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    host = next(
        (
            line.removeprefix("host:").strip()
            for line in rustc.stdout.splitlines()
            if line.startswith("host:")
        ),
        "",
    )
    if rustc.returncode != 0 or not host:
        raise FileDropCapabilityError(
            "rustc -vV could not resolve the host target for cargo metadata: "
            + rustc.stderr.strip()
        )
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--format-version=1",
            "--locked",
            "--offline",
            "--filter-platform",
            host,
        ],
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


def _library_artifact(
    version: dict[str, object], download_root: Path, coordinate: str
) -> tuple[str, str, Path]:
    libraries = [
        item for item in version.get("libraries", []) if item.get("name") == coordinate
    ]
    if len(libraries) != 1:
        raise FileDropCapabilityError(
            f"Vanilla metadata must contain exactly one {coordinate} library"
        )
    artifact = libraries[0].get("downloads", {}).get("artifact", {})
    relative = artifact.get("path")
    expected_sha1 = artifact.get("sha1")
    if not isinstance(relative, str) or not isinstance(expected_sha1, str):
        raise FileDropCapabilityError(
            f"Vanilla library metadata is incomplete: {coordinate}"
        )
    jar_path = download_root / "libraries" / relative
    if not jar_path.is_file() or sha1_file(jar_path) != expected_sha1:
        raise FileDropCapabilityError(f"Vanilla library hash mismatch: {coordinate}")
    return relative, expected_sha1, jar_path


def _zip_entry_receipts(
    archive: zipfile.ZipFile, entries: tuple[str, ...]
) -> list[dict[str, str]]:
    names = set(archive.namelist())
    if not set(entries).issubset(names):
        missing = sorted(set(entries) - names)
        raise FileDropCapabilityError(f"Vanilla LWJGL entries are absent: {missing!r}")
    return [
        {"path": entry, "sha256": hashlib.sha256(archive.read(entry)).hexdigest()}
        for entry in entries
    ]


def _vanilla_runtime_receipt(download_root: Path) -> dict[str, object]:
    version_path = download_root / f"{VANILLA_VERSION}.json"
    if not version_path.is_file():
        raise FileDropCapabilityError(
            f"Vanilla version metadata is absent: {version_path}"
        )
    version = json.loads(version_path.read_text(encoding="utf-8"))
    java_runtime = version.get("javaVersion")
    if java_runtime != {"component": "java-runtime-epsilon", "majorVersion": 25}:
        raise FileDropCapabilityError(
            f"Vanilla Java runtime contract changed: {java_runtime!r}"
        )

    core_relative, core_sha1, core_path = _library_artifact(
        version, download_root, f"org.lwjgl:lwjgl:{LWJGL_VERSION}"
    )
    decoder_entry = (
        "META-INF/versions/11/org/lwjgl/system/MultiReleaseTextDecoding.class"
    )
    memory_entry = "META-INF/versions/25/org/lwjgl/system/MemoryUtil.class"
    with zipfile.ZipFile(core_path) as archive:
        manifest = archive.read("META-INF/MANIFEST.MF")
        if b"Multi-Release: true" not in manifest:
            raise FileDropCapabilityError("Vanilla LWJGL core is no longer multi-release")
        core_entries = _zip_entry_receipts(archive, (decoder_entry, memory_entry))

    glfw_relative, glfw_sha1, glfw_path = _library_artifact(
        version, download_root, f"org.lwjgl:lwjgl-glfw:{LWJGL_VERSION}"
    )
    callback_entry = "org/lwjgl/glfw/GLFWDropCallback.class"
    with zipfile.ZipFile(glfw_path) as archive:
        callback_entries = _zip_entry_receipts(archive, (callback_entry,))

    native_relative, native_sha1, native_path = _library_artifact(
        version,
        download_root,
        f"org.lwjgl:lwjgl-glfw:{LWJGL_VERSION}:natives-linux",
    )

    prefix = "linux/x64/org/lwjgl/glfw/libglfw.so"
    git_entry = f"META-INF/{prefix}.git"
    sha1_entry = f"META-INF/{prefix}.sha1"
    with zipfile.ZipFile(native_path) as archive:
        names = set(archive.namelist())
        required = {prefix, git_entry, sha1_entry}
        if not required.issubset(names):
            raise FileDropCapabilityError("Vanilla GLFW native provenance entries are absent")
        native = archive.read(prefix)
        embedded_commit = archive.read(git_entry).decode("ascii").strip()
        embedded_sha1 = archive.read(sha1_entry).decode("ascii").strip()
    if embedded_commit != GLFW_COMMIT:
        raise FileDropCapabilityError(
            f"Vanilla GLFW commit changed: {embedded_commit!r}"
        )
    actual_native_sha1 = hashlib.sha1(native).hexdigest()
    if embedded_sha1 != actual_native_sha1:
        raise FileDropCapabilityError("Vanilla GLFW embedded native hash mismatch")
    version_marker = b"3.5.0 Wayland X11 GLX Null EGL OSMesa monotonic shared"
    if version_marker not in native:
        raise FileDropCapabilityError("Vanilla GLFW Linux backend marker changed")
    return {
        "java_runtime": java_runtime,
        "lwjgl_core": {
            "artifact": core_relative,
            "artifact_sha1": core_sha1,
            "artifact_sha256": sha256_file(core_path),
            "multi_release": True,
            "selected_java_25_entries": core_entries,
        },
        "lwjgl_glfw_java": {
            "artifact": glfw_relative,
            "artifact_sha1": glfw_sha1,
            "artifact_sha256": sha256_file(glfw_path),
            "selected_entries": callback_entries,
        },
        "lwjgl_glfw_native_linux": {
            "artifact": native_relative,
            "artifact_sha1": native_sha1,
            "artifact_sha256": sha256_file(native_path),
            "embedded_glfw_commit": embedded_commit,
            "native_entry": prefix,
            "native_sha1": actual_native_sha1,
            "native_sha256": hashlib.sha256(native).hexdigest(),
            "version_backend_marker": version_marker.decode("ascii"),
        },
    }


def build_report(
    repo_root: Path,
    vanilla_root: Path,
    vanilla_download_root: Path,
    winit_root: Path,
) -> dict[str, object]:
    dependency = _winit_lock_record(repo_root / "Cargo.lock")
    vanilla_runtime = _vanilla_runtime_receipt(vanilla_download_root)
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
    screen_receipt = _receipt(
        vanilla_root,
        SCREEN,
        {"base onFilesDrop is empty": r"public void onFilesDrop\([^\n]+\) \{\s*\}"},
    )
    pack_selection_receipt = _receipt(
        vanilla_root,
        PACK_SELECTION_SCREEN,
        {
            "only observable valid-path consumer": r"@Override\s+public void onFilesDrop\([^\n]+\)",
            "drop opens a confirmation screen": r"String names = extractPackNames\(files\)[\s\S]+?new ConfirmScreen",
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
            "invalid UTF-8 is representable internally": r"InvalidUtf8\([^\n]*Utf8Error\)",
        },
    )

    bbb_receipt = _receipt(
        repo_root,
        BBB_MAIN,
        {
            "window-event adapter": r"fn enqueue_simulation_window_event\(",
            "Linux Java 25 scope ruling": r"official LWJGL decoding makes that failure\s+/// branch unreachable",
        },
    )
    main_text = (repo_root / BBB_MAIN).read_text(encoding="utf-8")
    adapter_body_start = main_text.index("fn enqueue_simulation_window_event(")
    adapter_body_end = main_text.index("\n}\n", adapter_body_start) + 2
    adapter_body = main_text[adapter_body_start:adapter_body_end]
    if "WindowEvent::DroppedFile" in adapter_body:
        raise FileDropCapabilityError(
            "BBB restored the lossy winit DroppedFile path inside the window adapter"
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
                "valid_path_values_consumed": False,
                "valid_path_values_scope": (
                    "Screen.onFilesDrop is empty on the declared screen surface; "
                    "PackSelectionScreen and in-game configuration UI are out of scope."
                ),
            },
            "backends": {
                "linux-wayland": {
                    "contract": "no file-drop WindowEvent implementation",
                    "scope": "not-needed: no observable declared-screen effect",
                    "source_tree": _tree_receipt(winit_root, WINIT_WAYLAND_ROOT),
                },
                "linux-x11": {
                    "contract": x11_contract,
                    "scope": "not-needed release row",
                    "sources": [x11_processor_receipt, x11_dnd_receipt],
                },
                "macos": {
                    "contract": {
                        "batch_identity_exposed": False,
                        "failed_path_count_exposed": False,
                        "success_delivery": "DroppedFile-per-NSString",
                    },
                    "scope": "deferred release validation",
                    "source": macos_receipt,
                },
                "windows": {
                    "contract": {
                        "batch_identity_exposed": False,
                        "failed_path_count_exposed": False,
                        "success_delivery": "DroppedFile-per-PathBuf",
                    },
                    "scope": "not-needed platform",
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
        "scope_ruling": {
            "classification": "not-needed-declared-linux-wayland",
            "final_conclusion": (
                "On official Java 25, GLFW paths are NUL-terminated before LWJGL's Java-11 "
                "multi-release UTF-8 decoder replaces malformed input. The resulting valid "
                "Java String cannot trigger UnixPath's NUL or malformed-UTF-16 rejection, so "
                "failedCount remains zero on Linux/Wayland. The only onFilesDrop override is "
                "PackSelectionScreen, excluded by the command-line-only configuration scope."
            ),
            "predicates": [
                "official runtime major version is Java 25",
                "GLFWDropCallback.getName uses NUL-terminated MemoryUtil.memUTF8",
                "Java 25 selects LWJGL's versions/25 MemoryUtil and versions/11 UTF-8 decoder",
                "the selected decoder uses StandardCharsets.UTF_8 replacement semantics",
                "UnixPath rejects NUL or malformed UTF-16, neither of which survives that chain",
                "Screen.onFilesDrop is empty and PackSelectionScreen is the only override",
            ],
            "reopen_condition": (
                "The official Java/LWJGL/GLFW/JDK conversion chain changes, a supported platform "
                "can produce InvalidPathException, BBB adds PackSelectionScreen or another "
                "onFilesDrop consumer, or the declared platform/configuration scope changes."
            ),
            "reproduce_command": REPRODUCE_COMMAND,
        },
        "generated_by": "scripts/generate-file-drop-capability.py",
        "generator_inputs": generator_inputs,
        "mechanism_ids": list(MECHANISM_IDS),
        "owner_ids": list(OWNER_IDS),
        "schema_version": 1,
        "status": "scope-ruled-not-needed",
        "upstream_source_receipts": list(UPSTREAM_SOURCE_RECEIPTS),
        "valid": True,
        "vanilla_contract": {
            "all_invalid_batch_dispatches_callback": False,
            "callback_state": ["valid_paths", "failed_path_count"],
            "failure_toast_condition": "valid_paths is non-empty and failed_path_count > 0",
            "official_runtime": vanilla_runtime,
            "on_files_drop_occurrences": _on_files_drop_occurrences(vanilla_root),
            "sources": [
                mouse_receipt,
                toast_receipt,
                screen_receipt,
                pack_selection_receipt,
            ],
            "toast_identity": "FILE_DROP_FAILURE",
            "toast_translation_keys": [
                "gui.fileDropFailure.title",
                "gui.fileDropFailure.detail",
            ],
        },
        "vanilla_version": VANILLA_VERSION,
    }


def render_report(
    repo_root: Path,
    vanilla_root: Path,
    vanilla_download_root: Path,
    winit_root: Path,
) -> str:
    return json.dumps(
        build_report(repo_root, vanilla_root, vanilla_download_root, winit_root),
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
    ) + "\n"
