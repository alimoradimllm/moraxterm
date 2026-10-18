// Two-tone line icons: every protocol and every toolbar action has its own colour AND its own shape.
// `data-f` marks the shapes that get a soft tint of the icon colour; everything else is a plain stroke.
const FILL = 'data-f';
const make = (color, body, cls) =>
  `<svg class="${cls}" viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="${color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${body.replaceAll(FILL, `fill="${color}" fill-opacity=".26"`)}</svg>`;

// Session / protocol icons (sidebar, tabs, editor, home).
const icons = {
  SSH: ['#ffd166', '<rect data-f x="4" y="10" width="16" height="10" rx="2"/><path d="M7 10V8a5 5 0 0 1 10 0v2"/><circle cx="12" cy="15" r="1.2"/>'],
  Telnet: ['#4cd68a', '<circle data-f cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>'],
  Rdp: ['#5aa9ff', '<rect data-f x="3" y="4" width="18" height="12" rx="2"/><path d="M8 21h8M12 16v5"/>'],
  Vnc: ['#ff8a5c', '<path data-f d="M2 12s3.6-6.5 10-6.5S22 12 22 12s-3.6 6.5-10 6.5S2 12 2 12z"/><circle cx="12" cy="12" r="2.6"/>'],
  WSL: ['#b18cff', '<rect data-f x="3" y="4" width="18" height="16" rx="3"/><path d="m8 9 3 3-3 3M13 15h3.5"/>'],
  Shell: ['#6ec6ff', '<rect data-f x="3" y="4" width="18" height="16" rx="2"/><path d="M3 8.5h18M7 13l2.5 2L7 17"/>'],
  Serial: ['#ffb454', '<rect data-f x="7" y="7" width="10" height="10" rx="1.5"/><path d="M10 7V3M14 7V3M10 21v-4M14 21v-4M7 10H3M7 14H3M21 10h-4M21 14h-4"/>'],
  Unix: ['#2ee6c8', '<path data-f d="M12 2.8 20 7.4v9.2l-8 4.6-8-4.6V7.4z"/><path d="m9 10.5 2.5 2L9 14.5M13 14.5h2.5"/>'],
};

