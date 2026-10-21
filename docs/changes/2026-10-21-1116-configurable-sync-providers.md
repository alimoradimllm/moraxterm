# Add configurable vault sync provider settings

Settings now exposes a typed provider registry: Local only is ready, while GitHub/GitLab repository, Google Drive, and WebDAV/object-store entries are explicit unavailable contracts. Users can select a conflict policy, and the UI states that no OAuth, network, upload, download, or fake synchronization runs. Existing encrypted vault export/import remains client-side and provider-neutral.

Verified: `npm --prefix tauri-ui run typecheck`; `npm --prefix tauri-ui run test:unit` (51 pass); `npm --prefix tauri-ui run build`; `git diff --check`; `docs_sync.py check --staged`.

Open: provider OAuth/keyring integration, endpoint transports, encrypted envelope upload/download, logout/revoke, conflict interoperability, and E2E team/cloud sync are unavailable until reviewed backends and tests exist. Provider selection is configuration metadata only.

Docs: no status change: this change file records the provider contracts and explicit unavailable states.
