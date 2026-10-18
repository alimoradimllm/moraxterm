import type { Session } from './react-types';

/** The common shape every sidebar change reports: records to save, then names to delete. */
export interface Change {
  ok: boolean;
  noop?: boolean;
  error?: string;
  sessions?: Session[];
  folders?: string[];
  changed?: Session[];
  deleted?: string[];
}

export interface SessionChange {
  ok: boolean;
  error?: string;
  sessions?: Session[];
  changed?: Session[];
  deleted?: string[];
  created?: boolean;
  renamed?: { from: string; to: string };
  duplicated?: string;
  favorite?: boolean;
  removed?: string;
}

export function uniqueName(sessions: Session[], base: string): string;
export function planSave(sessions: Session[], original: Session | undefined, draft: Session): SessionChange;
export function duplicateSession(sessions: Session[], key: string): SessionChange;
export function toggleFavorite(sessions: Session[], key: string): SessionChange;
export function removeSession(sessions: Session[], key: string): SessionChange;
