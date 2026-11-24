import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from client_extensions import load_extensions, validate_owner


class ClientExtensionsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / "docs").mkdir()

    def write(self, text):
        (self.repo / "docs/client-extensions.md").write_text(text, encoding="utf-8")

    def entry(self, slug="accessibility", status="active", dependencies="-"):
        text = f"""## client-extension/{slug}
- Status: {status}
- Owner: client-experience
- Goal: Improve client accessibility.
- Non-goals: Change Vanilla behavior on the server.
- Dependencies: {dependencies}
- Acceptance: Keyboard-only navigation is verified.
- Enablement: Startup flag --accessibility.
- Server capability: None required.
"""
        if status == "deferred":
            text += "- Reopen when: A keyboard-only use case is selected.\n"
        return text

    def test_missing_empty_and_preamble_only_ledgers(self):
        self.assertEqual(load_extensions(self.repo), {})
        for text in ("", "# Client extensions\n\nNo outstanding extensions.\n## Policy\nRules.\n"):
            with self.subTest(text=text):
                self.write(text)
                self.assertEqual(load_extensions(self.repo), {})

    def test_active_and_deferred_entries_preserve_contract(self):
        self.write("# Client extensions\n\n" + self.entry() + "\n" + self.entry("controller", "deferred", "client-extension/accessibility"))
        entries = load_extensions(self.repo)
        active = entries["client-extension/accessibility"]
        self.assertEqual(active.key, "client-extension/accessibility")
        self.assertEqual(active.owner, "client-experience")
        self.assertEqual(active.status, "active")
        self.assertEqual(active.dependencies, ())
        self.assertEqual(active.reopen_when, "")
        self.assertEqual(active.goal, "Improve client accessibility.")
        self.assertEqual(active.non_goals, "Change Vanilla behavior on the server.")
        self.assertEqual(active.acceptance, "Keyboard-only navigation is verified.")
        self.assertEqual(active.enablement, "Startup flag --accessibility.")
        self.assertEqual(active.server_capability, "None required.")
        deferred = entries["client-extension/controller"]
        self.assertEqual(deferred.dependencies, ("client-extension/accessibility",))
        self.assertTrue(deferred.reopen_when)
        validate_owner(self.repo, "client-experience", active.key, "extension/accessibility")

    def test_invalid_fields_and_duplicate_entries(self):
        valid = self.entry()
        cases = [
            (valid + valid, "duplicate extension ID"),
            (valid + "- Owner: another\n", "duplicate field: Owner"),
            (valid + "- Surprise: value\n", "unknown field: Surprise"),
            (valid.replace("- Goal: Improve client accessibility.\n", ""), "missing fields: Goal"),
            (valid.replace("- Goal: Improve client accessibility.", "- Goal:   "), "empty field: Goal"),
            (valid.replace("Status: active", "Status: completed"), "invalid Status"),
            (valid.replace("Status: active", "Status: deferred"), "requires Reopen when"),
            (valid.replace("\n- Status:", "\n\n- Status:"), "expected '- Field: value'"),
        ]
        for text, message in cases:
            with self.subTest(message=message):
                self.write(text)
                with self.assertRaisesRegex(ValueError, message):
                    load_extensions(self.repo)

    def test_every_required_field_is_required_and_nonempty(self):
        for line in self.entry().splitlines()[1:]:
            field = line.split(":", 1)[0]
            for replacement, message in (("", "missing fields"), (field + ":", "empty field")):
                with self.subTest(field=field, replacement=replacement):
                    self.write(self.entry().replace(line + "\n", replacement + "\n" if replacement else ""))
                    with self.assertRaisesRegex(ValueError, message):
                        load_extensions(self.repo)

    def test_malformed_extension_headings(self):
        for heading in ("## client-extension/Upper", "## client-extension/bad_slug", "## client-extension/", "### client-extension/foo", "## client-extensions/foo", "## client-extension/foo extra", "## client-extension/foo--bar", "##client-extension/foo"):
            with self.subTest(heading=heading):
                self.write(self.entry().replace("## client-extension/accessibility", heading))
                with self.assertRaisesRegex(ValueError, "malformed extension heading"):
                    load_extensions(self.repo)

    def test_invalid_dependencies(self):
        for deps, message in (("client-extension/unknown", "missing dependency"), ("client-extension/accessibility", "self dependency"), ("wrong/id", "invalid dependency ID"), ("client-extension/other,", "invalid dependency ID"), ("client-extension/other, client-extension/other", "duplicate dependency ID")):
            with self.subTest(deps=deps):
                self.write(self.entry(dependencies=deps))
                with self.assertRaisesRegex(ValueError, message):
                    load_extensions(self.repo)
        self.write(self.entry(dependencies="client-extension/controller") + self.entry("controller", dependencies="client-extension/accessibility"))
        with self.assertRaisesRegex(ValueError, "dependency cycle:.*accessibility.*controller.*accessibility"):
            load_extensions(self.repo)

    def test_owner_admission_fails_closed(self):
        self.write(self.entry() + self.entry("controller", "deferred"))
        for stream, capability, coverage, message in (
            ("other", "client-extension/accessibility", "extension/accessibility", "Owner"),
            ("client-experience", "client-extension/controller", "extension/controller", "active"),
            ("client-experience", "client-extension/accessibility", "extension/other", "coverage key"),
            ("client-experience", "client-extension/accessibility", "-", "coverage key"),
            ("client-experience", "client-extension/missing", "extension/missing", "unknown"),
        ):
            with self.subTest(message=message, coverage=coverage):
                with self.assertRaisesRegex(ValueError, message):
                    validate_owner(self.repo, stream, capability, coverage)
        self.write(self.entry(dependencies="client-extension/controller") + self.entry("controller"))
        with self.assertRaisesRegex(ValueError, "remaining dependencies.*controller"):
            validate_owner(self.repo, "client-experience", "client-extension/accessibility", "extension/accessibility")
        (self.repo / "docs/client-extensions.md").unlink()
        with self.assertRaisesRegex(ValueError, "unknown"):
            validate_owner(self.repo, "client-experience", "client-extension/accessibility", "extension/accessibility")


if __name__ == "__main__":
    unittest.main()
