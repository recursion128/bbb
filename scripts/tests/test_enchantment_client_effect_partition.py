from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
sys.path.insert(0, str(SCRIPTS))

from vanilla_java_source import JavaMethod, JavaSource, descendants, parse_source  # noqa: E402


CLIENT_COMPONENTS = {
    "ATTRIBUTES",
    "CROSSBOW_CHARGE_TIME",
    "TRIDENT_SOUND",
    "PREVENT_ARMOR_CHANGE",
    "TRIDENT_SPIN_ATTACK_STRENGTH",
}

SERVER_CONDITIONAL_COMPONENTS = {
    "DAMAGE_PROTECTION",
    "DAMAGE_IMMUNITY",
    "DAMAGE",
    "SMASH_DAMAGE_PER_FALLEN_BLOCK",
    "KNOCKBACK",
    "ARMOR_EFFECTIVENESS",
    "POST_ATTACK",
    "POST_PIERCING_ATTACK",
    "HIT_BLOCK",
    "ITEM_DAMAGE",
    "EQUIPMENT_DROPS",
    "LOCATION_CHANGED",
    "TICK",
    "AMMO_USE",
    "PROJECTILE_PIERCING",
    "PROJECTILE_SPAWNED",
    "PROJECTILE_SPREAD",
    "PROJECTILE_COUNT",
    "TRIDENT_RETURN_ACCELERATION",
    "FISHING_TIME_REDUCTION",
    "FISHING_LUCK_BONUS",
    "BLOCK_EXPERIENCE",
    "MOB_EXPERIENCE",
    "REPAIR_WITH_XP",
}

SERVER_SPECIAL_COMPONENTS = {
    "CROSSBOW_CHARGING_SOUNDS",
    "PREVENT_EQUIPMENT_DROP",
}


class EnchantmentClientEffectPartitionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        paths = {
            "components": "net/minecraft/world/item/enchantment/EnchantmentEffectComponents.java",
            "enchantment": "net/minecraft/world/item/enchantment/Enchantment.java",
            "helper": "net/minecraft/world/item/enchantment/EnchantmentHelper.java",
            "crossbow": "net/minecraft/world/item/CrossbowItem.java",
            "trident": "net/minecraft/world/item/TridentItem.java",
            "armor_slot": "net/minecraft/world/inventory/ArmorSlot.java",
            "crossbow_pull": (
                "net/minecraft/client/renderer/item/properties/numeric/CrossbowPull.java"
            ),
            "item_in_hand": "net/minecraft/client/renderer/ItemInHandRenderer.java",
        }
        missing = [relative for relative in paths.values() if not (SOURCE_ROOT / relative).is_file()]
        if missing:
            raise unittest.SkipTest("local vanilla 26.1 sources are unavailable")
        cls.sources = {
            name: parse_source(relative, (SOURCE_ROOT / relative).read_bytes())
            for name, relative in paths.items()
        }

    def test_synchronized_component_partition_is_exhaustive(self) -> None:
        component_source = self.sources["components"]
        actual = {
            field.name
            for field in component_source.fields
            if field.type_spelling.startswith("DataComponentType")
        }
        expected = (
            CLIENT_COMPONENTS
            | SERVER_CONDITIONAL_COMPONENTS
            | SERVER_SPECIAL_COMPONENTS
        )
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 31)

    def test_conditional_effect_components_enter_only_server_level_methods(self) -> None:
        execution_sources = [self.sources["enchantment"], self.sources["helper"]]
        for component in sorted(SERVER_CONDITIONAL_COMPONENTS):
            usages = [
                method
                for source in execution_sources
                for method in source.methods
                if self.method_references_component(source, method, component)
            ]
            self.assertTrue(usages, component)
            for method in usages:
                self.assertTrue(
                    any("ServerLevel" in parameter.canonical for parameter in method.parameters),
                    f"{component} escaped ServerLevel through {method.owner}.{method.name}",
                )

    def test_five_client_components_keep_their_client_observable_consumers(self) -> None:
        enchantment = self.sources["enchantment"]
        crossbow_method = self.single_method(enchantment, "modifyCrossbowChargeTime")
        self.assertTrue(
            self.method_references_component(
                enchantment, crossbow_method, "CROSSBOW_CHARGE_TIME"
            )
        )
        self.assertIn("RandomSource", self.method_text(enchantment, crossbow_method))
        self.assertNotIn("ServerLevel", self.method_text(enchantment, crossbow_method))

        trident_strength = self.single_method(
            enchantment, "modifyTridentSpinAttackStrength"
        )
        self.assertTrue(
            self.method_references_component(
                enchantment,
                trident_strength,
                "TRIDENT_SPIN_ATTACK_STRENGTH",
            )
        )
        self.assertIn("RandomSource", self.method_text(enchantment, trident_strength))
        self.assertNotIn("ServerLevel", self.method_text(enchantment, trident_strength))

        helper = self.sources["helper"]
        attribute_consumers = [
            method
            for method in helper.methods
            if self.method_references_component(helper, method, "ATTRIBUTES")
        ]
        self.assertEqual([method.name for method in attribute_consumers], ["forEachModifier"] * 2)
        self.assertTrue(
            all(
                "ServerLevel" not in self.method_text(helper, method)
                for method in attribute_consumers
            )
        )

        armor_slot = self.sources["armor_slot"]
        may_pickup = self.single_method(armor_slot, "mayPickup")
        self.assertTrue(
            self.method_references_component(
                armor_slot, may_pickup, "PREVENT_ARMOR_CHANGE"
            )
        )

        trident = self.sources["trident"]
        release = self.single_method(trident, "releaseUsing")
        release_body = self.method_body_text(trident, release)
        self.assertTrue(
            self.method_references_component(trident, release, "TRIDENT_SOUND")
        )
        self.assertLess(
            release_body.index("EnchantmentEffectComponents.TRIDENT_SOUND"),
            release_body.index("level instanceof ServerLevel"),
        )

    def test_crossbow_render_queries_and_server_owned_sounds_stay_separate(self) -> None:
        crossbow_pull = self.sources["crossbow_pull"]
        pull = self.single_method(crossbow_pull, "get")
        pull_body = self.method_body_text(crossbow_pull, pull)
        self.assertIn("owner.asLivingEntity()", pull_body)
        self.assertIn("CrossbowItem.getChargeDuration(itemStack, entity)", pull_body)

        item_in_hand = self.sources["item_in_hand"]
        render = self.single_method(item_in_hand, "renderArmWithItem")
        self.assertIn(
            "CrossbowItem.getChargeDuration(itemStack, player)",
            self.method_body_text(item_in_hand, render),
        )

        crossbow = self.sources["crossbow"]
        on_use_tick = self.single_method(crossbow, "onUseTick")
        on_use_tick_body = self.method_body_text(crossbow, on_use_tick)
        self.assertIn("if (!level.isClientSide())", on_use_tick_body)
        self.assertIn("this.getChargingSounds(itemStack)", on_use_tick_body)
        charging_sounds = self.single_method(crossbow, "getChargingSounds")
        self.assertTrue(
            self.method_references_component(
                crossbow, charging_sounds, "CROSSBOW_CHARGING_SOUNDS"
            )
        )

    def test_prevent_equipment_drop_remains_in_server_drop_lifecycle(self) -> None:
        expected = {
            (
                "net/minecraft/world/entity/decoration/ArmorStand.java",
                "brokenByAnything",
            ),
            ("net/minecraft/world/entity/player/Player.java", "destroyVanishingCursedItems"),
            (
                "net/minecraft/world/entity/animal/equine/AbstractHorse.java",
                "dropEquipment",
            ),
            ("net/minecraft/world/entity/animal/allay/Allay.java", "dropEquipment"),
            ("net/minecraft/world/entity/Mob.java", "dropCustomDeathLoot"),
        }
        token = b"EnchantmentEffectComponents.PREVENT_EQUIPMENT_DROP"
        actual: dict[tuple[str, str], tuple[JavaSource, JavaMethod]] = {}
        for path in SOURCE_ROOT.joinpath("net/minecraft").rglob("*.java"):
            source_bytes = path.read_bytes()
            if token not in source_bytes:
                continue
            relative = path.relative_to(SOURCE_ROOT).as_posix()
            if relative.endswith(
                (
                    "/Enchantments.java",
                    "/EnchantmentEffectComponents.java",
                )
            ):
                continue
            source = parse_source(relative, source_bytes)
            for method in source.methods:
                if self.method_references_component(
                    source, method, "PREVENT_EQUIPMENT_DROP"
                ):
                    actual[(relative, method.name)] = (source, method)
        self.assertEqual(set(actual), expected)

        for key, (source, method) in actual.items():
            if key[1] == "destroyVanishingCursedItems":
                drop_equipment = self.single_method(source, "dropEquipment")
                drop_body = self.method_body_text(source, drop_equipment)
                self.assertIn("ServerLevel", self.method_text(source, drop_equipment))
                self.assertIn("this.destroyVanishingCursedItems()", drop_body)
            else:
                self.assertTrue(
                    any(
                        "ServerLevel" in parameter.canonical
                        for parameter in method.parameters
                    ),
                    f"{key} escaped the server drop lifecycle",
                )

    @staticmethod
    def single_method(source: JavaSource, name: str) -> JavaMethod:
        matches = [method for method in source.methods if method.name == name]
        if len(matches) != 1:
            raise AssertionError(
                f"expected one {source.relative_path}#{name}, found {len(matches)}"
            )
        return matches[0]

    @staticmethod
    def method_references_component(
        source: JavaSource, method: JavaMethod, component: str
    ) -> bool:
        expected = f"EnchantmentEffectComponents.{component}"
        return any(
            node.type == "field_access" and source.text(node) == expected
            for node in descendants(method.node)
        )

    @staticmethod
    def method_text(source: JavaSource, method: JavaMethod) -> str:
        return method.span.bytes(source.source).decode("utf-8")

    @staticmethod
    def method_body_text(source: JavaSource, method: JavaMethod) -> str:
        if method.body is None:
            raise AssertionError(f"{method.owner}.{method.name} has no body")
        return method.body.bytes(source.source).decode("utf-8")


if __name__ == "__main__":
    unittest.main()
