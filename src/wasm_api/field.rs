//! 필드(누름틀) 위치 조회 · 활성 필드 API (WASM 바인딩).
//!
//! 커서 위치의 필드 범위 조회, 필드 제거, 활성 필드 설정/해제 등
//! `HwpDocument`의 필드 위치 관련 `#[wasm_bindgen]` 메서드 모음.

use wasm_bindgen::prelude::*;

use super::HwpDocument;
use crate::document_core::DocumentCore;

#[wasm_bindgen]
impl HwpDocument {
    /// 커서 위치의 필드 범위 정보를 조회한다 (본문 문단).
    ///
    /// 반환: `{inField, fieldId?, startCharIdx?, endCharIdx?, isGuide?, guideName?}`
    #[wasm_bindgen(js_name = getFieldInfoAt)]
    pub fn get_field_info_at_api(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> String {
        self.get_field_info_at(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
    }

    /// 커서 위치의 필드 범위 정보를 조회한다 (셀/글상자 내 문단).
    #[wasm_bindgen(js_name = getFieldInfoAtInCell)]
    pub fn get_field_info_at_in_cell_api(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        is_textbox: bool,
    ) -> String {
        self.get_field_info_at_in_cell(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            is_textbox,
        )
    }

    /// 커서 위치의 누름틀 필드를 제거한다 (본문 문단).
    #[wasm_bindgen(js_name = removeFieldAt)]
    pub fn remove_field_at_api(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> String {
        match self.remove_field_at(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        ) {
            Ok(s) => s,
            Err(e) => {
                let escaped = e.to_string().replace('\\', "\\\\").replace('"', "\\\"");
                format!("{{\"ok\":false,\"error\":\"{}\"}}", escaped)
            }
        }
    }

    /// 커서 위치의 누름틀 필드를 제거한다 (셀/글상자 내 문단).
    #[wasm_bindgen(js_name = removeFieldAtInCell)]
    pub fn remove_field_at_in_cell_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        is_textbox: bool,
    ) -> String {
        match self.remove_field_at_in_cell(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            is_textbox,
        ) {
            Ok(s) => s,
            Err(e) => {
                let escaped = e.to_string().replace('\\', "\\\\").replace('"', "\\\"");
                format!("{{\"ok\":false,\"error\":\"{}\"}}", escaped)
            }
        }
    }

    /// 활성 필드를 설정한다 (본문 문단 — 안내문 숨김용).
    #[wasm_bindgen(js_name = setActiveField)]
    pub fn set_active_field_api(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> bool {
        self.set_active_field(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
    }

    /// 활성 필드를 설정한다 (셀/글상자 내 문단 — 안내문 숨김용).
    /// 변경이 발생하면 true를 반환한다.
    #[wasm_bindgen(js_name = setActiveFieldInCell)]
    pub fn set_active_field_in_cell_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
        is_textbox: bool,
    ) -> bool {
        self.set_active_field_in_cell(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
            is_textbox,
        )
    }

    /// path 기반: 중첩 표 셀의 필드 범위 정보를 조회한다.
    #[wasm_bindgen(js_name = getFieldInfoAtByPath)]
    pub fn get_field_info_at_by_path_api(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
    ) -> String {
        match DocumentCore::parse_cell_path(path_json) {
            Ok(path) => self.get_field_info_at_by_path(
                section_idx as usize,
                parent_para_idx as usize,
                &path,
                char_offset as usize,
            ),
            Err(_) => r#"{"inField":false}"#.to_string(),
        }
    }

    /// path 기반: 중첩 표 셀 내 활성 필드를 설정한다.
    #[wasm_bindgen(js_name = setActiveFieldByPath)]
    pub fn set_active_field_by_path_api(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        path_json: &str,
        char_offset: u32,
    ) -> bool {
        match DocumentCore::parse_cell_path(path_json) {
            Ok(path) => self.set_active_field_by_path(
                section_idx as usize,
                parent_para_idx as usize,
                &path,
                char_offset as usize,
            ),
            Err(_) => false,
        }
    }

    /// 활성 필드를 해제한다 (안내문 다시 표시).
    #[wasm_bindgen(js_name = clearActiveField)]
    pub fn clear_active_field_api(&mut self) {
        self.clear_active_field();
    }
}
