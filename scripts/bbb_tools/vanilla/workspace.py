"""The sole owner of Vanilla cache-root discovery and version paths."""

from __future__ import annotations

import json
import os
import re
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .errors import IntegrityFailure, VanillaError, WorkspaceMissing

_VERSION = re.compile(r"[0-9A-Za-z][0-9A-Za-z._-]*\Z")


def _absolute(path: Path) -> Path:
    return path.expanduser().absolute()


def default_vanilla_root(
    *, platform: str | None = None, environ: Mapping[str, str] | None = None
) -> Path:
    """Return the platform cache root without probing any legacy location."""

    platform = platform or sys.platform
    environ = os.environ if environ is None else environ
    home_value = environ.get("HOME")
    if not home_value:
        raise VanillaError("HOME is unavailable; pass --vanilla-root explicitly")
    home = _absolute(Path(home_value))
    if platform == "darwin":
        return home / "Library" / "Caches" / "bbb" / "vanilla"
    if platform.startswith("linux"):
        xdg = environ.get("XDG_CACHE_HOME")
        if xdg:
            xdg_path = Path(xdg).expanduser()
            if not xdg_path.is_absolute():
                raise VanillaError("XDG_CACHE_HOME must be an absolute path")
            cache = xdg_path
        else:
            cache = home / ".cache"
        return cache / "bbb" / "vanilla"
    raise VanillaError(
        f"no default Vanilla cache root for platform {platform!r}; pass --vanilla-root explicitly"
    )


@dataclass(frozen=True, slots=True)
class VanillaWorkspace:
    """Resolved paths for one immutable Minecraft version."""

    root: Path
    version: str

    def __post_init__(self) -> None:
        if _VERSION.fullmatch(self.version) is None:
            raise VanillaError(f"invalid Minecraft version {self.version!r}")
        object.__setattr__(self, "root", _absolute(self.root))

    @classmethod
    def resolve(
        cls,
        version: str,
        explicit_root: str | os.PathLike[str] | None = None,
        *,
        platform: str | None = None,
        environ: Mapping[str, str] | None = None,
    ) -> VanillaWorkspace:
        environ = os.environ if environ is None else environ
        if explicit_root is not None:
            root = Path(explicit_root)
        elif root_value := environ.get("BBB_VANILLA_ROOT"):
            root = Path(root_value)
        else:
            root = default_vanilla_root(platform=platform, environ=environ)
        return cls(root=root, version=version)

    @property
    def downloads(self) -> Path:
        return self.root / "downloads" / self.version

    @property
    def sources(self) -> Path:
        return self.root / "sources" / self.version

    @property
    def assets(self) -> Path:
        return self.sources / "assets" / "minecraft"

    @property
    def indexed_assets(self) -> Path:
        return self.downloads / "assets"

    @property
    def libraries(self) -> Path:
        return self.downloads / "libraries"

    @property
    def client_jar(self) -> Path:
        return self.downloads / f"client-{self.version}.jar"

    @property
    def server_jar(self) -> Path:
        return self.downloads / f"server-{self.version}.jar"

    @property
    def metadata(self) -> Path:
        return self.downloads / f"{self.version}.json"

    @property
    def state(self) -> Path:
        return self.root / "state" / f"{self.version}.json"

    @property
    def lock_file(self) -> Path:
        return self.root / ".locks" / f"{self.version}.lock"

    @property
    def staging(self) -> Path:
        return self.root / ".staging"

    def prepare_command(self) -> str:
        return f"scripts/vanilla prepare {self.version} --vanilla-root {self.root}"

    def load_state(self) -> dict[str, Any]:
        if not self.state.is_file():
            raise WorkspaceMissing(
                f"Vanilla {self.version} workspace is unavailable at {self.root}.\n"
                f"Prepare it explicitly with:\n  {self.prepare_command()}"
            )
        try:
            value = json.loads(self.state.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise IntegrityFailure(
                f"cannot read Vanilla state manifest {self.state}: {error}"
            ) from error
        if not isinstance(value, dict) or value.get("status") != "complete":
            raise IntegrityFailure(f"Vanilla state manifest is not complete: {self.state}")
        return value

    def require_prepared(self) -> VanillaWorkspace:
        self.load_state()
        for label, path in (
            ("downloads", self.downloads),
            ("sources", self.sources),
            ("assets", self.assets),
        ):
            if not path.is_dir():
                raise IntegrityFailure(
                    f"Vanilla {label} path is missing: {path}; rerun {self.prepare_command()}"
                )
        return self

    def selected_path(self, kind: str) -> Path:
        paths = {
            "root": self.root,
            "sources": self.sources,
            "assets": self.assets,
            "downloads": self.downloads,
            "indexed-assets": self.indexed_assets,
            "libraries": self.libraries,
            "client-jar": self.client_jar,
            "server-jar": self.server_jar,
            "state": self.state,
        }
        if kind == "java":
            java = self.load_state().get("java", {}).get("executable")
            if not isinstance(java, str) or not java:
                raise IntegrityFailure("Vanilla state does not record a Java executable")
            return Path(java)
        try:
            return paths[kind]
        except KeyError as error:
            raise VanillaError(f"unknown Vanilla path kind {kind!r}") from error
