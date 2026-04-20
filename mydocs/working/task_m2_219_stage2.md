# Task #219 — 2단계 완료보고서

## Info.plist UTType / Document Types 등록 ✅

### 작업 내용

`rhwp-ios/Sources/Info.plist`에 HWP/HWPX 파일 타입 선언 + 문서 연관 등록.

### UTImportedTypeDeclarations

커스텀 UTI 2개 선언:

| UTI | 확장자 | MIME 타입 | 상위 UTI |
|-----|--------|-----------|----------|
| `com.hancom.hwp` | `.hwp` | `application/x-hwp`, `application/haansofthwp` | `public.data`, `public.content` |
| `com.hancom.hwpx` | `.hwpx` | `application/hwp+zip` | `public.data`, `public.content`, `public.zip-archive` |

### CFBundleDocumentTypes

앱이 열 수 있는 문서 타입 등록:

```xml
<dict>
    <key>CFBundleTypeName</key>
    <string>한글 문서</string>
    <key>CFBundleTypeRole</key>
    <string>Viewer</string>
    <key>LSHandlerRank</key>
    <string>Alternate</string>   <!-- Owner가 한컴오피스이므로 Alternate -->
    <key>LSItemContentTypes</key>
    <array>
        <string>com.hancom.hwp</string>
        <string>com.hancom.hwpx</string>
    </array>
</dict>
```

- `LSHandlerRank: Alternate` — 기본 앱이 아닌 대체 뷰어로 선언 (한컴오피스가 Owner)
- 파일앱에서 HWP/HWPX 롱탭 → "알한글로 열기" 옵션 노출
- 이메일 첨부 HWP → 공유 시트에 앱 아이콘 표시

### 검증 결과

- Info.plist 구문: ✅ `plutil -lint` OK
- Xcode 빌드: ✅ BUILD SUCCEEDED
- 실제 파일앱에서 "알한글로 열기" 노출 여부는 3단계 실기기 검증 시 확인

### 미구현 (후속 단계)

| 항목 | 단계 |
|------|------|
| 에러 UX 개선 | 3단계 |
| 시뮬레이터/실기기 파일 열기 검증 | 3단계 |
