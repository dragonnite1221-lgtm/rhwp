//! quality-ledger 회귀 테스트: 가상 셀 필드의 ID 기반 설정, 손상된
//! field_range 범위 검증, 그리고 그 검증 실패 시 raw_stream을 건드리지
//! 않아야 한다는 불변조건을 재현한다. 나머지 회귀 테스트는
//! `regression_tests_edge_cases.rs`에 있다 (200줄 제한 분할).

use super::*;
use crate::model::control::{Control, Field, FieldType};
use crate::model::document::{Document, Section};
use crate::model::paragraph::{FieldRange, Paragraph};
use crate::model::table::{Cell, Table};

/// `core.document.sections`에 직접 push하는 대신 `set_document`를 거쳐
/// `composed`/`styles`/페이지네이션 벡터를 문서 구조와 일관되게 초기화한다.
/// `recompose_section`은 `self.composed[section_idx]`를 인덱싱하므로,
/// 이 초기화 없이 섹션을 직접 밀어 넣으면 (개별 필드 함수 자체와는
/// 무관한 이유로) 인덱스 초과 panic이 난다.
fn core_with_section(section: Section) -> DocumentCore {
    let mut doc = Document::default();
    doc.sections.push(section);
    let mut core = DocumentCore::new_empty();
    core.set_document(doc);
    core
}

fn make_field_control(ctrl_id: u32) -> Control {
    Control::Field(Field {
        field_type: FieldType::ClickHere,
        command: String::new(),
        properties: 0,
        extra_properties: 0,
        field_id: ctrl_id,
        ctrl_id,
        ctrl_data_name: None,
        memo_index: 0,
    })
}

fn table_with_named_cell(field_name: &str, value: &str) -> Control {
    let mut cell = Cell::new_empty(0, 0, 1000, 1000, 0);
    cell.field_name = Some(field_name.to_string());
    cell.paragraphs[0].text = value.to_string();
    Control::Table(Box::new(Table {
        cells: vec![cell],
        ..Default::default()
    }))
}

/// 회귀 테스트 (536aada7ddc1): `set_field_value_by_id`가 가상 셀 필드도
/// 일반 field_ranges 경로(`set_field_text_at`)로 처리하면, 이 필드의
/// 호스트 문단에는 애초에 이 가상 필드에 대응하는 field_range가 없으므로
/// (셀의 field_name에서 합성한 값일 뿐이다) "field_range 인덱스 초과"로
/// 실패하거나, 우연히 같은 문단에 있는 *다른* field_range를 잘못 덮어쓴다.
/// `set_field_value_by_name`은 이미 `is_virtual_cell_field`로 분기하고
/// 있었으므로, ID 기반 설정도 동일하게 동작해야 한다.
#[test]
fn set_field_value_by_id_on_virtual_cell_field_updates_the_cell_text() {
    let mut para = Paragraph::default();
    para.controls.push(table_with_named_cell("cell-field", "old"));
    let mut core = core_with_section(Section { paragraphs: vec![para], ..Default::default() });

    let fields = core.collect_all_fields();
    assert_eq!(fields.len(), 1);
    assert!(fields[0].is_virtual_cell_field);
    let field_id = fields[0].field.field_id;

    core.set_field_value_by_id(field_id, "new").expect(
        "setting a virtual cell field by its own ID must succeed, not fail with \
         'field_range 인덱스 초과' or silently touch an unrelated field",
    );

    let Control::Table(table) = &core.document.sections[0].paragraphs[0].controls[0] else {
        panic!("expected table control");
    };
    assert_eq!(
        table.cells[0].paragraphs[0].text, "new",
        "the cell's own text must be updated, not left unchanged or corrupted"
    );
}

/// 회귀 테스트 (513dec11eb42): 손상되거나 조작된 문서가 텍스트 길이를
/// 벗어나는 field_range를 담고 있으면, 검증 없이 `text_chars[..start]` /
/// `text_chars[end..]`로 슬라이싱할 때 panic한다. 명확한 오류로 처리되어야
/// 한다.
#[test]
fn set_field_text_at_rejects_out_of_bounds_field_range_instead_of_panicking() {
    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());

    let mut para = Paragraph::default();
    para.text = "AB".to_string();
    para.controls.push(make_field_control(1));
    // end_char_idx(99)가 실제 텍스트 길이(2)를 훨씬 초과한다.
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 99, control_idx: 0 });
    core.document.sections[0].paragraphs.push(para);

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };
    let result = core.set_field_text_at(&location, 0, "x");

    assert!(
        result.is_err(),
        "an out-of-bounds field_range must return an error, not panic"
    );
}

/// 회귀 테스트: 위 out-of-bounds 검증이 실패하는 경우, `set_field_text_at`은
/// 섹션의 `raw_stream`을 절대 건드리면 안 된다. 초기 구현은 검증보다 먼저
/// `raw_stream = None`을 실행했는데, 그 상태에서 검증이 실패해 `Err`를
/// 반환해도 모델 자체는 전혀 바뀌지 않은 채였다 -- 그런데 `raw_stream`만
/// 지워진 채로 남아 있으면, 다음 저장 시 `serialize_section`이 (원본
/// 바이트 대신) 손실 위험이 있는 재직렬화 경로로 빠지게 되어, 실패한
/// 시도 하나가 무관한 이후 저장까지 오염시킨다. `raw_stream` 무효화는
/// 검증을 통과해 실제로 모델을 바꾸기로 확정된 뒤에만 일어나야 한다.
#[test]
fn set_field_text_at_preserves_raw_stream_when_validation_fails() {
    let mut para = Paragraph::default();
    para.text = "AB".to_string();
    para.controls.push(make_field_control(1));
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 99, control_idx: 0 });
    let mut core = core_with_section(Section {
        paragraphs: vec![para],
        raw_stream: Some(vec![0xAA, 0xBB]),
        ..Default::default()
    });

    let location = FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] };
    let result = core.set_field_text_at(&location, 0, "x");

    assert!(result.is_err(), "the out-of-bounds field_range must still be rejected");
    assert!(
        core.document.sections[0].raw_stream.is_some(),
        "a failed, no-op call must not clear raw_stream -- doing so would make the \
         next successful save re-serialize an unmodified model instead of returning \
         the original bytes, silently risking data loss unrelated to this call"
    );
}
