//! 중첩 표 path 기반 셀 편집 API (WASM 바인딩).
//!
//! `HwpDocument`의 path 기반 셀 텍스트 편집 `#[wasm_bindgen]` 메서드 모음
//! (순수 이동, 동작 보존). 모두 `document_core`의 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::DocumentCore;

#[wasm_bindgen]
impl HwpDocument {
    // ─── 중첩 표 path 기반 편집 API ──────────────────────────

    #[wasm_bindgen(js_name = insertTextInCellByPath)]
    pub fn insert_text_in_cell_by_path_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        self.insert_text_in_cell_by_path(
            section_idx as usize,
            parent_para_idx as usize,
            &path,
            char_offset as usize,
            text,
        )
        .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = deleteTextInCellByPath)]
    pub fn delete_text_in_cell_by_path_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        self.delete_text_in_cell_by_path(
            section_idx as usize,
            parent_para_idx as usize,
            &path,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = splitParagraphInCellByPath)]
    pub fn split_paragraph_in_cell_by_path_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        self.split_paragraph_in_cell_by_path(
            section_idx as usize,
            parent_para_idx as usize,
            &path,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = mergeParagraphInCellByPath)]
    pub fn merge_paragraph_in_cell_by_path_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
    ) -> Result<String, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        self.merge_paragraph_in_cell_by_path(section_idx as usize, parent_para_idx as usize, &path)
            .map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name = getTextInCellByPath)]
    pub fn get_text_in_cell_by_path_api(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        let path = DocumentCore::parse_cell_path(path_json)?;
        self.get_text_in_cell_by_path(
            section_idx as usize,
            parent_para_idx as usize,
            &path,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }
}
