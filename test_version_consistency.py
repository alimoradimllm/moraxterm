import importlib.util
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check_versions", ROOT / "scripts" / "check_versions.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
RUNTIME_SPEC = importlib.util.spec_from_file_location("prepare_embedded_runtime", ROOT / "scripts" / "prepare_embedded_runtime.py")
assert RUNTIME_SPEC and RUNTIME_SPEC.loader
RUNTIME = importlib.util.module_from_spec(RUNTIME_SPEC)
RUNTIME_SPEC.loader.exec_module(RUNTIME)


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
        self.assertEqual(
            RUNTIME.ARCHIVE_NAME,
            "Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz",
        )


if __name__ == "__main__":
    unittest.main()
