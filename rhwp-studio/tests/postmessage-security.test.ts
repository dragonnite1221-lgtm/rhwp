import assert from 'node:assert/strict';
import test from 'node:test';

import {
  postRpcResponse,
  readRpcToken,
  trustedRpcChannel,
} from '../src/postmessage-security.ts';

const TOKEN = 'a'.repeat(64);

function windows() {
  const messages: Array<{ payload: unknown; origin: string }> = [];
  const parent = {
    postMessage(payload: unknown, origin: string) {
      messages.push({ payload, origin });
    },
  } as unknown as Window;
  const self = {
    location: { origin: 'https://studio.example' },
    parent,
  } as unknown as Window;
  return { messages, parent, self };
}

test('runtime capability authorizes only the direct parent with the exact token', () => {
  const { parent, self } = windows();
  const allowed = new Set(['https://studio.example']);
  const event = {
    source: parent,
    origin: 'https://consumer.example',
    data: { type: 'rhwp-request', rpcToken: TOKEN },
  } as unknown as MessageEvent;

  assert.ok(trustedRpcChannel(event, self, allowed, TOKEN));
  assert.equal(trustedRpcChannel({
    ...event,
    data: { ...event.data, rpcToken: 'b'.repeat(64) },
  } as MessageEvent, self, allowed, TOKEN), null);
  assert.equal(trustedRpcChannel({
    ...event,
    source: {} as Window,
  } as MessageEvent, self, allowed, TOKEN), null);
});

test('responses preserve the authenticated origin and capability', () => {
  const { messages, parent } = windows();
  postRpcResponse({
    source: parent,
    origin: 'https://consumer.example',
    rpcToken: TOKEN,
  }, { type: 'rhwp-response', id: 1, result: true });

  assert.deepEqual(messages, [{
    origin: 'https://consumer.example',
    payload: { type: 'rhwp-response', id: 1, result: true, rpcToken: TOKEN },
  }]);
});

test('fragment token parsing is strict', () => {
  assert.equal(readRpcToken(`#rhwp-rpc-token=${TOKEN}`), TOKEN);
  assert.equal(readRpcToken('#rhwp-rpc-token=short'), null);
  assert.equal(readRpcToken(''), null);
});
