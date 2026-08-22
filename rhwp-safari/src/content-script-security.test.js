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

test('Safari validates thumbnail URLs immediately before assigning img.src', () => {
  assert.match(source, /SAFE_THUMBNAIL_DATA_URI\s*=\s*\/\^data:image\\\//);
  assert.match(source, /function normalizeSafeThumbnailSource\(value\)/);
  assert.match(source, /const safeSrc = normalizeSafeThumbnailSource\(src\);\s*if \(!safeSrc\) return false;\s*const img = document\.createElement\('img'\);\s*img\.src = safeSrc;/s);
  assert.doesNotMatch(source, /img\.src = src;/);
});

test('Safari iOS overlay obtains its grant-bearing viewer URL from the background', () => {
  assert.match(source, /type:\s*'prepare-viewer'/);
  assert.match(source, /iframe\.src\s*=\s*prepared\.viewerUrl/);
  assert.doesNotMatch(source, /params\.set\('url',\s*url\)/);
});
