import { useEffect, useState } from 'react';
import { Session, tauriInvoke, tauriListen } from './react-types';
import { applyProgress, describeProgress, percent, type Meter, type ProgressEvent } from './transferProgress';

interface SftpPaneProps { session?: Session; onClose: () => void; }

interface RemoteRow { text: string; name?: string; directory: boolean; }
interface QueueItem { id: string; label: string; state: 'running' | 'completed' | 'failed'; }

function parseRows(text: string): RemoteRow[] {
  return text.split(/\r?\n/).map(line => line.trim()).filter(Boolean).map(text => {
    // OpenSSH's `ls -la` output starts with the mode; the final token is the
    // filename (the native command already quotes paths safely).
    const fields = text.split(/\s+/);
    const directory = text.startsWith('d');
    const name = directory && fields.length >= 9 ? fields.slice(8).join(' ') : undefined;
    return { text, name: name && !['.', '..'].includes(name) ? name : undefined, directory };
  });
}

function joinRemotePath(base: string, name: string): string {
  if (base === '~' || base === '') return `~/${name}`;
  return `${base.replace(/\/$/, '')}/${name}`;
}

function parentRemotePath(value: string): string {
  if (!value || value === '~' || value === '/') return value || '~';
  const trimmed = value.replace(/\/$/, '');
  const slash = trimmed.lastIndexOf('/');
  if (slash < 0) return '~';
  if (slash === 0) return '/';
  if (trimmed.startsWith('~/') && slash === 1) return '~';
  return trimmed.slice(0, slash);
}

