# Correct Tauri v0.1.26 artifact metadata

Align the Tauri bundle configuration with the frontend and native package manifests so Windows and Linux artifacts are named 0.1.26 instead of inheriting the previous 0.1.25 version.

Verified: the first tagged build exposed the stale Tauri version because its artifacts were named 0.1.25; no release was published. The draft release will be rebuilt after this correction.

Open: tag-triggered Windows and Linux workflows must be rerun and their exact assets verified before publication.

Docs: no status change: this is release metadata correction.
