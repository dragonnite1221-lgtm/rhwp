//! 클립보드 API (WASM 바인딩).
//!
//! `HwpDocument`의 클립보드 관련 `#[wasm_bindgen]` 메서드를 모은 모듈.
//! 모두 `document_core`의 `*_native` 구현에 위임하는 얇은 래퍼이다.

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 내부 클립보드에 데이터가 있는지 확인한다.
    #[wasm_bindgen(js_name = hasInternalClipboard)]
    pub fn has_internal_clipboard(&self) -> bool {
        self.has_internal_clipboard_native()
    }

    /// 내부 클립보드의 플레인 텍스트를 반환한다.
    #[wasm_bindgen(js_name = getClipboardText)]
    pub fn get_clipboard_text(&self) -> String {
        self.get_clipboard_text_native()
    }

    /// 내부 클립보드를 초기화한다.
    #[wasm_bindgen(js_name = clearClipboard)]
    pub fn clear_clipboard(&mut self) {
        self.clear_clipboard_native()
    }

    /// 선택 영역을 내부 클립보드에 복사한다.
    ///
    /// 반환값: JSON `{"ok":true,"text":"<plain_text>"}`
    #[wasm_bindgen(js_name = copySelection)]
    pub fn copy_selection(
        &mut self,
        section_idx: u32,
        start_para_idx: u32,
        start_char_offset: u32,
        end_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.copy_selection_native(
            section_idx as usize,
            start_para_idx as usize,
            start_char_offset as usize,
            end_para_idx as usize,
            end_char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 표 셀 내부 선택 영역을 내부 클립보드에 복사한다.
    #[wasm_bindgen(js_name = copySelectionInCell)]
    pub fn copy_selection_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        start_cell_para_idx: u32,
        start_char_offset: u32,
        end_cell_para_idx: u32,
        end_char_offset: u32,
    ) -> Result<String, JsValue> {
        self.copy_selection_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            start_cell_para_idx as usize,
            start_char_offset as usize,
            end_cell_para_idx as usize,
            end_char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 컨트롤 객체(표, 이미지, 도형)를 내부 클립보드에 복사한다.
    #[wasm_bindgen(js_name = copyControl)]
    pub fn copy_control(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.copy_control_native(
            section_idx as usize,
            para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 내부 클립보드에 컨트롤(표/그림/도형)이 포함되어 있는지 확인한다.
    #[wasm_bindgen(js_name = clipboardHasControl)]
    pub fn clipboard_has_control(&self) -> bool {
        self.clipboard_has_control_native()
    }

    /// 내부 클립보드의 컨트롤 객체를 캐럿 위치에 붙여넣는다.
    ///
    /// 반환값: JSON `{"ok":true,"paraIdx":<idx>,"controlIdx":0}`
    #[wasm_bindgen(js_name = pasteControl)]
    pub fn paste_control(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.paste_control_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 내부 클립보드의 내용을 캐럿 위치에 붙여넣는다 (본문 문단).
    ///
    /// 반환값: JSON `{"ok":true,"paraIdx":<idx>,"charOffset":<offset>}`
    #[wasm_bindgen(js_name = pasteInternal)]
    pub fn paste_internal(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.paste_internal_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

    /// 내부 클립보드의 내용을 표 셀 내부에 붙여넣는다.
    ///
    /// 반환값: JSON `{"ok":true,"cellParaIdx":<idx>,"charOffset":<offset>}`
    #[wasm_bindgen(js_name = pasteInternalInCell)]
    pub fn paste_internal_in_cell(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        cell_idx: u32,
        cell_para_idx: u32,
        char_offset: u32,
    ) -> Result<String, JsValue> {
        self.paste_internal_in_cell_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            cell_idx as usize,
            cell_para_idx as usize,
            char_offset as usize,
        )
        .map_err(|e| e.into())
    }

}
