# Stop the Linux CI job from picking up old packages restored from its cache

PR #30 (version bump to 0.1.25) failed the Linux job at "Package contents are sane": `ls .../bundle/deb/*.deb` returned two files, `Moraxterm_0.1.24_amd64.deb` and `Moraxterm_0.1.25_amd64.deb`, so `dpkg-deb -c` was given two arguments and failed. Cause: the workflow caches the whole `tauri-ui/src-tauri/target` folder, which includes `target/release/bundle` from the previous version, and Tauri names packages by version. Any version bump would have broken it, and an unbumped build could have checked a stale package instead of the fresh one. The workflow now deletes `target/release/bundle` right before building the packages.

Verified: the failing run's log (run 39211014375) shows the two-path `dpkg-deb` error quoted above; the `build-linux.yml` edit is five lines. Whether the fix works is proven by the Linux job on this pull request and on #30 after it, not by anything run locally.

Open: the same stale-bundle risk would apply to any other workflow that caches `target/`; `build-windows.yml` does not cache it. The `.rpm` has still never been installed.
