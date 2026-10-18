# Support VNC and WSL bookmark modes in MobaXterm imports

The supplied `.mxtsessions` export contains VNC bookmark mode `5`, which previously raised an unsupported-mode error, and WSL mode `14`, which the Python importer could misinterpret as a network port. The Python and Rust importers now map mode `5` to `Vnc` and handle mode `14` as a WSL distribution record.

Verified: `python -m unittest test_moraxterm_importer -q` — 5 passed; supplied export parsed read-only as 374 sessions (`SSH=352`, `Rdp=18`, `Vnc=1`, `WSL=3`) with `secret_fields=0`; `cargo test --manifest-path rust-core/Cargo.toml --test bookmarks` — 4 passed; `git diff --check` passed. The supplied file was not copied into the repository and no live connection was attempted.

Open: Installed-app import UI testing and duplicate-name behavior remain open.
