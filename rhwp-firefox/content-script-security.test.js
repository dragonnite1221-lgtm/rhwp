import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const source = await readFile(new URL('./content-script.js', import.meta.url), 'utf8');

test('content script never performs automatic thumbnail prefetch', () => {
  assert.doesNotMatch(source, /prefetchThumbnails|drainPrefetchQueue|PREFETCH_CONCURRENCY/);
});

test('document open and thumbnail events require a real user event', () => {
  const checks = source.match(/\.isTrusted/g) || [];
  assert.ok(checks.length >= 3, `expected at least three isTrusted checks, got ${checks.length}`);
});
