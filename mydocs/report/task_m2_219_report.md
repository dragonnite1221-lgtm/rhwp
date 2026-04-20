# Task #219 — 최종 완료보고서

## iPad 뷰어: 로컬 파일 선택 로딩 기능 완성

### 목표

사용자가 직관적으로 로컬 파일을 선택하여 열 수 있도록 한다. 기존에는 `ContentView`가 `NavigationStack` 안에 없어 툴바 버튼이 표시되지 않았고, 번들 샘플 외 파일을 열 수단이 없었다.

### 단계별 결과

| 단계 | 내용 | 결과 |
|------|------|------|
| 1 | NavigationStack + 하단 툴바 | ✅ 파일 열기/페이지 번호/설정 자리 배치 |
| 2 | Info.plist UTType + Document Types | ✅ `com.hancom.hwp`/`.hwpx` 커스텀 UTI + Viewer 등록 |
| 3 | 에러 UX + UX/UI 피드백 반영 | ✅ RhwpError 세분화, 툴바 디자인 개선, 3-레이어 경계 |

### 핵심 설계

**레이아웃 (iPhone/iPad 공통):**

```
┌─────────────────────────────┐
│  🏝️ Dynamic Island (시스템)  │
├─────────────────────────────┤
│  알한글 — sample.hwpx         │  상단: 흰색 + Divider
├─────────────────────────────┤
│                             │
│  (회색 배경 #D1D1D1)          │  문서 영역: 중간 회색
│    ┌──────────────┐         │   └─ 페이지(흰색)와 대비
│    │              │         │
│    │   페이지 흰색   │         │
│    │              │         │
│    └──────────────┘         │
│                             │
├─────────────────────────────┤
│   📁        1 / 66          │  하단: 흰색 툴바
└─────────────────────────────┘
```

**UX/UI 전문 피드백 반영 (P0):**
- `folder.badge.plus` → `folder` (더 보편적인 "열기" 아이콘)
- Label(아이콘+텍스트) → Image + accessibilityLabel (Safari/Mail 패턴)
- `.caption` + "쪽" → `.footnote.monospacedDigit()` + "쪽" 제거
- 영구 disabled `gearshape` 제거
- **3-레이어 색상 대비**: 흰색(헤더) ↔ 회색(문서영역) ↔ 흰색(툴바)

### Info.plist 등록

```xml
<UTImportedTypeDeclarations>
    com.hancom.hwp → .hwp, application/x-hwp
    com.hancom.hwpx → .hwpx, application/hwp+zip
</UTImportedTypeDeclarations>

<CFBundleDocumentTypes>
    LSItemContentTypes: [com.hancom.hwp, com.hancom.hwpx]
    LSHandlerRank: Alternate
</CFBundleDocumentTypes>
```

→ 파일앱 또는 이메일에서 HWP 파일 → "알한글로 열기" 옵션 제공

### 에러 처리

| 상황 | 메시지 |
|------|--------|
| HWP 파싱 실패 | "이 파일은 HWP/HWPX 형식이 아니거나 손상되었습니다 ({filename})" |
| 파일 읽기 실패 | "파일을 읽을 수 없습니다 ({filename})" |
| 보안 범위 접근 거부 | "파일에 접근할 수 없습니다 ({filename}). 파일앱에서 다시 선택해 주세요." |

### 생성/변경 파일

| 파일 | 역할 |
|------|------|
| `rhwp-ios/Sources/ContentView.swift` | NavigationStack + 하단 툴바 + 개선된 아이콘 |
| `rhwp-ios/Sources/DocumentView.swift` | 헤더바 단순화 + Divider + 배경 |
| `rhwp-ios/Sources/DocumentPickerView.swift` | onError 콜백, 에러 세분화 |
| `rhwp-ios/Sources/DocumentViewModel.swift` | RhwpError errorDescription 활용 |
| `rhwp-ios/Sources/RhwpDocument.swift` | RhwpError 4개 케이스 확장 |
| `rhwp-ios/Sources/PagedScrollView.swift` | 문서 영역 배경 대비 강화 |
| `rhwp-ios/Sources/Info.plist` | UTImportedTypeDeclarations + CFBundleDocumentTypes |

### 검증 결과

- iPad Simulator (iPad Pro 11-inch M4): ✅ 3-레이어 경계, 툴바, 파일 선택
- iPhone 12 Pro 실기기: ✅ Dynamic Island 간섭 없음, 하단 툴바 정상, 경계 명확
- Xcode 빌드: ✅
- cargo test: ✅ (Rust 변경 없음, 회귀 없음)

### 후속 이관

| 항목 | 이관 대상 |
|------|-----------|
| 페이지 번호 탭 → 점프 시트 | #100 iPad 뷰어 UX 개선 |
| 뷰어 자동 숨김 (iPhone) | #100 |
| 공유 기능 (`UIActivityViewController`) | 후속 이슈 |
| 설정 화면 | 후속 이슈 |
| iPad NavigationSplitView + 썸네일 | M3 |
