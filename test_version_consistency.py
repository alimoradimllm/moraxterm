import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_versions", ROOT / "scripts" / "check_versions.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class VersionConsistencyTests(unittest.TestCase):
    def test_release_versions_match(self):
        values = MODULE.versions()
        self.assertEqual(len(set(values.values())), 1, values)
        self.assertEqual(values["VERSION"], "0.1.24")

    def test_embedded_runtime_name_uses_release_version(self):
        self.assertEqual(
            MODULE.versions()["VERSION"],
            MODULE.versions()["tauri-ui/src-tauri/Cargo.toml"],
        )


if __name__ == "__main__":
    unittest.main()
