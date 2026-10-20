# Add Ctrl/Cmd+J command palette and focus mode

The React workspace now opens a searchable command palette with Ctrl+J or Cmd+J. Commands include creating a local terminal, opening Settings or Tools, returning home, and toggling focus mode. Focus mode hides the menu, toolbar, rail, and sidebar while keeping terminal panes active.

Verified: 52 frontend unit tests pass, TypeScript typecheck passes, Vite production build passes, and `git diff --check` passes.

Open: installed desktop keyboard/runtime capture remains subject to the existing UI automation limitation.

Docs: no status change: this documents a new React productivity capability and its installed-app runtime gap.
