import { useEffect, useState } from 'react';
import type { Session, SessionType } from './react-types';
import { tauriInvoke } from './react-types';
import { colorFor, iconFor } from './icons.js';

interface SessionEditorProps {
  session: Session;
  onSave: (session: Session) => void;
  onCancel: () => void;
}

const protocols: SessionType[] = ['SSH', 'Telnet', 'Rdp', 'Vnc', 'WSL', 'Shell', 'Serial', 'Unix'];

export default function SessionEditor({ session, onSave, onCancel }: SessionEditorProps) {
  const [draft, setDraft] = useState<Session>(session);
  const [tab, setTab] = useState<'Basic' | 'SSH' | 'Terminal' | 'Protocol'>('Basic');
  const [knownHostsStatus, setKnownHostsStatus] = useState('Not checked');
  const [fingerprint, setFingerprint] = useState('');
  const [trustBusy, setTrustBusy] = useState(false);
  useEffect(() => setDraft(session), [session]);
  const update = <K extends keyof Session>(key: K, value: Session[K]) => setDraft(current => ({ ...current, [key]: value }));
  const isSsh = draft.session_type === 'SSH';
  const isProtocol = ['WSL', 'Serial', 'Telnet', 'Rdp', 'Vnc', 'Unix'].includes(draft.session_type);
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
        <label>Folder<input value={draft.folder ?? ''} onChange={event => update('folder', event.target.value || undefined)} placeholder="User sessions" /></label>
      </>}
      {tab === 'SSH' && <>
        <div className="react-editor-note">SSH compatibility settings are stored as typed session metadata and passed to the existing Rust bridge.</div>
        <label>Remote shell<select value={draft.remote_shell ?? ''} onChange={event => update('remote_shell', event.target.value || undefined)}><option value="">Server default</option><option value="bash">Bash</option><option value="fish">Fish (requires remote Fish)</option><option value="fish-or-sh">Fish if available, otherwise POSIX sh</option><option value="zsh">Zsh</option><option value="sh">POSIX sh</option></select></label>
        <label className="react-check"><input type="checkbox" checked={draft.ssh_agent_forwarding ?? false} onChange={event => update('ssh_agent_forwarding', event.target.checked)} /> Enable SSH-agent forwarding</label>
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
