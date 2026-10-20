# Add a safe local-first AI preview boundary

Settings now expose an opt-in AI assistance boundary with an explicit disabled or local provider choice. The provider interface redacts passwords, tokens, secrets, and private-key blocks and returns preview-only unsupported status when no adapter is installed. The UI never executes generated commands.

Verified: AI helper tests, frontend unit tests, TypeScript typecheck, Vite production build, and diff checks pass.

Open: no local model adapter is bundled; provider configuration remains an explicit unsupported state until an approved runtime is connected.

Docs: no status change: this records the opt-in, preview-only foundation and unsupported provider state.
