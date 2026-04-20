# Task #219 — 수행계획서

## iPad 뷰어: 로컬 파일 선택 로딩 기능 완성

### 배경

Task #93에서 `DocumentPickerView` + `ContentView` 툴바 버튼이 구현되었지만, `ContentView`가 `NavigationStack` 안에 배치되지 않아 **툴바가 표시되지 않는다**. 실기기/시뮬레이터 모두 번들에 포함된 `sample.hwpx`만 열리고 사용자가 다른 파일을 선택할 수단이 없다.

### 현재 코드 상태

```swift
// ContentView.swift
DocumentView(viewModel: viewModel)
    .toolbar { ... }  // NavigationStack 없이는 표시 안됨
    .sheet(...) { DocumentPickerView { ... } }
```

- `DocumentPickerView.swift`: UIDocumentPickerViewController 래퍼, 보안 범위 접근 처리 포함
- UTType: `.data` + `com.hancom.hwp` + `com.hancom.hwpx` (커스텀 UTI는 없을 시 폴백)
- Info.plist: Document Types 미등록 — 다른 앱에서 "알한글로 열기" 불가

### 목표

사용자가 직관적으로 로컬 파일(iCloud Drive, 파일앱, "내 iPhone" 등)을 선택하여 열 수 있도록 한다.

1. **열기 버튼 노출**: 확실히 표시되는 위치에 배치
2. **파일 선택 → 로드 → 렌더링** 플로우 정상 동작 검증
3. **HWP/HWPX UTType 처리**: 타 앱 공유/확장자 연관 시 앱에서 열기
4. **에러 UX**: 지원하지 않는 파일 선택 시 사용자에게 안내

### 범위

**포함:**
- NavigationStack 도입 또는 헤더바 직접 배치
- 파일 선택 플로우 검증 (시뮬레이터 + 실기기)
- Info.plist Document Types + UTImportedTypeDeclarations
- 파일 로드 실패 시 에러 메시지 개선

**제외 (#100 UX 개선에서 처리):**
- 최근 파일 목록 (recent files)
- 빈 상태 안내 화면 개선
- 앱 아이콘 롱프레스 바로가기

### 기술 검토

**채택: NavigationStack + 하단 툴바**

iPhone의 Dynamic Island/노치가 상단 영역을 차지하므로, iOS 표준처럼 **하단 툴바**(Safari, Files, Mail 등과 동일)로 액션 버튼을 배치한다.

```
┌─────────────────────────┐
│  🏝️ Dynamic Island      │  ← 상단 (시스템 영역)
├─────────────────────────┤
│  알한글 — sample.hwpx    │  ← DocumentView 헤더바 (정보: 파일명, 페이지)
├─────────────────────────┤
│                         │
│      문서 렌더링          │  ← PagedScrollView
│                         │
├─────────────────────────┤
│  📁 열기      1/66쪽 ⚙️  │  ← 하단 툴바 (액션: 파일 열기, 설정)
└─────────────────────────┘
```

- `NavigationStack` 도입으로 `.toolbar { ToolbarItem(placement: .bottomBar) { ... } }` 사용
- 상단 헤더바는 정보만 (파일명, 페이지 번호 등), 하단은 액션 버튼
- iPad에서도 동일한 구조가 적용되나 iPad는 여유 있으므로 문제 없음

### 상단 헤더바 vs 하단 툴바 구분

| 영역 | 역할 | 예시 |
|------|------|------|
| 상단 헤더바 (DocumentView 내부) | **정보 표시** (변경 불가) | "알한글 — sample.hwpx" |
| 하단 툴바 (NavigationStack) | **액션** (사용자 조작) | 📁 열기, 설정, 페이지 이동 |

### 위험 요소

| 위험 | 대응 |
|------|------|
| 실기기에서 보안 범위 접근 실패 | `startAccessingSecurityScopedResource` 이미 처리됨 |
| HWP 커스텀 UTI 미등록 | Info.plist `UTImportedTypeDeclarations` 추가 |
| iCloud Drive에서 다운로드 지연 | 로딩 인디케이터 표시 (isLoading 이미 존재) |

### 산출물

| 파일 | 내용 |
|------|------|
| `rhwp-ios/Sources/ContentView.swift` | NavigationStack 도입 + 하단 툴바에 파일 열기 버튼 |
| `rhwp-ios/Sources/DocumentView.swift` | 헤더바는 정보만 (변경 최소) |
| `rhwp-ios/Sources/Info.plist` | Document Types, UTImportedTypeDeclarations 추가 |
| `mydocs/working/task_m2_219_stage*.md` | 단계별 완료보고서 |
| `mydocs/report/task_m2_219_report.md` | 최종 완료보고서 |
