export function formatBytes(value: unknown): string;
export function percent(done: unknown, total: unknown): number | null;
export function describeProgress(done: number | null | undefined, total: number | null | undefined): string;
export interface Meter { done: number; total: number | null; }
export interface ProgressEvent { id?: string; transferred_bytes?: number | null; total_bytes?: number | null; success?: boolean; message?: string; }
export function applyProgress(current: Meter | null, event: ProgressEvent | null | undefined, jobId: string): Meter | null;
