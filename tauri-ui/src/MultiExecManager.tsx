import { useMemo, useState } from 'react';
import { canStop } from './multiexec.js';

export interface MultiExecTarget { id: string; name: string; type: string; state: string; }
interface MultiExecProps {
  targets: MultiExecTarget[];
  excluded: Set<string>;
  status: Record<string, 'sent' | 'failed'>;
  onToggle: (id: string) => void;
  onSetExcluded: (next: Set<string>) => void;
  onSend: (data: string) => void;
  onStop: (id: string) => void;
  onExit: () => void;
  onClose: () => void;
}

export default function MultiExecManager({ targets, excluded, status, onToggle, onSetExcluded, onSend, onStop, onExit, onClose }: MultiExecProps) {
  const activeTargets = useMemo(() => targets.filter(target => target.state !== 'exited'), [targets]);
  const stoppedTargets = useMemo(() => targets.filter(target => target.state === 'exited'), [targets]);
  const [data, setData] = useState('');
  const [enter, setEnter] = useState(true);
  const selected = activeTargets.filter(target => !excluded.has(target.id)).length;
  const send = () => { if (!data || !selected) return; onSend(enter ? data.replace(/\r?\n$/, '') + '\r' : data); };
  return <section className="react-editor" aria-label="MultiExec manager">
    <header className="react-editor-header"><div><span className="react-editor-kicker">SYNCHRONIZED TERMINALS</span><h2>MultiExec targets</h2></div><button onClick={onClose} aria-label="Close MultiExec">×</button></header>
    <div className="react-multiexec-banner"><strong>MultiExec mode is active</strong><span>{selected} of {activeTargets.length} selected</span></div>
    <div className="react-multiexec-layout"><div className="react-multiexec-targets">{activeTargets.map(target => <div className="react-multiexec-row" key={target.id}><label className="react-multiexec-pick"><input type="checkbox" checked={!excluded.has(target.id)} onChange={() => onToggle(target.id)} /><span><strong>{target.name}</strong><small>{target.type} · {target.state}</small></span></label><em className={status[target.id] || ''}>{excluded.has(target.id) ? 'excluded' : status[target.id] === 'sent' ? '✓ sent' : status[target.id] === 'failed' ? '✕ failed' : ''}</em><button className="danger react-multiexec-stop" disabled={!canStop(target)} onClick={() => onStop(target.id)} title={`Stop ${target.name}: end only this terminal's process`} aria-label={`Stop ${target.name}`}>■ Stop</button></div>)}{stoppedTargets.map(target => <div className="react-multiexec-row stopped" key={target.id}><span className="react-multiexec-pick"><span><strong>{target.name}</strong><small>{target.type} · stopped</small></span></span><em>stopped</em></div>)}{!activeTargets.length && !stoppedTargets.length && <p className="react-empty">Open at least one terminal to use MultiExec.</p>}</div><div className="react-multiexec-compose"><label>Command or input<textarea value={data} onChange={event => setData(event.target.value)} placeholder="Type a command to send to selected terminals..." /></label><label className="react-multiexec-enter"><input type="checkbox" checked={enter} onChange={event => setEnter(event.target.checked)} /> Press Enter after sending</label><div className="react-editor-note">Typing in any terminal also goes to every selected terminal while this mode is active. Ctrl+Shift+Insert pastes the clipboard into all of them. Exited and failed terminals are skipped.</div><button className="primary react-broadcast" disabled={!data || !selected} onClick={send}>⇄ Send to selected</button></div></div>
    <footer className="react-editor-footer"><button onClick={() => onSetExcluded(new Set())}>Select all</button><button onClick={() => onSetExcluded(new Set(activeTargets.map(target => target.id)))}>Clear</button><button onClick={onClose}>Done</button><button className="danger" onClick={onExit}>Exit multi-execution mode</button></footer>
  </section>;
}
