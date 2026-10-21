import type { UiSettings } from './Settings';
import type { Session } from './react-types';

export interface SecretRecord {
  reference: string;
  kind: 'password' | 'private-key';
  value?: string;
}

export interface VaultPayload {
  version: 1;
  settings?: Partial<UiSettings>;
  sessions?: Session[];
  secrets?: SecretRecord[];
  providers?: Record<string, unknown>[];
}

export interface RestoredVault {
  settings: Partial<UiSettings>;
  sessions: Session[];
  secrets: SecretRecord[];
  providers: Record<string, unknown>[];
}

export function readVaultPayload(value: unknown): Partial<UiSettings> {
  const restored = readFullVaultPayload(value);
  return restored.settings;
}

export function readFullVaultPayload(value: unknown): RestoredVault {
  if (!value || typeof value !== 'object') throw new Error('vault payload is invalid');
  const payload = value as Record<string, unknown>;
  if (payload.version !== 1) throw new Error('unsupported vault version');

  let settings: Partial<UiSettings> = {};
  if (payload.settings && typeof payload.settings === 'object' && !Array.isArray(payload.settings)) {
    settings = payload.settings as Partial<UiSettings>;
  } else if (Array.isArray(payload.providers) && payload.providers.length > 0) {
    const first = payload.providers[0];
    if (first && typeof first === 'object' && !Array.isArray(first)) {
      settings = first as Partial<UiSettings>;
    }
  }

  const sessions = Array.isArray(payload.sessions) ? (payload.sessions as Session[]) : [];
  const secrets = Array.isArray(payload.secrets) ? (payload.secrets as SecretRecord[]) : [];
  const providers = Array.isArray(payload.providers) ? (payload.providers as Record<string, unknown>[]) : [];

  return { settings, sessions, secrets, providers };
}
