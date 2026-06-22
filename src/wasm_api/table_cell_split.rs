//! 표 셀 병합·분할 API (WASM 바인딩).
//!
//! `HwpDocument`의 표 셀 병합, 셀 분할 `#[wasm_bindgen]` 메서드 모음
//! (순수 이동, 동작 보존). 모두 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 표의 셀을 병합한다.
    ///
    /// 반환값: JSON `{"ok":true,"cellCount":<N>}`
    #[wasm_bindgen(js_name = mergeTableCells)]
    pub fn merge_table_cells(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Result<String, JsValue> {
        self.merge_table_cells_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            start_row as u16,
            start_col as u16,
            end_row as u16,
            end_col as u16,
        )
        .map_err(|e| e.into())
    }

    /// 병합된 셀을 나눈다 (split).
    ///
    /// 반환값: JSON `{"ok":true,"cellCount":<N>}`
    #[wasm_bindgen(js_name = splitTableCell)]
    pub fn split_table_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        row: u32,
        col: u32,
    ) -> Result<String, JsValue> {
        self.split_table_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            row as u16,
            col as u16,
        )
        .map_err(|e| e.into())
    }

    /// 셀을 N줄 × M칸으로 분할한다.
    ///
    /// 반환값: JSON `{"ok":true,"cellCount":<N>}`
    #[wasm_bindgen(js_name = splitTableCellInto)]
    pub fn split_table_cell_into(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        row: u32,
        col: u32,
        n_rows: u32,
        m_cols: u32,
        equal_row_height: bool,
        merge_first: bool,
    ) -> Result<String, JsValue> {
        self.split_table_cell_into_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            row as u16,
            col as u16,
            n_rows as u16,
            m_cols as u16,
            equal_row_height,
            merge_first,
        )
        .map_err(|e| e.into())
    }

    /// 범위 내 셀들을 각각 N줄 × M칸으로 분할한다.
    ///
    /// 반환값: JSON `{"ok":true,"cellCount":<N>}`
    #[wasm_bindgen(js_name = splitTableCellsInRange)]
    pub fn split_table_cells_in_range(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
        n_rows: u32,
        m_cols: u32,
        equal_row_height: bool,
    ) -> Result<String, JsValue> {
        self.split_table_cells_in_range_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            start_row as u16,
            start_col as u16,
            end_row as u16,
            end_col as u16,
            n_rows as u16,
            m_cols as u16,
            equal_row_height,
        )
        .map_err(|e| e.into())
    }
}
