from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from core_registry_catalog_extractor import (  # noqa: E402
    CoreRegistryExtractionError,
    block_entity_type_entries,
    damage_type_entries,
    data_component_predicate_type_entries,
    data_component_type_entries,
    entity_type_entries,
    particle_type_entries,
)
from vanilla_java_source import parse_source  # noqa: E402


class CoreRegistryCatalogExtractorTests(unittest.TestCase):
    def test_damage_keys_follow_bootstrap_call_order(self) -> None:
        source = parse_source(
            "fixture/DamageTypes.java",
            b"""
package fixture;
public interface DamageTypes {
  ResourceKey<DamageType> FIRST = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("first"));
  ResourceKey<DamageType> SECOND = ResourceKey.create(
      Registries.DAMAGE_TYPE, Identifier.withDefaultNamespace("second"));
  static void bootstrap(Context context) {
    // context.register(FAKE, fake());
    context.register(SECOND, value()); context.register(FIRST, value());
  }
}
""",
        )
        self.assertEqual(
            [(entry.constant_name, entry.resource_id) for entry in damage_type_entries(source)],
            [("SECOND", "minecraft:second"), ("FIRST", "minecraft:first")],
        )

    def test_entity_builder_property_comes_from_initializer_calls(self) -> None:
        source = parse_source(
            "fixture/EntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register(
      "one", Builder.of(One::new).sized(1, 1));
  public static final EntityType<Two> TWO = register(
      "two", Builder.of(Two::new)
          .notInPeaceful());
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(TWO);
  private static String fake = ".notInPeaceful() register(\\\"fake\\\")";
}
""",
        )
        entries = entity_type_entries(source, expected_count=2)
        self.assertEqual([entry.resource_id for entry in entries], ["minecraft:one", "minecraft:two"])
        self.assertEqual([entry.allowed_in_peaceful for entry in entries], [True, False])
        self.assertEqual([entry.only_op_can_set_nbt for entry in entries], [False, True])

    def test_other_registry_field_shapes_use_cst_arguments_and_builder_calls(self) -> None:
        block_source = parse_source(
            "fixture/BlockEntityType.java",
            b"""
package fixture;
public class BlockEntityType {
  public static final BlockEntityType<One> ONE = register(
      "one", One::new, Blocks.ONE, Blocks.OTHER);
  private static final Set<BlockEntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(ONE);
}
""",
        )
        predicate_source = parse_source(
            "fixture/DataComponentPredicates.java",
            b"""
package fixture;
public class DataComponentPredicates {
  public static final DataComponentPredicate.Type<One> ONE = register(
      "one", One.CODEC);
}
""",
        )
        component_source = parse_source(
            "fixture/DataComponents.java",
            b"""
package fixture;
public class DataComponents {
  public static final DataComponentType<One> ONE = register(
      "one", builder -> builder.persistent(One.CODEC));
  public static final DataComponentType<Two> TWO = register(
      "two", builder -> builder.networkSynchronized(Two.CODEC));
}
""",
        )
        particle_source = parse_source(
            "fixture/ParticleTypes.java",
            b"""
package fixture;
public class ParticleTypes {
  public static final SimpleParticleType ONE = register("one", true);
  public static final ParticleType<Two> TWO = register(
      "two", false, Two.CODEC, Two.STREAM_CODEC);
}
""",
        )
        self.assertEqual(
            block_entity_type_entries(block_source, expected_count=1)[0].resource_id,
            "minecraft:one",
        )
        self.assertTrue(
            block_entity_type_entries(block_source, expected_count=1)[0].only_op_can_set_nbt
        )
        self.assertEqual(
            data_component_predicate_type_entries(predicate_source, expected_count=1)[
                0
            ].constant_name,
            "ONE",
        )
        self.assertEqual(
            [entry.persistent for entry in data_component_type_entries(component_source, 2)],
            [True, False],
        )
        self.assertEqual(
            [entry.override_limiter for entry in particle_type_entries(particle_source, 2)],
            [True, False],
        )

    def test_unsupported_registration_fails_with_source_span(self) -> None:
        source = parse_source(
            "fixture/BadEntityType.java",
            b"""
package fixture;
public class BadEntityType {
  public static final EntityType<One> ONE = guess("one", Builder.of(One::new));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of();
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"fixture/BadEntityType\.java:\d+:\d+: .*not initialized by register",
        ):
            entity_type_entries(source, expected_count=1)

    def test_operator_only_set_rejects_unknown_registry_constants(self) -> None:
        source = parse_source(
            "fixture/UnknownOperatorEntityType.java",
            b"""
package fixture;
public class EntityType {
  public static final EntityType<One> ONE = register("one", Builder.of(One::new));
  private static final Set<EntityType<?>> OP_ONLY_CUSTOM_DATA = Set.of(MISSING);
}
""",
        )
        with self.assertRaisesRegex(
            CoreRegistryExtractionError,
            r"UnknownOperatorEntityType\.java: OP_ONLY_CUSTOM_DATA references unknown entity types",
        ):
            entity_type_entries(source, expected_count=1)


if __name__ == "__main__":
    unittest.main()
