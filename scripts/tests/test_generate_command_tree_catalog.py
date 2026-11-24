import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "generate-command-tree-catalog.py"
SPEC = importlib.util.spec_from_file_location("generate_command_tree_catalog", SCRIPT)
GENERATOR = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
# The generator's dataclass resolves its own annotations through sys.modules,
# so the module has to be registered before it runs.
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)


HEADER = "version=26.1 selection=DEDICATED nodes={count} roots={roots} jfr=1 dev_commands=0 chase=0"


def report(*rows: str) -> str:
    roots = sum(1 for row in rows if row.split("\t")[1] == "0")
    header = HEADER.format(count=len(rows) + 1, roots=roots)
    root = "0\t-1\t\troot\t\t0\t-1\t0\t\t\t"
    return "\n".join((header, root, *rows))


ROOT_ROW = "1\t0\tsay\tliteral\tsay\t0\t-1\t2\t\t\t"
ARGUMENT_ROW = (
    "2\t1\tsay message\targument\tmessage\t1\t-1\t2\tminecraft:message"
    "\tminecraft:ask_server\t{\"amount\":\"single\"}"
)


class GenerateCommandTreeCatalogTests(unittest.TestCase):
    def test_parse_tree_projects_columns_and_derives_child_ranges(self) -> None:
        nodes = GENERATOR.parse_tree(report(ROOT_ROW, ARGUMENT_ROW))

        self.assertEqual(len(nodes), 3)
        root, say, message = nodes
        self.assertEqual((root.kind, root.parent, root.name), ("root", None, ""))
        self.assertEqual((root.first_child, root.child_count), (1, 1))
        self.assertEqual((say.parent, say.path, say.permission), (0, ("say",), 2))
        self.assertFalse(say.executable)
        self.assertEqual((say.first_child, say.child_count), (2, 1))
        self.assertEqual(message.path, ("say", "message"))
        self.assertTrue(message.executable)
        self.assertEqual(message.parser, "minecraft:message")
        self.assertEqual(message.suggestions, "minecraft:ask_server")
        self.assertEqual(message.properties, '{"amount":"single"}')
        # A leaf keeps an empty run rather than an out-of-range first index.
        self.assertEqual((message.first_child, message.child_count), (0, 0))

    def test_parse_tree_reads_redirects_as_indices(self) -> None:
        rows = (
            ROOT_ROW,
            "2\t0\ttell\tliteral\ttell\t0\t1\t2\t\t\t",
        )
        nodes = GENERATOR.parse_tree(report(*rows))

        self.assertEqual(nodes[2].redirect, 1)
        self.assertIsNone(nodes[1].redirect)

    def test_parse_tree_rejects_an_environment_that_changes_the_tree(self) -> None:
        source = report(ROOT_ROW).replace("jfr=1", "jfr=0")
        with self.assertRaisesRegex(ValueError, "jfr='0'"):
            GENERATOR.parse_tree(source)

        source = report(ROOT_ROW).replace("dev_commands=0", "dev_commands=1")
        with self.assertRaisesRegex(ValueError, "dev_commands='1'"):
            GENERATOR.parse_tree(source)

        source = report(ROOT_ROW).replace("DEDICATED", "ALL")
        with self.assertRaisesRegex(ValueError, "selection='ALL'"):
            GENERATOR.parse_tree(source)

    def test_parse_tree_rejects_a_walk_that_lost_a_command_root(self) -> None:
        source = report(ROOT_ROW).replace("roots=1", "roots=2")
        with self.assertRaisesRegex(ValueError, "walked 1 command roots"):
            GENERATOR.parse_tree(source)

    def test_parse_tree_rejects_rows_that_contradict_the_walk(self) -> None:
        # The probe prints the path it walked; a path that does not extend the
        # parent's would mean the compact parent column has lost information.
        wrong_path = "2\t1\tsay text\targument\tmessage\t1\t-1\t2\tminecraft:message\t\t"
        with self.assertRaisesRegex(ValueError, "contradicts its parent"):
            GENERATOR.parse_tree(report(ROOT_ROW, wrong_path))

        forward_parent = "1\t2\tsay\tliteral\tsay\t0\t-1\t2\t\t\t"
        with self.assertRaisesRegex(ValueError, "forward parent 2"):
            GENERATOR.parse_tree(report(forward_parent, ARGUMENT_ROW))

        stray_parent = "1\t9\tsay\tliteral\tsay\t0\t-1\t2\t\t\t"
        with self.assertRaisesRegex(ValueError, "outside the reported tree"):
            GENERATOR.parse_tree(report(stray_parent))

        unsatisfiable = "1\t0\tsay\tliteral\tsay\t0\t-1\t-1\t\t\t"
        with self.assertRaisesRegex(ValueError, "wider encoding"):
            GENERATOR.parse_tree(report(unsatisfiable))

        literal_with_parser = "1\t0\tsay\tliteral\tsay\t0\t-1\t2\tminecraft:message\t\t"
        with self.assertRaisesRegex(ValueError, "pairs kind 'literal'"):
            GENERATOR.parse_tree(report(literal_with_parser))

    def test_parse_tree_rejects_a_walk_that_interleaves_siblings(self) -> None:
        # Contiguous runs are the whole point of the first_child/child_count
        # encoding, so an interleaved walk has to fail rather than be repaired.
        rows = (
            ROOT_ROW,
            "2\t1\tsay message\targument\tmessage\t1\t-1\t2\tminecraft:message\t\t",
            "3\t0\ttell\tliteral\ttell\t0\t-1\t0\t\t\t",
        )
        with self.assertRaisesRegex(ValueError, "children of node 0 are not contiguous"):
            GENERATOR.parse_tree(report(*rows))

    def test_registration_order_is_pinned_by_difficulty_children(self) -> None:
        rows = (
            "1\t0\tdifficulty\tliteral\tdifficulty\t1\t-1\t2\t\t\t",
            "2\t1\tdifficulty peaceful\tliteral\tpeaceful\t1\t-1\t0\t\t\t",
            "3\t1\tdifficulty easy\tliteral\teasy\t1\t-1\t0\t\t\t",
            "4\t1\tdifficulty normal\tliteral\tnormal\t1\t-1\t0\t\t\t",
            "5\t1\tdifficulty hard\tliteral\thard\t1\t-1\t0\t\t\t",
        )
        nodes = GENERATOR.parse_tree(report(*rows))
        GENERATOR.validate_registration_order(nodes)

        alphabetized_children = (rows[2], rows[4], rows[3], rows[1])
        alphabetized = (
            rows[0],
            *(
                f"{index}\t{row.split(maxsplit=1)[1]}"
                for index, row in enumerate(alphabetized_children, start=2)
            ),
        )
        with self.assertRaisesRegex(ValueError, "not in Vanilla registration order"):
            GENERATOR.validate_registration_order(
                GENERATOR.parse_tree(report(*alphabetized))
            )

    def test_only_direct_gamerule_children_require_canonical_order(self) -> None:
        sorted_rows = (
            "1\t0\tgamerule\tliteral\tgamerule\t0\t-1\t2\t\t\t",
            "2\t1\tgamerule alpha\tliteral\talpha\t1\t-1\t0\t\t\t",
            "3\t1\tgamerule beta\tliteral\tbeta\t1\t-1\t0\t\t\t",
            "4\t2\tgamerule alpha zeta\tliteral\tzeta\t1\t-1\t0\t\t\t",
            "5\t2\tgamerule alpha aardvark\tliteral\taardvark\t1\t-1\t0\t\t\t",
        )
        GENERATOR.validate_gamerule_order(GENERATOR.parse_tree(report(*sorted_rows)))

        unsorted_rows = (
            sorted_rows[0],
            "2\t1\tgamerule beta\tliteral\tbeta\t1\t-1\t0\t\t\t",
            "3\t1\tgamerule alpha\tliteral\talpha\t1\t-1\t0\t\t\t",
            "4\t3\tgamerule alpha zeta\tliteral\tzeta\t1\t-1\t0\t\t\t",
            "5\t3\tgamerule alpha aardvark\tliteral\taardvark\t1\t-1\t0\t\t\t",
        )
        with self.assertRaisesRegex(ValueError, "not in canonical lexical order"):
            GENERATOR.validate_gamerule_order(
                GENERATOR.parse_tree(report(*unsorted_rows))
            )

    def test_render_rust_keeps_provenance_the_skip_and_one_row_per_node(self) -> None:
        rendered = GENERATOR.render_rust(
            GENERATOR.parse_tree(report(ROOT_ROW, ARGUMENT_ROW))
        )

        self.assertIn("@generated", rendered)
        self.assertIn("CommandSelection.DEDICATED", rendered)
        self.assertIn("Commands.createCompilationContext", rendered)
        # Without the skip, rustfmt explodes every row into a field block and
        # `--check` never matches a formatted tree.
        self.assertEqual(rendered.count("#[rustfmt::skip]"), 1)
        self.assertIn("use super::{CommandNodeType, VanillaCommandNode};", rendered)
        self.assertIn("difficulty_children_keep_vanilla_registration_order", rendered)
        self.assertIn("gamerule_children_keep_canonical_order", rendered)
        self.assertIn(
            'VanillaCommandNode { name: "say", kind: CommandNodeType::Literal, '
            "parent: Some(0), first_child: 2, child_count: 1, redirect: None, "
            "permission: 2, executable: false, parser: None, suggestions: None, "
            "properties: None, },",
            rendered,
        )
        self.assertIn(
            'parser: Some("minecraft:message"), '
            'suggestions: Some("minecraft:ask_server"), '
            'properties: Some("{\\"amount\\":\\"single\\"}"), },',
            rendered,
        )

    def test_render_rust_anchors_each_observed_permission_level_on_a_path(self) -> None:
        rows = (
            ROOT_ROW,
            "2\t0\thelp\tliteral\thelp\t1\t-1\t0\t\t\t",
            "3\t0\tstop\tliteral\tstop\t1\t-1\t4\t\t\t",
        )
        rendered = GENERATOR.render_rust(GENERATOR.parse_tree(report(*rows)))

        self.assertIn('vanilla_command_permission(&["help"]), Some(0)', rendered)
        self.assertIn('vanilla_command_permission(&["say"]), Some(2)', rendered)
        self.assertIn('vanilla_command_permission(&["stop"]), Some(4)', rendered)
        self.assertIn("VANILLA_COMMAND_TREE.len(), 4", rendered)
        self.assertIn("vanilla_command_root_literals().count(), 3", rendered)

    def test_rust_string_escapes_the_json_properties_verbatim(self) -> None:
        self.assertEqual(
            GENERATOR.rust_string('{"registry":"minecraft:entity_type"}'),
            '"{\\"registry\\":\\"minecraft:entity_type\\"}"',
        )
        self.assertEqual(GENERATOR.rust_option_string(None), "None")


if __name__ == "__main__":
    unittest.main()
