//! 필드 조회/설정 API (Task 230)
//!
//! 문서 전체에서 필드를 재귀 탐색하여 조회·설정하는 기능을 제공한다.

use crate::document_core::DocumentCore;
use crate::model::control::{Control, Field, FieldType};
use crate::model::paragraph::Paragraph;
use crate::error::HwpError;

/// 필드 위치 정보
#[derive(Debug, Clone)]
pub struct FieldLocation {
    pub section_index: usize,
    pub para_index: usize,
    /// 표/글상자 내 필드인 경우 중첩 경로
    pub nested_path: Vec<NestedEntry>,
}

/// 중첩 경로 항목 (표 셀 또는 글상자 내부)
#[derive(Debug, Clone)]
pub enum NestedEntry {
    /// 표 셀: (control_index, cell_index, para_index)
    TableCell { control_index: usize, cell_index: usize, para_index: usize },
    /// 글상자: (control_index, para_index)
    TextBox { control_index: usize, para_index: usize },
}

mod virtual_cell_id;
use virtual_cell_id::{resolve_virtual_field_id_collisions, virtual_cell_field_id};

/// 필드 검색 결과
#[derive(Debug)]
pub struct FieldInfo {
    pub field: Field,
    pub location: FieldLocation,
    /// 필드 범위 내 텍스트 (빈 필드이면 빈 문자열)
    pub value: String,
    /// field_ranges에서의 인덱스
    pub field_range_index: usize,
    /// 실제 Field 컨트롤이 아니라 표 셀 자체의 field_name에서 합성한 값인가.
    /// `field.ctrl_id == 0`은 이 구분에 쓸 수 없다: HWP3/HWPX 파서가
    /// `Field::default()`로 만드는 실제 필드(메일머지, 색인 표시, HWPX
    /// FIELD_BEGIN 등)도 ctrl_id를 명시적으로 채우지 않아 그대로 0이다.
    pub is_virtual_cell_field: bool,
}

impl DocumentCore {
    /// 문서 전체에서 모든 필드를 검색하여 목록으로 반환한다.
    pub fn collect_all_fields(&self) -> Vec<FieldInfo> {
        let mut result = Vec::new();
        for (si, sec) in self.document.sections.iter().enumerate() {
            for (pi, para) in sec.paragraphs.iter().enumerate() {
                let loc = FieldLocation {
                    section_index: si,
                    para_index: pi,
                    nested_path: Vec::new(),
                };
                collect_fields_from_paragraph(para, &loc, &mut result);
            }
        }
        resolve_virtual_field_id_collisions(&mut result);
        result
    }

    /// getFieldList: 모든 필드를 JSON 배열로 반환
    pub fn get_field_list_json(&self) -> String {
        let fields = self.collect_all_fields();
        let entries: Vec<String> = fields.iter().map(|fi| {
            let name = fi.field.field_name().unwrap_or("");
            let guide = fi.field.guide_text().unwrap_or("");
            let location_json = field_location_json(&fi.location);
            format!(
                "{{\"fieldId\":{},\"fieldType\":\"{}\",\"name\":{},\"guide\":{},\"command\":{},\"value\":{},\"location\":{}}}",
                fi.field.field_id,
                fi.field.field_type_str(),
                json_escape(name),
                json_escape(guide),
                json_escape(&fi.field.command),
                json_escape(&fi.value),
                location_json,
            )
        }).collect();
        format!("[{}]", entries.join(","))
    }

    /// getFieldValue: field_id로 필드 값 조회
    pub fn get_field_value_by_id(&self, field_id: u32) -> Result<String, HwpError> {
        let fields = self.collect_all_fields();
        let fi = find_field_by_id(&fields, field_id)?;
        Ok(format!("{{\"ok\":true,\"value\":{}}}", json_escape(&fi.value)))
    }

    /// getFieldValueByName: 필드 이름으로 값 조회
    pub fn get_field_value_by_name(&self, name: &str) -> Result<String, HwpError> {
        let fields = self.collect_all_fields();
        for fi in &fields {
            if let Some(field_name) = fi.field.field_name() {
                if field_name == name {
                    return Ok(format!(
                        "{{\"ok\":true,\"fieldId\":{},\"value\":{}}}",
                        fi.field.field_id,
                        json_escape(&fi.value),
                    ));
                }
            }
        }
        Err(HwpError::InvalidField(format!("필드 이름 '{}' 없음", name)))
    }

