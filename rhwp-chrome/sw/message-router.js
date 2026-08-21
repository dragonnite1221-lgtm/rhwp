// Content Script ↔ Service Worker 메시지 라우팅
// - Content Script에서 파일 열기 요청
// - 뷰어 탭에서 파일 fetch 요청 (CORS 우회)
// - 향후: 호버 미리보기, 파일 캐싱 등

import { openViewer } from './viewer-launcher.js';
import { extractThumbnailFromUrl } from './thumbnail-extractor.js';
import { fetchPublicResource } from './secure-fetch.js';
import { validateFetchGrant } from './fetch-grants.js';
import { resolveDocumentUrl } from './document-url-resolver.js';
import { validatePublicUrl } from '../security/url-validator.js';
import { validateSender } from '../security/sender-validator.js';
import { storeDocumentTransfer } from './document-transfer-store.js';

const MAX_DOCUMENT_BYTES = 64 * 1024 * 1024;

function validateMessage(message, sender) {
  if (!message || typeof message !== 'object' || Array.isArray(message)) {
    return { allowed: false, reason: '메시지 객체가 올바르지 않음' };
  }
  if (typeof message.type !== 'string' || message.type.length > 64) {
    return { allowed: false, reason: '메시지 유형이 올바르지 않음' };
  }
  const senderResult = validateSender(message.type, sender);
  if (!senderResult.allowed) return senderResult;
  if (message.type !== 'get-settings') {
    if (typeof message.url !== 'string' || message.url.length > 8192) {
      return { allowed: false, reason: 'URL 필드가 올바르지 않음' };
    }
  }
  return { allowed: true, reason: '메시지와 발신자 확인' };
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

/**
 * 메시지 라우터를 설정한다.
 */
export function setupMessageRouter() {
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    const validation = validateMessage(message, sender);
    if (!validation.allowed) {
      sendResponse({ error: validation.reason });
      return false;
    }
    const handler = messageHandlers[message.type];
    if (!handler) {
      sendResponse({ error: '지원하지 않는 메시지 유형' });
      return false;
    }
    Promise.resolve(handler(message, sender))
      .then(sendResponse)
      .catch(err => sendResponse({ error: err.message }));
    return true;
  });
}

const messageHandlers = {
  /**
   * Content Script → Service Worker: HWP 파일 열기 요청
   */
  'open-hwp': async (message, sender) => {
    const validation = validateContentTarget(message.url, sender.url);
    if (!validation.allowed) return { error: validation.reason };
    await openViewer({ url: message.url, filename: message.filename });
    return { ok: true };
  },

  /**
   * 뷰어 탭 → Service Worker: CORS 우회 파일 fetch
   * Service Worker의 fetch는 host_permissions에 의해 CORS 제한 없음
   */
  'fetch-file': async (message) => {
    try {
      if (!await validateFetchGrant(message.grant, message.url)) {
        return { error: '만료되었거나 일치하지 않는 파일 접근 권한' };
      }
      const result = await fetchPublicResource(message.url, {
        maxBytes: MAX_DOCUMENT_BYTES,
        timeoutMs: 60_000,
      });
      const transferId = await storeDocumentTransfer(result.data, result.contentType);
      return { transferId, contentType: result.contentType };
    } catch (err) {
      return { error: err.message };
    }
  },

  /**
   * Content Script → Service Worker: HWP 썸네일 추출
   * Service Worker에서 fetch + CFB PrvImage 추출 (CORS 우회)
   */
  'extract-thumbnail': async (message, sender) => {
    try {
      const validation = validateContentTarget(message.url, sender.url);
      if (!validation.allowed) return { error: validation.reason };
      const result = await extractThumbnailFromUrl(message.url);
      return result || { error: 'PrvImage not found' };
    } catch (err) {
      return { error: err.message };
    }
  },

  /**
   * Content Script → Service Worker: 설정 조회
   */
  'get-settings': async () => {
    const settings = await chrome.storage.sync.get({
      autoOpen: true,
      showBadges: true,
      hoverPreview: true
    });
    return settings;
  }
};

export { validateContentTarget, validateMessage };
