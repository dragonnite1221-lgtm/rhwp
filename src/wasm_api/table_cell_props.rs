//! 표 차원·셀 정보·셀 속성 API (WASM 바인딩).
//!
//! `HwpDocument`의 표 차원 조회, 셀 정보/속성 조회·설정, 셀 리사이즈,
//! 표 이동 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 표의 행/열/셀 수를 반환한다.
    ///
    /// 반환: JSON `{"rowCount":N,"colCount":N,"cellCount":N}`
    #[wasm_bindgen(js_name = getTableDimensions)]
    pub fn get_table_dimensions(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_table_dimensions_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀의 행/열/병합 정보를 반환한다.
    ///
    /// 반환: JSON `{"row":N,"col":N,"rowSpan":N,"colSpan":N}`
    #[wasm_bindgen(js_name = getCellInfo)]
    pub fn get_cell_info(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_cell_info_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 셀 속성을 조회한다.
    ///
    /// 반환: JSON `{width, height, paddingLeft, paddingRight, paddingTop, paddingBottom, verticalAlign, textDirection, isHeader}`
    #[wasm_bindgen(js_name = getCellProperties)]
    pub fn get_cell_properties(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_cell_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 셀 속성을 수정한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = setCellProperties)]
    pub fn set_cell_properties(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        json: &str,
    ) -> Result<String, JsValue> {
        self.set_cell_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            json,
        )
        .map_err(|e| e.into())
    }

    /// 여러 셀의 width/height를 한 번에 조절한다 (배치).
    ///
    /// json: `[{"cellIdx":0,"widthDelta":150},{"cellIdx":2,"heightDelta":-100}]`
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = resizeTableCells)]
    pub fn resize_table_cells(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        json: &str,
    ) -> Result<String, JsValue> {
        self.resize_table_cells_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            json,
        )
        .map_err(|e| e.into())
    }

    /// 표의 위치 오프셋(vertical_offset, horizontal_offset)을 이동한다.
    ///
    /// delta_h, delta_v: HWPUNIT 단위 이동량 (양수=오른쪽/아래, 음수=왼쪽/위)
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = moveTableOffset)]
    pub fn move_table_offset(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        delta_h: i32,
        delta_v: i32,
    ) -> Result<String, JsValue> {
        self.move_table_offset_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            delta_h,
            delta_v,
        )
        .map_err(|e| e.into())
    }
}
