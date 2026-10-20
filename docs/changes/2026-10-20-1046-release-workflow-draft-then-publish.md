# Release workflows attach assets to a draft; publishing is a separate, checked step

The Windows workflow created and published a release by itself the moment a `v*` tag was pushed, and the Linux workflow never attached its packages to any release. Publishing fires `release-signing.yml`, which signs the checksum of whatever is attached at that moment, so a release published before both platforms had uploaded would carry a signature that misses a platform.

Now a `v*` tag push builds both platforms. The Windows job creates the release as a draft (if it does not exist) and uploads the MSI, NSIS installer and runtime archive; the new Linux `publish-assets` job (runs only for a tag, after the build passed) uploads the `.deb`, `.rpm` and `SHA256SUMS-linux.txt` to the same draft, and refuses to touch a release that is missing or already published or if it does not find exactly one `.deb` and one `.rpm`. Nothing publishes automatically; the release is published by hand after the assets are read back, which then triggers signing over the complete set.

Verified: both workflow files parse as YAML. I probed the draft behaviour on the real repository with a throwaway draft release: `gh release view <tag>` and `gh release upload <tag>` both work on a draft by tag name, and deleting it left no release and no tag behind. The new Linux job itself has not run yet; its first real run is the 0.1.25 tag build.

Open: the Linux job and the Windows publish step are exercised only by the next tag. The runtime archive keeps its `0.1.24` file name on purpose (it is the runtime's version, taken from the existing release asset the repository variable points at), so the 0.1.25 release carries a `…0.1.24.tar.xz`.

Docs: updated docs/REMAINING_WORK.md (release line now describes the draft-then-publish flow).
