import test from 'node:test';
import assert from 'node:assert/strict';
import {
  sessionKey, normalizePath, parentOf, leafOf, joinPath, isInside, validateFolderName, allFolders,
  buildRows, moveSession, createFolder, renameFolder, moveFolder, deleteFolder,
} from '../src/folders.js';

const s = (name, folder = '', type = 'SSH') => ({ name, session_type: type, folder, host: `${name}.example` });
const sample = () => [s('web1', 'Work/Prod'), s('web2', 'Work/Prod'), s('db', 'Work'), s('lab', 'Home'), s('scratch', '')];
const folderOf = (sessions, name) => sessions.find(x => x.name === name).folder;

test('paths are normalised and split into parent and leaf', () => {
  assert.equal(normalizePath(' Work / Prod//EU '), 'Work/Prod/EU');
  assert.equal(normalizePath(undefined), '');
  assert.equal(parentOf('Work/Prod/EU'), 'Work/Prod');
  assert.equal(parentOf('Work'), '');
  assert.equal(leafOf('Work/Prod/EU'), 'EU');
  assert.equal(joinPath('', 'Work'), 'Work');
  assert.equal(joinPath('Work', 'Prod'), 'Work/Prod');
});

test('isInside matches the folder and its descendants but not look-alike names', () => {
  assert.ok(isInside('Work', 'Work'));
  assert.ok(isInside('Work/Prod', 'Work'));
  assert.ok(!isInside('Workshop', 'Work'));
  assert.ok(isInside('anything', ''));
});

test('folder names are validated', () => {
  assert.equal(validateFolderName('Prod'), '');
  for (const bad of ['', '   ', 'a/b', 'a\\b', '.', '..', 'x'.repeat(65)]) assert.notEqual(validateFolderName(bad), '', JSON.stringify(bad));
});

test('allFolders includes parents of nested folders and kept empty folders', () => {
  assert.deepEqual(allFolders(sample(), ['Empty/Deep']).sort(), ['Empty', 'Empty/Deep', 'Home', 'Work', 'Work/Prod']);
});

test('rows are a tree: folders first (sorted), then the sessions of that level, indented by depth', () => {
  const rows = buildRows({ sessions: sample(), folders: [], collapsed: new Set(), query: '' });
  const shape = rows.map(r => (r.kind === 'folder' ? `F${r.depth}:${r.path}(${r.count})` : `S${r.depth}:${r.session.name}`));
  assert.deepEqual(shape, ['F0:Home(1)', 'S1:lab', 'F0:Work(3)', 'F1:Work/Prod(2)', 'S2:web1', 'S2:web2', 'S1:db', 'S0:scratch']);
});

test('a collapsed folder hides its contents but keeps its count', () => {
  const rows = buildRows({ sessions: sample(), folders: [], collapsed: new Set(['Work']), query: '' });
  const work = rows.find(r => r.kind === 'folder' && r.path === 'Work');
  assert.equal(work.collapsed, true);
  assert.equal(work.count, 3);
  assert.ok(!rows.some(r => r.kind === 'session' && ['web1', 'web2', 'db'].includes(r.session.name)));
});

test('an empty folder is listed and reports no children', () => {
  const rows = buildRows({ sessions: sample(), folders: ['Empty'], collapsed: new Set(), query: '' });
  const empty = rows.find(r => r.kind === 'folder' && r.path === 'Empty');
  assert.equal(empty.count, 0);
  assert.equal(empty.hasChildren, false);
});

test('searching shows matches and their parent folders, expanded even if collapsed, and hides the rest', () => {
  const rows = buildRows({ sessions: sample(), folders: ['Empty'], collapsed: new Set(['Work', 'Work/Prod']), query: 'WEB1' });
  const shape = rows.map(r => (r.kind === 'folder' ? `F:${r.path}` : `S:${r.session.name}`));
  assert.deepEqual(shape, ['F:Work', 'F:Work/Prod', 'S:web1']);
});

