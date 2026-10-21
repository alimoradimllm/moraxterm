import { useState } from 'react';
import { tauriInvoke } from './react-types';

interface ToolsPanelProps { onClose: () => void; }
type Tool = 'ping' | 'traceroute' | 'dns' | 'port';
const serverOptions = ['TFTP server', 'TFTP HTTP server', 'SSH server', 'SFTP server', 'Telnet server', 'NFS server', 'VNC server', 'Cron server', 'iPerf server'];

export default function ToolsPanel({ onClose }: ToolsPanelProps) {
  const [tool, setTool] = useState<Tool>('ping');
  const [target, setTarget] = useState('example.com');
  const [port, setPort] = useState(443);
  const [startPort, setStartPort] = useState(1);
  const [endPort, setEndPort] = useState(32);
  const [output, setOutput] = useState('');
  const [status, setStatus] = useState('Ready');

  const run = async () => {
    setStatus('Running...');
    setOutput('');
    try {
      const result = await tauriInvoke<string>('run_network_tool', { tool, target, port: tool === 'port' ? port : null });
      setOutput(result || '(no output)');
      setStatus('Completed');
    } catch (error) {
      setStatus('Failed');
      setOutput(error instanceof Error ? error.message : String(error));
    }
  };

  const scan = async () => {
    setStatus('Scanning...');
    setOutput('');
    try {
      const result = await tauriInvoke<string>('scan_ports', { target, startPort, endPort });
      setOutput(result || '(no open ports)');
      setStatus('Completed');
    } catch (error) {
      setStatus('Failed');
      setOutput(error instanceof Error ? error.message : String(error));
    }
  };

  return <section className="react-editor" aria-label="Network tools">
    <header className="react-editor-header"><div><span className="react-editor-kicker">READ-ONLY NETWORK DIAGNOSTICS</span><h2>Tools</h2></div><button onClick={onClose} aria-label="Close tools">×</button></header>
    <div className="react-editor-form">
      <div className="react-editor-grid"><label>Tool<select value={tool} onChange={event => setTool(event.target.value as Tool)}><option value="ping">Ping</option><option value="traceroute">Traceroute</option><option value="dns">DNS lookup</option><option value="port">TCP port check</option></select></label><label>Target<input value={target} onChange={event => setTarget(event.target.value)} placeholder="example.com" /></label></div>
      {tool === 'port' && <label>Port<input type="number" min="1" max="65535" value={port} onChange={event => setPort(Number(event.target.value) || 1)} /></label>}
      <div className="react-editor-actions"><button className="primary" disabled={!target.trim()} onClick={run}>Run {tool}</button></div>
      <div className="react-editor-note">{status}. Targets are passed to the native bridge; no shell command text is accepted from this panel.</div>
      <label>Bounded TCP scan (maximum 32 ports)<div className="react-editor-grid"><input aria-label="Start port" type="number" min="1" max="65535" value={startPort} onChange={event => setStartPort(Number(event.target.value) || 1)} /><input aria-label="End port" type="number" min="1" max="65535" value={endPort} onChange={event => setEndPort(Number(event.target.value) || 1)} /></div></label>
      <button onClick={scan} disabled={!target.trim()}>Scan port range</button>
      <pre className="react-tool-output" aria-label="Tool output">{output || 'Results will appear here.'}</pre>
      <h3 className="react-settings-heading">Server options</h3>
      <div className="react-editor-note">Server launchers are unavailable in this build. These entries are configuration placeholders only; no process, service, registry entry, or network listener is started.</div>
      <div className="react-server-options">{serverOptions.map(option => <button key={option} disabled title={`${option} is unavailable`}>{option} — unavailable</button>)}</div>
    </div>
    <footer className="react-editor-footer"><button onClick={onClose}>Close</button></footer>
  </section>;
}
