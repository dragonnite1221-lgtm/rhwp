// rhwp Safari Web Extension — module service worker.
// Network, capability, sender, and thumbnail parsing policies are shared with
// the Chrome and Firefox packages.

import { createFetchGrant, validateFetchGrant } from './sw/fetch-grants.js';
import { fetchPublicResource } from './sw/secure-fetch.js';
import { extractThumbnail } from './sw/thumbnail-parser.js';
import { validatePublicUrl } from './security/url-validator.js';
import { validateSender } from './security/sender-validator.js';
import { storeDocumentTransfer } from './sw/document-transfer-store.js';

const DEFAULT_ALLOWED_DOMAINS = ['.go.kr', '.or.kr', '.ac.kr', '.mil.kr', '.korea.kr', '.sc.kr'];
const MAX_DOCUMENT_BYTES = 64 * 1024 * 1024;
const MAX_THUMBNAIL_SOURCE_BYTES = 32 * 1024 * 1024;
const HWP_SIGNATURE = [0xD0, 0xCF, 0x11, 0xE0];
const HWPX_SIGNATURE = [0x50, 0x4B, 0x03, 0x04];
const MENU_ID = 'rhwp-open-link';
const THUMBNAIL_CACHE = new Map();
const CACHE_MAX_SIZE = 100;

function sanitizeFilename(filename) {
  if (!filename || typeof filename !== 'string') return '';
  let safe = filename.normalize?.('NFC') ?? filename;
  try {
    safe = decodeURIComponent(safe);
    try { safe = decodeURIComponent(safe); } catch { /* already decoded */ }
  } catch { /* retain undecodable input for sanitization */ }
  safe = safe.replace(/\0/g, '').replace(/\.\./g, '').replace(/[/\\]/g, '_');
  safe = safe.replace(/[^a-zA-Z0-9가-힣ㄱ-ㅎㅏ-ㅣ.\-_ ]/g, '');
  return safe.replace(/^[\s.]+|[\s.]+$/g, '').slice(0, 255) || 'document';
}

function isDocumentPath(pathname) {
  try {
    return /\.(hwp|hwpx)$/i.test(decodeURIComponent(pathname));
  } catch {
    return /\.(hwp|hwpx)$/i.test(pathname);
  }
}

function resolveDocumentUrl(url) {
  let parsed;
  try { parsed = new URL(url); } catch { return url; }
  if (parsed.protocol !== 'https:' || parsed.hostname !== 'github.com') return url;
  const segments = parsed.pathname.split('/').filter(Boolean);
  const [owner, repo, marker, ref, ...pathParts] = segments;
  if (!owner || !repo || marker !== 'blob' || !ref || !isDocumentPath(pathParts.join('/'))) {
    return url;
  }
  return `https://raw.githubusercontent.com/${owner}/${repo}/${ref}/${pathParts.join('/')}`;
}

function validateContentTarget(url, senderUrl) {
  const validation = validatePublicUrl(url);
  if (!validation.allowed) return validation;
  try {
    const resolved = resolveDocumentUrl(url);
    const senderOrigin = new URL(senderUrl).origin;
    const targetOrigin = new URL(resolved).origin;
    const githubAdapter = senderOrigin === 'https://github.com'
      && targetOrigin === 'https://raw.githubusercontent.com'
      && resolved !== url;
    if (senderOrigin !== targetOrigin && !githubAdapter) {
      return { allowed: false, reason: '교차 출처 문서 요청 차단' };
    }
    return { allowed: true, reason: '동일 출처 또는 승인된 provider adapter' };
  } catch {
    return { allowed: false, reason: '문서 URL 출처 확인 실패' };
  }
}

function validateMessage(message, sender) {
  if (!message || typeof message !== 'object' || Array.isArray(message)
      || typeof message.type !== 'string' || message.type.length > 64) {
    return { allowed: false, reason: '메시지 형식이 올바르지 않음' };
  }
  const senderResult = validateSender(message.type, sender);
  if (!senderResult.allowed) return senderResult;
  if (message.type !== 'get-settings'
      && (typeof message.url !== 'string' || message.url.length > 8192)) {
    return { allowed: false, reason: 'URL 필드가 올바르지 않음' };
  }
  return { allowed: true, reason: '메시지와 발신자 확인' };
}

function isAllowedDomain(hostname, domains) {
  return domains.some(domain => hostname.endsWith(domain));
}

function isDownloadEndpoint(parsed) {
  return /\.(do|action|jsp|aspx|php)$/i.test(parsed.pathname)
    || /download|filedown|attach/i.test(parsed.pathname);
}

async function logSecurity(type, url, reason) {
  try {
    const { securityLog } = await browser.storage.local.get({ securityLog: false });
    if (!securityLog) return;
    const { securityEvents } = await browser.storage.local.get({ securityEvents: [] });
    securityEvents.push({
      time: new Date().toISOString(),
      type,
      url: (url || '').slice(0, 500),
      reason,
    });
    await browser.storage.local.set({ securityEvents: securityEvents.slice(-250) });
  } catch { /* logging must not affect policy */ }
}

