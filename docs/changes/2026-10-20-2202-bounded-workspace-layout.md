# Expand split workspace to a bounded sixteen-pane layout

Split mode now accumulates independent panes from the active tab set, uses a responsive grid, persists pane grouping in local storage, and caps the layout at 16 panes. All terminal panes remain mounted while switching views so native process lifecycle behavior is preserved.

Verified: workspace layout unit tests, frontend unit suite, TypeScript typecheck, Vite build, and diff checks pass.

Open: installed-app runtime verification of sixteen simultaneous panes and restart restoration remains open; layout persistence drops tabs that are no longer open.

Docs: no status change: this records the bounded local workspace layout capability and runtime gap.
