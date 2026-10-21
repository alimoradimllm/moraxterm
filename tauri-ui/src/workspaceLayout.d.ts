export const MAX_WORKSPACE_PANES: 16;
export function addPane(layout: string[], id: string): string[];
export function removePane(layout: string[], id: string): string[];
export function restoreLayout(value: unknown, available: string[]): string[];
