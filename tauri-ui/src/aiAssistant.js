const SECRET_PATTERNS = [
  /(?:password|passwd|passphrase|token|api[_-]?key|secret)\s*[=:]\s*[^\s]+/gi,
  /-----BEGIN [^-]+-----[\s\S]*?-----END [^-]+-----/g,
];

export function redactSecrets(text) {
  return SECRET_PATTERNS.reduce((value, pattern) => value.replace(pattern, '[REDACTED]'), String(text ?? ''));
}

export function createAiProvider(config) {
  const provider = config?.provider ?? 'none';
  return {
    name: provider,
    enabled: Boolean(config?.enabled) && provider !== 'none',
    preview: async (prompt) => ({
      supported: false,
      previewOnly: true,
      text: provider === 'none' ? 'AI assistance is disabled. Configure an approved local provider in Settings.' : `Provider “${provider}” is configured but no runtime adapter is installed.`,
      prompt: redactSecrets(prompt),
    }),
  };
}
