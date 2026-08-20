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
}, { skipLoadApp: true });
