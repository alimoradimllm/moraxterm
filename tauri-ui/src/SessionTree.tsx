import React, { useEffect, useMemo, useRef, useState } from 'react';
import { colorFor, iconFor, toolColor, toolIconFor } from './icons.js';
import {
  allFolders, buildRows, createFolder, deleteFolder, isInside, leafOf, moveFolder, moveSession,
  normalizePath, parentOf, renameFolder, sessionKey,
} from './folders.js';
import type { FolderChange } from './folders.js';
import type { Session } from './react-types';

type Drag = { type: 'session' | 'folder'; id: string };
type Editing = { mode: 'new'; parent: string } | { mode: 'rename'; path: string } | null;
type MenuTarget = { kind: 'folder'; path: string } | { kind: 'session'; session: Session };
type Menu = (MenuTarget & { x: number; y: number }) | null;

const COLLAPSED_KEY = 'moraxterm-collapsed-folders';
const readCollapsed = (): Set<string> => {
  try { const value = JSON.parse(localStorage.getItem(COLLAPSED_KEY) || '[]'); return new Set(Array.isArray(value) ? value.filter(item => typeof item === 'string') : []); } catch { return new Set(); }
};

function Icon({ name }: { name: string }) {
  return <span className="mini" aria-hidden="true" dangerouslySetInnerHTML={{ __html: toolIconFor(name) }} />;
}

function NameInput({ initial, depth, label, onSubmit, onCancel }: { initial: string; depth: number; label: string; onSubmit: (value: string) => string; onCancel: () => void }) {
  const [value, setValue] = useState(initial);
  const [error, setError] = useState('');
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { ref.current?.focus(); ref.current?.select(); }, []);
  return <div className="react-tree-edit" style={{ paddingLeft: 8 + depth * 16 }}>
    <span className="react-tool-icon-wrap" style={{ '--c': toolColor('folder') } as React.CSSProperties}><Icon name="folder" /></span>
    <input ref={ref} aria-label={label} value={value} placeholder="Folder name" maxLength={64} className={error ? 'bad' : ''}
      onChange={event => { setValue(event.target.value); setError(''); }}
      onKeyDown={event => { if (event.key === 'Enter') { event.preventDefault(); setError(onSubmit(value)); } else if (event.key === 'Escape') { event.preventDefault(); onCancel(); } }}
      onBlur={() => { if (!error) onCancel(); }} />
    {error && <small role="alert">{error}</small>}
  </div>;
}

