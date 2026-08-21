import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const local = new Map();
const createdTabs = [];
let messageListener;

globalThis.browser = {
  runtime: {
    id: 'trusted',
    getURL: path => `safari-web-extension://trusted/${path}`,
    onMessage: { addListener(listener) { messageListener = listener; } },
    onInstalled: { addListener() {} },
  },
  storage: {
    // Safari 15 has no storage.session. The local fallback is TTL-bounded and
    // survives background worker suspension between grant creation and fetch.
    local: {
      async set(entries) {
        for (const [key, value] of Object.entries(entries)) local.set(key, value);
      },
      async get(query) {
        if (query === null) return Object.fromEntries(local);
        if (typeof query === 'string') {
          return local.has(query) ? { [query]: local.get(query) } : {};
        }
        const result = { ...(query || {}) };
        for (const key of Object.keys(result)) {
          if (local.has(key)) result[key] = local.get(key);
        }
        return result;
      },
      async remove(key) { local.delete(key); },
    },
  },
  tabs: { async create(options) { createdTabs.push(options); } },
  contextMenus: {
    async removeAll() {},
    create() {},
    onClicked: { addListener() {} },
  },
  i18n: { getMessage() { return ''; } },
  action: { onClicked: { addListener() {} } },
};

const {
  openViewer,
  validateContentTarget,
  validateMessage,
  verifyHwpSignature,
} = await import('./background.js');
const { validateFetchGrant } = await import('./sw/fetch-grants.js');

const sender = {
  id: 'trusted',
  url: 'https://example.com/page',
  tab: { id: 3 },
  frameId: 0,
};

test('Safari rejects forged senders but accepts user-selected public targets', () => {
  assert.equal(validateMessage({ type: 'open-hwp', url: 'https://example.com/a.hwp' }, {
    ...sender,
    id: 'forged',
  }).allowed, false);
  assert.equal(validateContentTarget('https://example.com/a.hwp', sender.url).allowed, true);
  assert.equal(validateContentTarget('https://cdn.example.net/a.hwp', sender.url).allowed, true);
  assert.equal(validateContentTarget('http://127.0.0.1/a.hwp', sender.url).allowed, false);
});

test('Safari viewer grants only public canonical URLs', async () => {
  assert.equal((await openViewer({ url: 'http://127.0.0.1/a.hwp', explicit: true })).ok, false);
  assert.equal((await openViewer({ url: 'https://example.com/a.hwp', explicit: true })).ok, true);
  const opened = new URL(createdTabs.at(-1).url);
  assert.equal(opened.searchParams.get('url'), 'https://example.com/a.hwp');
  const token = opened.searchParams.get('grant');
  assert.ok(token);
  const stored = local.get(`fetch-grant:${token}`);
  assert.match(stored.urlDigest, /^[0-9a-f]{64}$/);
  assert.equal('url' in stored, false);
  assert.equal(await validateFetchGrant(token, 'https://example.com/a.hwp'), true);
});

test('Safari iOS overlay preparation returns a grant-bearing internal viewer URL', async () => {
  const result = await messageListener({
    type: 'prepare-viewer',
    url: 'https://example.com/overlay.hwp',
    filename: 'overlay.hwp',
  }, sender);
  assert.equal(result.ok, true);
  const prepared = new URL(result.viewerUrl);
  assert.equal(prepared.protocol, 'safari-web-extension:');
  assert.equal(prepared.searchParams.get('url'), 'https://example.com/overlay.hwp');
  assert.ok(prepared.searchParams.get('grant'));
});

test('Safari HWP/HWPX signatures and Promise message replies are strict', async () => {
  assert.equal(verifyHwpSignature(new Uint8Array([0xD0, 0xCF, 0x11, 0xE0])), true);
  assert.equal(verifyHwpSignature(new Uint8Array([0x50, 0x4B, 0x03, 0x04])), true);
  assert.equal(verifyHwpSignature(new Uint8Array([0x50, 0x4B, 0x05, 0x06])), false);
  const result = messageListener({ type: 'unknown' }, sender);
  assert.equal(typeof result?.then, 'function');
  assert.match((await result).error, /알 수 없는 메시지 유형/);
});

test('Safari manifest uses a non-persistent script event page and HTTP(S)-only host access', async () => {
  const manifest = JSON.parse(await readFile(new URL('./manifest.json', import.meta.url), 'utf8'));
  assert.deepEqual(manifest.background.scripts, ['background.js']);
  assert.equal(manifest.background.persistent, false);
  assert.deepEqual(manifest.host_permissions, ['http://*/*', 'https://*/*']);
  assert.doesNotMatch(JSON.stringify(manifest), /<all_urls>/);
});

test('Safari fetch responses use the shared one-time binary transfer store', async () => {
  const source = await readFile(new URL('./background.js', import.meta.url), 'utf8');
  assert.match(source, /storeDocumentTransfer\(result\.data, result\.contentType\)/);
  assert.doesNotMatch(source, /return \{ data: result\.data\.buffer/);
});
