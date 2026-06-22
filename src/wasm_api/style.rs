//! 스타일(Style) 목록·상세·생성·수정·삭제 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 문서에 정의된 스타일 목록을 조회한다.
    ///
    /// 반환값: JSON 배열 [{ id, name, englishName, type, paraShapeId, charShapeId }, ...]
    #[wasm_bindgen(js_name = getStyleList)]
    pub fn get_style_list(&self) -> String {
        let styles = &self.core.document.doc_info.styles;
        let mut items = Vec::new();
        for (i, s) in styles.iter().enumerate() {
            items.push(format!(
                "{{\"id\":{},\"name\":\"{}\",\"englishName\":\"{}\",\"type\":{},\"nextStyleId\":{},\"paraShapeId\":{},\"charShapeId\":{}}}",
                i,
                s.local_name.replace('"', "\\\""),
                s.english_name.replace('"', "\\\""),
                s.style_type,
                s.next_style_id,
                s.para_shape_id,
                s.char_shape_id
            ));
        }
        format!("[{}]", items.join(","))
    }

    /// 특정 스타일의 CharShape/ParaShape 속성을 상세 조회한다.
    ///
    /// 반환값: JSON { charProps: {...}, paraProps: {...} }
    #[wasm_bindgen(js_name = getStyleDetail)]
    pub fn get_style_detail(&self, style_id: u32) -> String {
        let styles = &self.core.document.doc_info.styles;
        let style = match styles.get(style_id as usize) {
            Some(s) => s,
            None => return "{}".to_string(),
        };
        let char_json = self
            .core
            .build_char_properties_json_by_id(style.char_shape_id);

        // 스타일의 기본 ParaShape에 번호 정보가 없으면,
        // 이 스타일을 사용하는 실제 문단의 ParaShape에서 조회
        let effective_psid =
            self.find_effective_para_shape_for_style(style_id, style.para_shape_id);
        let para_json = self.core.build_para_properties_json(effective_psid, 0);
        format!(
            "{{\"charProps\":{},\"paraProps\":{}}}",
            char_json, para_json
        )
    }

    /// 스타일의 실효 ParaShape ID를 찾는다.
    /// 스타일 정의의 ParaShape에 번호 정보가 없으면, 이 스타일을 사용하는 문단에서 조회한다.
    fn find_effective_para_shape_for_style(&self, style_id: u32, base_psid: u16) -> u16 {
        use crate::model::style::HeadType;
        // 기본 ParaShape에 이미 번호 정보가 있으면 그대로 사용
        if let Some(ps) = self
            .core
            .document
            .doc_info
            .para_shapes
            .get(base_psid as usize)
        {
            if ps.head_type != HeadType::None {
                return base_psid;
            }
        }
        // 이 스타일을 사용하는 첫 번째 문단의 para_shape_id에서 번호 정보 탐색
        let sid = style_id as u8;
        for section in &self.core.document.sections {
            for para in &section.paragraphs {
                if para.style_id == sid {
                    if let Some(ps) = self
                        .core
                        .document
                        .doc_info
                        .para_shapes
                        .get(para.para_shape_id as usize)
                    {
                        if ps.head_type != HeadType::None {
                            return para.para_shape_id;
                        }
                    }
                }
            }
        }
        base_psid
    }

}
