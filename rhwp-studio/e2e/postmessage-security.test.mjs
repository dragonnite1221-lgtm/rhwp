import { runTest, assert } from './helpers.mjs';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';

async function startCrossOriginEditorHost(studioUrl) {
  const editorSource = await readFile(new URL('../../npm/editor/index.js', import.meta.url), 'utf8');
  const server = createServer((request, response) => {
    if (request.url === '/editor.js') {
      response.writeHead(200, { 'content-type': 'text/javascript; charset=utf-8' });
      response.end(editorSource);
      return;
    }
    response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
    response.end(`<!doctype html>
      <div id="editor" style="width:800px;height:600px"></div>
      <script type="module">
        import { createEditor } from '/editor.js';
        try {
          window.__editor = await createEditor('#editor', { studioUrl: ${JSON.stringify(studioUrl)} });
          window.__rpcReady = true;
        } catch (error) {
          window.__rpcError = error.message || String(error);
        }
      </script>`);
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: () => new Promise(resolve => server.close(resolve)),
  };
}

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
    const pendingTransfers = await Promise.all(pending.map(id => store.takeDocumentTransfer(id)));
    return {
      bytes: first ? Array.from(new Uint8Array(first.data)) : null,
      contentType: first?.contentType,
      second,
      invalid: await store.takeDocumentTransfer('../forged'),
      pending: pendingTransfers.map(item => (
        item ? Array.from(new Uint8Array(item.data))[0] : null
      )),
    };
  });
  assert(JSON.stringify(transfer.bytes) === '[1,2,3]', 'binary transfer preserves exact view bytes');
  assert(transfer.contentType === 'application/x-hwp', 'binary transfer preserves content type');
  assert(transfer.second === null, 'binary transfer is one-time');
  assert(transfer.invalid === null, 'invalid binary transfer IDs fail closed');
  assert(JSON.stringify(transfer.pending) === '[4,5,6]', 'active transfers are not evicted before consumption');

  const parentHost = await startCrossOriginEditorHost(studioUrl);
  try {
    await page.goto(parentHost.url, { waitUntil: 'domcontentloaded', timeout: 30000 });
    await page.waitForFunction(
      () => window.__rpcReady === true || typeof window.__rpcError === 'string',
      { timeout: 60000 },
    );
    const embedded = await page.evaluate(() => ({
      ready: window.__rpcReady === true,
      error: window.__rpcError || null,
      iframeSrc: document.querySelector('iframe')?.src || '',
    }));
    assert(embedded.ready, `cross-origin @rhwp/editor ready handshake succeeds (${embedded.error})`);
    const token = new URLSearchParams(new URL(embedded.iframeSrc).hash.slice(1))
      .get('rhwp-rpc-token');
    assert(/^[0-9a-f]{64}$/.test(token || ''), 'cross-origin iframe carries a 256-bit capability');
  } finally {
    await parentHost.close();
  }
}, { skipLoadApp: true });
