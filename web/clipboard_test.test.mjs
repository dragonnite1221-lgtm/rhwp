import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('./clipboard_test.html', import.meta.url), 'utf8');

test('clipboard diagnostics render attacker-controlled values through DOM text nodes', () => {
  assert.doesNotMatch(source, /(?:results|\w+)\.innerHTML\s*=/);
  assert.match(source, /element\.textContent = text/);
  assert.match(source, /badge\.textContent = type/);
  assert.match(source, /pre\.textContent = text/);
  assert.match(source, /results\.replaceChildren\(\)/);
});

test('HTML preview remains sandboxed without granting the frame the parent origin', () => {
  assert.match(source, /frame\.setAttribute\('sandbox', ''\)/);
  assert.match(source, /frame\.srcdoc = data/);
  assert.match(source, /frame\.referrerPolicy = 'no-referrer'/);
});
