//! 문단 번호(Numbering)·글머리표(Bullet) 목록·생성 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 문서에 정의된 문단 번호(Numbering) 목록을 조회한다.
    ///
    /// 반환값: JSON 배열 [{ id, levelFormats: [...] }, ...]
    /// id는 1-based (ParaShape.numbering_id와 동일)
    #[wasm_bindgen(js_name = getNumberingList)]
    pub fn get_numbering_list(&self) -> String {
        let numberings = &self.core.document.doc_info.numberings;
        let mut items = Vec::new();
        for (i, n) in numberings.iter().enumerate() {
            let formats: Vec<String> = n
                .level_formats
                .iter()
                .map(|f| format!("\"{}\"", f.replace('"', "\\\"")))
                .collect();
            items.push(format!(
                "{{\"id\":{},\"levelFormats\":[{}],\"startNumber\":{}}}",
                i + 1,
                formats.join(","),
                n.start_number
            ));
        }
        format!("[{}]", items.join(","))
    }

    /// 문서에 정의된 글머리표(Bullet) 목록을 조회한다.
    ///
    /// 반환값: JSON 배열 [{ id, char }, ...]
    /// id는 1-based (ParaShape.numbering_id와 동일)
    #[wasm_bindgen(js_name = getBulletList)]
    pub fn get_bullet_list(&self) -> String {
        let bullets = &self.core.document.doc_info.bullets;
        let mut items = Vec::new();
        for (i, b) in bullets.iter().enumerate() {
            let mapped = crate::renderer::layout::map_pua_bullet_char(b.bullet_char);
            let raw_code = b.bullet_char as u32;
            items.push(format!(
                "{{\"id\":{},\"char\":\"{}\",\"rawCode\":{}}}",
                i + 1,
                mapped,
                raw_code
            ));
        }
        format!("[{}]", items.join(","))
    }

    /// 문서에 기본 문단 번호 정의가 없으면 생성한다.
    ///
    /// 반환값: Numbering ID (1-based)
    #[wasm_bindgen(js_name = ensureDefaultNumbering)]
    pub fn ensure_default_numbering(&mut self) -> u16 {
        let numberings = &self.core.document.doc_info.numberings;
        if !numberings.is_empty() {
            return 1; // 이미 있으면 첫 번째 반환
        }
        // 기본 7수준 번호 형식 생성 (한컴 기본 패턴)
        use crate::model::style::{Numbering, NumberingHead};
        let mut n = Numbering::default();
        n.level_formats = [
            "^1.".to_string(), // 1.
            "^2)".to_string(), // 가)
            "^3)".to_string(), // (1)
            "^4)".to_string(), // (가)
            "^5)".to_string(), // ①
            "^6)".to_string(), // ㄱ)
            "^7)".to_string(), // a)
        ];
        n.start_number = 1;
        n.level_start_numbers = [1; 7];
        // 수준별 번호 형식 코드 설정
        n.heads[0] = NumberingHead {
            number_format: 0,
            ..Default::default()
        }; // 1,2,3
        n.heads[1] = NumberingHead {
            number_format: 8,
            ..Default::default()
        }; // 가,나,다
        n.heads[2] = NumberingHead {
            number_format: 0,
            ..Default::default()
        }; // 1,2,3
        n.heads[3] = NumberingHead {
            number_format: 8,
            ..Default::default()
        }; // 가,나,다
        n.heads[4] = NumberingHead {
            number_format: 1,
            ..Default::default()
        }; // ①②③
        n.heads[5] = NumberingHead {
            number_format: 10,
            ..Default::default()
        }; // ㄱ,ㄴ,ㄷ
        n.heads[6] = NumberingHead {
            number_format: 5,
            ..Default::default()
        }; // a,b,c
        self.core.document.doc_info.numberings.push(n);
        1
    }

    /// JSON으로 지정된 번호 형식으로 Numbering 정의를 생성한다.
    ///
    /// json: {"levelFormats":["^1.","^2)",...],"numberFormats":[0,8,...],"startNumber":1}
    /// 반환값: Numbering ID (1-based)
    #[wasm_bindgen(js_name = createNumbering)]
    pub fn create_numbering(&mut self, json: &str) -> u16 {
        use crate::document_core::helpers::json_i32;
        use crate::model::style::{Numbering, NumberingHead};

        let mut n = Numbering::default();

        // levelFormats 배열 파싱
        if let Some(arr_start) = json.find("\"levelFormats\"") {
            let rest = &json[arr_start..];
            if let Some(bracket_start) = rest.find('[') {
                if let Some(bracket_end) = rest[bracket_start..].find(']') {
                    let arr_str = &rest[bracket_start + 1..bracket_start + bracket_end];
                    let mut level = 0;
                    for part in arr_str.split(',') {
                        if level >= 7 {
                            break;
                        }
                        let trimmed = part.trim().trim_matches('"');
                        if !trimmed.is_empty() {
                            n.level_formats[level] = trimmed.to_string();
                            level += 1;
                        }
                    }
                }
            }
        }

        // numberFormats 배열 파싱
        if let Some(arr_start) = json.find("\"numberFormats\"") {
            let rest = &json[arr_start..];
            if let Some(bracket_start) = rest.find('[') {
                if let Some(bracket_end) = rest[bracket_start..].find(']') {
                    let arr_str = &rest[bracket_start + 1..bracket_start + bracket_end];
                    let mut level = 0;
                    for part in arr_str.split(',') {
                        if level >= 7 {
                            break;
                        }
                        if let Ok(code) = part.trim().parse::<u8>() {
                            n.heads[level] = NumberingHead {
                                number_format: code,
                                ..Default::default()
                            };
                            level += 1;
                        }
                    }
                }
            }
        }

        n.start_number = json_i32(json, "startNumber").unwrap_or(1) as u16;
        n.level_start_numbers = [n.start_number as u32; 7];
        self.core.document.doc_info.numberings.push(n);
        self.core.document.doc_info.numberings.len() as u16
    }

    /// 특정 문자의 글머리표 정의가 없으면 생성한다.
    ///
    /// 반환값: Bullet ID (1-based)
    #[wasm_bindgen(js_name = ensureDefaultBullet)]
    pub fn ensure_default_bullet(&mut self, bullet_char_str: &str) -> u16 {
        let bullet_ch = bullet_char_str.chars().next().unwrap_or('●');
        // 이미 해당 문자의 Bullet이 있는지 검색
        let bullets = &self.core.document.doc_info.bullets;
        for (i, b) in bullets.iter().enumerate() {
            let mapped = crate::renderer::layout::map_pua_bullet_char(b.bullet_char);
            if mapped == bullet_ch {
                return (i + 1) as u16;
            }
        }
        // 없으면 새로 생성
        use crate::model::style::Bullet;
        let b = Bullet {
            bullet_char: bullet_ch,
            text_distance: 50,
            ..Default::default()
        };
        self.core.document.doc_info.bullets.push(b);
        self.core.document.doc_info.bullets.len() as u16
    }

}
