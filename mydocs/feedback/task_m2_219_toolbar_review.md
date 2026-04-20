# 알한글 iOS 하단 툴바 UX/UI 리뷰 — Task M2 #219

리뷰 대상: `rhwp-ios/Sources/ContentView.swift` 의 `ToolbarItemGroup(placement: .bottomBar)`
리뷰어: UX/UI 전문 리뷰 (Apple HIG 기준)
대상 디바이스: iPhone (Dynamic Island) + iPad

---

## 1. 종합 평가 등급: **B−**

| 항목 | 등급 | 근거 |
|---|---|---|
| HIG 준수도 | B | `.bottomBar` 플레이스먼트, SF Symbols 사용, Safe Area 자동 처리 등 기본은 잘 지켜짐. 다만 아이콘 혼합(Label vs Image only)과 disabled 상태 노출은 HIG 권고 위반에 가까움 |
| 기능성 | C | "열기"와 "쪽수" 외에는 실제 기능 전무. 문서 뷰어에 필수적인 "페이지 이동/검색/공유/줌" 액션 부재 |
| 시각 일관성 | B− | 왼쪽은 Label(아이콘+텍스트), 오른쪽은 Image(아이콘만)로 **비대칭**. 시선 무게중심이 왼쪽으로 쏠림 |
| 접근성 | C | Label은 자동 VO 라벨 획득하나, Image(systemName:)만 쓴 gearshape는 `.accessibilityLabel`이 없어 VoiceOver에서 "gearshape"로 읽힐 위험 |
| 확장성 | B | NavigationStack 기반이라 탭/시트 확장은 용이. 단, bottomBar의 Spacer 3등분은 버튼이 늘어나면 재설계 불가피 |

**총평**: "동작은 하지만 정체성이 없다." 현재 툴바는 iOS의 기본 컴포넌트를 올바르게 사용했지만, **문서 뷰어**라는 앱 성격에 맞춘 액션 구성·아이콘 선택·라벨 전략이 부재한 상태. Apple Books / Preview / Adobe Acrobat Reader iOS가 채택한 "뷰어 모드에서는 컨트롤이 자동 숨김, 탭하면 등장" 패턴을 고려하면 지금 항상 노출되는 툴바는 페이지 가독성에도 손해를 준다.

---

## 2. HIG 준수도 평가

### 2.1 부합 항목 ✓

- **Bottom toolbar 위치**: HIG "Toolbars" 섹션은 iPhone에서 주요 액션을 bottomBar에 배치하도록 권장 (한 손 조작). `.bottomBar` placement 사용은 적절.
- **SF Symbols**: `folder.badge.plus`, `gearshape` 모두 표준 심볼이며 다크/라이트 모드 자동 대응.
- **Safe Area**: SwiftUI `.toolbar` 모디파이어는 홈 인디케이터 영역을 자동 회피. 별도 처리 불필요.
- **터치 타겟**: ToolbarItem은 기본 44×44pt 히트박스를 보장.

### 2.2 위반 / 경계 항목 ✗

| 항목 | HIG 권고 | 현재 상태 |
|---|---|---|
| **아이콘 일관성** | 툴바 내 아이템은 라벨 유무를 통일 | Label("열기") + Image(gearshape) 혼재 |
| **Disabled 노출** | "사용자가 일시적으로 사용할 수 없는 기능"에만 쓸 것. 아예 미구현이면 **감추기** 권고 | 영구적으로 disabled인 gearshape 노출 |
| **중앙 라벨** | bottomBar 중앙에 정적 텍스트를 두는 패턴은 iOS 16+ `ToolbarItem(placement: .principal)` 또는 `.status` 전용 | `Spacer() - Text - Spacer()`로 수동 구현 (기능적으론 동일하나 시맨틱하지 않음) |
| **VoiceOver 라벨** | 모든 인터랙티브 요소에 의미 있는 라벨 | `Image(systemName: "gearshape")`는 `.accessibilityLabel("설정")` 누락 |
| **"쪽" 단위 표기** | 지역화 대응 | 현재 하드코딩. LocalizedStringKey 미사용 |

---

## 3. 우선순위별 개선안

### P0 (필수, 이번 태스크에서 반영)

