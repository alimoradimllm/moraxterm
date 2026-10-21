import { tauriInvoke } from './react-types.ts';
import type { ConflictPolicy, SyncProviderId } from './syncProviders.ts';

export interface EncryptedVaultEnvelope {
  version: number;
  salt: string;
  nonce: string;
  ciphertext: string;
}

/**
 * Validates that the payload is an authenticated encrypted vault envelope.
 * Strictly rejects any unencrypted sessions, keys, or plaintext secrets.
 */
export function assertEncryptedEnvelope(raw: string): EncryptedVaultEnvelope {
  if (typeof raw !== 'string' || !raw.trim()) {
    throw new Error('Vault payload cannot be empty');
  }

  // Safety check: explicit plaintext rejection
  if (
    raw.includes('"password"') ||
    raw.includes('"private_key"') ||
    raw.includes('"credentials"') ||
    (raw.includes('"sessions"') && !raw.includes('"ciphertext"'))
  ) {
    throw new Error('Unencrypted payload rejected: only sealed vault envelopes may be transmitted');
  }

  let text = raw.trim();
  // If base64-encoded, decode first
  if (!text.startsWith('{')) {
    try {
      text = atob(text);
    } catch {
      // not base64, proceed to json parse attempt
    }
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new Error(`Invalid vault envelope JSON: ${error instanceof Error ? error.message : String(error)}`);
  }

  if (!parsed || typeof parsed !== 'object') {
    throw new Error('Invalid vault envelope: root must be an object');
  }

  const obj = parsed as Record<string, unknown>;
  if (obj.version !== 1) {
    throw new Error(`Unsupported vault envelope version: ${String(obj.version)}; expected 1`);
  }

  if (
    typeof obj.salt !== 'string' || !obj.salt ||
    typeof obj.nonce !== 'string' || !obj.nonce ||
    typeof obj.ciphertext !== 'string' || !obj.ciphertext
  ) {
    throw new Error('Invalid vault envelope: missing required cryptographic envelope fields (salt, nonce, ciphertext)');
  }

  return {
    version: obj.version,
    salt: obj.salt,
    nonce: obj.nonce,
    ciphertext: obj.ciphertext,
  };
}

export interface SyncTransportResult {
  success: boolean;
  envelope?: string;
  remoteRevision?: string;
  error?: string;
  conflict?: boolean;
  statusCode?: number;
}

export interface GitHubSyncConfig {
  owner: string;
  repo: string;
  path?: string;
  branch?: string;
  baseUrl?: string;
}

export interface GitLabSyncConfig {
  projectId: string;
  path?: string;
  branch?: string;
  baseUrl?: string;
}

export interface WebDAVSyncConfig {
  endpointUrl: string;
}

export interface S3SyncConfig {
  endpointUrl: string;
  bucket: string;
  key?: string;
}

export type ProviderSyncConfig =
  | { provider: 'github-repository'; config: GitHubSyncConfig }
  | { provider: 'gitlab-repository'; config: GitLabSyncConfig }
  | { provider: 'webdav-object-store'; config: WebDAVSyncConfig }
  | { provider: 's3-compatible'; config: S3SyncConfig }
  | { provider: 'google-drive'; config: Record<string, unknown> }
  | { provider: 'local-only'; config: Record<string, unknown> };

export interface FetchRetryOptions {
  timeoutMs?: number;
  maxRetries?: number;
  backoffMs?: number;
  fetchImpl?: typeof fetch;
}

export async function fetchWithRetry(
  url: string,
  init: RequestInit,
  options: FetchRetryOptions = {}
): Promise<Response> {
  const timeoutMs = options.timeoutMs ?? 10000;
  const maxRetries = options.maxRetries ?? 2;
  const backoffMs = options.backoffMs ?? 100;
  const fetchFn = options.fetchImpl ?? globalThis.fetch;

  let attempt = 0;
  while (true) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);

    try {
      const response = await fetchFn(url, {
        ...init,
        signal: controller.signal,
      });
      clearTimeout(timer);

      // Retry on rate limit or server errors
      if (
        (response.status === 429 || (response.status >= 500 && response.status <= 504)) &&
        attempt < maxRetries
      ) {
        attempt++;
        await new Promise(r => setTimeout(r, backoffMs * attempt));
        continue;
      }

      return response;
    } catch (error) {
      clearTimeout(timer);
      if (attempt < maxRetries) {
        attempt++;
        await new Promise(r => setTimeout(r, backoffMs * attempt));
        continue;
      }
      throw error;
    }
  }
}

