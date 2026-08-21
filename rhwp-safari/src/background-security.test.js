import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

const session = new Map();
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
    session: {
      async set(entries) {
        for (const [key, value] of Object.entries(entries)) session.set(key, value);
      },
      async get(key) { return session.has(key) ? { [key]: session.get(key) } : {}; },
      async remove(key) { session.delete(key); },
    },
    local: {
      async get(defaults) { return defaults; },
      async set() {},
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

const sender = {
  id: 'trusted',
  url: 'https://example.com/page',
  tab: { id: 3 },
  frameId: 0,
};

test('Safari sender and target checks fail forged and cross-origin messages closed', () => {
  assert.equal(validateMessage({ type: 'open-hwp', url: 'https://example.com/a.hwp' }, {
    ...sender,
    id: 'forged',
  }).allowed, false);
  assert.equal(validateContentTarget('https://example.com/a.hwp', sender.url).allowed, true);
  assert.equal(validateContentTarget('https://attacker.test/a.hwp', sender.url).allowed, false);
});

test('Safari viewer grants only public canonical URLs', async () => {
  assert.equal((await openViewer({ url: 'http://127.0.0.1/a.hwp', explicit: true })).ok, false);
  assert.equal((await openViewer({ url: 'https://example.com/a.hwp', explicit: true })).ok, true);
  const opened = new URL(createdTabs.at(-1).url);
  assert.equal(opened.searchParams.get('url'), 'https://example.com/a.hwp');
  assert.ok(opened.searchParams.get('grant'));
});

test('Safari HWP/HWPX signatures and Promise message replies are strict', async () => {
  assert.equal(verifyHwpSignature(new Uint8Array([0xD0, 0xCF, 0x11, 0xE0])), true);
  assert.equal(verifyHwpSignature(new Uint8Array([0x50, 0x4B, 0x03, 0x04])), true);
  assert.equal(verifyHwpSignature(new Uint8Array([0x50, 0x4B, 0x05, 0x06])), false);
  const result = messageListener({ type: 'unknown' }, sender);
  assert.equal(typeof result?.then, 'function');
  assert.match((await result).error, /알 수 없는 메시지 유형/);
});

test('Safari manifest uses a module service worker and HTTP(S)-only host access', async () => {
  const manifest = JSON.parse(await readFile(new URL('./manifest.json', import.meta.url), 'utf8'));
  assert.equal(manifest.background.service_worker, 'background.js');
  assert.equal(manifest.background.type, 'module');
  assert.deepEqual(manifest.host_permissions, ['http://*/*', 'https://*/*']);
  assert.doesNotMatch(JSON.stringify(manifest), /<all_urls>/);
});
