import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("mcpb_package", ROOT / "scripts/package-mcpb.py")
bundle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bundle)


class McpbBundleTest(unittest.TestCase):
    def test_stages_native_platform_paths_and_three_skills(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            paths = []
            for name in ("mac", "linux", "windows"):
                path = base / name
                path.write_bytes((name + "-synthetic").encode())
                path.chmod(0o755)
                paths.append(path)
            destination = base / "bundle"
            bundle.stage_bundle(*paths, destination)
            manifest = json.loads((destination / "manifest.json").read_text())
            self.assertEqual(manifest["server"]["type"], "binary")
            config = manifest["server"]["mcp_config"]
            mapping = {
                "darwin": config["command"],
                "linux": config["platform_overrides"]["linux"]["command"],
                "win32": config["platform_overrides"]["win32"]["command"],
            }
            for platform, payload in zip(("darwin", "linux", "win32"), ("mac", "linux", "windows")):
                executable = Path(mapping[platform].replace("${__dirname}", str(destination)))
                self.assertEqual(executable.read_bytes(), (payload + "-synthetic").encode())
            self.assertEqual(len(manifest["tools"]), 10)
            for skill in ("resume-project", "record-decision", "prepare-handoff"):
                self.assertTrue((destination / "skills" / skill / "SKILL.md").is_file())
            self.assertTrue((destination / "examples/mcp-config.json").is_file())
            self.assertTrue((destination / "docs/publishing.md").is_file())


if __name__ == "__main__":
    unittest.main()
