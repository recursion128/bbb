from __future__ import annotations

import importlib.util
import json
import sys
import unittest
from collections import Counter
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from client_options_catalog_extractor import (  # noqa: E402
    ClientOptionsCatalogExtractionError,
    client_option_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


GENERATOR_SPEC = importlib.util.spec_from_file_location(
    "generate_client_options_catalog",
    SCRIPTS / "generate-client-options-catalog.py",
)
assert GENERATOR_SPEC is not None and GENERATOR_SPEC.loader is not None
generator = importlib.util.module_from_spec(GENERATOR_SPEC)
GENERATOR_SPEC.loader.exec_module(generator)


PREAMBLE = b"""package net.minecraft.client;
class Options {
"""


def source(body: bytes):
    return parse_source("net/minecraft/client/Options.java", PREAMBLE + body + b"}\n")


FIXTURE = b"""
   private final OptionInstance<Boolean> autoJump = OptionInstance.createBoolean("options.autoJump", false);
   private final OptionInstance<Integer> renderDistance = new OptionInstance<>(
      "options.renderDistance", noTooltip(), label, new OptionInstance.IntRange(2, 32), 12, value -> refresh()
   );
   public String lastMpIp = "";
   public boolean hideServerAddress;
   private void processDumpedOptions(final OptionAccess access) {
      access.process("renderDistance", this.renderDistance);
   }
   private void processOptions(final FieldAccess access) {
      this.processDumpedOptions(access);
      access.process("autoJump", this.autoJump);
      this.lastMpIp = access.process("lastServer", this.lastMpIp);
      this.hideServerAddress = access.process("hideServerAddress", this.hideServerAddress);
   }
"""


class ClientOptionsCatalogExtractorTests(unittest.TestCase):
    def test_extracts_save_order_kinds_and_defaults(self) -> None:
        options, groups = client_option_entries(source(FIXTURE))

        self.assertEqual([option.key for option in options], [
            "renderDistance",
            "autoJump",
            "lastServer",
            "hideServerAddress",
        ])
        self.assertEqual([option.order for option in options], [0, 1, 2, 3])
        self.assertEqual([option.kind for option in options], [
            "codec",
            "codec",
            "string",
            "boolean",
        ])
        # `FieldAccess.process(String, OptionInstance)` runs the option's codec
        # through JsonOps, so the value set decides the on-disk shape.
        self.assertEqual([option.encoding for option in options], [
            "integer",
            "boolean",
            "string",
            "boolean",
        ])
        # The trailing onValueUpdate callback is not the default, an
        # uninitialized primitive falls back to Java's implicit value, and a
        # constructor-assigned OptionInstance still resolves.
        self.assertEqual([option.default for option in options], [
            "12",
            "false",
            '""',
            "false",
        ])
        self.assertEqual(groups, [])

    def test_constructor_assigned_option_resolves_its_default(self) -> None:
        options, _ = client_option_entries(
            source(
                b"""
   private final OptionInstance<Integer> simulationDistance;
   Options(final Minecraft minecraft, final File directory) {
      this.simulationDistance = new OptionInstance<>(
         "options.simulationDistance", noTooltip(), label, new OptionInstance.IntRange(5, 32), 12, value -> refresh()
      );
   }
   private void processDumpedOptions(final OptionAccess access) {
      access.process("simulationDistance", this.simulationDistance);
   }
   private void processOptions(final FieldAccess access) {
      this.processDumpedOptions(access);
   }
"""
            )
        )

        self.assertEqual([(option.key, option.default) for option in options], [
            ("simulationDistance", "12")
        ])

    def test_unknown_target_type_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            ClientOptionsCatalogExtractionError, "unsupported type"
        ):
            client_option_entries(
                source(
                    b"""
   public double someNewOption = 1.0;
   private void processDumpedOptions(final OptionAccess access) {}
   private void processOptions(final FieldAccess access) {
      this.processDumpedOptions(access);
      access.process("someNewOption", this.someNewOption);
   }
"""
                )
            )

    def test_duplicate_key_fails_closed(self) -> None:
        with self.assertRaisesRegex(
            ClientOptionsCatalogExtractionError, "duplicate options.txt key"
        ):
            client_option_entries(
                source(
                    b"""
   public boolean flag;
   private void processDumpedOptions(final OptionAccess access) {
      access.process("flag", this.flag);
   }
   private void processOptions(final FieldAccess access) {
      this.processDumpedOptions(access);
      access.process("flag", this.flag);
   }
"""
                )
            )


