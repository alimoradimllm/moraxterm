import { useEffect, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { applyAppTheme, terminalThemeFor } from './theme.js';
import { defaultUiSettings } from './Settings';
import { normalizeTerminalEvent, tauriInvoke, tauriListen } from './react-types';

/** A terminal shown in its own window. The process belongs to the main window's tab; this window only shows it and sends input.
 *  It replays the output held by the native side, then follows live events, dropping any whose byte total the replay already covered.
 *  Closing this window never closes the terminal: the native side tells the main window to show the tab again. */
export default function DetachedTerminal({ id }: { id: string }) {
  const hostRef = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<'attaching' | 'live' | 'exited' | 'failed'>('attaching');
  useEffect(() => {
    if (!hostRef.current) return;
    let settings = defaultUiSettings;
    try { settings = { ...defaultUiSettings, ...JSON.parse(localStorage.getItem('moraxterm-settings') || '{}') }; } catch { /* keep defaults */ }
    applyAppTheme(settings.appTheme);
    const terminal = new Terminal({ convertEol: true, cursorBlink: settings.cursorBlink, fontSize: settings.fontSize, theme: terminalThemeFor(settings.terminalTheme) });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(hostRef.current);
    fit.fit();
    terminal.focus();
    let replayed = -1;                                   // byte total covered by the replay; -1 until it has arrived
    const waiting: { data: string; total: number }[] = [];
    const show = (data: string, total: number) => { if (data && total > replayed) terminal.write(data); };
    const output = tauriListen?.('terminal-output', event => {
      const payload = normalizeTerminalEvent(event);
      if (payload.id !== id || !payload.data) return;
      const total = payload.total ?? 0;
      if (replayed < 0) waiting.push({ data: payload.data, total }); else show(payload.data, total);
    });
    const exit = tauriListen?.('terminal-exit', event => { if (normalizeTerminalEvent(event).id === id) { setState('exited'); terminal.write('\r\n[process exited]\r\n'); } });
    // Listeners are in place before the replay is requested, so no output can fall between the two.
    Promise.all([output, exit]).then(() => tauriInvoke<{ data: string; total: number }>('terminal_replay', { id })).then(replay => {
      if (replay.data) terminal.write(replay.data);
      replayed = replay.total;
      waiting.splice(0).forEach(item => show(item.data, item.total));
      setState(current => current === 'exited' ? current : 'live');
      tauriInvoke?.('resize_terminal', { id, cols: terminal.cols, rows: terminal.rows }).catch(() => {});
    }).catch(() => setState('failed'));
    const data = terminal.onData(value => { tauriInvoke?.('write_terminal', { id, data: value }).catch(() => setState('failed')); });
    const resize = terminal.onResize(({ cols, rows }) => { tauriInvoke?.('resize_terminal', { id, cols, rows }).catch(() => {}); });
    let pending: number | undefined;
    const observer = new ResizeObserver(() => { window.clearTimeout(pending); pending = window.setTimeout(() => fit.fit(), 50); });
    observer.observe(hostRef.current);
    const context = async (event: MouseEvent) => { event.preventDefault(); const text = await navigator.clipboard?.readText(); if (text) tauriInvoke?.('write_terminal', { id, data: text }).catch(() => {}); };
    hostRef.current.addEventListener('contextmenu', context);
    const selection = terminal.onSelectionChange(() => { const text = terminal.getSelection(); if (text) navigator.clipboard?.writeText(text).catch(() => {}); });
    return () => {
      observer.disconnect(); window.clearTimeout(pending); data.dispose(); resize.dispose(); selection.dispose();
      hostRef.current?.removeEventListener('contextmenu', context);
      Promise.resolve(output).then(unlisten => unlisten?.()); Promise.resolve(exit).then(unlisten => unlisten?.());
      terminal.dispose();
    };
  }, [id]);
  return <div className="react-detached">
    <div className="react-detached-bar"><span className={`react-state ${state === 'live' ? 'connected' : state === 'attaching' ? 'starting' : state}`}>{state}</span>
      <button onClick={() => { tauriInvoke?.('attach_terminal', { id }).catch(() => {}); }} title="Close this window and show the terminal in its tab again">Return to tab</button></div>
    <div className="react-detached-host" ref={hostRef} tabIndex={0} />
  </div>;
}
