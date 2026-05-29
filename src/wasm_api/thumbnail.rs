use wasm_bindgen::prelude::*;

/// HWP 파일에서 썸네일 이미지만 경량 추출 (전체 파싱 없이)
///
/// 반환: JSON `{ "format": "png"|"gif", "base64": "...", "width": N, "height": N }`
/// PrvImage가 없으면 `null` 반환
#[wasm_bindgen(js_name = extractThumbnail)]
pub fn extract_thumbnail(data: &[u8]) -> JsValue {
    match crate::parser::extract_thumbnail_only(data) {
        Some(result) => {
            let base64 = base64_encode(&result.data);
            let mime = match result.format.as_str() {
                "png" => "image/png",
                "bmp" => "image/bmp",
                "gif" => "image/gif",
                _ => "application/octet-stream",
            };
            let json = format!(
                r#"{{"format":"{}","base64":"{}","dataUri":"data:{};base64,{}","width":{},"height":{}}}"#,
                result.format, base64, mime, base64, result.width, result.height
            );
            JsValue::from_str(&json)
        }
        None => JsValue::NULL,
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
