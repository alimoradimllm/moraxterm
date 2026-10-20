import test from 'node:test';
import assert from 'node:assert/strict';
import { addPane, MAX_WORKSPACE_PANES, removePane, restoreLayout } from '../src/workspaceLayout.js';

test('workspace layout is bounded at sixteen panes and deduplicates', () => {
  const ids = Array.from({ length: 16 }, (_, index) => `tab-${index}`);
  assert.equal(addPane(ids, 'tab-16').length, MAX_WORKSPACE_PANES);
  assert.deepEqual(addPane(ids.slice(0, 2), 'tab-1'), ids.slice(0, 2));
});

test('layout restore drops closed tabs and preserves order', () => {
  assert.deepEqual(restoreLayout(['b', 'gone', 'a'], ['a', 'b']), ['b', 'a']);
  assert.deepEqual(removePane(['a', 'b'], 'a'), ['b']);
});
