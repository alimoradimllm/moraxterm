export function filterPaletteCommands(commands, query) {
  const needle = query.trim().toLowerCase();
  return commands.filter(command => command.label.toLowerCase().includes(needle));
}

export function isPaletteShortcut(event) {
  return (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'j';
}
