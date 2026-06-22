//! 머리말/꼬리말(header/footer) CRUD·텍스트 편집 API (WASM 바인딩).
//!
//! `HwpDocument`의 머리말/꼬리말 조회·생성·텍스트 편집 `#[wasm_bindgen]`
//! 메서드 모음 (순수 이동, 동작 보존). 모두 `*_native` 구현에 위임한다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    // ─── 머리말/꼬리말 API ──────────────────────────────────

    /// 머리말/꼬리말 조회
    ///
    /// 반환: JSON `{"ok":true,"exists":true/false,...}`
    #[wasm_bindgen(js_name = getHeaderFooter)]
    pub fn get_header_footer(
        &self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
    ) -> Result<String, JsValue> {
        self.get_header_footer_native(section_idx as usize, is_header, apply_to)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 생성 (빈 문단 1개 포함)
    ///
    /// 반환: JSON `{"ok":true,"kind":"header/footer","applyTo":N,...}`
    #[wasm_bindgen(js_name = createHeaderFooter)]
    pub fn create_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
    ) -> Result<String, JsValue> {
        self.create_header_footer_native(section_idx as usize, is_header, apply_to)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내 텍스트 삽입
    ///
    /// 반환: JSON `{"ok":true,"charOffset":<new_offset>}`
    #[wasm_bindgen(js_name = insertTextInHeaderFooter)]
    pub fn insert_text_in_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
        char_offset: u32,
        text: &str,
    ) -> Result<String, JsValue> {
        self.insert_text_in_header_footer_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
            char_offset as usize,
            text,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내 텍스트 삭제
    ///
    /// 반환: JSON `{"ok":true,"charOffset":<offset>}`
    #[wasm_bindgen(js_name = deleteTextInHeaderFooter)]
    pub fn delete_text_in_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
        char_offset: u32,
        count: u32,
    ) -> Result<String, JsValue> {
        self.delete_text_in_header_footer_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
            char_offset as usize,
            count as usize,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내 문단 분할 (Enter 키)
    ///
    /// 반환: JSON `{"ok":true,"hfParaIndex":<new_idx>,"charOffset":0}`
    #[wasm_bindgen(js_name = splitParagraphInHeaderFooter)]
    pub fn split_paragraph_in_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.split_paragraph_in_header_footer_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 내 문단 병합 (Backspace at start)
    ///
    /// 반환: JSON `{"ok":true,"hfParaIndex":<prev_idx>,"charOffset":<merge_point>}`
    #[wasm_bindgen(js_name = mergeParagraphInHeaderFooter)]
    pub fn merge_paragraph_in_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
    ) -> Result<String, JsValue> {
        self.merge_paragraph_in_header_footer_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 문단 정보 조회
    ///
    /// 반환: JSON `{"ok":true,"paraCount":N,"charCount":N}`
    #[wasm_bindgen(js_name = getHeaderFooterParaInfo)]
    pub fn get_header_footer_para_info(
        &self,
        section_idx: u32,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_header_footer_para_info_native(
            section_idx as usize,
            is_header,
            apply_to,
            hf_para_idx as usize,
        )
        .map_err(|e| e.into())
    }
}
