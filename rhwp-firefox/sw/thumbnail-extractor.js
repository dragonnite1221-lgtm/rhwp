// Fetch an HWP/HWPX document and delegate container parsing to the shared
// fail-closed parser.

import { resolveDocumentUrl } from './document-url-resolver.js';
import { extractThumbnail } from './thumbnail-parser.js';
import { fetchPublicResource } from './secure-fetch.js';

const THUMBNAIL_CACHE = new Map();
const CACHE_MAX_SIZE = 100;

export async function extractThumbnailFromUrl(url) {
  if (THUMBNAIL_CACHE.has(url)) return THUMBNAIL_CACHE.get(url);

  try {
    const { data } = await fetchPublicResource(resolveDocumentUrl(url), {
      maxBytes: 32 * 1024 * 1024,
      timeoutMs: 30_000,
    });
    const result = await extractThumbnail(data);
    if (result) {
      if (THUMBNAIL_CACHE.size >= CACHE_MAX_SIZE) {
        THUMBNAIL_CACHE.delete(THUMBNAIL_CACHE.keys().next().value);
      }
      THUMBNAIL_CACHE.set(url, result);
    }
    return result;
  } catch {
    return null;
  }
}
