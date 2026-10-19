// Pure helpers for the SFTP transfer meter, kept out of the component so they can be unit tested.

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** 0 -> "0 B", 1536 -> "1.5 KB", 1500000 -> "1.4 MB". Anything that is not a non-negative number shows as "0 B". */
export function formatBytes(value) {
  let n = Number(value);
  if (!Number.isFinite(n) || n < 0) n = 0;
  let unit = 0;
  while (n >= 1024 && unit < UNITS.length - 1) { n /= 1024; unit += 1; }
  const text = unit === 0 || n >= 100 ? String(Math.round(n)) : n.toFixed(1).replace(/\.0$/, '');
  return `${text} ${UNITS[unit]}`;
}

/** A whole percent from 0 to 100, or null when the total is unknown, so the UI can show "unknown" instead of a made-up number. */
export function percent(done, total) {
  const d = Number(done), t = Number(total);
  if (!Number.isFinite(d) || !Number.isFinite(t) || t <= 0 || d < 0) return null;
  return Math.min(100, Math.floor((d / t) * 100));
}

/** The line shown next to the bar. */
export function describeProgress(done, total) {
  const p = percent(done, total);
  if (done == null) return 'Starting…';
  if (p === null) return `${formatBytes(done)} transferred (size unknown)`;
  return `${formatBytes(done)} of ${formatBytes(total)} (${p}%)`;
}

/**
 * Folds a stream of backend progress events into one value that never goes backwards or above the total, so a late or reordered event
 * cannot make the bar jump back. Events from other jobs are ignored.
 */
export function applyProgress(current, event, jobId) {
  if (!event || event.id !== jobId || typeof event.transferred_bytes !== 'number') return current;
  const total = typeof event.total_bytes === 'number' && event.total_bytes > 0 ? event.total_bytes : (current?.total ?? null);
  let done = event.transferred_bytes;
  if (total !== null && done > total) done = total;
  if (current && current.done > done) done = current.done;
  return { done, total };
}
