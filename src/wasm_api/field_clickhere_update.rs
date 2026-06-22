//! 누름틀(ClickHere) 필드 속성 수정 API (WASM 바인딩).
//!
//! `HwpDocument`의 `#[wasm_bindgen]` 메서드 모음 (순수 이동, 동작 보존).

use wasm_bindgen::prelude::*;

use super::HwpDocument;

#[wasm_bindgen]
impl HwpDocument {
    /// 누름틀 필드의 속성을 수정한다.
    ///
    /// 반환: JSON `{"ok":true}` 또는 `{"ok":false}`
    #[wasm_bindgen(js_name = updateClickHereProps)]
    pub fn update_click_here_props(
        &mut self,
        field_id: u32,
        guide: &str,
        memo: &str,
        name: &str,
        editable: bool,
    ) -> String {
        use crate::model::control::{Control, Field, FieldType};

        let new_props_bit = if editable { 1u32 } else { 0u32 };

        // 필드를 찾아 수정하고, ctrl_data_records 바이너리도 갱신
        fn update_field_in_para(
            para: &mut crate::model::paragraph::Paragraph,
            field_id: u32,
            guide: &str,
            memo: &str,
            new_props_bit: u32,
            new_name: &str,
        ) -> bool {
            for (ci, ctrl) in para.controls.iter_mut().enumerate() {
                if let Control::Field(f) = ctrl {
                    if f.field_id == field_id && f.field_type == FieldType::ClickHere {
                        // guide/memo가 원본과 동일하면 command 문자열을 보존한다.
                        // 원본 command에는 trailing space 등이 포함될 수 있으므로
                        // 불필요한 재구축을 피해야 한컴 호환성이 유지된다.
                        let orig_guide = f.guide_text().unwrap_or("").to_string();
                        let orig_memo = f.memo_text().unwrap_or("").to_string();
                        if guide != orig_guide || memo != orig_memo {
                            // guide 또는 memo가 변경되었으므로 command 재구축
                            let new_command = Field::build_clickhere_command(guide, memo, "");
                            f.command = new_command;
                        }
                        // command가 변경되지 않았으면 원본 보존

                        f.properties = (f.properties & !1) | new_props_bit;
                        f.ctrl_data_name = if new_name.is_empty() {
                            None
                        } else {
                            Some(new_name.to_string())
                        };
                        // ctrl_data_records 바이너리 갱신
                        update_ctrl_data_name(&mut para.ctrl_data_records, ci, new_name);
                        return true;
                    }
                }
            }
            false
        }

        /// ctrl_data_records[ci]의 필드 이름 부분을 새 이름으로 재구축
        fn update_ctrl_data_name(records: &mut Vec<Option<Vec<u8>>>, ci: usize, new_name: &str) {
            // records 확장 (인덱스 부족 시)
            while records.len() <= ci {
                records.push(None);
            }
            if let Some(ref mut data) = records[ci] {
                if data.len() >= 12 {
                    // 헤더(10바이트) 보존, 이름 부분 재구축
                    let header = data[..10].to_vec();
                    let name_chars: Vec<u16> = new_name.encode_utf16().collect();
                    let name_len = name_chars.len() as u16;
                    let mut new_data = header;
                    new_data.extend_from_slice(&name_len.to_le_bytes());
                    for ch in &name_chars {
                        new_data.extend_from_slice(&ch.to_le_bytes());
                    }
                    *data = new_data;
                }
            } else {
                // CTRL_DATA가 없었던 경우: 새로 생성
                // 기본 헤더(10바이트) + 이름
                let name_chars: Vec<u16> = new_name.encode_utf16().collect();
                let name_len = name_chars.len() as u16;
                let mut data = vec![0x1Bu8, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x40, 0x01, 0x00];
                data.extend_from_slice(&name_len.to_le_bytes());
                for ch in &name_chars {
                    data.extend_from_slice(&ch.to_le_bytes());
                }
                records[ci] = Some(data);
            }
        }

        for sec in &mut self.document.sections {
            sec.raw_stream = None;
            for para in &mut sec.paragraphs {
                if update_field_in_para(para, field_id, guide, memo, new_props_bit, name) {
                    self.invalidate_page_tree_cache();
                    return r#"{"ok":true}"#.to_string();
                }
                // 표/글상자 내부
                for ctrl in &mut para.controls {
                    let found = match ctrl {
                        Control::Table(t) => t.cells.iter_mut().any(|c| {
                            c.paragraphs.iter_mut().any(|p| {
                                update_field_in_para(p, field_id, guide, memo, new_props_bit, name)
                            })
                        }),
                        Control::Shape(s) => {
                            if let Some(tb) = s.drawing_mut().and_then(|d| d.text_box.as_mut()) {
                                tb.paragraphs.iter_mut().any(|p| {
                                    update_field_in_para(
                                        p,
                                        field_id,
                                        guide,
                                        memo,
                                        new_props_bit,
                                        name,
                                    )
                                })
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };
                    if found {
                        self.invalidate_page_tree_cache();
                        return r#"{"ok":true}"#.to_string();
                    }
                }
            }
        }
        r#"{"ok":false}"#.to_string()
    }

}
