export type PaletteCommand = { label: string; run: () => void };
export function filterPaletteCommands(commands: PaletteCommand[], query: string): PaletteCommand[];
export function isPaletteShortcut(event: Pick<KeyboardEvent, 'ctrlKey' | 'metaKey' | 'key'>): boolean;
