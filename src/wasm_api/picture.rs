//! 그림(Picture) 삽입·외부 이미지 주입·속성 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 커서 위치에 그림을 삽입한다.
    ///
    /// image_data: 이미지 바이너리 데이터 (PNG/JPG/GIF/BMP 등)
    /// width, height: HWPUNIT 단위 크기
    /// extension: 파일 확장자 (jpg, png 등)
    ///
    /// 반환: JSON `{"ok":true,"paraIdx":<N>,"controlIdx":0}`
    #[wasm_bindgen(js_name = insertPicture)]
    pub fn insert_picture(
        &mut self,
        section_idx: u32,
        para_idx: u32,
        char_offset: u32,
        image_data: &[u8],
        width: u32,
        height: u32,
        natural_width_px: u32,
        natural_height_px: u32,
        extension: &str,
        description: &str,
    ) -> Result<String, JsValue> {
        self.insert_picture_native(
            section_idx as usize,
            para_idx as usize,
            char_offset as usize,
            image_data,
            width,
            height,
            natural_width_px,
            natural_height_px,
            extension,
            description,
        )
        .map_err(|e| e.into())
    }


    /// 그림 컨트롤의 속성을 조회한다.
    ///
    /// 반환: JSON `{ width, height, treatAsChar, ... }`
    #[wasm_bindgen(js_name = getPictureProperties)]
    pub fn get_picture_properties(
        &self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.get_picture_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

    /// 그림 컨트롤의 속성을 변경한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = setPictureProperties)]
    pub fn set_picture_properties(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
        props_json: &str,
    ) -> Result<String, JsValue> {
        self.set_picture_properties_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
            props_json,
        )
        .map_err(|e| e.into())
    }

    /// 그림 컨트롤을 문단에서 삭제한다.
    ///
    /// 반환: JSON `{"ok":true}`
    #[wasm_bindgen(js_name = deletePictureControl)]
    pub fn delete_picture_control(
        &mut self,
        section_idx: u32,
        parent_para_idx: u32,
        control_idx: u32,
    ) -> Result<String, JsValue> {
        self.delete_picture_control_native(
            section_idx as usize,
            parent_para_idx as usize,
            control_idx as usize,
        )
        .map_err(|e| e.into())
    }

}