1. **비활성 `gearshape` 버튼 제거**
   - 영구 disabled 버튼은 HIG 위반. 설정 기능이 실제로 구현되는 시점에 추가한다.
   - 대신 **뷰어 앱에 실제로 유용한 액션**을 배치:
     - **공유**(`square.and.arrow.up`) — `UIActivityViewController`로 원본 HWP/HWPX 공유
     - **또는 페이지 점프**(`text.magnifyingglass` 혹은 `arrow.up.to.line` / `arrow.down.to.line`)

2. **아이콘 일관성 확보 — 라벨 제거 쪽으로 통일**
   - Safari/Mail/Books 모두 bottomBar는 **아이콘 전용**.
   - `Label("열기", systemImage:)` → `Image(systemName: "folder")` + `.accessibilityLabel("문서 열기")`.
   - 공간 절약 + Safari류 감성과 일치.

3. **아이콘 재검토**
   - `folder.badge.plus`는 "폴더에 새 항목 추가"(생성) 뉘앙스가 강함 → **열기에는 다소 부적절**.
   - **권장: `folder`** (Files 앱이 "폴더 열기" 맥락에서 사용).
   - 대안: `doc.text.magnifyingglass` (문서 탐색), `tray.and.arrow.down` (가져오기). Books는 서재 아이콘 `books.vertical` 사용.
   - 결론: **`folder`가 가장 보편적이며 "파일 선택"이라는 동작과 일치**.

4. **VoiceOver 라벨 명시**
   ```swift
   Button { ... } label: { Image(systemName: "folder") }
       .accessibilityLabel("문서 열기")
       .accessibilityHint("파일 앱에서 HWP 또는 HWPX 문서를 선택합니다")
   ```

### P1 (권장, 차기 태스크)

5. **중앙 페이지 번호를 인터랙티브 컨트롤로 승격**
   - 정적 텍스트 → **탭하면 페이지 점프 시트 등장**.
   - Apple Books / Acrobat Reader iOS 동일 패턴.
   - 시트 내용: 슬라이더 + "N쪽으로 이동" 입력 필드.

6. **"1 / 66쪽" → "1 / 66"로 단순화**
   - 한국어 뷰어도 Apple Books 한국어판은 "1페이지 / 66"이 아니라 **"1 / 66"** 만 쓴다 (시스템 폭 절약).
   - "쪽" 접미어가 꼭 필요하면 `.accessibilityLabel("66쪽 중 1쪽")`로만 읽어주고 시각 표시는 `1 / 66`.
   - 중앙이 버튼이 되면 접근성 라벨만 "쪽"을 포함해도 충분.

7. **뷰어 자동 숨김 모드 도입 (iPhone)**
   - 탭 한 번 → 상/하단 툴바 페이드 아웃, 본문만 전체화면.
   - `@State private var chromeVisible = true` + `.animation(.easeInOut(duration: 0.2))`.
   - iPad는 화면이 커서 항상 표시해도 방해 적으니 **디바이스별 분기**.

8. **iPad 전용 레이아웃 고려**
   - iPad는 bottomBar가 너무 길고 비어 보임. **상단 `navigationBar`로 이동**하거나 **NavigationSplitView + 사이드바(페이지 썸네일)** 패턴이 적합.
   - 현재 M2 범위 초과 → 별도 이슈(M3 후보).

### P2 (선택, 장기)

9. **최근 파일 드롭다운** — `folder` 버튼 **길게 누르기** 메뉴로 최근 5개 파일.
10. **다크 모드 검증** — 현재 코드는 시스템 배경 사용으로 자동 대응하나, **실제 다크 모드 스크린샷 확인 필요**.
11. **Dynamic Type** — `Text("\(currentPage + 1) / \(pageCount)쪽")`의 `.caption`은 시스템 확대 시 가독성 약함. 최소 `.footnote` 또는 `.monospacedDigit()` 적용 권장.

---

## 4. 구체적 Swift 코드 제안

### 변경 전 (현재)

```swift
ToolbarItemGroup(placement: .bottomBar) {
    Button {
        showFilePicker = true
    } label: {
        Label("열기", systemImage: "folder.badge.plus")
    }
    Spacer()
    if viewModel.pageCount > 0 {
        Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)쪽")
            .font(.caption)
            .foregroundColor(.secondary)
    }
    Spacer()
    Button { } label: {
        Image(systemName: "gearshape")
    }
    .disabled(true)
}
```

