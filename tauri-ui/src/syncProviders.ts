export type SyncProviderId =
  | 'local-only'
  | 'github-repository'
  | 'gitlab-repository'
  | 'webdav-object-store'
  | 's3-compatible'
  | 'google-drive';

export type ConflictPolicy = 'reject' | 'keep-local' | 'replace-local';

export interface SyncProvider {
  id: SyncProviderId;
  label: string;
  state: 'ready' | 'unavailable';
  detail: string;
}

export const syncProviders: SyncProvider[] = [
  {
    id: 'local-only',
    label: 'Local only',
    state: 'ready',
    detail: 'Encrypted vault stays on this device; no network calls.',
  },
  {
    id: 'github-repository',
    label: 'GitHub repository',
    state: 'ready',
    detail: 'Transfers encrypted vault envelopes via GitHub contents API with OS-keyring token auth.',
  },
  {
    id: 'gitlab-repository',
    label: 'GitLab repository',
    state: 'ready',
    detail: 'Transfers encrypted vault envelopes via GitLab repository API with OS-keyring token auth.',
  },
  {
    id: 'webdav-object-store',
    label: 'WebDAV storage',
    state: 'ready',
    detail: 'Transfers encrypted vault envelopes via WebDAV HTTP PUT/GET with ETag concurrency.',
  },
  {
    id: 's3-compatible',
    label: 'S3-compatible / network storage',
    state: 'ready',
    detail: 'Transfers encrypted vault envelopes via S3-compatible REST endpoint with ETag concurrency.',
  },
  {
    id: 'google-drive',
    label: 'Google Drive (contract)',
    state: 'unavailable',
    detail: 'Explicit contract: requires verified OAuth 2.0 PKCE implementation before activation.',
  },
];

export function providerStatus(id: SyncProviderId): SyncProvider {
  return syncProviders.find(item => item.id === id) ?? syncProviders[0];
}
