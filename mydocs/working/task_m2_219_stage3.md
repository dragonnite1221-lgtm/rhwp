# Task #219 — 3단계 완료보고서

## 에러 UX 개선 + UX/UI 피드백 반영 + 실기기 검증 ✅

### 작업 내용

1. **에러 UX 개선** — `RhwpError` 케이스 세분화, 파일명 포함
2. **UX/UI 전문 피드백 반영** — 하단 툴바 디자인 개선
3. **시각적 경계 강화** — 헤더/문서영역/툴바 색상 대비
4. **iPad Simulator + iPhone 12 Pro 실기기 검증**

### 에러 UX 개선

`RhwpError` 케이스 확장:

| 케이스 | 메시지 |
|--------|--------|
| `parseFailure(filename:)` | "이 파일은 HWP/HWPX 형식이 아니거나 손상되었습니다 (파일명)." |
| `invalidData` | "유효하지 않은 데이터입니다." |
| `fileReadFailure(filename:)` | "파일을 읽을 수 없습니다 (파일명)." |
| `accessDenied(filename:)` | "파일에 접근할 수 없습니다 (파일명). 파일앱에서 다시 선택해 주세요." |

`DocumentPickerView`에 `onError` 콜백 추가:
- 보안 범위 접근 실패 → `accessDenied`
- 파일 읽기 실패 → `fileReadFailure`
- 각 케이스에 filename 전달

### UX/UI 피드백 반영 (하단 툴바)

리뷰: `mydocs/feedback/task_m2_219_toolbar_review.md` (종합 등급 B−)

**P0 반영 항목:**

| 항목 | 변경 전 | 변경 후 |
|------|---------|---------|
| 좌측 아이콘 | `folder.badge.plus` + "열기" 라벨 | `folder` 아이콘만 |
| 접근성 | Label 자동 | `.accessibilityLabel("문서 열기")` + Hint |
| 중앙 텍스트 | "1 / 66쪽" (`.caption`) | "1 / 66" (`.footnote.monospacedDigit()`) |
| 우측 버튼 | 영구 disabled `gearshape` | 투명 플레이스홀더 (균형용) |
| 좌우 일관성 | Label + Image 혼용 | 아이콘만으로 통일 |

### 시각적 경계 강화

**문제**: 상단/문서/하단 모두 비슷한 연한 회색으로 경계 불명확.

**해결**:
- **상단 헤더바**: `secondarySystemBackground` → `systemBackground`(흰색) + Divider
- **하단 툴바**: `.toolbarBackground(.visible, for: .bottomBar)` + `systemBackground`
- **문서 영역**: `systemGroupedBackground` → 중간 회색 (`UIColor(white: 0.82, alpha: 1.0)`) + 다크모드 대응

3-레이어 대비로 현대적이고 명확한 경계 구현.

### 변경 파일

| 파일 | 변경 |
|------|------|
| `rhwp-ios/Sources/RhwpDocument.swift` | RhwpError 케이스 확장 + filename 전달 |
| `rhwp-ios/Sources/DocumentPickerView.swift` | onError 콜백, 접근/읽기 실패 구분 |
| `rhwp-ios/Sources/DocumentViewModel.swift` | RhwpError `errorDescription` 사용 |
| `rhwp-ios/Sources/ContentView.swift` | 툴바 아이콘/라벨 개선 + 배경색 |
| `rhwp-ios/Sources/DocumentView.swift` | 헤더바 Divider + 배경색 |
| `rhwp-ios/Sources/PagedScrollView.swift` | 문서 영역 배경 대비 강화 |

### 검증 결과

- iPad Simulator: ✅ 3-레이어 경계 명확, 모던한 UX
- iPhone 12 Pro 실기기: ✅ 경계 정상, Dynamic Island 간섭 없음
- Xcode 빌드: ✅
- cargo test: ✅ (Rust 변경 없음)

### 미반영 (후속 이관)

| 항목 | 이관 |
|------|------|
| 페이지 번호 탭 → 점프 시트 (P1) | #100 |
| 뷰어 자동 숨김 모드 (iPhone) | #100 |
| 공유 버튼 (`UIActivityViewController`) | 후속 이슈 |
| iPad NavigationSplitView + 썸네일 사이드바 | M3 |
