const END_OF_CHAIN = 0xFFFFFFFE;
const FREE_SECTOR = 0xFFFFFFFF;
const MAX_REGULAR_SECTOR = 0xFFFFFFF9;
const MAX_CONTAINER_BYTES = 32 * 1024 * 1024;
const MAX_IMAGE_BYTES = 10 * 1024 * 1024;
const MAX_IMAGE_DIMENSION = 8192;
const MAX_IMAGE_PIXELS = 40 * 1024 * 1024;

function inRange(data, offset, length) {
  return Number.isSafeInteger(offset)
    && Number.isSafeInteger(length)
    && offset >= 0
    && length >= 0
    && offset <= data.length - length;
}

function u16(data, offset) {
  return inRange(data, offset, 2) ? data[offset] | (data[offset + 1] << 8) : null;
}

function u32(data, offset) {
  if (!inRange(data, offset, 4)) return null;
  return (data[offset]
    | (data[offset + 1] << 8)
    | (data[offset + 2] << 16)
    | (data[offset + 3] << 24)) >>> 0;
}

function boundedU64(data, offset, max) {
  const low = u32(data, offset);
  const high = u32(data, offset + 4);
  if (low === null || high === null || high !== 0 || low > max) return null;
  return low;
}

function sectorOffset(data, sector, sectorSize) {
  const totalSectors = Math.floor(data.length / sectorSize) - 1;
  if (!Number.isInteger(sector) || sector < 0 || sector >= totalSectors) return null;
  const offset = (sector + 1) * sectorSize;
  return inRange(data, offset, sectorSize) ? offset : null;
}

function walkChain(start, table, maxSteps, isValidSector) {
  const chain = [];
  const visited = new Set();
  let sector = start;
  while (sector !== END_OF_CHAIN) {
    if (sector > MAX_REGULAR_SECTOR || !isValidSector(sector)) return null;
    if (visited.has(sector) || chain.length >= maxSteps) return null;
    visited.add(sector);
    chain.push(sector);
    if (sector >= table.length) return null;
    sector = table[sector];
  }
  return chain;
}

function readChain(data, chain, size, sectorSize, offsetForSector) {
  if (!Number.isSafeInteger(size) || size < 0 || size > MAX_CONTAINER_BYTES) return null;
  if (chain.length < Math.ceil(size / sectorSize)) return null;
  const result = new Uint8Array(size);
  let written = 0;
  for (const sector of chain) {
    if (written >= size) break;
    const offset = offsetForSector(sector);
    const count = Math.min(sectorSize, size - written);
    if (offset === null || !inRange(data, offset, count)) return null;
    result.set(data.subarray(offset, offset + count), written);
    written += count;
  }
  return written === size ? result : null;
}

function buildFat(data, sectorSize) {
  const totalSectors = Math.floor(data.length / sectorSize) - 1;
  const fatCount = u32(data, 44);
  const difatStart = u32(data, 68);
  const difatCount = u32(data, 72);
  if (fatCount === null || difatStart === null || difatCount === null) return null;
  if (fatCount > totalSectors || difatCount > totalSectors) return null;

  const fatSectors = [];
  for (let i = 0; i < 109 && fatSectors.length < fatCount; i++) {
    const sector = u32(data, 76 + i * 4);
    if (sector === null) return null;
    if (sector !== FREE_SECTOR) fatSectors.push(sector);
  }

  const difatVisited = new Set();
  let difatSector = difatStart;
  for (let i = 0; i < difatCount; i++) {
    if (difatSector > MAX_REGULAR_SECTOR || difatVisited.has(difatSector)) return null;
    difatVisited.add(difatSector);
    const offset = sectorOffset(data, difatSector, sectorSize);
    if (offset === null) return null;
    const entries = sectorSize / 4 - 1;
    for (let j = 0; j < entries && fatSectors.length < fatCount; j++) {
      const sector = u32(data, offset + j * 4);
      if (sector === null) return null;
      if (sector !== FREE_SECTOR) fatSectors.push(sector);
    }
    difatSector = u32(data, offset + entries * 4);
    if (difatSector === null) return null;
  }
  if ((difatCount === 0 && difatStart !== END_OF_CHAIN)
      || (difatCount > 0 && difatSector !== END_OF_CHAIN)) return null;
  if (fatSectors.length !== fatCount || new Set(fatSectors).size !== fatSectors.length) {
    return null;
  }

  const fat = [];
  for (const sector of fatSectors) {
    const offset = sectorOffset(data, sector, sectorSize);
    if (offset === null) return null;
    for (let i = 0; i < sectorSize / 4; i++) fat.push(u32(data, offset + i * 4));
  }
  return fat.some(entry => entry === null) ? null : fat;
}