test('moving a session changes only its folder; moving to the same folder is a no-op', () => {
  const before = sample();
  const r = moveSession(before, sessionKey(before[3]), 'Work/Prod');
  assert.ok(r.ok);
  assert.equal(folderOf(r.sessions, 'lab'), 'Work/Prod');
  assert.equal(r.changed.length, 1);
  assert.equal(r.changed[0].host, 'lab.example');
  assert.equal(before[3].folder, 'Home', 'the input must not be mutated');
  assert.equal(moveSession(r.sessions, sessionKey(before[3]), 'Work/Prod').noop, true);
  assert.equal(folderOf(moveSession(before, sessionKey(before[0]), '').sessions, 'web1'), '');
  assert.equal(moveSession(before, 'SSH:ghost', 'Home').ok, false);
});

test('creating folders and sub-folders rejects duplicates, ignoring case', () => {
  const sessions = sample();
  const made = createFolder(sessions, [], 'Work', 'Staging');
  assert.deepEqual(made.folders, ['Work/Staging']);
  assert.equal(createFolder(sessions, made.folders, 'Work', 'staging').ok, false);
  assert.equal(createFolder(sessions, [], 'Work', 'Prod').ok, false);
  assert.equal(createFolder(sessions, [], '', 'a/b').ok, false);
  assert.equal(createFolder(sessions, [], '', 'Work').ok, false);
});

test('renaming a folder rewrites the sessions and empty folders below it, and nothing else', () => {
  const sessions = sample();
  const r = renameFolder(sessions, ['Work/Empty', 'Home/Kept'], 'Work', 'Job');
  assert.ok(r.ok);
  assert.equal(folderOf(r.sessions, 'web1'), 'Job/Prod');
  assert.equal(folderOf(r.sessions, 'db'), 'Job');
  assert.equal(folderOf(r.sessions, 'lab'), 'Home');
  assert.equal(folderOf(r.sessions, 'scratch'), '');
  assert.deepEqual(r.folders.sort(), ['Home/Kept', 'Job/Empty']);
  assert.equal(r.changed.length, 3);
  assert.equal(renameFolder(sessions, [], 'Work', 'Home').ok, false);
  assert.equal(renameFolder(sessions, [], 'Work', 'Work').noop, true);
  assert.equal(renameFolder(sessions, [], 'Work', 'a/b').ok, false);
});

test('rename does not touch a folder that only shares a prefix', () => {
  const sessions = [s('a', 'Work'), s('b', 'Workshop')];
  const r = renameFolder(sessions, [], 'Work', 'Job');
  assert.equal(folderOf(r.sessions, 'a'), 'Job');
  assert.equal(folderOf(r.sessions, 'b'), 'Workshop');
});

test('moving a folder under another keeps its sub-folders; a loop is refused', () => {
  const sessions = sample();
  const r = moveFolder(sessions, [], 'Work/Prod', 'Home');
  assert.ok(r.ok);
  assert.equal(folderOf(r.sessions, 'web1'), 'Home/Prod');
  assert.equal(folderOf(r.sessions, 'db'), 'Work');
  const up = moveFolder(sessions, [], 'Work/Prod', '');
  assert.equal(folderOf(up.sessions, 'web2'), 'Prod');
  assert.equal(moveFolder(sessions, [], 'Work', 'Work/Prod').ok, false);
  assert.equal(moveFolder(sessions, [], 'Work', 'Work').ok, false);
  assert.equal(moveFolder(sessions, [], 'Work/Prod', 'Work').noop, true);
  const clash = moveFolder([s('x', 'A/Same'), s('y', 'B/Same')], [], 'A/Same', 'B');
  assert.equal(clash.ok, false);
});

test('deleting a folder keeps its sessions and moves them up to the parent', () => {
  const sessions = sample();
  const r = deleteFolder(sessions, ['Work/Empty', 'Home/Kept'], 'Work/Prod');
  assert.equal(folderOf(r.sessions, 'web1'), 'Work');
  assert.equal(r.sessions.length, sessions.length, 'no session is deleted');
  assert.equal(r.parent, 'Work');
  const top = deleteFolder(sessions, ['Work/Empty'], 'Work');
  assert.equal(folderOf(top.sessions, 'web1'), '');
  assert.equal(folderOf(top.sessions, 'db'), '');
  assert.deepEqual(top.folders, []);
  assert.equal(top.moved, 3);
});
