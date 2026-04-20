# Task #219 — 1단계 완료보고서

## NavigationStack + 하단 툴바 구조 전환 ✅

### 작업 내용

1. **ContentView** — `NavigationStack` 도입 + `.toolbar { ToolbarItemGroup(placement: .bottomBar) }`
2. **DocumentView 헤더바 단순화** — 페이지 번호/크기 표시를 하단 툴바로 이관, 헤더바는 파일명만 유지
3. **역할 분리** — 상단 정보 / 하단 액션

### UI 구조

```
┌─────────────────────────────┐
│  🏝️ Dynamic Island (시스템)  │
├─────────────────────────────┤
│  알한글 — sample.hwpx         │  상단 헤더바 (정보)
├─────────────────────────────┤
│                             │
│       PagedScrollView        │  문서 렌더링
│                             │
├─────────────────────────────┤
│  📁      1 / 66쪽       ⚙️   │  하단 툴바 (액션)
└─────────────────────────────┘
```

### 하단 툴바 구성

| 위치 | 항목 | 역할 |
|------|------|------|
| 좌측 | 📁 열기 | `showFilePicker = true` → DocumentPickerView 표시 |
| 중앙 | "N / 총쪽수" | 현재 페이지 번호 (viewModel 관찰) |
| 우측 | ⚙️ 설정 | 비활성 (후속 #100) |

### 검증 결과

- Xcode 빌드: ✅ BUILD SUCCEEDED
- iPad Simulator 시각 확인: ✅ 하단 툴바 정상 표시
- 상단 Dynamic Island 영역과 간섭 없음

### 미구현 (후속 단계)

| 항목 | 단계 |
|------|------|
| UTType / Document Types 등록 | 2단계 |
| 에러 UX 개선 | 3단계 |
| 실기기 검증 | 3단계 |
