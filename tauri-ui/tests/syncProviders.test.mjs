import test from 'node:test';
import assert from 'node:assert/strict';

test('sync providers expose ready transports and explicit unavailable Google Drive contract', async () => {
  const { providerStatus, syncProviders } = await import('../src/syncProviders.ts');
  assert.equal(providerStatus('local-only').state, 'ready');
  assert.equal(providerStatus('github-repository').state, 'ready');
  assert.equal(providerStatus('gitlab-repository').state, 'ready');
  assert.equal(providerStatus('webdav-object-store').state, 'ready');
  assert.equal(providerStatus('s3-compatible').state, 'ready');
  assert.equal(providerStatus('google-drive').state, 'unavailable');
  assert.match(providerStatus('google-drive').detail, /OAuth/);
  assert.equal(syncProviders.length, 6);
});
