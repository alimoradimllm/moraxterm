import test from 'node:test';
import assert from 'node:assert/strict';
import { uniqueName, planSave, duplicateSession, toggleFavorite, removeSession } from '../src/sessionOps.js';
import { sessionKey, buildRows } from '../src/folders.js';

const s = (name, type = 'SSH', extra = {}) => ({ name, session_type: type, host: `${name}.example`, ...extra });
const names = list => list.map(x => x.name).sort();

test('uniqueName appends a number only when the name is taken', () => {
  assert.equal(uniqueName([s('a')], 'b'), 'b');
  assert.equal(uniqueName([s('a copy')], 'a copy'), 'a copy 2');
  assert.equal(uniqueName([s('a copy'), s('a copy 2'), s('a copy 3')], 'a copy'), 'a copy 4');
});

test('saving a new session adds it and deletes nothing', () => {
  const list = [s('web')];
  const r = planSave(list, undefined, s('db'));
  assert.ok(r.ok && r.created);
  assert.deepEqual(names(r.sessions), ['db', 'web']);
  assert.deepEqual(r.deleted, []);
  assert.equal(r.changed.length, 1);
});

test('a "New session" draft that is not in the list counts as new even though it has a name', () => {
  const list = [s('web')];
  const r = planSave(list, s('New SSH session'), s('New SSH session'));
  assert.ok(r.ok && r.created);
  assert.equal(r.sessions.length, 2);
});

test('editing a session in place saves it once and deletes nothing', () => {
  const list = [s('web'), s('db')];
  const r = planSave(list, list[0], { ...list[0], host: 'new.example' });
  assert.ok(r.ok && !r.created);
  assert.deepEqual(r.deleted, []);
  assert.equal(r.sessions.length, 2);
  assert.equal(r.sessions.find(x => x.name === 'web').host, 'new.example');
});

test('renaming saves the new name and deletes the old record, so no duplicate is left behind', () => {
  const list = [s('web', 'SSH', { folder: 'Work' }), s('db')];
  const r = planSave(list, list[0], { ...list[0], name: ' web-prod ' });
  assert.ok(r.ok);
  assert.deepEqual(r.deleted, ['web']);
  assert.deepEqual(r.renamed, { from: 'web', to: 'web-prod' });
  assert.deepEqual(names(r.sessions), ['db', 'web-prod']);
  assert.equal(r.changed[0].name, 'web-prod');
  assert.equal(r.changed[0].folder, 'Work', 'other fields travel with the rename');
});

test('a name that belongs to another session is refused, whatever its type, because the store would overwrite it', () => {
  const list = [s('web'), s('db', 'Rdp')];
  const toExisting = planSave(list, list[0], { ...list[0], name: 'db' });
  assert.equal(toExisting.ok, false);
  assert.match(toExisting.error, /already exists/);
  const newOne = planSave(list, undefined, s('db', 'SSH'));
  assert.equal(newOne.ok, false);
});

test('changing the type of a session keeps one record under that name', () => {
  const list = [s('box', 'SSH')];
  const r = planSave(list, list[0], { ...list[0], session_type: 'Telnet' });
  assert.ok(r.ok);
  assert.equal(r.sessions.length, 1);
  assert.equal(r.sessions[0].session_type, 'Telnet');
  assert.deepEqual(r.deleted, []);
});

test('an empty or blank name is refused', () => {
  assert.equal(planSave([], undefined, s('')).ok, false);
  assert.equal(planSave([], undefined, s('   ')).ok, false);
});

test('duplicate makes a uniquely named, non-favourite copy that keeps the folder', () => {
  const list = [s('web', 'SSH', { favorite: true, folder: 'Work/Prod' })];
  const first = duplicateSession(list, sessionKey(list[0]));
  assert.equal(first.duplicated, 'web copy');
  assert.equal(first.changed[0].favorite, false);
  assert.equal(first.changed[0].folder, 'Work/Prod');
  const second = duplicateSession(first.sessions, sessionKey(list[0]));
  assert.equal(second.duplicated, 'web copy 2');
  assert.equal(duplicateSession(list, 'SSH:ghost').ok, false);
});

test('toggling favourite flips only that session and does not mutate the input', () => {
  const list = [s('a'), s('b', 'SSH', { favorite: true })];
  const on = toggleFavorite(list, sessionKey(list[0]));
  assert.equal(on.favorite, true);
  assert.equal(on.sessions[0].favorite, true);
  assert.equal(list[0].favorite, undefined);
  const off = toggleFavorite(list, sessionKey(list[1]));
  assert.equal(off.favorite, false);
  assert.equal(off.changed.length, 1);
});

test('removing a session reports the name to delete and keeps the others', () => {
  const list = [s('a'), s('b')];
  const r = removeSession(list, sessionKey(list[0]));
  assert.ok(r.ok);
  assert.deepEqual(r.deleted, ['a']);
  assert.deepEqual(names(r.sessions), ['b']);
  assert.deepEqual(r.changed, []);
  assert.equal(removeSession(list, 'SSH:ghost').ok, false);
});

test('favourites are listed first inside their folder, then by name', () => {
  const list = [s('alpha', 'SSH', { folder: 'F' }), s('zulu', 'SSH', { folder: 'F', favorite: true }), s('bravo', 'SSH', { folder: 'F' })];
  const rows = buildRows({ sessions: list, folders: [], collapsed: new Set(), query: '' });
  assert.deepEqual(rows.filter(r => r.kind === 'session').map(r => r.session.name), ['zulu', 'alpha', 'bravo']);
});
