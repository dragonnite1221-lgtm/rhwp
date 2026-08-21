import test from 'node:test';
import assert from 'node:assert/strict';

const values = new Map();
globalThis.chrome = {
  runtime: {
    getURL: path => `chrome-extension://trusted/${path}`,
  },
  storage: {
    local: {
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

test('missing and renewal-expired grants fail closed', async () => {
  assert.equal(await validateFetchGrant('missing', 'https://example.com/a.hwp'), false);
  const token = await createFetchGrant('https://example.com/expired.hwp');
  const [key] = [...values.keys()].filter(item => item.endsWith(token));
  values.set(key, { ...values.get(key), expiresAt: Date.now() - 1, renewUntil: Date.now() - 1 });
  assert.equal(await validateFetchGrant(token, 'https://example.com/expired.hwp'), false);
  assert.equal(values.has(key), false);

  const corrupt = await createFetchGrant('https://example.com/corrupt.hwp');
  const [corruptKey] = [...values.keys()].filter(item => item.endsWith(corrupt));
  values.set(corruptKey, { ...values.get(corruptKey), expiresAt: undefined });
  assert.equal(await validateFetchGrant(corrupt, 'https://example.com/corrupt.hwp'), false);
  assert.equal(values.has(corruptKey), false);
});

test('an exact-URL grant renews within its absolute recovery window', async () => {
  const token = await createFetchGrant('https://example.com/reload.hwp');
  const key = [...values.keys()].find(item => item.endsWith(token));
  const renewUntil = Date.now() + 60_000;
  values.set(key, { ...values.get(key), expiresAt: Date.now() - 1, renewUntil });
  assert.equal(await validateFetchGrant(token, 'https://example.com/reload.hwp'), true);
  assert.ok(values.get(key).expiresAt > Date.now());
  assert.equal(values.get(key).renewUntil, renewUntil);
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
