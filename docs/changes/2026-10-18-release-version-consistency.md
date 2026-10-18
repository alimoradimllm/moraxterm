# Centralize release version validation

What changed and why: added a repository `VERSION` source, a validation script, and regression tests so the Python, npm, and Tauri package versions cannot silently diverge. CI now validates versions before Windows and Linux builds, and the Windows release workflow derives the embedded runtime archive name instead of hardcoding an old version.

Verified: `python scripts/check_versions.py` and `python -m unittest test_version_consistency.py` pass.

Open: the release workflow still needs a completed tagged run before published assets can be considered verified.
