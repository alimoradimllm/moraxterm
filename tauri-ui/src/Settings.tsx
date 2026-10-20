import { useState } from 'react';

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
  const update = <K extends keyof UiSettings>(key: K, next: UiSettings[K]) => setDraft(current => ({ ...current, [key]: next }));
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
      </>}
    </div>
    <footer className="react-editor-footer"><button onClick={onCancel}>Cancel</button><button className="primary" onClick={() => onSave(draft)}>Save settings</button></footer>
  </section>;
}
