# Mirror MultiExec input from detached terminal windows

Detached windows previously wrote directly to their PTY, bypassing the React MultiExec router. Typing or right-click paste in a detached terminal therefore reached only that shell. The new `detached_input` command preserves the direct source write, then emits a tagged event to the main workspace. `App.tsx` routes that event through a live `detachedInput` callback, broadcasting only to the other selected running targets and updating their sent/failed status; the source is not written twice. The harness uses the established per-character CDP typing path so it tests terminal input rather than a focus race.

Verified: `node --check tauri-ui/src/main.js`, TypeScript check, `npm --prefix tauri-ui run build`, `cargo check --manifest-path tauri-ui/src-tauri/Cargo.toml`, `npm run tauri -- build --no-bundle`, and `git diff --check` pass. Runtime verification by the team lead against the rebuilt exe on an isolated profile and port 9363 passes 7/7: three tabs; MultiExec enabled; detached window opened; detached shell executed the marker; the marker appeared in both other main tabs; all three tabs remained present. No process remained after cleanup. An initial 4/7 run used one `Input.insertText` call and failed the basic detached-shell check due to harness focus timing; the final harness uses `App.type()` per character and passes 7/7.

Open: detached windows still do not display MultiExec status or exclusion controls; Stop is not exposed in the detached window; SSH/mixed-protocol detached targets were not runtime-tested; only Windows was tested.

Docs: updated `docs/REMAINING_WORK.md` and the `AGENTS.md` Known open work detached-window entry.
