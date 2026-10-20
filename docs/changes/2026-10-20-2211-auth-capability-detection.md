# Report biometric and FIDO2 capability states honestly

Settings now reports whether the host exposes browser WebAuthn primitives, while explicitly showing Windows Hello and FIDO2 SSH as unsupported because no native platform adapter exists. The slice never generates keys, invokes OS prompts, or claims authentication success.

Verified: capability detection tests, frontend unit suite, TypeScript typecheck, Vite build, and diff checks pass.

Open: real Windows Hello, Linux biometric, and FIDO2 SSH support require reviewed native platform APIs and are not implemented.

Docs: no status change: this records detection-only behavior and explicit unsupported states.
