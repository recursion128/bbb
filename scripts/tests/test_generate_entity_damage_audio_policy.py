from __future__ import annotations

import importlib.util
import sys
import unittest
from dataclasses import replace
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
SOURCE_ROOT = Path.home() / "Work/mc-code/sources/26.1"
sys.path.insert(0, str(SCRIPTS))

from vanilla_java_source import parse_source  # noqa: E402


def load_generator():
    path = SCRIPTS / "generate-entity-metadata-schema.py"
    spec = importlib.util.spec_from_file_location(
        "generate_entity_metadata_schema_for_test", path
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class GeneratedEntityLivingEventAudioPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.generator = load_generator()
        cls.result = cls.generator.extract(SOURCE_ROOT)
        cls.class_map = {item.fqcn: item for item in cls.result.classes}
        cls.sound_events = cls.generator.sound_event_resources_by_constant(cls.result)

    def is_living(self, runtime_class: str | None) -> bool:
        current = runtime_class
        seen: set[str] = set()
        while current is not None and current not in seen:
            if current == "net.minecraft.world.entity.LivingEntity":
                return True
            seen.add(current)
            item = self.class_map.get(current)
            current = item.superclass if item is not None else None
        return False

    def test_all_fixed_living_types_have_closed_source_policies(self) -> None:
        death_owner_cache: dict[str, str] = {}
        death_policy_cache: dict[str, str] = {}
        hurt_owner_cache: dict[str, str] = {}
        hurt_policy_cache: dict[str, str] = {}
        volume_owner_cache: dict[str, str] = {}
        volume_policy_cache: dict[str, str] = {}
        living = []
        death_owners = set()
        hurt_owners = set()
        volume_owners = set()
        for entry in self.result.entity_types:
            if not self.is_living(entry.runtime_class):
                continue
            living.append(entry.resource_id)
            death_owners.add(
                self.generator.class_nearest_method_owner(
                    self.result,
                    self.class_map,
                    entry.runtime_class,
                    "getDeathSound",
                    0,
                    death_owner_cache,
                )
            )
            hurt_owners.add(
                self.generator.class_nearest_method_owner(
                    self.result,
                    self.class_map,
                    entry.runtime_class,
                    "getHurtSound",
                    1,
                    hurt_owner_cache,
                )
            )
            volume_owners.add(
                self.generator.class_nearest_method_owner(
                    self.result,
                    self.class_map,
                    entry.runtime_class,
                    "getSoundVolume",
                    0,
                    volume_owner_cache,
                )
            )
            death = self.generator.class_death_sound_policy(
                self.result,
                self.class_map,
                entry.runtime_class,
                self.sound_events,
                death_owner_cache,
                death_policy_cache,
            )
            hurt = self.generator.class_hurt_sound_policy(
                self.result,
                self.class_map,
                entry.runtime_class,
                self.sound_events,
                hurt_owner_cache,
                hurt_policy_cache,
            )
            volume = self.generator.class_sound_volume_policy(
                self.result,
                self.class_map,
                entry.runtime_class,
                volume_owner_cache,
                volume_policy_cache,
            )
            self.assertNotIn("NonLiving", death, entry.resource_id)
            self.assertNotIn("NonLiving", hurt, entry.resource_id)
            self.assertNotIn("NonLiving", volume, entry.resource_id)

        self.assertEqual(len(living), 92)
        self.assertEqual(len(death_owners), 88)
        self.assertEqual(len(hurt_owners), 89)
        self.assertEqual(len(volume_owners), 13)

    def test_representative_dynamic_policies_preserve_branch_inputs(self) -> None:
        by_resource = {
            item.resource_id: item.runtime_class for item in self.result.entity_types
        }
        death_owner_cache: dict[str, str] = {}
        death_policy_cache: dict[str, str] = {}
        hurt_owner_cache: dict[str, str] = {}
        hurt_policy_cache: dict[str, str] = {}
        volume_owner_cache: dict[str, str] = {}
        volume_policy_cache: dict[str, str] = {}

        def death(resource_id: str) -> str:
            return self.generator.class_death_sound_policy(
                self.result,
                self.class_map,
                by_resource[resource_id],
                self.sound_events,
                death_owner_cache,
                death_policy_cache,
            )

        def hurt(resource_id: str) -> str:
            return self.generator.class_hurt_sound_policy(
                self.result,
                self.class_map,
                by_resource[resource_id],
                self.sound_events,
                hurt_owner_cache,
                hurt_policy_cache,
            )

        def volume(resource_id: str) -> str:
            return self.generator.class_sound_volume_policy(
                self.result,
                self.class_map,
                by_resource[resource_id],
                volume_owner_cache,
                volume_policy_cache,
            )

        self.assertIn("SoundVariantFamily::Wolf", death("minecraft:wolf"))
        self.assertIn("SoundVariantFamily::Cow", death("minecraft:mooshroom"))
        self.assertIn("UnderWater", death("minecraft:zombie_nautilus"))
        self.assertIn("NautilusAgeWater", death("minecraft:nautilus"))
        self.assertIn("CopperGolemOxidation", death("minecraft:copper_golem"))
        self.assertIn("GoatScreaming", death("minecraft:goat"))
        self.assertIn("Tiny", death("minecraft:slime"))
        self.assertIn("PlayerDamageType", hurt("minecraft:player"))
        self.assertIn("SoundVariantFamily::Cow", hurt("minecraft:cow"))
        self.assertIn("WolfVariant", hurt("minecraft:wolf"))
        self.assertIn("UnderWater", hurt("minecraft:zombie_nautilus"))
        self.assertIn("NautilusAgeWater", hurt("minecraft:nautilus"))
        self.assertIn("CopperGolemOxidation", hurt("minecraft:copper_golem"))
        self.assertIn("CreakingHeartBound", hurt("minecraft:creaking"))
        self.assertIn("SlimeSize", volume("minecraft:magma_cube"))
        self.assertIn("HappyGhastAge", volume("minecraft:happy_ghast"))
        self.assertEqual(
            volume("minecraft:horse"),
            "VanillaEntitySoundVolumePolicy::Fixed(0.8_f32.to_bits())",
        )

    def test_checked_in_snapshot_matches_source_policy_generation(self) -> None:
        generated = self.generator.render(self.result)
        checked_in = (
            REPO_ROOT / "crates/bbb-world/src/entities/schema/generated_26_1.rs"
        ).read_text()
        self.assertEqual(checked_in, generated)

    def test_unrecognized_dynamic_hurt_body_fails_closed(self) -> None:
        owner = "net.minecraft.world.entity.monster.Slime"
        original = self.generator.source_for_owner(self.result, owner)
        changed_sources = dict(self.result.sources)
        changed_sources[original.relative_path] = parse_source(
            original.relative_path,
            b"""
package net.minecraft.world.entity.monster;
class Slime {
  SoundEvent getHurtSound(DamageSource source) {
    return chooseUnknownHurtSound();
  }
}
""",
        )
        changed = replace(self.result, sources=changed_sources)
        with self.assertRaisesRegex(
            self.generator.ExtractionError, "unsupported body"
        ):
            self.generator.class_hurt_sound_policy(
                changed,
                self.class_map,
                owner,
                self.sound_events,
                {},
                {},
            )

    def test_unrecognized_dynamic_death_body_fails_closed(self) -> None:
        owner = "net.minecraft.world.entity.monster.Slime"
        original = self.generator.source_for_owner(self.result, owner)
        changed_sources = dict(self.result.sources)
        changed_sources[original.relative_path] = parse_source(
            original.relative_path,
            b"""
package net.minecraft.world.entity.monster;
class Slime {
  SoundEvent getDeathSound() {
    return chooseUnknownDeathSound();
  }
}
""",
        )
        changed = replace(self.result, sources=changed_sources)
        with self.assertRaisesRegex(
            self.generator.ExtractionError, "unsupported body"
        ):
            self.generator.class_death_sound_policy(
                changed,
                self.class_map,
                owner,
                self.sound_events,
                {},
                {},
            )


if __name__ == "__main__":
    unittest.main()
