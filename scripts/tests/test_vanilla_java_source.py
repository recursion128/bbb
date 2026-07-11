from __future__ import annotations

import sys
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from vanilla_java_source import (  # noqa: E402
    JavaSourceError,
    canonical_expression,
    parse_source,
    resolve_class_hierarchy,
)


class VanillaJavaSourceFrontendTests(unittest.TestCase):
    def test_cst_ignores_fake_calls_and_handles_multiline_generics_nested_types(self) -> None:
        raw = br'''
            package fixture.child;
            import fixture.base.Parent;
            // Fake.call(one, two);
            public class Child<T> extends Parent {
                private static final java.util.Map<String,
                    java.util.List<Integer>> VALUES = Factory.make(
                        "Fake.call(ignored)", '}', java.util.List.of(1, 2));

                void run(java.util.Map<String, java.util.List<Integer>> input) {
                    first(); second(qualified.Type.call(input, new int[] {1, 2}));
                }

                record Nested(String value) {}
            }
        '''
        source = parse_source("fixture/child/Child.java", raw)
        self.assertEqual(source.package, "fixture.child")
        self.assertEqual(source.imports["Parent"], "fixture.base.Parent")
        self.assertEqual(
            [item.fqcn for item in source.types],
            ["fixture.child.Child", "fixture.child.Child.Nested"],
        )
        self.assertEqual(source.types[0].superclass_spelling, "Parent")

        values = next(item for item in source.fields if item.name == "VALUES")
        self.assertEqual(
            values.type_spelling.replace(" ", ""),
            "java.util.Map<String,java.util.List<Integer>>",
        )
        self.assertEqual(
            values.initializer.canonical,
            'Factory.make ("Fake.call(ignored)", \'}\', java.util.List.of (1, 2))',
        )
        self.assertEqual(
            values.initializer.span.bytes(raw).lstrip().splitlines()[0], b"Factory.make("
        )

        call_names = [item.name for item in source.calls]
        self.assertNotIn("ignored", call_names)
        qualified = next(
            item
            for item in source.calls
            if item.name == "call" and item.qualifier is not None
        )
        self.assertEqual(qualified.qualifier.canonical, "qualified.Type")
        self.assertEqual(qualified.arguments[0].canonical, "input")
        self.assertEqual(
            qualified.span.bytes(raw),
            b"qualified.Type.call(input, new int[] {1, 2})",
        )
        self.assertEqual([item.name for item in source.methods], ["run"])

    def test_balanced_delimiters_nested_generic_and_same_line_expressions(self) -> None:
        raw = b"""package fixture; class Balanced<T extends Map<String, List<Integer>>> {
            void x(){ target(a[call(1, 2)], new int[]{3, 4}); first(); second(); }
        }"""
        source = parse_source("fixture/Balanced.java", raw)
        target = next(item for item in source.calls if item.name == "target")
        self.assertEqual(
            [item.canonical for item in target.arguments],
            ["a [call (1, 2)]", "new int [] {3, 4}"],
        )
        self.assertEqual(
            [item.name for item in source.calls if item.name in {"first", "second"}],
            ["first", "second"],
        )

    def test_package_import_superclass_resolution_across_units(self) -> None:
        parent = parse_source(
            "fixture/base/Parent.java",
            b"package fixture.base; public class Parent {}",
        )
        child = parse_source(
            "fixture/child/Child.java",
            b"package fixture.child; import fixture.base.Parent; public class Child extends Parent {}",
        )
        types = parent.types + child.types
        resolve_class_hierarchy(types)
        self.assertEqual(child.types[0].superclass, "fixture.base.Parent")

    def test_byte_spans_and_canonical_expression_are_whitespace_stable(self) -> None:
        compact = b"package fixture; class X { int v = Factory.make(1, 2); }"
        spaced = b"package fixture; class X { int v = Factory /*x*/ . make ( 1 , 2 ); }"
        first = parse_source("fixture/X.java", compact).fields[0].initializer
        second = parse_source("fixture/X.java", spaced).fields[0].initializer
        self.assertEqual(first.canonical, second.canonical)
        self.assertEqual(first.span.bytes(compact), b"Factory.make(1, 2)")
        self.assertEqual(second.span.bytes(spaced), b"Factory /*x*/ . make ( 1 , 2 )")
        self.assertEqual(canonical_expression(compact, first.node), first.canonical)

    def test_malformed_java_reports_error_or_missing_span(self) -> None:
        with self.assertRaisesRegex(
            JavaSourceError,
            r"fixture/Broken.java:1:\d+-1:\d+: Tree-sitter Java (ERROR|MISSING) node",
        ):
            parse_source(
                "fixture/Broken.java",
                b"package fixture; class Broken { void x( {",
            )
        with self.assertRaisesRegex(JavaSourceError, r"Tree-sitter Java MISSING node"):
            parse_source(
                "fixture/Missing.java",
                b"package fixture; class Missing { int value }",
            )


if __name__ == "__main__":
    unittest.main()
