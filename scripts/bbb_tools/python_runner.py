"""Cross-platform resolution of the primary worktree's managed uv Python runner."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Mapping
from pathlib import Path

_TARGET_SUFFIX_RE = re.compile(r"[A-Za-z0-9_-]+")


def default_target_root(environ: Mapping[str, str] | None = None) -> Path:
    """Resolve the shared Cargo target root, matching `bbb-xtask`'s `default_target_root`."""
    env = os.environ if environ is None else environ
    override = env.get("BBB_TARGET_ROOT", "").strip()
    if override:
        return Path(override)
    if os.name == "nt":
        return Path(tempfile.gettempdir())
    return Path("/tmp")


def default_target_dir(
    name: str = "main", environ: Mapping[str, str] | None = None
) -> Path:
    """Resolve the Cargo target dir (`CARGO_TARGET_DIR` > `BBB_TARGET_ROOT` > platform default)."""
    env = os.environ if environ is None else environ
    explicit = env.get("CARGO_TARGET_DIR", "").strip()
    if explicit:
        return Path(explicit)
    target_name = env.get("BBB_CARGO_TARGET_NAME", "").strip() or name
    return default_target_root(env) / f"bbb-target-{target_name}"


def _scoped_web_dir(
    env_key: str,
    base_name: str,
    *,
    environ: Mapping[str, str] | None = None,
    default_root: Path | None = None,
) -> Path:
    env = os.environ if environ is None else environ
    explicit = env.get(env_key, "").strip()
    if explicit:
        return Path(explicit)
    root = default_root if default_root is not None else default_target_root(env)
    target_name = env.get("BBB_CARGO_TARGET_NAME", "").strip()
    if target_name and target_name != "main" and _TARGET_SUFFIX_RE.fullmatch(target_name):
        return root / f"{base_name}-{target_name}"
    return root / base_name


def default_web_out_dir(
    environ: Mapping[str, str] | None = None,
    *,
    default_root: Path | None = None,
) -> Path:
    """Resolve the default Web WASM bundle directory, matching `bbb-xtask`."""
    return _scoped_web_dir(
        "BBB_WEB_OUT_DIR", "bbb-web-dist", environ=environ, default_root=default_root
    )


def default_web_cache_dir(
    environ: Mapping[str, str] | None = None,
    *,
    default_root: Path | None = None,
) -> Path:
    """Resolve the default Web asset-bundle cache directory, matching `bbb-xtask`."""
    return _scoped_web_dir(
        "BBB_WEB_CACHE_DIR", "bbb-web-cache", environ=environ, default_root=default_root
    )


def primary_worktree_root(repo_root: Path) -> Path:
    if venv_python_path(repo_root).is_file():
        return repo_root
    result = subprocess.run(
        ["git", "-C", str(repo_root), "worktree", "list", "--porcelain"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            if line.startswith("worktree "):
                candidate = Path(line[len("worktree ") :])
                if candidate.is_dir():
                    return candidate
    return repo_root


def venv_python_path(project_root: Path) -> Path:
    if os.name == "nt":
        return project_root / ".venv" / "Scripts" / "python.exe"
    return project_root / ".venv" / "bin" / "python"


def managed_command(repo_root: Path, *argv: str) -> list[str]:
    if not argv:
        raise ValueError("managed_command requires a tool or command")
    project_root = primary_worktree_root(repo_root)
    venv_py = venv_python_path(project_root)
    uv_bin = shutil.which("uv")
    if (
        uv_bin is not None
        and venv_py.is_file()
        and (project_root / "pyproject.toml").is_file()
        and (project_root / "uv.lock").is_file()
    ):
        return [
            uv_bin,
            "run",
            "--offline",
            "--locked",
            "--exact",
            "--no-python-downloads",
            "--project",
            str(project_root),
            "--",
            *argv,
        ]
    if venv_py.is_file():
        if argv[0] == "python":
            return [str(venv_py), *argv[1:]]
        sibling = venv_py.with_name(f"{argv[0]}.exe" if os.name == "nt" else argv[0])
        if sibling.is_file():
            return [str(sibling), *argv[1:]]
        return [str(venv_py), "-m", *argv]
    if argv[0] == "python":
        return [sys.executable, *argv[1:]]
    resolved = shutil.which(argv[0])
    if resolved is not None:
        return [resolved, *argv[1:]]
    return list(argv)


def python_env(repo_root: Path) -> dict[str, str]:
    env = os.environ.copy()
    scripts_dir = str(repo_root / "scripts")
    existing = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = f"{scripts_dir}{os.pathsep}{existing}" if existing else scripts_dir
    env.setdefault("PYTHONDONTWRITEBYTECODE", "1")
    return env


def _latest_xtask_source_mtime(repo_root: Path) -> float:
    xtask_dir = repo_root / "crates" / "bbb-xtask"
    mtimes = [
        path.stat().st_mtime
        for path in (xtask_dir / "Cargo.toml", *(xtask_dir / "src").rglob("*.rs"))
        if path.is_file()
    ]
    return max(mtimes, default=0.0)


def ensure_xtask_binary(repo_root: Path) -> Path:
    """Resolve an up-to-date bbb-xtask binary without relinking a running executable."""
    env_bin = os.environ.get("BBB_XTASK_BIN")
    if env_bin:
        candidate = Path(env_bin)
        if candidate.is_file():
            return candidate

    exe_name = "bbb-xtask.exe" if os.name == "nt" else "bbb-xtask"
    latest_mtime = _latest_xtask_source_mtime(repo_root)
    primary_bin = repo_root / ".local" / "xtask-target" / "debug" / exe_name
    if primary_bin.is_file() and primary_bin.stat().st_mtime >= latest_mtime:
        return primary_bin

    test_target = repo_root / ".local" / "xtask-test-target"
    test_target.mkdir(parents=True, exist_ok=True)
    test_bin = test_target / "debug" / exe_name
    if test_bin.is_file() and test_bin.stat().st_mtime >= latest_mtime:
        return test_bin

    lock_path = test_target / ".build.lock"
    deadline = time.monotonic() + 90.0
    fd: int | None = None
    while fd is None:
        try:
            fd = os.open(str(lock_path), os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            os.write(fd, f"{os.getpid()}\n".encode("ascii"))
        except FileExistsError:
            try:
                if time.time() - lock_path.stat().st_mtime > 60.0:
                    lock_path.unlink(missing_ok=True)
                    continue
            except OSError:
                pass
            if time.monotonic() >= deadline:
                raise TimeoutError(f"timed out waiting for {lock_path}") from None
            time.sleep(0.05)

    try:
        if test_bin.is_file() and test_bin.stat().st_mtime >= latest_mtime:
            return test_bin
        cargo_bin = os.environ.get("CARGO") or "cargo"
        subprocess.run(
            [
                cargo_bin,
                "build",
                "--target-dir",
                str(test_target),
                "--package",
                "bbb-xtask",
                "--locked",
            ],
            cwd=repo_root,
            check=True,
        )
        return test_bin
    finally:
        os.close(fd)
        lock_path.unlink(missing_ok=True)
