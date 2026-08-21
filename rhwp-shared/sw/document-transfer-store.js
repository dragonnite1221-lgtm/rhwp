import { secureRandomUuid } from '../security/random-uuid.js';

const DB_NAME = 'rhwp-document-transfers';
const DB_VERSION = 2;
const STORE_NAME = 'transfers';
const TRANSFER_TTL_MS = 2 * 60 * 1000;
const MAX_PENDING_TRANSFER_BYTES = 128 * 1024 * 1024;

function requestResult(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error('IndexedDB 요청 실패'));
  });
}

function transactionDone(transaction) {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onabort = () => reject(transaction.error || new Error('IndexedDB 트랜잭션 중단'));
    transaction.onerror = () => reject(transaction.error || new Error('IndexedDB 트랜잭션 실패'));
  });
}

async function openTransferDatabase(indexedDb = globalThis.indexedDB) {
  if (!indexedDb) throw new Error('문서 전송 저장소를 사용할 수 없음');
  const request = indexedDb.open(DB_NAME, DB_VERSION);
  request.onupgradeneeded = event => {
    const database = request.result;
    const store = database.objectStoreNames.contains(STORE_NAME)
      ? request.transaction.objectStore(STORE_NAME)
      : database.createObjectStore(STORE_NAME, { keyPath: 'id' });
    if (!store.indexNames.contains('createdAt')) {
      store.createIndex('createdAt', 'createdAt');
    }
    if (event.oldVersion > 0 && event.oldVersion < 2) {
      // Transfers are ephemeral; clear version-1 records that lack createdAt.
      store.clear();
    }
  };
  return requestResult(request);
}

function exactArrayBuffer(data) {
  if (data instanceof ArrayBuffer) return data.slice(0);
  if (ArrayBuffer.isView(data)) {
    return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
  }
  throw new Error('문서 전송 데이터는 ArrayBuffer 또는 typed array여야 함');
}

function validTransferId(id) {
  return typeof id === 'string'
    && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(id);
}

function planTransferAdmission(
  records,
  newByteLength,
  now = Date.now(),
  maxBytes = MAX_PENDING_TRANSFER_BYTES,
) {
  if (!Number.isSafeInteger(newByteLength) || newByteLength < 0) {
    throw new Error('문서 전송 크기가 올바르지 않음');
  }
  const expiredIds = [];
  let activeBytes = 0;
  for (const record of records) {
    if (!record || record.expiresAt <= now || !(record.data instanceof ArrayBuffer)) {
      if (record?.id) expiredIds.push(record.id);
      continue;
    }
    activeBytes += record.data.byteLength;
  }
  return {
    expiredIds,
    activeBytes,
    allowed: activeBytes + newByteLength <= maxBytes,
  };
}

export async function storeDocumentTransfer(data, contentType = null) {
  const id = secureRandomUuid();
  const transferData = exactArrayBuffer(data);
  const database = await openTransferDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    const now = Date.now();
    const existing = await requestResult(store.index('createdAt').getAll());
    const admission = planTransferAdmission(existing, transferData.byteLength, now);
    for (const expiredId of admission.expiredIds) store.delete(expiredId);
    if (!admission.allowed) {
      await transactionDone(transaction);
      throw new Error('대기 중인 문서 전송 용량 초과');
    }
    store.put({
      id,
      data: transferData,
      contentType: typeof contentType === 'string' ? contentType : null,
      createdAt: now,
      expiresAt: now + TRANSFER_TTL_MS,
    });
    await transactionDone(transaction);
    return id;
  } finally {
    database.close();
  }
}

export async function takeDocumentTransfer(id) {
  if (!validTransferId(id)) return null;
  const database = await openTransferDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, 'readwrite');
    const store = transaction.objectStore(STORE_NAME);
    const record = await requestResult(store.get(id));
    if (record) store.delete(id);
    await transactionDone(transaction);
    if (!record || record.expiresAt < Date.now() || !(record.data instanceof ArrayBuffer)) {
      return null;
    }
    return { data: record.data, contentType: record.contentType };
  } finally {
    database.close();
  }
}

export { MAX_PENDING_TRANSFER_BYTES, TRANSFER_TTL_MS, planTransferAdmission };
