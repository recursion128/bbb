from __future__ import annotations

import ast
import sys
import unittest
from pathlib import Path


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from vanilla_java_source import (  # noqa: E402
    JavaSourceError,
    canonical_expression,
    expression_fact,
    parse_source,
    resolve_class_hierarchy,
)


class VanillaJavaSourceFrontendTests(unittest.TestCase):
    def test_every_production_java_source_reader_uses_the_shared_frontend(
        self,
    ) -> None:
        scripts_root = Path(__file__).resolve().parents[1]
        frontend_consumers = set()
        runtime_probes = set()

        for path in sorted(scripts_root.glob("*.py")):
            if path.name == "vanilla_java_source.py":
                continue
            module = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
            strings = {
                node.value
                for node in ast.walk(module)
                if isinstance(node, ast.Constant) and isinstance(node.value, str)
            }
            call_names = {
                node.func.id
                if isinstance(node.func, ast.Name)
                else node.func.attr
                for node in ast.walk(module)
                if isinstance(node, ast.Call)
                and isinstance(node.func, (ast.Name, ast.Attribute))
            }
            module_imports = {
                alias.name
                for node in module.body
                if isinstance(node, ast.Import)
                for alias in node.names
            }
            declares_java_input = any(".java" in value for value in strings)
            if not declares_java_input:
                continue

            if any("vanilla-runtime" in value for value in strings):
                self.assertIn(
                    "subprocess",
                    module_imports,
                    f"{path.name} declares a runtime Java probe but never executes it",
                )
                self.assertIn(
                    "run",
                    call_names,
                    f"{path.name} declares a runtime Java probe but never runs it",
                )
                self.assertNotIn(
                    "read_bytes",
                    call_names,
                    f"{path.name} mixes the runtime-probe and source-parser boundaries",
                )
                runtime_probes.add(path.name)
                continue
            if "read_bytes" not in call_names:
                continue

            frontend_imports = {
                alias.name
                for node in module.body
                if isinstance(node, ast.ImportFrom)
                and node.module == "vanilla_java_source"
                for alias in node.names
            }
            self.assertTrue(
                {"parse_source", "parse_sources"} & frontend_imports,
                f"{path.name} reads Java source without importing the shared frontend",
            )
            self.assertTrue(
                {"parse_source", "parse_sources"} & call_names,
                f"{path.name} imports but does not call the shared frontend",
            )
            frontend_consumers.add(path.name)

        # Lock the audit scanner itself to representative source and runtime
        # paths so a future refactor cannot make the test pass vacuously.
        self.assertIn("generate-packet-catalog.py", frontend_consumers)
        self.assertIn("generate-entity-renderer-catalog.py", frontend_consumers)
        self.assertEqual(
            runtime_probes,
            {
                "generate-block-placement-defaults.py",
                "generate-block-state-collision-shapes.py",
                "generate-block-state-fluid-replaceability.py",
                "generate-block-state-support-facts.py",
            },
        )

    def test_cst_ignores_fake_calls_and_handles_multiline_generics_nested_types(self) -> None:
        raw = br'''
            package fixture.child;
            import fixture.base.Parent;
            // Fake.call(one, two);
            public class Child<T> extends Parent implements Runnable, java.io.Serializable {
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
        self.assertEqual(
            source.types[0].interface_spellings,
            ("Runnable", "java.io.Serializable"),
        )

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

    def test_class_and_interface_supertypes_resolve_through_shared_type_ir(self) -> None:
        interface = parse_source(
            "fixture/base/Capability.java",
            b"package fixture.base; public interface Capability {}",
        )
        extended = parse_source(
            "fixture/base/Extended.java",
            b"package fixture.base; public interface Extended extends Capability {}",
        )
        child = parse_source(
            "fixture/child/Child.java",
            b"package fixture.child; import fixture.base.Extended; public class Child implements Extended {}",
        )
        types = interface.types + extended.types + child.types
        resolve_class_hierarchy(types)
        self.assertEqual(extended.types[0].interfaces, ("fixture.base.Capability",))
        self.assertEqual(child.types[0].interfaces, ("fixture.base.Extended",))

    def test_nested_enum_body_declarations_are_public_fields_and_methods(self) -> None:
        raw = b"""package fixture; class Owner { enum Variant {
            FIRST(0), SECOND(1); static final Variant DEFAULT = FIRST;
            private final int id; Variant(int id) { this.id = id; }
            int id() { return this.id; }
        } }"""
        source = parse_source("fixture/Owner.java", raw)
        owner = "fixture.Owner.Variant"
        self.assertEqual(
            [(item.name, item.initializer.canonical if item.initializer else None) for item in source.fields_for(owner)],
            [("DEFAULT", "FIRST"), ("id", None)],
        )
        self.assertEqual(
            [(item.name, item.is_constructor) for item in source.methods_for(owner)],
            [("Variant", True), ("id", False)],
        )

    def test_byte_spans_and_canonical_expression_are_whitespace_stable(self) -> None:
        compact = b"package fixture; class X { int v = Factory.make(1, 2); }"
        spaced = b"package fixture; class X { int v = Factory /*x*/ . make ( 1 , 2 ); }"
        first = parse_source("fixture/X.java", compact).fields[0].initializer
        second = parse_source("fixture/X.java", spaced).fields[0].initializer
        self.assertEqual(first.canonical, second.canonical)
        self.assertEqual(first.span.bytes(compact), b"Factory.make(1, 2)")
        self.assertEqual(second.span.bytes(spaced), b"Factory /*x*/ . make ( 1 , 2 )")
        self.assertEqual(canonical_expression(compact, first.node), first.canonical)

    def test_expression_fact_projects_calls_objects_and_arguments_from_cst(self) -> None:
        source = parse_source(
            "fixture/X.java",
            b"package fixture; class X { Object v = Factory.make(Owner.VALUE, new Box(1)); }",
        )
        fact = expression_fact(source, source.fields[0].initializer)

        self.assertEqual(fact["kind"], "method_invocation")
        self.assertEqual(fact["name"], "make")
        self.assertEqual(fact["object"]["canonical"], "Factory")
        self.assertEqual(fact["arguments"][0]["name"], "VALUE")
        self.assertEqual(fact["arguments"][1]["name"], "Box")
        self.assertEqual(fact["arguments"][1]["arguments"][0]["canonical"], "1")

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
