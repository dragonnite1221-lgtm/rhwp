//! 각주(footnote) · 각주 내 편집 API (WASM 바인딩).
//!
//! `HwpDocument`의 각주 삽입/삭제, 각주 내 텍스트 편집, 각주 hit-test 등
//! 각주 관련 `#[wasm_bindgen]` 메서드 모음. (수식 삽입 `insert_equation` 포함.)
//! 모두 `document_core`의 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 각주를 삽입한다.
    #[wasm_bindgen(js_name = insertFootnote)]
    pub fn insert_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.insert_footnote_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 수식을 삽입한다.
    #[wasm_bindgen(js_name = insertEquation)]
    pub fn insert_equation(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        script: &str,
        font_size: u32,
        color: u32,
    ) -> Result<String, JsValue> {
        self.insert_equation_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            script,
            font_size,
            color,
        )
        .map_err(|e| e.into())
    }

    /// 각주 정보를 조회한다.
    #[wasm_bindgen(js_name = getFootnoteInfo)]
    pub fn get_footnote_info(
        &self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_footnote_info_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 본문 커서 위치의 각주 마커를 조회한다.
    ///
    /// direction: "backward" 또는 "forward"
    #[wasm_bindgen(js_name = getFootnoteAtCursor)]
    pub fn get_footnote_at_cursor(
        &self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        direction: &str,
    ) -> Result<String, JsValue> {
        self.get_footnote_at_cursor_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            direction,
        )
        .map_err(|e| e.into())
    }

    /// 본문 각주 컨트롤을 삭제한다.
    #[wasm_bindgen(js_name = deleteFootnote)]
    pub fn delete_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_footnote_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 각주 내 텍스트를 삽입한다.
    #[wasm_bindgen(js_name = insertTextInFootnote)]
    pub fn insert_text_in_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
        fn_para_idx: u32,
        char_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        self.insert_text_in_footnote_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
            fn_para_idx as usize,
            char_offset as usize,
            text,
        )
        .map_err(|e| e.into())
    }

    /// 각주 내 텍스트를 삭제한다.
    #[wasm_bindgen(js_name = deleteTextInFootnote)]
    pub fn delete_text_in_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
        fn_para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.delete_text_in_footnote_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
            fn_para_idx as usize,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    /// 각주 내 문단을 분할한다 (Enter).
    #[wasm_bindgen(js_name = splitParagraphInFootnote)]
    pub fn split_paragraph_in_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
        fn_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.split_paragraph_in_footnote_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
            fn_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 각주 내 문단을 병합한다 (Backspace at start).
    #[wasm_bindgen(js_name = mergeParagraphInFootnote)]
    pub fn merge_paragraph_in_footnote(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
        fn_para_idx: u32,
    ) -> Result<String, JsValue> {
        self.merge_paragraph_in_footnote_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
            fn_para_idx as usize,
        )
        .map_err(|e| e.into())
    }

}