function readDirectoryName(data, offset) {
  const byteLength = u16(data, offset + 64);
  if (byteLength === null || byteLength < 2 || byteLength > 64 || byteLength % 2 !== 0) {
    return null;
  }
  let value = '';
  for (let i = 0; i < byteLength - 2; i += 2) {
    const code = u16(data, offset + i);
    if (code === null) return null;
    if (code === 0) break;
    value += String.fromCharCode(code);
  }
  return value;
}

function validImageDimensions(width, height) {
  return Number.isInteger(width)
    && Number.isInteger(height)
    && width > 0
    && height > 0
    && width <= MAX_IMAGE_DIMENSION
    && height <= MAX_IMAGE_DIMENSION
    && width * height <= MAX_IMAGE_PIXELS;
}

function parseImageData(data) {
  if (!(data instanceof Uint8Array) || data.length === 0 || data.length > MAX_IMAGE_BYTES) {
    return null;
  }
  let mime;
  let width = 0;
  let height = 0;
  const pngMagic = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
  if (data.length >= 24 && pngMagic.every((byte, index) => data[index] === byte)) {
    mime = 'image/png';
    width = u32be(data, 16);
    height = u32be(data, 20);
  } else if (data.length >= 26 && data[0] === 0x42 && data[1] === 0x4D) {
    mime = 'image/bmp';
    width = u32(data, 18);
    const rawHeight = i32(data, 22);
    height = rawHeight === null ? null : Math.abs(rawHeight);
  } else if (data.length >= 10 && data[0] === 0x47 && data[1] === 0x49
      && data[2] === 0x46) {
    mime = 'image/gif';
    width = u16(data, 6);
    height = u16(data, 8);
  } else {
    return null;
  }
  if (!validImageDimensions(width, height)) return null;

  const parts = [];
  for (let offset = 0; offset < data.length; offset += 0x8000) {
    parts.push(String.fromCharCode(...data.subarray(offset, offset + 0x8000)));
  }
  return {
    dataUri: `data:${mime};base64,${btoa(parts.join(''))}`,
    width,
    height,
    mime,
  };
}

