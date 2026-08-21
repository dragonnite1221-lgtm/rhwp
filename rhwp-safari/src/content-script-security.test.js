import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const source = await readFile(new URL('./content-script.js', import.meta.url), 'utf8');

test('Safari privileged UI actions require browser-trusted events', () => {
  const checks = source.match(/\.isTrusted/g) || [];
  assert.ok(checks.length >= 4, `expected at least four isTrusted checks, got ${checks.length}`);
});

test('Safari content script has no automatic thumbnail prefetch queue', () => {
  assert.doesNotMatch(source, /prefetch|drainPrefetchQueue|PREFETCH_CONCURRENCY/i);
});

test('Safari iOS overlay obtains its grant-bearing viewer URL from the background', () => {
  assert.match(source, /type:\s*'prepare-viewer'/);
  assert.match(source, /iframe\.src\s*=\s*prepared\.viewerUrl/);
  assert.doesNotMatch(source, /params\.set\('url',\s*url\)/);
});
