import React, { Fragment, useEffect, useMemo, useRef, useState } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { colorFor, iconFor, logoSvg, toolColor, toolIconFor } from './icons.js';
import { applyAppTheme, terminalThemeFor } from './theme.js';
import SessionEditor from './SessionEditor';
import SessionTree from './SessionTree';
import { allFolders } from './folders.js';
import { planSave } from './sessionOps.js';
import { addPane, removePane, restoreLayout } from './workspaceLayout.js';
import type { Change } from './sessionOps.js';
import Settings, { defaultUiSettings, UiSettings } from './Settings';
import MacroManager, { Macro } from './MacroManager';
import MultiExecManager from './MultiExecManager';
import { isMultiPasteShortcut, mirrorTargets, pasteTargets } from './multiexec.js';
import TunnelManager from './TunnelManager';
import ToolsPanel from './ToolsPanel';
import SftpPane from './SftpPane';
import { filterPaletteCommands, isPaletteShortcut } from './commandPalette.js';
import {
  fallbackSessions,
  normalizeTerminalEvent,
  Session,
  tauriInvoke,
  tauriListen,
} from './react-types';

interface Tab {
  id: string;
  session: Session;
  state: 'starting' | 'connected' | 'failed' | 'exited';
  error?: string;
  /** True while the terminal is shown in its own window; the tab keeps its xterm mounted but shows a placeholder. */
  detached?: boolean;
}

const remoteTypes = new Set(['SSH', 'TELNET', 'RDP', 'Rdp', 'VNC', 'Vnc', 'WSL', 'Serial', 'Unix', 'Shell']);

type ToolItem = { icon: string; label: string; action: () => void; primary?: boolean; on?: boolean };

function Tile({ icon }: { icon: string }) {
  return <span className="tile" aria-hidden="true" dangerouslySetInnerHTML={{ __html: toolIconFor(icon) }} />;
}

function ToolButton({ item }: { item: ToolItem }) {
  return <button className={`react-tool${item.primary ? ' primary' : ''}${item.on ? ' on' : ''}`} style={{ '--c': toolColor(item.icon) } as React.CSSProperties} onClick={item.action} title={item.label}>
    <Tile icon={item.icon} /><small>{item.label}</small>
  </button>;
}

function Toolbar({ multiExec, split, onNewShell, onHome, onEdit, onSettings, onMultiExec, onSftp, onSplit, onDuplicate, onTools, onView, onTunnel, onHelp, onXserver }: { multiExec: boolean; split: boolean; onNewShell: () => void; onHome: () => void; onEdit: () => void; onSettings: () => void; onMacros: () => void; onMultiExec: () => void; onSftp: () => void; onSplit: () => void; onDuplicate: () => void; onTools: () => void; onView: () => void; onTunnel: () => void; onHelp: () => void; onXserver: () => void }) {
  const groups: ToolItem[][] = [
    [{ icon: 'session', label: 'Session', action: onNewShell, primary: true }, { icon: 'servers', label: 'Servers', action: onHome }, { icon: 'sessions', label: 'Sessions', action: onEdit }],
    [{ icon: 'view', label: 'View', action: onView }, { icon: 'split', label: 'Split', action: onSplit, on: split }, { icon: 'duplicate', label: 'Duplicate', action: onDuplicate }, { icon: 'multiexec', label: 'MultiExec', action: onMultiExec, on: multiExec }],
    [{ icon: 'tools', label: 'Tools', action: onTools }, { icon: 'tunnel', label: 'Tunneling', action: onTunnel }, { icon: 'sftp', label: 'SFTP', action: onSftp }],
    [{ icon: 'settings', label: 'Settings', action: onSettings }, { icon: 'help', label: 'Help', action: onHelp }],
  ];
  return <div className="react-toolbar">
    {groups.map((group, index) => <Fragment key={index}>{index > 0 && <span className="react-tool-sep" />}{group.map(item => <ToolButton key={item.label} item={item} />)}</Fragment>)}
    <span className="react-toolbar-spacer" />
    <ToolButton item={{ icon: 'xserver', label: 'X server', action: onXserver }} />
    <ToolButton item={{ icon: 'exit', label: 'Exit', action: () => window.close() }} />
  </div>;
}