// OS Keyring abstraction
export interface KeyringStorage {
  getStatus(provider: string): Promise<boolean>;
  storeSecret(provider: string, secret: string): Promise<void>;
  deleteSecret(provider: string): Promise<boolean>;
  getSecret(provider: string): Promise<string>;
}

class TauriKeyringStorage implements KeyringStorage {
  async getStatus(provider: string): Promise<boolean> {
    try {
      return await tauriInvoke<boolean>('sync_credential_status', { provider });
    } catch {
      return false;
    }
  }
  async storeSecret(provider: string, secret: string): Promise<void> {
    await tauriInvoke<void>('sync_credential_store', { provider, secret });
  }
  async deleteSecret(provider: string): Promise<boolean> {
    try {
      return await tauriInvoke<boolean>('sync_credential_delete', { provider });
    } catch {
      return false;
    }
  }
  async getSecret(provider: string): Promise<string> {
    return await tauriInvoke<string>('sync_credential_get', { provider });
  }
}

export function createMemoryKeyring(): KeyringStorage {
  const store = new Map<string, string>();
  return {
    async getStatus(provider: string): Promise<boolean> {
      return store.has(provider.toLowerCase());
    },
    async storeSecret(provider: string, secret: string): Promise<void> {
      if (!secret.trim()) throw new Error('Secret cannot be empty');
      store.set(provider.toLowerCase(), secret.trim());
    },
    async deleteSecret(provider: string): Promise<boolean> {
      return store.delete(provider.toLowerCase());
    },
    async getSecret(provider: string): Promise<string> {
      const val = store.get(provider.toLowerCase());
      if (!val) throw new Error('not authenticated');
      return val;
    },
  };
}

let activeKeyring: KeyringStorage = new TauriKeyringStorage();
export function setKeyringStorage(storage: KeyringStorage): void {
  activeKeyring = storage;
}
export function getKeyringStorage(): KeyringStorage {
  return activeKeyring;
}

