//! 머리말/꼬리말 서식·필드·템플릿 API (WASM 바인딩).
//!
//! `HwpDocument`의 머리말/꼬리말 문단 서식, 필드 삽입, 템플릿 적용,
//! 삭제·목록·내비게이션 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 머리말/꼬리말 문단의 문단 속성을 조회한다.
    #[wasm_bindgen(js_name = getParaPropertiesInHf)]
    pub fn get_para_properties_in_hf(
        &self,
        section_idx: usize,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: usize,
    ) -> Result<String, JsValue> {
        self.get_para_properties_in_hf_native(section_idx, is_header, apply_to, hf_para_idx)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 문단에 문단 서식을 적용한다.
    #[wasm_bindgen(js_name = applyParaFormatInHf)]
    pub fn apply_para_format_in_hf(
        &mut self,
        section_idx: usize,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: usize,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.apply_para_format_in_hf_native(
            section_idx,
            is_header,
            apply_to,
            hf_para_idx,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 문단에 필드 마커를 삽입한다.
    #[wasm_bindgen(js_name = insertFieldInHf)]
    pub fn insert_field_in_hf(
        &mut self,
        section_idx: usize,
        is_header: bool,
        apply_to: u8,
        hf_para_idx: usize,
        char_offset: usize,
        field_type: u8,
    ) -> Result<String, JsValue> {
        self.insert_field_in_hf_native(
            section_idx,
            is_header,
            apply_to,
            hf_para_idx,
            char_offset,
            field_type,
        )
        .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 마당(템플릿)을 적용한다.
    #[wasm_bindgen(js_name = applyHfTemplate)]
    pub fn apply_hf_template(
        &mut self,
        section_idx: usize,
        is_header: bool,
        apply_to: u8,
        template_id: u8,
    ) -> Result<String, JsValue> {
        self.apply_hf_template_native(section_idx, is_header, apply_to, template_id)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말을 삭제한다 (컨트롤 자체 제거).
    #[wasm_bindgen(js_name = deleteHeaderFooter)]
    pub fn delete_header_footer(
        &mut self,
        section_idx: u32,
        is_header: bool,
        apply_to: u32,
    ) -> Result<String, JsValue> {
        self.delete_header_footer_native(section_idx as usize, is_header, apply_to as u8)
            .map_err(|e| e.into())
    }

    /// 문서 전체의 머리말/꼬리말 목록을 반환한다.
    #[wasm_bindgen(js_name = getHeaderFooterList)]
    pub fn get_header_footer_list(
        &self,
        current_section_idx: u32,
        current_is_header: bool,
        current_apply_to: u32,
    ) -> Result<String, JsValue> {
        self.get_header_footer_list_native(
            current_section_idx as usize,
            current_is_header,
            current_apply_to as u8,
        )
        .map_err(|e| e.into())
    }

    /// 페이지 단위로 이전/다음 머리말·꼬리말로 이동한다.
    ///
    /// 반환: JSON `{"ok":true,"pageIndex":N,"sectionIdx":N,"isHeader":bool,"applyTo":N}`
    /// 또는 더 이상 이동할 페이지가 없으면 `{"ok":false}`
    #[wasm_bindgen(js_name = navigateHeaderFooterByPage)]
    pub fn navigate_header_footer_by_page(
        &self,
        current_page: u32,
        is_header: bool,
        direction: i32,
    ) -> Result<String, JsValue> {
        self.navigate_header_footer_by_page_native(current_page, is_header, direction)
            .map_err(|e| e.into())
    }

    /// 머리말/꼬리말 감추기를 토글한다 (현재 쪽만).
    ///
    /// 반환: JSON `{"hidden":true/false}` — 토글 후 상태
    #[wasm_bindgen(js_name = toggleHideHeaderFooter)]
    pub fn toggle_hide_header_footer(
        &mut self,
        page_index: u32,
        is_header: bool,
    ) -> Result<String, JsValue> {
        self.toggle_hide_header_footer_native(page_index, is_header)
            .map_err(|e| e.into())
    }
}
