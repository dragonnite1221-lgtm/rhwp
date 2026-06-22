//! 스타일(Style) 생성·수정·삭제 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 스타일의 메타 정보(이름/영문이름/nextStyleId)를 수정한다.
    ///
    /// json: {"name":"...", "englishName":"...", "nextStyleId":0}
    #[wasm_bindgen(js_name = updateStyle)]
    pub fn update_style(&mut self, style_id: u32, json: &str) -> bool {
        use crate::document_core::helpers::json_i32;
        let styles = &mut self.core.document.doc_info.styles;
        let style = match styles.get_mut(style_id as usize) {
            Some(s) => s,
            None => return false,
        };
        // 이름 파싱
        if let Some(name) = crate::document_core::helpers::json_str(json, "name") {
            style.local_name = name;
        }
        if let Some(en) = crate::document_core::helpers::json_str(json, "englishName") {
            style.english_name = en;
        }
        if let Some(v) = json_i32(json, "nextStyleId") {
            style.next_style_id = v as u8;
        }
        // raw_data 무효화 (수정됨)
        style.raw_data = None;
        true
    }

    /// 스타일의 CharShape/ParaShape를 수정한다.
    ///
    /// charMods/paraMods는 기존 parse_char_shape_mods/parse_para_shape_mods와 동일한 JSON 형식
    #[wasm_bindgen(js_name = updateStyleShapes)]
    pub fn update_style_shapes(
        &mut self,
        style_id: u32,
        char_mods_json: &str,
        para_mods_json: &str,
    ) -> bool {
        let styles = &self.core.document.doc_info.styles;
        let style = match styles.get(style_id as usize) {
            Some(s) => s.clone(),
            None => return false,
        };

        // CharShape 수정
        if !char_mods_json.is_empty() && char_mods_json != "{}" {
            let char_mods = crate::document_core::helpers::parse_char_shape_mods(char_mods_json);
            if let Some(cs) = self
                .core
                .document
                .doc_info
                .char_shapes
                .get(style.char_shape_id as usize)
            {
                let new_cs = char_mods.apply_to(cs);
                // 새 CharShape를 추가하고 스타일에 연결
                self.core.document.doc_info.char_shapes.push(new_cs);
                let new_id = (self.core.document.doc_info.char_shapes.len() - 1) as u16;
                self.core.document.doc_info.styles[style_id as usize].char_shape_id = new_id;
            }
        }

        // ParaShape 수정
        if !para_mods_json.is_empty() && para_mods_json != "{}" {
            let para_mods = crate::document_core::helpers::parse_para_shape_mods(para_mods_json);
            if let Some(ps) = self
                .core
                .document
                .doc_info
                .para_shapes
                .get(style.para_shape_id as usize)
            {
                let new_ps = para_mods.apply_to(ps);
                self.core.document.doc_info.para_shapes.push(new_ps);
                let new_id = (self.core.document.doc_info.para_shapes.len() - 1) as u16;
                self.core.document.doc_info.styles[style_id as usize].para_shape_id = new_id;
            }
        }

        // raw_data 무효화
        self.core.document.doc_info.styles[style_id as usize].raw_data = None;

        // ── 스타일 변경을 해당 스타일을 사용하는 모든 문단에 전파 ──
        let updated_style = self.core.document.doc_info.styles[style_id as usize].clone();
        let sid = style_id as u8;
        let new_csid = updated_style.char_shape_id as u32;
        let new_psid = updated_style.para_shape_id;
        for section in &mut self.core.document.sections {
            for para in &mut section.paragraphs {
                if para.style_id == sid {
                    para.para_shape_id = new_psid;
                    para.char_shapes.clear();
                    para.char_shapes
                        .push(crate::model::paragraph::CharShapeRef {
                            start_pos: 0,
                            char_shape_id: new_csid,
                        });
                }
                // 셀 내 문단도 전파
                for ctrl in &mut para.controls {
                    if let crate::model::control::Control::Table(ref mut table) = *ctrl {
                        for cell in &mut table.cells {
                            for cpara in &mut cell.paragraphs {
                                if cpara.style_id == sid {
                                    cpara.para_shape_id = new_psid;
                                    cpara.char_shapes.clear();
                                    cpara
                                        .char_shapes
                                        .push(crate::model::paragraph::CharShapeRef {
                                            start_pos: 0,
                                            char_shape_id: new_csid,
                                        });
                                }
                            }
                        }
                    }
                }
            }
            section.raw_stream = None;
        }

        // 스타일 캐시 무효화 + 전체 리빌드
        let num_sections = self.core.document.sections.len();
        for sec_idx in 0..num_sections {
            self.core.rebuild_section(sec_idx);
        }
        true
    }

}
