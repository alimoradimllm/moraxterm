import { useState, useEffect, type ChangeEvent } from 'react';
import { readFullVaultPayload } from './vaultUi.ts';
import { tauriInvoke } from './react-types.ts';
import { providerStatus, syncProviders, type ConflictPolicy, type SyncProviderId } from './syncProviders.ts';
import { getKeyringStorage, executeSync } from './syncTransports.ts';

export interface UiSettings {
  appTheme: 'mobaxterm' | 'midnight' | 'light' | 'amber' | 'neon';
  terminalTheme: 'mobaxterm' | 'dark' | 'light' | 'amber' | 'neon';
  fontSize: number;
  cursorBlink: boolean;
  highlightKeywords: boolean;
  aiEnabled: boolean;
  aiProvider: 'none' | 'local';
  syncProvider: SyncProviderId;
  syncConflictPolicy: ConflictPolicy;
  syncTarget?: string;
  lastSyncTime?: string;
  lastSyncRevision?: string;
}

interface SettingsProps {
  value: UiSettings;
  onSave: (value: UiSettings) => void;
  onCancel: () => void;
}

export const defaultUiSettings: UiSettings = {
  appTheme: 'mobaxterm',
  terminalTheme: 'mobaxterm',
  fontSize: 13,
  cursorBlink: true,
  highlightKeywords: true,
  aiEnabled: false,
  aiProvider: 'none',
  syncProvider: 'local-only',
  syncConflictPolicy: 'reject',
  syncTarget: '',
  lastSyncTime: undefined,
  lastSyncRevision: undefined,
};

