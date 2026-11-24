#!/usr/bin/env python3
"""Generate the Vanilla 26.1 dedicated-server Brigadier command tree catalog."""

from __future__ import annotations

import argparse
import difflib
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-protocol/src/packets/commands/command_tree_26_1.rs"
)
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/CommandTreeReport.java"
PROBE_CLASS = "CommandTreeReport"

# The header the probe prints. Every field except the two counts pins an
# environment input that changes which commands `Commands` registers, so a
# runner that differs is rejected instead of silently producing a smaller tree:
# `jfr` gates `/jfr` (Commands.java:265-267), `dev_commands` gates the
# `/raid`, `/debugpath`, `/debugmobspawning`, `/wardenspawntracker`,
# `/spawnarmortrims`, `/serverpack` and `/debugconfig` block (:273-282), and
# `chase` gates `/chase` (:269-271).
EXPECTED_ENVIRONMENT = {
    "version": "26.1",
    "selection": "DEDICATED",
    "jfr": "1",
    "dev_commands": "0",
    "chase": "0",
}

KINDS = {"root": "Root", "literal": "Literal", "argument": "Argument"}

# `PermissionLevel` has five values (PermissionLevel.java:9-13); the probe
# reports `PermissionLevel.id()` of the lowest one that satisfies `requires`.
PERMISSION_LEVELS = 5

# DifficultyCommand.java:21-23 registers Difficulty.values(), whose declaration
# order is Difficulty.java:12-16.
DIFFICULTY_CHILDREN = ("peaceful", "easy", "normal", "hard")


@dataclass(frozen=True, slots=True)
class CommandTreeNode:
    """One row of the probe report, already cross-checked against its parent."""

    parent: int | None
    path: tuple[str, ...]
    kind: str
    name: str
    executable: bool
    redirect: int | None
    permission: int
    parser: str | None
    suggestions: str | None
    properties: str | None
    first_child: int
    child_count: int


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--tree-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    resolve_runtime_inputs(args)
    return args


