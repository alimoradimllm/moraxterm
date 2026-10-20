export const MAX_WORKSPACE_PANES = 16;

export function addPane(layout, id) {
  if (layout.includes(id) || layout.length >= MAX_WORKSPACE_PANES) return layout;
  return [...layout, id];
}

export function removePane(layout, id) { return layout.filter(value => value !== id); }

export function restoreLayout(value, available) {
  if (!Array.isArray(value)) return [];
  const allowed = new Set(available);
  return value.filter(id => typeof id === 'string' && allowed.has(id)).slice(0, MAX_WORKSPACE_PANES);
}
