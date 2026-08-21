import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

let listener;
globalThis.browser = {
  runtime: {
    id: 'trusted',
    getURL: path => `moz-extension://trusted/${path}`,
    onMessage: { addListener(value) { listener = value; } },
  },
  storage: {
    sync: { async get(defaults) { return defaults; } },
  },
};

const {
  setupMessageRouter,
  validateContentTarget,
  validateMessage,
} = await import('./message-router.js');

const contentSender = {
  id: 'trusted',
  url: 'https://example.com/page',
  tab: { id: 4 },
  frameId: 0,
};

test('Firefox content-script targets remain same-origin', () => {
  assert.equal(validateContentTarget(
    'https://example.com/files/a.hwp', contentSender.url,
  ).allowed, true);
  assert.equal(validateContentTarget(
    'https://cdn.attacker.test/a.hwp', contentSender.url,
  ).allowed, false);
});

test('Firefox message validation rejects forged senders and invalid payloads', () => {
  assert.equal(validateMessage({ type: 'open-hwp', url: 'https://example.com/a.hwp' }, {
    ...contentSender,
    id: 'forged',
  }).allowed, false);
  assert.equal(validateMessage(null, contentSender).allowed, false);
  assert.equal(validateMessage({ type: 'unknown' }, contentSender).allowed, false);
});

test('Firefox router returns a Promise and fails unknown messages closed', async () => {
  setupMessageRouter();
  assert.equal(typeof listener, 'function');
  const response = listener({ type: 'unknown' }, contentSender);
  assert.equal(typeof response?.then, 'function');
  assert.match((await response).error, /알 수 없는 메시지 유형/);
});

test('Firefox fetch responses use the shared one-time binary transfer store', async () => {
  const source = await readFile(new URL('./message-router.js', import.meta.url), 'utf8');
  assert.match(source, /storeDocumentTransfer\(result\.data, result\.contentType\)/);
  assert.doesNotMatch(source, /return \{ data: result\.data\.buffer/);
});
