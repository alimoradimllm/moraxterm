# Remove the implicit pytest baseline dependency

What changed and why: converted the embedded-runtime smoke test from pytest decorators and assertions to standard-library `unittest`. The documented `python -m unittest discover -q` baseline no longer fails during collection when pytest is absent.

Verified: `python -m unittest discover -q` passes with the platform-specific embedded-runtime test skipped on non-Windows hosts.

Open: the embedded runtime test still requires an installed managed runtime on Windows to execute rather than skip.
