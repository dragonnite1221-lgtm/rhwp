//! 스타일(Style) 생성·삭제 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 새 스타일을 생성한다.
    ///
    /// json: {"name":"...", "englishName":"...", "type":0, "nextStyleId":0}
    /// 반환값: 새 스타일 ID (0-based)
    #[wasm_bindgen(js_name = createStyle)]
    pub fn create_style(&mut self, json: &str) -> i32 {
        use crate::document_core::helpers::{json_i32, json_str};
        use crate::model::style::Style;

        let name = json_str(json, "name").unwrap_or_default();
        let english_name = json_str(json, "englishName").unwrap_or_default();
        let style_type = json_i32(json, "type").unwrap_or(0) as u8;
        let next_style_id = json_i32(json, "nextStyleId").unwrap_or(0) as u8;

        // 기본 "바탕글" 스타일(ID 0)의 CharShape/ParaShape를 복사
        let base_style = self.core.document.doc_info.styles.first();
        let (char_shape_id, para_shape_id) = match base_style {
            Some(s) => (s.char_shape_id, s.para_shape_id),
            None => (0, 0),
        };

        let new_style = Style {
            raw_data: None,
            local_name: name,
            english_name,
            style_type,
            next_style_id,
            para_shape_id,
            char_shape_id,
        };
        self.core.document.doc_info.styles.push(new_style);
        let new_id = (self.core.document.doc_info.styles.len() - 1) as i32;
        // 스타일 캐시 갱신
        self.core.styles = crate::renderer::style_resolver::resolve_styles(
            &self.core.document.doc_info,
            self.core.dpi,
        );
        new_id
    }

    /// 스타일을 삭제한다.
    ///
    /// 바탕글(ID 0)은 삭제할 수 없다.
    /// 삭제된 스타일을 사용 중인 문단은 바탕글(ID 0)로 변경된다.
    #[wasm_bindgen(js_name = deleteStyle)]
    pub fn delete_style(&mut self, style_id: u32) -> bool {
        if style_id == 0 {
            return false; // 바탕글은 삭제 불가
        }
        let styles = &self.core.document.doc_info.styles;
        if style_id as usize >= styles.len() {
            return false;
        }
        let sid = style_id as u8;
        // 해당 스타일을 사용 중인 문단을 바탕글(0)로 변경
        for section in &mut self.core.document.sections {
            for para in &mut section.paragraphs {
                if para.style_id == sid {
                    para.style_id = 0;
                }
            }
        }
        // 스타일 삭제 (인덱스 기반이므로 뒤의 ID가 변경됨에 주의)
        self.core.document.doc_info.styles.remove(style_id as usize);
        // 삭제된 ID보다 큰 style_id를 가진 문단들 보정
        for section in &mut self.core.document.sections {
            for para in &mut section.paragraphs {
                if para.style_id > sid {
                    para.style_id -= 1;
                }
            }
        }
        // next_style_id 보정
        for s in &mut self.core.document.doc_info.styles {
            if s.next_style_id == sid {
                s.next_style_id = 0;
            } else if s.next_style_id > sid {
                s.next_style_id -= 1;
            }
        }
        // 스타일 캐시 갱신
        self.core.styles = crate::renderer::style_resolver::resolve_styles(
            &self.core.document.doc_info,
            self.core.dpi,
        );
        true
    }

}
