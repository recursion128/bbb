"""Stable failures shared by the Vanilla command surface."""

from __future__ import annotations


class VanillaError(RuntimeError):
    """A user-actionable Vanilla tooling failure."""

    exit_code = 2
    kind = "configuration"


class WorkspaceMissing(VanillaError):
    exit_code = 10
    kind = "workspace-missing"


class DownloadFailure(VanillaError):
    exit_code = 11
    kind = "download-failed"


class DigestMismatch(VanillaError):
    exit_code = 12
    kind = "digest-mismatch"


class WorkspaceLocked(VanillaError):
    exit_code = 13
    kind = "workspace-locked"


class JavaIncompatible(VanillaError):
    exit_code = 14
    kind = "java-incompatible"


class DecompileFailure(VanillaError):
    exit_code = 15
    kind = "decompile-failed"


class IntegrityFailure(VanillaError):
    exit_code = 16
    kind = "integrity-failed"


class UnsafeClean(VanillaError):
    exit_code = 17
    kind = "unsafe-clean"
