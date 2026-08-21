import { runTest, assert } from './helpers.mjs';

runTest('postMessage RPC origin and source boundary', async ({ page }) => {
  const studioUrl = process.env.VITE_URL || 'http://127.0.0.1:7700';
  await page.goto(studioUrl, { waitUntil: 'domcontentloaded', timeout: 120000 });
  await page.waitForFunction(() => !!window.__wasm, { timeout: 120000 });

  const rejected = await page.evaluate(async () => {
    const ids = ['forged-origin', 'missing-source'];
    const responses = [];
    const listener = (event) => {
      if (event.data?.type === 'rhwp-response' && ids.includes(event.data.id)) {
        responses.push(event.data.id);
      }
    };
    window.addEventListener('message', listener);

    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'rhwp-request', id: ids[0], method: 'exportHwp' },
      origin: 'https://attacker.invalid',
      source: window,
    }));
    window.dispatchEvent(new MessageEvent('message', {
      data: { type: 'rhwp-request', id: ids[1], method: 'exportHwp' },
      origin: window.location.origin,
      source: null,
    }));

    await new Promise(resolve => setTimeout(resolve, 200));
    window.removeEventListener('message', listener);
    return responses;
  });
  assert(rejected.length === 0, `forged RPC requests rejected (${rejected})`);

  const valid = await page.evaluate(() => new Promise((resolve) => {
    const id = 'same-origin-ready';
    const timeout = window.setTimeout(() => resolve({ timeout: true }), 60000);
    const listener = (event) => {
      if (event.origin !== window.location.origin) return;
      if (event.source !== window) return;
      if (event.data?.type !== 'rhwp-response' || event.data.id !== id) return;
      window.clearTimeout(timeout);
      window.removeEventListener('message', listener);
      resolve(event.data);
    };
    window.addEventListener('message', listener);
    window.postMessage(
      { type: 'rhwp-request', id, method: 'ready', params: {} },
      window.location.origin,
    );
  }));
  assert(valid.timeout !== true, 'same-origin RPC returned before timeout');
  assert(valid.result === true, `same-origin ready result is true (${valid.result})`);

  const transfer = await page.evaluate(async () => {
    const store = await import('/src/document-transfer-store.ts');
    const source = new Uint8Array([99, 1, 2, 3, 88]).subarray(1, 4);
    const transferId = await store.storeDocumentTransfer(source, 'application/x-hwp');
    const first = await store.takeDocumentTransfer(transferId);
    const second = await store.takeDocumentTransfer(transferId);
    const pending = [];
    for (const value of [4, 5, 6]) {
      pending.push(await store.storeDocumentTransfer(new Uint8Array([value]), null));
    }
    const cappedOldest = await store.takeDocumentTransfer(pending[0]);
    const cappedMiddle = await store.takeDocumentTransfer(pending[1]);
    const cappedNewest = await store.takeDocumentTransfer(pending[2]);
    return {
      bytes: first ? Array.from(new Uint8Array(first.data)) : null,
      contentType: first?.contentType,
      second,
      invalid: await store.takeDocumentTransfer('../forged'),
      capped: [cappedOldest, cappedMiddle, cappedNewest].map(item => (
        item ? Array.from(new Uint8Array(item.data))[0] : null
      )),
    };
  });
  assert(JSON.stringify(transfer.bytes) === '[1,2,3]', 'binary transfer preserves exact view bytes');
  assert(transfer.contentType === 'application/x-hwp', 'binary transfer preserves content type');
  assert(transfer.second === null, 'binary transfer is one-time');
  assert(transfer.invalid === null, 'invalid binary transfer IDs fail closed');
  assert(JSON.stringify(transfer.capped) === '[null,5,6]', 'binary transfer backlog is bounded');
}, { skipLoadApp: true });