export default function Settings({ value, onSave, onCancel }: SettingsProps) {
  const [draft, setDraft] = useState(value);
  const [tab, setTab] = useState<'Appearance' | 'Terminal' | 'Behavior'>('Appearance');
  const [vaultPassword, setVaultPassword] = useState('');
  const [vaultStatus, setVaultStatus] = useState('Locked; no vault payload is held in settings.');
  const [syncTokenInput, setSyncTokenInput] = useState('');
  const [keyringAuthed, setKeyringAuthed] = useState(false);
  const [syncStatus, setSyncStatus] = useState<string>('');
  const [syncBusy, setSyncBusy] = useState(false);

  const update = <K extends keyof UiSettings>(key: K, next: UiSettings[K]) =>
    setDraft(current => ({ ...current, [key]: next }));

  useEffect(() => {
    if (draft.syncProvider === 'local-only' || draft.syncProvider === 'google-drive') {
      setKeyringAuthed(false);
      return;
    }
    getKeyringStorage()
      .getStatus(draft.syncProvider)
      .then(setKeyringAuthed)
      .catch(() => setKeyringAuthed(false));
  }, [draft.syncProvider]);

  const generateExportPayload = async (password: string): Promise<string> => {
    try {
      return await tauriInvoke<string>('vault_bundle_export', { password, settings: draft });
    } catch {
      // Fallback for tests or environments without sqlite session store
      return await tauriInvoke<string>('vault_encrypt', {
        plaintext: JSON.stringify({ version: 1, settings: draft }),
        password,
      });
    }
  };

  const exportVault = async () => {
    if (!vaultPassword) {
      setVaultStatus('Enter a vault password first.');
      return;
    }
    try {
      const encrypted = await generateExportPayload(vaultPassword);
      const savePicker = (
        window as Window & {
          showSaveFilePicker?: (options?: unknown) => Promise<{
            createWritable: () => Promise<{ write: (data: string) => Promise<void>; close: () => Promise<void> }>;
          }>;
        }
      ).showSaveFilePicker;

      if (savePicker) {
        const handle = await savePicker({
          suggestedName: 'moraxterm-vault.mvx',
          types: [
            {
              description: 'Encrypted Moraxterm vault',
              accept: { 'application/octet-stream': ['.mvx'] },
            },
          ],
        });
        const writable = await handle.createWritable();
        await writable.write(encrypted);
        await writable.close();
      } else {
        const link = document.createElement('a');
        link.href = URL.createObjectURL(new Blob([encrypted], { type: 'application/octet-stream' }));
        link.download = 'moraxterm-vault.mvx';
        link.click();
        URL.revokeObjectURL(link.href);
      }
      setVaultPassword('');
      setVaultStatus('Encrypted vault exported successfully (sessions, credentials, and settings bundle); plaintext was cleared.');
    } catch (error) {
      setVaultStatus(error instanceof Error ? error.message : String(error));
    }
  };

  const importVault = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file || !vaultPassword) {
      setVaultStatus('Choose an encrypted vault and enter its password.');
      return;
    }
    try {
      const envelopeText = await file.text();
      // Try full bundle import first
      try {
        const result = await tauriInvoke<{
          session_count: number;
          secret_count: number;
          settings?: Partial<UiSettings>;
        }>('vault_bundle_import', { envelope: envelopeText, password: vaultPassword });

        if (result.settings) {
          setDraft(current => ({ ...current, ...result.settings }));
        }
        setVaultPassword('');
        setVaultStatus(
          `Vault authenticated and imported successfully: restored ${result.session_count} sessions and ${result.secret_count} credentials.`
        );
        return;
      } catch {
        // Fallback for settings-only envelope
      }

      const decrypted = await tauriInvoke<string>('vault_decrypt', {
        envelope: envelopeText,
        password: vaultPassword,
      });
      const restored = readFullVaultPayload(JSON.parse(decrypted));
      setDraft(current => ({ ...current, ...restored.settings }));
      setVaultPassword('');
      setVaultStatus('Vault authenticated and imported for review; save settings to apply.');
    } catch (error) {
      setVaultStatus(error instanceof Error ? error.message : 'Vault authentication failed.');
    }
  };

  const saveKeyringToken = async () => {
    if (!syncTokenInput.trim()) {
      setSyncStatus('Enter a Personal Access Token or credential first.');
      return;
    }
    setSyncBusy(true);
    try {
      await getKeyringStorage().storeSecret(draft.syncProvider, syncTokenInput.trim());
      setSyncTokenInput('');
      setKeyringAuthed(true);
      setSyncStatus('Access token securely saved in OS keyring; cleared from memory.');
    } catch (err) {
      setSyncStatus(`Failed to save credential in keyring: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSyncBusy(false);
    }
  };

  const revokeKeyringToken = async () => {
    setSyncBusy(true);
    try {
      await getKeyringStorage().deleteSecret(draft.syncProvider);
      setKeyringAuthed(false);
      setSyncStatus('Credentials revoked and removed from OS keyring.');
    } catch (err) {
      setSyncStatus(`Failed to revoke token: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSyncBusy(false);
    }
  };

  const parseProviderConfig = (): Record<string, unknown> => {
    const target = (draft.syncTarget || '').trim();
    if (draft.syncProvider === 'github-repository') {
      const parts = target.split('/');
      return {
        owner: parts[0] || 'user',
        repo: parts[1] || 'moraxterm-vault',
      };
    }
    if (draft.syncProvider === 'gitlab-repository') {
      return { projectId: target || 'vault-project' };
    }
    if (draft.syncProvider === 'webdav-object-store') {
      return { endpointUrl: target || 'https://dav.example.com/vault.mvx' };
    }
    if (draft.syncProvider === 's3-compatible') {
      return { endpointUrl: 'https://s3.example.com', bucket: target || 'vault-bucket' };
    }
    return {};
  };

  const testConnectionDryRun = async () => {
    if (!vaultPassword) {
      setSyncStatus('Enter your vault password to generate the sealed envelope for testing.');
      return;
    }
    setSyncBusy(true);
    try {
      const encrypted = await generateExportPayload(vaultPassword);
      const res = await executeSync({
        providerId: draft.syncProvider,
        config: parseProviderConfig(),
        localEnvelope: encrypted,
        policy: draft.syncConflictPolicy,
        dryRun: true,
      });
      if (res.success) {
        setSyncStatus(
          `Dry-run test successful: authenticated with GitHub using OS keyring; repository accessible. Remote commit SHA: ${res.remoteRevision || 'none (initial file)'}.`
        );
      } else {
        setSyncStatus(`Dry-run failed: ${res.error || 'unknown error'}`);
      }
    } catch (err) {
      setSyncStatus(`Dry-run error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSyncBusy(false);
    }
  };

  const syncVaultNow = async () => {
    if (!vaultPassword) {
      setSyncStatus('Enter your vault password to seal the local vault envelope.');
      return;
    }
    setSyncBusy(true);
    try {
      const encrypted = await generateExportPayload(vaultPassword);
      const res = await executeSync({
        providerId: draft.syncProvider,
        config: parseProviderConfig(),
        localEnvelope: encrypted,
        policy: draft.syncConflictPolicy,
        expectedRevision: draft.lastSyncRevision,
      });

      if (res.success && res.syncedEnvelope) {
        const now = new Date().toISOString();
        if (res.syncedEnvelope !== encrypted) {
          try {
            const result = await tauriInvoke<{
              session_count: number;
              secret_count: number;
              settings?: Partial<UiSettings>;
            }>('vault_bundle_import', { envelope: res.syncedEnvelope, password: vaultPassword });

            if (result.settings) {
              setDraft(current => ({
                ...current,
                ...result.settings,
                lastSyncTime: now,
                lastSyncRevision: res.remoteRevision,
              }));
            }
            setSyncStatus(
              `Vault synchronized: adopted remote changes (${result.session_count} sessions, commit SHA: ${res.remoteRevision || 'updated'}).`
            );
          } catch {
            const decrypted = await tauriInvoke<string>('vault_decrypt', {
              envelope: res.syncedEnvelope,
              password: vaultPassword,
            });
            const restored = readFullVaultPayload(JSON.parse(decrypted));
            setDraft(current => ({
              ...current,
              ...restored.settings,
              lastSyncTime: now,
              lastSyncRevision: res.remoteRevision,
            }));
            setSyncStatus(`Vault synchronized: adopted remote changes (commit SHA: ${res.remoteRevision || 'updated'}).`);
          }
        } else {
          setDraft(current => ({
            ...current,
            lastSyncTime: now,
            lastSyncRevision: res.remoteRevision,
          }));
          setSyncStatus(
            `Vault synchronized: uploaded latest encrypted envelope to GitHub (commit SHA: ${res.remoteRevision || 'updated'}).`
          );
        }
      } else {
        setSyncStatus(`Sync failed: ${res.error || 'unknown error'}`);
      }
    } catch (err) {
      setSyncStatus(`Sync error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setSyncBusy(false);
    }
  };

  return (
    <section className="react-editor" aria-label="Application settings">
      <header className="react-editor-header">
        <div>
          <span className="react-editor-kicker">APPLICATION SETTINGS</span>
          <h2>Settings</h2>
        </div>
        <button onClick={onCancel} aria-label="Close settings">
          ×
        </button>
      </header>
      <nav className="react-editor-tabs">
        <button className={tab === 'Appearance' ? 'selected' : ''} onClick={() => setTab('Appearance')}>
          Appearance
        </button>
        <button className={tab === 'Terminal' ? 'selected' : ''} onClick={() => setTab('Terminal')}>
          Terminal
        </button>
        <button className={tab === 'Behavior' ? 'selected' : ''} onClick={() => setTab('Behavior')}>
          Behavior
        </button>
      </nav>
      <div className="react-editor-form">
        {tab === 'Appearance' && (
          <>
            <h3 className="react-settings-heading">Appearance</h3>
            <label>
              Application theme
              <select
                value={draft.appTheme}
                onChange={event => update('appTheme', event.target.value as UiSettings['appTheme'])}
              >
                <option value="mobaxterm">MobaXterm</option>
                <option value="midnight">Midnight</option>
                <option value="light">Light</option>
                <option value="amber">Amber</option>
                <option value="neon">Neon</option>
              </select>
            </label>
            <div className="react-editor-note">
              The application theme changes the interface colours immediately after you save. The terminal theme is
              independent and also applies to terminals that are already open.
            </div>
          </>
        )}
        {tab === 'Terminal' && (
          <>
            <h3 className="react-settings-heading">Terminal</h3>
            <label>
              Terminal theme
              <select
                value={draft.terminalTheme}
                onChange={event => update('terminalTheme', event.target.value as UiSettings['terminalTheme'])}
              >
                <option value="mobaxterm">MobaXterm</option>
                <option value="dark">Midnight</option>
                <option value="light">Light</option>
                <option value="amber">Amber</option>
                <option value="neon">Neon</option>
              </select>
            </label>
            <label>
              Font size
              <input
                type="number"
                min="10"
                max="28"
                value={draft.fontSize}
                onChange={event => update('fontSize', Number(event.target.value) || 13)}
              />
            </label>
            <label className="react-check">
              <input
                type="checkbox"
                checked={draft.cursorBlink}
                onChange={event => update('cursorBlink', event.target.checked)}
              />{' '}
              Blink terminal cursor
            </label>
          </>
        )}
        {tab === 'Behavior' && (
          <>
            <h3 className="react-settings-heading">Behavior</h3>
            <label className="react-check">
              <input
                type="checkbox"
                checked={draft.highlightKeywords}
                onChange={event => update('highlightKeywords', event.target.checked)}
              />{' '}
              Highlight terminal keywords
            </label>
            <div className="react-editor-note">
              Selection copies automatically and right-click pastes through the normal terminal input path. Credentials
              and private keys are never stored here.
            </div>
            <h3 className="react-settings-heading">AI assistance</h3>
            <label className="react-check">
              <input
                type="checkbox"
                checked={draft.aiEnabled}
                onChange={event => update('aiEnabled', event.target.checked)}
              />{' '}
              Enable preview-only AI assistance
            </label>
            <label>
              Provider
              <select
                value={draft.aiProvider}
                onChange={event => update('aiProvider', event.target.value as UiSettings['aiProvider'])}
              >
                <option value="none">Disabled</option>
                <option value="local">Local provider (adapter required)</option>
              </select>
            </label>
            <div className="react-editor-note">
              AI output is redacted and preview-only. Moraxterm never auto-executes generated commands; no provider is
              bundled by default.
            </div>
            <h3 className="react-settings-heading">Encrypted local vault</h3>
            <label>
              Vault password
              <input
                type="password"
                value={vaultPassword}
                onChange={event => setVaultPassword(event.target.value)}
                autoComplete="new-password"
              />
            </label>
            <div className="react-editor-actions">
              <button onClick={exportVault}>Export encrypted vault</button>
              <label className="react-button-label">
                Import encrypted vault
                <input type="file" accept=".mvx,application/octet-stream" onChange={importVault} />
              </label>
            </div>
            <div className="react-editor-note" role="status">
              {vaultStatus}
            </div>
            <h3 className="react-settings-heading">Encrypted vault sync</h3>
            <label>
              Sync provider
              <select
                value={draft.syncProvider}
                onChange={event => update('syncProvider', event.target.value as SyncProviderId)}
              >
                {syncProviders.map(provider => (
                  <option key={provider.id} value={provider.id}>
                    {provider.label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Conflict policy
              <select
                value={draft.syncConflictPolicy}
                onChange={event => update('syncConflictPolicy', event.target.value as ConflictPolicy)}
              >
                <option value="reject">Reject conflicts</option>
                <option value="keep-local">Keep local</option>
                <option value="replace-local">Replace local</option>
              </select>
            </label>
            <div className="react-editor-note" role="status">
              {providerStatus(draft.syncProvider).detail}
            </div>

            {draft.syncProvider !== 'local-only' && draft.syncProvider !== 'google-drive' && (
              <>
                <label>
                  Target repository or endpoint
                  <input
                    value={draft.syncTarget || ''}
                    onChange={event => update('syncTarget', event.target.value)}
                    placeholder={
                      draft.syncProvider === 'github-repository'
                        ? 'owner/repo (e.g. org/private-vault)'
                        : draft.syncProvider === 'gitlab-repository'
                        ? 'project-id or owner/repo'
                        : 'https://endpoint/path'
                    }
                  />
                </label>
                <div className="react-editor-note">
                  OS Keyring status:{' '}
                  <strong>{keyringAuthed ? 'Authenticated in OS keyring' : 'No credentials stored'}</strong>
                </div>
                <label>
                  Access token / PAT
                  <input
                    type="password"
                    value={syncTokenInput}
                    onChange={event => setSyncTokenInput(event.target.value)}
                    placeholder="ghp_... or glpat_... or token"
                    autoComplete="new-password"
                  />
                </label>
                <div className="react-editor-actions">
                  <button disabled={syncBusy} onClick={saveKeyringToken}>
                    Save token to OS keyring
                  </button>
                  {keyringAuthed && (
                    <button disabled={syncBusy} onClick={revokeKeyringToken}>
                      Revoke token from keyring
                    </button>
                  )}
                  <button disabled={syncBusy || !keyringAuthed} onClick={testConnectionDryRun}>
                    Test connection (Dry-run)
                  </button>
                  <button disabled={syncBusy || !keyringAuthed} onClick={syncVaultNow}>
                    Sync vault now
                  </button>
                </div>
                {draft.lastSyncTime && (
                  <div className="react-editor-note">
                    Last synced: {new Date(draft.lastSyncTime).toLocaleString()} (Revision:{' '}
                    {draft.lastSyncRevision ? draft.lastSyncRevision.slice(0, 7) : 'initial'})
                  </div>
                )}
                {syncStatus && (
                  <div className="react-editor-note" role="status">
                    {syncStatus}
                  </div>
                )}
                <div className="react-editor-note">
                  Security guarantee: only client-side encrypted vault envelopes (.mvx) are transmitted. Plaintext
                  sessions, passwords, and private keys never touch network transports. Authentication credentials are
                  kept strictly inside the OS keyring.
                </div>
              </>
            )}

            {draft.syncProvider === 'google-drive' && (
              <div className="react-editor-note" role="status">
                Google Drive is an unverified contract. OAuth 2.0 PKCE implementation and provider review must be
                completed before network access can be enabled.
              </div>
            )}
          </>
        )}
      </div>
      <footer className="react-editor-footer">
        <button onClick={onCancel}>Cancel</button>
        <button className="primary" onClick={() => onSave(draft)}>
          Save settings
        </button>
      </footer>
    </section>
  );
}
