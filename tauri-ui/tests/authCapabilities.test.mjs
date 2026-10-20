import test from 'node:test';
import assert from 'node:assert/strict';
import { detectAuthCapabilities } from '../src/authCapabilities.js';

test('reports unsupported capability without browser APIs', () => {
  const result = detectAuthCapabilities({ navigator: { platform: 'Win32' } });
  assert.equal(result.webAuthn, false);
  assert.equal(result.windowsHello, false);
  assert.equal(result.fido2Ssh, false);
  assert.match(result.status, /unsupported/i);
});

test('reports WebAuthn exposure without claiming Windows Hello or SSH support', () => {
  const result = detectAuthCapabilities({ navigator: { platform: 'Win32', credentials: {} }, PublicKeyCredential: function PublicKeyCredential() {} });
  assert.equal(result.webAuthn, true);
  assert.equal(result.windowsHello, false);
  assert.equal(result.fido2Ssh, false);
  assert.match(result.status, /no platform authentication adapter/i);
});
