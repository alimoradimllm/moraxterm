export function isTerminalReply(data: string): boolean;
export function mirrorTargets(args: {
  on: boolean;
  sourceId: string;
  tabs: { id: string; state: string }[];
  excluded: Set<string>;
  data: string;
}): string[];
export function pasteTargets(args: { tabs: { id: string; state: string }[]; excluded: Set<string> }): string[];
export function isMultiPasteShortcut(event: { ctrlKey: boolean; shiftKey: boolean; altKey: boolean; key: string; code: string }): boolean;
export function canStop(tab: { state: string }): boolean;
