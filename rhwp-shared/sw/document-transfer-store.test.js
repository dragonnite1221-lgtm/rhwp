import assert from 'node:assert/strict';
import test from 'node:test';

import { planTransferAdmission } from './document-transfer-store.js';

test('admission retains every active transfer regardless of record count', () => {
  const now = 1000;
  const records = [1, 2, 3].map((value) => ({
    id: `active-${value}`,
    data: new Uint8Array([value]).buffer,
    expiresAt: now + 1000,
  }));
  const result = planTransferAdmission(records, 1, now);
  assert.equal(result.allowed, true);
  assert.equal(result.activeBytes, 3);
  assert.deepEqual(result.expiredIds, []);
});

test('admission prunes only expired records and rejects byte exhaustion', () => {
  const now = 1000;
  const records = [{
    id: 'expired',
    data: new Uint8Array([1]).buffer,
    expiresAt: now,
  }, {
    id: 'active',
    data: new ArrayBuffer(4),
    expiresAt: now + 1000,
  }];
  const result = planTransferAdmission(records, 1, now, 4);
  assert.equal(result.allowed, false);
  assert.equal(result.activeBytes, 4);
  assert.deepEqual(result.expiredIds, ['expired']);
});
