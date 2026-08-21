import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { validateContentTarget } from './message-router.js';

test('user-selected public documents may use a different origin', () => {
  assert.equal(validateContentTarget(
    'https://example.com/files/a.hwp',
    'https://example.com/page',
  ).allowed, true);
  assert.equal(validateContentTarget(
    'https://cdn.attacker.test/a.hwp',
    'https://example.com/page',
  ).allowed, true);
  assert.equal(validateContentTarget(
    'http://127.0.0.1/a.hwp',
    'https://example.com/page',
  ).allowed, false);
});

test('GitHub blob and raw document URLs remain allowed', () => {
  assert.equal(validateContentTarget(
    'https://github.com/owner/repo/blob/main/a.hwp',
    'https://github.com/owner/repo/blob/main/README.md',
  ).allowed, true);
  assert.equal(validateContentTarget(
    'https://raw.githubusercontent.com/owner/repo/main/a.hwp',
    'https://github.com/owner/repo',
  ).allowed, true);
});

test('fetch responses use one-time transfer IDs instead of JSON-serialized binary', async () => {
  const source = await readFile(new URL('./message-router.js', import.meta.url), 'utf8');
  assert.match(source, /storeDocumentTransfer\(result\.data, result\.contentType\)/);
  assert.doesNotMatch(source, /return \{ data: result\.data\.buffer/);
});
