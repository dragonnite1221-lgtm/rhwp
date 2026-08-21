import assert from 'node:assert/strict';
import { test } from 'node:test';

const values = new Map();
globalThis.browser = {
  runtime: {
    id: 'trusted',
    getURL: path => `moz-extension://trusted/${path}`,
  },
  storage: {
    session: {
      async set(entries) {
        for (const [key, value] of Object.entries(entries)) values.set(key, value);
      },
      async get(key) {
        return values.has(key) ? { [key]: values.get(key) } : {};
      },
      async remove(key) {
        values.delete(key);
      },
    },
  },
};

const { createFetchGrant, validateFetchGrant } = await import('./fetch-grants.js');
const { buildViewerUrl } = await import('./viewer-launcher.js');

test('Firefox grants bind one canonical URL and expire closed', async () => {
  const token = await createFetchGrant('https://example.com/a.hwp');
  assert.equal(await validateFetchGrant(token, 'https://example.com/a.hwp'), true);
  assert.equal(await validateFetchGrant(token, 'https://example.com/b.hwp'), false);

  const expired = await createFetchGrant('https://example.com/expired.hwp');
  const key = [...values.keys()].find(item => item.endsWith(expired));
  values.set(key, { ...values.get(key), expiresAt: Date.now() - 1 });
  assert.equal(await validateFetchGrant(expired, 'https://example.com/expired.hwp'), false);
  assert.equal(values.has(key), false);
});

test('Firefox viewer URL contains a grant for the resolved document URL', async () => {
  const viewerUrl = await buildViewerUrl('moz-extension://trusted/viewer.html', {
    url: 'https://example.com/document.hwp',
  });
  const parsed = new URL(viewerUrl);
  const token = parsed.searchParams.get('grant');
  const documentUrl = parsed.searchParams.get('url');
  assert.ok(token);
  assert.equal(await validateFetchGrant(token, documentUrl), true);
});
