# Task #219 — 구현계획서

## iPad 뷰어: 로컬 파일 선택 로딩 기능 완성

### 설계 개요

**NavigationStack + 하단 툴바 구조로 전환**

```swift
NavigationStack {
    DocumentView(viewModel: viewModel)
        .toolbar {
            ToolbarItemGroup(placement: .bottomBar) {
                Button { /* 파일 열기 */ } label: { Label("열기", systemImage: "folder.badge.plus") }
                Spacer()
                Text("\(current)/\(total)쪽")
                Spacer()
                Button { /* 설정 */ } label: { Image(systemName: "gearshape") }  // 후속
            }
        }
        .sheet(isPresented: $showFilePicker) {
            DocumentPickerView { data, filename in ... }
        }
}
```

상단 헤더바는 `DocumentView` 내부에 **정보 표시만** 유지 (파일명).

### 구현 단계 (3단계)

---

#### 1단계: NavigationStack + 하단 툴바 구조 전환

**1-1. ContentView**

```swift
NavigationStack {
    DocumentView(viewModel: viewModel)
        .navigationBarHidden(true)  // 상단 네비게이션 바 숨김 (DocumentView 자체 헤더 사용)
        .toolbar {
            ToolbarItemGroup(placement: .bottomBar) {
                Button(action: { showFilePicker = true }) {
                    Label("열기", systemImage: "folder.badge.plus")
                }
                Spacer()
                if viewModel.pageCount > 0 {
                    Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)쪽")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                Spacer()
                // 설정 버튼 — 향후 #100에서 구현
            }
        }
        .sheet(isPresented: $showFilePicker) { DocumentPickerView { ... } }
}
```

**1-2. DocumentView 헤더바 축소**

페이지 번호/크기 표시는 **하단 툴바로 이전**하므로 DocumentView 헤더바는 파일명만 유지:

```swift
// 기존: "알한글 — sample.hwpx  |  1/66쪽 (793×1122pt)"
// 변경: "알한글 — sample.hwpx"
```

**검증**: iPad Simulator + iPhone 실기기에서 하단 툴바 표시 확인, 파일 열기 버튼 탭 → 파일 선택 시트 표시 확인

---

#### 2단계: Info.plist UTType / Document Types 등록

**2-1. UTImportedTypeDeclarations** — HWP/HWPX 커스텀 UTI 선언

```xml
<key>UTImportedTypeDeclarations</key>
<array>
    <dict>
        <key>UTTypeIdentifier</key>
        <string>com.hancom.hwp</string>
        <key>UTTypeDescription</key>
        <string>한글(HWP) 문서</string>
        <key>UTTypeConformsTo</key>
        <array><string>public.data</string></array>
        <key>UTTypeTagSpecification</key>
        <dict>
            <key>public.filename-extension</key>
            <array><string>hwp</string></array>
        </dict>
    </dict>
    <dict>
        <key>UTTypeIdentifier</key>
        <string>com.hancom.hwpx</string>
        <!-- ... hwpx 동일 구조 -->
    </dict>
</array>
```

**2-2. CFBundleDocumentTypes** — 앱이 열 수 있는 문서 타입 등록

```xml
<key>CFBundleDocumentTypes</key>
<array>
    <dict>
        <key>CFBundleTypeName</key>
        <string>한글 문서</string>
        <key>LSItemContentTypes</key>
        <array>
            <string>com.hancom.hwp</string>
            <string>com.hancom.hwpx</string>
        </array>
        <key>CFBundleTypeRole</key>
        <string>Viewer</string>
    </dict>
</array>
```

- 파일앱에서 HWP/HWPX 파일 롱탭 → "알한글로 열기" 노출
- 이메일/메신저 첨부 파일 열기 시 앱 공유 시트에 표시

**검증**: 파일앱 또는 이메일에서 HWP/HWPX 파일 → "알한글" 선택 가능

---

#### 3단계: 통합 + 실기기 검증 + 에러 UX 개선

**3-1. 에러 메시지 개선**

파일 로드 실패 시:
- HWP 파서 실패: "이 파일은 HWP/HWPX 형식이 아니거나 손상되었습니다."
- 데이터 읽기 실패: "파일에 접근할 수 없습니다."
- 현재는 `RhwpError.parseFailure`만 표시되므로 UX 개선

**3-2. 검증**

- iPad Simulator: 번들 샘플 로드 + 파일 선택(시뮬레이터 파일 앱) + 에러 파일 선택
- iPhone 12 Pro 실기기:
  - 하단 툴바 표시 (Dynamic Island 간섭 없음)
  - 파일앱에서 HWP 파일 선택 → 앱에서 정상 렌더링
  - 이메일 첨부 HWP → "알한글로 열기" 표시 확인 (Document Types 등록 후)
- `cargo test` 회귀 없음

---

### 파일 변경 목록

| 파일 | 변경 | 단계 |
|------|------|------|
| `rhwp-ios/Sources/ContentView.swift` | NavigationStack + 하단 툴바 | 1 |
| `rhwp-ios/Sources/DocumentView.swift` | 헤더바 단순화 (파일명만) | 1 |
| `rhwp-ios/Sources/Info.plist` | UTImportedTypeDeclarations + CFBundleDocumentTypes | 2 |
| `rhwp-ios/Sources/DocumentViewModel.swift` | 에러 메시지 구체화 | 3 |
| `rhwp-ios/Sources/RhwpDocument.swift` | RhwpError 케이스 추가 (선택적) | 3 |

### M2 범위에서 제외 (후속 이관)

| 항목 | 이관 |
|------|------|
| 최근 파일 목록 | #100 |
| 빈 상태 안내 화면 디자인 | #100 |
| 설정 화면 | 후속 이슈 |
| Scene Delegate URL 처리 | 후속 (Open In... 플로우 완성 시) |
