import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "check-docs.py"


class DocsCheckTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.write("Cargo.toml", '[package]\nname="project-memory-mcp"\nversion="0.2.0"\n')
        self.write("src/server.rs", '#[tool(description="Read")]\nasync fn get_record() {}\n')
        self.write("README.md", "# Test\n[API](docs/api-reference.md)\n")
        self.write("docs/api-reference.md", "# API\n| `get_record` | An ID | Record |\n")
        self.write("server.json", json.dumps({"version":"0.2.0","packages":[{"version":"0.2.0"}]}))
        self.write("apps/studio/package.json", json.dumps({"version":"0.2.0"}))
        self.write("apps/studio/src-tauri/tauri.conf.json", json.dumps({"version":"0.2.0"}))
        self.write("apps/studio/src-tauri/Cargo.toml", '[package]\nversion="0.2.0"\n')

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def check(self):
        self.assertTrue(SCRIPT.is_file(), "Documentation validator must exist")
        spec = importlib.util.spec_from_file_location("check_docs", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module.check_docs(self.root)

    def test_valid_local_links_and_external_links_pass_without_network(self):
        self.write("docs/guide.md", "[API](api-reference.md#tools) [External](https://example.invalid/never-fetch) [Self](#local)")
        self.assertEqual(self.check(), [])

    def test_broken_user_guide_link_fails(self):
        self.write("docs/guide.md", "[Missing](not-present.md)")
        self.assertTrue(any("not-present.md" in error for error in self.check()))

    def test_reference_tool_mismatch_fails(self):
        self.write("docs/api-reference.md", "| `invented_tool` | None | Fiction |")
        self.assertTrue(any("tool inventory" in error.lower() for error in self.check()))

    def test_version_mismatch_fails(self):
        self.write("apps/studio/package.json", '{"version":"0.1.0"}')
        self.assertTrue(any("version" in error.lower() for error in self.check()))

    def test_doc_link_cannot_escape_the_checkout(self):
        self.write("docs/guide.md", "[Outside](../../outside.txt)")
        self.assertTrue(any("outside" in error.lower() for error in self.check()))


if __name__ == "__main__":
    unittest.main()
