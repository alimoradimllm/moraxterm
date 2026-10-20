export function redactSecrets(text: unknown): string;
export function createAiProvider(config?: { enabled?: boolean; provider?: string }): { name: string; enabled: boolean; preview: (prompt: string) => Promise<{ supported: boolean; previewOnly: true; text: string; prompt: string }> };
