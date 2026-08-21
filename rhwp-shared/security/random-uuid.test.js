import assert from 'node:assert/strict';
import test from 'node:test';

import { secureRandomUuid } from './random-uuid.js';

test('uses randomUUID when the runtime provides it', () => {
  const expected = '11111111-2222-4333-8444-555555555555';
  assert.equal(secureRandomUuid({ randomUUID: () => expected }), expected);
});

test('Safari 15 fallback generates an RFC 4122 version-4 UUID', () => {
  const cryptoWithoutRandomUuid = {
    getRandomValues(bytes) {
      bytes.fill(0xff);
      return bytes;
    },
  };
  const value = secureRandomUuid(cryptoWithoutRandomUuid);
  assert.equal(value, 'ffffffff-ffff-4fff-bfff-ffffffffffff');
  assert.match(value, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
});

test('fails closed without a cryptographic random source', () => {
  assert.throws(() => secureRandomUuid({}), /안전한 난수/);
});