async function openViewer(options = {}) {
  const viewerBase = browser.runtime.getURL('viewer.html');
  const params = new URLSearchParams();
  if (options.url) {
    const validation = validatePublicUrl(options.url);
    if (!validation.allowed) {
      await logSecurity('url-blocked', options.url, validation.reason);
      return { ok: false, reason: validation.reason };
    }
    if (!options.explicit) {
      const [{ allowedDomains }, { allSitesEnabled }] = await Promise.all([
        browser.storage.local.get({ allowedDomains: DEFAULT_ALLOWED_DOMAINS }),
        browser.storage.local.get({ allSitesEnabled: false }),
      ]);
      if (!isDocumentPath(validation.parsed.pathname) && !allSitesEnabled
          && !isAllowedDomain(validation.parsed.hostname, allowedDomains)
          && !isDownloadEndpoint(validation.parsed)) {
        await logSecurity('url-blocked', options.url, 'domain-blocked');
        return { ok: false, reason: 'domain-blocked', hostname: validation.parsed.hostname };
      }
    }
    const resolved = resolveDocumentUrl(options.url);
    params.set('url', resolved);
    params.set('grant', await createFetchGrant(resolved));
  }
  if (options.filename) params.set('filename', sanitizeFilename(options.filename));
  const query = params.toString();
  await browser.tabs.create({ url: query ? `${viewerBase}?${query}` : viewerBase });
  return { ok: true };
}

function verifyHwpSignature(data) {
  if (!(data instanceof Uint8Array) || data.length < 4) return false;
  return HWP_SIGNATURE.every((value, index) => data[index] === value)
    || HWPX_SIGNATURE.every((value, index) => data[index] === value);
}

async function documentByteLimit() {
  const { maxFileSize } = await browser.storage.local.get({ maxFileSize: 20 });
  const requested = Number(maxFileSize);
  const megabytes = Number.isFinite(requested) ? Math.min(64, Math.max(1, requested)) : 20;
  return Math.floor(megabytes * 1024 * 1024);
}

async function fetchDocument(message) {
  if (!await validateFetchGrant(message.grant, message.url)) {
    return { error: '만료되었거나 일치하지 않는 파일 접근 권한' };
  }
  const { allowHttp } = await browser.storage.local.get({ allowHttp: true });
  if (!allowHttp && new URL(message.url).protocol === 'http:') {
    return { error: 'HTTP 차단 (설정에서 비허용)' };
  }
  try {
    const result = await fetchPublicResource(message.url, {
      maxBytes: Math.min(MAX_DOCUMENT_BYTES, await documentByteLimit()),
      timeoutMs: 60_000,
    });
    const contentType = (result.contentType || '').toLowerCase();
    if (contentType.includes('text/html') || contentType.includes('application/json')
        || contentType.includes('text/javascript')) {
      return { error: `예상치 않은 응답 유형: ${contentType}` };
    }
    if (!verifyHwpSignature(result.data)) {
      await logSecurity('signature-blocked', message.url, '매직 넘버 불일치');
      return { error: 'HWP 파일이 아닙니다' };
    }
    const transferId = await storeDocumentTransfer(result.data, result.contentType);
    return { transferId, contentType: result.contentType };
  } catch (error) {
    return { error: error.message };
  }
}

async function extractThumbnailFromUrl(url) {
  if (THUMBNAIL_CACHE.has(url)) return THUMBNAIL_CACHE.get(url);
  try {
    const { data } = await fetchPublicResource(resolveDocumentUrl(url), {
      maxBytes: MAX_THUMBNAIL_SOURCE_BYTES,
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

const messageHandlers = {
  'open-hwp': async (message, sender) => {
    const validation = validateContentTarget(message.url, sender.url);
    if (!validation.allowed) return { error: validation.reason };
    return openViewer({ url: message.url, filename: message.filename, explicit: true });
  },
  'fetch-file': fetchDocument,
  'extract-thumbnail': async (message, sender) => {
    const validation = validateContentTarget(message.url, sender.url);
    if (!validation.allowed) return { error: validation.reason };
    return await extractThumbnailFromUrl(message.url) || { error: 'PrvImage not found' };
  },
  'get-settings': async () => browser.storage.local.get({
    autoOpen: true,
    showBadges: true,
    hoverPreview: true,
    allowHttp: true,
    httpWarning: true,
    allowedDomains: DEFAULT_ALLOWED_DOMAINS,
    allSitesEnabled: false,
  }),
};

browser.runtime.onMessage.addListener((message, sender) => {
  const validation = validateMessage(message, sender);
  if (!validation.allowed) {
    logSecurity('sender-blocked', message?.url, validation.reason);
    return Promise.resolve({ error: validation.reason });
  }
  return Promise.resolve(messageHandlers[message.type](message, sender))
    .catch(error => ({ error: error.message }));
});

async function setupContextMenus() {
  await browser.contextMenus.removeAll();
  browser.contextMenus.create({
    id: MENU_ID,
    title: browser.i18n.getMessage('contextMenuOpen') || 'rhwp로 열기',
    contexts: ['link'],
  });
}

browser.contextMenus.onClicked.addListener((info) => {
  if (info.menuItemId === MENU_ID && info.linkUrl) {
    openViewer({ url: info.linkUrl, explicit: true })
      .catch(error => console.error('[rhwp] 컨텍스트 메뉴 열기 실패:', error));
  }
});

browser.runtime.onInstalled.addListener((details) => {
  setupContextMenus().catch(error => console.error('[rhwp] 컨텍스트 메뉴 초기화 실패:', error));
  if (details.reason === 'install') {
    browser.storage.local.set({
      autoOpen: true,
      showBadges: true,
      hoverPreview: true,
      allowHttp: true,
      httpWarning: true,
      securityLog: false,
      allowedDomains: DEFAULT_ALLOWED_DOMAINS,
      allSitesEnabled: false,
      maxFileSize: 20,
    });
  }
});

browser.action.onClicked.addListener(() => {
  openViewer().catch(error => console.error('[rhwp] 뷰어 탭 생성 실패:', error));
});

export { openViewer, validateContentTarget, validateMessage, verifyHwpSignature };
