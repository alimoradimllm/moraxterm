// Application and terminal themes. The CSS colour tokens for each application theme live in
// react-ui.css (:root[data-app-theme="..."]); this module only names them and holds the xterm palettes.

export const appThemeNames = Object.freeze(['mobaxterm', 'midnight', 'light', 'amber', 'neon']);
export const terminalThemeNames = Object.freeze(['mobaxterm', 'dark', 'light', 'amber', 'neon']);

const ansi = (c) => ({
  black: c[0], red: c[1], green: c[2], yellow: c[3], blue: c[4], magenta: c[5], cyan: c[6], white: c[7],
  brightBlack: c[8], brightRed: c[9], brightGreen: c[10], brightYellow: c[11], brightBlue: c[12],
  brightMagenta: c[13], brightCyan: c[14], brightWhite: c[15],
});

const terminalThemes = {
  mobaxterm: {
    background: '#080b0e', foreground: '#e6edf3', cursor: '#55d6ff', selectionBackground: '#27506b',
    ...ansi(['#1b2330', '#ff6b6b', '#34e0a1', '#ffd166', '#55b9ff', '#d17bff', '#2ee6c8', '#d7e0ea',
      '#4a5a6c', '#ff8f8f', '#6ef2bd', '#ffe08a', '#8acfff', '#e3a6ff', '#7cf2de', '#ffffff']),
  },
  dark: {
    background: '#070b14', foreground: '#dbe6f5', cursor: '#5aa9ff', selectionBackground: '#243b63',
    ...ansi(['#18233a', '#ff7a8a', '#5be3a8', '#f4d06f', '#6aa9ff', '#c49bff', '#5fd8e8', '#cfdbee',
      '#44577a', '#ff9aa6', '#86f0c1', '#ffe28f', '#93c1ff', '#d8b8ff', '#8fe9f5', '#ffffff']),
  },
  light: {
    background: '#fbfcfd', foreground: '#1c2733', cursor: '#0b7f6c', selectionBackground: '#bfe3dc',
    ...ansi(['#1c2733', '#c4262e', '#0d7a3f', '#8a6100', '#1c5fb8', '#8a3fc4', '#0a7c86', '#c9d2db',
      '#5a6b7c', '#e0454d', '#16a05a', '#b07a00', '#3b7fe0', '#a85ee0', '#12a0ad', '#ffffff']),
  },
  amber: {
    background: '#120e07', foreground: '#f3e7cf', cursor: '#ffb02e', selectionBackground: '#5a4216',
    ...ansi(['#2a2113', '#ff7b5c', '#b9d46a', '#ffb02e', '#e9a35b', '#e58fb0', '#8fcfb0', '#e6d8bb',
      '#6b5a3a', '#ff9c82', '#d3e88c', '#ffc95e', '#f4bd83', '#f2b0cb', '#b2e3ca', '#fff6e3']),
  },
  neon: {
    background: '#06060c', foreground: '#ecebff', cursor: '#00f5d4', selectionBackground: '#33307a',
    ...ansi(['#14142a', '#ff4d8d', '#00f5a0', '#fee440', '#4d9fff', '#c77dff', '#00f5d4', '#dcdcf5',
      '#52528a', '#ff7aa9', '#5dffc0', '#fff17a', '#7dbaff', '#dca6ff', '#6dfbe6', '#ffffff']),
  },
};

export function terminalThemeFor(name) {
  return terminalThemes[name] || terminalThemes.mobaxterm;
}

export function applyAppTheme(name) {
  const root = document.documentElement;
  root.dataset.appTheme = appThemeNames.includes(name) ? name : 'mobaxterm';
}
