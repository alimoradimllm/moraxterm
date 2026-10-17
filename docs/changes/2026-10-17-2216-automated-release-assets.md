# Automate tagged Windows release assets

The Windows build workflow now creates or updates the matching GitHub Release on tag builds and uploads the MSI, NSIS installer, and validated embedded Unix runtime archive. This removes the manual asset-upload step while keeping `workflow_dispatch` builds artifact-only.

Verified: Python docs-sync tests passed (`16 tests OK`); the release workflow YAML was parsed successfully; the runtime staging tool validated the real `0.1.24` archive and staged 137940124 bytes.

Open: a tagged GitHub Actions run must still be executed to verify release upload permissions and `gh release` behavior in the hosted Windows runner.