class ClientOptionsCatalogGeneratorTests(unittest.TestCase):
    def test_committed_catalog_matches_the_vanilla_source(self) -> None:
        source_root = VanillaWorkspace.resolve("26.1").sources
        if not (source_root / generator.OPTIONS_SOURCE).is_file():
            self.skipTest("local Vanilla 26.1 source tree is absent")
        rendered = generator.render_catalog(source_root)

        self.assertEqual(
            generator.DEFAULT_OUTPUT.read_text(encoding="utf-8"),
            rendered,
            "regenerate with scripts/generate-client-options-catalog.py",
        )

    def test_catalog_covers_every_options_txt_line(self) -> None:
        document = json.loads(
            generator.DEFAULT_OUTPUT.read_text(encoding="utf-8")
        )
        options = document["options"]

        self.assertEqual(len(options), 108)
        self.assertEqual([option["order"] for option in options], list(range(108)))
        self.assertEqual(len({option["key"] for option in options}), 108)
        # `save` writes the dumped block first, so `ao` opens the file and the
        # three per-element loops close it.
        self.assertEqual(options[0]["key"], "ao")
        self.assertEqual(
            [group["prefix"] for group in document["dynamic_groups"]],
            ["key_", "soundCategory_", "modelPart_"],
        )
        self.assertEqual(
            [group["order"] for group in document["dynamic_groups"]], [108, 109, 110]
        )
        # Every line has a resolved on-disk shape; an unmapped value set or
        # codec fails the generator instead of landing here as a placeholder.
        self.assertEqual(
            Counter(option["encoding"] for option in options),
            Counter(
                {
                    "boolean": 54,
                    "double": 22,
                    "integer": 14,
                    "enum-id": 6,
                    "enum-name": 5,
                    "string": 3,
                    "string-list": 2,
                    "enum-name-or-boolean": 1,
                    "tutorial-step": 1,
                }
            ),
        )
        # Enum defaults arrive as the text `save` writes, resolved from the
        # enum's own constructor rather than a table beside the option.
        defaults = {option["key"]: option["default"] for option in options}
        self.assertEqual(defaults["graphicsPreset"], '"fancy"')
        self.assertEqual(defaults["mainHand"], '"right"')
        self.assertEqual(defaults["musicFrequency"], '"DEFAULT"')
        self.assertEqual(defaults["attackIndicator"], "1")
        self.assertEqual(defaults["narrator"], "0")
        # `CloudStatus.FANCY` serializes as the legacy `true`.
        self.assertEqual(defaults["renderClouds"], '"true"')
        # Only the `OptionInstance` overload goes through GSON, so the scalar
        # overloads write their value bare.
        self.assertEqual(defaults["lang"], "en_us")
        self.assertEqual(defaults["lastServer"], "")
        self.assertEqual(defaults["tutorialStep"], "movement")
        self.assertEqual(defaults["soundDevice"], '""')
        # `fov` stores 70 but its codec writes `(70 - 70.0) / 40.0`.
        self.assertEqual(defaults["fov"], "0.0")
        # `renderClouds` is the one key that still reads the pre-1.20 boolean
        # through `Codec.withAlternative`.
        self.assertEqual(
            [
                option["key"]
                for option in options
                if option["encoding"] == "enum-name-or-boolean"
            ],
            ["renderClouds"],
        )


if __name__ == "__main__":
    unittest.main()