function ProtocolIcon({ type }: { type: string }) {
  return <span className="react-protocol-icon" style={{ '--c': colorFor(type) } as React.CSSProperties} dangerouslySetInnerHTML={{ __html: iconFor(type) }} />;
}

function MiniIcon({ name }: { name: string }) {
  return <span className="mini" style={{ display: 'inline-block', width: 18, height: 18 }} aria-hidden="true" dangerouslySetInnerHTML={{ __html: toolIconFor(name) }} />;
}

const quickProtocols = ['SSH', 'Unix', 'WSL', 'Shell', 'Telnet', 'Rdp', 'Vnc', 'Serial'] as const;
const defaultPorts: Record<string, number> = { SSH: 22, Telnet: 23, Rdp: 3389, Vnc: 5900 };

function SessionSidebar({ sessions, folders, activeId, onOpen, onHome, onEdit, onTools, onMacros, onApply }: {
  sessions: Session[]; folders: string[]; activeId?: string; onOpen: (session: Session) => void; onHome: () => void; onEdit: (session: Session) => void; onTools: () => void; onMacros: () => void; onApply: (result: Change, message?: string) => unknown;
}) {
  const [query, setQuery] = useState('');
  return <aside className="react-sidebar">
    <div className="react-side-tabs"><button className="selected" onClick={onHome}>Sessions</button><button onClick={onTools}>Tools</button><button onClick={onMacros}>Macros</button></div>
    <div className="react-side-search"><input aria-label="Quick find sessions" placeholder="Quick find sessions" value={query} onChange={event => setQuery(event.target.value)} /></div>
    <SessionTree sessions={sessions} folders={folders} query={query} activeName={activeId} onOpen={onOpen} onEdit={onEdit} onRefresh={onHome} onApply={onApply}
      onEditActive={() => { const selected = sessions.find(session => session.name === activeId); if (selected) onEdit(selected); }} />
    <div className="react-side-footer"><span><MiniIcon name="monitor" /> Remote monitoring</span><label><input type="checkbox" defaultChecked /> Follow terminal folder</label></div>
  </aside>;
}

