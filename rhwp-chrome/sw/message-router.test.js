import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { validateContentTarget } from './message-router.js';

test('content-script document requests stay on the sender origin', () => {
  assert.equal(validateContentTarget(
    'https://example.com/files/a.hwp',
    'https://example.com/page',
  ).allowed, true);
  assert.equal(validateContentTarget(
    'https://cdn.attacker.test/a.hwp',
    'https://example.com/page',
  ).allowed, false);
});

test('the exact GitHub blob-to-raw provider adapter remains allowed', () => {
  assert.equal(validateContentTarget(
    'https://github.com/owner/repo/blob/main/a.hwp',
    'https://github.com/owner/repo/blob/main/README.md',
  ).allowed, true);
  assert.equal(validateContentTarget(
    'https://raw.githubusercontent.com/owner/repo/main/a.hwp',
    'https://github.com/owner/repo',
  ).allowed, false);
});

test('fetch responses use one-time transfer IDs instead of JSON-serialized binary', async () => {
  const source = await readFile(new URL('./message-router.js', import.meta.url), 'utf8');
  assert.match(source, /storeDocumentTransfer\(result\.data, result\.contentType\)/);
  assert.doesNotMatch(source, /return \{ data: result\.data\.buffer/);
});