// ----------------- GitHub Transport -----------------
export async function downloadGitHubVault(
  config: GitHubSyncConfig,
  token: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  const baseUrl = config.baseUrl || 'https://api.github.com';
  const path = config.path || 'moraxterm-vault.mvx';
  const branch = config.branch || 'main';
  const url = `${baseUrl}/repos/${config.owner}/${config.repo}/contents/${path}?ref=${branch}`;

  const res = await fetchWithRetry(
    url,
    {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'User-Agent': 'Moraxterm-Sync',
      },
    },
    options
  );

  if (res.status === 404) {
    return { success: true, envelope: undefined, remoteRevision: undefined, statusCode: 404 };
  }
  if (!res.ok) {
    return {
      success: false,
      error: `GitHub download failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const data = (await res.json()) as { content?: string; encoding?: string; sha?: string };
  if (!data.content) {
    return { success: false, error: 'GitHub returned no file content', statusCode: res.status };
  }

  const rawEnvelope = atob(data.content.replace(/\s/g, ''));
  assertEncryptedEnvelope(rawEnvelope);

  return {
    success: true,
    envelope: rawEnvelope,
    remoteRevision: data.sha,
    statusCode: res.status,
  };
}

export async function uploadGitHubVault(
  config: GitHubSyncConfig,
  token: string,
  envelope: string,
  expectedRevision?: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  assertEncryptedEnvelope(envelope);

  const baseUrl = config.baseUrl || 'https://api.github.com';
  const path = config.path || 'moraxterm-vault.mvx';
  const branch = config.branch || 'main';
  const url = `${baseUrl}/repos/${config.owner}/${config.repo}/contents/${path}`;

  const body: Record<string, unknown> = {
    message: 'Update encrypted Moraxterm vault',
    content: btoa(envelope),
    branch,
  };
  if (expectedRevision) {
    body.sha = expectedRevision;
  }

  const res = await fetchWithRetry(
    url,
    {
      method: 'PUT',
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: 'application/vnd.github+json',
        'Content-Type': 'application/json',
        'User-Agent': 'Moraxterm-Sync',
      },
      body: JSON.stringify(body),
    },
    options
  );

  if (res.status === 409) {
    return {
      success: false,
      conflict: true,
      error: 'Conflict: remote GitHub repository file was modified concurrently',
      statusCode: 409,
    };
  }

  if (!res.ok) {
    return {
      success: false,
      error: `GitHub upload failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const data = (await res.json()) as { content?: { sha?: string } };
  return {
    success: true,
    remoteRevision: data.content?.sha,
    statusCode: res.status,
  };
}

// ----------------- GitLab Transport -----------------
export async function downloadGitLabVault(
  config: GitLabSyncConfig,
  token: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  const baseUrl = config.baseUrl || 'https://gitlab.com';
  const path = encodeURIComponent(config.path || 'moraxterm-vault.mvx');
  const project = encodeURIComponent(config.projectId);
  const branch = config.branch || 'main';
  const url = `${baseUrl}/api/v4/projects/${project}/repository/files/${path}/raw?ref=${branch}`;

  const res = await fetchWithRetry(
    url,
    {
      method: 'GET',
      headers: {
        'PRIVATE-TOKEN': token,
        'User-Agent': 'Moraxterm-Sync',
      },
    },
    options
  );

  if (res.status === 404) {
    return { success: true, envelope: undefined, remoteRevision: undefined, statusCode: 404 };
  }
  if (!res.ok) {
    return {
      success: false,
      error: `GitLab download failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const rawEnvelope = await res.text();
  assertEncryptedEnvelope(rawEnvelope);
  const commitId = res.headers.get('x-gitlab-commit-id') || res.headers.get('x-gitlab-blob-id') || res.headers.get('etag') || undefined;

  return {
    success: true,
    envelope: rawEnvelope,
    remoteRevision: commitId,
    statusCode: res.status,
  };
}

export async function uploadGitLabVault(
  config: GitLabSyncConfig,
  token: string,
  envelope: string,
  expectedRevision?: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  assertEncryptedEnvelope(envelope);

  const baseUrl = config.baseUrl || 'https://gitlab.com';
  const path = encodeURIComponent(config.path || 'moraxterm-vault.mvx');
  const project = encodeURIComponent(config.projectId);
  const branch = config.branch || 'main';
  const url = `${baseUrl}/api/v4/projects/${project}/repository/files/${path}`;

  const body: Record<string, unknown> = {
    branch,
    commit_message: 'Update encrypted Moraxterm vault',
    content: envelope,
  };
  if (expectedRevision) {
    body.last_commit_id = expectedRevision;
  }

  const method = expectedRevision ? 'PUT' : 'POST';
  let res = await fetchWithRetry(
    url,
    {
      method,
      headers: {
        'PRIVATE-TOKEN': token,
        'Content-Type': 'application/json',
        'User-Agent': 'Moraxterm-Sync',
      },
      body: JSON.stringify(body),
    },
    options
  );

  // If initial POST failed because file already exists, fallback to PUT
  if (!expectedRevision && res.status === 400) {
    res = await fetchWithRetry(
      url,
      {
        method: 'PUT',
        headers: {
          'PRIVATE-TOKEN': token,
          'Content-Type': 'application/json',
          'User-Agent': 'Moraxterm-Sync',
        },
        body: JSON.stringify(body),
      },
      options
    );
  }

  if (res.status === 400 || res.status === 409) {
    const errorText = await res.text();
    if (errorText.toLowerCase().includes('modified') || errorText.toLowerCase().includes('conflict')) {
      return {
        success: false,
        conflict: true,
        error: 'Conflict: remote GitLab repository file was modified concurrently',
        statusCode: res.status,
      };
    }
  }

  if (!res.ok) {
    return {
      success: false,
      error: `GitLab upload failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const data = (await res.json()) as Record<string, unknown>;
  return {
    success: true,
    remoteRevision: (data.file_path || data.branch || 'updated') as string,
    statusCode: res.status,
  };
}

// ----------------- WebDAV Transport -----------------
export async function downloadWebDAVVault(
  config: WebDAVSyncConfig,
  authHeader: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  const res = await fetchWithRetry(
    config.endpointUrl,
    {
      method: 'GET',
      headers: {
        Authorization: authHeader,
        'User-Agent': 'Moraxterm-Sync',
      },
    },
    options
  );

  if (res.status === 404) {
    return { success: true, envelope: undefined, remoteRevision: undefined, statusCode: 404 };
  }
  if (!res.ok) {
    return {
      success: false,
      error: `WebDAV download failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const rawEnvelope = await res.text();
  assertEncryptedEnvelope(rawEnvelope);
  const etag = res.headers.get('etag') || res.headers.get('last-modified') || undefined;

  return {
    success: true,
    envelope: rawEnvelope,
    remoteRevision: etag,
    statusCode: res.status,
  };
}

export async function uploadWebDAVVault(
  config: WebDAVSyncConfig,
  authHeader: string,
  envelope: string,
  expectedRevision?: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  assertEncryptedEnvelope(envelope);

  const headers: Record<string, string> = {
    Authorization: authHeader,
    'Content-Type': 'application/octet-stream',
    'User-Agent': 'Moraxterm-Sync',
  };
  if (expectedRevision) {
    headers['If-Match'] = expectedRevision;
  }

  const res = await fetchWithRetry(
    config.endpointUrl,
    {
      method: 'PUT',
      headers,
      body: envelope,
    },
    options
  );

  if (res.status === 412 || res.status === 409) {
    return {
      success: false,
      conflict: true,
      error: 'Conflict: remote WebDAV file was modified concurrently (precondition failed)',
      statusCode: res.status,
    };
  }

  if (!res.ok && res.status !== 201 && res.status !== 204) {
    return {
      success: false,
      error: `WebDAV upload failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const newEtag = res.headers.get('etag') || undefined;
  return {
    success: true,
    remoteRevision: newEtag,
    statusCode: res.status,
  };
}

// ----------------- S3-Compatible Transport -----------------
export async function downloadS3Vault(
  config: S3SyncConfig,
  authHeader: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  const key = config.key || 'moraxterm-vault.mvx';
  const url = `${config.endpointUrl.replace(/\/$/, '')}/${config.bucket}/${key}`;

  const res = await fetchWithRetry(
    url,
    {
      method: 'GET',
      headers: {
        Authorization: authHeader,
        'User-Agent': 'Moraxterm-Sync',
      },
    },
    options
  );

  if (res.status === 404) {
    return { success: true, envelope: undefined, remoteRevision: undefined, statusCode: 404 };
  }
  if (!res.ok) {
    return {
      success: false,
      error: `S3 download failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const rawEnvelope = await res.text();
  assertEncryptedEnvelope(rawEnvelope);
  const etag = res.headers.get('etag') || undefined;

  return {
    success: true,
    envelope: rawEnvelope,
    remoteRevision: etag,
    statusCode: res.status,
  };
}

export async function uploadS3Vault(
  config: S3SyncConfig,
  authHeader: string,
  envelope: string,
  expectedRevision?: string,
  options?: FetchRetryOptions
): Promise<SyncTransportResult> {
  assertEncryptedEnvelope(envelope);

  const key = config.key || 'moraxterm-vault.mvx';
  const url = `${config.endpointUrl.replace(/\/$/, '')}/${config.bucket}/${key}`;

  const headers: Record<string, string> = {
    Authorization: authHeader,
    'Content-Type': 'application/octet-stream',
    'User-Agent': 'Moraxterm-Sync',
  };
  if (expectedRevision) {
    headers['If-Match'] = expectedRevision;
  }

  const res = await fetchWithRetry(
    url,
    {
      method: 'PUT',
      headers,
      body: envelope,
    },
    options
  );

  if (res.status === 412 || res.status === 409) {
    return {
      success: false,
      conflict: true,
      error: 'Conflict: remote S3 object was modified concurrently',
      statusCode: res.status,
    };
  }

  if (!res.ok) {
    return {
      success: false,
      error: `S3 upload failed (${res.status}): ${await res.text()}`,
      statusCode: res.status,
    };
  }

  const newEtag = res.headers.get('etag') || undefined;
  return {
    success: true,
    remoteRevision: newEtag,
    statusCode: res.status,
  };
}

// ----------------- Google Drive Transport -----------------
export async function downloadGoogleDriveVault(): Promise<SyncTransportResult> {
  return {
    success: false,
    error: 'Google Drive sync is an unverified contract: OAuth PKCE flow and drive scopes require manual authorization',
    statusCode: 501,
  };
}

export async function uploadGoogleDriveVault(): Promise<SyncTransportResult> {
  return {
    success: false,
    error: 'Google Drive sync is an unverified contract: OAuth PKCE flow and drive scopes require manual authorization',
    statusCode: 501,
  };
}

// ----------------- Conflict Resolution & Execution -----------------
export function resolveConflict(
  localEnvelope: string,
  remoteEnvelope: string,
  policy: ConflictPolicy
): string {
  if (localEnvelope === remoteEnvelope) {
    return localEnvelope;
  }
  switch (policy) {
    case 'reject':
      throw new Error('Sync conflict: remote vault was updated concurrently. Policy is set to reject.');
    case 'keep-local':
      return localEnvelope;
    case 'replace-local':
      return remoteEnvelope;
    default:
      throw new Error(`Unknown conflict policy: ${String(policy)}`);
  }
}

export interface ExecuteSyncParams {
  providerId: SyncProviderId;
  config: Record<string, unknown>;
  localEnvelope: string;
  policy: ConflictPolicy;
  expectedRevision?: string;
  dryRun?: boolean;
  fetchOptions?: FetchRetryOptions;
}

export interface ExecuteSyncResult {
  success: boolean;
  syncedEnvelope?: string;
  remoteRevision?: string;
  conflict?: boolean;
  dryRun?: boolean;
  error?: string;
}

export async function executeSync(params: ExecuteSyncParams): Promise<ExecuteSyncResult> {
  const { providerId, config, localEnvelope, policy, expectedRevision, dryRun, fetchOptions } = params;

  if (providerId === 'local-only') {
    return {
      success: true,
      syncedEnvelope: localEnvelope,
      dryRun,
    };
  }

  if (providerId === 'google-drive') {
    return {
      success: false,
      error: 'Google Drive is an unverified contract: requires verified OAuth 2.0 PKCE implementation before activation',
    };
  }

  // Validate local envelope before attempting any network transport
  try {
    assertEncryptedEnvelope(localEnvelope);
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }

  // Check keyring authentication
  const keyring = getKeyringStorage();
  let secret: string;
  try {
    secret = await keyring.getSecret(providerId);
  } catch {
    return {
      success: false,
      error: `Not authenticated: no credentials found in OS keyring for provider '${providerId}'`,
    };
  }

  // Step 1: Download current remote state
  let downloadRes: SyncTransportResult;
  switch (providerId) {
    case 'github-repository':
      downloadRes = await downloadGitHubVault(config as unknown as GitHubSyncConfig, secret, fetchOptions);
      break;
    case 'gitlab-repository':
      downloadRes = await downloadGitLabVault(config as unknown as GitLabSyncConfig, secret, fetchOptions);
      break;
    case 'webdav-object-store':
      downloadRes = await downloadWebDAVVault(config as unknown as WebDAVSyncConfig, secret, fetchOptions);
      break;
    case 's3-compatible':
      downloadRes = await downloadS3Vault(config as unknown as S3SyncConfig, secret, fetchOptions);
      break;
    default:
      return { success: false, error: `Unsupported provider: ${providerId}` };
  }

  if (!downloadRes.success) {
    return {
      success: false,
      error: downloadRes.error,
    };
  }

  if (dryRun) {
    return {
      success: true,
      dryRun: true,
      remoteRevision: downloadRes.remoteRevision,
      syncedEnvelope: downloadRes.envelope || localEnvelope,
    };
  }

  // If remote has an envelope and it differs from local
  let targetEnvelope = localEnvelope;
  if (downloadRes.envelope) {
    if (expectedRevision && downloadRes.remoteRevision !== expectedRevision) {
      try {
        targetEnvelope = resolveConflict(localEnvelope, downloadRes.envelope, policy);
      } catch (err) {
        return {
          success: false,
          conflict: true,
          error: err instanceof Error ? err.message : String(err),
        };
      }
    }
  }

  // If local wins or remote did not exist, upload targetEnvelope
  if (targetEnvelope === localEnvelope) {
    let uploadRes: SyncTransportResult;
    switch (providerId) {
      case 'github-repository':
        uploadRes = await uploadGitHubVault(config as unknown as GitHubSyncConfig, secret, targetEnvelope, downloadRes.remoteRevision, fetchOptions);
        break;
      case 'gitlab-repository':
        uploadRes = await uploadGitLabVault(config as unknown as GitLabSyncConfig, secret, targetEnvelope, downloadRes.remoteRevision, fetchOptions);
        break;
      case 'webdav-object-store':
        uploadRes = await uploadWebDAVVault(config as unknown as WebDAVSyncConfig, secret, targetEnvelope, downloadRes.remoteRevision, fetchOptions);
        break;
      case 's3-compatible':
        uploadRes = await uploadS3Vault(config as unknown as S3SyncConfig, secret, targetEnvelope, downloadRes.remoteRevision, fetchOptions);
        break;
      default:
        return { success: false, error: `Unsupported provider: ${providerId}` };
    }

    if (!uploadRes.success) {
      return {
        success: false,
        conflict: uploadRes.conflict,
        error: uploadRes.error,
      };
    }

    return {
      success: true,
      syncedEnvelope: targetEnvelope,
      remoteRevision: uploadRes.remoteRevision,
    };
  }

  // Remote wins under replace-local
  return {
    success: true,
    syncedEnvelope: targetEnvelope,
    remoteRevision: downloadRes.remoteRevision,
  };
}
