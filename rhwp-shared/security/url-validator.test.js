import test from 'node:test';
import assert from 'node:assert/strict';

import {
  isNonPublicIpAddress,
  validatePublicUrl,
  validateResolvedAddresses,
} from './url-validator.js';

test('blocks literal private, reserved, and local destinations', () => {
  const blocked = [
    'http://127.0.0.1/document.hwp',
    'http://2130706433/document.hwp',
    'http://10.0.0.1/document.hwp',
    'http://169.254.169.254/latest/meta-data',
    'http://[::1]/document.hwp',
    'http://[fd00::1]/document.hwp',
    'http://printer.local/document.hwp',
    'http://intranet/document.hwp',
    'file:///etc/passwd',
    'https://user@example.com/document.hwp',
  ];
  for (const url of blocked) {
    assert.equal(validatePublicUrl(url).allowed, false, url);
  }
});

test('accepts syntactically public HTTP(S) URLs', () => {
  assert.equal(validatePublicUrl('https://example.com/document.hwp').allowed, true);
  assert.equal(validatePublicUrl('http://203.0.114.10/document.hwp').allowed, true);
});

test('DNS answers fail closed if any address is non-public', () => {
  assert.equal(validateResolvedAddresses([]).allowed, false);
  assert.equal(validateResolvedAddresses(['not-an-address']).allowed, false);
  assert.equal(validateResolvedAddresses(['2001::::1']).allowed, false);
  assert.equal(validateResolvedAddresses(['93.184.216.34', '127.0.0.1']).allowed, false);
  assert.equal(validateResolvedAddresses(['93.184.216.34', '2606:2800:220:1::']).allowed, true);
});

test('IP classifier covers carrier, documentation, and mapped ranges', () => {
  for (const address of [
    '100.64.0.1',
    '192.0.2.1',
    '198.18.0.1',
    '198.51.100.1',
    '203.0.113.1',
    '224.0.0.1',
    '::ffff:127.0.0.1',
    '2001:db8::1',
  ]) {
    assert.equal(isNonPublicIpAddress(address), true, address);
  }
});
