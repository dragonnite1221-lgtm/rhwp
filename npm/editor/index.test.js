import assert from 'node:assert/strict';
import test from 'node:test';

import { createEditor } from './index.js';

test('createEditor binds arbitrary parent origins with an unguessable iframe capability', async () => {
  const listeners = [];
  const sent = [];
  const iframe = {
    style: {},
    allow: '',
    src: '',
    addEventListener(type, listener) {
      if (type === 'load') queueMicrotask(listener);
    },
    remove() {},
  };
  iframe.contentWindow = {
    postMessage(message, targetOrigin) {
      sent.push({ message, targetOrigin });
      queueMicrotask(() => {
        for (const listener of listeners) {
          listener({
            source: iframe.contentWindow,
            origin: targetOrigin,
            data: {
              type: 'rhwp-response',
              id: message.id,
              result: true,
              rpcToken: message.rpcToken,
            },
          });
        }
      });
    },
  };

  globalThis.window = {
    location: { href: 'https://consumer.example/page' },
    addEventListener(type, listener) {
      if (type === 'message') listeners.push(listener);
    },
    removeEventListener(type, listener) {
      if (type !== 'message') return;
      const index = listeners.indexOf(listener);
      if (index >= 0) listeners.splice(index, 1);
    },
  };
  globalThis.document = {
    createElement(tag) {
      assert.equal(tag, 'iframe');
      return iframe;
    },
  };
  const container = {
    appendChild(node) {
      assert.equal(node, iframe);
    },
  };

  const editor = await createEditor(container, {
    studioUrl: 'https://studio.example/rhwp/',
  });
  const embedded = new URL(editor.element.src);
  const token = new URLSearchParams(embedded.hash.slice(1)).get('rhwp-rpc-token');
  assert.match(token, /^[0-9a-f]{64}$/);
  assert.equal(sent[0].targetOrigin, 'https://studio.example');
  assert.equal(sent[0].message.rpcToken, token);
  editor.destroy();
});
