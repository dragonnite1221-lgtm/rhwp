import test from 'node:test';
import assert from 'node:assert/strict';

import { validateSender } from './sender-validator.js';

globalThis.chrome = {
  runtime: {
    id: 'trusted-extension-id',
    getURL: path => `chrome-extension://trusted-extension-id/${path}`,
  },
};

const contentSender = {
  id: 'trusted-extension-id',
  url: 'https://example.com/page',
  tab: { id: 7 },
  frameId: 0,
};
const internalSender = {
  id: 'trusted-extension-id',
  url: 'chrome-extension://trusted-extension-id/viewer.html',
};

test('privileged fetch is restricted to the extension origin', () => {
  assert.equal(validateSender('fetch-file', internalSender).allowed, true);
  assert.equal(validateSender('fetch-file', contentSender).allowed, false);
});

test('content-script actions require extension id, tab, frame, and web URL', () => {
  assert.equal(validateSender('open-hwp', contentSender).allowed, true);
  assert.equal(validateSender('extract-thumbnail', contentSender).allowed, true);
  assert.equal(validateSender('open-hwp', { ...contentSender, id: 'attacker' }).allowed, false);
  assert.equal(validateSender('open-hwp', { ...contentSender, frameId: undefined }).allowed, false);
  assert.equal(validateSender('open-hwp', { ...contentSender, url: 'file:///tmp/a' }).allowed, false);
});

test('settings and unknown messages fail closed for untrusted senders', () => {
  assert.equal(validateSender('get-settings', contentSender).allowed, true);
  assert.equal(validateSender('get-settings', {}).allowed, false);
  assert.equal(validateSender('unknown', internalSender).allowed, false);
});
