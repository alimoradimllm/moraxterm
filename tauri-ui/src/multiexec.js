// Pure helpers for MultiExec so the rules can be tested without a browser.

// Sequences xterm.js emits on its own (answers to terminal queries, focus and mouse reports).
// They describe ONE terminal's state, so mirroring them into other terminals would corrupt those
// shells (for example a cursor-position reply typed into an unrelated prompt). Real keystrokes,
// pasted text, and arrow/function keys are not matched here and are mirrored.
const TERMINAL_REPLY = [
  /^\x1b\[[?>]?[\d;]*[Rcn]$/, // cursor position / device attributes / status replies
  /^\x1b\[[IO]$/, // focus in / focus out (mode 1004)
  /^\x1b\[<[\d;]+[Mm]$/, // SGR mouse reports
  /^\x1b\[M[\s\S]{3}$/, // legacy mouse reports
  /^\x1b[\]P^_][\s\S]*(\x07|\x1b\\)$/, // OSC / DCS / PM / APC replies
];

export function isTerminalReply(data) {
  return TERMINAL_REPLY.some(pattern => pattern.test(data));
}

// Which terminals receive `data` typed into `sourceId`.
// - Outside MultiExec mode, or when the source is excluded, only the source gets it.
// - Otherwise every non-excluded terminal that is still running gets it; the source is always included.
export function mirrorTargets({ on, sourceId, tabs, excluded, data }) {
  if (!on || excluded.has(sourceId) || isTerminalReply(data)) return [sourceId];
  const ids = tabs
    .filter(tab => !excluded.has(tab.id) && tab.state !== 'exited' && tab.state !== 'failed')
    .map(tab => tab.id);
  if (!ids.includes(sourceId)) ids.unshift(sourceId);
  return ids;
}

// Targets for Multi-paste and the panel's Send: every non-excluded terminal that can take input.
export function pasteTargets({ tabs, excluded }) {
  return tabs
    .filter(tab => !excluded.has(tab.id) && tab.state !== 'exited' && tab.state !== 'failed')
    .map(tab => tab.id);
}

export function isMultiPasteShortcut(event) {
  return event.ctrlKey && event.shiftKey && !event.altKey && (event.key === 'Insert' || event.code === 'Insert');
}

// A terminal can be stopped from MultiExec while its process is still alive. Stopping kills only that one process; the other targets keep
// receiving mirrored input and the stopped tab stays open so it can be reconnected.
export function canStop(tab) {
  return tab.state === 'starting' || tab.state === 'connected';
}
