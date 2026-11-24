#!/usr/bin/env python3
"""Generate Vanilla 26.1 entity model layers and keyframe animations.

Runs ``scripts/vanilla-runtime/EntityModelReport.java`` against the official
client jar: every ``LayerDefinitions.createRoots()`` layer (texture size and
part tree with poses and cubes) and every ``AnimationDefinition`` declared in
``net.minecraft.client.animation.definitions``.

Update: scripts/python python scripts/generate-entity-models.py
Check:  scripts/python python scripts/generate-entity-models.py --check
"""

from __future__ import annotations

import argparse
import difflib
import hashlib
import os
import re
import struct
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

from bbb_tools.vanilla.arguments import add_vanilla_runtime, resolve_runtime_inputs

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = (
    REPO_ROOT / "crates/bbb-client-render/src/entity_models/model_data_26_1.rs"
)
PROBE_SOURCE = REPO_ROOT / "scripts/vanilla-runtime/EntityModelReport.java"
PROBE_CLASS = "EntityModelReport"


@dataclass(frozen=True)
class Cube:
    origin: tuple[float, float, float]
    dimensions: tuple[float, float, float]
    grow: tuple[float, float, float]
    tex: tuple[float, float]
    tex_scale: tuple[float, float]
    mirror: bool
    faces: int


@dataclass
class Part:
    name: str
    offset: tuple[float, float, float]
    rotation: tuple[float, float, float]
    scale: tuple[float, float, float]
    cubes: list[Cube]
    children: list[Part]


@dataclass(frozen=True)
class Layer:
    location: str
    texture_size: tuple[int, int]
    root: Part


@dataclass(frozen=True)
class Key:
    timestamp: float
    pre: tuple[float, float, float]
    post: tuple[float, float, float]
    interpolation: str


@dataclass(frozen=True)
class Animation:
    owner: str
    field: str
    length: float
    looping: bool
    bones: tuple[tuple[str, tuple[tuple[str, tuple[Key, ...]], ...]], ...]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    add_vanilla_runtime(parser)
    parser.add_argument("--report-input", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.report_input is None:
        resolve_runtime_inputs(args)
    return args


def runtime_report(args: argparse.Namespace) -> str:
    jars = [args.client_jar, *sorted(args.libraries_root.rglob("*.jar"))]
    missing = [path for path in (args.javac, args.java, *jars[:1]) if not path.is_file()]
    if missing:
        raise FileNotFoundError(
            "missing Vanilla runtime probe input: "
            + ", ".join(str(path) for path in missing)
        )
    classpath = os.pathsep.join(str(path) for path in jars)
    with tempfile.TemporaryDirectory(prefix="bbb-entity-models-") as directory:
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
                str(args.client_jar),
            ],
            check=False,
            text=True,
            capture_output=True,
        )
    if result.returncode != 0:
        raise RuntimeError("Vanilla entity model probe failed:\n" + result.stderr.strip())
    return result.stdout


def f32(bits: str) -> float:
    return struct.unpack(">f", int(bits).to_bytes(4, "big"))[0]


def floats(fields: list[str]) -> tuple[float, ...]:
    return tuple(f32(value) for value in fields)


def parse_report(source: str) -> tuple[list[Layer], list[Animation]]:
    lines = [line for line in source.splitlines() if line]
    header = re.fullmatch(r"version=26\.1 layers=(\d+)", lines[0]) if lines else None
    if header is None:
        raise ValueError("entity model report lacks its 26.1 header")
    layers: list[Layer] = []
    animations: list[Animation] = []
    stack: list[Part] = []
    animation: dict | None = None
    bone: list | None = None
    channel: list | None = None

    def finish_animation() -> None:
        if animation is None:
            return
        animations.append(
            Animation(
                owner=animation["owner"],
                field=animation["field"],
                length=animation["length"],
                looping=animation["looping"],
                bones=tuple(
                    (name, tuple((target, tuple(keys)) for target, keys in channels))
                    for name, channels in animation["bones"]
                ),
            )
        )

    for line in lines[1:]:
        kind, *fields = line.split(" ")
        if kind == "layer":
            location, width, height = fields
            stack = []
            layers.append(Layer(location, (int(width), int(height)), None))  # type: ignore[arg-type]
        elif kind == "part":
            depth = int(fields[0])
            values = floats(fields[2:])
            part = Part(
                fields[1], values[0:3], values[3:6], values[6:9], [], []
            )  # type: ignore[arg-type]
            del stack[depth:]
            if depth == 0:
                if layers[-1].root is not None:
                    raise ValueError(f"layer {layers[-1].location} has two roots")
                layers[-1] = Layer(layers[-1].location, layers[-1].texture_size, part)
            else:
                stack[depth - 1].children.append(part)
            stack.append(part)
        elif kind == "cube":
            values = floats(fields[:13])
            mirror, faces = int(fields[13]), int(fields[14])
            if mirror not in (0, 1) or not 0 <= faces <= 0x3F:
                raise ValueError(f"invalid cube flags: {line}")
            stack[-1].cubes.append(
                Cube(
                    values[0:3], values[3:6], values[6:9], values[9:11], values[11:13],
                    bool(mirror), faces,
                )  # type: ignore[arg-type]
            )
        elif kind == "animation":
            finish_animation()
            owner, field = fields[0].split(".")
            animation = {
                "owner": owner,
                "field": field,
                "length": f32(fields[1]),
                "looping": fields[2] == "1",
                "bones": [],
            }
        elif kind == "bone":
            bone = [fields[0], []]
            animation["bones"].append(bone)  # type: ignore[index]
        elif kind == "channel":
            if fields[0] not in ("position", "rotation", "scale"):
                raise ValueError(f"unknown animation target: {line}")
            channel = [fields[0], []]
            bone[1].append(channel)  # type: ignore[index]
        elif kind == "key":
            values = floats(fields[:7])
            if fields[7] not in ("linear", "catmullrom"):
                raise ValueError(f"unknown interpolation: {line}")
            channel[1].append(Key(values[0], values[1:4], values[4:7], fields[7]))  # type: ignore[index, arg-type]
        else:
            raise ValueError(f"unknown report line: {line}")
    finish_animation()
    if len(layers) != int(header.group(1)):
        raise ValueError("entity model report layer count mismatch")
    return layers, animations


