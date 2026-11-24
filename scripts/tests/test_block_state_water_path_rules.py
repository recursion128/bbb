import tempfile
import unittest
from pathlib import Path

from block_state_water_path_rules import (
    LIQUID_SOURCE_NOT_LAVA,
    STATE_FLUID_WATER,
    STATIC,
    classify_owner,
)
from vanilla_java_source import JavaSourceError


class BlockStateWaterPathRulesTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def source(self, name, body, parent=None):
        path = self.root / "net/minecraft/world/level/block" / f"{name}.java"
        path.parent.mkdir(parents=True, exist_ok=True)
        extends = f" extends {parent}" if parent else ""
        path.write_text(
            "package net.minecraft.world.level.block;\n"
            f"class {name}{extends} {{\n"
            " protected boolean isPathfindable(BlockState state, PathComputationType type) "
            f"{{ {body} }}\n"
            "}\n"
        )
        return f"net.minecraft.world.level.block.{name}"

    def test_switch_inheritance_and_air_ternary_reach_state_fluid_rule(self):
        base = self.source(
            "Base",
            "switch(type) { case WATER: return state.getFluidState().is(FluidTags.WATER); "
            "default: return false; }",
        )
        self.source(
            "Child",
            "return type == PathComputationType.AIR && !this.hasCollision ? true : "
            "super.isPathfindable(state, type);",
            "Base",
        )
        self.assertEqual(classify_owner(self.root, base), STATE_FLUID_WATER)
        self.assertEqual(
            classify_owner(self.root, "net.minecraft.world.level.block.Child"), STATE_FLUID_WATER
        )

    def test_static_waterlogged_and_liquid_source_rules(self):
        trap = self.source(
            "Trap",
            "switch(type) { case WATER: return state.getValue(WATERLOGGED); "
            "default: return false; }",
        )
        liquid = self.source("Liquid", "return !this.fluid.is(FluidTags.LAVA);")
        self.assertEqual(classify_owner(self.root, trap), STATIC)
        self.assertEqual(classify_owner(self.root, liquid), LIQUID_SOURCE_NOT_LAVA)

    def test_unknown_expression_owner_signature_and_parent_fail_closed(self):
        unknown = self.source("Unknown", "return state.isAir();")
        missing_parent = self.source(
            "MissingParent",
            "return false;",
            "Gone",
        )
        overload = self.source("Overload", "return false;")
        file = self.root / "net/minecraft/world/level/block/Overload.java"
        file.write_text(
            file.read_text().replace(
                "}\n",
                " protected boolean isPathfindable(BlockState state, int type) "
                "{ return false; }\n}\n",
            )
        )
        for owner in [unknown, missing_parent, overload, "net.minecraft.world.level.block.Absent"]:
            with self.subTest(owner=owner), self.assertRaises((JavaSourceError, FileNotFoundError)):
                classify_owner(self.root, owner)


if __name__ == "__main__":
    unittest.main()
