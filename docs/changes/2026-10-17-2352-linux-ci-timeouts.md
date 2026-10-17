# Add time limits to the Linux package workflow so a hang shows up as a failed step

The first run of build-linux.yml was still on the Rust tests step after about 20 minutes. GitHub shows no log for a running job, so a slow cold build could not be told apart from a hung test. The job now has a 60 minute limit, and the Rust tests step has its own 30 minute limit with a 10 minute cap on the rust-core suite and 20 on the Tauri crate, run as separate log groups so a hang names the suite that caused it.

Verified: the workflow file parses as YAML. The earlier steps of the first run passed on GitHub: system libraries, Node, Rust, cache, npm ci, and the frontend unit tests. The first run was cancelled by hand and produced no verdict on the Rust tests or the package build.

Open: no complete run of build-linux.yml has finished yet, so the package build, the contents check and the clean-container start are all still unproven. The cause of the long wait is unknown: it is either a cold cache compile or a test that hangs on the CI runner.
