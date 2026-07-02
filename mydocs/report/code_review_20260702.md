> 리뷰 일자: 2026-07-02 · 대상: 5a3fcce 시점 스냅샷 · 읽기 전용 정적 분석 (코드 미수정)

# rhwp 종합 코드리뷰

## 리포 개요

Rust로 작성된 HWP 파일 뷰어/에디터. HWPX(ZIP+XML)/HWP5(OLE 복합)/HWP3(고전 바이너리) 3개 파서가 공통 `Document` IR(`src/model/document.rs`)로 변환하고, 렌더러(`src/renderer/`)가 SVG/Canvas/HTML/PDF로 출력한다. WASM(`src/wasm_api/`, 31개 모듈)으로 브라우저에서 구동되며, `rhwp-studio`(TS 프론트엔드) + Puppeteer e2e, 브라우저 확장(Chrome/Firefox/Safari), VSCode 확장까지 포함한 대형 프로젝트다. `src/` 하위 433개 .rs 파일.

## 종합 평가: **B+ (양호, 보안 방어의 포맷 간 비대칭이 최대 약점)**

공학적 체계(테스트 1,386개, issue 번호 앵커링된 회귀 테스트, golden SVG 스냅샷, CI+clippy -D warnings, CodeQL, 200줄 게이트, 3,000+ 문서)는 동급 오픈소스에서 보기 드물게 우수하다. 파서 에러 전파도 Result 기반으로 잘 설계되어 있다. 그러나 **HWPX에는 압축 폭탄 방어가 테스트까지 갖춰져 있는 반면, HWP5/HWP3 경로에는 동일한 방어가 없고**, HWP3 파서에 u32 길이 필드 기반 무제한 선할당이 다수 남아 있다. 신뢰할 수 없는 입력을 브라우저에서 여는 제품 특성상 이것이 1순위 개선 대상이다.

---

## 정량 지표 (파서 안전성)

| 지표 | 값 | 비고 |
|---|---|---|
| `unwrap()` — src 전체 | 1,421 | 아래 분해 참조 |
| `unwrap()` — src/parser/ | 91 | **비테스트 코드에서는 거의 0** (doc_info.rs 12개 등 대부분 `#[cfg(test)]` 내부, 유일한 비테스트 unwrap인 hwp3/mod.rs:1648은 가드됨) |
| `unwrap()` — src/wasm_api/ | 701 | **전부 테스트** (tests.rs 15,759줄 통합 테스트 코퍼스) |
| `unwrap()` — 비테스트 파일 전체 | 417 | 대부분 document_core 편집 커맨드/serializer (text_editing.rs 49개 최다) |
| `unwrap_or*` (방어적 기본값) — src/parser/ | 517 | 파서의 지배적 스타일 |
| `panic!` — src 전체 / 파서 비테스트 | 153 / **0** | 파서 panic은 전부 테스트 코드 |
| `unsafe` | 5곳 | `src/wmf/parser/constants/enums/character_set.rs` 정적 테이블 참조에 국한 |
| ` as usize` 캐스트 — src/parser/ 비테스트 | 95 | 다수는 무해, 일부 길이 필드가 문제 (아래) |
| 200줄 게이트 | **통과** (신규 위반 0) | 베이스라인 동결 149파일, 최대 3,926줄 |

---

## 발견 사항

### High

**H1. HWP5 압축 해제 폭탄 무방비 — `src/parser/cfb_reader.rs:550-567`**
```rust
pub fn decompress_stream(data: &[u8]) -> Result<Vec<u8>, CfbError> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decompressed = Vec::new();
    match decoder.read_to_end(&mut decompressed) { ... }
```
BodyText/DocInfo 등 모든 압축 스트림에 상한 없는 `read_to_end`. 수 KB deflate 입력이 수 GB로 팽창 가능 → 브라우저 WASM(최대 4GB 선형 메모리)에서는 즉시 abort DoS. **HWPX 쪽(`hwpx/reader.rs`의 `read_limited`, `MAX_XML_SIZE=32MB`)에는 동일 공격에 대한 방어와 zip-bomb 회귀 테스트까지 있는데 HWP5에는 없다.** 실패 시 zlib 재시도 구조라 실패한 1차 시도에서도 이미 대량 할당이 발생한다.
→ **권고**: `read_limited`와 동일한 `Read::take(cap)` 상한을 `decompress_stream`에 적용 (원본 크기 대비 배율 상한도 가능).

