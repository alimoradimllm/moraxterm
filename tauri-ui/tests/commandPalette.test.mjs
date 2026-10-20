import test from 'node:test';
import assert from 'node:assert/strict';

const source = await import('../src/commandPalette.js');

test('palette filtering is case insensitive and preserves order', () => {
  const commands = [{ label: 'Open Settings' }, { label: 'Open Tools' }, { label: 'Toggle Focus Mode' }];
  assert.deepEqual(source.filterPaletteCommands(commands, 'focus').map(item => item.label), ['Toggle Focus Mode']);
  assert.deepEqual(source.filterPaletteCommands(commands, 'OPEN').map(item => item.label), ['Open Settings', 'Open Tools']);
});

test('palette shortcut accepts Ctrl and Command J', () => {
  assert.equal(source.isPaletteShortcut({ ctrlKey: true, metaKey: false, key: 'j' }), true);
  assert.equal(source.isPaletteShortcut({ ctrlKey: false, metaKey: true, key: 'J' }), true);
  assert.equal(source.isPaletteShortcut({ ctrlKey: false, metaKey: false, key: 'j' }), false);
});