    /// setFieldValue: field_id로 필드 값 설정
    pub fn set_field_value_by_id(&mut self, field_id: u32, value: &str) -> Result<String, HwpError> {
        // 먼저 필드 위치 찾기
        let fields = self.collect_all_fields();
        let fi = find_field_by_id(&fields, field_id)?;

        let location = fi.location.clone();
        let fri = fi.field_range_index;
        let old_value = fi.value.clone();
        let is_cell_field = fi.is_virtual_cell_field;

        let section_index = location.section_index;
        if is_cell_field {
            // 가상 셀 필드: 이 위치에는 실제 FIELD_BEGIN/END로 만들어진
            // field_ranges가 없다 (셀의 field_name에서 합성한 값일 뿐이다).
            // 그런데도 아래처럼 일반 field_ranges 경로(set_field_text_at)로
            // 처리하면, 우연히 이 문단(nested_path가 가리키는 셀 문단)에
            // 존재하는 *다른* field_range를 field_range_index=0 으로 잘못
            // 골라 그 필드를 덮어쓰거나, field_ranges가 아예 없으면
            // "field_range 인덱스 초과" 오류로 실패한다. 셀의 첫 문단
            // 텍스트를 직접 교체하는 set_cell_field_text를 써야 한다
            // (set_field_value_by_name이 이미 이렇게 분기하고 있었다).
            self.set_cell_field_text(&location, value)?;
            if let Some(sec) = self.document.sections.get_mut(section_index) {
                sec.raw_stream = None;
            }
        } else {
            self.set_field_text_at(&location, fri, value)?;
        }
        self.recompose_section(section_index);

        Ok(format!(
            "{{\"ok\":true,\"fieldId\":{},\"oldValue\":{},\"newValue\":{}}}",
            field_id,
            json_escape(&old_value),
            json_escape(value),
        ))
    }

    /// setFieldValueByName: 필드 이름으로 값 설정
    pub fn set_field_value_by_name(&mut self, name: &str, value: &str) -> Result<String, HwpError> {
        let fields = self.collect_all_fields();
        let fi = fields.iter().find(|f| {
            f.field.field_name().map(|n| n == name).unwrap_or(false)
        }).ok_or_else(|| HwpError::InvalidField(format!("필드 이름 '{}' 없음", name)))?;

        let field_id = fi.field.field_id;
        let location = fi.location.clone();
        let fri = fi.field_range_index;
        let old_value = fi.value.clone();
        let is_cell_field = fi.is_virtual_cell_field;

        let section_index = location.section_index;

        if is_cell_field {
            // 셀 필드: 셀의 첫 문단 텍스트를 직접 교체
            self.set_cell_field_text(&location, value)?;
        } else {
            // ClickHere 필드: field_ranges 기반 교체
            self.set_field_text_at(&location, fri, value)?;
        }

        // raw_stream 무효화
        if let Some(sec) = self.document.sections.get_mut(section_index) {
            sec.raw_stream = None;
        }
        self.recompose_section(section_index);

        Ok(format!(
            "{{\"ok\":true,\"fieldId\":{},\"oldValue\":{},\"newValue\":{}}}",
            field_id,
            json_escape(&old_value),
            json_escape(value),
        ))
    }

