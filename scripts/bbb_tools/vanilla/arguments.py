"""Argparse adapters that keep workspace resolution out of generators."""

from __future__ import annotations

import argparse
import os
import shutil
from pathlib import Path

from .workspace import VanillaWorkspace

VANILLA_VERSION = "26.1"


def add_vanilla_root(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--vanilla-root",
        type=Path,
        help=(
            "managed Vanilla cache root; otherwise BBB_VANILLA_ROOT or the "
            "platform cache default is used"
        ),
    )


def resolve_workspace(args: argparse.Namespace) -> VanillaWorkspace:
    return VanillaWorkspace.resolve(
        VANILLA_VERSION, getattr(args, "vanilla_root", None)
    ).require_prepared()


def resolve_source_root(args: argparse.Namespace) -> Path:
    return resolve_workspace(args).sources


def resolve_download_root(args: argparse.Namespace) -> Path:
    return resolve_workspace(args).downloads


def add_vanilla_runtime(parser: argparse.ArgumentParser) -> None:
    add_vanilla_root(parser)
    parser.add_argument("--javac")
    parser.add_argument("--java")


def _executable(value: str | Path | None, fallback: str | Path) -> Path:
    value = fallback if value is None else value
    resolved = shutil.which(str(value))
    return Path(resolved) if resolved is not None else Path(value)


def resolve_runtime_inputs(args: argparse.Namespace) -> None:
    workspace = resolve_workspace(args)
    args.client_jar = workspace.client_jar
    args.libraries_root = workspace.libraries
    state_java = workspace.load_state().get("java", {}).get("executable", "java")
    java_fallback = os.environ.get("BBB_VANILLA_JAVA", state_java)
    args.java = _executable(args.java, java_fallback)
    javac_fallback = os.environ.get("BBB_VANILLA_JAVAC", args.java.with_name("javac"))
    args.javac = _executable(args.javac, javac_fallback)
