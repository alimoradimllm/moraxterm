export interface Snippet { name: string; template: string; }
export interface SnippetContext { host?: string; user?: string; date?: string; }

const allowed = new Set(['host', 'user', 'date']);
const token = /\{\{\s*([a-z][a-z0-9_]*)\s*\}\}/gi;

export function validateSnippet(snippet: Snippet): string | undefined {
  if (!snippet.name.trim() || !snippet.template.trim()) return 'Name and command are required.';
  if (snippet.template.length > 4000) return 'Snippet command is limited to 4000 characters.';
  let match: RegExpExecArray | null;
  token.lastIndex = 0;
  while ((match = token.exec(snippet.template))) {
    if (!allowed.has(match[1].toLowerCase())) return `Unsupported variable: ${match[1]}`;
  }
  if (/password|passphrase|private[_ -]?key|secret|token/i.test(snippet.template)) return 'Secret-like values are not allowed in snippets.';
  return undefined;
}

export function renderSnippet(template: string, context: SnippetContext): string {
  const values = { host: context.host ?? '', user: context.user ?? '', date: context.date ?? new Date().toISOString().slice(0, 10) };
  token.lastIndex = 0;
  return template.replace(token, (_, key: string) => values[key.toLowerCase() as keyof typeof values] ?? '');
}

export function searchSnippets(snippets: Snippet[], query: string): Snippet[] {
  const needle = query.trim().toLowerCase();
  return needle ? snippets.filter(item => `${item.name} ${item.template}`.toLowerCase().includes(needle)) : snippets;
}
