import test from 'node:test';
import assert from 'node:assert/strict';

const validEnvelopeJson = JSON.stringify({
  version: 1,
  salt: 'c2FsdF9leGFtcGxl',
  nonce: 'bm9uY2VfZXhhbXBsZQ==',
  ciphertext: 'Y2lwaGVydGV4dF9leGFtcGxl',
});
const validEnvelopeBase64 = Buffer.from(validEnvelopeJson).toString('base64');

test('assertEncryptedEnvelope validates authentic envelopes and rejects plaintext', async () => {
  const { assertEncryptedEnvelope } = await import('../src/syncTransports.ts');

  // Valid raw JSON envelope
  const parsed1 = assertEncryptedEnvelope(validEnvelopeJson);
  assert.equal(parsed1.version, 1);
  assert.equal(parsed1.salt, 'c2FsdF9leGFtcGxl');
  assert.equal(parsed1.ciphertext, 'Y2lwaGVydGV4dF9leGFtcGxl');

  // Valid base64 envelope
  const parsed2 = assertEncryptedEnvelope(validEnvelopeBase64);
  assert.equal(parsed2.version, 1);

  // Rejects empty string
  assert.throws(() => assertEncryptedEnvelope(''), /empty/);

  // Rejects unencrypted sessions
  assert.throws(
    () => assertEncryptedEnvelope(JSON.stringify({ version: 1, sessions: [{ name: 'prod', host: '10.0.0.1' }] })),
    /Unencrypted payload rejected/
  );

  // Rejects plaintext password
  assert.throws(
    () => assertEncryptedEnvelope(JSON.stringify({ version: 1, password: 'supersecret', salt: 's', nonce: 'n', ciphertext: 'c' })),
    /Unencrypted payload rejected/
  );

  // Rejects unencrypted private key
  assert.throws(
    () => assertEncryptedEnvelope(JSON.stringify({ version: 1, private_key: '-----BEGIN OPENSSH PRIVATE KEY-----' })),
    /Unencrypted payload rejected/
  );

  // Rejects unsupported version
  assert.throws(
    () => assertEncryptedEnvelope(JSON.stringify({ version: 2, salt: 's', nonce: 'n', ciphertext: 'c' })),
    /Unsupported vault envelope version/
  );

  // Rejects missing crypto fields
  assert.throws(
    () => assertEncryptedEnvelope(JSON.stringify({ version: 1, salt: 's' })),
    /missing required cryptographic envelope fields/
  );
});

test('keyring storage manages secrets with explicit store, status, get, and delete', async () => {
  const { createMemoryKeyring, setKeyringStorage, getKeyringStorage } = await import('../src/syncTransports.ts');
  const keyring = createMemoryKeyring();
  setKeyringStorage(keyring);

  assert.equal(await keyring.getStatus('github-repository'), false);
  await assert.rejects(async () => await keyring.getSecret('github-repository'), /not authenticated/);

  await keyring.storeSecret('github-repository', 'ghp_mock_token_12345');
  assert.equal(await keyring.getStatus('github-repository'), true);
  assert.equal(await keyring.getSecret('github-repository'), 'ghp_mock_token_12345');

  const deleted = await keyring.deleteSecret('github-repository');
  assert.equal(deleted, true);
  assert.equal(await keyring.getStatus('github-repository'), false);
  await assert.rejects(async () => await keyring.getSecret('github-repository'), /not authenticated/);
});

