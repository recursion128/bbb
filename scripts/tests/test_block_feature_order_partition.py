from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from vanilla_java_source import JavaCall, JavaMethod, JavaSource, parse_source  # noqa: E402

SOURCE_ROOT = VanillaWorkspace.resolve("26.1").sources


class BlockFeatureOrderPartitionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        paths = {
            "storage": "net/minecraft/client/renderer/SubmitNodeStorage.java",
            "block_feature": (
                "net/minecraft/client/renderer/feature/BlockFeatureRenderer.java"
            ),
            "dispatcher": (
                "net/minecraft/client/renderer/feature/FeatureRenderDispatcher.java"
            ),
            "falling_block": (
                "net/minecraft/client/renderer/entity/FallingBlockRenderer.java"
            ),
            "piston": (
                "net/minecraft/client/renderer/blockentity/PistonHeadRenderer.java"
            ),
            "block_model": (
                "net/minecraft/client/renderer/block/BlockModelRenderState.java"
            ),
            "level": "net/minecraft/client/renderer/LevelRenderer.java",
            "map": "net/minecraft/client/renderer/MapRenderer.java",
            "display": (
                "net/minecraft/client/renderer/entity/DisplayRenderer.java"
            ),
            "sign": (
                "net/minecraft/client/renderer/blockentity/AbstractSignRenderer.java"
            ),
        }
        missing = [relative for relative in paths.values() if not (SOURCE_ROOT / relative).is_file()]
        if missing:
            raise unittest.SkipTest("local vanilla 26.1 sources are unavailable")
        cls.sources = {
            name: parse_source(relative, (SOURCE_ROOT / relative).read_bytes())
            for name, relative in paths.items()
        }

    def test_block_feature_renderer_owns_exactly_three_node_families(self) -> None:
        storage = self.sources["storage"]
        block_records = {
            java_type.simple_name
            for java_type in storage.types
            if java_type.kind == "record" and "Block" in java_type.simple_name
        }
        self.assertEqual(
            block_records,
            {
                "MovingBlockSubmit",
                "BlockModelSubmit",
                "BreakingBlockModelSubmit",
            },
        )

        renderer = self.sources["block_feature"]
        getters = {
            call.name
            for call in renderer.calls
            if call.name.startswith("get")
            and "Block" in call.name
            and call.name.endswith("Submits")
        }
        self.assertEqual(
            getters,
            {
                "getMovingBlockSubmits",
                "getBlockModelSubmits",
                "getBreakingBlockModelSubmits",
            },
        )
        self.assertEqual(
            self.selected_call_names(
                renderer,
                self.single_method(renderer, "renderSolid"),
                {
                    "renderMovingBlockSubmits",
                    "renderBlockModelSubmits",
                    "renderBreakingBlockModelSubmits",
                },
            ),
            ["renderMovingBlockSubmits", "renderBlockModelSubmits"],
        )
        self.assertEqual(
            self.selected_call_names(
                renderer,
                self.single_method(renderer, "renderTranslucent"),
                {
                    "renderMovingBlockSubmits",
                    "renderBlockModelSubmits",
                    "renderBreakingBlockModelSubmits",
                },
            ),
            [
                "renderMovingBlockSubmits",
                "renderBlockModelSubmits",
                "renderBreakingBlockModelSubmits",
            ],
        )

    def test_vanilla_producers_keep_each_block_submission_family(self) -> None:
        cases = [
            ("falling_block", "submit", "submitMovingBlock", 1),
            ("piston", "submit", "submitMovingBlock", 2),
            ("block_model", "submitModel", "submitBlockModel", 2),
            (
                "level",
                "submitBlockDestroyAnimation",
                "submitBreakingBlockModel",
                1,
            ),
        ]
        for source_name, method_name, call_name, expected_count in cases:
            with self.subTest(source=source_name, method=method_name):
                source = self.sources[source_name]
                method = self.single_method(source, method_name)
                calls = [
                    call
                    for call in self.calls_in_method(source, method)
                    if call.name == call_name
                ]
                self.assertEqual(len(calls), expected_count)

    def test_feature_dispatcher_orders_item_block_custom_for_every_collection(self) -> None:
        dispatcher = self.sources["dispatcher"]
        solid = self.single_method(dispatcher, "renderSolidFeatures")
        translucent = self.single_method(dispatcher, "renderTranslucentFeatures")
        for method in (solid, translucent):
            self.assertIn(
                "this.submitNodeStorage.getSubmitsPerOrder().values()",
                self.method_body_text(dispatcher, method),
            )

        self.assert_renderer_call_order(
            dispatcher,
            solid,
            [
                "this.itemFeatureRenderer",
                "this.blockFeatureRenderer",
                "this.customFeatureRenderer",
            ],
        )
        self.assert_renderer_call_order(
            dispatcher,
            translucent,
            [
                "this.textFeatureRenderer",
                "this.itemFeatureRenderer",
                "this.blockFeatureRenderer",
                "this.customFeatureRenderer",
            ],
        )

    def test_map_custom_geometry_and_labels_keep_zero_then_one_order(self) -> None:
        source = self.sources["map"]
        render = self.single_method(source, "render")
        calls = self.calls_in_method(source, render)
        custom = [call for call in calls if call.name == "submitCustomGeometry"]
        self.assertEqual(len(custom), 2)
        self.assertEqual(
            [self.compact(call.arguments[1].canonical) for call in custom],
            [
                "RenderTypes.text(mapRenderState.texture)",
                "RenderTypes.text(atlasSprite.atlasLocation())",
            ],
        )

        order = self.single_call(calls, "order")
        text = self.single_call(calls, "submitText")
        self.assertEqual([self.compact(arg.canonical) for arg in order.arguments], ["1"])
        self.assertEqual(
            self.compact(text.qualifier.canonical if text.qualifier else ""),
            "submitNodeCollector.order(1)",
        )
        self.assertLess(custom[-1].span.start_byte, order.span.start_byte)
        self.assertEqual(order.span.start_byte, text.span.start_byte)

    def test_text_display_background_selects_order_one_but_plain_text_stays_zero(
        self,
    ) -> None:
        source = self.sources["display"]
        method = self.single_method(
            source,
            "submitInner",
            owner_suffix=".TextDisplayRenderer",
        )
        calls = self.calls_in_method(source, method)
        custom = self.single_call(calls, "submitCustomGeometry")
        self.assertEqual(
            self.compact(custom.arguments[1].canonical),
            (
                "seeThrough?RenderTypes.textBackgroundSeeThrough()"
                ":RenderTypes.textBackground()"
            ),
        )
        order = self.single_call(calls, "order")
        self.assertEqual(
            self.compact(order.arguments[0].canonical),
            "backgroundColor!=0?1:0",
        )
        text = self.single_call(calls, "submitText")
        self.assertEqual(
            self.compact(text.qualifier.canonical if text.qualifier else ""),
            "textCollector",
        )
        self.assertLess(custom.span.start_byte, order.span.start_byte)
        self.assertLess(order.span.start_byte, text.span.start_byte)

    def test_sign_text_uses_the_default_order_zero_collector(self) -> None:
        source = self.sources["sign"]
        method = self.single_method(source, "submitSignText")
        calls = self.calls_in_method(source, method)
        text = self.single_call(calls, "submitText")
        self.assertEqual(
            self.compact(text.qualifier.canonical if text.qualifier else ""),
            "submitNodeCollector",
        )
        self.assertFalse(any(call.name == "order" for call in calls))

    def assert_renderer_call_order(
        self,
        source: JavaSource,
        method: JavaMethod,
        qualifiers: list[str],
    ) -> None:
        calls = self.calls_in_method(source, method)
        positions = []
        for qualifier in qualifiers:
            matches = [
                call
                for call in calls
                if self.compact(call.qualifier.canonical if call.qualifier else "")
                == qualifier
            ]
            self.assertEqual(len(matches), 1, qualifier)
            positions.append(matches[0].span.start_byte)
        self.assertEqual(positions, sorted(positions))

    @staticmethod
    def selected_call_names(
        source: JavaSource,
        method: JavaMethod,
        names: set[str],
    ) -> list[str]:
        return [
            call.name
            for call in BlockFeatureOrderPartitionTests.calls_in_method(source, method)
            if call.name in names
        ]

    @staticmethod
    def calls_in_method(source: JavaSource, method: JavaMethod) -> list[JavaCall]:
        return [
            call
            for call in source.calls
            if method.span.start_byte <= call.span.start_byte < method.span.end_byte
        ]

    @staticmethod
    def single_call(calls: list[JavaCall], name: str) -> JavaCall:
        matches = [call for call in calls if call.name == name]
        if len(matches) != 1:
            raise AssertionError(f"expected one call to {name}, found {len(matches)}")
        return matches[0]

    @staticmethod
    def single_method(
        source: JavaSource,
        name: str,
        *,
        owner_suffix: str | None = None,
    ) -> JavaMethod:
        matches = [
            method
            for method in source.methods
            if method.name == name
            and (owner_suffix is None or method.owner.endswith(owner_suffix))
        ]
        if len(matches) != 1:
            raise AssertionError(
                f"expected one {source.relative_path}#{name}, found {len(matches)}"
            )
        return matches[0]

    @staticmethod
    def method_body_text(source: JavaSource, method: JavaMethod) -> str:
        if method.body is None:
            raise AssertionError(f"{method.owner}.{method.name} has no body")
        return method.body.bytes(source.source).decode("utf-8")

    @staticmethod
    def compact(value: str) -> str:
        return value.replace(" ", "")


if __name__ == "__main__":
    unittest.main()
