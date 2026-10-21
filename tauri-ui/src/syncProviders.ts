export type SyncProviderId = 'local-only' | 'github-repository' | 'gitlab-repository' | 'google-drive' | 'webdav-object-store';
export type ConflictPolicy = 'reject' | 'keep-local' | 'replace-local';
export interface SyncProvider { id: SyncProviderId; label: string; state: 'ready' | 'unavailable'; detail: string; }
export const syncProviders: SyncProvider[] = [
  { id: 'local-only', label: 'Local only', state: 'ready', detail: 'Encrypted vault stays on this device; no network calls.' },
  { id: 'github-repository', label: 'GitHub repository (contract)', state: 'unavailable', detail: 'Requires a reviewed API backend and OS-keyring OAuth token.' },
  { id: 'gitlab-repository', label: 'GitLab repository (contract)', state: 'unavailable', detail: 'Requires a reviewed API backend and OS-keyring OAuth token.' },
  { id: 'google-drive', label: 'Google Drive (contract)', state: 'unavailable', detail: 'Requires a reviewed OAuth flow and provider interoperability tests.' },
  { id: 'webdav-object-store', label: 'WebDAV/object store (contract)', state: 'unavailable', detail: 'Requires an explicit endpoint contract and authenticated transport.' },
];
export function providerStatus(id: SyncProviderId): SyncProvider { return syncProviders.find(item => item.id === id) ?? syncProviders[0]; }
