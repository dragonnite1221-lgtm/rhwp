//! 경로 기반 중첩 표 좌표·이동 API (WASM 바인딩).
//!
//! `HwpDocument`의 path 기반 커서 좌표, 셀 정보, 표 차원, 수직 이동
//! `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    // ─── 경로 기반 중첩 표 API ───────────────────────────────

    /// 경로 기반 커서 좌표 조회 (중첩 표용).
    ///
    /// path_json: `[{"controlIndex":N,"cellIndex":N,"cellParaIndex":N}, ...]`
    /// 반환: JSON `{"pageIndex":N,"x":F,"y":F,"height":F}`
    #[wasm_bindgen(js_name = getCursorRectByPath)]
    pub fn get_cursor_rect_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.get_cursor_rect_by_path_native(
            section_idx as usize,
            parent_para_idx as usize,
            path_json,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 경로 기반 셀 정보 조회 (중첩 표용).
    ///
    /// 반환: JSON `{"row":N,"col":N,"rowSpan":N,"colSpan":N}`
    #[wasm_bindgen(js_name = getCellInfoByPath)]
    pub fn get_cell_info_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<String, JsValue> {
        self.get_cell_info_by_path_native(section_idx as usize, parent_para_idx as usize, path_json)
            .map_err(|e| e.into())
    }

    /// 경로 기반 표 차원 조회 (중첩 표용).
    ///
    /// 반환: JSON `{"rowCount":N,"colCount":N,"cellCount":N}`
    #[wasm_bindgen(js_name = getTableDimensionsByPath)]
    pub fn get_table_dimensions_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<String, JsValue> {
        self.get_table_dimensions_by_path_native(
            section_idx as usize,
            parent_para_idx as usize,
            path_json,
        )
        .map_err(|e| e.into())
    }

    /// 경로 기반 표 셀 바운딩박스 조회 (중첩 표용).
    ///
    /// 반환: JSON 배열 `[{"cellIdx":N,"row":N,"col":N,...,"x":F,"y":F,"w":F,"h":F}, ...]`
    #[wasm_bindgen(js_name = getTableCellBboxesByPath)]
    pub fn get_table_cell_bboxes_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<String, JsValue> {
        self.get_table_cell_bboxes_by_path_native(
            section_idx as usize,
            parent_para_idx as usize,
            path_json,
        )
        .map_err(|e| e.into())
    }

    /// 경로 기반 수직 커서 이동 (중첩 표용).
    ///
    /// 반환: JSON `{DocumentPosition + CursorRect + preferredX}`
    #[wasm_bindgen(js_name = moveVerticalByPath)]
    pub fn move_vertical_by_path(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
        delta: i32,
        preferred_x: f64,
    ) -> Result<String, JsValue> {
        self.move_vertical_by_path_native(
            section_idx as usize,
            parent_para_idx as usize,
            path_json,
            char_offset as usize,
            delta,
            preferred_x,
        )
        .map_err(|e| e.into())
    }
}
