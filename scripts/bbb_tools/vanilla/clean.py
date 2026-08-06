"""Version-scoped, managed-cache-only cleanup."""

from __future__ import annotations

import shutil
from pathlib import Path

from .errors import UnsafeClean
from .locking import VersionLock
from .workspace import VanillaWorkspace

_REPO_ROOT = Path(__file__).resolve().parents[3]


def _is_relative_to(path: Path, parent: Path) -> bool:
    try:
        path.relative_to(parent)
        return True
    except ValueError:
        return False


def clean_workspace(workspace: VanillaWorkspace) -> dict[str, object]:
    root = workspace.root.resolve()
    home = Path.home().resolve()
    if root in {Path("/"), home, _REPO_ROOT.resolve()}:
        raise UnsafeClean(f"refusing unsafe Vanilla root: {root}")
    if _is_relative_to(root, _REPO_ROOT.resolve()):
        raise UnsafeClean(f"Vanilla cache must not be inside the bbb worktree: {root}")
    if (root / ".git").exists():
        raise UnsafeClean(f"refusing to clean a Git repository: {root}")
    removed: list[str] = []
    with VersionLock(workspace.lock_file):
        for target in (
            workspace.downloads,
            workspace.sources,
            root / "logs" / workspace.version,
        ):
            if target.is_symlink():
                raise UnsafeClean(f"refusing symlinked Vanilla target: {target}")
            if target.is_dir():
                shutil.rmtree(target)
                removed.append(str(target))
            elif target.exists():
                raise UnsafeClean(f"expected directory, found file: {target}")
        if workspace.state.is_file() and not workspace.state.is_symlink():
            workspace.state.unlink()
            removed.append(str(workspace.state))
        for target in sorted(workspace.staging.glob(f"{workspace.version}-*")):
            if target.is_symlink() or not target.is_dir():
                raise UnsafeClean(f"refusing unsafe Vanilla staging target: {target}")
            shutil.rmtree(target)
            removed.append(str(target))
    return {"status": "cleaned", "version": workspace.version, "removed": removed}
