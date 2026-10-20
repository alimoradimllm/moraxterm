import React from 'react';
import { createRoot } from 'react-dom/client';
import '@xterm/xterm/css/xterm.css';
import './react-ui.css';
import App from './App';
import DetachedTerminal from './DetachedTerminal';

const root = document.getElementById('app');
if (!root) throw new Error('Moraxterm root element is missing');
// A terminal opened in its own window loads the same page with ?detached=<terminal id>; only plain identifier characters are accepted.
const detached = new URLSearchParams(window.location.search).get('detached');
const page = detached && /^[A-Za-z0-9_-]{1,80}$/.test(detached) ? <DetachedTerminal id={detached} /> : <App />;
createRoot(root).render(<React.StrictMode>{page}</React.StrictMode>);
