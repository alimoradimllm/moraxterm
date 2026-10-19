"""Smoke tests for the managed Windows embedded Unix runtime."""

from __future__ import annotations

import os
import subprocess
import unittest
from pathlib import Path


RUNTIME = Path(os.environ.get("LOCALAPPDATA", "")) / "Moraxterm" / "unix-runtime"
FISH = RUNTIME / "usr" / "bin" / "fish.exe"


@unittest.skipUnless(os.name == "nt", "managed runtime path is Windows-specific")
class EmbeddedRuntimeTests(unittest.TestCase):
    def test_baseline_tools(self) -> None:
        if not FISH.is_file():
            self.skipTest(f"managed runtime is not installed: {FISH}")

        env = os.environ.copy()
        env["PATH"] = os.pathsep.join(
            [str(RUNTIME / "usr" / "bin"), str(RUNTIME / "bin"), r"C:\Windows\System32"]
        )
        result = subprocess.run(
            [
                str(FISH),
                "-lc",
                "command -v fish; command -v ping; command -v ssh; command -v telnet; command -v dig; ssh -V; dig localhost",
            ],
            env=env,
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )

        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
        output = result.stdout + result.stderr
        for command in ("fish", "ping", "ssh", "telnet", "dig"):
            self.assertIn(command, output)
        self.assertIn("OpenSSH_", output)
        self.assertTrue("Name:" in output or "Address:" in output, output)


if __name__ == "__main__":
    unittest.main()
