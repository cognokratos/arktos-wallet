#!/usr/bin/env python3
"""The documentation checker must actually fail on the drift it exists to catch.

A link checker that passes on everything looks identical to one that works, so
each class of defect is planted in a throwaway tree and must be reported.
"""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import verify_docs  # noqa: E402


class SlugTests(unittest.TestCase):
    def test_github_heading_anchors(self) -> None:
        cases = {
            "C1 — Model cryptographic authority": "c1--model-cryptographic-authority",
            "Why `DATABASE_KEY` is independent from `MASTER_KEY`":
                "why-database_key-is-independent-from-master_key",
            "Challenge 4 — HSM/KMS-backed keys": "challenge-4--hsmkms-backed-keys",
            "14. An already-derived account short-circuits": "14-an-already-derived-account-short-circuits",
            "Performance Targets (NFR-compliant)": "performance-targets-nfr-compliant",
        }
        for heading, slug in cases.items():
            self.assertEqual(verify_docs.slugify(heading), slug, heading)

    def test_duplicate_headings_get_numbered_anchors(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "doc.md"
            path.write_text("## Lab\n\n## Lab\n", encoding="utf-8")
            self.assertEqual(verify_docs.anchors(path), {"lab", "lab-1"})


class CheckFileTests(unittest.TestCase):
    def check(self, files: dict[str, str], targets: set[str] | None = None) -> list[str]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            for name, text in files.items():
                (root / name).parent.mkdir(parents=True, exist_ok=True)
                (root / name).write_text(text, encoding="utf-8")
            return verify_docs.check_file(root / "doc.md", targets or set(), {}, root=root)

    def test_a_clean_document_passes(self) -> None:
        errors = self.check(
            {
                "doc.md": "# Title\n\nSee [other](other.md#a-heading), [self](#title) and `src/keys.rs:12`.\n\n"
                "`make docs-check`\n",
                "other.md": "## A heading\n",
                "src/keys.rs": "",
            },
            targets={"docs-check"},
        )
        self.assertEqual(errors, [])

    def test_a_missing_file_is_reported(self) -> None:
        errors = self.check({"doc.md": "[gone](missing.md)\n"})
        self.assertEqual(len(errors), 1)
        self.assertIn("broken link", errors[0])

    def test_a_missing_anchor_is_reported(self) -> None:
        errors = self.check({"doc.md": "[x](other.md#nope)\n", "other.md": "## Yes\n"})
        self.assertEqual(len(errors), 1)
        self.assertIn("no heading for anchor", errors[0])

    def test_a_link_leaving_the_repository_is_reported(self) -> None:
        errors = self.check({"doc.md": "[x](../../outside.md)\n"})
        self.assertEqual(len(errors), 1)
        self.assertIn("leaves the repository", errors[0])

    def test_a_missing_repository_path_in_a_code_span_is_reported(self) -> None:
        errors = self.check({"doc.md": "See `src/db.rs` and `migrations/V9__nope.sql`.\n"})
        self.assertEqual(len(errors), 2, errors)
        self.assertIn("referenced path does not exist: `src/db.rs`", errors[0])

    def test_non_path_code_spans_are_ignored(self) -> None:
        errors = self.check({"doc.md": "`data/arktos.db`, `wallet_name`, `m/86'/0'/0'/0/0`, `/data`\n"})
        self.assertEqual(errors, [])

    def test_an_unknown_make_target_is_reported_inline_and_in_shell_blocks(self) -> None:
        errors = self.check(
            {"doc.md": "Run `make nope`.\n\n```bash\nmake also-nope   # comment\nexport K=$(make real)\n```\n"},
            targets={"real"},
        )
        self.assertEqual(len(errors), 2, errors)

    def test_make_in_prose_and_non_shell_blocks_is_ignored(self) -> None:
        errors = self.check({"doc.md": "This would make sense.\n\n```rust\n// make things\n```\n"})
        self.assertEqual(errors, [])

    def test_external_links_are_not_fetched(self) -> None:
        errors = self.check({"doc.md": "[x](https://example.invalid/nowhere)\n"})
        self.assertEqual(errors, [])

    def test_headings_inside_code_blocks_are_not_anchors(self) -> None:
        errors = self.check({"doc.md": "[x](#fake)\n\n```text\n# fake\n```\n"})
        self.assertEqual(len(errors), 1)


if __name__ == "__main__":
    unittest.main()
