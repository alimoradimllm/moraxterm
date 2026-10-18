import test from 'node:test';
import assert from 'node:assert/strict';
import { iconFor, protocolIconTypes, toolIconFor, toolIconNames, toolColor, colorFor, logoSvg } from '../src/icons.js';

const body = svg => svg.replace(/^<svg [^>]*>/, '').replace(/<\/svg>$/, '');

test('exports a colorful accessible icon for every protocol', () => {
  for (const type of protocolIconTypes) {
    const svg = iconFor(type);
    assert.match(svg, /^<svg /);
    assert.match(svg, /aria-hidden="true"/);
    assert.match(svg, /stroke="#[0-9a-f]{6}"/i);
  }
});

test('unknown protocol uses the shell fallback icon', () => {
  assert.equal(iconFor('unknown'), iconFor('Shell'));
});

test('every protocol has its own colour and its own shape', () => {
  const colours = protocolIconTypes.map(colorFor);
  const shapes = protocolIconTypes.map(type => body(iconFor(type)).replace(/#[0-9a-f]{6}/gi, ''));
  assert.equal(new Set(colours).size, protocolIconTypes.length, `duplicate protocol colours: ${colours}`);
  assert.equal(new Set(shapes).size, protocolIconTypes.length, 'two protocols share the same graphic');
});

test('every toolbar and navigation icon has its own colour and its own shape', () => {
  assert.ok(toolIconNames.length >= 15);
  const colours = toolIconNames.map(toolColor);
  const shapes = toolIconNames.map(name => body(toolIconFor(name)).replace(/#[0-9a-f]{6}/gi, ''));
  const dupColours = colours.filter((colour, index) => colours.indexOf(colour) !== index);
  assert.deepEqual(dupColours, [], `duplicate tool colours: ${dupColours}`);
  assert.equal(new Set(shapes).size, toolIconNames.length, 'two tools share the same graphic');
});

test('tool icons are two-tone: a tinted fill plus a stroke of the same colour', () => {
  for (const name of toolIconNames) {
    const svg = toolIconFor(name);
    assert.match(svg, /aria-hidden="true"/);
    assert.ok(svg.includes(`stroke="${toolColor(name)}"`), `${name} stroke`);
    assert.ok(!svg.includes('data-f'), `${name} left a fill marker unreplaced`);
  }
});

test('no colour is reused between a protocol icon and a tool icon', () => {
  const protocols = new Set(protocolIconTypes.map(colorFor));
  const shared = toolIconNames.map(toolColor).filter(colour => protocols.has(colour));
  assert.deepEqual(shared, [], `colours shared by protocols and tools: ${shared}`);
});

test('the brand logo is an accessible inline svg', () => {
  assert.match(logoSvg, /^<svg /);
  assert.match(logoSvg, /aria-hidden="true"/);
});
