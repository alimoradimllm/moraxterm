import { useMemo, useState } from 'react';
import { renderSnippet, searchSnippets, validateSnippet, type Snippet } from './snippets';

interface Target { id: string; name: string; host?: string; user?: string; }
interface Props { targets: Target[]; onExecute: (targetIds: string[], command: string) => Promise<void>; onClose: () => void; }
const key = 'moraxterm-snippets';
const load = (): Snippet[] => { try { const value = JSON.parse(localStorage.getItem(key) || '[]'); return Array.isArray(value) ? value.filter(item => item && typeof item.name === 'string' && typeof item.template === 'string') : []; } catch { return []; } };

export default function SnippetManager({ targets, onExecute, onClose }: Props) {
  const [snippets, setSnippets] = useState(load);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const [name, setName] = useState('');
  const [template, setTemplate] = useState('');
  const [targetIds, setTargetIds] = useState<string[]>(targets.map(target => target.id));
  const [status, setStatus] = useState('Preview commands before sending.');
  const [preview, setPreview] = useState('');
  const visible = useMemo(() => searchSnippets(snippets, query), [snippets, query]);
  const current = visible[selected];
  const persist = (next: Snippet[]) => { setSnippets(next.slice(0, 100)); localStorage.setItem(key, JSON.stringify(next.slice(0, 100))); };
  const choose = (item: Snippet, index: number) => { setSelected(index); setName(item.name); setTemplate(item.template); setPreview(''); setStatus('Preview commands before sending.'); };
  const save = () => { const snippet = { name: name.trim(), template }; const error = validateSnippet(snippet); if (error) { setStatus(error); return; } persist([snippet, ...snippets.filter(item => item.name !== snippet.name)]); setStatus('Snippet saved locally without secrets.'); };
  const makePreview = () => { const error = validateSnippet({ name: name.trim() || 'preview', template }); if (error) { setStatus(error); setPreview(''); return; } const lines = targets.filter(target => targetIds.includes(target.id)).map(target => `${target.name}: ${renderSnippet(template, { host: target.host, user: target.user })}`); setPreview(lines.join('\n')); setStatus(lines.length ? 'Preview ready. Review it before sending.' : 'Select at least one target.'); };
  const execute = async () => { if (!preview || !targetIds.length) { setStatus('Generate and review a preview first.'); return; } await onExecute(targetIds, preview.split('\n').map(line => line.slice(line.indexOf(': ') + 2)).join('\n')); setStatus(`Sent reviewed preview to ${targetIds.length} selected target(s).`); };
  return <section className="react-editor" aria-label="Command snippets"><header className="react-editor-header"><div><span className="react-editor-kicker">PRODUCTIVITY</span><h2>Command snippets</h2></div><button onClick={onClose} aria-label="Close snippets">×</button></header>
    <div className="react-editor-form"><label>Search snippets<input aria-label="Search snippets" value={query} onChange={event => setQuery(event.target.value)} placeholder="Find by name or command" /></label><div className="react-snippet-list">{visible.map((item, index) => <button key={item.name} className={item === current ? 'selected' : ''} onClick={() => choose(item, index)}><strong>{item.name}</strong><small>{item.template}</small></button>)}{!visible.length && <span className="react-empty">No snippets saved.</span>}</div>
      <label>Name<input value={name} onChange={event => setName(event.target.value)} /></label><label>Command template<textarea value={template} onChange={event => setTemplate(event.target.value)} placeholder="ssh {{user}}@{{host}}" rows={3} /></label><div className="react-editor-note">Allowed variables: <code>{'{{host}}'}</code>, <code>{'{{user}}'}</code>, <code>{'{{date}}'}</code>. Secrets and arbitrary expressions are rejected.</div><div className="react-editor-actions"><button onClick={save}>Save snippet</button><button onClick={makePreview}>Preview selected hosts</button></div>
      <fieldset><legend>MultiExec targets</legend>{targets.map(target => <label key={target.id} className="react-check"><input type="checkbox" checked={targetIds.includes(target.id)} onChange={() => setTargetIds(current => current.includes(target.id) ? current.filter(id => id !== target.id) : [...current, target.id])} />{target.name}</label>)}</fieldset>{preview && <pre className="react-snippet-preview" aria-label="Snippet preview">{preview}</pre>}<div className="react-editor-note" role="status">{status} Nothing runs until you explicitly send the reviewed preview.</div><button className="primary" onClick={execute} disabled={!preview || !targetIds.length}>Send reviewed preview to selected targets</button></div><footer className="react-editor-footer"><button onClick={onClose}>Done</button></footer></section>;
}