// Toolbar / navigation icons.
const tools = {
  session: ['#3ddc97', '<rect data-f x="3" y="4" width="18" height="16" rx="2.5"/><path d="M12 9v6M9 12h6"/>'],
  servers: ['#4d9fff', '<rect data-f x="3" y="3.5" width="18" height="7" rx="2"/><rect data-f x="3" y="13.5" width="18" height="7" rx="2"/><path d="M7 7h.01M7 17h.01M12 7h5M12 17h5"/>'],
  tools: ['#ffb02e', '<path data-f d="M14.7 6.3a4 4 0 0 0-5.4 5.2l-5.8 5.8a1.8 1.8 0 0 0 2.5 2.5l5.8-5.8a4 4 0 0 0 5.2-5.4l-2.5 2.5-2.3-.6-.6-2.3z"/>'],
  sessions: ['#f6c945', '<path data-f d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.5 2.9 1-6.1-4.4-4.3 6.1-.9z"/>'],
  view: ['#3cc9f0', '<rect data-f x="8" y="8" width="8" height="8" rx="1"/><path d="M4 9V5a1 1 0 0 1 1-1h4M20 9V5a1 1 0 0 0-1-1h-4M4 15v4a1 1 0 0 0 1 1h4M20 15v4a1 1 0 0 1-1 1h-4"/>'],
  split: ['#8f7cff', '<rect data-f x="3" y="4" width="18" height="16" rx="2"/><path d="M12 4v16"/>'],
  duplicate: ['#d17bff', '<rect data-f x="9" y="9" width="12" height="12" rx="2"/><path d="M15 9V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h3"/>'],
  multiexec: ['#ff7a3d', '<circle data-f cx="5.5" cy="12" r="2.5"/><path d="M8 12h12M8 12l6-6h6M8 12l6 6h6"/>'],
  tunnel: ['#2ec4b6', '<circle data-f cx="4.5" cy="12" r="2.3"/><circle data-f cx="19.5" cy="12" r="2.3"/><path d="M7.2 12h9.6M13.8 9l3 3-3 3"/>'],
  sftp: ['#7bd957', '<path data-f d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M12 17v-6M9.5 13.5 12 11l2.5 2.5"/>'],
  settings: ['#8fa8c8', '<circle data-f cx="16" cy="7" r="2.2"/><circle data-f cx="8" cy="17" r="2.2"/><path d="M4 7h9.8M18.2 7H20M4 17h1.8M10.2 17H20"/>'],
  help: ['#ff7eb6', '<circle data-f cx="12" cy="12" r="9"/><path d="M9.6 9.6a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1.1.9-1.1 1.7M12 17h.01"/>'],
  xserver: ['#e86ad0', '<rect data-f x="3" y="4" width="18" height="13" rx="2"/><path d="m9.5 8.5 5 4.5M14.5 8.5l-5 4.5M9 21h6M12 17v4"/>'],
  exit: ['#ff5d5d', '<path d="M12 3v8"/><path data-f d="M6.3 6.8a8 8 0 1 0 11.4 0"/>'],
  home: ['#ff9f43', '<path data-f d="m3 11 9-8 9 8v8a2 2 0 0 1-2 2h-4v-6H9v6H5a2 2 0 0 1-2-2z"/>'],
  folderPlus: ['#ffc857', '<path data-f d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M12 11v5M9.5 13.5h5"/>'],
  edit: ['#5ac8fa', '<path data-f d="m4 20 1-4L16.5 4.5a2.1 2.1 0 0 1 3 3L8 19z"/><path d="m14.5 6.5 3 3"/>'],
  refresh: ['#6ee7a8', '<path d="M20 11a8 8 0 0 0-14.3-4.6L4 8"/><path d="M4 4v4h4M4 13a8 8 0 0 0 14.3 4.6L20 16"/><path d="M20 20v-4h-4"/>'],
  monitor: ['#4dd0e1', '<rect data-f x="3" y="4" width="18" height="13" rx="2"/><path d="M6 12l3-3 2.5 2.5L15 8l3 3M9 21h6"/>'],
  recover: ['#b388ff', '<path data-f d="M4 12a8 8 0 1 0 2.4-5.7L4 8.6"/><path d="M4 4v4.6h4.6M12 8v4l3 2"/>'],
  play: ['#34e0a1', '<path data-f d="m8 5 11 7-11 7z"/>'],
};

export const protocolIconTypes = Object.freeze(Object.keys(icons));
export const toolIconNames = Object.freeze(Object.keys(tools));

export function iconFor(type) {
  const [color, body] = icons[type] || icons.Shell;
  return make(color, body, 'protocol-icon');
}

export function colorFor(type) {
  return (icons[type] || icons.Shell)[0];
}

export function toolIconFor(name) {
  const [color, body] = tools[name] || tools.session;
  return make(color, body, 'tool-icon');
}

export function toolColor(name) {
  return (tools[name] || tools.session)[0];
}

// The brand mark: a rounded tile with a prompt. The gradient is the only gradient in the app.
export const logoSvg = '<svg class="logo-mark" viewBox="0 0 64 64" aria-hidden="true" focusable="false"><defs><linearGradient id="mx-logo" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#2ee6c8"/><stop offset=".55" stop-color="#4d9fff"/><stop offset="1" stop-color="#e86ad0"/></linearGradient></defs><rect x="4" y="4" width="56" height="56" rx="15" fill="url(#mx-logo)"/><path d="m20 22 12 10-12 10" fill="none" stroke="#fff" stroke-width="6" stroke-linecap="round" stroke-linejoin="round"/><path d="M36 44h12" fill="none" stroke="#fff" stroke-width="6" stroke-linecap="round"/></svg>';