def runtime_tree(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: "
            + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-command-tree-") as directory:
        classes = Path(directory)
        subprocess.run(
            [str(args.javac), "-cp", classpath, "-d", str(classes), str(PROBE_SOURCE)],
            check=True,
        )
        result = subprocess.run(
            [
                str(args.java),
                "-cp",
                os.pathsep.join((str(classes), classpath)),
                PROBE_CLASS,
            ],
            check=False,
            text=True,
            capture_output=True,
        )
        if result.returncode != 0:
            raise RuntimeError(
                "Vanilla command-tree probe failed:\n" + result.stderr.strip()
            )
    return result.stdout


def parse_environment(header: str) -> dict[str, str]:
    fields = dict(field.split("=", 1) for field in header.split(" ") if "=" in field)
    for key, expected in EXPECTED_ENVIRONMENT.items():
        if fields.get(key) != expected:
            raise ValueError(
                f"runtime report header field {key}={fields.get(key)!r} is not "
                f"{expected!r}; the command tree depends on it"
            )
    return fields


def _optional_index(field: str, label: str, limit: int) -> int | None:
    value = int(field)
    if value == -1:
        return None
    if not 0 <= value < limit:
        raise ValueError(f"{label} index {value} is outside the reported tree")
    return value


def parse_tree(source: str) -> tuple[CommandTreeNode, ...]:
    """Project the probe TSV onto validated nodes with contiguous child ranges."""

    lines = source.splitlines()
    if not lines:
        raise ValueError("empty runtime report")
    fields = parse_environment(lines[0])
    for count in ("nodes", "roots"):
        if count not in fields:
            raise ValueError(f"runtime report header lacks {count}")
    expected_count = int(fields["nodes"])
    rows = lines[1:]
    if len(rows) != expected_count:
        raise ValueError(f"expected {expected_count} node rows, got {len(rows)}")

    parents: list[int | None] = []
    paths: list[tuple[str, ...]] = []
    partial: list[dict[str, object]] = []
    for index, row in enumerate(rows):
        columns = row.split("\t")
        if len(columns) != 11:
            raise ValueError(f"node row {index} has {len(columns)} columns, expected 11")
        (
            reported_index,
            reported_parent,
            reported_path,
            kind,
            name,
            executable,
            redirect,
            permission,
            parser,
            suggestions,
            properties,
        ) = columns
        if reported_index != str(index):
            raise ValueError(f"node row {index} reports index {reported_index!r}")
        if kind not in KINDS:
            raise ValueError(f"node row {index} has unknown kind {kind!r}")
        if (kind == "root") != (index == 0):
            raise ValueError(f"node row {index} is the wrong place for kind {kind!r}")
        if executable not in ("0", "1"):
            raise ValueError(f"node row {index} has non-boolean executable")
        parent = _optional_index(reported_parent, "parent", expected_count)
        if (parent is None) != (index == 0):
            raise ValueError(f"node row {index} has a parent inconsistent with the root")
        if parent is not None and parent >= index:
            raise ValueError(f"node row {index} has forward parent {parent}")
        # The probe walks breadth-first and prints the path it built while
        # walking; recomputing it from the parent chain proves the two agree, so
        # the compact parent column below can stand in for the path everywhere.
        path = () if parent is None else (*paths[parent], name)
        if " ".join(path) != reported_path:
            raise ValueError(f"node row {index} path {reported_path!r} contradicts its parent")
        if (name == "") != (index == 0):
            raise ValueError(f"node row {index} has an empty name")
        if any(character.isspace() for character in name):
            raise ValueError(f"node row {index} name {name!r} contains whitespace")
        level = int(permission)
        if not 0 <= level < PERMISSION_LEVELS:
            raise ValueError(
                f"node row {index} requires permission {permission!r}; no "
                "PermissionLevel satisfies it, so the table needs a wider encoding"
            )
        if (kind == "argument") != bool(parser):
            raise ValueError(f"node row {index} pairs kind {kind!r} with parser {parser!r}")
        if kind != "argument" and (suggestions or properties):
            raise ValueError(f"node row {index} carries argument fields on a {kind}")
        parents.append(parent)
        paths.append(path)
        partial.append(
            {
                "parent": parent,
                "path": path,
                "kind": kind,
                "name": name,
                "executable": executable == "1",
                "redirect_raw": redirect,
                "permission": level,
                "parser": parser or None,
                "suggestions": suggestions or None,
                "properties": properties or None,
            }
        )

    ranges = _child_ranges(parents)
    # The probe counts the root's children off the live dispatcher, so a
    # mismatch means the walk lost or invented a command.
    if ranges[0][1] != int(fields["roots"]):
        raise ValueError(
            f"walked {ranges[0][1]} command roots, probe reported {fields['roots']}"
        )
    nodes = []
    for index, row in enumerate(partial):
        first_child, child_count = ranges[index]
        nodes.append(
            CommandTreeNode(
                parent=row["parent"],
                path=row["path"],
                kind=row["kind"],
                name=row["name"],
                executable=row["executable"],
                redirect=_optional_index(row["redirect_raw"], "redirect", len(partial)),
                permission=row["permission"],
                parser=row["parser"],
                suggestions=row["suggestions"],
                properties=row["properties"],
                first_child=first_child,
                child_count=child_count,
            )
        )
    return tuple(nodes)


def _child_ranges(parents: list[int | None]) -> list[tuple[int, int]]:
    """Fold the parent column into one contiguous index range per node.

    The probe emits nodes breadth-first in sibling registration order except
    for its canonical direct-gamerule run. Every parent's children therefore
    form one consecutive range without further generator-side reordering.
    """

    ranges = [(0, 0)] * len(parents)
    for index, parent in enumerate(parents):
        if parent is None:
            continue
        first, count = ranges[parent]
        if count == 0:
            ranges[parent] = (index, 1)
        elif first + count == index:
            ranges[parent] = (first, count + 1)
        else:
            raise ValueError(f"children of node {parent} are not contiguous at {index}")
    return ranges


def _child_names(
    nodes: tuple[CommandTreeNode, ...], path: tuple[str, ...]
) -> tuple[str, ...]:
    parent = next((node for node in nodes if node.path == path), None)
    if parent is None:
        raise ValueError(f"runtime report lacks the {' '.join(path)} command")
    return tuple(
        nodes[index].name
        for index in range(
            parent.first_child,
            parent.first_child + parent.child_count,
        )
    )


def validate_registration_order(nodes: tuple[CommandTreeNode, ...]) -> None:
    children = _child_names(nodes, ("difficulty",))
    if children != DIFFICULTY_CHILDREN:
        raise ValueError(
            "difficulty children are not in Vanilla registration order: "
            f"expected {DIFFICULTY_CHILDREN!r}, got {children!r}"
        )


def validate_gamerule_order(nodes: tuple[CommandTreeNode, ...]) -> None:
    children = _child_names(nodes, ("gamerule",))
    if children != tuple(sorted(children)):
        raise ValueError("gamerule children are not in canonical lexical order")


def rust_string(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def rust_option_string(value: str | None) -> str:
    return "None" if value is None else f"Some({rust_string(value)})"


def rust_option_index(value: int | None) -> str:
    return "None" if value is None else f"Some({value})"


def render_row(node: CommandTreeNode) -> str:
    return (
        "    VanillaCommandNode {"
        f" name: {rust_string(node.name)},"
        f" kind: CommandNodeType::{KINDS[node.kind]},"
        f" parent: {rust_option_index(node.parent)},"
        f" first_child: {node.first_child},"
        f" child_count: {node.child_count},"
        f" redirect: {rust_option_index(node.redirect)},"
        f" permission: {node.permission},"
        f" executable: {str(node.executable).lower()},"
        f" parser: {rust_option_string(node.parser)},"
        f" suggestions: {rust_option_string(node.suggestions)},"
        f" properties: {rust_option_string(node.properties)},"
        " },"
    )


def render_rust(nodes: tuple[CommandTreeNode, ...]) -> str:
    roots = [node for node in nodes if node.parent == 0]
    levels = [
        sum(1 for node in nodes if node.permission == level)
        for level in range(PERMISSION_LEVELS)
    ]
    parsers = sorted({node.parser for node in nodes if node.parser is not None})
    lines = [
        "// @generated by scripts/generate-command-tree-catalog.py; do not edit manually.",
        "// Source: the Brigadier tree a Vanilla 26.1 `Commands` instance built for",
        "// `CommandSelection.DEDICATED` holds, read out by the checked-in runtime probe",
        "// scripts/vanilla-runtime/CommandTreeReport.java. Reconstructing it from the",
        "// registration classes under net/minecraft/server/commands is not possible:",
        "// the shape comes out of builder chains, loops and recursive helpers.",
        "//",
        "// Columns, in the order they appear on each row:",
        "// - name: `CommandNode.getName()`; the root's is empty.",
        "// - kind: the node class (`RootCommandNode` / `LiteralCommandNode` /",
        "//   `ArgumentCommandNode`), which is also the wire node type.",
        "// - parent: index of the node this one hangs off; `None` only for the root.",
        "// - first_child / child_count: this node's children, always one contiguous run",
        "//   because the probe walks breadth-first. Runs preserve `CommandNode.getChildren()`",
        "//   registration order except direct `gamerule` children, which are sorted by name:",
        "//   GameRuleCommand.java:15-25 adds them while GameRules.java:160-165 iterates the",
        "//   `Reference2ObjectOpenHashMap` in GameRuleMap.java:18-45; its identity-hash",
        "//   order varies by JVM.",
        "// - redirect: `CommandNode.getRedirect()` resolved to an index. Redirects are",
        "//   the tree's only cycles (`execute run` points back at the root), so they are",
        "//   never followed when walking children.",
        "// - permission: `PermissionLevel.id()` of the lowest level whose",
        "//   `Commands.createCompilationContext` source satisfies `requires`. Vanilla",
        "//   26.1 checks `PermissionSet`s rather than numeric op levels",
        "//   (Commands.java:162-166), so this is probed, never read off a lambda.",
        "// - executable: `CommandNode.getCommand() != null`.",
        "// - parser: the `minecraft:command_argument_type` id of an argument node's",
        "//   `ArgumentType`, via `ArgumentTypeInfos.unpack`. The numeric wire id stays",
        "//   owned by VANILLA_COMMAND_ARGUMENT_TYPES; see `parser_type_id`.",
        "// - suggestions: `SuggestionProviders.getName(node.getCustomSuggestions())`.",
        "// - properties: `ArgumentTypeInfo.serializeToJson` output, the same projection",
        "//   ClientboundCommandsPacket writes to the network; `None` when it is empty.",
        "// Update: scripts/vanilla-source-tools.sh scripts/generate-command-tree-catalog.py",
        "// Check: scripts/vanilla-source-tools.sh scripts/generate-command-tree-catalog.py --check",
        "",
        "use super::{CommandNodeType, VanillaCommandNode};",
        "",
        # rustfmt would rewrap every row onto its own block of field lines, which
        # both explodes the file and makes `--check` fail against a formatted tree.
        "#[rustfmt::skip]",
        "pub const VANILLA_COMMAND_TREE: &[VanillaCommandNode] = &[",
        *(render_row(node) for node in nodes),
        "];",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use std::collections::BTreeSet;",
        "",
        "    use super::super::{",
        "        vanilla_command_lookup, vanilla_command_permission, vanilla_command_root_literals,",
        "        VANILLA_COMMAND_ARGUMENT_TYPES,",
        "    };",
        "    use super::*;",
        "",
        "    #[test]",
        "    fn dedicated_command_tree_matches_vanilla_26_1() {",
        f"        assert_eq!(VANILLA_COMMAND_TREE.len(), {len(nodes)});",
        f"        assert_eq!(vanilla_command_root_literals().count(), {len(roots)});",
        *(
            f"        assert_eq!(VANILLA_COMMAND_TREE.iter().filter(|node| node.permission == {level}).count(), {count});"
            for level, count in enumerate(levels)
        ),
        f"        assert_eq!(VANILLA_COMMAND_TREE.iter().filter(|node| node.executable).count(), {sum(1 for node in nodes if node.executable)});",
        f"        assert_eq!(VANILLA_COMMAND_TREE.iter().filter(|node| node.redirect.is_some()).count(), {sum(1 for node in nodes if node.redirect is not None)});",
        "    }",
        "",
        "    #[test]",
        "    fn difficulty_children_keep_vanilla_registration_order() {",
        '        let difficulty = vanilla_command_lookup(&["difficulty"]).expect("difficulty command");',
        "        let children = VANILLA_COMMAND_TREE[usize::from(difficulty)]",
        "            .children()",
        "            .map(|index| VANILLA_COMMAND_TREE[index].name)",
        "            .collect::<Vec<_>>();",
        "        assert_eq!(children, ["
        + ", ".join(rust_string(name) for name in DIFFICULTY_CHILDREN)
        + "]);",
        "    }",
        "",
        "    #[test]",
        "    fn gamerule_children_keep_canonical_order() {",
        '        let gamerule = vanilla_command_lookup(&["gamerule"]).expect("gamerule command");',
        "        let children = VANILLA_COMMAND_TREE[usize::from(gamerule)]",
        "            .children()",
        "            .map(|index| VANILLA_COMMAND_TREE[index].name)",
        "            .collect::<Vec<_>>();",
        "        assert!(children.windows(2).all(|pair| pair[0] < pair[1]));",
        "    }",
        "",
        "    #[test]",
        "    fn every_parser_resolves_through_the_generated_argument_type_ids() {",
        "        let parsers = VANILLA_COMMAND_TREE",
        "            .iter()",
        "            .filter_map(|node| node.parser)",
        "            .collect::<BTreeSet<_>>();",
        f"        assert_eq!(parsers.len(), {len(parsers)});",
        "        for parser in parsers {",
        "            assert!(",
        "                VANILLA_COMMAND_ARGUMENT_TYPES.contains(&parser),",
        '                "unknown command argument type {parser}"',
        "            );",
        "        }",
        "        for node in VANILLA_COMMAND_TREE {",
        "            assert_eq!(node.parser.is_some(), node.parser_type_id().is_some());",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn parent_child_and_redirect_indices_stay_inside_the_table() {",
        "        assert_eq!(VANILLA_COMMAND_TREE[0].kind, CommandNodeType::Root);",
        "        assert_eq!(VANILLA_COMMAND_TREE[0].parent, None);",
        "        for (index, node) in VANILLA_COMMAND_TREE.iter().enumerate() {",
        "            if index > 0 {",
        "                let parent = node.parent.expect(\"only the root has no parent\");",
        "                assert!(usize::from(parent) < index);",
        "                let siblings = VANILLA_COMMAND_TREE[usize::from(parent)].children();",
        "                assert!(siblings.contains(&index));",
        "            }",
        "            let own = u16::try_from(index).expect(\"the table is far shorter than u16::MAX\");",
        "            for child in node.children() {",
        "                assert_eq!(VANILLA_COMMAND_TREE[child].parent, Some(own));",
        "            }",
        "            if let Some(redirect) = node.redirect {",
        "                assert!(usize::from(redirect) < VANILLA_COMMAND_TREE.len());",
        "            }",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn probed_permission_levels_survive_the_lookup_path() {",
        *(
            f"        assert_eq!(vanilla_command_permission(&{_rust_path(node.path)}), Some({node.permission}));"
            for node in _permission_anchors(nodes)
        ),
        "        assert!(vanilla_command_lookup(&[\"list\", \"uuids\"]).is_some());",
        "        assert_eq!(vanilla_command_lookup(&[\"list\", \"nope\"]), None);",
        "    }",
        "}",
        "",
    ]
    return format_rust(lines)


def _rust_path(path: tuple[str, ...]) -> str:
    return "[" + ", ".join(rust_string(part) for part in path) + "]"


def _permission_anchors(nodes: tuple[CommandTreeNode, ...]) -> list[CommandTreeNode]:
    """One shortest root command per observed permission level.

    Anchoring the assertion on real paths keeps the probe honest: a regression
    that flattened every requirement would still pass a bare histogram.
    """

    anchors: dict[int, CommandTreeNode] = {}
    for node in nodes:
        if node.parent != 0:
            continue
        current = anchors.get(node.permission)
        if current is None or node.name < current.name:
            anchors[node.permission] = node
    return [anchors[level] for level in sorted(anchors)]


def format_rust(lines: list[str]) -> str:
    return subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input="\n".join(lines),
        text=True,
        capture_output=True,
        check=True,
    ).stdout


def check_output(path: Path, generated: str) -> bool:
    current = path.read_text() if path.exists() else ""
    if current == generated:
        return True
    sys.stderr.writelines(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            generated.splitlines(keepends=True),
            fromfile=str(path),
            tofile="generated",
        )
    )
    return False


def main() -> int:
    args = parse_args()
    report = (
        args.tree_input.read_text()
        if args.tree_input is not None
        else runtime_tree(args)
    )
    nodes = parse_tree(report)
    validate_registration_order(nodes)
    validate_gamerule_order(nodes)
    generated = render_rust(nodes)
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(generated)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
