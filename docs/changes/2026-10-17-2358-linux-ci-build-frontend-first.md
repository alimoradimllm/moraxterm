# Build the frontend before the Rust tests in the Linux workflow

The first full run of build-linux.yml on main failed in the Rust tests step: the Tauri crate's generate_context! macro panics with "frontendDist is set to ../dist but this path doesn't exist", because the workflow ran npm ci but never npm run build. The step order is now npm ci, npm run build, then the Rust tests and the package build. This also carries the earlier time limits (60 minute job, 30 minute Rust tests step, one log group per suite) because those commits were left behind when PR 8 merged before they were pushed.

Verified: the failure was read from the real log of run 39065130642 on main: every step up to the frontend unit tests passed on GitHub, then Rust tests failed on the missing ../dist path. The same dist directory exists in my local Debian WSL runs, which is why they passed there. The workflow file parses as YAML.

Open: this fix has not run on GitHub yet, so the Rust tests, the .deb and .rpm build, the check that no EmbeddedUnix archive is packaged, and the clean-container install and start are all still unproven. A separate worktree reported a 135.84 MiB .deb, which is about the size of the 137 MB Windows archive, so the Linux package may be shipping it; the new contents check in this workflow will say.
