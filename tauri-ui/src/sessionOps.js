// Session operations (save, rename, duplicate, favourite, delete) as pure functions, so they can be unit tested.
// The Rust store keys sessions by NAME alone (name TEXT PRIMARY KEY), so a rename is "save the new name, delete the old
// one", and two sessions can never share a name, whatever their type. Every function returns the same shape as
// folders.js: { ok, sessions, changed, deleted, ... } where `changed` are the records to save and `deleted` are names to
// remove. Saves are applied before deletes, so a failure part-way leaves an extra record rather than losing one.
import { sessionKey } from './folders.js';

const fail = error => ({ ok: false, error });

export function uniqueName(sessions, base) {
  const taken = new Set(sessions.map(session => session.name));
  if (!taken.has(base)) return base;
  let n = 2;
  while (taken.has(`${base} ${n}`)) n += 1;
  return `${base} ${n}`;
}

/**
 * Saves `draft`, which was opened for editing as `original` (undefined or not in the list means a new session).
 * Refuses a name that belongs to a different session, because the store would silently overwrite it.
 */
export function planSave(sessions, original, draft) {
  const name = String(draft.name ?? '').trim();
  if (!name) return fail('Give the session a name before saving.');
  const existing = original ? sessions.find(session => session.name === original.name && session.session_type === original.session_type) : undefined;
  const clash = sessions.find(session => session.name === name && session !== existing);
  if (clash) return fail(`A session named "${name}" already exists. Session names must be unique, so choose another name.`);
  const saved = { ...draft, name };
  const deleted = existing && existing.name !== name ? [existing.name] : [];
  const rest = sessions.filter(session => session !== existing && session.name !== name);
  return { ok: true, sessions: [...rest, saved], changed: [saved], deleted, created: !existing, renamed: deleted.length ? { from: deleted[0], to: name } : undefined };
}

export function duplicateSession(sessions, key) {
  const source = sessions.find(session => sessionKey(session) === key);
  if (!source) return fail('That session no longer exists.');
  const copy = { ...source, name: uniqueName(sessions, `${source.name} copy`), favorite: false };
  return { ok: true, sessions: [...sessions, copy], changed: [copy], deleted: [], duplicated: copy.name };
}

export function toggleFavorite(sessions, key) {
  const source = sessions.find(session => sessionKey(session) === key);
  if (!source) return fail('That session no longer exists.');
  const updated = { ...source, favorite: !source.favorite };
  return { ok: true, sessions: sessions.map(session => (session === source ? updated : session)), changed: [updated], deleted: [], favorite: updated.favorite };
}

export function removeSession(sessions, key) {
  const source = sessions.find(session => sessionKey(session) === key);
  if (!source) return fail('That session no longer exists.');
  return { ok: true, sessions: sessions.filter(session => session !== source), changed: [], deleted: [source.name], removed: source.name };
}