function TerminalPane({ tab, settings, multiExec, excluded, execStatus, onToggleExclude, onInput, onState, onError, onClose, onDetach, onAttach }: { tab: Tab; settings: UiSettings; multiExec: boolean; excluded: boolean; execStatus?: 'sent' | 'failed'; onToggleExclude: () => void; onInput: (id: string, data: string) => void; onState: (state: Tab['state']) => void; onError: (error: string) => void; onClose: () => void; onDetach: () => void; onAttach: () => void }) {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  // While the terminal is in its own window that window decides the PTY size, so this pane must not resize it.
  const detachedRef = useRef(false);
  detachedRef.current = Boolean(tab.detached);
  const syncRef = useRef<() => void>(() => {});
  useEffect(() => {
    if (!hostRef.current) return;
    const terminal = new Terminal({
      convertEol: true,
      cursorBlink: settings.cursorBlink,
      fontSize: settings.fontSize,
      theme: terminalThemeFor(settings.terminalTheme),
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(hostRef.current);
    fit.fit();
    terminal.focus();
    terminalRef.current = terminal;
    const id = tab.id;
    const unlistenPromise = tauriListen?.('terminal-output', event => {
      const payload = normalizeTerminalEvent(event);
      if (payload.id === id && payload.data) { onState('connected'); terminal.write(payload.data); }
    });
    const exitPromise = tauriListen?.('terminal-exit', event => {
      const payload = normalizeTerminalEvent(event);
      if (payload.id === id) onState('exited');
    });
    const resize = terminal.onResize(({ cols, rows }) => { tauriInvoke?.('resize_terminal', { id, cols, rows }).catch(() => {}); });
    const data = terminal.onData(value => onInput(id, value));
    const selection = terminal.onSelectionChange(() => {
      const text = terminal.getSelection();
      if (text) navigator.clipboard?.writeText(text).catch(() => {});
    });
    const context = async (event: MouseEvent) => {
      event.preventDefault();
      const text = await navigator.clipboard?.readText();
      if (text) onInput(id, text);
    };
    hostRef.current.addEventListener('contextmenu', context);
    onState('starting');
    const session = remoteTypes.has(tab.session.session_type) ? tab.session : null;
    // The first fit() runs before the native process exists, so its resize is dropped. Re-send the real size once
    // the process has started, and keep the PTY in sync whenever the pane is resized (window, split, sidebar).
    const syncSize = () => { fit.fit(); if (detachedRef.current) return; tauriInvoke?.('resize_terminal', { id, cols: terminal.cols, rows: terminal.rows }).catch(() => {}); };
    syncRef.current = syncSize;
    let pending: number | undefined;
    const observer = new ResizeObserver(() => { window.clearTimeout(pending); pending = window.setTimeout(syncSize, 50); });
    observer.observe(hostRef.current);
    tauriInvoke?.('start_terminal', { id, session, program: null }).then(() => { syncSize(); onState('connected'); }).catch(error => { onError(error instanceof Error ? error.message : String(error)); onState('failed'); });
    return () => {
      observer.disconnect(); window.clearTimeout(pending);
      resize.dispose(); data.dispose(); selection.dispose();
      hostRef.current?.removeEventListener('contextmenu', context);
      Promise.resolve(unlistenPromise).then(unlisten => unlisten?.());
      Promise.resolve(exitPromise).then(unlisten => unlisten?.());
      tauriInvoke?.('close_terminal', { id }).catch(() => {});
      terminal.dispose();
    };
  }, []);
  useEffect(() => { if (terminalRef.current) terminalRef.current.options.theme = terminalThemeFor(settings.terminalTheme); }, [settings.terminalTheme]);
  // Back in the tab: take the PTY size from this pane again.
  useEffect(() => { if (!tab.detached) syncRef.current(); }, [tab.detached]);
  const detach = async () => {
    try { await tauriInvoke?.('detach_terminal', { id: tab.id, title: tab.session.name }); onDetach(); }
    catch (error) { onError(error instanceof Error ? error.message : String(error)); }
  };
  const reconnect = async () => {
    onState('starting');
    try {
      await tauriInvoke?.('close_terminal', { id: tab.id });
      await tauriInvoke?.('start_terminal', { id: tab.id, session: remoteTypes.has(tab.session.session_type) ? tab.session : null, program: null });
      onState('connected');
    } catch (error) { onError(error instanceof Error ? error.message : String(error)); onState('failed'); }
  };
  const resume = async () => {
    try { await tauriInvoke?.('resume_terminal', { id: tab.id }); onState('connected'); } catch { onState('failed'); }
  };
  return <div className="react-terminal-pane">
    <div className="react-pane-bar"><span><ProtocolIcon type={tab.session.session_type} /> {tab.session.name}</span><span className="react-terminal-actions">{multiExec && <label className="react-exclude"><input type="checkbox" checked={excluded} onChange={onToggleExclude} /> Exclude {tab.session.name} from MultiExec mode</label>}{multiExec && !excluded && execStatus && <span className={`react-exec-status ${execStatus}`}>{execStatus === 'sent' ? '✓ sent' : '✕ failed'}</span>}<span className={`react-state ${tab.state}`}>{tab.state}</span><button onClick={resume} disabled={tab.state === 'starting'} title="Resume terminal process">Resume</button><button onClick={reconnect} title="Reconnect terminal">Reconnect</button><button onClick={tab.detached ? onAttach : detach} disabled={tab.state === 'starting' || tab.state === 'failed'} title={tab.detached ? 'Close the separate window and show the terminal here again' : 'Open this terminal in its own window'}>{tab.detached ? 'Return' : 'Detach'}</button><button onClick={onClose} title="Close terminal">Close</button></span></div>
    {tab.error && <div className="react-terminal-error" role="alert">{tab.error}</div>}
    <div className="react-terminal-body">
      <div className="react-terminal-host" style={{ background: terminalThemeFor(settings.terminalTheme).background }} ref={hostRef} tabIndex={0} onPointerDown={() => terminalRef.current?.focus()} onKeyDown={() => terminalRef.current?.focus()} />
      {tab.detached && <div className="react-detached-note" role="status"><strong>{tab.session.name}</strong> is open in a separate window.<button onClick={onAttach}>Return to this tab</button></div>}
    </div>
  </div>;
}

export default function App() {
  const [sessions, setSessions] = useState<Session[]>(fallbackSessions);
  const [folders, setFolders] = useState<string[]>(() => { try { const value = JSON.parse(localStorage.getItem('moraxterm-session-folders') || '[]'); return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []; } catch { return []; } });
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeId, setActiveId] = useState<string>();
  const [splitIds, setSplitIds] = useState<string[]>(() => { try { return restoreLayout(JSON.parse(localStorage.getItem('moraxterm-workspace-layout') || '[]'), []); } catch { return []; } });
  const [editorSession, setEditorSession] = useState<Session>();
  const [panel, setPanel] = useState<'settings' | 'macros' | 'multiexec' | 'sftp' | 'tunnel' | 'tools'>();
  const [notice, setNotice] = useState<string>();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [focusMode, setFocusMode] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState('');
  const [uiSettings, setUiSettings] = useState<UiSettings>(() => { try { return { ...defaultUiSettings, ...JSON.parse(localStorage.getItem('moraxterm-settings') || '{}') }; } catch { return defaultUiSettings; } });
  const [quick, setQuick] = useState('');
  const [multiExec, setMultiExec] = useState(false);
  const [excluded, setExcluded] = useState<Set<string>>(new Set());
  const [execStatus, setExecStatus] = useState<Record<string, 'sent' | 'failed'>>({});
  const latest = useRef<{ sendInput: (id: string, data: string) => void; multiPaste: () => void }>({ sendInput: () => {}, multiPaste: () => {} });
  useEffect(() => { applyAppTheme(uiSettings.appTheme); }, [uiSettings.appTheme]);
  useEffect(() => {
    tauriInvoke?.<Session[]>('list_sessions').then(value => { if (Array.isArray(value) && value.length) setSessions(value); }).catch(() => {});
  }, []);
  const openSession = (session: Session) => {
    // A panel or the session editor would otherwise stay on top of the new tab, so the click looks like a no-op.
    setPanel(undefined); setEditorSession(undefined); setNotice(undefined);
    const existing = tabs.find(tab => tab.session.name === session.name && tab.session.session_type === session.session_type);
    if (existing) { setActiveId(existing.id); return; }
    const tab = { id: `tab-${Date.now()}-${tabs.length}`, session, state: 'starting' as const };
    setTabs(current => [...current, tab]); setActiveId(tab.id);
  };
  const closeTab = (id: string) => {
    tauriInvoke?.('close_terminal', { id }).catch(() => {});
    setTabs(current => {
      const remaining = current.filter(item => item.id !== id);
      setSplitIds(current => { const next = removePane(current, id); localStorage.setItem('moraxterm-workspace-layout', JSON.stringify(next)); return next; });
      setActiveId(activeId === id ? remaining[remaining.length - 1]?.id : activeId);
      return remaining;
    });
  };
  const setDetached = (id: string, detached: boolean) => setTabs(current => current.map(item => item.id === id && Boolean(item.detached) !== detached ? { ...item, detached } : item));
  // The native side announces when a terminal's separate window is gone (closed by the user or returned), so the tab can show it again.
  useEffect(() => {
    const unlisten = tauriListen?.('terminal-attached', event => { const id = (event as { payload?: unknown }).payload; if (typeof id === 'string') setDetached(id, false); });
    return () => { Promise.resolve(unlisten).then(fn => fn?.()); };
  }, []);
  const splitWorkspace = () => {
    if (!activeId) return;
    const next = tabs.find(tab => tab.id !== activeId && !splitIds.includes(tab.id));
    if (next) { const layout = addPane(splitIds, next.id); setSplitIds(layout); localStorage.setItem('moraxterm-workspace-layout', JSON.stringify(layout)); }
    else if (splitIds.length) { setSplitIds([]); localStorage.setItem('moraxterm-workspace-layout', '[]'); }
  };
  const openQuick = () => {
    const host = quick.trim();
    if (host) openSession({ name: host, host, session_type: 'SSH', port: 22 });
  };
  const newSessionOf = (type: string) => setEditorSession({ name: `New ${type} session`, session_type: type, host: '', username: '', port: defaultPorts[type] ?? 0, ...(type === 'Unix' ? { unix_shell: 'fish' } : {}) });
  // Saving goes through planSave: a rename becomes "save the new name, delete the old record" (the store keys sessions by name),
  // and a name that belongs to another session is refused instead of silently overwriting it.
  const saveSession = async (draft: Session) => {
    const plan = planSave(sessions, editorSession, draft);
    if (!plan.ok) { setNotice(plan.error); return; }
    const message = plan.renamed ? `Renamed ${plan.renamed.from} to ${plan.renamed.to}.` : `Saved ${plan.changed?.[0]?.name ?? 'session'}.`;
    if (await applyChange(plan, message)) {
      setEditorSession(undefined);
      // Only the label changes for a terminal that is already running; its other settings apply the next time it starts.
      const renamed = plan.renamed;
      if (renamed) setTabs(current => current.map(tab => tab.session.name === renamed.from ? { ...tab, session: { ...tab.session, name: renamed.to } } : tab));
    }
  };
  const saveFolders = (next: string[]) => { setFolders(next); localStorage.setItem('moraxterm-session-folders', JSON.stringify(next)); };
  // Every sidebar action (folders, drag and drop, save, rename, duplicate, favourite, delete) arrives here as a computed change.
  // Sessions are stored by the Rust side, so changed sessions are saved first and removed names deleted after; if anything fails the
  // whole change is undone in the interface and the user is told. Returns true when the change was applied.
  const applyChange = async (result: Change, message?: string): Promise<boolean> => {
    if (!result.ok) { if (!result.noop) setNotice(result.error || 'That change is not possible.'); return false; }
    const previousSessions = sessions; const previousFolders = folders;
    if (result.sessions) setSessions(result.sessions);
    if (result.folders) saveFolders(result.folders);
    try {
      await Promise.all((result.changed ?? []).map(session => tauriInvoke('save_session', { session })));
      for (const name of result.deleted ?? []) await tauriInvoke('delete_session', { name });
    } catch (error) {
      setSessions(previousSessions); saveFolders(previousFolders);
      setNotice(`The change was not saved and has been undone: ${error instanceof Error ? error.message : String(error)}`);
      return false;
    }
    if (message) setNotice(message);
    return true;
  };
  const saveSettings = (next: UiSettings) => { setUiSettings(next); localStorage.setItem('moraxterm-settings', JSON.stringify(next)); setPanel(undefined); };
  const playMacro = (macro: Macro, terminalId: string) => { macro.events.reduce((chain, data) => chain.then(() => tauriInvoke?.('write_terminal', { id: terminalId, data })), Promise.resolve()).catch(() => {}); };
  const broadcast = async (ids: string[], data: string): Promise<Record<string, 'sent' | 'failed'>> => {
    const results = await Promise.all(ids.map(async id => { try { await tauriInvoke?.('write_terminal', { id, data }); return [id, 'sent'] as const; } catch { return [id, 'failed'] as const; } }));
    return Object.fromEntries(results) as Record<string, 'sent' | 'failed'>;
  };
  const mergeStatus = (result: Record<string, 'sent' | 'failed'>) => setExecStatus(current => {
    const keys = Object.keys(result);
    return keys.every(key => current[key] === result[key]) ? current : { ...current, ...result };
  });
  // Every keystroke and paste from any terminal goes through here so MultiExec can mirror it.
  const sendInput = async (sourceId: string, data: string) => {
    const ids = mirrorTargets({ on: multiExec, sourceId, tabs, excluded, data });
    const result = await broadcast(ids, data);
    if (multiExec) mergeStatus(result);
    if (result[sourceId] === 'failed') setTabs(current => current.map(item => item.id === sourceId ? { ...item, state: 'failed' } : item));
  };
  const multiPaste = async () => {
    const text = await navigator.clipboard?.readText().catch(() => '');
    const ids = pasteTargets({ tabs, excluded });
    if (!text || !ids.length) { showNotice(text ? 'No MultiExec target can receive input.' : 'Clipboard is empty or unavailable.'); return; }
    mergeStatus(await broadcast(ids, text));
  };
  latest.current = { sendInput, multiPaste };
  const toggleExclude = (id: string) => setExcluded(current => { const next = new Set(current); if (next.has(id)) next.delete(id); else next.add(id); return next; });
  const openMultiExec = () => { setMultiExec(true); setNotice(undefined); setPanel('multiexec'); };
  const exitMultiExec = () => { setMultiExec(false); setExecStatus({}); setPanel(current => current === 'multiexec' ? undefined : current); };
  useEffect(() => {
    if (!multiExec) return;
    const onKey = (event: KeyboardEvent) => { if (isMultiPasteShortcut(event)) { event.preventDefault(); event.stopPropagation(); latest.current.multiPaste(); } };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [multiExec]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isPaletteShortcut(event)) { event.preventDefault(); setPaletteOpen(true); setPaletteQuery(''); }
      if (event.key === 'Escape' && paletteOpen) { event.preventDefault(); setPaletteOpen(false); }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [paletteOpen]);
  const active = tabs.find(tab => tab.id === activeId);
  const split = tabs.find(tab => splitIds.includes(tab.id) && tab.id !== activeId);
  const duplicateActive = () => {
    if (!active) return;
    const base = `${active.session.name} (copy)`;
    let name = base;
    let suffix = 2;
    while (tabs.some(tab => tab.session.name === name && tab.session.session_type === active.session.session_type)) name = `${base} ${suffix++}`;
    const copy: Tab = { id: `tab-${Date.now()}-${tabs.length}`, session: { ...active.session, name }, state: 'starting' };
    setTabs(current => [...current, copy]);
    setActiveId(copy.id);
  };
  const showNotice = (message: string) => { setNotice(message); setEditorSession(undefined); };
  const openTools = () => { setNotice(undefined); setPanel('tools'); };
  const openView = () => { if (active) splitWorkspace(); else showNotice('Open a terminal before using View or Split.'); };
  const openTunnel = () => { setNotice(undefined); setPanel('tunnel'); };
  const openHelp = () => showNotice('Shortcuts: Enter executes terminal input, R reconnects, S resumes, and the toolbar opens Sessions, Split, Duplicate, MultiExec, Packages, and Settings.');
  const openXserver = async () => {
    try { showNotice(await tauriInvoke<string>('xserver_status')); }
    catch (error) { showNotice(`X server status failed: ${error instanceof Error ? error.message : String(error)}`); }
  };
  const overlayOpen = Boolean(editorSession || panel || !tabs.length || !active);
  const commands = filterPaletteCommands([
    { label: 'New local terminal', run: () => openSession({ name: 'Local Shell', session_type: 'Unix', host: '', username: '', port: 0, unix_shell: 'fish' }) },
    { label: 'Open settings', run: () => setPanel('settings') }, { label: 'Open tools', run: openTools }, { label: 'Toggle focus mode', run: () => setFocusMode(value => !value) }, { label: 'Go home', run: () => { setActiveId(undefined); setPanel(undefined); } },
  ], paletteQuery);
  return <div className={`react-app-shell${focusMode ? ' focus-mode' : ''}`}>
    {!focusMode && <><div className="react-menu-row"><span className="react-brand"><span className="logo" aria-hidden="true" dangerouslySetInnerHTML={{ __html: logoSvg }} />Moraxterm</span><button onClick={() => showNotice('Use Session or Start local terminal to open a terminal.')}>Terminal</button><button onClick={() => { setTabs([]); setPanel(undefined); }}>Sessions</button><button onClick={openView}>View</button><button onClick={openXserver}>X server</button><button onClick={openTools}>Tools</button><button onClick={() => setPanel('settings')}>Settings</button><button onClick={() => setPanel('macros')}>Macros</button><button onClick={openHelp}>Help</button></div>
    <Toolbar multiExec={multiExec} split={Boolean(split)} onNewShell={() => openSession({ name: 'Local Shell', session_type: 'Unix', host: '', username: '', port: 0, unix_shell: 'fish' })} onHome={() => { setActiveId(undefined); setEditorSession(undefined); setPanel(undefined); }} onEdit={() => { if (active) setEditorSession(active.session); else if (sessions[0]) setEditorSession(sessions[0]); }} onSettings={() => setPanel('settings')} onMacros={() => setPanel('macros')} onMultiExec={openMultiExec} onSftp={() => setPanel('sftp')} onSplit={splitWorkspace} onDuplicate={duplicateActive} onTools={openTools} onView={openView} onTunnel={openTunnel} onHelp={openHelp} onXserver={openXserver} /></>}
    {paletteOpen && <div className="react-command-palette" role="dialog" aria-label="Command palette"><input autoFocus placeholder="Type a command..." value={paletteQuery} onChange={event => setPaletteQuery(event.target.value)} onKeyDown={event => { if (event.key === 'Escape') setPaletteOpen(false); if (event.key === 'Enter' && commands[0]) { commands[0].run(); setPaletteOpen(false); } }} />{commands.map(command => <button key={command.label} onClick={() => { command.run(); setPaletteOpen(false); }}>{command.label}</button>)}</div>}
    {notice && <div className="react-action-notice" role="status">{notice}<button aria-label="Dismiss notice" onClick={() => setNotice(undefined)}>×</button></div>}
    <main className="react-main"><nav className="react-rail"><button className={!active ? 'active' : ''} style={{ '--c': toolColor('home') } as React.CSSProperties} onClick={() => { setActiveId(undefined); setPanel(undefined); }} title="Home"><Tile icon="home" /></button><button style={{ '--c': toolColor('tools') } as React.CSSProperties} onClick={openTools} title="Tools"><Tile icon="tools" /></button><button style={{ '--c': toolColor('tunnel') } as React.CSSProperties} onClick={openTunnel} title="Tunneling"><Tile icon="tunnel" /></button><button style={{ '--c': toolColor('view') } as React.CSSProperties} onClick={openView} title="View"><Tile icon="view" /></button></nav><SessionSidebar sessions={sessions} folders={folders} activeId={active?.session.name} onOpen={openSession} onHome={() => setActiveId(undefined)} onEdit={setEditorSession} onTools={openTools} onMacros={() => setPanel('macros')} onApply={applyChange} />
      <section className="react-workspace">
        {multiExec && <div className="react-multiexec-active" role="status">
          <strong>MultiExec mode is active:</strong> input typed in any terminal is sent to every other terminal that is not excluded.
          <span className="react-multiexec-count">{pasteTargets({ tabs, excluded }).length} target(s)</span>
          <span className="react-multiexec-actions"><button onClick={() => latest.current.multiPaste()} title="Ctrl+Shift+Insert">Multi-paste</button><button onClick={() => setPanel('multiexec')}>Targets</button><button className="danger" onClick={exitMultiExec}>Exit multi-execution mode</button></span>
        </div>}
        {overlayOpen ? (editorSession ? <SessionEditor session={editorSession} folders={allFolders(sessions, folders)} onSave={saveSession} onCancel={() => setEditorSession(undefined)} /> : panel === 'settings' ? <Settings value={uiSettings} onSave={saveSettings} onCancel={() => setPanel(undefined)} /> : panel === 'macros' ? <MacroManager activeTerminalId={active?.id} onPlay={playMacro} onClose={() => setPanel(undefined)} /> : panel === 'multiexec' ? <MultiExecManager targets={tabs.map(tab => ({ id: tab.id, name: tab.session.name, type: tab.session.session_type, state: tab.state }))} excluded={excluded} status={execStatus} onToggle={toggleExclude} onSetExcluded={setExcluded} onSend={(data) => { const ids = pasteTargets({ tabs, excluded }); broadcast(ids, data).then(mergeStatus); }} onExit={exitMultiExec} onClose={() => setPanel(undefined)} /> : panel === 'tunnel' ? <TunnelManager onClose={() => setPanel(undefined)} /> : panel === 'tools' ? <ToolsPanel onClose={() => setPanel(undefined)} /> : panel === 'sftp' ? <SftpPane session={active?.session || sessions.find(item => item.session_type === 'SSH')} onClose={() => setPanel(undefined)} /> : !tabs.length || !active ? <div className="react-home"><div className="react-home-mark" aria-hidden="true" dangerouslySetInnerHTML={{ __html: logoSvg }} /><h1>Moraxterm</h1><p>Your terminal workspace</p><div className="react-home-actions"><button className="primary" onClick={() => openSession({ name: 'Local Shell', session_type: 'Unix', host: '', username: '', port: 0, unix_shell: 'fish' })}>＋ Start local terminal</button><button onClick={() => setActiveId(undefined)}>Recover previous sessions</button></div><input aria-label="Find existing session or server name" placeholder="Find existing session or server name..." /><h2>New session</h2><div className="react-protocol-tiles">{quickProtocols.map(type => <button key={type} className="react-protocol-tile" style={{ '--c': colorFor(type) } as React.CSSProperties} onClick={() => newSessionOf(type)} title={`New ${type} session`}><span className="glyph" aria-hidden="true" dangerouslySetInnerHTML={{ __html: iconFor(type) }} /><span className="name">{type === 'Rdp' ? 'RDP' : type === 'Vnc' ? 'VNC' : type}</span></button>)}</div><h2>Recent sessions</h2><div className="react-recent">{sessions.slice(0, 6).map(session => <button key={session.name} style={{ '--c': colorFor(session.session_type) } as React.CSSProperties} onClick={() => openSession(session)}><ProtocolIcon type={session.session_type} /><span>{session.name}<small>{session.host || session.session_type}</small></span></button>)}</div></div> : null) : null}
        {tabs.length > 0 && <div className="react-terminals" style={{ display: overlayOpen ? 'none' : 'flex' }}>
          <nav className="react-tabs">{tabs.map(tab => <button key={tab.id} style={{ '--c': colorFor(tab.session.session_type) } as React.CSSProperties} className={tab.id === activeId ? 'active' : ''} onClick={() => setActiveId(tab.id)}><ProtocolIcon type={tab.session.session_type} />{tab.session.name}<span className={`dot ${tab.state}`} /><b onClick={event => { event.stopPropagation(); closeTab(tab.id); }}>×</b></button>)}</nav>
          <div className={split ? 'react-panes react-panes-split' : 'react-panes'}>
            {tabs.map(tab => {
              const shown = !!active && (tab.id === active.id || splitIds.includes(tab.id));
              // Every tab stays mounted: unmounting a TerminalPane closes its native process, so hiding is the only safe way to switch tabs or open a panel.
              return <div key={tab.id} className="react-pane-slot" style={{ display: shown ? 'flex' : 'none', order: splitIds.indexOf(tab.id) + 1 }}>
                <TerminalPane tab={tab} settings={uiSettings} multiExec={multiExec} excluded={excluded.has(tab.id)} execStatus={execStatus[tab.id]} onToggleExclude={() => toggleExclude(tab.id)} onInput={(id, data) => latest.current.sendInput(id, data)} onClose={() => closeTab(tab.id)} onDetach={() => setDetached(tab.id, true)} onAttach={() => { tauriInvoke?.('attach_terminal', { id: tab.id }).catch(() => {}); setDetached(tab.id, false); }} onError={error => setTabs(current => current.map(item => item.id === tab.id ? { ...item, error } : item))} onState={state => setTabs(current => current.map(item => item.id === tab.id ? { ...item, state, ...(state === 'starting' ? { error: undefined } : {}) } : item))} />
              </div>;
            })}
          </div>
        </div>}
        <footer className="react-status"><span>● Ready</span><span>CPU —</span><span>RAM —</span><span>Network —</span><span>Desktop</span></footer>
      </section>
    </main>
  </div>;
}