export default function SftpPane({ session, onClose }: SftpPaneProps) {
  const host = session?.host || '';
  const username = session?.username || '';
  const port = session?.port || 22;
  const [path, setPath] = useState(session?.working_directory || '~');
  const [output, setOutput] = useState('');
  const [status, setStatus] = useState('Ready');
  const [busy, setBusy] = useState(false);
  const [operationPath, setOperationPath] = useState('');
  const [target, setTarget] = useState('');
  const [mode, setMode] = useState('0644');
  const [resume, setResume] = useState(false);
  // A running or just-finished transfer: events are kept per job id so an event that arrives before the id is known is not lost.
  const [jobId, setJobId] = useState<string | null>(null);
  const [jobLabel, setJobLabel] = useState('');
  const [meters, setMeters] = useState<Record<string, Meter | null>>({});
  const [exits, setExits] = useState<Record<string, ProgressEvent>>({});
  const [queue, setQueue] = useState<QueueItem[]>([]);

  useEffect(() => { setPath(session?.working_directory || '~'); }, [session?.name, session?.working_directory]);
  useEffect(() => {
    const progress = tauriListen?.<ProgressEvent>('sftp-transfer-progress', event => {
      const payload = event.payload;
      if (payload?.id) setMeters(current => ({ ...current, [payload.id as string]: applyProgress(current[payload.id as string] ?? null, payload, payload.id as string) }));
    });
    const exit = tauriListen?.<ProgressEvent>('sftp-transfer-exit', event => {
      const payload = event.payload;
      if (payload?.id) setExits(current => ({ ...current, [payload.id as string]: payload }));
    });
    return () => { progress?.then(off => off()); exit?.then(off => off()); };
  }, []);
  useEffect(() => {
    setQueue(items => items.map(item => {
      const result = exits[item.id];
      return result ? { ...item, state: result.success ? 'completed' : 'failed' } : item;
    }));
  }, [exits]);

  const exit = jobId ? exits[jobId] : undefined;
  const meter = jobId ? meters[jobId] ?? null : null;
  const running = Boolean(jobId) && !exit;
  useEffect(() => {
    if (!jobId || !exit) return;
    setStatus(exit.success ? `${jobLabel} completed` : `Failed: ${exit.message || 'transfer failed'}`);
    setBusy(false);
  }, [exit, jobId, jobLabel]);

  const rows = parseRows(output);
  const navigate = (next: string) => { setPath(next); setOutput(''); void request(() => invoke('sftp_list', { host, username, port, path: next }), `Listed ${next}`); };
  const request = async (action: () => Promise<unknown>, success: string) => {
    setBusy(true); setStatus('Working…');
    try { const result = await action(); setOutput(String(result || '')); setStatus(success); }
    catch (error) { setStatus(`Failed: ${String(error)}`); }
    finally { setBusy(false); }
  };
  const invoke = (command: string, args: Record<string, unknown>) => tauriInvoke ? tauriInvoke(command, args) : Promise.reject('Tauri bridge unavailable');
  const list = () => request(() => invoke('sftp_list', { host, username, port, path }), `Listed ${path}`);
  const operate = (operation: string) => request(() => invoke('sftp_operation', { host, username, port, operation, path: operationPath, target, mode }), `${operation} completed`);
  const transfer = async (direction: 'upload' | 'download') => {
    setBusy(true); setJobId(null); setJobLabel(direction); setStatus(`${direction} starting…`);
    try {
      const id = await invoke('start_sftp_transfer', { host, username, port, direction, localPath: operationPath, remotePath: target || path, resume }) as string;
      setJobId(id); setQueue(items => [...items, { id, label: `${direction}: ${target || path}`, state: 'running' }]); setStatus(`${direction} running…`);
    } catch (error) { setStatus(`Failed: ${String(error)}`); setBusy(false); }
  };
  const cancel = async () => {
    if (!jobId) return;
    try { await invoke('cancel_sftp_transfer', { id: jobId }); } catch (error) { setStatus(`Cancel failed: ${String(error)}`); }
  };
  const barPercent = meter ? percent(meter.done, meter.total) : null;

  return <section className="react-sftp-pane">
    <header className="react-sftp-header"><div><strong>SFTP / FTP Browser</strong><small>{session ? `${username}@${host}:${port}` : 'No active session'}</small></div><button onClick={onClose}>Close</button></header>
    <div className="react-sftp-path"><label>Remote path<input value={path} onChange={event => setPath(event.target.value)} /></label><button disabled={busy || !host || !username} onClick={list}>Refresh</button><button disabled={busy || path === '~' || path === '/'} onClick={() => navigate(parentRemotePath(path))}>Up</button></div>
    <nav className="react-sftp-breadcrumbs" aria-label="Remote path breadcrumbs">{path.split('/').filter(Boolean).map((part, index, parts) => { const crumb = path.startsWith('~') ? `~/${parts.slice(1, index + 1).join('/')}` : `/${parts.slice(0, index + 1).join('/')}`; return <button key={`${crumb}-${index}`} disabled={busy || crumb === path} onClick={() => navigate(crumb)}>{part}</button>; })}</nav>
    <div className="react-sftp-body"><div className="react-sftp-list"><h3>Remote directory</h3>{rows.length ? rows.map((row, index) => row.directory && row.name ? <button className="react-sftp-row" key={`${index}-${row.text}`} onClick={() => navigate(joinRemotePath(path, row.name!))}>{row.text}</button> : <div className="react-sftp-row" key={`${index}-${row.text}`}>{row.text}</div>) : <p className="react-sftp-empty">No listing loaded.</p>}</div>
      <div className="react-sftp-actions"><h3>Operations</h3><label>Path<input value={operationPath} onChange={event => setOperationPath(event.target.value)} placeholder="/remote/file" /></label><label>Target<input value={target} onChange={event => setTarget(event.target.value)} placeholder="/remote/target or local file" /></label><label>Mode<input value={mode} onChange={event => setMode(event.target.value)} /></label><label className="react-sftp-check"><input type="checkbox" checked={resume} onChange={event => setResume(event.target.checked)} /> Resume transfer</label><div className="react-sftp-buttons"><button disabled={busy || !host} onClick={() => operate('mkdir')}>New folder</button><button disabled={busy || !host} onClick={() => operate('rename')}>Rename</button><button disabled={busy || !host} onClick={() => operate('delete')}>Delete</button><button disabled={busy || !host} onClick={() => operate('chmod')}>Permissions</button><button disabled={busy || !host} onClick={() => transfer('upload')}>Upload</button><button disabled={busy || !host} onClick={() => transfer('download')}>Download</button></div>
        {queue.length > 0 && <div className="react-transfer-queue"><strong>Transfer queue</strong>{queue.map(item => <div className="react-transfer-queue-row" key={item.id}><span>{item.label}</span><span data-state={item.state}>{item.state}</span></div>)}</div>}
        {jobId && <div className="react-transfer" data-state={running ? 'running' : exit?.success ? 'done' : 'failed'}>
          <div className="react-transfer-bar" role="progressbar" aria-label={`${jobLabel} progress`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={barPercent ?? (exit?.success ? 100 : undefined)}><span style={{ width: `${exit?.success ? 100 : barPercent ?? 0}%` }} /></div>
          <div className="react-transfer-line"><span className="react-transfer-text">{exit?.success ? 'Done' : exit ? 'Stopped' : describeProgress(meter?.done, meter?.total)}</span>{running && <button onClick={cancel}>Cancel</button>}</div>
        </div>}
      </div>
    </div><footer className="react-sftp-footer"><span>{status}</span><span>{rows.length} lines</span></footer>
  </section>;
}
