from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from command_argument_catalog_extractor import (  # noqa: E402
    CommandArgumentCatalogExtractionError,
    command_argument_entries,
    command_slot_entries,
    serialized_enum_names,
)
from vanilla_java_source import parse_source  # noqa: E402


class CommandArgumentCatalogExtractorTests(unittest.TestCase):
    def test_bootstrap_calls_preserve_cst_order_and_default_namespace(self) -> None:
        source = parse_source(
            "fixture/ArgumentTypeInfos.java",
            b'''
package fixture;
class ArgumentTypeInfos {
  static Object bootstrap(Registry registry) {
    // register(registry, "fake", Type.class, info);
    register(registry, "brigadier:bool", Bool.class, info);
    return register(
        registry, "entity", Entity.class,
        factory("register(registry, \\"ignored\\", Type.class, info)"));
  }
}
''',
        )
        self.assertEqual(
            [entry.resource_id for entry in command_argument_entries(source, 2)],
            ["brigadier:bool", "minecraft:entity"],
        )

    def test_unsupported_call_and_count_change_fail_explicitly(self) -> None:
        source = parse_source(
            "fixture/ArgumentTypeInfos.java",
            b'''package fixture;
class ArgumentTypeInfos {
  static Object bootstrap(Registry other) {
    return register(other, "entity", Entity.class, info);
  }
}''',
        )
        with self.assertRaisesRegex(
            CommandArgumentCatalogExtractionError, "unsupported argument register call"
        ):
            command_argument_entries(source, 1)

    def test_slot_range_lambda_calls_preserve_cst_order_and_expand_ranges(self) -> None:
        source = parse_source(
            "fixture/SlotRanges.java",
            b'''
package fixture;
class SlotRanges {
  private static final List<SlotRange> SLOTS = Util.make(
      new ArrayList<>(),
      output -> {
        addSingleSlot(output, "contents", 0);
        addSlotRange(
            output,
            "container.",
            0,
            2
        );
        log("addSlots(output, \\"ignored\\", 1)");
        addSlots(output, "armor.*", 1, 2);
      });
}
''',
        )
        self.assertEqual(
            [
                (entry.name, entry.single)
                for entry in command_slot_entries(source, expected_count=5)
            ],
            [
                ("contents", True),
                ("container.0", True),
                ("container.1", True),
                ("container.*", False),
                ("armor.*", False),
            ],
        )

    def test_slot_range_unknown_output_helper_fails_explicitly(self) -> None:
        source = parse_source(
            "fixture/SlotRanges.java",
            b'''
package fixture;
class SlotRanges {
  private static final List<SlotRange> SLOTS = Util.make(
      new ArrayList<>(), output -> append(output, "contents", 0));
}
''',
        )
        with self.assertRaisesRegex(
            CommandArgumentCatalogExtractionError,
            "unsupported SLOTS helper append",
        ):
            command_slot_entries(source, expected_count=1)

    def test_serialized_enum_names_use_enum_constant_arguments_in_order(self) -> None:
        mirror = parse_source(
            "fixture/Mirror.java",
            b'''
package fixture;
public enum Mirror implements StringRepresentable {
  NONE("none", Group.IDENTITY),
  // FAKE("ignored")
  LEFT_RIGHT(
      "left_right",
      Group.INVERT_Z
  ),
  FRONT_BACK("front_back", Group.INVERT_X);
  String unrelated() { return "not_a_constant"; }
}
''',
        )
        rotation = parse_source(
            "fixture/Rotation.java",
            b'''
package fixture;
public enum Rotation implements StringRepresentable {
  NONE(0, "none", Group.IDENTITY),
  CLOCKWISE_90(1, "clockwise_90", Group.ROT_90),
  CLOCKWISE_180(2, "180", Group.ROT_180),
  COUNTERCLOCKWISE_90(3, "counterclockwise_90", Group.ROT_270);
}
''',
        )
        self.assertEqual(
            serialized_enum_names(mirror, expected_count=3),
            ["none", "left_right", "front_back"],
        )
        self.assertEqual(
            serialized_enum_names(rotation, expected_count=4),
            ["none", "clockwise_90", "180", "counterclockwise_90"],
        )

    def test_serialized_enum_shape_change_fails_explicitly(self) -> None:
        source = parse_source(
            "fixture/Mirror.java",
            b'''
package fixture;
public enum Mirror implements StringRepresentable {
  NONE("none", "second");
}
''',
        )
        with self.assertRaisesRegex(
            CommandArgumentCatalogExtractionError,
            "exactly one serialized-name literal",
        ):
            serialized_enum_names(source, expected_count=1)


if __name__ == "__main__":
    unittest.main()
