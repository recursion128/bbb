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

    def test_selected_method_surface_fails_closed_when_owner_census_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Owner.java"
            self.write_source(
                root,
                relative,
                "package fixture; public class Owner { public void other() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "selected method census changed",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "decision",
                        "fixture",
                        "fixture-mechanism",
                        relative,
                        selected_methods=("selected",),
                    )
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
        # Online mode is a declared non-goal: the profile-key lifecycle owns no
        # implementation and resolves to the ledger's not-needed decision.
        self.assertEqual(
            inventory.MECHANISM_EVIDENCE[profile_keys.mechanism_id],
            ("docs/unsupported-features.md",),
        )
        self.assertEqual(
            inventory.MECHANISM_EVIDENCE[skins.mechanism_id],
            ("crates/bbb-item-model/src/skin_runtime.rs",),
        )

    def test_language_owner_census_includes_client_and_recipe_trigger_evidence(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        language_owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "resources-language-manager"
        ]
        bidi_owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "ui-bidirectional-shaping"
        ]

        self.assertEqual(len(language_owners), 15)
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in language_owners),
            Counter(
                {
                    "net/minecraft/client/resources/language/LanguageManager.java": 7,
                    "net/minecraft/client/resources/language/ClientLanguage.java": 6,
                    "net/minecraft/client/gui/screens/recipebook/RecipeBookComponent.java": 2,
                }
            ),
        )
        self.assertEqual(
            value["mechanisms"]["resources-language-manager"]["owner_evidence_count"],
            15,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in language_owners
            )
        )
        self.assertEqual(len(bidi_owners), 1)
        self.assertIn("#getVisualOrder(", bidi_owners[0]["vanilla_owner"])
        self.assertEqual(
            value["mechanisms"]["ui-bidirectional-shaping"]["owner_evidence_count"],
            1,
        )
        source_hashes = {
            source["path"]: source["sha256"] for source in value["source_files"]
        }
        self.assertEqual(
            source_hashes[
                "net/minecraft/client/resources/language/LanguageManager.java"
            ],
            "7750a201dbc857147bb080cb3e8eb483eaa8560d0179745cfba41547ab1662a2",
        )
        self.assertEqual(
            source_hashes[
                "net/minecraft/client/resources/language/ClientLanguage.java"
            ],
            "198cb3e37827ec1bdb3039b0acaf5bf917713efabd0dd5df0904caa7429be536",
        )
        self.assertEqual(
            source_hashes[
                "net/minecraft/client/gui/screens/recipebook/RecipeBookComponent.java"
            ],
            "0651995cc1cac7928dc01737dd98150ee3001eb9e377c5179452125d6ba46da6",
        )

    def test_camera_fog_owner_census_is_exact_and_has_per_owner_evidence(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "rendering-camera-fluid-fog"
        ]
        expected_by_source = Counter(
            {
                "net/minecraft/client/Camera.java": 2,
                "net/minecraft/client/renderer/fog/FogRenderer.java": 3,
                "net/minecraft/client/renderer/fog/environment/FogEnvironment.java": 6,
                "net/minecraft/client/renderer/fog/environment/MobEffectFogEnvironment.java": 4,
                "net/minecraft/client/renderer/fog/environment/AtmosphericFogEnvironment.java": 5,
                "net/minecraft/client/renderer/fog/environment/WaterFogEnvironment.java": 3,
                "net/minecraft/client/renderer/fog/environment/LavaFogEnvironment.java": 3,
                "net/minecraft/client/renderer/fog/environment/PowderedSnowFogEnvironment.java": 3,
                "net/minecraft/client/renderer/fog/environment/BlindnessFogEnvironment.java": 3,
                "net/minecraft/client/renderer/fog/environment/DarknessFogEnvironment.java": 3,
            }
        )

        self.assertEqual(len(owners), 35)
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in owners),
            expected_by_source,
        )
        mechanism = value["mechanisms"]["rendering-camera-fluid-fog"]
        self.assertEqual(mechanism["owner_count"], 35)
        self.assertEqual(mechanism["owner_evidence_count"], 35)
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in owners
            )
        )

    def test_chunk_layer_owner_census_covers_routing_and_sort_contracts(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "rendering-chunk-layer-routing"
        ]

        self.assertEqual(len(owners), 10)
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in owners),
            Counter(
                {
                    "net/minecraft/client/renderer/chunk/SectionCompiler.java": 2,
                    "com/mojang/blaze3d/vertex/MeshData.java": 2,
                    "com/mojang/blaze3d/vertex/VertexSorting.java": 5,
                    "net/minecraft/client/renderer/chunk/SectionRenderDispatcher.java": 1,
                }
            ),
        )
        mechanism = value["mechanisms"]["rendering-chunk-layer-routing"]
        self.assertEqual(mechanism["owner_count"], 10)
        self.assertEqual(mechanism["owner_evidence_count"], 10)
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in owners
            )
        )

    def test_audio_owner_census_has_complete_evidence_and_explicit_scope_rulings(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "audio-sound-manager-engine"
        ]

        self.assertEqual(len(owners), 159)
        self.assertEqual(
            value["mechanisms"]["audio-sound-manager-engine"][
                "owner_evidence_count"
            ],
            159,
        )
        self.assertEqual(
            value["mechanisms"]["audio-sound-manager-engine"].get("owner_gap_count", 0),
            0,
        )
        self.assertTrue(
            all(
                (
                    owner["bbb_evidence"].get("implementation")
                    and owner["bbb_evidence"].get("tests")
                )
                for owner in owners
            )
        )
        rulings = [
            owner["bbb_evidence"]["scope_ruling"]
            for owner in owners
            if "scope_ruling" in owner["bbb_evidence"]
        ]
        self.assertEqual(len(rulings), 12)
        hashed_sources = {source["path"] for source in value["source_files"]}
        self.assertTrue(
            {
                "com/mojang/blaze3d/audio/Library.java",
                "com/mojang/blaze3d/audio/Channel.java",
                "com/mojang/blaze3d/audio/Listener.java",
                "com/mojang/blaze3d/audio/SoundBuffer.java",
                "net/minecraft/client/sounds/ChannelAccess.java",
                "net/minecraft/client/sounds/SoundBufferLibrary.java",
                "net/minecraft/client/sounds/MusicManager.java",
            }.issubset(hashed_sources)
        )
        self.assertTrue(
            all(
                ruling["conclusion"] == "not-reachable-in-current-scope"
                and ruling["vanilla_basis"]
                and all(
                    anchor.partition(":")[0] in hashed_sources
                    for anchor in ruling["vanilla_basis"]
                )
                and ruling["current_scope_reason"]
                and ruling["reopen_condition"]
                for ruling in rulings
            )
        )

    def test_server_pack_owner_census_has_complete_evidence_and_scope_rulings(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == "resources-server-pack-lifecycle"
        ]

        self.assertEqual(len(owners), 68)
        mechanism = value["mechanisms"]["resources-server-pack-lifecycle"]
        self.assertEqual(mechanism["owner_count"], 68)
        self.assertEqual(mechanism["owner_evidence_count"], 68)
        self.assertEqual(mechanism.get("owner_gap_count", 0), 0)
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in owners),
            Counter(
                {
                    "net/minecraft/client/Minecraft.java": 1,
                    "net/minecraft/client/resources/server/DownloadedPackSource.java": 25,
                    "net/minecraft/client/resources/server/ServerPackManager.java": 22,
                    "net/minecraft/server/packs/DownloadQueue.java": 6,
                    "net/minecraft/server/packs/DownloadCacheCleaner.java": 3,
                    "net/minecraft/util/HttpUtil.java": 11,
                }
            ),
        )
        self.assertTrue(
            all(
                (
                    owner["bbb_evidence"].get("implementation")
                    and owner["bbb_evidence"].get("tests")
                )
                for owner in owners
            )
        )
        open_gaps = {
            owner["vanilla_owner"]: owner["bbb_evidence"]["open_gap"]
            for owner in owners
            if "open_gap" in owner["bbb_evidence"]
        }
        self.assertEqual(open_gaps, {})
        scoped = {
            owner["vanilla_owner"]: owner["bbb_evidence"]["scope_ruling"]
            for owner in owners
            if "scope_ruling" in owner["bbb_evidence"]
        }
        self.assertEqual(len(scoped), 5)
        self.assertTrue(
            all(ruling["reopen_condition"] for ruling in scoped.values())
        )
        self.assertIn(
            "net.minecraft.util.HttpUtil#HttpUtil()",
            scoped,
        )

        http_port_owners = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] == "net/minecraft/util/HttpUtil.java"
            and owner["vanilla_owner"].split("#", 1)[1].split("(", 1)[0]
            in {"getAvailablePort", "isPortAvailable"}
        ]
        self.assertEqual(len(http_port_owners), 2)
        self.assertTrue(
            all(
                owner["mechanism_id"] == "scope-singleplayer-integrated"
                and owner["scope"] == "out-of-scope"
                for owner in http_port_owners
            )
        )

    def test_toast_owner_census_has_complete_evidence_scope_and_open_gaps(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        toast_paths = {
            "net/minecraft/client/PeriodicNotificationManager.java": 10,
            "net/minecraft/client/gui/components/toasts/AdvancementToast.java": 6,
            "net/minecraft/client/gui/components/toasts/NowPlayingToast.java": 16,
            "net/minecraft/client/gui/components/toasts/RecipeToast.java": 6,
            "net/minecraft/client/gui/components/toasts/SystemToast.java": 24,
            "net/minecraft/client/gui/components/toasts/Toast.java": 13,
            "net/minecraft/client/gui/components/toasts/ToastManager.java": 21,
            "net/minecraft/client/gui/components/toasts/TutorialToast.java": 11,
        }
        owners = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] in toast_paths
        ]

        self.assertEqual(len(owners), 107)
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in owners),
            Counter(toast_paths),
        )
        self.assertTrue(
            all(owner["mechanism_id"] == "hud-frame-extract-order" for owner in owners)
        )
        self.assertTrue(
            all(
                (
                    owner["bbb_evidence"].get("implementation")
                    and owner["bbb_evidence"].get("tests")
                )
                or set(owner["bbb_evidence"]) == {"open_gap"}
                for owner in owners
            )
        )
        gaps = [
            owner["bbb_evidence"]["open_gap"]
            for owner in owners
            if "open_gap" in owner["bbb_evidence"]
        ]
        self.assertEqual(len(gaps), 1)
        self.assertTrue(
            all(
                gap["ledger"]
                == "docs/unsupported-features.md#Machine-readable parity acceptance closure"
                and gap["artifact"]
                == "docs/evidence/vanilla-26.1/file-drop-capability.json"
                and gap["reproduce_command"]
                == "scripts/python scripts/generate-file-drop-capability.py --vanilla-root "
                '"$(scripts/vanilla path 26.1 --kind root)" --check'
                and gap["reason"]
                and gap["close_condition"]
                for gap in gaps
            )
        )
        timed_tutorial_owners = [
            owner
            for owner in owners
            if owner["source"]["path"]
            == "net/minecraft/client/gui/components/toasts/TutorialToast.java"
            and owner["vanilla_owner"].split("#", 1)[1].split("(", 1)[0]
            == "TutorialToast"
        ]
        self.assertEqual(len(timed_tutorial_owners), 2)
        self.assertTrue(
            all(
                "crates/bbb-world/src/toasts.rs#pub fn "
                "show_social_interactions_tutorial_if_first_join"
                in owner["bbb_evidence"]["implementation"]
                and "crates/bbb-native/src/runtime/tests.rs#"
                "social_tutorial_toast_freezes_localized_key_content_and_hides_strictly_after_8000ms"
                in owner["bbb_evidence"]["tests"]
                for owner in timed_tutorial_owners
            )
        )
        dynamic_title_owners = [
            owner
            for owner in owners
            if owner["source"]["path"]
            == "net/minecraft/client/gui/components/toasts/SystemToast.java"
            and owner["vanilla_owner"].split("#", 1)[1].split("(", 1)[0]
            in {"SystemToast", "extractRenderState", "reset"}
        ]
        self.assertEqual(len(dynamic_title_owners), 4)
        self.assertTrue(
            all(
                "crates/bbb-native/src/runtime/tests.rs"
                "#system_toast_title_relocalizes_while_message_lines_and_width_stay_frozen"
                in owner["bbb_evidence"]["tests"]
                for owner in dynamic_title_owners
            )
        )
        scoped = [
            owner["bbb_evidence"]["scope_ruling"]
            for owner in owners
            if "scope_ruling" in owner["bbb_evidence"]
        ]
        self.assertEqual(len(scoped), 6)
        self.assertTrue(
            all(
                ruling["conclusion"] == "not-reachable-in-current-scope"
                and ruling["vanilla_basis"]
                and ruling["current_scope_reason"]
                and ruling["reopen_condition"]
                for ruling in scoped
            )
        )
        mechanism = value["mechanisms"]["hud-frame-extract-order"]
        self.assertEqual(mechanism["owner_count"], 118)
        self.assertEqual(mechanism["owner_evidence_count"], 118)
        self.assertEqual(mechanism["owner_gap_count"], 1)

    def test_music_toast_display_state_is_hashed_and_routed_to_startup_consumption(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"]
            == "net/minecraft/client/MusicToastDisplayState.java"
        ]

        self.assertEqual(len(owners), 6)
        self.assertTrue(
            all(owner["mechanism_id"] == "persistence-options-apply" for owner in owners)
        )
        self.assertTrue(
            all(
                owner["bbb_evidence"].get("implementation")
                and owner["bbb_evidence"].get("tests")
                for owner in owners
            )
        )
        self.assertEqual(
            sum("scope_ruling" in owner["bbb_evidence"] for owner in owners),
            2,
        )
    def test_minecraft_method_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
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

    def test_local_player_and_game_mode_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        selected_paths = {
            "net/minecraft/client/player/LocalPlayer.java": 104,
            "net/minecraft/client/multiplayer/MultiPlayerGameMode.java": 39,
        }
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] in selected_paths
            and owner["kind"] in {"method", "constructor"}
        ]
        by_source = Counter(owner["source"]["path"] for owner in methods)
        by_mechanism = Counter(owner["mechanism_id"] for owner in methods)

        self.assertEqual(len(methods), 143)
        self.assertEqual(by_source, Counter(selected_paths))
        self.assertEqual(
            by_mechanism,
            Counter(
                {
                    "player-local-session-respawn": 4,
                    "player-local-vitals-status": 4,
                    "player-local-movement-reporting": 4,
                    "player-local-input-movement": 20,
                    "player-local-vehicle-control": 7,
                    "player-local-permission-chat-state": 6,
                    "player-local-client-status-options": 4,
                    "player-local-item-use-state": 6,
                    "player-local-inventory-actions": 9,
                    "player-local-world-edit-ui": 9,
                    "player-local-environment-effects": 8,
                    "player-local-camera-targeting": 8,
                    "interaction-game-mode-lifecycle": 6,
                    "interaction-game-mode-guards": 5,
                    "interaction-block-breaking": 6,
                    "interaction-prediction-sequence": 1,
                    "interaction-carried-slot-sync": 2,
                    "interaction-item-use": 4,
                    "interaction-entity-actions": 4,
                    "interaction-container-actions": 4,
                    "interaction-creative-actions": 2,
                    "interaction-pick-actions": 2,
                    "scope-support-accessors": 18,
                }
            ),
        )
        self.assertNotIn("world-local-player-decisions", value["mechanisms"])
        self.assertNotIn("interaction-game-mode-decisions", value["mechanisms"])
        generator_source = Path(inventory.__file__).read_text(encoding="utf-8")
        self.assertNotIn('"world-local-player-decisions"', generator_source)
        self.assertNotIn('"interaction-game-mode-decisions"', generator_source)

        minecraft_interaction_methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] == "net/minecraft/client/Minecraft.java"
            and owner["vanilla_owner"].split("#", 1)[1].split("(", 1)[0]
            in {
                "continueAttack",
                "startAttack",
                "startUseItem",
                "canSwitchGameMode",
                "pickBlockOrEntity",
                "pick",
            }
        ]
        self.assertEqual(len(minecraft_interaction_methods), 6)
        self.assertEqual(len(methods) + len(minecraft_interaction_methods), 149)

        input_movement_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-input-movement"
        ]
        self.assertEqual(len(input_movement_owners), 20)
        self.assertEqual(
            value["mechanisms"]["player-local-input-movement"]["owner_evidence_count"],
            20,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in input_movement_owners
            )
        )
        environment_effects_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-environment-effects"
        ]
        self.assertEqual(len(environment_effects_owners), 8)
        self.assertEqual(
            value["mechanisms"]["player-local-environment-effects"][
                "owner_evidence_count"
            ],
            8,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in environment_effects_owners
            )
        )
        world_edit_ui_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-world-edit-ui"
        ]
        self.assertEqual(len(world_edit_ui_owners), 9)
        self.assertEqual(
            value["mechanisms"]["player-local-world-edit-ui"][
                "owner_evidence_count"
            ],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in world_edit_ui_owners
            )
        )
        camera_targeting_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-camera-targeting"
        ]
        # The ninth owner (`Minecraft#pick`) lives outside this partition's
        # source filter; the mechanism-level count covers the full route.
        self.assertEqual(len(camera_targeting_owners), 8)
        self.assertEqual(
            value["mechanisms"]["player-local-camera-targeting"][
                "owner_evidence_count"
            ],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in camera_targeting_owners
            )
        )
        inventory_actions_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-inventory-actions"
        ]
        self.assertEqual(len(inventory_actions_owners), 9)
        self.assertEqual(
            value["mechanisms"]["player-local-inventory-actions"][
                "owner_evidence_count"
            ],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in inventory_actions_owners
            )
        )
        vehicle_control_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-vehicle-control"
        ]
        self.assertEqual(len(vehicle_control_owners), 7)
        self.assertEqual(
            value["mechanisms"]["player-local-vehicle-control"][
                "owner_evidence_count"
            ],
            7,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in vehicle_control_owners
            )
        )
        item_use_state_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-item-use-state"
        ]
        self.assertEqual(len(item_use_state_owners), 6)
        self.assertEqual(
            value["mechanisms"]["player-local-item-use-state"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in item_use_state_owners
            )
        )
        permission_chat_state_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-permission-chat-state"
        ]
        self.assertEqual(len(permission_chat_state_owners), 6)
        self.assertEqual(
            value["mechanisms"]["player-local-permission-chat-state"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in permission_chat_state_owners
            )
        )
        game_mode_lifecycle_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "interaction-game-mode-lifecycle"
        ]
        # The seventh owner (`Minecraft#canSwitchGameMode`) lives outside this
        # partition's source filter; the mechanism-level count covers the route.
        self.assertEqual(len(game_mode_lifecycle_owners), 6)
        self.assertEqual(
            value["mechanisms"]["interaction-game-mode-lifecycle"][
                "owner_evidence_count"
            ],
            7,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in game_mode_lifecycle_owners
            )
        )
        # All three primary-action-routing owners live in Minecraft.java,
        # outside this partition's source filter.
        self.assertEqual(
            value["mechanisms"]["interaction-primary-action-routing"][
                "owner_evidence_count"
            ],
            3,
        )
        # ui-screen-lifecycle's three owners also live in Minecraft.java.
        self.assertEqual(
            value["mechanisms"]["ui-screen-lifecycle"]["owner_evidence_count"],
            3,
        )
        game_mode_guards_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "interaction-game-mode-guards"
        ]
        self.assertEqual(len(game_mode_guards_owners), 5)
        self.assertEqual(
            value["mechanisms"]["interaction-game-mode-guards"][
                "owner_evidence_count"
            ],
            5,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in game_mode_guards_owners
            )
        )
        session_respawn_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-session-respawn"
        ]
        self.assertEqual(len(session_respawn_owners), 4)
        self.assertEqual(
            value["mechanisms"]["player-local-session-respawn"][
                "owner_evidence_count"
            ],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in session_respawn_owners
            )
        )
        vitals_status_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-vitals-status"
        ]
        self.assertEqual(len(vitals_status_owners), 4)
        self.assertEqual(
            value["mechanisms"]["player-local-vitals-status"][
                "owner_evidence_count"
            ],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in vitals_status_owners
            )
        )
        movement_reporting_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "player-local-movement-reporting"
        ]
        self.assertEqual(len(movement_reporting_owners), 4)
        self.assertEqual(
            value["mechanisms"]["player-local-movement-reporting"][
                "owner_evidence_count"
            ],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in movement_reporting_owners
            )
        )
        for mechanism_id, expected_count in (
            ("player-local-client-status-options", 4),
            ("interaction-item-use", 4),
            ("interaction-entity-actions", 4),
            ("interaction-container-actions", 4),
            ("interaction-creative-actions", 2),
        ):
            batch_owners = [
                owner
                for owner in methods
                if owner["mechanism_id"] == mechanism_id
            ]
            self.assertEqual(len(batch_owners), expected_count, mechanism_id)
            self.assertEqual(
                value["mechanisms"][mechanism_id]["owner_evidence_count"],
                expected_count,
                mechanism_id,
            )
            self.assertTrue(
                all(
                    set(owner["bbb_evidence"]) == {"implementation", "tests"}
                    and owner["bbb_evidence"]["implementation"]
                    and owner["bbb_evidence"]["tests"]
                    for owner in batch_owners
                ),
                mechanism_id,
            )
        # pick-actions' third owner (`Minecraft#pickBlockOrEntity`) lives
        # outside this partition's source filter.
        for mechanism_id, partition_count, evidence_count in (
            ("interaction-carried-slot-sync", 2, 2),
            ("interaction-pick-actions", 2, 3),
        ):
            closing_owners = [
                owner
                for owner in methods
                if owner["mechanism_id"] == mechanism_id
            ]
            self.assertEqual(len(closing_owners), partition_count, mechanism_id)
            self.assertEqual(
                value["mechanisms"][mechanism_id]["owner_evidence_count"],
                evidence_count,
                mechanism_id,
            )
            self.assertTrue(
                all(
                    set(owner["bbb_evidence"]) == {"implementation", "tests"}
                    and owner["bbb_evidence"]["implementation"]
                    and owner["bbb_evidence"]["tests"]
                    for owner in closing_owners
                ),
                mechanism_id,
            )
        block_breaking_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "interaction-block-breaking"
        ]
        self.assertEqual(len(block_breaking_owners), 6)
        self.assertEqual(
            value["mechanisms"]["interaction-block-breaking"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in block_breaking_owners
            )
        )

    def test_client_level_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"]
            == "net/minecraft/client/multiplayer/ClientLevel.java"
            and owner["kind"] in {"method", "constructor"}
        ]
        by_mechanism = Counter(owner["mechanism_id"] for owner in methods)

        self.assertEqual(len(methods), 121)
        self.assertEqual(
            by_mechanism,
            Counter(
                {
                    "audio-client-level-sound-routing": 6,
                    "diagnostics-client-level-report": 4,
                    "interaction-prediction-sequence": 4,
                    "rendering-client-biome-tint": 6,
                    "rendering-client-block-entity-tracking": 1,
                    "rendering-client-level-dirty-handoff": 6,
                    "rendering-client-particle-routing": 6,
                    "scope-client-level-noop-hooks": 5,
                    "scope-support-accessors": 17,
                    "world-client-ambient-animation": 9,
                    "world-client-chunk-light-lifecycle": 5,
                    "world-client-clock-weather": 8,
                    "world-client-entity-lifecycle": 17,
                    "world-client-level-events": 3,
                    "world-client-level-lifecycle": 3,
                    "world-client-map-data": 4,
                    "world-client-respawn-data": 9,
                    "world-client-session-services": 8,
                }
            ),
        )
        self.assertNotIn("world-client-level-decisions", by_mechanism)
        level_lifecycle_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-level-lifecycle"
        ]
        self.assertEqual(len(level_lifecycle_owners), 3)
        self.assertEqual(
            value["mechanisms"]["world-client-level-lifecycle"][
                "owner_evidence_count"
            ],
            3,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in level_lifecycle_owners
            )
        )
        clock_weather_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-clock-weather"
        ]
        self.assertEqual(len(clock_weather_owners), 8)
        self.assertEqual(
            value["mechanisms"]["world-client-clock-weather"]["owner_evidence_count"],
            8,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in clock_weather_owners
            )
        )
        block_entity_tracking_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-client-block-entity-tracking"
        ]
        self.assertEqual(len(block_entity_tracking_owners), 1)
        self.assertEqual(
            value["mechanisms"]["rendering-client-block-entity-tracking"][
                "owner_evidence_count"
            ],
            1,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in block_entity_tracking_owners
            )
        )
        level_events_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-level-events"
        ]
        self.assertEqual(len(level_events_owners), 3)
        self.assertEqual(
            value["mechanisms"]["world-client-level-events"]["owner_evidence_count"],
            3,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in level_events_owners
            )
        )
        map_data_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-map-data"
        ]
        self.assertEqual(len(map_data_owners), 4)
        self.assertEqual(
            value["mechanisms"]["world-client-map-data"]["owner_evidence_count"],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in map_data_owners
            )
        )
        diagnostics_report_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "diagnostics-client-level-report"
        ]
        self.assertEqual(len(diagnostics_report_owners), 4)
        self.assertEqual(
            value["mechanisms"]["diagnostics-client-level-report"][
                "owner_evidence_count"
            ],
            4,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in diagnostics_report_owners
            )
        )
        particle_routing_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-client-particle-routing"
        ]
        self.assertEqual(len(particle_routing_owners), 6)
        self.assertEqual(
            value["mechanisms"]["rendering-client-particle-routing"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in particle_routing_owners
            )
        )
        sound_routing_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "audio-client-level-sound-routing"
        ]
        self.assertEqual(len(sound_routing_owners), 6)
        self.assertEqual(
            value["mechanisms"]["audio-client-level-sound-routing"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in sound_routing_owners
            )
        )
        biome_tint_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-client-biome-tint"
        ]
        self.assertEqual(len(biome_tint_owners), 6)
        self.assertEqual(
            value["mechanisms"]["rendering-client-biome-tint"]["owner_evidence_count"],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in biome_tint_owners
            )
        )
        session_services_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-session-services"
        ]
        self.assertEqual(len(session_services_owners), 8)
        self.assertEqual(
            value["mechanisms"]["world-client-session-services"][
                "owner_evidence_count"
            ],
            8,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in session_services_owners
            )
        )
        respawn_data_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-respawn-data"
        ]
        self.assertEqual(len(respawn_data_owners), 9)
        self.assertEqual(
            value["mechanisms"]["world-client-respawn-data"]["owner_evidence_count"],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in respawn_data_owners
            )
        )
        ambient_animation_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-ambient-animation"
        ]
        self.assertEqual(len(ambient_animation_owners), 9)
        self.assertEqual(
            value["mechanisms"]["world-client-ambient-animation"][
                "owner_evidence_count"
            ],
            9,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in ambient_animation_owners
            )
        )
        dirty_handoff_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "rendering-client-level-dirty-handoff"
        ]
        self.assertEqual(len(dirty_handoff_owners), 6)
        self.assertEqual(
            value["mechanisms"]["rendering-client-level-dirty-handoff"][
                "owner_evidence_count"
            ],
            6,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in dirty_handoff_owners
            )
        )
        chunk_light_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-chunk-light-lifecycle"
        ]
        self.assertEqual(len(chunk_light_owners), 5)
        self.assertEqual(
            value["mechanisms"]["world-client-chunk-light-lifecycle"][
                "owner_evidence_count"
            ],
            5,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in chunk_light_owners
            )
        )
        entity_lifecycle_owners = [
            owner
            for owner in methods
            if owner["mechanism_id"] == "world-client-entity-lifecycle"
        ]
        self.assertEqual(len(entity_lifecycle_owners), 17)
        self.assertEqual(
            value["mechanisms"]["world-client-entity-lifecycle"][
                "owner_evidence_count"
            ],
            17,
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in entity_lifecycle_owners
            )
        )
        self.assertEqual(
            value["selection_contract"]["client_level_method_partition"],
            "explicit homogeneous routes plus CST-derived direct-return support owners; unclassified methods fail closed",
        )

    def test_client_level_unknown_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/ClientLevel.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.multiplayer; class ClientLevel { void unclassifiedDecision() {} }\n",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "ClientLevel method lacks a homogeneous route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "world", "world", "world-client-level-decisions", relative
                    )
                )

    def test_client_level_overloads_share_one_explicit_route(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/ClientLevel.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.multiplayer;
class ClientLevel {
  void addParticle(int value) {}
  void addParticle(String value) {}
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_method_surface(
                inventory.MethodSurface(
                    "world", "world", "world-client-level-decisions", relative
                )
            )

        self.assertEqual(len(builder.owners), 2)
        self.assertEqual(
            {owner["mechanism_id"] for owner in builder.owners},
            {"rendering-client-particle-routing"},
        )

    def test_client_level_support_route_rejects_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/ClientLevel.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.multiplayer;
class ClientLevel {
  Object getWorldBorder() { mutate(); return border; }
  void mutate() {}
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "ClientLevel support route is not a direct accessor",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "world", "world", "world-client-level-decisions", relative
                    )
                )

    def test_client_level_noop_route_rejects_new_statement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/ClientLevel.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.multiplayer;
class ClientLevel {
  void gameEvent() { recordEvent(); }
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "ClientLevel no-op route is not source-empty",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "world", "world", "world-client-level-decisions", relative
                    )
                )

    def test_block_breaking_owner_without_implementation_test_evidence_fails_closed(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.multiplayer; class MultiPlayerGameMode { void stopDestroyBlock() {} }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.INTERACTION_BLOCK_BREAKING_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "decision", "interaction", "fixture", relative
                    )
                )

    def test_local_player_and_game_mode_unknown_methods_fail_closed(self) -> None:
        fixtures = (
            (
                "net/minecraft/client/player/LocalPlayer.java",
                "package net.minecraft.client.player; class LocalPlayer { void newDecision() {} }",
                "LocalPlayer method lacks a homogeneous route",
            ),
            (
                "net/minecraft/client/multiplayer/MultiPlayerGameMode.java",
                "package net.minecraft.client.multiplayer; class MultiPlayerGameMode { void newDecision() {} }",
                "MultiPlayerGameMode method lacks a homogeneous route",
            ),
        )
        for relative, source, message in fixtures:
            with self.subTest(relative=relative), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.write_source(root, relative, source)
                builder = inventory.InventoryBuilder(root)
                with self.assertRaisesRegex(
                    inventory.ClientMechanismInventoryError,
                    message,
                ):
                    builder.add_method_surface(
                        inventory.MethodSurface("decision", "fixture", "fixture", relative)
                    )

    def test_method_route_table_rejects_duplicate_owner_names(self) -> None:
        route = inventory.MethodRoute("decision", "fixture", "fixture-mechanism")
        with self.assertRaisesRegex(ValueError, "duplicate method route: decide"):
            inventory._method_routes(
                (route, ("decide",)),
                (route, ("decide",)),
            )

    def test_game_mode_overloads_share_one_lifecycle_route(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/MultiPlayerGameMode.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.multiplayer;
class MultiPlayerGameMode {
  void setLocalMode(int mode) {}
  void setLocalMode(String mode) {}
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_method_surface(
                inventory.MethodSurface("decision", "interaction", "fixture", relative)
            )

        self.assertEqual(len(builder.owners), 2)
        self.assertEqual(
            {owner["mechanism_id"] for owner in builder.owners},
            {"interaction-game-mode-lifecycle"},
        )

    def test_local_player_support_route_rejects_side_effects(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/player/LocalPlayer.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.player;
class LocalPlayer {
  boolean isLocalPlayer() { mutate(); return true; }
  void mutate() {}
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "support route is not a direct accessor",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface("decision", "movement", "fixture", relative)
                )

    def test_level_renderer_method_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
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

    def test_light_owner_without_implementation_test_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/LevelRenderer.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.renderer; class LevelRenderer { static int getLightCoords(Object level, Object pos) { return 0; } }",
            )
            builder = inventory.InventoryBuilder(root)
            with mock.patch.dict(
                inventory.LEVEL_RENDERER_LIGHT_OWNER_EVIDENCE, {}, clear=True
            ), self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "lacks BBB implementation/test evidence",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-light",
                        "rendering",
                        "rendering-level-light-query",
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

    def test_options_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] == "net/minecraft/client/Options.java"
        ]
        self.assertEqual(len(methods), 145)

        by_mechanism = Counter(owner["mechanism_id"] for owner in methods)
        self.assertEqual(
            dict(by_mechanism),
            {
                "persistence-options-file": 14,
                "persistence-options-derived": 15,
                "persistence-options-apply": 8,
                # Values that round-trip but have no consumer subsystem here.
                "persistence-options-unconsumed": 1,
                "scope-options-ui": 13,
                "scope-support-accessors": 94,
            },
        )
        # The 94 accessors are direct field returns, so they own no decision;
        # the persisted semantics stay with the codec and derived routes.
        self.assertTrue(
            all(
                owner["role"] == "support-non-decision"
                and owner["scope"] == "out-of-scope"
                for owner in methods
                if owner["mechanism_id"] == "scope-support-accessors"
            )
        )
        self.assertTrue(
            all(
                owner["role"] == "decision" and owner["scope"] == "external-client"
                for owner in methods
                if owner["mechanism_id"].startswith("persistence-options-")
            )
        )
        self.assertEqual(
            value["mechanisms"]["persistence-options-unconsumed"][
                "owner_evidence_count"
            ],
            1,
        )
        datafix = next(
            owner
            for owner in methods
            if owner["mechanism_id"] == "persistence-options-unconsumed"
        )
        self.assertIn(
            "crates/bbb-native/src/client_options/datafix.rs#update_to_current_version",
            datafix["bbb_evidence"]["implementation"],
        )
        self.assertIn(
            "crates/bbb-native/src/client_options.rs"
            "#old_options_are_data_fixed_before_current_fields_are_decoded",
            datafix["bbb_evidence"]["tests"],
        )

        server_list = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"]
            == "net/minecraft/client/multiplayer/ServerList.java"
        ]
        self.assertEqual(len(server_list), 13)
        # Only the owners `QuickPlay.joinMultiplayerWorld` and
        # `saveSingleServer` reach persist the file; the rest edit the list for
        # the server-selection screen, which BBB does not have.
        self.assertEqual(
            Counter(owner["mechanism_id"] for owner in server_list),
            Counter(
                {
                    "persistence-server-list": 8,
                    "scope-multiplayer-server-list": 5,
                }
            ),
        )
        self.assertTrue(
            all(
                owner["role"] == "decision" and owner["scope"] == "external-client"
                for owner in server_list
                if owner["mechanism_id"] == "persistence-server-list"
            )
        )
        self.assertEqual(
            sorted(
                owner["owner_id"].rsplit("#", 1)[1].split("(")[0]
                for owner in server_list
                if owner["mechanism_id"] == "scope-multiplayer-server-list"
            ),
            ["remove", "replace", "size", "swap", "unhide"],
        )
        self.assertNotIn("persistence-client-files", value["mechanisms"])

    def test_gui_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] == "net/minecraft/client/gui/Gui.java"
        ]

        self.assertEqual(len(methods), 70)
        self.assertEqual(
            Counter(owner["mechanism_id"] for owner in methods),
            Counter(
                {
                    "hud-frame-extract-order": 9,
                    "hud-player-vitals": 19,
                    "hud-hotbar-selection": 6,
                    "hud-overlay-effects": 10,
                    "hud-text-and-lists": 19,
                    # `getFont`/`getChat`/`getTabList`/`getBossOverlay`/
                    # `getDebugOverlay`/`getSpectatorGui`/`getGuiTicks`.
                    "scope-support-accessors": 7,
                }
            ),
        )
        self.assertTrue(
            all(
                owner["role"] == "support-non-decision"
                for owner in methods
                if owner["mechanism_id"] == "scope-support-accessors"
            )
        )
        self.assertNotIn("rendering-hud-step-surface", value["mechanisms"])

    def test_native_input_and_window_partition_is_total_and_homogeneous(self) -> None:
        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        sources = {
            "net/minecraft/client/KeyboardHandler.java": 22,
            "net/minecraft/client/MouseHandler.java": 25,
            "net/minecraft/client/KeyMapping.java": 34,
            "net/minecraft/client/GameNarrator.java": 16,
            "com/mojang/blaze3d/platform/ClipboardManager.java": 3,
            "com/mojang/blaze3d/platform/Window.java": 60,
        }
        methods = [
            owner
            for owner in value["owners"]
            if owner["source"]["path"] in sources
            and owner["mechanism_id"] != "protocol-serverbound-call-sites"
        ]
        self.assertEqual(
            Counter(owner["source"]["path"] for owner in methods), Counter(sources)
        )
        self.assertEqual(
            Counter(owner["mechanism_id"] for owner in methods),
            Counter(
                {
                    "input-keyboard-routing": 7,
                    "input-debug-key-actions": 13,
                    "input-mouse-routing": 19,
                    "input-key-mapping": 30,
                    "ui-accessibility-surface": 16,
                    "ui-clipboard-surface": 5,
                    "platform-window-lifecycle": 16,
                    "platform-window-display-mode": 10,
                    "platform-window-surface-state": 13,
                    "platform-window-chrome": 4,
                    # Six `MouseHandler` position/button reads, four `KeyMapping`
                    # name/category/default/down reads and 17 `Window` geometry
                    # and flag reads.
                    "scope-support-accessors": 27,
                }
            ),
        )
        self.assertTrue(
            all(
                owner["role"] == "support-non-decision"
                for owner in methods
                if owner["mechanism_id"] == "scope-support-accessors"
            )
        )

    def test_unclassified_window_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "com/mojang/blaze3d/platform/Window.java"
            self.write_source(
                root,
                relative,
                "package com.mojang.blaze3d.platform;"
                " class Window { void setOpacity(float value) { this.opacity = value; } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "Window method lacks a decision/support route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "platform",
                        "platform",
                        "platform-window-lifecycle",
                        relative,
                    )
                )

    def test_unclassified_keyboard_handler_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/KeyboardHandler.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client;"
                " class KeyboardHandler { void onGamepad(int button) { this.route(button); } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "KeyboardHandler method lacks a homogeneous route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "input",
                        "input",
                        "input-keyboard-routing",
                        relative,
                    )
                )

    def test_unclassified_gui_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/gui/Gui.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.gui;"
                " class Gui { void extractNewLayer() { this.layers.add(1); } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "Gui method lacks a decision/support route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "render-step",
                        "ui",
                        "hud-frame-extract-order",
                        relative,
                    )
                )

    def test_unclassified_server_list_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/multiplayer/ServerList.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client.multiplayer;"
                " class ServerList { void prune() { this.serverList.clear(); } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "ServerList method lacks a homogeneous route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "persistence",
                        "persistence",
                        "persistence-server-list",
                        relative,
                    )
                )

    def test_unclassified_options_method_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/Options.java"
            self.write_source(
                root,
                relative,
                "package net.minecraft.client; class Options { int decide() { return ready ? 1 : 0; } }",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaisesRegex(
                inventory.ClientMechanismInventoryError,
                "Options method lacks a decision/support route",
            ):
                builder.add_method_surface(
                    inventory.MethodSurface(
                        "persistence",
                        "persistence",
                        "persistence-options-file",
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

        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
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

    def test_block_entity_renderer_registrations_carry_per_owner_evidence(self) -> None:
        mechanism_id = "rendering-block-entity-registration"
        self.assertIn(mechanism_id, inventory.OWNER_EVIDENCE_MECHANISMS)

        generated = (
            inventory.REPO_ROOT
            / "docs/evidence/vanilla-26.1/client-mechanism-inventory.json"
        )
        value = json.loads(generated.read_text(encoding="utf-8"))
        owners = [
            owner
            for owner in value["owners"]
            if owner["mechanism_id"] == mechanism_id
        ]
        self.assertEqual(len(owners), 27)
        self.assertEqual(value["mechanisms"][mechanism_id]["owner_count"], 27)
        self.assertEqual(
            value["mechanisms"][mechanism_id]["owner_evidence_count"], 27
        )
        self.assertTrue(
            all(
                set(owner["bbb_evidence"]) == {"implementation", "tests"}
                and owner["bbb_evidence"]["implementation"]
                and owner["bbb_evidence"]["tests"]
                for owner in owners
            )
        )
        # Every registered `BlockEntityType` resolves to its own evidence cell;
        # sharing one cell across several types (chests, signs, spawners) is a
        # single BBB owner rendering several Vanilla registrations.
        registered = {
            owner["vanilla_owner"].split("(", 1)[1].split(",", 1)[0]
            for owner in owners
        }
        self.assertEqual(
            registered, set(inventory.BLOCK_ENTITY_RENDERER_OWNER_EVIDENCE)
        )

    def test_registration_owner_without_evidence_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/client/renderer/blockentity/BlockEntityRenderers.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.client.renderer.blockentity;
public class BlockEntityRenderers {
  private static void bootstrap() {
    register(BlockEntityType.NEWLY_ADDED, NewRenderer::new);
  }
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            with self.assertRaises(inventory.ClientMechanismInventoryError):
                builder.add_registration_surface(
                    "block-entity-renderer",
                    "rendering",
                    "rendering-block-entity-registration",
                    relative,
                    ("register",),
                )

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