**H2. HWP3 본문 압축 해제도 동일 무방비 — `src/parser/hwp3/mod.rs:1986-1988`**
```rust
let mut decoder = DeflateDecoder::new(remaining_data);
decoder.read_to_end(&mut decompressed_data).map_err(...)?;
```
H1과 동일 클래스. HWP3 압축 문서로 같은 공격 가능.

**H3. HWP3 u32 길이 필드 기반 무검증 선할당 (할당 폭탄) — 다수 지점**
- `src/parser/hwp3/records.rs:412-413` (`Hwp3AdditionalInfoBlock`):
  ```rust
  let length = reader.read_u32::<LittleEndian>()?;
  let mut data = vec![0u8; length as usize];   // 최대 4GB를 read 전에 선할당
  reader.read_exact(&mut data)?;
  ```
- `src/parser/hwp3/drawing.rs:343-345` (`Hwp3DrawingPolygon`): `point_count: u32` → `Vec::with_capacity(point_count as usize)` — `[i32;2]` 8바이트 × 최대 4G = **32GB 선할당 시도** (읽기 실패 이전에 abort).
- `src/parser/hwp3/drawing.rs:372` (`info2_len: u32` → `vec![0u8; info2_len]`), 같은 파일 :543-546, `src/parser/hwp3/ole.rs:38` (`vec![0u8; (total_length-4) as usize]`) 동일 패턴.

`read_exact`가 결국 실패하더라도 **할당이 읽기보다 먼저** 일어나므로 손상/악성 파일 한 개로 OOM abort. HWP3 쪽 상당수 길이 필드가 u16(예: `Hwp3ParaInfo.char_count/line_count`, `paragraph.rs:46-47` — 64K 상한으로 안전)인 것과 대조적.
→ **권고**: 남은 스트림 크기(또는 합리적 상한)와 대조 후 할당하거나, `Read::take` + 점진 할당으로 전환.

**H4. 레코드 헤더 확장 크기의 wasm32 오버플로 → 경계 검사 우회 — `src/parser/record.rs:53-69`**
```rust
if size == 0xFFF { size = cursor.read_u32...; }   // size: 임의 u32
let pos = cursor.position() as usize;
if pos + size as usize > data.len() { return Err(...); }  // wasm32 release에서 wrap 가능
let mut record_data = vec![0u8; size as usize];
```
이 프로젝트의 1차 타깃인 wasm32에서는 `usize`가 32비트라 `pos + size`가 release 빌드에서 감싸 넘칠 수 있다(예: `size` 근처 `0xFFFF_FFFF`, `pos` 작을 때). 감싸면 경계 검사가 통과되어 `vec![0u8; ~4GB]` 선할당으로 이어진다. `pos.checked_add(size).map_or(err)` 형태의 checked 연산이 안전. HWP5 레코드 파싱의 최상위 경로라 영향 큼.

### Medium

**M1. `read_utf16_string` 길이 오버플로 — `src/parser/byte_reader.rs:108-110`**
```rust
pub fn read_utf16_string(&mut self, char_count: usize) -> io::Result<String> {
    let byte_count = char_count * 2;   // char_count가 크면 곱셈 오버플로(디버그 패닉/릴리즈 wrap)
    let bytes = self.read_bytes(byte_count)?;
```
`read_bytes`가 `vec![0u8; len]` 선할당(byte_reader.rs:76)이라 H3와 결합. `read_hwp_string`은 호출자가 u16 길이라 안전하나, `read_utf16_string`을 직접 호출하는 경로는 상한 검증 필요. `char_count * 2`를 `checked_mul` + `remaining()` 대조로.

