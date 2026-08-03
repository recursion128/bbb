from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
import tempfile
import unittest
from collections import Counter
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import client_mechanism_inventory as inventory  # noqa: E402


GENERATOR_SPEC = importlib.util.spec_from_file_location(
    "generate_client_mechanism_inventory",
    SCRIPTS / "generate-client-mechanism-inventory.py",
)
assert GENERATOR_SPEC is not None and GENERATOR_SPEC.loader is not None
generator = importlib.util.module_from_spec(GENERATOR_SPEC)
GENERATOR_SPEC.loader.exec_module(generator)


class ClientMechanismInventoryTests(unittest.TestCase):
    def write_source(self, root: Path, relative: str, source: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")

    def test_method_surface_uses_cst_signatures_lines_and_source_hash(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Owner.java"
            source = """\
package fixture;
public class Owner {
  public Owner() {}
  public void decide(int value) {}
  public void decide(String value) {}
}
"""
            self.write_source(root, relative, source)
            builder = inventory.InventoryBuilder(root)
            builder.add_method_surface(
                inventory.MethodSurface("decision", "fixture", "fixture-mechanism", relative)
            )

        self.assertEqual([item["source"]["line"] for item in builder.owners], [3, 4, 5])
        self.assertEqual(len({item["owner_id"] for item in builder.owners}), 3)
        self.assertEqual(
            builder.hashes[relative], hashlib.sha256(source.encode("utf-8")).hexdigest()
        )

    def test_profile_key_and_skin_owners_are_distinct_lifecycles(self) -> None:
        by_source = {surface.source_path: surface for surface in inventory.METHOD_SURFACES}
        profile_keys = by_source[
            "net/minecraft/client/multiplayer/AccountProfileKeyPairManager.java"
        ]
        skins = by_source["net/minecraft/client/resources/SkinManager.java"]

        self.assertEqual(profile_keys.mechanism_id, "identity-profile-key-lifecycle")
        self.assertEqual(skins.mechanism_id, "identity-skin-presentation")
        self.assertNotEqual(profile_keys.mechanism_id, skins.mechanism_id)
        self.assertIn(
            "crates/bbb-net/src/chat_session/runtime.rs",
            inventory.MECHANISM_EVIDENCE[profile_keys.mechanism_id],
        )
        self.assertEqual(
            inventory.MECHANISM_EVIDENCE[skins.mechanism_id],
            ("crates/bbb-item-model/src/skin_runtime.rs",),
        )

    def test_minecraft_method_partition_is_total_and_homogeneous(self) -> None:
        generated = inventory.REPO_ROOT / "docs/vanilla-client-mechanism-inventory-26.1.json"
        value = json.loads(generated.read_text(encoding="utf-8"))
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] == "net/minecraft/client/Minecraft.java"
            and owner["kind"] in {"method", "constructor"}
        ]
        by_mechanism = Counter(owner["mechanism_id"] for owner in methods)

        self.assertEqual(len(methods), 187)
        self.assertEqual(by_mechanism["lifecycle-client-runtime"], 9)
        self.assertEqual(by_mechanism["lifecycle-session-replacement"], 12)
        self.assertEqual(by_mechanism["lifecycle-client-task-handoff"], 3)
        self.assertEqual(by_mechanism["scope-support-accessors"], 64)
        self.assertTrue(all(owner.get("role") for owner in methods))
        self.assertTrue(
            all(
                owner["role"] == "support-non-decision"
                for owner in methods
                if owner["mechanism_id"] == "scope-support-accessors"
            )
        )

    def test_level_renderer_method_partition_is_total_and_homogeneous(self) -> None:
        generated = inventory.REPO_ROOT / "docs/vanilla-client-mechanism-inventory-26.1.json"
        value = json.loads(generated.read_text(encoding="utf-8"))
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"]
            == "net/minecraft/client/renderer/LevelRenderer.java"
            and owner["kind"] in {"method", "constructor"}
        ]
        by_mechanism = Counter(owner["mechanism_id"] for owner in methods)

        self.assertEqual(len(methods), 77)
        self.assertEqual(
            by_mechanism,
            Counter(
                {
                    "rendering-level-backend-lifecycle": 9,
                    "rendering-level-section-lifecycle": 29,
                    "rendering-level-frame-graph": 14,
                    "rendering-level-scene-projection": 15,
                    "rendering-level-effect-lifecycle": 4,
                    "rendering-level-diagnostics-gizmos": 3,
                    "rendering-level-light-query": 3,
                }
            ),
        )
        self.assertNotIn("rendering-level-step-surface", value["mechanisms"])

        section_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-level-section-lifecycle"
        ]
        self.assertEqual(len(section_owners), 29)
        self.assertEqual(
            value["mechanisms"]["rendering-level-section-lifecycle"][
                "owner_evidence_count"
            ],
            29,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in section_owners
            )
        )
        backend_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-level-backend-lifecycle"
        ]
        self.assertEqual(len(backend_owners), 9)
        self.assertEqual(
            value["mechanisms"]["rendering-level-backend-lifecycle"][
                "owner_evidence_count"
            ],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in backend_owners
            )
        )
        scene_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-level-scene-projection"
        ]
        self.assertEqual(len(scene_owners), 15)
        self.assertEqual(
            value["mechanisms"]["rendering-level-scene-projection"][
                "owner_evidence_count"
            ],
            15,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in scene_owners
            )
        )
        frame_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-level-frame-graph"
        ]
        self.assertEqual(len(frame_owners), 14)
        self.assertEqual(
            value["mechanisms"]["rendering-level-frame-graph"][
                "owner_evidence_count"
            ],
            14,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in frame_owners
            )
        )
        effect_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-level-effect-lifecycle"
        ]
        self.assertEqual(len(effect_owners), 4)
        self.assertEqual(
            value["mechanisms"]["rendering-level-effect-lifecycle"][
                "owner_evidence_count"
            ],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in effect_owners
            )
        )

    def test_unclassified_level_renderer_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void newDecision() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks a homogeneous route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-frame-graph",
                        relative,
                    )
                )

    def test_section_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void compileSections() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_SECTION_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-section-lifecycle",
                        relative,
                    )
                )

    def test_backend_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void resize() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_BACKEND_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-backend-lifecycle",
                        relative,
                    )
                )

    def test_scene_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void extractBlockOutline() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_SCENE_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-frame-graph",
                        relative,
                    )
                )

    def test_frame_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void addCloudsPass() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_FRAME_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-frame-graph",
                        relative,
                    )
                )

    def test_effect_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { void tick() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_EFFECT_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-frame-graph",
                        relative,
                    )
                )

    def test_diagnostics_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { String getEntityStatistics() { return null; } }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_DIAGNOSTICS_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "rendering",
                        "rendering-level-diagnostics-gizmos",
                        relative,
                    )
                )

    def test_direct_field_return_is_a_support_accessor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/Minecraft.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client; class Minecraft { int getValue() { return this.value; } }",
            )
            source = inventory.InventoryBuilder(root).source(relative)
            method = source.methods[0]

        self.assertTrue(
            inventory.InventoryBuilder.is_direct_support_accessor(source, method)
        )

    def test_unclassified_minecraft_decision_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/Minecraft.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client; class Minecraft { int decide() { return ready ? 1 : 0; } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks a decision/support route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "lifecycle",
                        "lifecycle",
                        "lifecycle-client-runtime",
                        relative,
                    )
                )

    def test_multiplayer_screen_surfaces_are_homogeneous_and_exact(self) -> None:
        expected_by_source = {
            "net/minecraft/client/gui/screens/ChatScreen.java": (
                "ui-chat-screen-surface",
                26,
            ),
            "net/minecraft/client/gui/screens/DeathScreen.java": (
                "ui-death-screen-surface",
                16,
            ),
            "net/minecraft/client/gui/screens/DisconnectedScreen.java": (
                "ui-disconnected-screen-surface",
                8,
            ),
            "net/minecraft/client/gui/screens/PauseScreen.java": (
                "ui-pause-screen-surface",
                18,
            ),
        }
        surfaces = {
            surface.source_path: surface
            for surface in inventory.METHOD_SURFACES
            if surface.source_path in expected_by_source
        }
        self.assertEqual(set(surfaces), set(expected_by_source))
        self.assertNotIn("ui-multiplayer-screen-surface", inventory.MECHANISM_EVIDENCE)

        generated = inventory.REPO_ROOT / "docs/vanilla-client-mechanism-inventory-26.1.json"
        value = json.loads(generated.read_text(encoding="utf-8"))
        for source_path, (mechanism_id, owner_count) in expected_by_source.items():
            self.assertEqual(surfaces[source_path].mechanism_id, mechanism_id)
            self.assertEqual(value["mechanisms"][mechanism_id]["owner_count"], owner_count)
            self.assertIn(mechanism_id, inventory.MECHANISM_EVIDENCE)

        self.assertNotIn("ui-multiplayer-screen-surface", value["mechanisms"])

    def test_registration_surface_ignores_comments_and_string_literals(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Registrations.java"
            self.write_source(
                root,
                relative,
                """\
package fixture;
public class Registrations {
  public static void bootstrap() {
    // register(FAKE, Fake::new);
    String fake = "register(FAKE, Fake::new)";
    register(REAL, Real::new);
  }
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_registration_surface(
                "provider", "fixture", "fixture-mechanism", relative, ("register",)
            )

        self.assertEqual(len(builder.owners), 1)
        self.assertIn("REAL", builder.owners[0]["vanilla_owner"])

    def test_synchronized_registry_entries_come_from_list_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/resources/RegistryDataLoader.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.resources;
public class RegistryDataLoader {
  public static final List<RegistryData<?>> SYNCHRONIZED_REGISTRIES = List.of(
      new RegistryData<>(Registries.BIOME, Biome.NETWORK_CODEC),
      new RegistryData<>(Registries.CHAT_TYPE, ChatType.DIRECT_CODEC));
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_synchronized_registries()

        self.assertEqual(len(builder.owners), 2)
        self.assertIn("Registries.BIOME", builder.owners[0]["vanilla_owner"])
        self.assertIn("Registries.CHAT_TYPE", builder.owners[1]["vanilla_owner"])

    def test_serverbound_scan_finds_constructed_and_singleton_packets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Actions.java"
            self.write_source(
                root,
                relative,
                """\
package fixture;
public class Actions {
  public void act() {
    connection.send(new ServerboundUseItemPacket());
    connection.send(ServerboundConfigurationAcknowledgedPacket.INSTANCE);
    connection.send(new ClientboundWrongDirectionPacket());
  }
}
""",
            )
            original = inventory.SERVERBOUND_SCAN_PATHS
            inventory.SERVERBOUND_SCAN_PATHS = (relative,)
            try:
                builder = inventory.InventoryBuilder(root)
                builder.add_serverbound_calls()
            finally:
                inventory.SERVERBOUND_SCAN_PATHS = original

        self.assertEqual(
            sorted(item["vanilla_owner"].split(" -> ")[-1] for item in builder.owners),
            ["ServerboundConfigurationAcknowledgedPacket", "ServerboundUseItemPacket"],
        )

    def test_generator_check_rejects_stale_exact_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "inventory.json"
            output.write_text("stale\n", encoding="utf-8")
            args = SimpleNamespace(source_root=root, output=output, check=True)
            with mock.patch.object(generator, "parse_args", return_value=args), mock.patch.object(
                generator, "render_inventory", return_value="current\n"
            ):
                result = generator.main()

        self.assertEqual(result, 1)

    def test_generator_check_accepts_exact_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "inventory.json"
            output.write_text("current\n", encoding="utf-8")
            args = SimpleNamespace(source_root=root, output=output, check=True)
            with mock.patch.object(generator, "parse_args", return_value=args), mock.patch.object(
                generator, "render_inventory", return_value="current\n"
            ):
                result = generator.main()

        self.assertEqual(result, 0)


if __name__ == "__main__":
    unittest.main()
