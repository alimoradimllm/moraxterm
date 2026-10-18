import type { ITheme } from '@xterm/xterm';

export const appThemeNames: readonly string[];
export const terminalThemeNames: readonly string[];
export function terminalThemeFor(name: string): ITheme & { background: string };
export function applyAppTheme(name: string): void;