**M2. HWP3 `char_count` 기반 `vec![0; char_count]` 선할당 — `src/parser/hwp3/mod.rs:307`**
`char_count`가 u16이라 최대 128KB로 제한적이지만(H3보다 경미), 문단 수 × 반복이라 누적 가능. 상한은 u16이므로 실제 위험은 낮음 — 기록 차원.

**M3. `extract_dib_as_bmp` 배열 인덱싱 — `src/parser/ole_container.rs:180-206`**
루프 상한이 `scan_limit.saturating_sub(40)`인데 내부에서 `data[i+15]`~`data[i+35]`까지 접근한다 — 상한 여유(40)가 최대 인덱스(35)를 우연히 커버하는 것에 의존. 명시적 `i+35 < data.len()` 가드가 견고. 지금은 우발적 안전.

**M4. 편집 경로의 전량 재페이지네이션 — `src/document_core/queries/rendering.rs:1028` + 다수 커맨드**
`commands/document.rs`만 6곳에서 `self.paginate()` 직접 호출. `paginate`는 dirty 섹션 기반 증분 처리를 갖췄으나(양호), 편집 커맨드가 `paginate_if_needed`(batch 인식) 대신 `paginate`를 직접 부르는 곳이 섞여 있어 batch 모드 우회. 대용량 문서 연속 편집 시 O(문서) 재계산 반복 가능. 호출 규약 통일 권고.

**M5. 파서 문자열 진단이 사용자 노출 — 다수**
`BodyTextError::RecordError(e.to_string())` 등 하위 raw 에러 문자열을 그대로 감싼다. Issue #265에서 `HwpError`가 Debug 대신 Display를 쓰도록 이미 고쳤으나(error.rs 테스트로 앵커), 파서 내부 `format!("{e}")` 전파 경로에 파일 경로/내부 오프셋이 새어나올 여지. (심각도 낮음)

### Low

**L1. `decompress_stream` 1차 실패 시 침묵 — cfb_reader.rs:555** `Err(_) => {}`로 raw deflate 실패를 삼키고 zlib 재시도. 정상 폴백 설계지만 두 번째 대량 할당 유발 가능(H1 연계).

**L2. `main.rs` 3,369줄, `object_ops.rs` 3,926줄** 등 베이스라인 최상위 파일. 게이트는 통과(동결)하나 CLI 진입점/객체 커맨드가 거대 단일 파일. 문서화된 burndown 계획에 따라 우선 분할 대상.

**L3. `as` 캐스팅 다수(파서 95곳)** 대부분 무해하나 H3/H4/M1과 겹치는 길이·오프셋 캐스팅은 checked 연산 원칙 확립 권고.

**L4. `flate2` 폴백이 매번 새 Decoder+Vec 생성** — 미세 효율. 대용량 스트림에서 1차 실패 시 버퍼 2회 할당.

**L5. `Preview`/`bin_data_content`가 `Document`에 `Vec<u8>` 원본 보존** — 라운드트립 목적상 합리적이나, 대형 임베디드 이미지 다수 문서에서 IR 메모리 = 파일 크기 배수. 스트리밍/지연 로딩 여지.

---

## 확인했으나 문제없음 (긍정 사항)

