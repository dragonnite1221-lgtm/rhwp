// rhwp URL 검증 모듈 — Chrome/Safari 공통
'use strict';

/**
 * URL이 안전한 프로토콜인지 검증한다.
 * @param {string} urlString
 * @returns {{ valid: boolean, parsed?: URL, reason?: string }}
 */
export function validateProtocol(urlString) {
  if (!urlString || typeof urlString !== 'string') {
    return { valid: false, reason: 'URL이 비어있음' };
  }
  let parsed;
  try {
    parsed = new URL(urlString);
  } catch {
    return { valid: false, reason: 'URL 파싱 실패' };
  }
  if (parsed.protocol !== 'https:' && parsed.protocol !== 'http:') {
    return { valid: false, reason: `차단된 프로토콜: ${parsed.protocol}` };
  }
  // userinfo(@) 포함 도메인 차단: https://safe.go.kr@evil.com/
  if (parsed.username || parsed.password) {
    return { valid: false, reason: 'URL에 userinfo(@) 포함' };
  }
  return { valid: true, parsed };
}

/**
 * 호스트가 내부 네트워크 IP인지 검사한다.
 * @param {string} hostname
 * @returns {boolean} 내부 IP이면 true
 */
export function isPrivateHost(hostname) {
  if (typeof hostname !== 'string' || !hostname) return true;
  const host = hostname.toLowerCase().replace(/^\[|\]$/g, '').replace(/\.$/, '');
  if (!host.includes('.') && !host.includes(':')) return true;
  if (/\.(?:local|localhost|internal|home|lan)$/.test(host)) return true;
  return isNonPublicIpAddress(host);
}

/** Return true for IP literals that are private, reserved, or non-routable. */
export function isNonPublicIpAddress(address) {
  if (typeof address !== 'string' || !address) return true;
  const value = address.toLowerCase().replace(/^\[|\]$/g, '');
  if (value.includes(':')) {
    const mapped = value.match(/^(?:::ffff:|0:0:0:0:0:ffff:)(.+)$/);
    if (mapped) {
      if (mapped[1].includes('.')) return isNonPublicIpAddress(mapped[1]);
      const words = mapped[1].split(':');
      if (words.length !== 2 || words.some(word => !/^[0-9a-f]{1,4}$/.test(word))) return true;
      const high = Number.parseInt(words[0], 16);
      const low = Number.parseInt(words[1], 16);
      return isNonPublicIpAddress([
        high >>> 8,
        high & 0xff,
        low >>> 8,
        low & 0xff,
      ].join('.'));
    }
    if (value === '::' || value === '::1') return true;
    if (/^(?:fc|fd|fe[89ab]|ff)/.test(value)) return true;
    if (/^2001:db8(?::|$)/.test(value)) return true;
    const first = Number.parseInt(value.split(':', 1)[0], 16);
    return !Number.isInteger(first) || first < 0x2000 || first > 0x3fff;
  }

  const parts = value.split('.');
  if (parts.length !== 4 || parts.some(part => !/^\d{1,3}$/.test(part))) {
    return false;
  }
  const octets = parts.map(Number);
  if (octets.some(part => part > 255)) return true;
  const [a, b, c] = octets;
  return a === 0
    || a === 10
    || a === 127
    || (a === 100 && b >= 64 && b <= 127)
    || (a === 169 && b === 254)
    || (a === 172 && b >= 16 && b <= 31)
    || (a === 192 && b === 168)
    || (a === 192 && b === 0 && c === 0)
    || (a === 192 && b === 0 && c === 2)
    || (a === 192 && b === 88 && c === 99)
    || (a === 198 && (b === 18 || b === 19))
    || (a === 198 && b === 51 && c === 100)
    || (a === 203 && b === 0 && c === 113)
    || a >= 224;
}

export function isIpAddressLiteral(address) {
  if (typeof address !== 'string' || !address) return false;
  const value = address.toLowerCase().replace(/^\[|\]$/g, '');
  if (value.includes(':')) {
    if (!/^[0-9a-f:.]+$/.test(value)) return false;
    if ((value.match(/::/g) || []).length > 1) return false;
    const compressed = value.includes('::');
    const parts = value.split(':').filter(Boolean);
    let slots = 0;
    for (const part of parts) {
      if (part.includes('.')) {
        const ipv4 = part.split('.');
        if (ipv4.length !== 4 || ipv4.some(item => (
          !/^\d{1,3}$/.test(item) || Number(item) > 255
        ))) return false;
        slots += 2;
      } else {
        if (!/^[0-9a-f]{1,4}$/.test(part)) return false;
        slots += 1;
      }
    }
    return compressed ? slots < 8 : slots === 8;
  }
  const parts = value.split('.');
  return parts.length === 4
    && parts.every(part => /^\d{1,3}$/.test(part) && Number(part) <= 255);
}

/**
 * URL의 pathname에서 HWP 확장자를 확인한다.
 * @param {URL} parsed
 * @returns {boolean}
 */
export function hasHwpExtension(parsed) {
  const pathname = parsed.pathname.toLowerCase();
  return pathname.endsWith('.hwp') || pathname.endsWith('.hwpx');
}

/**
 * 호스트가 허용 도메인 목록에 포함되는지 검사한다.
 * @param {string} hostname
 * @param {string[]} allowedDomains — ['.go.kr', '.or.kr', ...]
 * @returns {boolean}
 */
