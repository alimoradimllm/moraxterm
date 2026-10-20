import { useEffect, useState } from 'react';
import type { Session, SessionType } from './react-types';
import { tauriInvoke } from './react-types';
import { colorFor, iconFor } from './icons.js';

interface SessionEditorProps {
  session: Session;
  folders?: string[];
  onSave: (session: Session) => void;
  onCancel: () => void;
}

const protocols: SessionType[] = ['SSH', 'Mosh', 'Telnet', 'Rdp', 'Vnc', 'WSL', 'Shell', 'Serial', 'Unix'];

export default function SessionEditor({ session, folders = [], onSave, onCancel }: SessionEditorProps) {
  const [draft, setDraft] = useState<Session>(session);
  const [tab, setTab] = useState<'Basic' | 'SSH' | 'Terminal' | 'Protocol'>('Basic');
  const [knownHostsStatus, setKnownHostsStatus] = useState('Not checked');
  const [fingerprint, setFingerprint] = useState('');
  const [trustBusy, setTrustBusy] = useState(false);
  const [credentialSecret, setCredentialSecret] = useState('');
  const [credentialStatus, setCredentialStatus] = useState('Not checked');
  const [credentialBusy, setCredentialBusy] = useState(false);
  useEffect(() => setDraft(session), [session]);
  const update = <K extends keyof Session>(key: K, value: Session[K]) => setDraft(current => ({ ...current, [key]: value }));
  const isSsh = draft.session_type === 'SSH' || draft.session_type === 'Mosh';
  const isProtocol = ['WSL', 'Serial', 'Telnet', 'Rdp', 'Vnc', 'Unix'].includes(draft.session_type);
  const identityName = draft.identity_name?.trim() ?? '';
  const credentialAccount = `${draft.username?.trim() ?? ''}@${draft.host?.trim() ?? ''}:${draft.port || 22}`;
  const credentialArgs = identityName ? { name: identityName, kind: 'password' } : { service: 'com.moraxterm.ssh', account: credentialAccount };
  const checkCredential = async () => {
    if (!draft.username?.trim() || !draft.host?.trim()) { setCredentialStatus('Enter username and host first.'); return; }
    setCredentialBusy(true);
    try { setCredentialStatus(await tauriInvoke<boolean>(identityName ? 'identity_status' : 'credential_status', credentialArgs) ? 'Saved in OS credential storage.' : 'No saved credential.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const saveCredential = async () => {
    if (!credentialSecret) { setCredentialStatus('Enter a password to save.'); return; }
    setCredentialBusy(true);
    if (identityName && await tauriInvoke<boolean>('identity_status', { name: identityName, kind: 'password' }) && !window.confirm(`Overwrite shared identity “${identityName}”?`)) { setCredentialBusy(false); return; }
    try { await tauriInvoke(identityName ? 'identity_store' : 'credential_store', { ...credentialArgs, secret: credentialSecret }); setCredentialSecret(''); setCredentialStatus('Saved in OS credential storage.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const deleteCredential = async () => {
    setCredentialBusy(true);
    if (identityName && !window.confirm(`Delete shared identity “${identityName}”?`)) { setCredentialBusy(false); return; }
    try { await tauriInvoke(identityName ? 'identity_delete' : 'credential_delete', credentialArgs); setCredentialStatus('Deleted from OS credential storage.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const savePrivateKeyIdentity = async () => {
    if (!identityName || !draft.private_key?.trim()) { setCredentialStatus('Enter an identity name and private key path first.'); return; }
    setCredentialBusy(true);
    if (await tauriInvoke<boolean>('identity_status', { name: identityName, kind: 'private-key' }) && !window.confirm(`Overwrite shared private-key identity “${identityName}”?`)) { setCredentialBusy(false); return; }
    try { await tauriInvoke('identity_store', { name: identityName, kind: 'private-key', secret: draft.private_key.trim() }); setCredentialStatus('Private-key identity saved in OS credential storage.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const planPublicKeyExport = async () => {
    if (!draft.private_key?.trim()) { setCredentialStatus('Enter a private key path before planning public-key export.'); return; }
    try { const plan = await tauriInvoke<string[]>('ssh_public_key_plan', { privateKey: draft.private_key.trim() }); setCredentialStatus(`Public-key export ready: ${plan.join(' ')}. Review and run locally; authorized_keys installation is not provided.`); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
  };
  const checkFido2 = async () => {
    setCredentialBusy(true);
    try { setCredentialStatus(await tauriInvoke<string>('ssh_fido2_status')); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const checkPrivateKeyIdentity = async () => {
    if (!identityName) { setCredentialStatus('Enter an identity name first.'); return; }
    setCredentialBusy(true);
    try { setCredentialStatus(await tauriInvoke<boolean>('identity_status', { name: identityName, kind: 'private-key' }) ? 'Private-key identity is saved.' : 'No saved private-key identity.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const deletePrivateKeyIdentity = async () => {
    if (!identityName) return;
    setCredentialBusy(true);
    if (!window.confirm(`Delete shared private-key identity “${identityName}”?`)) { setCredentialBusy(false); return; }
    try { await tauriInvoke('identity_delete', { name: identityName, kind: 'private-key' }); setCredentialStatus('Private-key identity deleted from OS credential storage.'); }
    catch (error) { setCredentialStatus(error instanceof Error ? error.message : String(error)); }
    finally { setCredentialBusy(false); }
  };
  const checkKnownHost = async () => {
    if (!draft.host?.trim() || !draft.port) { setKnownHostsStatus('Enter a host and valid port first.'); return; }
    setTrustBusy(true);
    try { setKnownHostsStatus(await tauriInvoke<string>('ssh_known_hosts_status', { host: draft.host.trim(), port: draft.port })); }
    catch (error) { setKnownHostsStatus(error instanceof Error ? error.message : String(error)); }
    finally { setTrustBusy(false); }
  };
  const scanKnownHost = async () => {
    if (!draft.host?.trim() || !draft.port) { setKnownHostsStatus('Enter a host and valid port first.'); return; }
    setTrustBusy(true); setFingerprint('');
    try { setFingerprint(await tauriInvoke<string>('ssh_known_hosts_scan', { host: draft.host.trim(), port: draft.port })); setKnownHostsStatus('Fingerprint scanned. Confirm it before approval.'); }
    catch (error) { setKnownHostsStatus(error instanceof Error ? error.message : String(error)); }
    finally { setTrustBusy(false); }
  };
  const approveKnownHost = async () => {
    if (!draft.host?.trim() || !draft.port || !fingerprint) return;
    setTrustBusy(true);
    try { setKnownHostsStatus(await tauriInvoke<string>('ssh_known_hosts_add', { host: draft.host.trim(), port: draft.port, expectedFingerprint: fingerprint })); }
    catch (error) { setKnownHostsStatus(error instanceof Error ? error.message : String(error)); }
    finally { setTrustBusy(false); }
  };
  const removeKnownHost = async () => {
    if (!draft.host?.trim() || !draft.port) return;
    setTrustBusy(true);
    try { setKnownHostsStatus(await tauriInvoke<string>('ssh_known_hosts_remove', { host: draft.host.trim(), port: draft.port })); setFingerprint(''); }
    catch (error) { setKnownHostsStatus(error instanceof Error ? error.message : String(error)); }
    finally { setTrustBusy(false); }
  };
  useEffect(() => { if (tab === 'Protocol' && !isProtocol) setTab('Basic'); }, [isProtocol, tab]);
  return <section className="react-editor" aria-label="Session editor">
    <header className="react-editor-header"><div className="react-editor-head-icon" style={{ flexDirection: 'row' }}><span className="react-protocol-icon" style={{ '--c': colorFor(draft.session_type) } as React.CSSProperties} dangerouslySetInnerHTML={{ __html: iconFor(draft.session_type) }} /><div style={{ display: 'flex', flexDirection: 'column' }}><span className="react-editor-kicker">SESSION EDITOR</span><h2>{draft.name || 'New session'}</h2></div></div><button onClick={onCancel} aria-label="Close session editor">×</button></header>
    <div className="react-editor-protocols">{protocols.map(protocol => <button key={protocol} className={draft.session_type === protocol ? 'selected' : ''} style={{ '--c': colorFor(protocol) } as React.CSSProperties} onClick={() => update('session_type', protocol)}><span className="glyph" aria-hidden="true" dangerouslySetInnerHTML={{ __html: iconFor(protocol) }} />{protocol}</button>)}</div>
    <nav className="react-editor-tabs">{(['Basic', 'SSH', 'Terminal', ...(isProtocol ? ['Protocol'] : [])] as const).map(name => <button key={name} className={tab === name ? 'selected' : ''} onClick={() => setTab(name as 'Basic' | 'SSH' | 'Terminal' | 'Protocol')}>{name}</button>)}</nav>
    <div className="react-editor-form">
      {tab === 'Basic' && <>
        <label>Session name<input value={draft.name} onChange={event => update('name', event.target.value)} /></label>
        <div className="react-editor-grid"><label>Host<input value={draft.host ?? ''} onChange={event => update('host', event.target.value || undefined)} /></label><label>Port<input type="number" min="1" max="65535" value={draft.port ?? (isSsh ? 22 : '')} onChange={event => update('port', event.target.value ? Number(event.target.value) : undefined)} /></label></div>
        <label>Username<input value={draft.username ?? ''} onChange={event => update('username', event.target.value || undefined)} autoComplete="off" /></label>
        <label>Folder<input list="react-folder-options" value={draft.folder ?? ''} onChange={event => update('folder', event.target.value || undefined)} placeholder="Top level (use Work/Prod for a sub-folder)" /><datalist id="react-folder-options">{folders.map(path => <option key={path} value={path} />)}</datalist></label>
      </>}
      {tab === 'SSH' && <>
        <div className="react-editor-note">SSH compatibility settings are stored as typed session metadata and passed to the existing Rust bridge.</div>
        <label>Identity name<input value={draft.identity_name ?? ''} onChange={event => update('identity_name', event.target.value || undefined)} placeholder="e.g. production-admin" autoComplete="off" /></label>
        <label>Saved password / key passphrase<input type="password" value={credentialSecret} onChange={event => setCredentialSecret(event.target.value)} autoComplete="new-password" placeholder="Leave blank to keep unchanged" /></label>
        <div className="react-editor-actions"><button disabled={credentialBusy} onClick={saveCredential}>Save password</button><button disabled={credentialBusy} onClick={checkCredential}>Check saved</button><button disabled={credentialBusy} onClick={deleteCredential}>Delete saved</button></div>
        <div className="react-editor-note">{credentialStatus} Passwords are stored only in the operating system credential manager, never in the session record.</div>
        <label>Private key path<input value={draft.private_key ?? ''} onChange={event => update('private_key', event.target.value || undefined)} placeholder="C:\\Users\\you\\.ssh\\id_ed25519" autoComplete="off" /></label>
        <label>SSH certificate path (optional)<input value={draft.certificate_path ?? ''} onChange={event => update('certificate_path', event.target.value || undefined)} placeholder="C:\\Users\\you\\.ssh\\id_ed25519-cert.pub" autoComplete="off" /></label>
        <div className="react-editor-actions"><button disabled={credentialBusy} onClick={checkFido2}>Check FIDO2 SSH capability</button></div><div className="react-editor-actions"><button disabled={credentialBusy} onClick={savePrivateKeyIdentity}>Save key identity</button><button disabled={credentialBusy} onClick={checkPrivateKeyIdentity}>Check key identity</button><button disabled={credentialBusy} onClick={deletePrivateKeyIdentity}>Delete key identity</button></div>
        <div className="react-editor-actions"><button disabled={credentialBusy} onClick={planPublicKeyExport}>Plan public-key export</button></div>
        <div className="react-editor-note">Only the path is saved. Private-key bytes and passphrases remain outside Moraxterm session storage.</div>
        <label>Remote shell<select value={draft.remote_shell ?? ''} onChange={event => update('remote_shell', event.target.value || undefined)}><option value="">Server default</option><option value="bash">Bash</option><option value="fish">Fish (requires remote Fish)</option><option value="fish-or-sh">Fish if available, otherwise POSIX sh</option><option value="zsh">Zsh</option><option value="sh">POSIX sh</option></select></label>
        <label className="react-check"><input type="checkbox" checked={draft.ssh_agent_forwarding ?? false} onChange={event => update('ssh_agent_forwarding', event.target.checked)} /> Enable SSH-agent forwarding</label>
        <label className="react-check"><input type="checkbox" checked={draft.x11_forwarding ?? false} onChange={event => update('x11_forwarding', event.target.checked)} /> Enable SSH X11 forwarding</label>
        <div className="react-editor-note">X11 forwarding uses the system SSH client and a running local X server. Moraxterm does not bundle an X.org server; use X server status to verify availability before connecting.</div>
        <label>ProxyJump / gateway<input value={draft.proxy_jump ?? ''} onChange={event => update('proxy_jump', event.target.value || undefined)} placeholder="jump-user@gateway.example.com" /></label>
        <label>Host-key policy<select value={draft.host_key_policy ?? 'ask'} onChange={event => update('host_key_policy', event.target.value)}><option value="ask">Ask on unknown host</option><option value="strict">Strict: reject unknown host</option><option value="accept-new">Accept new keys, reject changes</option></select></label>
        <div className="react-editor-note">ProxyJump is passed as a structured OpenSSH argument. Unknown or changed host keys are never silently trusted.</div>
        <div className="react-editor-note"><strong>Host-key trust</strong><br />{knownHostsStatus}{fingerprint && <><br /><code>{fingerprint}</code></>}</div>
        <div className="react-editor-actions"><button disabled={trustBusy} onClick={checkKnownHost}>Check known host</button><button disabled={trustBusy} onClick={scanKnownHost}>Scan fingerprint</button><button disabled={trustBusy || !fingerprint} onClick={approveKnownHost}>Approve scanned key</button><button disabled={trustBusy} onClick={removeKnownHost}>Remove stored key</button></div>
      </>}
      {tab === 'Terminal' && <>
        <label>Local shell<select value={draft.shell ?? ''} onChange={event => update('shell', event.target.value || undefined)}><option value="">System default</option><option value="cmd">Command Prompt</option><option value="powershell">PowerShell</option><option value="bash">Bash</option></select></label>
        <div className="react-editor-note">Terminal input preserves Moraxterm behavior: selection copies automatically and right-click pastes.</div>
      </>}
      {tab === 'Protocol' && draft.session_type === 'WSL' && <>
        <div className="react-editor-note">WSL settings are passed to the existing Windows Subsystem for Linux launcher.</div>
        <label>Distribution<input value={draft.host ?? ''} onChange={event => update('host', event.target.value || undefined)} placeholder="Debian" /></label>
        <label>Linux user<input value={draft.wsl_user ?? ''} onChange={event => update('wsl_user', event.target.value || undefined)} placeholder="Default user" /></label>
        <label>Startup command<input value={draft.wsl_command ?? ''} onChange={event => update('wsl_command', event.target.value || undefined)} placeholder="Optional command" /></label>
      </>}
      {tab === 'Protocol' && draft.session_type === 'Serial' && <>
        <div className="react-editor-note">Serial settings are validated by the native bridge before opening the device.</div>
        <label>Device<input value={draft.host ?? ''} onChange={event => update('host', event.target.value || undefined)} placeholder="COM3 or /dev/ttyUSB0" /></label>
        <div className="react-editor-grid"><label>Baud rate<input type="number" min="1" value={draft.serial_baud ?? 115200} onChange={event => update('serial_baud', Number(event.target.value) || 0)} /></label><label>Data bits<select value={draft.serial_data_bits ?? 8} onChange={event => update('serial_data_bits', Number(event.target.value))}><option value="5">5</option><option value="6">6</option><option value="7">7</option><option value="8">8</option></select></label></div>
        <div className="react-editor-grid"><label>Parity<select value={draft.serial_parity ?? 'none'} onChange={event => update('serial_parity', event.target.value)}><option value="none">None</option><option value="even">Even</option><option value="odd">Odd</option></select></label><label>Stop bits<select value={draft.serial_stop_bits ?? 1} onChange={event => update('serial_stop_bits', Number(event.target.value))}><option value="1">1</option><option value="2">2</option></select></label></div>
        <label>Flow control<select value={draft.serial_flow_control ?? 'none'} onChange={event => update('serial_flow_control', event.target.value)}><option value="none">None</option><option value="hardware">Hardware</option><option value="software">Software</option></select></label>
      </>}
      {tab === 'Protocol' && draft.session_type === 'Rdp' && <>
        <div className="react-editor-note">RDP settings are stored with the session and passed to the native external-client preflight.</div>
        <div className="react-editor-grid"><label>Width<input type="number" min="640" max="7680" value={draft.rdp_width ?? 1920} onChange={event => update('rdp_width', Number(event.target.value) || 0)} /></label><label>Height<input type="number" min="480" max="4320" value={draft.rdp_height ?? 1080} onChange={event => update('rdp_height', Number(event.target.value) || 0)} /></label></div>
        <label className="react-check"><input type="checkbox" checked={draft.rdp_fullscreen ?? false} onChange={event => update('rdp_fullscreen', event.target.checked)} /> Start in fullscreen</label>
        <label className="react-check"><input type="checkbox" checked={draft.rdp_audio ?? true} onChange={event => update('rdp_audio', event.target.checked)} /> Redirect audio</label>
      </>}
      {tab === 'Protocol' && draft.session_type === 'Vnc' && <>
        <div className="react-editor-note">VNC display and sharing options are stored for the native VNC preflight and launcher.</div>
        <label>Display number<input type="number" min="0" max="99" value={draft.vnc_display ?? 0} onChange={event => update('vnc_display', Number(event.target.value) || 0)} /></label>
        <label className="react-check"><input type="checkbox" checked={draft.vnc_view_only ?? false} onChange={event => update('vnc_view_only', event.target.checked)} /> View-only mode</label>
        <label className="react-check"><input type="checkbox" checked={draft.vnc_shared ?? true} onChange={event => update('vnc_shared', event.target.checked)} /> Allow shared connections</label>
      </>}
      {tab === 'Protocol' && draft.session_type === 'Unix' && <>
        <div className="react-editor-note">Embedded Unix runs Fish inside Moraxterm with a private runtime environment. It does not use WSL or Windows cmd.exe.</div>
        <label>Shell<select value={draft.unix_shell ?? 'fish'} onChange={event => update('unix_shell', event.target.value)}><option value="fish">Fish</option><option value="bash">Bash</option><option value="sh">POSIX sh</option></select></label>
        <label>Runtime root<input value={draft.unix_runtime_root ?? ''} onChange={event => update('unix_runtime_root', event.target.value || undefined)} placeholder="Use Moraxterm managed runtime" /></label>
        <label>Host filesystem mount<input value={draft.unix_host_mount ?? ''} onChange={event => update('unix_host_mount', event.target.value || undefined)} placeholder="Automatic host mount" /></label>
        <div className="react-editor-note">The host mount is exposed separately from the private Unix HOME, PATH, and temporary directories.</div>
      </>}
    </div>
    <footer className="react-editor-footer"><button onClick={onCancel}>Cancel</button><button className="primary" onClick={() => onSave(draft)}>Save session</button></footer>
  </section>;
}
