import { resolveDocumentUrl } from './document-url-resolver.js';
import { createFetchGrant } from './fetch-grants.js';

async function buildViewerUrl(viewerBase, options = {}) {
  const params = new URLSearchParams();
  if (options.url) {
    const url = resolveDocumentUrl(options.url);
    params.set('url', url);
    params.set('grant', await createFetchGrant(url));
  }
  if (options.filename) params.set('filename', options.filename);
  const query = params.toString();
  return query ? `${viewerBase}?${query}` : viewerBase;
}

export async function openViewer(options = {}) {
  const viewerBase = browser.runtime.getURL('viewer.html');
  const fullUrl = await buildViewerUrl(viewerBase, options);
  await browser.tabs.create({ url: fullUrl });
}

export async function openViewerOrReuse(options = {}) {
  const viewerBase = browser.runtime.getURL('viewer.html');
  const tabs = await browser.tabs.query({ url: `${viewerBase}*` });
  const emptyTab = tabs.find(tab => tab.url === viewerBase);
  if (emptyTab) {
    await browser.tabs.update(emptyTab.id, {
      url: await buildViewerUrl(viewerBase, options),
      active: true,
    });
  } else {
    await openViewer(options);
  }
}

export { buildViewerUrl };
