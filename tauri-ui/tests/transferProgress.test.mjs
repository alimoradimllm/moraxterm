import test from 'node:test';
import assert from 'node:assert/strict';
import { formatBytes, percent, describeProgress, applyProgress } from '../src/transferProgress.js';

test('formatBytes uses the right unit and rounds sensibly', () => {
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(1023), '1023 B');
  assert.equal(formatBytes(1024), '1 KB');
  assert.equal(formatBytes(1536), '1.5 KB');
  assert.equal(formatBytes(1500000), '1.4 MB');
  assert.equal(formatBytes(261120), '255 KB');
  assert.equal(formatBytes(3 * 1024 ** 3), '3 GB');
  assert.equal(formatBytes(5 * 1024 ** 4), '5 TB');
  assert.equal(formatBytes(1e18), '909495 TB', 'beyond TB stays in TB rather than inventing a unit');
});

test('formatBytes treats nonsense as zero instead of printing NaN', () => {
  for (const bad of [undefined, null, NaN, -5, 'abc', Infinity]) assert.equal(formatBytes(bad), '0 B', String(bad));
  assert.equal(formatBytes('2048'), '2 KB');
});

test('percent is a whole number 0-100 and null when the total is unknown', () => {
  assert.equal(percent(0, 100), 0);
  assert.equal(percent(50, 100), 50);
  assert.equal(percent(1044480, 1500000), 69);
  assert.equal(percent(99.9, 100), 99, 'never rounds up to 100 before it is done');
  assert.equal(percent(100, 100), 100);
  assert.equal(percent(150, 100), 100, 'clamped');
  assert.equal(percent(5, 0), null);
  assert.equal(percent(5, null), null);
  assert.equal(percent(5, undefined), null);
  assert.equal(percent(-1, 100), null);
  assert.equal(percent('x', 100), null);
});

test('describeProgress never shows a percentage it cannot know', () => {
  assert.equal(describeProgress(null, null), 'Starting…');
  assert.equal(describeProgress(undefined, 10), 'Starting…');
  assert.equal(describeProgress(1044480, 1500000), '1020 KB of 1.4 MB (69%)');
  assert.equal(describeProgress(2048, null), '2 KB transferred (size unknown)');
  assert.equal(describeProgress(0, 1500000), '0 B of 1.4 MB (0%)');
});

test('applyProgress ignores other jobs and malformed events', () => {
  const cur = { done: 10, total: 100 };
  assert.equal(applyProgress(cur, { id: 'other', transferred_bytes: 50, total_bytes: 100 }, 'mine'), cur);
  assert.equal(applyProgress(cur, { id: 'mine' }, 'mine'), cur);
  assert.equal(applyProgress(cur, { id: 'mine', transferred_bytes: 'lots' }, 'mine'), cur);
  assert.equal(applyProgress(cur, null, 'mine'), cur);
  assert.equal(applyProgress(null, undefined, 'mine'), null);
});

test('applyProgress never goes backwards or above the total', () => {
  let m = applyProgress(null, { id: 'j', transferred_bytes: 40, total_bytes: 100 }, 'j');
  assert.deepEqual(m, { done: 40, total: 100 });
  m = applyProgress(m, { id: 'j', transferred_bytes: 30, total_bytes: 100 }, 'j');
  assert.deepEqual(m, { done: 40, total: 100 }, 'a late smaller value does not move the bar back');
  m = applyProgress(m, { id: 'j', transferred_bytes: 70, total_bytes: 100 }, 'j');
  assert.deepEqual(m, { done: 70, total: 100 });
  m = applyProgress(m, { id: 'j', transferred_bytes: 999, total_bytes: 100 }, 'j');
  assert.deepEqual(m, { done: 100, total: 100 }, 'clamped to the total');
});

test('applyProgress keeps a known total when later events omit it, and copes with an unknown total', () => {
  let m = applyProgress(null, { id: 'j', transferred_bytes: 10, total_bytes: null }, 'j');
  assert.deepEqual(m, { done: 10, total: null });
  m = applyProgress(m, { id: 'j', transferred_bytes: 20, total_bytes: 200 }, 'j');
  assert.deepEqual(m, { done: 20, total: 200 });
  m = applyProgress(m, { id: 'j', transferred_bytes: 30, total_bytes: null }, 'j');
  assert.deepEqual(m, { done: 30, total: 200 }, 'a missing total does not erase the one already known');
  m = applyProgress(null, { id: 'j', transferred_bytes: 5, total_bytes: 0 }, 'j');
  assert.deepEqual(m, { done: 5, total: null }, 'a zero total means unknown');
});
