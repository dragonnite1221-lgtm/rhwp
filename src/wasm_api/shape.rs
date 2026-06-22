//! 글상자/도형(Shape) 생성·속성·그룹·이동 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::helpers::{json_bool, json_str, json_u32};

#[wasm_bindgen]
impl HwpDocument {
    // ─── Shape(글상자) API ───────────────────────────────

    /// 커서 위치에 글상자(Rectangle + TextBox)를 삽입한다.
    ///
    /// json: `{"sectionIdx":N,"paraIdx":N,"charOffset":N,"width":N,"height":N,
    ///         "horzOffset":N,"vertOffset":N,"treatAsChar":bool,"textWrap":"Square"}`
    /// 반환: JSON `{"ok":true,"paraIdx":<N>,"controlIdx":0}`
    #[wasm_bindgen(js_name = createShapeControl)]
    pub fn create_shape_control(&mut self, json: &str) -> Result<String, JsValue> {
        let sec = json_u32(json, "sectionIdx").unwrap_or(0) as usize;
        let para = json_u32(json, "paraIdx").unwrap_or(0) as usize;
        let offset = json_u32(json, "charOffset").unwrap_or(0) as usize;
        let width = json_u32(json, "width").unwrap_or(8504);
        let height = json_u32(json, "height").unwrap_or(8504);
        let horz_offset = json_u32(json, "horzOffset").unwrap_or(0);
        let vert_offset = json_u32(json, "vertOffset").unwrap_or(0);
        let shape_type = json_str(json, "shapeType").unwrap_or_else(|| "rectangle".to_string());
        // 글상자는 기본적으로 treat_as_char=true (한컴 기본값)
        let default_tac = shape_type == "textbox";
        let treat_as_char = json_bool(json, "treatAsChar").unwrap_or(default_tac);
        let text_wrap = json_str(json, "textWrap").unwrap_or_else(|| "Square".to_string());
        let line_flip_x = json_bool(json, "lineFlipX").unwrap_or(false);
        let line_flip_y = json_bool(json, "lineFlipY").unwrap_or(false);
        // 다각형 꼭짓점: "polygonPoints":[{"x":N,"y":N},...]
        let polygon_points: Vec<crate::model::Point> = if shape_type == "polygon" {
            Self::parse_polygon_points(json)
        } else {
            Vec::new()
        };
        let result = self.create_shape_control_native(
            sec,
            para,
            offset,
            width,
            height,
            horz_offset,
            vert_offset,
            treat_as_char,
            &text_wrap,
            &shape_type,
            line_flip_x,
            line_flip_y,
            &polygon_points,
        )?;

        // 연결선: SubjectID + 제어점 라우팅 설정 (생성 후)
        if shape_type.starts_with("connector-") {
            let ssid = json_u32(json, "startSubjectID").unwrap_or(0);
            let ssidx = json_u32(json, "startSubjectIndex").unwrap_or(0);
            let esid = json_u32(json, "endSubjectID").unwrap_or(0);
            let esidx = json_u32(json, "endSubjectIndex").unwrap_or(0);
            let pi = json_u32(&result, "paraIdx");
            let ci = json_u32(&result, "controlIdx");
            if let (Some(pi), Some(ci)) = (pi, ci) {
                self.update_connector_subject_ids(
                    sec,
                    pi as usize,
                    ci as usize,
                    ssid,
                    ssidx,
                    esid,
                    esidx,
                );
                self.recalculate_connector_routing(sec, pi as usize, ci as usize, ssidx, esidx);
            }
        }

        Ok(result)
    }

    /// Shape(글상자) 속성을 조회한다.
    ///
    /// 반환: JSON `{ width, height, treatAsChar, tbMarginLeft, ... }`
    #[wasm_bindgen(js_name = getShapeProperties)]
    pub fn get_shape_properties(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_shape_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// Shape(글상자) 속성을 변경한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = setShapeProperties)]
    pub fn set_shape_properties(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.set_shape_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// Shape(글상자) 컨트롤을 문단에서 삭제한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = deleteShapeControl)]
    pub fn delete_shape_control(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_shape_control_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// JSON에서 polygonPoints 배열 파싱
    fn parse_polygon_points(json: &str) -> Vec<crate::model::Point> {
        // 간단한 파싱: "polygonPoints":[{"x":1,"y":2},{"x":3,"y":4}]
        let key = "\"polygonPoints\":[";
        if let Some(start) = json.find(key) {
            let rest = &json[start + key.len()..];
            if let Some(end) = rest.find(']') {
                let arr = &rest[..end];
                return arr
                    .split("},")
                    .filter_map(|item| {
                        let item = item.trim().trim_start_matches('{').trim_end_matches('}');
                        let x =
                            crate::document_core::helpers::json_i32(&format!("{{{}}}", item), "x")?;
                        let y =
                            crate::document_core::helpers::json_i32(&format!("{{{}}}", item), "y")?;
                        Some(crate::model::Point { x, y })
                    })
                    .collect();
            }
        }
        Vec::new()
    }
}
