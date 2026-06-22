//! 문서 내보내기 · 라이프사이클 · 스냅샷 API (WASM 바인딩).
//!
//! HWP/HWPX export, 검증, batch 모드, undo/redo 스냅샷 등
//! `HwpDocument`의 라이프사이클 관련 `#[wasm_bindgen]` 메서드 모음.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 문서를 HWP 바이너리로 내보낸다.
    ///
    /// Document IR을 HWP 5.0 CFB 바이너리로 직렬화하여 반환한다.
    /// HWPX 출처 문서는 `export_hwp_with_adapter` 를 통해 HWPX→HWP IR 매핑 어댑터를
    /// 자동 적용하여 한컴 호환성과 자기 재로드 페이지 보존을 보장한다 (#178).
    /// HWP 출처는 어댑터가 no-op 이므로 기존 동작과 동일.
    #[wasm_bindgen(js_name = exportHwp)]
    pub fn export_hwp(&mut self) -> Result<Vec<u8>, JsValue> {
        self.export_hwp_with_adapter().map_err(|e| e.into())
    }

    /// Document IR을 HWPX(ZIP+XML)로 직렬화하여 반환한다.
    #[wasm_bindgen(js_name = exportHwpx)]
    pub fn export_hwpx(&self) -> Result<Vec<u8>, JsValue> {
        self.export_hwpx_native().map_err(|e| e.into())
    }

    /// 어댑터 적용 + HWP 직렬화 + 자기 재로드 검증을 수행하고 결과를 JSON 으로 반환한다 (#178).
    ///
    /// 반환 JSON:
    /// ```json
    /// {
    ///   "bytesLen": 678912,
    ///   "pageCountBefore": 9,
    ///   "pageCountAfter": 9,
    ///   "recovered": true
    /// }
    /// ```
    ///
    /// 본 함수는 검증 메타데이터만 반환하며 bytes 자체는 별도 호출 (`exportHwp`) 로 받아야 한다.
    /// 검증과 실제 사용을 분리하여 호출자가 결과에 따라 다른 동작을 취할 수 있도록 한다.
    #[wasm_bindgen(js_name = exportHwpVerify)]
    pub fn export_hwp_verify(&mut self) -> Result<String, JsValue> {
        let v = self.serialize_hwp_with_verify().map_err(JsValue::from)?;
        Ok(format!(
            "{{\"bytesLen\":{},\"pageCountBefore\":{},\"pageCountAfter\":{},\"recovered\":{}}}",
            v.bytes_len, v.page_count_before, v.page_count_after, v.recovered
        ))
    }

    /// 원본 파일 형식을 반환한다 ("hwp" 또는 "hwpx").
    #[wasm_bindgen(js_name = getSourceFormat)]
    pub fn get_source_format(&self) -> String {
        match self.core.source_format {
            crate::parser::FileFormat::Hwpx => "hwpx".to_string(),
            _ => "hwp".to_string(),
        }
    }

    /// HWPX 비표준 감지 경고를 JSON 문자열로 반환한다 (#177).
    ///
    /// ## 반환 형식
    ///
    /// ```json
    /// {
    ///   "count": 3,
    ///   "summary": {
    ///     "lineseg 배열이 비어있음": 1,
    ///     "lineseg 가 미계산 상태 (line_height=0)": 2
    ///   },
    ///   "warnings": [
    ///     {
    ///       "section": 0,
    ///       "paragraph": 5,
    ///       "kind": "LinesegArrayEmpty",
    ///       "cell": null
    ///     },
    ///     {
    ///       "section": 0,
    ///       "paragraph": 10,
    ///       "kind": "LinesegUncomputed",
    ///       "cell": {"ctrl": 0, "row": 0, "col": 1, "innerPara": 0}
    ///     }
    ///   ]
    /// }
    /// ```
    #[wasm_bindgen(js_name = getValidationWarnings)]
    pub fn get_validation_warnings(&self) -> String {
        let report = self.core.validation_report();

        // summary 직렬화 (HashMap 순서 안정화를 위해 키 정렬)
        let mut summary_parts: Vec<String> = Vec::new();
        let mut entries: Vec<(String, usize)> = report.summary().into_iter().collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (k, v) in &entries {
            // 경고 메시지는 한국어 고정 문자열이므로 `"` / `\` 만 escape.
            let escaped = k.replace('\\', "\\\\").replace('"', "\\\"");
            summary_parts.push(format!("\"{}\":{}", escaped, v));
        }

        // warnings 직렬화
        let mut warning_parts: Vec<String> = Vec::new();
        for w in &report.warnings {
            let cell_part = match &w.cell_path {
                Some(cp) => format!(
                    r#"{{"ctrl":{},"row":{},"col":{},"innerPara":{}}}"#,
                    cp.table_ctrl_idx, cp.row, cp.col, cp.inner_para_idx,
                ),
                None => "null".to_string(),
            };
            let kind_name = match &w.kind {
                crate::document_core::validation::WarningKind::LinesegArrayEmpty =>
                    "LinesegArrayEmpty",
                crate::document_core::validation::WarningKind::LinesegUncomputed =>
                    "LinesegUncomputed",
                crate::document_core::validation::WarningKind::LinesegTextRunReflow =>
                    "LinesegTextRunReflow",
            };
            warning_parts.push(format!(
                r#"{{"section":{},"paragraph":{},"kind":"{}","cell":{}}}"#,
                w.section_idx,
                w.paragraph_idx,
                kind_name,
                cell_part,
            ));
        }

        format!(
            r#"{{"count":{},"summary":{{{}}},"warnings":[{}]}}"#,
            report.len(),
            summary_parts.join(","),
            warning_parts.join(","),
        )
    }

    /// 사용자 명시 요청에 의한 lineseg 전체 reflow (#177).
    ///
    /// `reflow_zero_height_paragraphs` 의 자동 경로와 달리, "빈 line_segs + text 존재"
    /// 케이스까지 포함해 재계산한다. 반환값은 실제로 reflow 된 문단 개수.
    ///
    /// 호출 이후 렌더 캐시·페이지네이션이 갱신되므로 즉시 렌더링하면 보정된 결과가 보인다.
    #[wasm_bindgen(js_name = reflowLinesegs)]
    pub fn reflow_linesegs(&mut self) -> usize {
        self.core.reflow_linesegs_on_demand()
    }

    /// 배포용(읽기전용) 문서를 편집 가능한 일반 문서로 변환한다.
    ///
    /// 반환값: JSON `{"ok":true,"converted":true}` 또는 `{"ok":true,"converted":false}`
    #[wasm_bindgen(js_name = convertToEditable)]
    pub fn convert_to_editable(&mut self) -> Result<String, JsValue> {
        self.convert_to_editable_native().map_err(|e| e.into())
    }

    /// Batch 모드를 시작한다. 이후 Command 호출 시 paginate()를 건너뛴다.
    #[wasm_bindgen(js_name = beginBatch)]
    pub fn begin_batch(&mut self) -> Result<String, JsValue> {
        self.begin_batch_native().map_err(|e| e.into())
    }

    /// Batch 모드를 종료하고 누적된 이벤트를 반환한다.
    #[wasm_bindgen(js_name = endBatch)]
    pub fn end_batch(&mut self) -> Result<String, JsValue> {
        self.end_batch_native().map_err(|e| e.into())
    }

    /// 현재 이벤트 로그를 JSON으로 반환한다.
    #[wasm_bindgen(js_name = getEventLog)]
    pub fn get_event_log(&self) -> String {
        self.serialize_event_log()
    }

    // ─── Undo/Redo 스냅샷 API ──────────────────────────

    /// Document 스냅샷을 저장하고 ID를 반환한다.
    #[wasm_bindgen(js_name = saveSnapshot)]
    pub fn save_snapshot(&mut self) -> u32 {
        self.save_snapshot_native()
    }

    /// 지정 ID의 스냅샷으로 Document를 복원한다.
    #[wasm_bindgen(js_name = restoreSnapshot)]
    pub fn restore_snapshot(&mut self, id: u32) -> Result<String, JsValue> {
        self.restore_snapshot_native(id).map_err(|e| e.into())
    }

    /// 지정 ID의 스냅샷을 제거하여 메모리를 해제한다.
    #[wasm_bindgen(js_name = discardSnapshot)]
    pub fn discard_snapshot(&mut self, id: u32) {
        self.discard_snapshot_native(id)
    }
}