- **`src/parser/record.rs` 경계 검사**: 확장 크기 처리 후 `pos + size > data.len()` 검사로 `UnexpectedEof` 반환(H4의 wasm32 오버플로만 제외하면 견고). `remaining < 4` 조기 종료도 적절.
- **HWPX zip-bomb 방어 모범**: `read_limited` + `Read::take(cap)` + 전용 회귀 테스트(`test_zip_bomb_xml_entry_rejected`)까지 완비. **이 패턴을 HWP5/HWP3로 이식하면 H1~H3 대부분 해소** — 이미 리포 내에 정답이 있음.
- **ByteReader 설계**: `saturating_sub` 기반 `remaining()`, `skip`의 범위 검사, UTF-16 디코딩 실패를 `InvalidData`로 전파 — 방어적.
- **body_text 파서의 서로게이트 페어/컨트롤 문자 처리**: `pos + 3 < data.len()` 가드로 페어 읽기 안전, `char::from_u32` 실패를 조용히 스킵(패닉 없음).
- **에러 타입 설계**: `ParseError`가 포맷별 하위 에러를 열거형으로 통합, `From` 구현으로 `?` 전파. panic이 파서 비테스트 경로에 0건.
- **HWP3 격리 규칙 준수**: 공통 모듈(`renderer/`, `document_core/`)의 HWP3 언급은 전부 주석·테스트·소스포맷 경계 판정뿐. HWP3 전용 파싱 분기가 공통 코드에 유입되지 않음 — CLAUDE.md 규칙 실제 준수.
- **AES/LCG 복호화**(`crypto.rs`): 자체 구현이나 페이로드 크기 검증(`data.len() < 256`) 선행, 에러 열거형 완비.
- **증분 페이지네이션**: dirty 섹션·문단 비트맵 기반 재계산, 페이지 트리 캐시 무효화 — 성능 설계 존재.
- **200줄 게이트 실효성**: `--write-baseline` 남용 방지 로직(stale/growth 차단), 데이터 테이블 제외 규칙, CI+pre-push 이중 강제.

---

## 관점별 평가표

| 관점 | 점수 | 근거 |
|---|---|---|
| 보안성 | **6/10** | HWPX는 모범적이나 HWP5/HWP3 압축·길이 필드에 폭탄/OOM 방어 부재(H1~H4). unsafe 최소·panic 파서 0은 우수 |
| 안정성 | **8/10** | Result 전파 일관·손상 파일 조기 종료·서로게이트 가드. 파서 비테스트 panic 0. wasm32 오버플로만 흠 |
| 효율성 | **7/10** | 증분 페이지네이션 설계 우수. 다만 clone 546건, 편집 시 전량 재페이지네이션 경로 혼재, 임베디드 이미지 원본 메모리 상주 |
| 보수 용이성 | **8/10** | 200줄 게이트 통과·모듈 분할 활발(wasm_api 3라운드 burndown). 단 3,900줄급 잔존 파일 존재 |
| 확장성 | **9/10** | 3포맷 → 단일 IR, 컨트롤 `match ctrl_id` 디스패치·`Unknown` 폴백, RawRecord 라운드트립 보존. 새 레코드/컨트롤 추가 용이 |
| 체계성 | **9/10** | 테스트 1,386개+통합 30파일+golden SVG, CI(clippy -D warnings/CodeQL/wasm), issue 앵커 회귀, 3,000+ 문서, PDF 권위등급 체계 |

---

## 개선 우선순위 Top 5

1. **HWP5/HWP3 압축 해제에 상한 적용 (H1, H2)** — `hwpx/reader.rs`의 `read_limited` 패턴을 `decompress_stream`과 `hwp3/mod.rs:1986`에 재사용. 리포에 이미 정답 코드·테스트 존재하므로 저비용·고효과.
2. **HWP3 u32 길이 필드 선할당 방어 (H3)** — `records.rs`/`drawing.rs`/`ole.rs`의 `vec![0u8; len]`·`with_capacity(count)` 앞에 `remaining()`/상한 대조 추가. 손상 파일 OOM 즉시 차단.
3. **record.rs 확장 크기 checked 연산 (H4)** — wasm32 타깃의 `pos + size` 오버플로를 `checked_add`로. 최상위 파싱 경로라 파급 큼.
4. **byte_reader `char_count * 2` checked_mul (M1)** — 문자열 읽기 오버플로 + 선할당 결합 차단.
5. **편집 경로 재페이지네이션 규약 통일 (M4)** — 커맨드가 `paginate` 직접 호출 대신 `paginate_if_needed` 경유하도록 정리, 대용량 문서 편집 성능 확보.

전반적으로 파싱 로직의 에러 전파·격리·테스트 체계는 성숙 단계이며, 남은 리스크는 대부분 "신뢰 불가 입력에 대한 자원 상한(폭탄/OOM) 방어의 포맷 간 비대칭"이라는 단일 테마로 수렴한다. HWPX 경로가 이미 정답을 보유하고 있어 이식 비용이 낮은 것이 이 리포의 강점이다.