export function isAllowedDomain(hostname, allowedDomains) {
  return allowedDomains.some(domain => hostname.endsWith(domain));
}

/**
 * URL이 정부사이트 다운로드 패턴인지 검사한다.
 * 확장자 없이 *.do, *Download*, *download* 등의 패턴.
 * @param {URL} parsed
 * @returns {boolean}
 */
export function isDownloadEndpoint(parsed) {
  const pathname = parsed.pathname.toLowerCase();
  return /\.(do|action|jsp|aspx|php)$/i.test(pathname)
    || /download/i.test(pathname)
    || /filedown/i.test(pathname)
    || /attach/i.test(pathname);
}

/**
 * open-hwp 용 URL 검증 (3단계).
 * ① pathname에 .hwp/.hwpx → 즉시 허용
 * ② 허용 도메인 + 다운로드 패턴 → 허용 (viewer에서 재검증)
 * ③ 그 외 → 차단
 *
 * @param {string} urlString
 * @param {string[]} allowedDomains
 * @returns {{ allowed: boolean, reason: string }}
 */
export function validateOpenHwpUrl(urlString, allowedDomains) {
  const result = validateProtocol(urlString);
  if (!result.valid) return { allowed: false, reason: result.reason };
  const parsed = result.parsed;

  if (isPrivateHost(parsed.hostname)) {
    return { allowed: false, reason: `내부 IP 차단: ${parsed.hostname}` };
  }

  // ① 확장자 확인
  if (hasHwpExtension(parsed)) {
    return { allowed: true, reason: 'HWP 확장자 확인' };
  }

  // ② 허용 도메인 + 다운로드 패턴
  if (isAllowedDomain(parsed.hostname, allowedDomains)) {
    if (isDownloadEndpoint(parsed)) {
      return { allowed: true, reason: '허용 도메인 다운로드 엔드포인트 (viewer에서 재검증)' };
    }
    // 허용 도메인이지만 다운로드 패턴이 아닌 경우도 허용 (content-script가 HWP 링크로 판별했으므로)
    return { allowed: true, reason: '허용 도메인 (viewer에서 재검증)' };
  }

  // ③ 그 외 차단
  return { allowed: false, reason: `미허용 도메인 + 확장자 없음: ${parsed.hostname}` };
}

/**
 * fetch-file 용 URL 검증.
 * open-hwp 보다 엄격: HTTPS 강제, 내부 IP 차단.
 *
 * @param {string} urlString
 * @param {string[]} allowedDomains
 * @param {boolean} allowHttp — 사용자 설정
 * @returns {{ allowed: boolean, reason: string, upgradedUrl?: string }}
 */
export function validateFetchUrl(urlString, allowedDomains, allowHttp) {
  const result = validateProtocol(urlString);
  if (!result.valid) return { allowed: false, reason: result.reason };
  const parsed = result.parsed;

  if (isPrivateHost(parsed.hostname)) {
    return { allowed: false, reason: `내부 IP 차단: ${parsed.hostname}` };
  }

  // HTTPS 강제 (설정에 따라 HTTP 허용)
  if (parsed.protocol === 'http:') {
    if (!allowHttp) {
      return { allowed: false, reason: 'HTTP 차단 (설정에서 비허용)' };
    }
    // HTTP → HTTPS 업그레이드 시도
    const upgraded = urlString.replace(/^http:/, 'https:');
    return { allowed: true, reason: 'HTTP → HTTPS 업그레이드', upgradedUrl: upgraded };
  }

  return { allowed: true, reason: '검증 통과' };
}

/** Validate a URL before any privileged extension fetch or navigation. */
export function validatePublicUrl(urlString) {
  const result = validateProtocol(urlString);
  if (!result.valid) return { allowed: false, reason: result.reason };
  const parsed = result.parsed;
  if (!parsed.hostname || parsed.port.length > 5) {
    return { allowed: false, reason: '호스트 또는 포트가 올바르지 않음' };
  }
  if (isPrivateHost(parsed.hostname)) {
    return { allowed: false, reason: `비공개 또는 예약 호스트 차단: ${parsed.hostname}` };
  }
  return { allowed: true, reason: '공개 HTTP(S) URL', parsed };
}

/**
 * Privileged extension fetches require HTTPS. A public DNS answer can rebind
 * between validation and browser connection; TLS hostname verification keeps
 * the browser from sending the HTTP request to an unrelated private service.
 */
export function validatePublicHttpsUrl(urlString) {
  const result = validatePublicUrl(urlString);
  if (!result.allowed) return result;
  if (result.parsed.protocol !== 'https:') {
    return { allowed: false, reason: '권한 있는 네트워크 요청은 HTTPS만 허용' };
  }
  return result;
}

/** Require every DNS answer to be a public, globally routable address. */
export function validateResolvedAddresses(addresses) {
  if (!Array.isArray(addresses) || addresses.length === 0) {
    return { allowed: false, reason: 'DNS 공개 주소를 확인하지 못함' };
  }
  const blocked = addresses.find(address => (
    !isIpAddressLiteral(address) || isNonPublicIpAddress(address)
  ));
  if (blocked) {
    return { allowed: false, reason: `DNS가 비공개 또는 예약 주소를 반환함: ${blocked}` };
  }
  return { allowed: true, reason: 'DNS 공개 주소 확인' };
}