### 변경 후 (P0 반영 최소본)

```swift
ToolbarItemGroup(placement: .bottomBar) {
    // 좌: 문서 열기 (아이콘만, 접근성 라벨 명시)
    Button {
        showFilePicker = true
    } label: {
        Image(systemName: "folder")
    }
    .accessibilityLabel("문서 열기")
    .accessibilityHint("파일 앱에서 HWP 또는 HWPX 문서를 선택합니다")

    Spacer()

    // 중: 페이지 번호 (향후 P1에서 버튼으로 승격)
    if viewModel.pageCount > 0 {
        Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)")
            .font(.footnote.monospacedDigit())
            .foregroundColor(.secondary)
            .accessibilityLabel("\(viewModel.pageCount)쪽 중 \(viewModel.currentPage + 1)쪽")
    }

    Spacer()

    // 우: 공유 (gearshape 대신, 문서 뷰어에 실제로 유용)
    Button {
        shareCurrentDocument()
    } label: {
        Image(systemName: "square.and.arrow.up")
    }
    .accessibilityLabel("공유")
    .disabled(viewModel.document == nil)
}
```

### 변경 후 (P1 반영 권장본)

```swift
ToolbarItemGroup(placement: .bottomBar) {
    Button { showFilePicker = true } label: {
        Image(systemName: "folder")
    }
    .accessibilityLabel("문서 열기")

    Spacer()

    // 중앙을 탭하면 페이지 점프 시트
    if viewModel.pageCount > 0 {
        Button { showPageJumpSheet = true } label: {
            Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)")
                .font(.footnote.monospacedDigit())
                .foregroundColor(.primary)
        }
        .accessibilityLabel("\(viewModel.pageCount)쪽 중 \(viewModel.currentPage + 1)쪽")
        .accessibilityHint("탭하여 원하는 쪽으로 이동합니다")
    }

    Spacer()

    Button { shareCurrentDocument() } label: {
        Image(systemName: "square.and.arrow.up")
    }
    .disabled(viewModel.document == nil)
    .accessibilityLabel("공유")
}
.sheet(isPresented: $showPageJumpSheet) {
    PageJumpView(
        currentPage: viewModel.currentPage,
        pageCount: viewModel.pageCount
    ) { newPage in
        viewModel.currentPage = newPage
    }
    .presentationDetents([.fraction(0.3)])
}
```

---

## 5. A/B 비교 스케치

### A) 현재 디자인

```
┌────────────────────────────────────────────────┐
│                                                │
│                 (본문 렌더링)                   │
│                                                │
├────────────────────────────────────────────────┤
│ [📁+ 열기]        1 / 66쪽         [⚙️]        │
│  ↑ Label        ↑ .caption      ↑ disabled    │
│  비대칭 무게      회색 정적 텍스트    영구 비활성    │
└────────────────────────────────────────────────┘
```

문제:
- 좌측만 텍스트 라벨 → 시각 무게 쏠림
- 중앙은 인터랙티브 힌트 없음 (누르면 뭐 되는지 모름)
- 우측은 "곧 옵니다" 대신 현재는 잡음

### B) 개선안 (P0)

```
┌────────────────────────────────────────────────┐
│                                                │
│                 (본문 렌더링)                   │
│                                                │
├────────────────────────────────────────────────┤
│    [📁]            1 / 66            [⬆]       │
│     ↑            ↑ .footnote          ↑        │
│   아이콘만      monospacedDigit       공유       │
│   균등 무게       시스템 틱 안정        현재       │
│   "folder"                          유효 액션   │
└────────────────────────────────────────────────┘
```

### C) 이상안 (P1 + 자동 숨김)

```
탭 없음 (뷰 전체):
┌────────────────────────────────────────────────┐
│                                                │
│                                                │
│                 (본문 렌더링)                   │
│                    전체화면                     │
│                                                │
│                                                │
└────────────────────────────────────────────────┘

본문 탭 1회:
┌────────────────────────────────────────────────┐
│ ← 알한글 — sample.hwpx                          │
├────────────────────────────────────────────────┤
│                                                │
│                 (본문 렌더링)                   │
│                                                │
├────────────────────────────────────────────────┤
│    [📁]       [1 / 66 ▼]            [⬆]       │
│               ↑ 탭하면 점프 시트                  │
└────────────────────────────────────────────────┘
```