export default function SessionTree({ sessions, folders, query, activeName, onOpen, onEditActive, onRefresh, onApply, onEdit }: {
  sessions: Session[]; folders: string[]; query: string; activeName?: string;
  onOpen: (session: Session) => void; onEditActive: () => void; onRefresh: () => void; onEdit: (session: Session) => void;
  onApply: (result: FolderChange, message?: string) => void;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(readCollapsed);
  const [selected, setSelected] = useState('');
  const [editing, setEditing] = useState<Editing>(null);
  const [menu, setMenu] = useState<Menu>(null);
  const [dragging, setDragging] = useState(false);
  const [over, setOver] = useState<{ target: string; ok: boolean } | null>(null);
  const drag = useRef<Drag | null>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const rows = useMemo(() => buildRows({ sessions, folders, collapsed, query }), [sessions, folders, collapsed, query]);
  const everyFolder = useMemo(() => allFolders(sessions, folders).sort((a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' })), [sessions, folders]);

  const setCollapsedPersist = (next: Set<string>) => { setCollapsed(next); localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next])); };
  const toggle = (path: string, open?: boolean) => { const next = new Set(collapsed); const wantOpen = open ?? next.has(path); if (wantOpen) next.delete(path); else next.add(path); setCollapsedPersist(next); };

  useEffect(() => {
    if (!menu) return;
    const close = () => setMenu(null);
    const key = (event: KeyboardEvent) => { if (event.key === 'Escape') close(); };
    window.addEventListener('click', close); window.addEventListener('blur', close); window.addEventListener('keydown', key); window.addEventListener('resize', close);
    return () => { window.removeEventListener('click', close); window.removeEventListener('blur', close); window.removeEventListener('keydown', key); window.removeEventListener('resize', close); };
  }, [menu]);

  // Hovering a collapsed folder while dragging opens it, so a sub-folder deep inside can be reached.
  useEffect(() => {
    if (!over || !over.target || !collapsed.has(over.target)) return;
    const timer = window.setTimeout(() => toggle(over.target, true), 700);
    return () => window.clearTimeout(timer);
  }, [over?.target, collapsed]);

  const canDrop = (target: string) => {
    const d = drag.current;
    if (!d) return false;
    if (d.type === 'session') { const session = sessions.find(item => sessionKey(item) === d.id); return !!session && normalizePath(session.folder || '') !== target; }
    return !isInside(target, d.id) && parentOf(d.id) !== target;
  };
  const endDrag = () => { drag.current = null; setDragging(false); setOver(null); };
  const startDrag = (event: React.DragEvent, value: Drag, label: string) => {
    drag.current = value;
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('text/plain', label);
    setDragging(true); setMenu(null);
  };
  const dragOver = (event: React.DragEvent, target: string) => {
    if (!drag.current) return;
    event.preventDefault(); event.stopPropagation();
    const ok = canDrop(target);
    event.dataTransfer.dropEffect = ok ? 'move' : 'none';
    if (!over || over.target !== target || over.ok !== ok) setOver({ target, ok });
  };
  const doMove = (value: Drag, target: string) => {
    if (value.type === 'session') {
      const session = sessions.find(item => sessionKey(item) === value.id);
      onApply(moveSession(sessions, value.id, target), `Moved ${session?.name ?? 'session'} to ${target || 'the top level'}.`);
    } else {
      const result = moveFolder(sessions, folders, value.id, target);
      onApply(result, `Moved folder ${leafOf(value.id)} to ${target || 'the top level'}${result.moved ? ` (${result.moved} session${result.moved === 1 ? '' : 's'} inside)` : ''}.`);
    }
  };
  const drop = (event: React.DragEvent, target: string) => {
    event.preventDefault(); event.stopPropagation();
    const value = drag.current;
    endDrag();
    if (value) doMove(value, target);
  };

  const submitName = (value: string): string => {
    if (!editing) return '';
    const result = editing.mode === 'new' ? createFolder(sessions, folders, editing.parent, value) : renameFolder(sessions, folders, editing.path, value);
    if (!result.ok && !result.noop) return result.error || 'That name is not possible.';
    if (editing.mode === 'new' && result.created) { onApply(result, `Created folder ${result.created}.`); setSelected(result.created); if (editing.parent) toggle(editing.parent, true); }
    else if (editing.mode === 'rename' && result.renamed) { onApply(result, `Renamed folder to ${result.renamed}.`); setSelected(result.renamed); }
    setEditing(null);
    return '';
  };
  const removeFolder = (path: string) => {
    const result = deleteFolder(sessions, folders, path);
    onApply(result, `Deleted folder ${path}. ${result.moved ? `${result.moved} session${result.moved === 1 ? '' : 's'} moved to ${result.parent || 'the top level'}.` : 'It was empty.'}`);
    if (isInside(selected, path)) setSelected(parentOf(path));
  };
  const newFolder = (parent: string) => { setMenu(null); if (parent) toggle(parent, true); setEditing({ mode: 'new', parent }); };
  const openMenu = (event: React.MouseEvent, value: MenuTarget) => {
    event.preventDefault(); event.stopPropagation();
    setMenu({ ...value, x: Math.min(event.clientX, window.innerWidth - 250), y: Math.min(event.clientY, window.innerHeight - 330) });
  };

  const folderRow = (row: Extract<ReturnType<typeof buildRows>[number], { kind: 'folder' }>) => {
    const isOver = over?.target === row.path;
    const cls = ['react-folder', selected === row.path ? 'selected' : '', isOver ? (over!.ok ? 'drop-ok' : 'drop-bad') : ''].filter(Boolean).join(' ');
    if (editing?.mode === 'rename' && editing.path === row.path) return <NameInput key={`edit:${row.path}`} initial={row.name} depth={row.depth} label="Rename folder" onSubmit={submitName} onCancel={() => setEditing(null)} />;
    return <div key={`f:${row.path}`} role="treeitem" aria-expanded={row.hasChildren ? !row.collapsed : undefined} aria-label={`Folder ${row.name}`} className={cls} style={{ paddingLeft: 8 + row.depth * 16 }}
      draggable onDragStart={event => startDrag(event, { type: 'folder', id: row.path }, row.name)} onDragEnd={endDrag}
      onDragOver={event => dragOver(event, row.path)} onDrop={event => drop(event, row.path)}
      onClick={() => { setSelected(row.path); if (!query) toggle(row.path); }} onContextMenu={event => openMenu(event, { kind: 'folder', path: row.path })}
      title="Click to open or close. Drag a session here to move it into this folder.">
      <span className={`chev${row.collapsed ? '' : ' open'}${row.hasChildren ? '' : ' none'}`} aria-hidden="true">▸</span>
      <span className="react-tool-icon-wrap" style={{ '--c': toolColor('folder') } as React.CSSProperties}><Icon name="folder" /></span>
      <span className="name">{row.name}</span>
      <span className="count">{row.count}</span>
      <button className="more" aria-label={`Actions for folder ${row.name}`} title="Folder actions" onClick={event => openMenu(event, { kind: 'folder', path: row.path })}>⋯</button>
    </div>;
  };

  const out: React.ReactNode[] = [];
  if (editing?.mode === 'new' && editing.parent === '') out.push(<NameInput key="new:" initial="" depth={0} label="New folder name" onSubmit={submitName} onCancel={() => setEditing(null)} />);
  for (const row of rows) {
    if (row.kind === 'folder') {
      out.push(folderRow(row));
      if (editing?.mode === 'new' && editing.parent === row.path && !row.collapsed) out.push(<NameInput key={`new:${row.path}`} initial="" depth={row.depth + 1} label="New sub-folder name" onSubmit={submitName} onCancel={() => setEditing(null)} />);
    } else {
      const session = row.session;
      out.push(<button key={sessionKey(session)} draggable style={{ '--c': colorFor(session.session_type), paddingLeft: 8 + row.depth * 16 } as React.CSSProperties}
        className={activeName === session.name ? 'react-session active' : 'react-session'}
        onDragStart={event => startDrag(event, { type: 'session', id: sessionKey(session) }, session.name)} onDragEnd={endDrag}
        onClick={() => onOpen(session)} onDoubleClick={() => onOpen(session)} onContextMenu={event => openMenu(event, { kind: 'session', session })}
        title="Drag onto a folder to move it. Right-click for more.">
        <span className="react-protocol-icon" dangerouslySetInnerHTML={{ __html: iconFor(session.session_type) }} /><span><strong>{session.name}</strong><small>{session.host || session.session_type}</small></span><em>{session.favorite ? '★' : ''}</em>
      </button>);
    }
  }

  const targets = (excludeFolder?: string, currentFolder?: string) => [{ path: '', label: 'Top level (no folder)', depth: 0 }, ...everyFolder.map(path => ({ path, label: leafOf(path), depth: path.split('/').length }))]
    .filter(item => item.path !== currentFolder && !(excludeFolder && isInside(item.path, excludeFolder) && item.path !== ''));

  return <>
    <div className="react-tree-header"><strong>User sessions</strong>
      <button title={selected ? `New sub-folder in ${selected}` : 'New folder'} aria-label="New folder" onClick={() => newFolder(selected)}><Icon name="folderPlus" /></button>
      <button title="Edit selected session" aria-label="Edit selected session" onClick={onEditActive}><Icon name="edit" /></button>
      <button title="Refresh sessions" aria-label="Refresh sessions" onClick={onRefresh}><Icon name="refresh" /></button></div>
    <div ref={listRef} className={`react-session-list${dragging ? ' dragging' : ''}`} role="tree" aria-label="Sessions"
      onDragOver={event => dragOver(event, '')} onDrop={event => drop(event, '')}
      onDragLeave={event => { if (!listRef.current?.contains(event.relatedTarget as Node | null)) setOver(null); }}>
      {dragging && <div className={`react-root-drop${over?.target === '' ? (over.ok ? ' drop-ok' : ' drop-bad') : ''}`}>Drop here to move to the top level</div>}
      {out}
      {!rows.length && !editing && <p className="react-empty">{query ? 'No matching sessions' : 'No sessions yet. Use the folder button above to organise them.'}</p>}
    </div>
    {menu && <div className="react-menu" role="menu" style={{ left: menu.x, top: menu.y }} onClick={event => event.stopPropagation()}>
      {menu.kind === 'folder' ? <>
        <button role="menuitem" onClick={() => newFolder(menu.path)}>New sub-folder</button>
        <button role="menuitem" onClick={() => { setMenu(null); setEditing({ mode: 'rename', path: menu.path }); }}>Rename</button>
        <button role="menuitem" className="danger" onClick={() => { setMenu(null); removeFolder(menu.path); }}>Delete folder (keeps its sessions)</button>
        <div className="sep">Move folder to</div>
        <div className="list">{targets(menu.path, parentOf(menu.path)).map(item => <button role="menuitem" key={item.path || 'top'} style={{ paddingLeft: 12 + item.depth * 12 }} onClick={() => { setMenu(null); doMove({ type: 'folder', id: (menu as { path: string }).path }, item.path); }}>{item.label}</button>)}</div>
      </> : <>
        <button role="menuitem" onClick={() => { const session = (menu as { session: Session }).session; setMenu(null); onOpen(session); }}>Open</button>
        <button role="menuitem" onClick={() => { const session = (menu as { session: Session }).session; setMenu(null); onEdit(session); }}>Edit</button>
        <div className="sep">Move to</div>
        <div className="list">{targets(undefined, normalizePath(menu.session.folder || '')).map(item => <button role="menuitem" key={item.path || 'top'} style={{ paddingLeft: 12 + item.depth * 12 }} onClick={() => { const key = sessionKey((menu as { session: Session }).session); setMenu(null); doMove({ type: 'session', id: key }, item.path); }}>{item.label}</button>)}</div>
      </>}
    </div>}
  </>;
}
