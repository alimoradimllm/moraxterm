# Derive embedded runtime staging from VERSION

What changed and why: `prepare_embedded_runtime.py` now derives the archive name from the repository `VERSION` file instead of retaining a second hardcoded release number. The version regression test now covers the helper's archive name.

Verified: `python -m unittest test_version_consistency.py -q` passes.

Open: the full Tauri build still requires downloading and validating the external embedded runtime archive.