---

## 6. 파일별 변경 권장 범위 (P0 한정)

| 파일 | 변경 내용 |
|---|---|
| `rhwp-ios/Sources/ContentView.swift` | 툴바 아이템 3개 교체 (folder / footnote 페이지 / 공유 또는 제거) |
| (신규) `rhwp-ios/Sources/ShareSheet.swift` | `UIViewControllerRepresentable`로 `UIActivityViewController` 래핑 (공유 기능 포함 시) |
| `DocumentViewModel.swift` | `var currentFileURL: URL?` 추가 (공유 시 원본 URL 필요) |

공유 기능은 범위가 커지면 별도 이슈로 쪼개고, **P0 최소본에서는 오른쪽 버튼을 빼버린 2-아이템 레이아웃**도 수용 가능:

```swift
ToolbarItemGroup(placement: .bottomBar) {
    Button { showFilePicker = true } label: { Image(systemName: "folder") }
        .accessibilityLabel("문서 열기")
    Spacer()
    if viewModel.pageCount > 0 {
        Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)")
            .font(.footnote.monospacedDigit())
            .foregroundColor(.secondary)
    }
    Spacer()
    // 일부러 비움 — 균형을 위해 투명 플레이스홀더를 쓰지 말 것 (HIG 권고)
}
```

이 경우 우측 Spacer 이후 아무것도 두지 않으면 중앙 텍스트가 오른쪽으로 밀린다. **진짜 2-아이템이라면 `Spacer()` 하나만 쓰고 좌-중 배치**로 전환하는 편이 시맨틱하다.

---

## 7. 결론 및 권장 액션

1. **이번 태스크(M2 #219)에 반영할 최소 변경**
   - `folder.badge.plus` → `folder`
   - Label → Image + accessibilityLabel
   - 비활성 gearshape **제거** (또는 공유 버튼으로 대체)
   - 페이지 텍스트를 `.footnote.monospacedDigit()`로 상향 + `.accessibilityLabel` 추가
   - "쪽" 제거한 `1 / 66` 포맷

2. **다음 이슈로 분리 권장 (M2 또는 M3)**
   - 페이지 점프 시트
   - 본문 탭 시 뷰어 자동 숨김 (iPhone 한정)
   - iPad NavigationSplitView 전환 (썸네일 사이드바)
   - 공유 기능 (`UIActivityViewController`)
   - 설정 화면 실제 구현 후 gearshape 재도입

3. **Dynamic Island / Safe Area**: 현재 코드는 `.toolbar` 사용으로 자동 대응. 추가 조치 불필요. 다만 iPhone 실기(시뮬레이터 아님)에서 홈 인디케이터와 bottomBar 간격을 육안 검증 필요.

4. **다크 모드 / VoiceOver**: P0 체크리스트에 포함. iOS 시뮬레이터의 "Accessibility Inspector"로 사전 검증 권장.

---

## 부록: 참고한 iOS 네이티브 앱 툴바 패턴

| 앱 | 하단 툴바 구성 | 라벨 | 비고 |
|---|---|---|---|
| Safari | ← → ⬆ 📚 🗂 | 아이콘만 | 뷰어/브라우저 5-아이템 표준 |
| Mail | 🗑 ⚡️ 📁 ↩ ✍ | 아이콘만 | 균등 분배 5-아이템 |
| Files | 최근/둘러보기 탭바 | 아이콘+텍스트 | TabView 패턴, bottomBar 아님 |
| Books (iOS) | 서재/북스토어 탭바 | 아이콘+텍스트 | 뷰어 진입 시 **전체화면**, 탭하면 상단 툴바 |
| Preview (macOS) | — | — | 썸네일 사이드바. iPad 적용 시 참고 가치 |
| Acrobat Reader iOS | 페이지 번호 중앙 + 공유 + 검색 + 북마크 | 아이콘만 | **알한글에 가장 가까운 레퍼런스** |

알한글은 **Acrobat Reader iOS + Books 뷰어 모드**를 혼합한 방향을 권장:
- iPhone: Books형 자동 숨김
- iPad: Acrobat형 항상 노출 + 사이드바
