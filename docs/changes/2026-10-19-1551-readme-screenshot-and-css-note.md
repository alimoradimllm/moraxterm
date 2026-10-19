# Add real screenshots to both READMEs and correct the note about the old style.css

Both READMEs now show a real screenshot of the app (`docs/screenshots/workspace-dark.png`, with `workspace-light.png` linked). They were captured from the built exe through its debugging port, using a throwaway profile and demo sessions on `example.test` hosts only, so no real session, host name or path appears. I looked at both images for personal data before adding them.

The remaining-work note said the old `style.css` is "still imported". Checking it: `style.css` is imported only by the legacy `main.js`, which `index.html` does not load; the React UI imports `react-ui.css` alone, and the built CSS contains none of `style.css`'s rules (`mobaxterm-session-editor` and `welcome-mark` occur 0 times in `dist/assets/*.css`). So it is dead weight, not a styling conflict. I corrected the note instead of deleting the file, because AGENTS.md says the legacy JavaScript must stay until feature parity is verified.

Verified: screenshots taken from the exe built from `main` at `3110baf`; each image path in both READMEs resolves to an existing file; the built-CSS grep above. `docs_sync.py generate --verify` passes.

Open: the screenshots show the home screen with the folder tree, not an open terminal, SFTP pane or MultiExec banner. The Persian caption has had no native-speaker review. The exe is the unpacked build, and the screenshot says v0.1.25 because that is the version of the code, not of a published release.
