import test from 'node:test';
import assert from 'node:assert/strict';

test('vault payload accepts version 1 settings', async () => {
  const { readVaultPayload } = await import('../src/vaultUi.ts');
  assert.deepEqual(readVaultPayload({ version: 1, settings: { fontSize: 16 } }), { fontSize: 16 });
});

test('vault payload rejects unsupported versions and malformed settings', async () => {
  const { readVaultPayload } = await import('../src/vaultUi.ts');
  assert.throws(() => readVaultPayload({ version: 2, settings: {} }), /unsupported vault version/);
  assert.throws(() => readVaultPayload({ version: 1, settings: [] }), /vault settings are invalid/);
});