export function extractPrvImageFromCfb(data) {
  try {
    if (!(data instanceof Uint8Array) || data.length < 512
        || data.length > MAX_CONTAINER_BYTES) return null;
    const magic = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if (!magic.every((byte, index) => data[index] === byte)) return null;
    if (u16(data, 28) !== 0xFFFE) return null;
    const majorVersion = u16(data, 26);
    const sectorShift = u16(data, 30);
    const miniShift = u16(data, 32);
    if ((majorVersion !== 3 && majorVersion !== 4)
        || (majorVersion === 3 && sectorShift !== 9)
        || (majorVersion === 4 && sectorShift !== 12)
        || miniShift !== 6) return null;
    const sectorSize = 2 ** sectorShift;
    const miniSectorSize = 2 ** miniShift;
    if (data.length < sectorSize || u32(data, 56) !== 4096) return null;

    const totalSectors = Math.floor(data.length / sectorSize) - 1;
    const offsetForSector = sector => sectorOffset(data, sector, sectorSize);
    const isRegular = sector => Number.isInteger(sector) && sector >= 0 && sector < totalSectors;
    const fat = buildFat(data, sectorSize);
    const directoryStart = u32(data, 48);
    if (!fat || directoryStart === null) return null;
    const directoryChain = walkChain(directoryStart, fat, totalSectors, isRegular);
    if (!directoryChain || directoryChain.length === 0) return null;

    const firstDirectoryOffset = offsetForSector(directoryChain[0]);
    if (firstDirectoryOffset === null || data[firstDirectoryOffset + 66] !== 5) return null;
    const miniStreamStart = u32(data, firstDirectoryOffset + 116);
    const miniStreamSize = boundedU64(data, firstDirectoryOffset + 120, MAX_CONTAINER_BYTES);
    if (miniStreamStart === null || miniStreamSize === null) return null;

    let target = null;
    for (const directorySector of directoryChain) {
      const base = offsetForSector(directorySector);
      if (base === null) return null;
      for (let offset = base; offset < base + sectorSize; offset += 128) {
        if (!inRange(data, offset, 128)) return null;
        if (data[offset + 66] !== 2 || readDirectoryName(data, offset) !== 'PrvImage') continue;
        const size = boundedU64(data, offset + 120, MAX_IMAGE_BYTES);
        const start = u32(data, offset + 116);
        if (size === null || size === 0 || start === null) return null;
        target = { size, start };
        break;
      }
      if (target) break;
    }
    if (!target) return null;

    if (target.size >= 4096) {
      const chain = walkChain(target.start, fat, totalSectors, isRegular);
      if (!chain) return null;
      return parseImageData(readChain(data, chain, target.size, sectorSize, offsetForSector));
    }

    const miniFatStart = u32(data, 60);
    const miniFatCount = u32(data, 64);
    if (miniFatStart === null || miniFatCount === null || miniFatCount > totalSectors) return null;
    const miniFatChain = walkChain(miniFatStart, fat, totalSectors, isRegular);
    if (!miniFatChain || miniFatChain.length !== miniFatCount) return null;
    const miniFatBytes = readChain(
      data, miniFatChain, miniFatCount * sectorSize, sectorSize, offsetForSector,
    );
    if (!miniFatBytes) return null;
    const miniFat = [];
    for (let offset = 0; offset < miniFatBytes.length; offset += 4) {
      miniFat.push(u32(miniFatBytes, offset));
    }
    if (miniFat.some(entry => entry === null)) return null;

    const miniStreamChain = walkChain(miniStreamStart, fat, totalSectors, isRegular);
    if (!miniStreamChain) return null;
    const miniStream = readChain(
      data, miniStreamChain, miniStreamSize, sectorSize, offsetForSector,
    );
    if (!miniStream) return null;
    const miniCount = Math.floor(miniStream.length / miniSectorSize);
    const miniChain = walkChain(
      target.start, miniFat, miniFat.length, sector => sector >= 0 && sector < miniCount,
    );
    if (!miniChain) return null;
    return parseImageData(readChain(
      miniStream,
      miniChain,
      target.size,
      miniSectorSize,
      sector => {
        const offset = sector * miniSectorSize;
        return inRange(miniStream, offset, miniSectorSize) ? offset : null;
      },
    ));
  } catch {
    return null;
  }
}

function findEocd(data) {
  const lower = Math.max(0, data.length - 65557);
  for (let offset = data.length - 22; offset >= lower; offset--) {
    if (u32(data, offset) === 0x06054B50) return offset;
  }
  return null;
}

async function inflateBounded(compressed, expectedSize) {
  const stream = new DecompressionStream('deflate-raw');
  const writer = stream.writable.getWriter();
  const reader = stream.readable.getReader();
  const writePromise = writer.write(compressed).then(() => writer.close());
  const chunks = [];
  let total = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      total += value.byteLength;
      if (total > expectedSize || total > MAX_IMAGE_BYTES) {
        await Promise.allSettled([writePromise, reader.cancel(), writer.abort()]);
        return null;
      }
      chunks.push(value);
    }
    await writePromise;
  } catch {
    await Promise.allSettled([writePromise, reader.cancel(), writer.abort()]);
    return null;
  }
  if (total !== expectedSize) return null;
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return result;
}

