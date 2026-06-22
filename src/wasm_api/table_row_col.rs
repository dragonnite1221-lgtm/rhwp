//! 표 행·열 삽입·삭제 API (WASM 바인딩).
//!
//! `HwpDocument`의 표 행/열 삽입·삭제 `#[wasm_bindgen]` 메서드 모음
//! (순수 이동, 동작 보존). 모두 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 표에 행을 삽입한다.
    ///
    /// 반환값: JSON `{"ok":true,"rowCount":<N>,"colCount":<M>}`
    #[wasm_bindgen(js_name = insertTableRow)]
    pub fn insert_table_row(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        row_idx: u32,
        below: bool,
    ) -> Result<String, JsValue> {
        self.insert_table_row_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            row_idx as u16,
            below,
        )
        .map_err(|e| e.into())
    }

    /// 표에 열을 삽입한다.
    ///
    /// 반환값: JSON `{"ok":true,"rowCount":<N>,"colCount":<M>}`
    #[wasm_bindgen(js_name = insertTableColumn)]
    pub fn insert_table_column(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        col_idx: u32,
        right: bool,
    ) -> Result<String, JsValue> {
        self.insert_table_column_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            col_idx as u16,
            right,
        )
        .map_err(|e| e.into())
    }

    /// 표에서 행을 삭제한다.
    ///
    /// 반환값: JSON `{"ok":true,"rowCount":<N>,"colCount":<M>}`
    #[wasm_bindgen(js_name = deleteTableRow)]
    pub fn delete_table_row(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        row_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_table_row_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            row_idx as u16,
        )
        .map_err(|e| e.into())
    }

    /// 표에서 열을 삭제한다.
    ///
    /// 반환값: JSON `{"ok":true,"rowCount":<N>,"colCount":<M>}`
    #[wasm_bindgen(js_name = deleteTableColumn)]
    pub fn delete_table_column(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        col_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_table_column_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            col_idx as u16,
        )
        .map_err(|e| e.into())
    }
}