test('GitHub transport downloads and uploads encrypted envelopes with conflict detection', async () => {
  const { downloadGitHubVault, uploadGitHubVault } = await import('../src/syncTransports.ts');

  // Mock download success
  const mockFetchDownload = async (url, init) => {
    assert.match(url, /repos\/acme\/vault\/contents\/moraxterm-vault\.mvx/);
    assert.equal(init.headers.Authorization, 'Bearer ghp_test');
    return new Response(JSON.stringify({
      content: Buffer.from(validEnvelopeJson).toString('base64'),
      sha: 'commit_sha_111',
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  };

  const dlResult = await downloadGitHubVault(
    { owner: 'acme', repo: 'vault' },
    'ghp_test',
    { fetchImpl: mockFetchDownload }
  );
  assert.equal(dlResult.success, true);
  assert.equal(dlResult.remoteRevision, 'commit_sha_111');
  assert.equal(dlResult.envelope, validEnvelopeJson);

  // Mock upload success
  const mockFetchUpload = async (url, init) => {
    assert.equal(init.method, 'PUT');
    const body = JSON.parse(init.body);
    assert.equal(body.sha, 'commit_sha_111');
    assert.equal(Buffer.from(body.content, 'base64').toString('utf8'), validEnvelopeJson);
    return new Response(JSON.stringify({
      content: { sha: 'commit_sha_222' },
    }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  };

  const upResult = await uploadGitHubVault(
    { owner: 'acme', repo: 'vault' },
    'ghp_test',
    validEnvelopeJson,
    'commit_sha_111',
    { fetchImpl: mockFetchUpload }
  );
  assert.equal(upResult.success, true);
  assert.equal(upResult.remoteRevision, 'commit_sha_222');

  // Mock 409 conflict
  const mockFetchConflict = async () => new Response(JSON.stringify({ message: 'Conflict' }), { status: 409 });
  const conflictResult = await uploadGitHubVault(
    { owner: 'acme', repo: 'vault' },
    'ghp_test',
    validEnvelopeJson,
    'stale_sha',
    { fetchImpl: mockFetchConflict }
  );
  assert.equal(conflictResult.success, false);
  assert.equal(conflictResult.conflict, true);
  assert.match(conflictResult.error, /Conflict/);
});

test('GitLab transport downloads and uploads encrypted envelopes', async () => {
  const { downloadGitLabVault, uploadGitLabVault } = await import('../src/syncTransports.ts');

  const mockFetchDownload = async () => {
    return new Response(validEnvelopeJson, {
      status: 200,
      headers: { 'x-gitlab-commit-id': 'gl_commit_123' },
    });
  };

  const dlResult = await downloadGitLabVault(
    { projectId: '42' },
    'glpat_test',
    { fetchImpl: mockFetchDownload }
  );
  assert.equal(dlResult.success, true);
  assert.equal(dlResult.remoteRevision, 'gl_commit_123');
  assert.equal(dlResult.envelope, validEnvelopeJson);

  const mockFetchUpload = async (url, init) => {
    assert.equal(init.method, 'PUT');
    return new Response(JSON.stringify({ file_path: 'moraxterm-vault.mvx' }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  };

  const upResult = await uploadGitLabVault(
    { projectId: '42' },
    'glpat_test',
    validEnvelopeJson,
    'gl_commit_123',
    { fetchImpl: mockFetchUpload }
  );
  assert.equal(upResult.success, true);
});

test('WebDAV transport handles GET and PUT with ETag and 412 Precondition Failed', async () => {
  const { downloadWebDAVVault, uploadWebDAVVault } = await import('../src/syncTransports.ts');

  const mockFetchDownload = async () => {
    return new Response(validEnvelopeJson, {
      status: 200,
      headers: { etag: '"etag-vault-1"' },
    });
  };

  const dlResult = await downloadWebDAVVault(
    { endpointUrl: 'https://dav.example.com/vault.mvx' },
    'Bearer token123',
    { fetchImpl: mockFetchDownload }
  );
  assert.equal(dlResult.success, true);
  assert.equal(dlResult.remoteRevision, '"etag-vault-1"');
  assert.equal(dlResult.envelope, validEnvelopeJson);

  // Successful upload with matching If-Match
  const mockFetchUpload = async (url, init) => {
    assert.equal(init.headers['If-Match'], '"etag-vault-1"');
    return new Response(null, { status: 204, headers: { etag: '"etag-vault-2"' } });
  };

  const upResult = await uploadWebDAVVault(
    { endpointUrl: 'https://dav.example.com/vault.mvx' },
    'Bearer token123',
    validEnvelopeJson,
    '"etag-vault-1"',
    { fetchImpl: mockFetchUpload }
  );
  assert.equal(upResult.success, true);
  assert.equal(upResult.remoteRevision, '"etag-vault-2"');

  // Conflict on stale ETag
  const mockFetch412 = async () => new Response('Precondition Failed', { status: 412 });
  const confResult = await uploadWebDAVVault(
    { endpointUrl: 'https://dav.example.com/vault.mvx' },
    'Bearer token123',
    validEnvelopeJson,
    '"stale-etag"',
    { fetchImpl: mockFetch412 }
  );
  assert.equal(confResult.success, false);
  assert.equal(confResult.conflict, true);
});

test('S3-compatible transport handles GET and PUT with ETag', async () => {
  const { downloadS3Vault, uploadS3Vault } = await import('../src/syncTransports.ts');

  const mockFetchDownload = async (url) => {
    assert.match(url, /https:\/\/s3\.example\.com\/my-bucket\/moraxterm-vault\.mvx/);
    return new Response(validEnvelopeJson, { status: 200, headers: { etag: '"s3-etag-1"' } });
  };

  const dlResult = await downloadS3Vault(
    { endpointUrl: 'https://s3.example.com', bucket: 'my-bucket' },
    'Bearer s3token',
    { fetchImpl: mockFetchDownload }
  );
  assert.equal(dlResult.success, true);
  assert.equal(dlResult.remoteRevision, '"s3-etag-1"');

  const mockFetchUpload = async () => new Response(null, { status: 200, headers: { etag: '"s3-etag-2"' } });
  const upResult = await uploadS3Vault(
    { endpointUrl: 'https://s3.example.com', bucket: 'my-bucket' },
    'Bearer s3token',
    validEnvelopeJson,
    '"s3-etag-1"',
    { fetchImpl: mockFetchUpload }
  );
  assert.equal(upResult.success, true);
  assert.equal(upResult.remoteRevision, '"s3-etag-2"');
});

test('Google Drive transport explicitly returns unverified contract error', async () => {
  const { downloadGoogleDriveVault, uploadGoogleDriveVault } = await import('../src/syncTransports.ts');
  const dl = await downloadGoogleDriveVault();
  assert.equal(dl.success, false);
  assert.equal(dl.statusCode, 501);
  assert.match(dl.error, /Google Drive sync is an unverified contract/);

  const up = await uploadGoogleDriveVault();
  assert.equal(up.success, false);
  assert.equal(up.statusCode, 501);
});

test('fetchWithRetry retries on 500/429 server errors and honors timeout', async () => {
  const { fetchWithRetry } = await import('../src/syncTransports.ts');

  let attempts = 0;
  const mockFlakyFetch = async () => {
    attempts++;
    if (attempts === 1) {
      return new Response('Busy', { status: 503 });
    }
    return new Response('OK', { status: 200 });
  };

  const res = await fetchWithRetry('https://example.com', {}, {
    fetchImpl: mockFlakyFetch,
    maxRetries: 2,
    backoffMs: 10,
  });
  assert.equal(res.status, 200);
  assert.equal(attempts, 2);

  // Fast timeout
  const mockSlowFetch = async (url, init) => {
    return new Promise((resolve, reject) => {
      if (init?.signal?.aborted) {
        return reject(new DOMException('This operation was aborted', 'AbortError'));
      }
      init?.signal?.addEventListener('abort', () => {
        reject(new DOMException('This operation was aborted', 'AbortError'));
      });
      setTimeout(() => resolve(new Response('Slow', { status: 200 })), 200);
    });
  };
  await assert.rejects(async () => {
    await fetchWithRetry('https://example.com', {}, {
      fetchImpl: mockSlowFetch,
      timeoutMs: 30,
      maxRetries: 0,
    });
  });
});

test('resolveConflict applies reject, keep-local, and replace-local policies', async () => {
  const { resolveConflict } = await import('../src/syncTransports.ts');
  const localEnv = validEnvelopeJson;
  const remoteEnv = JSON.stringify({
    version: 1,
    salt: 'salt_remote',
    nonce: 'nonce_remote',
    ciphertext: 'cipher_remote',
  });

  // Identical payloads always succeed
  assert.equal(resolveConflict(localEnv, localEnv, 'reject'), localEnv);

  // Conflict: reject throws
  assert.throws(() => resolveConflict(localEnv, remoteEnv, 'reject'), /Sync conflict/);

  // Conflict: keep-local preserves local
  assert.equal(resolveConflict(localEnv, remoteEnv, 'keep-local'), localEnv);

  // Conflict: replace-local replaces with remote
  assert.equal(resolveConflict(localEnv, remoteEnv, 'replace-local'), remoteEnv);
});

test('executeSync orchestrates dry-run and full sync workflows', async () => {
  const { executeSync, createMemoryKeyring, setKeyringStorage } = await import('../src/syncTransports.ts');
  const keyring = createMemoryKeyring();
  setKeyringStorage(keyring);

  // Not authenticated
  const unauthRes = await executeSync({
    providerId: 'github-repository',
    config: { owner: 'user', repo: 'vault' },
    localEnvelope: validEnvelopeJson,
    policy: 'reject',
  });
  assert.equal(unauthRes.success, false);
  assert.match(unauthRes.error, /Not authenticated/);

  // Store mock token
  await keyring.storeSecret('github-repository', 'ghp_token_abc');

  const mockFetch = async (url, init) => {
    if (init.method === 'GET') {
      return new Response(JSON.stringify({
        content: Buffer.from(validEnvelopeJson).toString('base64'),
        sha: 'sha_rev_1',
      }), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    if (init.method === 'PUT') {
      return new Response(JSON.stringify({
        content: { sha: 'sha_rev_2' },
      }), { status: 200, headers: { 'Content-Type': 'application/json' } });
    }
    throw new Error('Unexpected method');
  };

  // Dry run
  const dryRes = await executeSync({
    providerId: 'github-repository',
    config: { owner: 'user', repo: 'vault' },
    localEnvelope: validEnvelopeJson,
    policy: 'reject',
    dryRun: true,
    fetchOptions: { fetchImpl: mockFetch },
  });
  assert.equal(dryRes.success, true);
  assert.equal(dryRes.dryRun, true);
  assert.equal(dryRes.remoteRevision, 'sha_rev_1');

  // Full sync
  const fullRes = await executeSync({
    providerId: 'github-repository',
    config: { owner: 'user', repo: 'vault' },
    localEnvelope: validEnvelopeJson,
    policy: 'reject',
    dryRun: false,
    fetchOptions: { fetchImpl: mockFetch },
  });
  assert.equal(fullRes.success, true);
  assert.equal(fullRes.remoteRevision, 'sha_rev_2');
});
