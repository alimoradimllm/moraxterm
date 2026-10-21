import { useState, type ChangeEvent } from 'react';
import { readVaultPayload } from './vaultUi';
import { tauriInvoke } from './react-types';

export interface UiSettings {
  appTheme: 'mobaxterm' | 'midnight' | 'light' | 'amber' | 'neon';
  terminalTheme: 'mobaxterm' | 'dark' | 'light' | 'amber' | 'neon';
  fontSize: number;
  cursorBlink: boolean;
  highlightKeywords: boolean;
  aiEnabled: boolean;
  aiProvider: 'none' | 'local';
}

interface SettingsProps {
  value: UiSettings;
  onSave: (value: UiSettings) => void;
  onCancel: () => void;
}

export const defaultUiSettings: UiSettings = {
  appTheme: 'mobaxterm', terminalTheme: 'mobaxterm', fontSize: 13, cursorBlink: true, highlightKeywords: true, aiEnabled: false, aiProvider: 'none',
};

export default function Settings({ value, onSave, onCancel }: SettingsProps) {
  const [draft, setDraft] = useState(value);
  const [tab, setTab] = useState<'Appearance' | 'Terminal' | 'Behavior'>('Appearance');
  const [vaultPassword, setVaultPassword] = useState('');
  const [vaultStatus, setVaultStatus] = useState('Locked; no vault payload is held in settings.');
  const update = <K extends keyof UiSettings>(key: K, next: UiSettings[K]) => setDraft(current => ({ ...current, [key]: next }));
  const exportVault = async () => {
    if (!vaultPassword) { setVaultStatus('Enter a vault password first.'); return; }
    try {
      const encrypted = await tauriInvoke<string>('vault_encrypt', { plaintext: JSON.stringify({ version: 1, settings: draft }), password: vaultPassword });
      const link = document.createElement('a'); link.href = URL.createObjectURL(new Blob([encrypted], { type: 'application/octet-stream' })); link.download = 'moraxterm-vault.mvx'; link.click(); URL.revokeObjectURL(link.href); setVaultPassword(''); setVaultStatus('Encrypted vault exported; plaintext was cleared.');
    } catch (error) { setVaultStatus(error instanceof Error ? error.message : String(error)); }
  };
  const importVault = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]; if (!file || !vaultPassword) { setVaultStatus('Choose an encrypted vault and enter its password.'); return; }
    try { const settings = readVaultPayload(JSON.parse(await tauriInvoke<string>('vault_decrypt', { envelope: await file.text(), password: vaultPassword }))); setDraft(current => ({ ...current, ...settings })); setVaultPassword(''); setVaultStatus('Vault authenticated and imported for review; save settings to apply.'); }
    catch (error) { setVaultStatus(error instanceof Error ? error.message : 'Vault authentication failed.'); }
  };
  return <section className="react-editor" aria-label="Application settings">
    <header className="react-editor-header"><div><span className="react-editor-kicker">APPLICATION SETTINGS</span><h2>Settings</h2></div><button onClick={onCancel} aria-label="Close settings">×</button></header>
    <nav className="react-editor-tabs"><button className={tab === 'Appearance' ? 'selected' : ''} onClick={() => setTab('Appearance')}>Appearance</button><button className={tab === 'Terminal' ? 'selected' : ''} onClick={() => setTab('Terminal')}>Terminal</button><button className={tab === 'Behavior' ? 'selected' : ''} onClick={() => setTab('Behavior')}>Behavior</button></nav>
    <div className="react-editor-form">
      {tab === 'Appearance' && <>
        <h3 className="react-settings-heading">Appearance</h3>
        <label>Application theme<select value={draft.appTheme} onChange={event => update('appTheme', event.target.value as UiSettings['appTheme'])}><option value="mobaxterm">MobaXterm</option><option value="midnight">Midnight</option><option value="light">Light</option><option value="amber">Amber</option><option value="neon">Neon</option></select></label>
        <div className="react-editor-note">The application theme changes the interface colours immediately after you save. The terminal theme is independent and also applies to terminals that are already open.</div>
      </>}
      {tab === 'Terminal' && <>
        <h3 className="react-settings-heading">Terminal</h3>
        <label>Terminal theme<select value={draft.terminalTheme} onChange={event => update('terminalTheme', event.target.value as UiSettings['terminalTheme'])}><option value="mobaxterm">MobaXterm</option><option value="dark">Midnight</option><option value="light">Light</option><option value="amber">Amber</option><option value="neon">Neon</option></select></label>
        <label>Font size<input type="number" min="10" max="28" value={draft.fontSize} onChange={event => update('fontSize', Number(event.target.value) || 13)} /></label>
        <label className="react-check"><input type="checkbox" checked={draft.cursorBlink} onChange={event => update('cursorBlink', event.target.checked)} /> Blink terminal cursor</label>
      </>}
      {tab === 'Behavior' && <>
        <h3 className="react-settings-heading">Behavior</h3>
        <label className="react-check"><input type="checkbox" checked={draft.highlightKeywords} onChange={event => update('highlightKeywords', event.target.checked)} /> Highlight terminal keywords</label>
        <div className="react-editor-note">Selection copies automatically and right-click pastes through the normal terminal input path. Credentials and private keys are never stored here.</div>
        <h3 className="react-settings-heading">AI assistance</h3>
        <label className="react-check"><input type="checkbox" checked={draft.aiEnabled} onChange={event => update('aiEnabled', event.target.checked)} /> Enable preview-only AI assistance</label>
        <label>Provider<select value={draft.aiProvider} onChange={event => update('aiProvider', event.target.value as UiSettings['aiProvider'])}><option value="none">Disabled</option><option value="local">Local provider (adapter required)</option></select></label>
        <div className="react-editor-note">AI output is redacted and preview-only. Moraxterm never auto-executes generated commands; no provider is bundled by default.</div>
        <h3 className="react-settings-heading">Encrypted local vault</h3>
        <label>Vault password<input type="password" value={vaultPassword} onChange={event => setVaultPassword(event.target.value)} autoComplete="new-password" /></label>
        <div className="react-editor-actions"><button onClick={exportVault}>Export encrypted vault</button><label className="react-button-label">Import encrypted vault<input type="file" accept=".mvx,application/octet-stream" onChange={importVault} /></label></div>
        <div className="react-editor-note" role="status">{vaultStatus} Team/cloud sync is not available.</div>
      </>}
    </div>
    <footer className="react-editor-footer"><button onClick={onCancel}>Cancel</button><button className="primary" onClick={() => onSave(draft)}>Save settings</button></footer>
  </section>;
}
