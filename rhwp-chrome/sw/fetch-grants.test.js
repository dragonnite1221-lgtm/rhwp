import test from 'node:test';
import assert from 'node:assert/strict';

const values = new Map();
globalThis.chrome = {
  runtime: {
    getURL: path => `chrome-extension://trusted/${path}`,
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

test('grant is bound to one canonical URL', async () => {
  const token = await createFetchGrant('https://example.com/a.hwp');
  assert.equal(await validateFetchGrant(token, 'https://example.com/a.hwp'), true);
  assert.equal(await validateFetchGrant(token, 'https://example.com/b.hwp'), false);
});

test('missing and expired grants fail closed', async () => {
  assert.equal(await validateFetchGrant('missing', 'https://example.com/a.hwp'), false);
  const token = await createFetchGrant('https://example.com/expired.hwp');
  const [key] = [...values.keys()].filter(item => item.endsWith(token));
  values.set(key, { ...values.get(key), expiresAt: Date.now() - 1 });
  assert.equal(await validateFetchGrant(token, 'https://example.com/expired.hwp'), false);
  assert.equal(values.has(key), false);
});

test('viewer URL carries a grant bound to the resolved document URL', async () => {
  const viewerUrl = await buildViewerUrl(
    'chrome-extension://trusted/viewer.html',
    { url: 'https://example.com/document.hwp' },
  );
  const parsed = new URL(viewerUrl);
  const token = parsed.searchParams.get('grant');
  const documentUrl = parsed.searchParams.get('url');
  assert.ok(token);
  assert.equal(documentUrl, 'https://example.com/document.hwp');
  assert.equal(await validateFetchGrant(token, documentUrl), true);
});
