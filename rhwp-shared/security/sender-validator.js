// rhwp 메시지 발신자 검증 모듈 — Chrome/Safari 공통
'use strict';

function runtimeApi() {
  return globalThis.browser?.runtime || globalThis.chrome?.runtime || null;
}

function hasExpectedExtensionId(sender) {
  const runtime = runtimeApi();
  return !!runtime?.id && sender?.id === runtime.id;
}

/**
 * 메시지 발신자가 확장 내부 페이지(viewer.html 등)인지 확인한다.
 * @param {object} sender — runtime.onMessage의 sender 파라미터
 * @returns {boolean}
 */
export function isInternalPage(sender) {
  const runtime = runtimeApi();
  if (!runtime || !hasExpectedExtensionId(sender) || !sender?.url) return false;
  const extensionBase = runtime.getURL('');
  try {
    const senderUrl = new URL(sender.url);
    const baseUrl = new URL(extensionBase);
    return senderUrl.protocol === baseUrl.protocol
      && senderUrl.hostname === baseUrl.hostname;
  } catch {
    return false;
  }
}

/**
 * 메시지 발신자가 content script(탭에서 실행)인지 확인한다.
 * @param {object} sender
 * @returns {boolean}
 */
export function isContentScript(sender) {
  if (!hasExpectedExtensionId(sender)) return false;
  if (!Number.isInteger(sender?.tab?.id) || !Number.isInteger(sender?.frameId)) return false;
  try {
    const url = new URL(sender.url);
    return url.protocol === 'https:' || url.protocol === 'http:';
  } catch {
    return false;
  }
}

/**
 * 메시지 유형별 발신자를 검증한다.
 * @param {string} messageType — 메시지 type 필드
 * @param {object} sender
 * @returns {{ allowed: boolean, reason: string }}
 */
export function validateSender(messageType, sender) {
  switch (messageType) {
    case 'fetch-file':
      // 내부 페이지(viewer.html)만 허용
      if (!isInternalPage(sender)) {
        return { allowed: false, reason: `fetch-file: 외부 발신자 차단 (${sender?.url || 'unknown'})` };
      }
      return { allowed: true, reason: '내부 페이지 확인' };

    case 'open-hwp':
    case 'prepare-viewer':
    case 'extract-thumbnail':
      // content script만 허용
      if (!isContentScript(sender)) {
        return { allowed: false, reason: `open-hwp: content script가 아닌 발신자 (tab=${sender?.tab?.id})` };
      }
      return { allowed: true, reason: 'content script 확인' };

    case 'get-settings':
      if (!isInternalPage(sender) && !isContentScript(sender)) {
        return { allowed: false, reason: 'get-settings: 확장 발신자 확인 실패' };
      }
      return { allowed: true, reason: '확장 발신자 확인' };

    default:
      return { allowed: false, reason: `알 수 없는 메시지 유형: ${messageType}` };
  }
}
