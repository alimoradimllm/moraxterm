import test from 'node:test';
import assert from 'node:assert/strict';

test('vault payload accepts version 1 settings', async () => {
  const { readVaultPayload, readFullVaultPayload } = await import('../src/vaultUi.ts');
  assert.deepEqual(readVaultPayload({ version: 1, settings: { fontSize: 16 } }), { fontSize: 16 });

  const full = readFullVaultPayload({
    version: 1,
    sessions: [{ name: 'prod-server', host: '192.168.1.10', session_type: 'SSH' }],
    secrets: [{ reference: 'prod-identity', kind: 'password', value: 'secret' }],
    providers: [{ fontSize: 18, appTheme: 'midnight' }],
  });
  assert.equal(full.sessions.length, 1);
  assert.equal(full.sessions[0].name, 'prod-server');
  assert.equal(full.secrets.length, 1);
  assert.equal(full.settings.fontSize, 18);
});

test('vault payload rejects unsupported versions and malformed settings', async () => {
  const { readVaultPayload } = await import('../src/vaultUi.ts');
  assert.throws(() => readVaultPayload({ version: 2, settings: {} }), /unsupported vault version/);
  assert.throws(() => readVaultPayload(null), /vault payload is invalid/);
  assert.throws(() => readVaultPayload('not an object'), /vault payload is invalid/);
});
