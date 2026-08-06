from __future__ import annotations

import ast
import struct
import sys
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402

from entity_metadata_extractor import (  # noqa: E402
    Accessor,
    EntityTypeRegistration,
    ExtractionError,
    assign_data_ids,
    attach_defaults,
    extract,
    extract_accessors,
    extract_attribute_suppliers,
    extract_flags,
)
from vanilla_java_source import parse_source, resolve_class_hierarchy  # noqa: E402


def fixture_sources(files: dict[str, str]):
    sources = {
        path: parse_source(path, text.encode("utf-8"))
        for path, text in files.items()
    }
    classes = [item for source in sources.values() for item in source.types]
    resolve_class_hierarchy(classes)
    known = {item.fqcn: item for item in classes}
    accessors: list[Accessor] = []
    for source in sources.values():
        accessors.extend(extract_accessors(source, known))
    assign_data_ids(classes, accessors)
    accessor_map = {(item.owner, item.java_name): item for item in accessors}
    for source in sources.values():
        attach_defaults(source, accessor_map)
    return sources, classes, accessors


class EntityMetadataExtractorFixtureTests(unittest.TestCase):
    def test_default_attribute_supplier_membership_follows_factory_inheritance(self) -> None:
        source_text = {
            "net/minecraft/world/entity/ai/attributes/AttributeSupplier.java": """
                package net.minecraft.world.entity.ai.attributes;
                class AttributeSupplier {
                  static Builder builder() { return null; }
                  static class Builder {
                    Builder add(Object value) { return this; }
                    Builder add(Object value, double base) { return this; }
                    AttributeSupplier build() { return null; }
                  }
                }
            """,
            "net/minecraft/world/entity/ai/attributes/Attributes.java": """
                package net.minecraft.world.entity.ai.attributes;
                class Attributes {
                  static final Object MAX_HEALTH = null;
                  static final Object MOVEMENT_SPEED = null;
                }
            """,
            "net/minecraft/world/entity/ai/attributes/DefaultAttributes.java": """
                package net.minecraft.world.entity.ai.attributes;
                import net.minecraft.world.entity.EntityType;
                import net.minecraft.world.entity.monster.Zombie;
                class DefaultAttributes {
                  static final Object SUPPLIERS = Map.builder()
                    .put(EntityType.ZOMBIE, Zombie.createAttributes().build())
                    .build();
                }
            """,
            "net/minecraft/world/entity/EntityType.java": """
                package net.minecraft.world.entity;
                class EntityType { static final Object ZOMBIE = null; static final Object ITEM = null; }
            """,
            "net/minecraft/world/entity/LivingEntity.java": """
                package net.minecraft.world.entity;
                import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
                import net.minecraft.world.entity.ai.attributes.Attributes;
                class LivingEntity {
                  static AttributeSupplier.Builder createLivingAttributes() {
                    return AttributeSupplier.builder().add(Attributes.MAX_HEALTH);
                  }
                }
            """,
            "net/minecraft/world/entity/monster/Zombie.java": """
                package net.minecraft.world.entity.monster;
                import net.minecraft.world.entity.LivingEntity;
                import net.minecraft.world.entity.ai.attributes.AttributeSupplier;
                import net.minecraft.world.entity.ai.attributes.Attributes;
                class Zombie extends LivingEntity {
                  static AttributeSupplier.Builder createAttributes() {
                    return createLivingAttributes().add(Attributes.MOVEMENT_SPEED, 0.23);
                  }
                }
            """,
            "net/minecraft/world/entity/item/ItemEntity.java": """
                package net.minecraft.world.entity.item;
                class ItemEntity {}
            """,
        }
        sources = {
            path: parse_source(path, text.encode("utf-8"))
            for path, text in source_text.items()
        }
        classes = [item for source in sources.values() for item in source.types]
        resolve_class_hierarchy(classes)
        span = classes[0].span
        entity_types = [
            EntityTypeRegistration(
                0,
                "ZOMBIE",
                "minecraft:zombie",
                "net.minecraft.world.entity.monster.Zombie",
                None,
                span,
            ),
            EntityTypeRegistration(
                1,
                "ITEM",
                "minecraft:item",
                "net.minecraft.world.entity.item.ItemEntity",
                None,
                span,
            ),
        ]

        resolved = extract_attribute_suppliers(sources, classes, entity_types)

        self.assertEqual(
            resolved[0].attribute_constants,
            ("MAX_HEALTH", "MOVEMENT_SPEED"),
        )
        self.assertEqual(
            resolved[0].attribute_base_overrides,
            (
                ("MAX_HEALTH", None),
                (
                    "MOVEMENT_SPEED",
                    struct.unpack("!Q", struct.pack("!d", 0.23))[0],
                ),
            ),
        )
        self.assertEqual(
            resolved[0].attribute_factory_owner,
            "net.minecraft.world.entity.monster.Zombie",
        )
        self.assertEqual(resolved[1].attribute_constants, ())
        self.assertEqual(resolved[1].attribute_base_overrides, ())

    def test_multilevel_inheritance_empty_parent_sibling_index_reuse_multiline(self) -> None:
        _, _, accessors = fixture_sources(
            {
                "fixture/Base.java": """
                    package fixture;
                    class Base {
                      static final EntityDataAccessor<Byte> BASE = net.minecraft.network.syncher.SynchedEntityData.defineId(
                          Base.class,
                          net.minecraft.network.syncher.EntityDataSerializers.BYTE);
                      void defineSynchedData(Builder entityData) {
                        entityData.define(BASE, (byte) 0);
                      }
                    }
                """,
                "fixture/Middle.java": "package fixture; class Middle extends Base {}",
                "fixture/Left.java": """
                    package fixture; class Left extends Middle {
                      static final EntityDataAccessor<Boolean> LEFT = SynchedEntityData.defineId(Left.class, EntityDataSerializers.BOOLEAN);
                      void defineSynchedData(Builder b) { b.define(LEFT, false); }
                    }
                """,
                "fixture/Right.java": """
                    package fixture; class Right extends Middle {
                      static final EntityDataAccessor<Integer> RIGHT = SynchedEntityData.defineId(Right.class, EntityDataSerializers.INT);
                      void defineSynchedData(Builder b) { b.define(RIGHT, Optional.empty()); }
                    }
                """,
            }
        )
        values = {(item.owner.rsplit(".", 1)[-1], item.java_name): item for item in accessors}
        self.assertEqual(values[("Base", "BASE")].data_id, 0)
        self.assertEqual(values[("Left", "LEFT")].data_id, 1)
        self.assertEqual(values[("Right", "RIGHT")].data_id, 1)
        self.assertEqual(values[("Base", "BASE")].default_expression, "(byte) 0")
        self.assertEqual(values[("Right", "RIGHT")].default_expression, "Optional.empty ()")

    def test_enum_virtual_nested_and_comment_string_fake_calls(self) -> None:
        _, _, accessors = fixture_sources(
            {
                "fixture/Defaults.java": r'''
                    package fixture;
                    class Defaults {
                      // SynchedEntityData.defineId(Fake.class, EntityDataSerializers.INT)
                      static final String TEXT = "SynchedEntityData.defineId(Fake.class, Fake.BAD)";
                      static final EntityDataAccessor<Pose> POSE = SynchedEntityData.defineId(Defaults.class, EntityDataSerializers.POSE);
                      static final EntityDataAccessor<Integer> AIR = SynchedEntityData.defineId(Defaults.class, EntityDataSerializers.INT);
                      void defineSynchedData(Builder b) {
                        b.define(POSE, Pose.STANDING); b.define(AIR, this.getMaxAirSupply());
                      }
                      class Nested { String text = "b.define(AIR, 0)"; }
                    }
                ''',
            }
        )
        self.assertEqual(len(accessors), 2)
        by_name = {item.java_name: item for item in accessors}
        self.assertEqual(by_name["POSE"].default_expression, "Pose.STANDING")
        self.assertEqual(by_name["AIR"].default_expression, "this.getMaxAirSupply ()")

    def test_flag_shift_hex_or_and_ambiguous_binding(self) -> None:
        sources, _, accessors = fixture_sources(
            {
                "fixture/Flags.java": """
                    package fixture; class Flags {
                      static final EntityDataAccessor<Byte> A = SynchedEntityData.defineId(Flags.class, EntityDataSerializers.BYTE);
                      static final EntityDataAccessor<Byte> B = SynchedEntityData.defineId(Flags.class, EntityDataSerializers.BYTE);
                      static final int FLAG_SHIFT = 1 << 2;
                      static final int FLAG_HEX_OR = (byte) (0x10 | 0x20);
                      static final int FLAG_AMBIGUOUS = 0x40;
                      void defineSynchedData(Builder b) { b.define(A, (byte)0); b.define(B, (byte)0); }
                      boolean shift() { return (data.get(A) & FLAG_SHIFT) != 0; }
                      boolean hex() { return (data.get(A) & FLAG_HEX_OR) != 0; }
                      boolean ambiguous() { return (data.get(A) & FLAG_AMBIGUOUS) != 0 || (data.get(B) & FLAG_AMBIGUOUS) != 0; }
                    }
                """,
            }
        )
        flags = extract_flags(sources["fixture/Flags.java"], accessors)
        by_name = {item.java_name: item for item in flags}
        self.assertEqual(by_name["FLAG_SHIFT"].value, 4)
        self.assertEqual(by_name["FLAG_HEX_OR"].value, 0x30)
        self.assertEqual(by_name["FLAG_SHIFT"].accessor_name, "A")
        self.assertEqual(by_name["FLAG_AMBIGUOUS"].status, "ambiguous")
        self.assertIn("multiple accessor bindings", by_name["FLAG_AMBIGUOUS"].reason)

    def test_missing_and_duplicate_defaults_are_explicit(self) -> None:
        with self.assertRaisesRegex(ExtractionError, "duplicate default"):
            fixture_sources(
                {
                    "fixture/Broken.java": """
                        package fixture; class Broken {
                          static final EntityDataAccessor<Integer> VALUE = SynchedEntityData.defineId(Broken.class, EntityDataSerializers.INT);
                          void defineSynchedData(Builder b) { b.define(VALUE, 0); b.define(VALUE, 1); }
                        }
                    """
                }
            )
        _, _, accessors = fixture_sources(
            {
                "fixture/Missing.java": """
                    package fixture; class Missing {
                      static final EntityDataAccessor<Integer> VALUE = SynchedEntityData.defineId(Missing.class, EntityDataSerializers.INT);
                    }
                """
            }
        )
        self.assertIsNone(accessors[0].default_expression)

    def test_whitespace_and_comments_do_not_change_metadata_semantics(self) -> None:
        compact = {
            "fixture/X.java": "package fixture; class X { static final EntityDataAccessor<Integer> V=SynchedEntityData.defineId(X.class,EntityDataSerializers.INT); void defineSynchedData(Builder b){b.define(V,0);} }"
        }
        spaced = {
            "fixture/X.java": "package fixture; class X { static final EntityDataAccessor<Integer> V /*a*/ = SynchedEntityData . defineId ( X.class , EntityDataSerializers.INT ); void defineSynchedData(Builder b){ b /*b*/ . define ( V , 0 ); } }"
        }
        first = fixture_sources(compact)[2][0]
        second = fixture_sources(spaced)[2][0]
        self.assertEqual(
            (first.owner, first.java_name, first.serializer, first.data_id, first.default_expression),
            (second.owner, second.java_name, second.serializer, second.data_id, second.default_expression),
        )

    def test_extractor_has_no_private_java_parser(self) -> None:
        path = Path(__file__).resolve().parents[1] / "entity_metadata_extractor.py"
        module = ast.parse(path.read_text())
        function_names = {
            item.name for item in module.body if isinstance(item, ast.FunctionDef)
        }
        self.assertFalse(
            function_names
            & {"tokenize_java", "matching_token", "split_arguments", "parse_classes"}
        )

    def test_local_vanilla_cst_closes_227_accessors_defaults_and_157_types(self) -> None:
        source_root = VanillaWorkspace.resolve("26.1").sources
        if not source_root.is_dir():
            self.skipTest("local vanilla 26.1 sources unavailable")
        result = extract(source_root)
        self.assertEqual(len(result.accessors), 227)
        self.assertEqual(
            sum(item.default_expression is not None for item in result.accessors),
            227,
        )
        self.assertEqual(len(result.entity_types), 157)
        suppliers = [
            item for item in result.entity_types if item.attribute_constants
        ]
        self.assertEqual(len(suppliers), 92)
        player = next(item for item in suppliers if item.constant_name == "PLAYER")
        self.assertIn("BLOCK_INTERACTION_RANGE", player.attribute_constants)
        zombie = next(item for item in suppliers if item.constant_name == "ZOMBIE")
        self.assertIn("SPAWN_REINFORCEMENTS_CHANCE", zombie.attribute_constants)
        self.assertTrue(
            all(
                len(item.attribute_constants) == len(item.attribute_base_overrides)
                for item in suppliers
            )
        )
        turtle = next(item for item in suppliers if item.constant_name == "TURTLE")
        self.assertEqual(
            dict(turtle.attribute_base_overrides)["MAX_HEALTH"],
            struct.unpack("!Q", struct.pack("!d", 30.0))[0],
        )
        self.assertIsNone(dict(turtle.attribute_base_overrides)["ARMOR"])
        self.assertIn(
            "net/minecraft/world/level/block/WeatheringCopper.java",
            result.sources,
        )
        self.assertIn(
            "net/minecraft/core/registries/Registries.java",
            result.sources,
        )
        self.assertTrue(all(item.declaration_span.start_byte >= 0 for item in result.accessors))


if __name__ == "__main__":
    unittest.main()