def rust_f32(value: float) -> str:
    if value != value or value in (float("inf"), float("-inf")):
        raise ValueError(f"non-finite model float {value}")
    if value == 0.0:
        return "-0.0" if struct.pack(">f", value) != struct.pack(">f", 0.0) else "0.0"
    for precision in range(1, 10):
        text = f"{value:.{precision}g}"
        if struct.unpack(">f", struct.pack(">f", float(text)))[0] == value:
            break
    return repr(float(text))


def vec(values: tuple[float, ...]) -> str:
    return "[" + ", ".join(rust_f32(value) for value in values) + "]"


def layer_const_name(location: str) -> str:
    match = re.fullmatch(r"minecraft:([a-z0-9_/]+)#([a-z0-9_]+)", location)
    if match is None:
        raise ValueError(f"unexpected model layer location {location}")
    path, layer = match.groups()
    name = path.replace("/", "_").upper()
    return name if layer == "main" else f"{name}_{layer.upper()}"


def snake_case(name: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


class TreeTable:
    """Flattens part trees into shared cube runs, child lists and parts."""

    def __init__(self) -> None:
        self.cubes: list[Cube] = []
        self.cube_runs: dict[tuple[Cube, ...], int] = {}
        self.parts: list[tuple] = []
        self.part_ids: dict[tuple, int] = {}
        self.children: list[int] = []
        self.child_runs: dict[tuple[int, ...], int] = {}

    def cube_run(self, cubes: list[Cube]) -> tuple[int, int]:
        key = tuple(cubes)
        if not key:
            return 0, 0
        start = self.cube_runs.get(key)
        if start is None:
            start = len(self.cubes)
            self.cubes.extend(key)
            self.cube_runs[key] = start
        return start, len(key)

    def child_run(self, children: list[int]) -> tuple[int, int]:
        key = tuple(children)
        if not key:
            return 0, 0
        start = self.child_runs.get(key)
        if start is None:
            start = len(self.children)
            self.children.extend(key)
            self.child_runs[key] = start
        return start, len(key)

    def part(self, part: Part) -> int:
        children = [self.part(child) for child in part.children]
        names = [child.name for child in part.children]
        if len(set(names)) != len(names):
            raise ValueError(f"duplicate child names under {part.name}")
        key = (
            part.name,
            part.offset,
            part.rotation,
            part.scale,
            self.cube_run(part.cubes),
            self.child_run(children),
        )
        index = self.part_ids.get(key)
        if index is None:
            index = len(self.parts)
            self.parts.append(key)
            self.part_ids[key] = index
        return index


def render_cube(cube: Cube) -> str:
    grow = "G0" if cube.grow == (0.0, 0.0, 0.0) else vec(cube.grow)
    arguments = [
        vec(cube.origin),
        vec(cube.dimensions),
        grow,
        vec(cube.tex),
        "true" if cube.mirror else "false",
        str(cube.faces),
    ]
    if cube.tex_scale == (1.0, 1.0):
        return f"    c({', '.join(arguments)}),"
    return f"    cs({', '.join(arguments)}, {vec(cube.tex_scale)}),"


def render_key(key: Key) -> str:
    interpolation = "L" if key.interpolation == "linear" else "C"
    if key.pre == key.post:
        return f"k({rust_f32(key.timestamp)}, {vec(key.post)}, {interpolation})"
    return (
        f"Keyframe {{ timestamp: {rust_f32(key.timestamp)}, pre_target: {vec(key.pre)}, "
        f"post_target: {vec(key.post)}, interpolation: {interpolation} }}"
    )


def render_rust(layers: list[Layer], animations: list[Animation], probe_sha256: str) -> str:
    table = TreeTable()
    roots = [table.part(layer.root) for layer in layers]
    names = [layer_const_name(layer.location) for layer in layers]
    if len(set(names)) != len(names):
        raise ValueError("model layer constant names collide")
    for limit, label in (
        (len(table.cubes), "cubes"),
        (len(table.parts), "parts"),
        (len(table.children), "children"),
    ):
        if limit > 0xFFFF:
            raise ValueError(f"model {label} table does not fit in u16")

    out = [
        "// @generated by scripts/generate-entity-models.py; do not edit manually.",
        "// Source: Vanilla 26.1 LayerDefinitions.createRoots() and net.minecraft.client.animation.definitions.",
        f"// Probe SHA-256: {probe_sha256}",
        "// Check: scripts/python python scripts/generate-entity-models.py --check",
        "",
        "#![allow(clippy::approx_constant)]",
        "",
        "use super::keyframe::keyframe as k;",
        "use super::keyframe::AnimationTarget::{Position as P, Rotation as R, Scale as S};",
        "use super::keyframe::KeyframeInterpolation::{CatmullRom as C, Linear as L};",
        "use super::keyframe::{AnimationChannel as Ch, AnimationDefinition, BoneAnimation as B};",
        "use super::model_layer::{cube as c, cube_scaled as cs, G0};",
        "use super::model_layer::{LayerDef, ModelCube, ModelLayer, PartDef};",
        "",
        "#[rustfmt::skip]",
        f"pub(super) static LAYERS: [LayerDef; {len(layers)}] = [",
    ]
    for layer, root in zip(layers, roots):
        width, height = layer.texture_size
        out.append(f'    LayerDef {{ texture_size: [{width}, {height}], root: {root} }}, // {layer.location}')
    out.append("];")
    out.append("")
    out.append("#[rustfmt::skip]")
    out.append(f"pub(super) static PARTS: [PartDef; {len(table.parts)}] = [")
    for name, offset, rotation, scale, cubes, children in table.parts:
        scale_text = "S1" if scale == (1.0, 1.0, 1.0) else vec(scale)
        out.append(
            f'    PartDef::new("{name}", {vec(offset)}, {vec(rotation)}, {scale_text}, '
            f"{cubes[0]}, {cubes[1]}, {children[0]}, {children[1]}),"
        )
    out.append("];")
    out.append("")
    out.append("const S1: [f32; 3] = [1.0; 3];")
    out.append("")
    out.append("#[rustfmt::skip]")
    out.append(f"pub(super) static CHILDREN: [u16; {len(table.children)}] = [")
    for start in range(0, len(table.children), 16):
        out.append("    " + ", ".join(str(value) for value in table.children[start : start + 16]) + ",")
    out.append("];")
    out.append("")
    out.append("#[rustfmt::skip]")
    out.append(f"pub(super) static CUBES: [ModelCube; {len(table.cubes)}] = [")
    out.extend(render_cube(cube) for cube in table.cubes)
    out.append("];")
    out.append("")
    out.append("/// Vanilla `ModelLayers` locations, named after the location path and layer.")
    out.append("#[rustfmt::skip]")
    out.append("#[allow(dead_code)]")
    out.append("impl ModelLayer {")
    for index, name in enumerate(names):
        out.append(f"    pub(in crate::entity_models) const {name}: ModelLayer = ModelLayer({index});")
    out.append("}")

    owners: dict[str, list[Animation]] = {}
    for animation in animations:
        owners.setdefault(animation.owner, []).append(animation)
    for owner in sorted(owners):
        out.append("")
        out.append(f"/// Vanilla `{owner}`.")
        out.append("#[rustfmt::skip]")
        out.append("#[allow(dead_code)]")
        out.append(f"pub(in crate::entity_models) mod {snake_case(owner)} {{")
        out.append("    use super::*;")
        for animation in owners[owner]:
            out.append("")
            out.append(
                f"    pub(in crate::entity_models) static {animation.field.upper()}: AnimationDefinition = AnimationDefinition {{"
            )
            out.append(f"        length_seconds: {rust_f32(animation.length)},")
            out.append(f"        looping: {'true' if animation.looping else 'false'},")
            out.append("        bones: &[")
            for bone, channels in animation.bones:
                out.append(f'            B {{ bone: "{bone}", channels: &[')
                for target, keys in channels:
                    letter = {"position": "P", "rotation": "R", "scale": "S"}[target]
                    out.append(f"                Ch {{ target: {letter}, keyframes: &[")
                    out.extend(f"                    {render_key(key)}," for key in keys)
                    out.append("                ] },")
                out.append("            ] },")
            out.append("        ],")
            out.append("    };")
        out.append("}")
    out.append("")
    text = "\n".join(out)
    if "Keyframe {" in text:
        text = text.replace(
            "use super::keyframe::keyframe as k;",
            "use super::keyframe::{keyframe as k, Keyframe};",
        )
    return text


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
    source = (
        args.report_input.read_text()
        if args.report_input is not None
        else runtime_report(args)
    )
    layers, animations = parse_report(source)
    probe_sha256 = hashlib.sha256(PROBE_SOURCE.read_text().encode()).hexdigest()
    generated = render_rust(layers, animations, probe_sha256)
    if args.check:
        return 0 if check_output(args.output, generated) else 1
    args.output.write_text(generated)
    print(f"wrote {len(layers)} model layers and {len(animations)} animations to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
