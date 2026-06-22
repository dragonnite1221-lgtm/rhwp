//! 누름틀(ClickHere) 필드 속성 조회/수정 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::helpers::json_escape;

#[wasm_bindgen]
impl HwpDocument {
    // ─── 누름틀 속성 조회/수정 API ──────────────────────────────

    /// 누름틀 필드의 속성을 조회한다.
    ///
    /// 반환: JSON `{"ok":true,"guide":"안내문","memo":"메모","name":"이름","editable":true}`
    #[wasm_bindgen(js_name = getClickHereProps)]
    pub fn get_click_here_props(&self, field_id: u32) -> String {
        use crate::model::control::{Control, FieldType};
        // 문서 전체에서 fieldId로 필드 찾기
        for sec in &self.document.sections {
            for para in &sec.paragraphs {
                for ctrl in &para.controls {
                    if let Control::Field(f) = ctrl {
                        if f.field_id == field_id && f.field_type == FieldType::ClickHere {
                            return self.format_click_here_props(f);
                        }
                    }
                }
                // 표/글상자 내부도 탐색
                for ctrl in &para.controls {
                    let paras: Vec<&crate::model::paragraph::Paragraph> = match ctrl {
                        Control::Table(t) => t.cells.iter().flat_map(|c| &c.paragraphs).collect(),
                        Control::Shape(s) => s
                            .drawing()
                            .and_then(|d| d.text_box.as_ref())
                            .map(|tb| tb.paragraphs.iter().collect())
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    for p in paras {
                        for c in &p.controls {
                            if let Control::Field(f) = c {
                                if f.field_id == field_id && f.field_type == FieldType::ClickHere {
                                    return self.format_click_here_props(f);
                                }
                            }
                        }
                    }
                }
            }
        }
        r#"{"ok":false}"#.to_string()
    }

    /// ClickHere 필드 속성을 JSON으로 포맷한다.
    fn format_click_here_props(&self, f: &crate::model::control::Field) -> String {
        let guide = f.guide_text().unwrap_or("");
        let memo = f.memo_text().unwrap_or("");
        // 필드 이름: ctrl_data_name → command Name: 키 순서
        let name = f
            .ctrl_data_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| f.extract_wstring_value("Name:"))
            .unwrap_or("");
        let editable = f.is_editable_in_form();
        format!(
            "{{\"ok\":true,\"guide\":\"{}\",\"memo\":\"{}\",\"name\":\"{}\",\"editable\":{}}}",
            json_escape(guide),
            json_escape(memo),
            json_escape(name),
            editable,
        )
    }

}
