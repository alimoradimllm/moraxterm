import test from 'node:test';
import assert from 'node:assert/strict';
test('local-only provider is ready and remote providers are explicitly unavailable', async () => { const { providerStatus } = await import('../src/syncProviders.ts'); assert.equal(providerStatus('local-only').state, 'ready'); assert.equal(providerStatus('github-repository').state, 'unavailable'); assert.match(providerStatus('google-drive').detail, /OAuth/); });
