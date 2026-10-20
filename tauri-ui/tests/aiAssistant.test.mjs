import test from 'node:test';
import assert from 'node:assert/strict';
import { createAiProvider, redactSecrets } from '../src/aiAssistant.js';

test('redacts credential-like values and private key blocks', () => {
  assert.equal(redactSecrets('password=topsecret token:abc'), '[REDACTED] [REDACTED]');
  assert.equal(redactSecrets('-----BEGIN PRIVATE KEY-----secret-----END PRIVATE KEY-----'), '[REDACTED]');
});

test('default provider is explicit, unsupported, and preview-only', async () => {
  const result = await createAiProvider({ enabled: false, provider: 'none' }).preview('echo safe');
  assert.equal(result.supported, false);
  assert.equal(result.previewOnly, true);
  assert.match(result.text, /disabled/i);
});
