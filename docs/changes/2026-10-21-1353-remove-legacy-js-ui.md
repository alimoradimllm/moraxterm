# Remove obsolete legacy JavaScript UI

Remove the unused pre-React `tauri-ui/src/main.js` and `tauri-ui/src/style.css` entrypoint and stylesheet. The application entrypoint is `src/main.tsx`, which imports `react-ui.css`; the deleted files had no active build or runtime references.

Verified: tracked-reference audit found no active import or entrypoint for the deleted files; the React entrypoint remains `tauri-ui/index.html -> src/main.tsx`.

Open: none for this cleanup; legacy Python reference/CLI files remain separate compatibility surfaces.

Docs: README, README.fa_IR.md, AGENTS.md, and docs/REMAINING_WORK.md remove the obsolete legacy-UI references.
