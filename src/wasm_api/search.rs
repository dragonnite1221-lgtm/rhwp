//! 검색 · 치환 · 위치 변환 API (WASM 바인딩).
//!
//! 문서 텍스트 검색/치환, 전역 페이지 ↔ 문서 위치 변환 등
//! `HwpDocument`의 검색/위치 관련 `#[wasm_bindgen]` 메서드 모음.
//! 모두 `self.core`의 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 문서 텍스트 검색
    #[wasm_bindgen(js_name = searchText)]
    pub fn search_text(
        &self,
        query: &str,
        from_sec: u32,
        from_para: u32,
        from_char: u32,
        forward: bool,
        case_sensitive: bool,
    ) -> Result<String, JsValue> {
        self.core
            .search_text_native(
                query,
                from_sec as usize,
                from_para as usize,
                from_char as usize,
                forward,
                case_sensitive,
            )
            .map_err(|e| e.into())
    }

    /// 텍스트 치환 (단일)
    #[wasm_bindgen(js_name = replaceText)]
    pub fn replace_text(
        &mut self,
        sec: u32,
        para: u32,
        char_offset: u32,
        length: u32,
        new_text: &str,
    ) -> Result<String, JsValue> {
        self.core
            .replace_text_native(
                sec as usize,
                para as usize,
                char_offset as usize,
                length as usize,
                new_text,
            )
            .map_err(|e| e.into())
    }

    /// 단일 치환 (검색어 기반) — 첫 번째 매치만 교체
    #[wasm_bindgen(js_name = replaceOne)]
    pub fn replace_one(
        &mut self,
        query: &str,
        new_text: &str,
        case_sensitive: bool,
    ) -> Result<String, JsValue> {
        self.core.replace_one_native(query, new_text, case_sensitive)
            .map_err(|e| e.into())
    }

    /// 전체 치환
    #[wasm_bindgen(js_name = replaceAll)]
    pub fn replace_all(
        &mut self,
        query: &str,
        new_text: &str,
        case_sensitive: bool,
    ) -> Result<String, JsValue> {
        self.core
            .replace_all_native(query, new_text, case_sensitive)
            .map_err(|e| e.into())
    }

    /// 글로벌 쪽 번호에 해당하는 첫 문단 위치 반환
    #[wasm_bindgen(js_name = getPositionOfPage)]
    pub fn get_position_of_page(&self, global_page: u32) -> Result<String, JsValue> {
        self.core
            .get_position_of_page_native(global_page as usize)
            .map_err(|e| e.into())
    }

    /// 위치에 해당하는 글로벌 쪽 번호 반환
    #[wasm_bindgen(js_name = getPageOfPosition)]
    pub fn get_page_of_position(&self, section_idx: u32, para_idx: u32) -> Result<String, JsValue> {
        self.core
            .get_page_of_position_native(section_idx as usize, para_idx as usize)
            .map_err(|e| e.into())
    }
}
