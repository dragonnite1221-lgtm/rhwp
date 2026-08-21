import assert from 'node:assert/strict';
import { test } from 'node:test';
import { deflateRawSync } from 'node:zlib';

import {
  extractPrvImageFromCfb,
  extractPrvImageFromZip,
  extractThumbnail,
  MAX_IMAGE_BYTES,
} from './thumbnail-parser.js';

const encoder = new TextEncoder();

function set16(data, offset, value) {
  data[offset] = value & 0xFF;
  data[offset + 1] = (value >>> 8) & 0xFF;
}

function set32(data, offset, value) {
  data[offset] = value & 0xFF;
  data[offset + 1] = (value >>> 8) & 0xFF;
  data[offset + 2] = (value >>> 16) & 0xFF;
  data[offset + 3] = (value >>> 24) & 0xFF;
}

function concatenate(...parts) {
  const result = new Uint8Array(parts.reduce((sum, part) => sum + part.length, 0));
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}

function pngHeader(width = 2, height = 3) {
  const data = new Uint8Array(24);
  data.set([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
  data.set([0x49, 0x48, 0x44, 0x52], 12);
  data.set([(width >>> 24) & 0xFF, (width >>> 16) & 0xFF,
    (width >>> 8) & 0xFF, width & 0xFF], 16);
  data.set([(height >>> 24) & 0xFF, (height >>> 16) & 0xFF,
    (height >>> 8) & 0xFF, height & 0xFF], 20);
  return data;
}

function makeZip(payload, {
  name = 'Preview/PrvImage.png', method = 0, advertisedSize = payload.length,
} = {}) {
  const nameBytes = encoder.encode(name);
  const compressed = method === 8 ? new Uint8Array(deflateRawSync(payload)) : payload;
  const local = new Uint8Array(30 + nameBytes.length + compressed.length);
  set32(local, 0, 0x04034B50);
  set16(local, 4, 20);
  set16(local, 8, method);
  set32(local, 18, compressed.length);
  set32(local, 22, advertisedSize);
  set16(local, 26, nameBytes.length);
  local.set(nameBytes, 30);
  local.set(compressed, 30 + nameBytes.length);

  const central = new Uint8Array(46 + nameBytes.length);
  set32(central, 0, 0x02014B50);
  set16(central, 4, 20);
  set16(central, 6, 20);
  set16(central, 10, method);
  set32(central, 20, compressed.length);
  set32(central, 24, advertisedSize);
  set16(central, 28, nameBytes.length);
  set32(central, 42, 0);
  central.set(nameBytes, 46);

  const eocd = new Uint8Array(22);
  set32(eocd, 0, 0x06054B50);
  set16(eocd, 8, 1);
  set16(eocd, 10, 1);
  set32(eocd, 12, central.length);
  set32(eocd, 16, local.length);
  return concatenate(local, central, eocd);
}

function makeCfbDirectoryCycle() {
  const data = new Uint8Array(512 * 3);
  data.set([0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
  set16(data, 26, 3);
  set16(data, 28, 0xFFFE);
  set16(data, 30, 9);
  set16(data, 32, 6);
  set32(data, 44, 1);
  set32(data, 48, 0);
  set32(data, 56, 4096);
  set32(data, 60, 0xFFFFFFFE);
  set32(data, 64, 0);
  set32(data, 68, 0xFFFFFFFE);
  set32(data, 72, 0);
  for (let offset = 76; offset < 512; offset += 4) set32(data, offset, 0xFFFFFFFF);
  set32(data, 76, 1);
  data[512 + 66] = 5;
  set32(data, 1024, 0); // directory sector points to itself
  set32(data, 1028, 0xFFFFFFFD);
  for (let offset = 1032; offset < data.length; offset += 4) set32(data, offset, 0xFFFFFFFF);
  return data;
}

test('extracts bounded stored and deflated HWPX preview images', async () => {
  for (const method of [0, 8]) {
    const result = await extractPrvImageFromZip(makeZip(pngHeader(), { method }));
    assert.equal(result?.mime, 'image/png');
    assert.equal(result?.width, 2);
    assert.equal(result?.height, 3);
    assert.match(result?.dataUri ?? '', /^data:image\/png;base64,/);
  }
});

test('rejects advertised zip bombs before decompression', async () => {
  const zip = makeZip(pngHeader(), { method: 8, advertisedSize: MAX_IMAGE_BYTES + 1 });
  assert.equal(await extractPrvImageFromZip(zip), null);
});

test('rejects dimension bombs and non-exact preview paths', async () => {
  assert.equal(await extractPrvImageFromZip(makeZip(pngHeader(9000, 1))), null);
  assert.equal(await extractPrvImageFromZip(makeZip(pngHeader(), {
    name: 'Preview/PrvImage.png/child',
  })), null);
});

test('rejects local/central zip entry disagreement and trailing data', async () => {
  const mismatched = makeZip(pngHeader());
  set16(mismatched, 8, 8);
  assert.equal(await extractPrvImageFromZip(mismatched), null);
  assert.equal(await extractPrvImageFromZip(concatenate(makeZip(pngHeader()), new Uint8Array([1]))), null);
});

test('terminates on cyclic CFB FAT chains', () => {
  assert.equal(extractPrvImageFromCfb(makeCfbDirectoryCycle()), null);
});

test('rejects malformed CFB versions and sector shifts', () => {
  const data = makeCfbDirectoryCycle();
  set16(data, 30, 12);
  assert.equal(extractPrvImageFromCfb(data), null);
});

test('fails closed for a deterministic malformed-input corpus', async () => {
  let state = 0xC0FFEE;
  for (let sample = 0; sample < 500; sample++) {
    state = (state * 1664525 + 1013904223) >>> 0;
    const data = new Uint8Array(state % 2048);
    for (let index = 0; index < data.length; index++) {
      state = (state * 1664525 + 1013904223) >>> 0;
      data[index] = state & 0xFF;
    }
    const result = await extractThumbnail(data);
    assert.equal(result, null);
  }
});
