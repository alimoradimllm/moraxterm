import type { Session } from './react-types';

export interface FolderChange {
  ok: boolean;
  noop?: boolean;
  error?: string;
  sessions?: Session[];
  folders?: string[];
  changed?: Session[];
  moved?: number;
  created?: string;
  renamed?: string;
  parent?: string;
}

export type Row =
  | { kind: 'folder'; path: string; name: string; depth: number; count: number; collapsed: boolean; hasChildren: boolean }
  | { kind: 'session'; session: Session; depth: number };

export function sessionKey(session: Pick<Session, 'session_type' | 'name'>): string;
export function normalizePath(path?: string): string;
export function parentOf(path: string): string;
export function leafOf(path: string): string;
export function joinPath(parent: string, name: string): string;
export function isInside(path: string, folder: string): boolean;
export function validateFolderName(name: string): string;
export function allFolders(sessions: Session[], extra?: string[]): string[];
export function buildRows(input: { sessions: Session[]; folders?: string[]; collapsed?: Set<string>; query?: string }): Row[];
export function moveSession(sessions: Session[], key: string, folder: string): FolderChange;
export function createFolder(sessions: Session[], folders: string[], parent: string, name: string): FolderChange;
export function renameFolder(sessions: Session[], folders: string[], from: string, newName: string): FolderChange;
export function moveFolder(sessions: Session[], folders: string[], from: string, toParent: string): FolderChange;
export function deleteFolder(sessions: Session[], folders: string[], path: string): FolderChange;