    /// 셀 필드의 텍스트를 교체한다 (셀의 첫 문단 텍스트를 value로 대체).
    /// 중첩 표를 재귀적으로 탐색하여 임의 깊이를 지원한다.
    fn set_cell_field_text(&mut self, location: &FieldLocation, value: &str) -> Result<(), HwpError> {
        if location.nested_path.is_empty() {
            return Err(HwpError::InvalidField("셀 필드 위치에 중첩 경로 없음".into()));
        }
        let sec = self.document.sections.get_mut(location.section_index)
            .ok_or_else(|| HwpError::InvalidField("구역 초과".into()))?;
        let mut para: &mut Paragraph = sec.paragraphs.get_mut(location.para_index)
            .ok_or_else(|| HwpError::InvalidField("문단 초과".into()))?;

        // 마지막 항목 직전까지 중첩 탐색
        for (i, entry) in location.nested_path[..location.nested_path.len() - 1].iter().enumerate() {
            para = match entry {
                NestedEntry::TableCell { control_index, cell_index, para_index } => {
                    let ctrl = para.controls.get_mut(*control_index)
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 컨트롤 인덱스 {} 초과", i, control_index)))?;
                    if let Control::Table(ref mut table) = ctrl {
                        let cell = table.cells.get_mut(*cell_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 셀 인덱스 {} 초과", i, cell_index)))?;
                        cell.paragraphs.get_mut(*para_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 셀 문단 인덱스 {} 초과", i, para_index)))?
                    } else {
                        return Err(HwpError::InvalidField(
                            format!("경로[{}]: controls[{}]가 Table이 아님", i, control_index)));
                    }
                }
                NestedEntry::TextBox { control_index, para_index } => {
                    let ctrl = para.controls.get_mut(*control_index)
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 컨트롤 인덱스 {} 초과", i, control_index)))?;
                    if let Control::Shape(ref mut shape) = ctrl {
                        let drawing = shape.drawing_mut()
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: Shape에 DrawingObjAttr 없음", i)))?;
                        let tb = drawing.text_box.as_mut()
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: Shape에 TextBox 없음", i)))?;
                        tb.paragraphs.get_mut(*para_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 글상자 문단 인덱스 {} 초과", i, para_index)))?
                    } else {
                        return Err(HwpError::InvalidField(
                            format!("경로[{}]: controls[{}]가 Shape가 아님", i, control_index)));
                    }
                }
            };
        }

        // 마지막 항목: 셀의 첫 문단 텍스트를 교체
        let last_idx = location.nested_path.len() - 1;
        let last = location.nested_path.last().unwrap();
        match last {
            NestedEntry::TableCell { control_index, cell_index, .. } => {
                let ctrl = para.controls.get_mut(*control_index)
                    .ok_or_else(|| HwpError::InvalidField(
                        format!("경로[{}]: 컨트롤 인덱스 {} 초과", last_idx, control_index)))?;
                if let Control::Table(ref mut table) = ctrl {
                    let cell = table.cells.get_mut(*cell_index)
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 셀 인덱스 {} 초과", last_idx, cell_index)))?;
                    let cell_para = cell.paragraphs.first_mut()
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 셀 {}에 문단이 없어 값을 쓸 수 없음", last_idx, cell_index)))?;
                    cell_para.text = value.to_string();
                    rebuild_char_offsets(cell_para);
                    Ok(())
                } else {
                    Err(HwpError::InvalidField(
                        format!("경로[{}]: controls[{}]가 Table이 아님", last_idx, control_index)))
                }
            }
            _ => Err(HwpError::InvalidField("셀 필드가 아닌 위치".into())),
        }
    }

    /// 필드 위치에서 텍스트를 교체한다.
    fn set_field_text_at(&mut self, location: &FieldLocation, field_range_index: usize, value: &str) -> Result<(), HwpError> {
        let para = self.get_para_mut_at_location(location)?;
        let fr = para.field_ranges.get(field_range_index)
            .ok_or_else(|| HwpError::InvalidField("field_range 인덱스 초과".into()))?
            .clone();

        // 필드 범위 내 텍스트 교체
        let text_chars: Vec<char> = para.text.chars().collect();
        // field_ranges는 파일에서 파싱된 값을 그대로 담고 있어 신뢰할 수 없다
        // (손상되었거나 조작된 문서일 수 있다). start/end가 실제 텍스트 길이를
        // 벗어나거나 start > end이면 아래 슬라이싱(`text_chars[..start]`,
        // `text_chars[end..]`)이 범위를 벗어나 panic한다 -- 여기서 먼저
        // 검증해 잘못된 입력을 명확한 오류로 바꾼다. 이 검증(그리고 위
        // field_range 인덱스 조회)은 반드시 raw_stream을 지우기 *전에*
        // 끝나야 한다: 여기서 실패해 Err를 반환하면 모델은 전혀 바뀌지
        // 않는데, raw_stream만 먼저 지워버리면 다음 저장 시 (변경되지
        // 않은) 모델에서 다시 직렬화하게 되어 원본 바이트가 아닌, 손실
        // 가능성이 있는 재직렬화 결과로 대체되어 버린다.
        if fr.start_char_idx > fr.end_char_idx || fr.end_char_idx > text_chars.len() {
            return Err(HwpError::InvalidField(format!(
                "field_range 범위가 유효하지 않음: start={}, end={}, 텍스트 길이={}",
                fr.start_char_idx, fr.end_char_idx, text_chars.len()
            )));
        }

        // raw_stream 무효화: 여기부터는 실제로 모델을 변경하므로, 직렬화 시
        // 수정된 모델을 사용하도록 강제한다.
        if let Some(sec) = self.document.sections.get_mut(location.section_index) {
            sec.raw_stream = None;
        }
        let para = self.get_para_mut_at_location(location)?;
        let before: String = text_chars[..fr.start_char_idx].iter().collect();
        let after: String = text_chars[fr.end_char_idx..].iter().collect();
        para.text = format!("{}{}{}", before, value, after);

        // field_range 업데이트
        let new_end = fr.start_char_idx + value.chars().count();
        let delta = new_end as isize - fr.end_char_idx as isize;

        // 현재 필드 범위 업데이트
        if let Some(current_fr) = para.field_ranges.get_mut(field_range_index) {
            current_fr.end_char_idx = new_end;
        }

        // 이후/외부 필드 범위들의 위치 조정.
        //
        // "외부(포함하는) 필드"인지 판정할 때 경계값(start/end)만 보면 실제 중첩과
        // "경계가 우연히 맞닿은, 관계없는 형제 필드"를 구분할 수 없다. 두 가지
        // 대칭적인 오판 사례가 있다:
        //   (a) 이미 닫힌 형제 필드 [0,4) 바로 뒤에 빈 대상 필드 [4,4)가 있을 때,
        //       `other.start <= fr.start && other.end >= fr.end` 만으로는 앞선
        //       형제 필드가 fr을 "포함한다"고 오판한다.
        //   (b) fr이 자기 부모의 맨 앞에서 시작하는 빈 필드([p,p))일 때 — 예:
        //       부모 [0,2) 안에 자식 [0,0) — 부모의 start(0)도 fr.end(0)와 같아서
        //       "교체 구간 뒤에 완전히 위치한 필드"(other.start >= fr.end) 조건에도
        //       걸려버려, 진짜 부모인데도 시작 위치까지 함께 밀려나 버린다.
        //
        // 두 오판 모두 파서(parser/body_text.rs의 field_stack)가 이미 암묵적으로
        // 남겨 둔 두 가지 순서 정보로 구조적으로 해결할 수 있다. 진짜 중첩은
        // "부모가 먼저 열리고 나중에 닫힌다"는 것과 동치이고, 이 열림/닫힘 순서는
        // FieldRange 두 필드에 각각 그대로 남아 있다:
        //   - 닫힌 순서 = field_ranges 배열의 인덱스. FIELD_BEGIN/FIELD_END는
        //     스택(LIFO)으로 처리되므로 자식은 항상 부모보다 *먼저* 닫히고,
        //     따라서 field_ranges 배열에도 항상 부모보다 **더 작은 인덱스**로
        //     먼저 push된다 → 진짜 부모라면 other의 배열 인덱스(i)가 fr의 인덱스
        //     (field_range_index)보다 커야 한다.
        //   - 열린 순서 = control_idx (controls[] 안에서의 위치, FIELD_BEGIN을
        //     만날 때마다 순서대로 배정됨). 부모는 자식보다 먼저 열리므로 항상
        //     더 작은 control_idx를 가진다 → 진짜 부모라면 other의 control_idx가
        //     fr의 control_idx보다 작아야 한다.
        // 형제 관계에서는 이 두 순서 중 최소 하나가 반대로 뒤집힌다 — 앞선 형제는
        // 배열 인덱스가 더 작고(닫힘이 더 빠름), 뒤따르는 형제는 control_idx가 더
        // 크다(열림이 더 늦음) — 이므로 두 조건을 모두 요구하면 (a), (b) 두
        // 오판 사례를 모두 구조적으로 배제할 수 있다.
        for (i, other_fr) in para.field_ranges.iter_mut().enumerate() {
            if i == field_range_index {
                continue;
            }
            let is_genuine_parent = i > field_range_index
                && other_fr.control_idx < fr.control_idx
                && other_fr.start_char_idx <= fr.start_char_idx
                && other_fr.end_char_idx >= fr.end_char_idx;
            if is_genuine_parent {
                // 진짜 부모 필드(fr보다 먼저 열리고 나중에 닫힌, 경계상으로도
                // fr을 포함하는 필드)는 끝 위치만 delta만큼 보정한다. (수정 전에는
                // 이 보정이 아예 누락되어 외부 필드의 end_char_idx가 교체 후 텍스트
                // 길이를 초과한 채로 남아, 다음 슬라이스에서 panic으로 이어졌다.)
                other_fr.end_char_idx = (other_fr.end_char_idx as isize + delta) as usize;
            } else if other_fr.start_char_idx >= fr.end_char_idx {
                // 교체 구간 뒤에 완전히 위치한 필드(진짜 부모가 아닌 것으로 이미
                // 확인됨): 시작·끝 모두 이동
                other_fr.start_char_idx = (other_fr.start_char_idx as isize + delta) as usize;
                other_fr.end_char_idx = (other_fr.end_char_idx as isize + delta) as usize;
            }
            // 그 외 (경계에 걸쳐 부분적으로만 겹치는 필드)는 조정하지 않고 기존
            // 동작(변경 없음)을 유지한다.
        }

        // char_offsets 재생성: 컨트롤 문자(8 code unit)와 일반 문자(1~2 code unit) 반영
        rebuild_char_offsets(para);

        Ok(())
    }

    /// FieldLocation에 해당하는 Paragraph의 가변 참조를 반환한다.
    ///
    /// 중첩 표/글상자를 재귀적으로 탐색하여 임의 깊이를 지원한다.
    fn get_para_mut_at_location(&mut self, location: &FieldLocation) -> Result<&mut Paragraph, HwpError> {
        let sec = self.document.sections.get_mut(location.section_index)
            .ok_or_else(|| HwpError::InvalidField("구역 인덱스 초과".into()))?;
        let mut para = sec.paragraphs.get_mut(location.para_index)
            .ok_or_else(|| HwpError::InvalidField("문단 인덱스 초과".into()))?;

        for (i, entry) in location.nested_path.iter().enumerate() {
            para = match entry {
                NestedEntry::TableCell { control_index, cell_index, para_index } => {
                    let ctrl = para.controls.get_mut(*control_index)
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 컨트롤 인덱스 {} 초과", i, control_index)))?;
                    if let Control::Table(ref mut table) = ctrl {
                        let cell = table.cells.get_mut(*cell_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 셀 인덱스 {} 초과", i, cell_index)))?;
                        cell.paragraphs.get_mut(*para_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 셀 문단 인덱스 {} 초과", i, para_index)))?
                    } else {
                        return Err(HwpError::InvalidField(
                            format!("경로[{}]: controls[{}]가 Table이 아님", i, control_index)));
                    }
                }
                NestedEntry::TextBox { control_index, para_index } => {
                    let ctrl = para.controls.get_mut(*control_index)
                        .ok_or_else(|| HwpError::InvalidField(
                            format!("경로[{}]: 컨트롤 인덱스 {} 초과", i, control_index)))?;
                    if let Control::Shape(ref mut shape) = ctrl {
                        let drawing = shape.drawing_mut()
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: Shape에 DrawingObjAttr 없음", i)))?;
                        let tb = drawing.text_box.as_mut()
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: Shape에 TextBox 없음", i)))?;
                        tb.paragraphs.get_mut(*para_index)
                            .ok_or_else(|| HwpError::InvalidField(
                                format!("경로[{}]: 글상자 문단 인덱스 {} 초과", i, para_index)))?
                    } else {
                        return Err(HwpError::InvalidField(
                            format!("경로[{}]: controls[{}]가 Shape가 아님", i, control_index)));
                    }
                }
            };
        }

        Ok(para)
    }

    /// 본문 문단의 커서 위치에서 필드를 제거한다 (텍스트 유지, 필드 마커만 삭제).
    ///
    /// 성공 시 `{"ok":true}`, 필드가 없으면 에러를 반환한다.
    pub fn remove_field_at(&mut self, section_idx: usize, para_idx: usize, char_offset: usize) -> Result<String, HwpError> {
        let para = self.document.sections.get_mut(section_idx)
            .and_then(|s| s.paragraphs.get_mut(para_idx))
            .ok_or_else(|| HwpError::InvalidField("문단 위치 초과".into()))?;
        remove_field_in_para(para, char_offset)?;
        // raw_stream 무효화: 원본 스트림이 남아있으면 serialize_section이
        // 모델 변경을 무시하고 그 원본을 그대로 반환하므로, 여기서 지우지
        // 않으면 방금 제거한 필드가 저장 시 다시 살아난다.
        if let Some(sec) = self.document.sections.get_mut(section_idx) {
            sec.raw_stream = None;
        }
        self.recompose_section(section_idx);
        Ok(r#"{"ok":true}"#.to_string())
    }

    /// 셀/글상자 내 문단의 커서 위치에서 필드를 제거한다.
    pub fn remove_field_at_in_cell(
        &mut self,
        section_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        char_offset: usize,
        is_textbox: bool,
    ) -> Result<String, HwpError> {
        let para = {
            let host = self.document.sections.get_mut(section_idx)
                .and_then(|s| s.paragraphs.get_mut(parent_para_idx))
                .ok_or_else(|| HwpError::InvalidField("호스트 문단 위치 초과".into()))?;
            let ctrl = host.controls.get_mut(control_idx)
                .ok_or_else(|| HwpError::InvalidField("컨트롤 인덱스 초과".into()))?;
            if is_textbox {
                if let Control::Shape(shape) = ctrl {
                    let drawing = shape.drawing_mut()
                        .ok_or_else(|| HwpError::InvalidField("Shape에 DrawingObjAttr 없음".into()))?;
                    let tb = drawing.text_box.as_mut()
                        .ok_or_else(|| HwpError::InvalidField("Shape에 TextBox 없음".into()))?;
                    tb.paragraphs.get_mut(cell_para_idx)
                        .ok_or_else(|| HwpError::InvalidField("글상자 문단 인덱스 초과".into()))?
                } else {
                    return Err(HwpError::InvalidField("예상된 Shape 컨트롤이 아님".into()));
                }
            } else {
                if let Control::Table(table) = ctrl {
                    let cell = table.cells.get_mut(cell_idx)
                        .ok_or_else(|| HwpError::InvalidField("셀 인덱스 초과".into()))?;
                    cell.paragraphs.get_mut(cell_para_idx)
                        .ok_or_else(|| HwpError::InvalidField("셀 문단 인덱스 초과".into()))?
                } else {
                    return Err(HwpError::InvalidField("예상된 Table 컨트롤이 아님".into()));
                }
            }
        };
        remove_field_in_para(para, char_offset)?;
        // raw_stream 무효화: remove_field_at와 같은 이유로, 원본 스트림이
        // 남아있으면 이 제거가 직렬화 시 유실된다.
        if let Some(sec) = self.document.sections.get_mut(section_idx) {
            sec.raw_stream = None;
        }
        self.recompose_section(section_idx);
        Ok(r#"{"ok":true}"#.to_string())
    }

    /// 커서가 진입한 활성 필드를 설정한다 (안내문 렌더링 스킵용).
    ///
    /// 본문 문단: `set_active_field(sec, para, char_offset)`
    /// 설정 후 해당 페이지의 렌더 트리 캐시를 무효화한다.
    /// 활성 필드를 설정한다. 변경이 발생하면 true를 반환한다.
    pub fn set_active_field(&mut self, section_idx: usize, para_idx: usize, char_offset: usize) -> bool {
        use super::super::ActiveFieldInfo;
        let ctrl_idx = self.find_field_control_idx(section_idx, para_idx, char_offset, None);
        if let Some(ci) = ctrl_idx {
            let new_info = ActiveFieldInfo {
                section_idx, para_idx, control_idx: ci, cell_path: None,
            };
            if self.active_field.as_ref() != Some(&new_info) {
                self.active_field = Some(new_info);
                self.invalidate_page_tree_cache();
                return true;
            }
        }
        false
    }

    /// 셀/글상자 내 활성 필드를 설정한다. 변경이 발생하면 true를 반환한다.
    pub fn set_active_field_in_cell(
        &mut self, section_idx: usize, parent_para_idx: usize, control_idx: usize,
        cell_idx: usize, cell_para_idx: usize, char_offset: usize, is_textbox: bool,
    ) -> bool {
        use super::super::ActiveFieldInfo;
        let cell_path = Some(vec![(control_idx, cell_idx, cell_para_idx)]);
        let ctrl_idx = self.find_field_control_idx_in_cell(
            section_idx, parent_para_idx, control_idx, cell_idx, cell_para_idx, char_offset, is_textbox,
        );
        if let Some(ci) = ctrl_idx {
            let new_info = ActiveFieldInfo {
                section_idx, para_idx: cell_para_idx, control_idx: ci, cell_path,
            };
            if self.active_field.as_ref() != Some(&new_info) {
                self.active_field = Some(new_info);
                self.invalidate_page_tree_cache();
                return true;
            }
        }
        false
    }

    /// 활성 필드를 해제한다.
    pub fn clear_active_field(&mut self) {
        if self.active_field.is_some() {
            self.active_field = None;
            self.invalidate_page_tree_cache();
        }
    }

    /// 본문 문단의 커서 위치에서 필드 범위 정보를 조회한다.
    ///
    /// 커서가 필드 범위 내에 있으면 필드 정보를 JSON으로 반환하고,
    /// 필드 밖이면 `{"inField":false}`를 반환한다.
    pub fn get_field_info_at(&self, section_idx: usize, para_idx: usize, char_offset: usize) -> String {
        let para = match self.document.sections.get(section_idx)
            .and_then(|s| s.paragraphs.get(para_idx))
        {
            Some(p) => p,
            None => return r#"{"inField":false}"#.to_string(),
        };
        field_info_at_in_para(para, char_offset)
    }

    /// 셀/글상자 내 문단의 커서 위치에서 필드 범위 정보를 조회한다.
    pub fn get_field_info_at_in_cell(
        &self,
        section_idx: usize,
        parent_para_idx: usize,
        control_idx: usize,
        cell_idx: usize,
        cell_para_idx: usize,
        char_offset: usize,
        is_textbox: bool,
    ) -> String {
        let para = (|| {
            let host = self.document.sections.get(section_idx)?
                .paragraphs.get(parent_para_idx)?;
            let ctrl = host.controls.get(control_idx)?;
            if is_textbox {
                if let Control::Shape(shape) = ctrl {
                    let tb = shape.drawing()?.text_box.as_ref()?;
                    return tb.paragraphs.get(cell_para_idx);
                }
            } else {
                if let Control::Table(table) = ctrl {
                    let cell = table.cells.get(cell_idx)?;
                    return cell.paragraphs.get(cell_para_idx);
                }
            }
            None
        })();
        match para {
            Some(p) => field_info_at_in_para(p, char_offset),
            None => r#"{"inField":false}"#.to_string(),
        }
    }

    /// path 기반: 중첩 표 셀의 필드 범위 정보를 조회한다.
    pub fn get_field_info_at_by_path(
        &self, section_idx: usize, parent_para_idx: usize,
        path: &[(usize, usize, usize)], char_offset: usize,
    ) -> String {
        match self.resolve_paragraph_by_path(section_idx, parent_para_idx, path) {
            Ok(para) => field_info_at_in_para(para, char_offset),
            Err(_) => r#"{"inField":false}"#.to_string(),
        }
    }

    /// path 기반: 중첩 표 셀 내 활성 필드를 설정한다.
    pub fn set_active_field_by_path(
        &mut self, section_idx: usize, parent_para_idx: usize,
        path: &[(usize, usize, usize)], char_offset: usize,
    ) -> bool {
        use super::super::ActiveFieldInfo;
        let para = match self.resolve_paragraph_by_path(section_idx, parent_para_idx, path) {
            Ok(p) => p,
            Err(_) => return false,
        };
        let ctrl_idx = find_field_ctrl_idx_in_para(para, char_offset);
        if let Some(ci) = ctrl_idx {
            let last = path.last().unwrap();
            let cell_para_idx = last.2;
            // cell_path: 전체 path를 저장 (중첩 표 구분용)
            let cell_path = Some(path.to_vec());
            let new_info = ActiveFieldInfo {
                section_idx, para_idx: cell_para_idx, control_idx: ci, cell_path,
            };
            if self.active_field.as_ref() != Some(&new_info) {
                self.active_field = Some(new_info);
                self.invalidate_page_tree_cache();
                return true;
            }
        }
        false
    }
}

/// field_id로 필드를 찾는다. 정확히 하나만 일치해야 한다: 둘 이상 일치하면
/// (해시 충돌 또는 조작된 문서로 인한 잔여 위험) 첫 항목을 임의로 반환하는
/// 대신 명확한 오류를 반환한다 -- 어느 위치의 값을 반환할지 알 수 없는
/// 상태에서 조용히 잘못된 값을 돌려주는 것보다 안전하다.
fn find_field_by_id(fields: &[FieldInfo], field_id: u32) -> Result<&FieldInfo, HwpError> {
    let mut matches = fields.iter().filter(|fi| fi.field.field_id == field_id);
    let first = matches
        .next()
        .ok_or_else(|| HwpError::InvalidField(format!("필드 ID {} 없음", field_id)))?;
    if matches.next().is_some() {
        return Err(HwpError::InvalidField(format!(
            "필드 ID {} 가 둘 이상의 위치와 일치함 (내부 ID 충돌)",
            field_id
        )));
    }
    Ok(first)
}

/// 문단 내 커서 위치의 필드 범위 정보를 JSON으로 반환한다.
fn field_info_at_in_para(para: &Paragraph, char_offset: usize) -> String {
    for fr in &para.field_ranges {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            if field.field_type != FieldType::ClickHere {
                continue;
            }
            // 커서가 필드 범위 내에 있는지 확인 (start 이상, end 이하)
            // end가 exclusive이므로 커서가 end 위치에 있으면 필드 "끝"에 있는 것
            if char_offset >= fr.start_char_idx && char_offset <= fr.end_char_idx {
                let is_guide = fr.start_char_idx == fr.end_char_idx;
                let guide = field.guide_text().unwrap_or("");
                return format!(
                    "{{\"inField\":true,\"fieldId\":{},\"fieldType\":\"{}\",\"startCharIdx\":{},\"endCharIdx\":{},\"isGuide\":{},\"guideName\":{}}}",
                    field.field_id,
                    field.field_type_str(),
                    fr.start_char_idx,
                    fr.end_char_idx,
                    is_guide,
                    json_escape(guide),
                );
            }
        }
    }
    r#"{"inField":false}"#.to_string()
}

/// 문단에서 필드를 수집한다 (재귀: 표 셀, 글상자 내부 포함).
fn collect_fields_from_paragraph(
    para: &Paragraph,
    base_location: &FieldLocation,
    result: &mut Vec<FieldInfo>,
) {
    // 현재 문단의 field_ranges에서 필드 수집
    for (fri, fr) in para.field_ranges.iter().enumerate() {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            let value = if fr.start_char_idx < fr.end_char_idx {
                let chars: Vec<char> = para.text.chars().collect();
                if fr.end_char_idx <= chars.len() {
                    chars[fr.start_char_idx..fr.end_char_idx].iter().collect()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            result.push(FieldInfo {
                field: field.clone(),
                location: base_location.clone(),
                value,
                field_range_index: fri,
                is_virtual_cell_field: false,
            });
        }
    }

    // 컨트롤 내부 재귀 탐색 (표 셀, 글상자)
    for (ci, ctrl) in para.controls.iter().enumerate() {
        match ctrl {
            Control::Table(table) => {
                for (cell_i, cell) in table.cells.iter().enumerate() {
                    // 셀 자체의 field_name이 있으면 가상 필드로 추가
                    if let Some(ref fname) = cell.field_name {
                        let mut loc = base_location.clone();
                        loc.nested_path.push(NestedEntry::TableCell {
                            control_index: ci,
                            cell_index: cell_i,
                            para_index: 0,
                        });
                        // 셀의 첫 문단 텍스트를 값으로 사용
                        let value = cell.paragraphs.first()
                            .map(|p| p.text.clone())
                            .unwrap_or_default();
                        result.push(FieldInfo {
                            field: Field {
                                ctrl_id: 0,
                                field_id: virtual_cell_field_id(&loc),
                                field_type: FieldType::ClickHere,
                                command: String::new(),
                                properties: 0,
                                extra_properties: 0,
                                ctrl_data_name: Some(fname.clone()),
                                memo_index: 0,
                            },
                            location: loc,
                            value,
                            field_range_index: 0,
                            is_virtual_cell_field: true,
                        });
                    }
                    for (pi, cell_para) in cell.paragraphs.iter().enumerate() {
                        let mut loc = base_location.clone();
                        loc.nested_path.push(NestedEntry::TableCell {
                            control_index: ci,
                            cell_index: cell_i,
                            para_index: pi,
                        });
                        collect_fields_from_paragraph(cell_para, &loc, result);
                    }
                }
            }
            Control::Shape(shape) => {
                if let Some(drawing) = shape.drawing() {
                    if let Some(tb) = &drawing.text_box {
                        for (pi, tb_para) in tb.paragraphs.iter().enumerate() {
                            let mut loc = base_location.clone();
                            loc.nested_path.push(NestedEntry::TextBox {
                                control_index: ci,
                                para_index: pi,
                            });
                            collect_fields_from_paragraph(tb_para, &loc, result);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// FieldLocation을 JSON으로 변환
fn field_location_json(loc: &FieldLocation) -> String {
    if loc.nested_path.is_empty() {
        format!(
            "{{\"sectionIndex\":{},\"paraIndex\":{}}}",
            loc.section_index, loc.para_index,
        )
    } else {
        let path_entries: Vec<String> = loc.nested_path.iter().map(|e| match e {
            NestedEntry::TableCell { control_index, cell_index, para_index } => {
                format!("{{\"type\":\"cell\",\"controlIndex\":{},\"cellIndex\":{},\"paraIndex\":{}}}",
                    control_index, cell_index, para_index)
            }
            NestedEntry::TextBox { control_index, para_index } => {
                format!("{{\"type\":\"textbox\",\"controlIndex\":{},\"paraIndex\":{}}}",
                    control_index, para_index)
            }
        }).collect();
        format!(
            "{{\"sectionIndex\":{},\"paraIndex\":{},\"path\":[{}]}}",
            loc.section_index, loc.para_index, path_entries.join(","),
        )
    }
}

impl DocumentCore {
    /// 본문 문단에서 커서 위치의 필드 컨트롤 인덱스를 찾는다.
    fn find_field_control_idx(
        &self, section_idx: usize, para_idx: usize, char_offset: usize,
        _cell_path: Option<(usize, usize, usize)>,
    ) -> Option<usize> {
        let para = self.document.sections.get(section_idx)?
            .paragraphs.get(para_idx)?;
        find_field_ctrl_idx_in_para(para, char_offset)
    }

    /// 셀/글상자 내 문단에서 커서 위치의 필드 컨트롤 인덱스를 찾는다.
    fn find_field_control_idx_in_cell(
        &self, section_idx: usize, parent_para_idx: usize, control_idx: usize,
        cell_idx: usize, cell_para_idx: usize, char_offset: usize, is_textbox: bool,
    ) -> Option<usize> {
        let host = self.document.sections.get(section_idx)?
            .paragraphs.get(parent_para_idx)?;
        let ctrl = host.controls.get(control_idx)?;
        let para = if is_textbox {
            if let Control::Shape(shape) = ctrl {
                let tb = shape.drawing()?.text_box.as_ref()?;
                tb.paragraphs.get(cell_para_idx)?
            } else { return None; }
        } else {
            if let Control::Table(table) = ctrl {
                table.cells.get(cell_idx)?.paragraphs.get(cell_para_idx)?
            } else { return None; }
        };
        find_field_ctrl_idx_in_para(para, char_offset)
    }
}

/// 문단에서 커서 위치의 ClickHere 필드 컨트롤 인덱스를 반환한다.
fn find_field_ctrl_idx_in_para(para: &Paragraph, char_offset: usize) -> Option<usize> {
    for fr in &para.field_ranges {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            if field.field_type != FieldType::ClickHere { continue; }
            if char_offset >= fr.start_char_idx && char_offset <= fr.end_char_idx {
                return Some(fr.control_idx);
            }
        }
    }
    None
}

/// 문단 내 커서 위치의 누름틀 필드를 제거한다 (FieldRange만 삭제, 텍스트 유지).
fn remove_field_in_para(para: &mut Paragraph, char_offset: usize) -> Result<(), HwpError> {
    let idx = para.field_ranges.iter().position(|fr| {
        if let Some(Control::Field(field)) = para.controls.get(fr.control_idx) {
            if field.field_type != FieldType::ClickHere {
                return false;
            }
            char_offset >= fr.start_char_idx && char_offset <= fr.end_char_idx
        } else {
            false
        }
    });
    match idx {
        Some(i) => {
            para.field_ranges.remove(i);
            Ok(())
        }
        None => Err(HwpError::InvalidField("커서 위치에 누름틀 필드 없음".into())),
    }
}

/// 문자열을 JSON 이스케이프한다.
/// 문단의 char_offsets를 컨트롤/필드/텍스트 배치 순서에 맞게 재생성한다.
///
/// 원본 char_offsets에서 컨트롤 배치 패턴을 보존하면서,
/// 텍스트 길이 변경(필드 값 삽입)에 맞게 오프셋을 재계산한다.
fn rebuild_char_offsets(para: &mut Paragraph) {
    let text_chars: Vec<char> = para.text.chars().collect();
    let text_len = text_chars.len();

    if text_len == 0 {
        para.char_offsets = Vec::new();
        return;
    }

    // 원본 char_offsets에서 첫 문자 이전 컨트롤 수 추정
    // (원본 gap / 8 = 컨트롤 수)
    let ctrls_before_text = if !para.char_offsets.is_empty() {
        para.char_offsets[0] as usize / 8
    } else {
        para.controls.len()
    };

    // FIELD_BEGIN: control_idx >= ctrls_before_text이고 start > 0인 필드의 시작 위치에 갭 필요
    let mut field_begin_at: Vec<usize> = vec![0; text_len + 1];
    for fr in &para.field_ranges {
        if fr.control_idx >= ctrls_before_text && fr.start_char_idx > 0 {
            let idx = fr.start_char_idx.min(text_len);
            field_begin_at[idx] += 1;
        }
    }

    // FIELD_END 수: field_ranges에서 end가 텍스트 범위 내인 것
    let mut field_end_at: Vec<usize> = vec![0; text_len + 1];
    for fr in &para.field_ranges {
        let idx = fr.end_char_idx.min(text_len);
        field_end_at[idx] += 1;
    }

    let mut offset: u32 = ctrls_before_text as u32 * 8;
    let mut new_offsets = Vec::with_capacity(text_len);

    for (i, ch) in text_chars.iter().enumerate() {
        // 이 문자 앞에 FIELD_BEGIN 컨트롤 갭 삽입
        offset += field_begin_at[i] as u32 * 8;
        // 이 문자 앞에 FIELD_END 마커 갭 삽입
        offset += field_end_at[i] as u32 * 8;

        new_offsets.push(offset);

        let char_size = match *ch {
            '\t' => 8,
            '\n' | '\u{00A0}' => 1,
            c => {
                let mut buf = [0u16; 2];
                c.encode_utf16(&mut buf).len() as u32
            }
        };
        offset += char_size;
    }

    para.char_offsets = new_offsets;
}

fn json_escape(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => result.push(c),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod parent_range_tests;
#[cfg(test)]
mod virtual_cell_id_tests;
#[cfg(test)]
mod regression_tests;
