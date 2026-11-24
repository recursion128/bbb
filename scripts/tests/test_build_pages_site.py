"""Unit and repository-wide verification tests for scripts/build_pages_site.py."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS_DIR = REPO_ROOT / "scripts"
if str(SCRIPTS_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPTS_DIR))

import build_pages_site as pages  # noqa: E402


class BuildPagesSiteUnitTests(unittest.TestCase):
    def test_multiline_paragraph_bold_and_cjk_join(self) -> None:
        md = "前文**禁止临时设置\n`RUSTFLAGS='-D warnings'` 或其它覆盖 flag**，以免触发重编译。\n"
        _title, body, _mermaid = pages.render_markdown(md, "README.md", {"README.md"})
        self.assertIn(
            "<strong>禁止临时设置<code>RUSTFLAGS=&#x27;-D warnings&#x27;</code> 或其它覆盖 flag</strong>",
            body,
        )
        self.assertNotIn("**", body)

    def test_table_row_preserves_pipes_inside_code_and_escapes(self) -> None:
        cells = pages.split_table_row(
            r"| `--graphics-preset <fast\|fancy\|fabulous\|custom>` | A \| B | `x|y` |"
        )
        self.assertEqual(
            cells,
            ["`--graphics-preset <fast|fancy|fabulous|custom>`", "A | B", "`x|y`"],
        )

    def test_nested_list_and_indented_code_block(self) -> None:
        md = (
            "- 外层项：\n"
            "  ```sh\n"
            "  cargo dev bootstrap\n"
            "  ```\n"
            "  续行说明。\n"
            "  - 子项一\n"
            "  - 子项二\n"
        )
        _title, body, _mermaid = pages.render_markdown(md, "README.md", {"README.md"})
        self.assertIn("<pre><code class=\"language-sh\">cargo dev bootstrap</code></pre>", body)
        self.assertIn("<ul><li>子项一</li><li>子项二</li></ul>", body)
        self.assertIn("续行说明。", body)
        self.assertLess(body.index("外层项："), body.index("cargo dev bootstrap"))
        self.assertLess(body.index("cargo dev bootstrap"), body.index("续行说明。"))
        self.assertLess(body.index("续行说明。"), body.index("子项一"))

    def test_explicit_html_anchor_is_preserved(self) -> None:
        md = '<a id="custom-anchor"></a>\n\n## 章节\n'
        _title, body, _mermaid = pages.render_markdown(md, "README.md", {"README.md"})
        self.assertIn('<a id="custom-anchor"></a>', body)
        self.assertNotIn("&lt;a id=", body)

    def test_mermaid_block_emits_mermaid_pre_and_flag(self) -> None:
        md = "```mermaid\nflowchart LR\n  A[入口] --> B[出口]\n```\n"
        _title, body, has_mermaid = pages.render_markdown(md, "README.md", {"README.md"})
        self.assertTrue(has_mermaid)
        self.assertIn('<pre class="mermaid">', body)

    def test_directory_link_points_to_github_tree(self) -> None:
        href = pages.rewrite_link("evidence/vanilla-26.1/", "docs/README.md", {"docs/README.md"})
        self.assertEqual(
            href,
            "https://github.com/recursion128/bbb/tree/main/docs/evidence/vanilla-26.1",
        )


class BuildPagesSiteRepositoryTests(unittest.TestCase):
    def test_repository_pages_site_builds_cleanly_with_all_links_clickable(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp) / "pages-site"
            pages.build_site(out_dir, repo_root=REPO_ROOT)

            self.assertTrue((out_dir / ".nojekyll").is_file())
            self.assertTrue((out_dir / "index.html").is_file())
            self.assertTrue((out_dir / "docs" / "README.html").is_file())
            self.assertTrue((out_dir / "docs" / "index.html").is_file())
            self.assertTrue((out_dir / "docs" / "wiki" / "README.html").is_file())
            self.assertTrue((out_dir / "docs" / "wiki" / "index.html").is_file())
            self.assertTrue((out_dir / "AGENTS.html").is_file())
            self.assertTrue((out_dir / "SECURITY.html").is_file())
            self.assertTrue((out_dir / "packaging" / "README.html").is_file())
            self.assertFalse((out_dir / "play").exists())

            errors = pages.verify_site(out_dir)
            self.assertEqual(errors, [], "\n".join(errors))

            # Verify document-inventory table rows are clickable links to rendered pages.
            inv_html = (
                out_dir / "docs" / "wiki" / "development" / "document-inventory.html"
            ).read_text(encoding="utf-8")
            self.assertIn('href="../../../index.html"><code>README.md</code></a>', inv_html)
            self.assertNotIn("WebGPU Client", inv_html)


if __name__ == "__main__":
    unittest.main()
