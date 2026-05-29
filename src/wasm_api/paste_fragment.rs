use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 보존된 raw section/header XML 을 사용해 HWPX fragment 를 현재 Document 에 paste 한다.
    ///
    /// Phase 2 의 `pasteHwpxFragmentRaw` 와 달리 클라이언트가 zip/unzip 라운드트립을 다룰
    /// 필요가 없다. Document IR 도 자동으로 재파싱돼 후속 명령(rendering/edit)이 그대로 사용 가능.
    ///
    /// 반환 JSON 스키마:
    /// `{"inserted_para_count":N,"id_remap_char_pr":{...},"id_remap_para_pr":{...},
    ///   "id_remap_style":{...},"id_remap_border_fill":{...}}`
    ///
    /// 에러: 문서가 HWP 로 로드됐거나 raw XML 보존이 없으면 `NoSourceXml`,
    /// section_idx 가 범위 밖이면 `SectionOutOfRange`,
    /// fragment 가 well-formed 아니면 `Paste(...)`.
    #[wasm_bindgen(js_name = pasteHwpxFragmentInDocument)]
    pub fn paste_hwpx_fragment_in_document(
        &mut self,
        section_idx: u32,
        after_para_idx: u32,
        fragment_xml: &str,
        source_char_prs: &str,
        source_para_prs: &str,
        source_styles: &str,
        source_border_fills: &str,
    ) -> Result<String, JsValue> {
        use crate::document_core::SourceDefinitions;
        let source = SourceDefinitions {
            char_prs: source_char_prs.to_string(),
            para_prs: source_para_prs.to_string(),
            styles: source_styles.to_string(),
            border_fills: source_border_fills.to_string(),
        };
        let result = self
            .core
            .paste_hwpx_fragment_in_document_native(
                section_idx as usize,
                after_para_idx as usize,
                fragment_xml,
                &source,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // paste 후 layout 재계산 — 새 IR 위에 page count/measured 가 갱신되도록.
        // 미호출 시 클라이언트가 refreshPages 해도 pageCount/getPageInfo 가 stale 이라
        // 캔버스에 새 fragment 가 보이지 않는다.
        self.core.paginate();

        let mut json = String::with_capacity(256);
        json.push_str("{\"inserted_para_count\":");
        json.push_str(&result.inserted_para_count.to_string());
        json.push_str(",\"id_remap_char_pr\":");
        push_remap_json(&mut json, &result.id_remap.char_pr);
        json.push_str(",\"id_remap_para_pr\":");
        push_remap_json(&mut json, &result.id_remap.para_pr);
        json.push_str(",\"id_remap_style\":");
        push_remap_json(&mut json, &result.id_remap.style);
        json.push_str(",\"id_remap_border_fill\":");
        push_remap_json(&mut json, &result.id_remap.border_fill);
        json.push('}');
        Ok(json)
    }
}

/// 외부 HWPX fragment(원본 양식의 byte-exact slice)를 caret 위치에 byte-preserving + ID remap +
/// 표 정합성 보존하며 paste 한다.
///
/// **Document 모델 미사용** — 클라이언트가 hwpx unzip 후 raw section_xml/header_xml을 인자로
/// 전달하고, 결과 JSON의 `section_xml`/`header_xml`을 받아 zip에 다시 packing 한다.
/// 이 설계는 Document IR 동기화 문제를 회피해 byte-preserving 동작을 보장한다.
#[wasm_bindgen(js_name = pasteHwpxFragmentRaw)]
pub fn paste_hwpx_fragment_raw(
    section_xml: &str,
    header_xml: &str,
    after_para_idx: u32,
    fragment_xml: &str,
    source_char_prs: &str,
    source_para_prs: &str,
    source_styles: &str,
    source_border_fills: &str,
) -> Result<String, JsValue> {
    use crate::document_core::{paste_fragment_into_section, SourceDefinitions};
    let mut header_mut = header_xml.to_string();
    let source = SourceDefinitions {
        char_prs: source_char_prs.to_string(),
        para_prs: source_para_prs.to_string(),
        styles: source_styles.to_string(),
        border_fills: source_border_fills.to_string(),
    };
    let result = paste_fragment_into_section(
        section_xml,
        &mut header_mut,
        after_para_idx as usize,
        fragment_xml,
        &source,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut json = String::with_capacity(result.new_section_xml.len() + header_mut.len() + 256);
    json.push_str("{\"section_xml\":");
    push_paste_json_string(&mut json, &result.new_section_xml);
    json.push_str(",\"header_xml\":");
    push_paste_json_string(&mut json, &header_mut);
    json.push_str(",\"inserted_para_count\":");
    json.push_str(&result.inserted_para_count.to_string());
    json.push_str(",\"id_remap_char_pr\":");
    push_remap_json(&mut json, &result.id_remap.char_pr);
    json.push_str(",\"id_remap_para_pr\":");
    push_remap_json(&mut json, &result.id_remap.para_pr);
    json.push_str(",\"id_remap_style\":");
    push_remap_json(&mut json, &result.id_remap.style);
    json.push_str(",\"id_remap_border_fill\":");
    push_remap_json(&mut json, &result.id_remap.border_fill);
    json.push('}');
    Ok(json)
}

fn push_paste_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn push_remap_json(out: &mut String, remap: &std::collections::HashMap<u32, u32>) {
    out.push('{');
    let mut first = true;
    for (k, v) in remap {
        if !first {
            out.push(',');
        }
        first = false;
        out.push('"');
        out.push_str(&k.to_string());
        out.push_str("\":");
        out.push_str(&v.to_string());
    }
    out.push('}');
}