export async function extractPrvImageFromZip(data) {
  try {
    if (!(data instanceof Uint8Array) || data.length < 22
        || data.length > MAX_CONTAINER_BYTES) return null;
    const eocd = findEocd(data);
    if (eocd === null || u16(data, eocd + 4) !== 0 || u16(data, eocd + 6) !== 0) {
      return null;
    }
    const diskEntries = u16(data, eocd + 8);
    const totalEntries = u16(data, eocd + 10);
    const directorySize = u32(data, eocd + 12);
    const directoryOffset = u32(data, eocd + 16);
    const commentLength = u16(data, eocd + 20);
    if ([diskEntries, totalEntries, directorySize, directoryOffset, commentLength]
      .some(value => value === null)) return null;
    if (diskEntries !== totalEntries || totalEntries > 4096) return null;
    if (!inRange(data, directoryOffset, directorySize)
        || directoryOffset + directorySize > eocd
        || !inRange(data, eocd + 22, commentLength)
        || eocd + 22 + commentLength !== data.length) return null;

    const directoryEnd = directoryOffset + directorySize;
    let offset = directoryOffset;
    for (let index = 0; index < totalEntries; index++) {
      if (!inRange(data, offset, 46) || offset + 46 > directoryEnd
          || u32(data, offset) !== 0x02014B50) return null;
      const flags = u16(data, offset + 8);
      const method = u16(data, offset + 10);
      const compressedSize = u32(data, offset + 20);
      const uncompressedSize = u32(data, offset + 24);
      const nameLength = u16(data, offset + 28);
      const extraLength = u16(data, offset + 30);
      const entryCommentLength = u16(data, offset + 32);
      const localOffset = u32(data, offset + 42);
      if ([flags, method, compressedSize, uncompressedSize, nameLength,
        extraLength, entryCommentLength, localOffset].some(value => value === null)) return null;
      const entryLength = 46 + nameLength + extraLength + entryCommentLength;
      if (!inRange(data, offset, entryLength) || offset + entryLength > directoryEnd) return null;
      const name = new TextDecoder().decode(data.subarray(offset + 46, offset + 46 + nameLength));

      if (/^Preview\/PrvImage(?:\.[A-Za-z0-9]{1,16})?$/.test(name)) {
        if ((flags & 1) !== 0 || (method !== 0 && method !== 8)) return null;
        if (compressedSize > MAX_IMAGE_BYTES || uncompressedSize > MAX_IMAGE_BYTES
            || uncompressedSize === 0 || !inRange(data, localOffset, 30)
            || u32(data, localOffset) !== 0x04034B50) return null;
        const localFlags = u16(data, localOffset + 6);
        const localMethod = u16(data, localOffset + 8);
        const localNameLength = u16(data, localOffset + 26);
        const localExtraLength = u16(data, localOffset + 28);
        if ([localFlags, localMethod, localNameLength, localExtraLength]
          .some(value => value === null)) return null;
        if (localFlags !== flags || localMethod !== method
            || localNameLength !== nameLength) return null;
        const localNameStart = localOffset + 30;
        if (!inRange(data, localNameStart, localNameLength)
            || !data.subarray(localNameStart, localNameStart + localNameLength)
              .every((byte, index) => byte === data[offset + 46 + index])) return null;
        const start = localOffset + 30 + localNameLength + localExtraLength;
        if (!inRange(data, start, compressedSize) || start + compressedSize > directoryOffset) {
          return null;
        }
        const compressed = data.subarray(start, start + compressedSize);
        if (method === 0) {
          if (compressedSize !== uncompressedSize) return null;
          return parseImageData(compressed);
        }
        const inflated = await inflateBounded(compressed, uncompressedSize);
        return inflated ? parseImageData(inflated) : null;
      }
      offset += entryLength;
    }
    if (offset !== directoryEnd) return null;
    return null;
  } catch {
    return null;
  }
}

export async function extractThumbnail(data) {
  if (!(data instanceof Uint8Array)) return null;
  const isZip = data.length >= 4 && u32(data, 0) === 0x04034B50;
  return isZip ? extractPrvImageFromZip(data) : extractPrvImageFromCfb(data);
}

function u32be(data, offset) {
  if (!inRange(data, offset, 4)) return null;
  return ((data[offset] << 24) | (data[offset + 1] << 16)
    | (data[offset + 2] << 8) | data[offset + 3]) >>> 0;
}

function i32(data, offset) {
  const value = u32(data, offset);
  return value === null ? null : value | 0;
}

export { MAX_CONTAINER_BYTES, MAX_IMAGE_BYTES };
