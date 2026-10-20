import test from 'node:test';
import assert from 'node:assert/strict';
import { canStop, isMultiPasteShortcut, isTerminalReply, mirrorTargets, pasteTargets } from '../src/multiexec.js';

const tabs = [
  { id: 'a', state: 'connected' },
  { id: 'b', state: 'connected' },
  { id: 'c', state: 'connected' },
  { id: 'd', state: 'exited' },
  { id: 'e', state: 'failed' },
];

test('keystrokes are mirrored to every running, non-excluded terminal', () => {
  const ids = mirrorTargets({ on: true, sourceId: 'a', tabs, excluded: new Set(), data: 'l' });
  assert.deepEqual(ids.sort(), ['a', 'b', 'c']);
});

test('nothing is mirrored when MultiExec is off', () => {
  assert.deepEqual(mirrorTargets({ on: false, sourceId: 'a', tabs, excluded: new Set(), data: 'l' }), ['a']);
});

test('typing in an excluded terminal stays in that terminal', () => {
  assert.deepEqual(mirrorTargets({ on: true, sourceId: 'a', tabs, excluded: new Set(['a']), data: 'l' }), ['a']);
});

test('excluded terminals do not receive input from others', () => {
  const ids = mirrorTargets({ on: true, sourceId: 'a', tabs, excluded: new Set(['b']), data: 'l' });
  assert.deepEqual(ids.sort(), ['a', 'c']);
});

test('exited and failed terminals are skipped', () => {
  const ids = mirrorTargets({ on: true, sourceId: 'a', tabs, excluded: new Set(), data: 'x' });
  assert.ok(!ids.includes('d') && !ids.includes('e'));
});

test('the source terminal always receives its own input, even when starting', () => {
  const ids = mirrorTargets({ on: true, sourceId: 's', tabs: [{ id: 's', state: 'starting' }, ...tabs], excluded: new Set(), data: 'x' });
  assert.ok(ids.includes('s'));
});

test('xterm.js replies and reports are never mirrored', () => {
  for (const reply of ['\x1b[1;18R', '\x1b[?1;2c', '\x1b[0n', '\x1b[I', '\x1b[O', '\x1b[<0;10;5M', '\x1b[<0;10;5m', '\x1b]10;rgb:ffff/ffff/ffff\x07']) {
    assert.ok(isTerminalReply(reply), JSON.stringify(reply));
    assert.deepEqual(mirrorTargets({ on: true, sourceId: 'a', tabs, excluded: new Set(), data: reply }), ['a']);
  }
});

test('real keys, arrows, Enter, and pasted text are mirrored', () => {
  for (const key of ['a', '\r', '\x03', '\x1b[A', '\x1b[1;5C', '\x1b[3~', 'echo hi', '\x1b']) {
    assert.ok(!isTerminalReply(key), JSON.stringify(key));
  }
});

test('multi-paste targets skip excluded, exited, and failed terminals', () => {
  assert.deepEqual(pasteTargets({ tabs, excluded: new Set(['c']) }), ['a', 'b']);
});

test('Ctrl+Shift+Insert is the multi-paste shortcut and nothing close to it is', () => {
  const ev = (o) => ({ ctrlKey: false, shiftKey: false, altKey: false, key: '', code: '', ...o });
  assert.ok(isMultiPasteShortcut(ev({ ctrlKey: true, shiftKey: true, key: 'Insert', code: 'Insert' })));
  assert.ok(!isMultiPasteShortcut(ev({ shiftKey: true, key: 'Insert', code: 'Insert' })));
  assert.ok(!isMultiPasteShortcut(ev({ ctrlKey: true, key: 'Insert', code: 'Insert' })));
  assert.ok(!isMultiPasteShortcut(ev({ ctrlKey: true, shiftKey: true, altKey: true, key: 'Insert', code: 'Insert' })));
  assert.ok(!isMultiPasteShortcut(ev({ ctrlKey: true, shiftKey: true, key: 'v', code: 'KeyV' })));
});

test('only a live terminal can be stopped, and a stopped one is no longer a mirror target', () => {
  assert.equal(canStop({ state: 'connected' }), true);
  assert.equal(canStop({ state: 'starting' }), true);
  assert.equal(canStop({ state: 'exited' }), false);
  assert.equal(canStop({ state: 'failed' }), false);
  const after = tabs.map(tab => tab.id === 'b' ? { ...tab, state: 'exited' } : tab);
  assert.deepEqual(mirrorTargets({ on: true, sourceId: 'a', tabs: after, excluded: new Set(), data: 'l' }).sort(), ['a', 'c']);
  assert.deepEqual(pasteTargets({ tabs: after, excluded: new Set() }).sort(), ['a', 'c']);
});
