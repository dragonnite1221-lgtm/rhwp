//! 표 속성·바운딩박스·생성 API (WASM 바인딩).
//!
//! `HwpDocument`의 표 속성 조회·설정, 셀/표 바운딩박스, 표 삭제·생성
//! `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 표 속성을 조회한다.
    ///
    /// 반환: JSON `{cellSpacing, paddingLeft, paddingRight, paddingTop, paddingBottom, pageBreak, repeatHeader}`
    #[wasm_bindgen(js_name = getTableProperties)]
    pub fn get_table_properties(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_table_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 속성을 수정한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = setTableProperties)]
    pub fn set_table_properties(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        json: &str,
    ) -> Result<String, JsValue> {
        self.set_table_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            json,
        )
        .map_err(|e| e.into())
    }

    /// 표의 모든 셀 bbox를 반환한다 (F5 셀 선택 모드용).
    ///
    /// 반환: JSON `[{cellIdx, row, col, rowSpan, colSpan, pageIndex, x, y, w, h}, ...]`
    #[wasm_bindgen(js_name = getTableCellBboxes)]
    pub fn get_table_cell_bboxes(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        page_hint: Option<u32>,
    ) -> Result<String, JsValue> {
        self.get_table_cell_bboxes_from_page(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            page_hint.unwrap_or(0) as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 전체의 바운딩박스를 반환한다.
    ///
    /// 반환: JSON `{"pageIndex":<N>,"x":<f>,"y":<f>,"width":<f>,"height":<f>}`
    #[wasm_bindgen(js_name = getTableBBox)]
    pub fn get_table_bbox(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_table_bbox_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 컨트롤을 문단에서 삭제한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = deleteTableControl)]
    pub fn delete_table_control(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_table_control_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 커서 위치에 새 표를 삽입한다.
    ///
    /// 반환: JSON `{"ok":true,"paraIdx":<N>,"controlIdx":0}`
    #[wasm_bindgen(js_name = createTable)]
    pub fn create_table(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        row_count: u32,
        col_count: u32,
    ) -> Result<String, JsValue> {
        self.create_table_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            row_count as u16,
            col_count as u16,
        )
        .map_err(|e| e.into())
    }

    /// 커서 위치에 표를 삽입한다 (확장, JSON 옵션).
    ///
    /// options JSON: { sectionIdx, paraIdx, charOffset, rowCount, colCount,
    ///                 treatAsChar?: bool, colWidths?: [u32, ...] }
    #[wasm_bindgen(js_name = createTableEx)]
    pub fn create_table_ex(&mut self, options_json: &str) -> Result<String, JsValue> {
        use crate::document_core::helpers::{json_bool, json_u32};
        let section_idx = json_u32(options_json, "sectionIdx").unwrap_or(0) as usize;
        let para_idx = json_u32(options_json, "paraIdx").unwrap_or(0) as usize;
        let char_offset = json_u32(options_json, "charOffset").unwrap_or(0) as usize;
        let row_count = json_u32(options_json, "rowCount").unwrap_or(2) as u16;
        let col_count = json_u32(options_json, "colCount").unwrap_or(2) as u16;
        let treat_as_char = json_bool(options_json, "treatAsChar").unwrap_or(false);
        // colWidths: JSON 배열에서 u32 목록 추출
        let col_widths: Option<Vec<u32>> = {
            let key = "colWidths";
            if let Some(start) = options_json.find(&format!("\"{}\"", key)) {
                let rest = &options_json[start..];
                if let Some(arr_start) = rest.find('[') {
                    if let Some(arr_end) = rest[arr_start..].find(']') {
                        let arr_str = &rest[arr_start + 1..arr_start + arr_end];
                        let nums: Vec<u32> = arr_str
                            .split(',')
                            .filter_map(|s| s.trim().parse::<u32>().ok())
                            .collect();
                        if !nums.is_empty() {
                            Some(nums)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        self.create_table_ex_native(
            section_idx,
            para_idx,
            char_offset,
            row_count,
            col_count,
            treat_as_char,
            col_widths.as_deref(),
        )
        .map_err(|e| e.into())
    }
}
