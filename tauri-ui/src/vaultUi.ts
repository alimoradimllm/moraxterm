import type { UiSettings } from './Settings';

export interface VaultPayload {
  version: 1;
  settings: Partial<UiSettings>;
}

export function readVaultPayload(value: unknown): Partial<UiSettings> {
  if (!value || typeof value !== 'object') throw new Error('vault payload is invalid');
  const payload = value as Partial<VaultPayload>;
  if (payload.version !== 1) throw new Error('unsupported vault version');
  if (!payload.settings || typeof payload.settings !== 'object' || Array.isArray(payload.settings)) {
    throw new Error('vault settings are invalid');
  }
  return payload.settings;
}
