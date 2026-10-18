// Session folders: pure functions, no DOM and no Tauri, so they can be unit tested.
// A folder is a path such as "Work/Prod/EU"; "/" separates sub-folders and "" is the top level.
// A session belongs to a folder through its `folder` field, which is how the Rust store keeps it.
// Empty folders have no session to carry them, so the UI keeps a separate list of those.

const SEP = '/';

export const sessionKey = session => `${session.session_type}:${session.name}`;

export function normalizePath(path = '') {
  return String(path).split(SEP).map(part => part.trim()).filter(Boolean).join(SEP);
}

export const parentOf = path => (path.lastIndexOf(SEP) < 0 ? '' : path.slice(0, path.lastIndexOf(SEP)));
export const leafOf = path => path.slice(path.lastIndexOf(SEP) + 1);
export const joinPath = (parent, name) => (parent ? `${parent}${SEP}${name}` : name);

/** True when `path` is `folder` itself or lives somewhere below it. The top level ("") contains everything. */
export const isInside = (path, folder) => folder === '' || path === folder || path.startsWith(folder + SEP);

export function validateFolderName(name) {
  const value = String(name ?? '').trim();
  if (!value) return 'Enter a folder name.';
  if (value.includes(SEP) || value.includes('\\')) return 'A folder name cannot contain / or \\. Create the sub-folder inside its parent instead.';
  if (value === '.' || value === '..') return 'That folder name is not allowed.';
  if (value.length > 64) return 'Folder names are limited to 64 characters.';
  return '';
}

function ancestors(path) {
  const out = [];
  let current = '';
  for (const part of path.split(SEP).filter(Boolean)) {
    current = joinPath(current, part);
    out.push(current);
  }
  return out;
}

/** Every folder that exists: those holding sessions, the kept empty ones, and all their parents. */
export function allFolders(sessions, extra = []) {
  const found = new Set();
  for (const session of sessions) ancestors(normalizePath(session.folder || '')).forEach(path => found.add(path));
  for (const path of extra) ancestors(normalizePath(path)).forEach(path2 => found.add(path2));
  return [...found];
}

const byName = (a, b) => a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });

/**
 * Flattens the tree into rows for rendering.
 * Folder row: { kind: 'folder', path, name, depth, count, collapsed, hasChildren }
 * Session row: { kind: 'session', session, depth }
 * While searching, only matching sessions and the folders above them are shown, all expanded.
 */
export function buildRows({ sessions, folders = [], collapsed = new Set(), query = '' }) {
  const q = query.trim().toLowerCase();
  const matches = session => !q || JSON.stringify(session).toLowerCase().includes(q);
  const base = sessions.filter(matches);
  const paths = new Set(q ? base.flatMap(s => ancestors(normalizePath(s.folder || ''))) : allFolders(sessions, folders));

  const counts = new Map();
  for (const session of base) for (const path of ancestors(normalizePath(session.folder || ''))) counts.set(path, (counts.get(path) || 0) + 1);

  const childFolders = new Map();
  for (const path of paths) {
    const parent = parentOf(path);
    if (!childFolders.has(parent)) childFolders.set(parent, []);
    childFolders.get(parent).push(path);
  }
  const sessionsIn = new Map();
  for (const session of base) {
    const folder = normalizePath(session.folder || '');
    if (!sessionsIn.has(folder)) sessionsIn.set(folder, []);
    sessionsIn.get(folder).push(session);
  }

  const rows = [];
  const walk = (parent, depth) => {
    for (const path of (childFolders.get(parent) || []).sort((a, b) => byName(leafOf(a), leafOf(b)))) {
      const isCollapsed = !q && collapsed.has(path);
      const hasChildren = (childFolders.get(path) || []).length > 0 || (sessionsIn.get(path) || []).length > 0;
      rows.push({ kind: 'folder', path, name: leafOf(path), depth, count: counts.get(path) || 0, collapsed: isCollapsed, hasChildren });
      if (!isCollapsed) walk(path, depth + 1);
    }
    for (const session of (sessionsIn.get(parent) || []).sort((a, b) => Number(!!b.favorite) - Number(!!a.favorite) || byName(a.name, b.name))) rows.push({ kind: 'session', session, depth });
  };
  walk('', 0);
  return rows;
}

const failure = error => ({ ok: false, error });
const noop = () => ({ ok: false, noop: true });

export function moveSession(sessions, key, folder) {
  const target = normalizePath(folder);
  const session = sessions.find(item => sessionKey(item) === key);
  if (!session) return failure('That session no longer exists.');
  if (normalizePath(session.folder || '') === target) return noop();
  const moved = { ...session, folder: target };
  return { ok: true, sessions: sessions.map(item => (item === session ? moved : item)), changed: [moved], moved: 1 };
}

export function createFolder(sessions, folders, parent, name) {
  const problem = validateFolderName(name);
  if (problem) return failure(problem);
  const path = joinPath(normalizePath(parent), name.trim());
  if (allFolders(sessions, folders).some(existing => existing.toLowerCase() === path.toLowerCase())) return failure(`A folder named "${name.trim()}" already exists here.`);
  return { ok: true, folders: [...new Set([...folders.map(normalizePath), path])], changed: [], created: path };
}

function remap(sessions, folders, from, to) {
  const swap = path => (path === from ? to : to + path.slice(from.length));
  const changed = [];
  const next = sessions.map(session => {
    const folder = normalizePath(session.folder || '');
    if (!isInside(folder, from) || folder === '' ) return session;
    const moved = { ...session, folder: swap(folder) };
    changed.push(moved);
    return moved;
  });
  const nextFolders = [...new Set(folders.map(normalizePath).filter(Boolean).map(path => (isInside(path, from) ? swap(path) : path)))];
  return { ok: true, sessions: next, folders: nextFolders, changed, moved: changed.length };
}

export function renameFolder(sessions, folders, from, newName) {
  const problem = validateFolderName(newName);
  if (problem) return failure(problem);
  const to = joinPath(parentOf(from), newName.trim());
  if (to === from) return noop();
  const clash = allFolders(sessions, folders).find(existing => existing.toLowerCase() === to.toLowerCase() && existing !== from);
  if (clash) return failure(`A folder named "${newName.trim()}" already exists here.`);
  return { ...remap(sessions, folders, from, to), renamed: to };
}

/** Moves the folder `from` (with everything inside it) under `toParent` ("" = top level). */
export function moveFolder(sessions, folders, from, toParent) {
  const parent = normalizePath(toParent);
  if (isInside(parent, from)) return failure('A folder cannot be moved into itself or one of its own sub-folders.');
  if (parentOf(from) === parent) return noop();
  const to = joinPath(parent, leafOf(from));
  if (allFolders(sessions, folders).some(existing => existing.toLowerCase() === to.toLowerCase())) return failure(`"${parent || 'The top level'}" already has a folder named "${leafOf(from)}".`);
  return { ...remap(sessions, folders, from, to), renamed: to };
}

/** Deletes a folder and its sub-folders. The sessions inside are kept and move up to its parent folder. */
export function deleteFolder(sessions, folders, path) {
  const parent = parentOf(path);
  const changed = [];
  const next = sessions.map(session => {
    const folder = normalizePath(session.folder || '');
    if (folder === '' || !isInside(folder, path)) return session;
    const moved = { ...session, folder: parent };
    changed.push(moved);
    return moved;
  });
  const kept = folders.map(normalizePath).filter(item => item && !isInside(item, path));
  return { ok: true, sessions: next, folders: kept, changed, moved: changed.length, parent };
}
