//! quality-ledger 회귀 테스트 모음: 가상 셀 필드의 ID 기반 설정, 손상된
//! field_range 범위, 필드 제거 후 raw_stream 무효화, 문단 없는 셀에 대한
//! 값 설정을 각각 재현한다.

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

/// 회귀 테스트 (ceedfcb8a189 / 76220fb80809): `remove_field_at`과
/// `remove_field_at_in_cell`은 필드를 지운 뒤 섹션의 raw_stream을
/// 무효화해야 한다. `serialize_section`은 raw_stream이 Some이면 모델의
/// 변경을 무시하고 그 원본 바이트를 그대로 반환하므로, 무효화하지 않으면
/// 방금 제거한 필드가 저장 시 다시 나타난다(제거가 유실됨).
#[test]
fn remove_field_at_invalidates_raw_stream() {
    let mut para = Paragraph::default();
    para.text = "AB".to_string();
    para.controls.push(make_field_control(1));
    para.field_ranges.push(FieldRange { start_char_idx: 0, end_char_idx: 2, control_idx: 0 });
    let mut core = core_with_section(Section {
        paragraphs: vec![para],
        raw_stream: Some(vec![0xAA, 0xBB]),
        ..Default::default()
    });

    core.remove_field_at(0, 0, 0).unwrap();

    assert!(
        core.document.sections[0].raw_stream.is_none(),
        "raw_stream must be cleared so the removal survives serialization"
    );
}

/// 위와 동일한 근본 원인(raw_stream 무효화 누락)을 셀/글상자 내부 필드
/// 제거 경로에서도 검증한다.
#[test]
fn remove_field_at_in_cell_invalidates_raw_stream() {
    let mut cell = Cell::new_empty(0, 0, 1000, 1000, 0);
    cell.paragraphs[0].text = "AB".to_string();
    cell.paragraphs[0].controls.push(make_field_control(1));
    cell.paragraphs[0].field_ranges.push(FieldRange {
        start_char_idx: 0, end_char_idx: 2, control_idx: 0,
    });
    let table = Control::Table(Box::new(Table { cells: vec![cell], ..Default::default() }));

    let mut host_para = Paragraph::default();
    host_para.controls.push(table);
    let mut core = core_with_section(Section {
        paragraphs: vec![host_para],
        raw_stream: Some(vec![0xAA, 0xBB]),
        ..Default::default()
    });

    core.remove_field_at_in_cell(0, 0, 0, 0, 0, 0, false).unwrap();

    assert!(
        core.document.sections[0].raw_stream.is_none(),
        "raw_stream must be cleared so the removal survives serialization"
    );
}

/// 회귀 테스트 (4406d65a97e9): 셀에 문단이 하나도 없으면(비정상이지만
/// 방어적으로 처리해야 하는 상태) 값을 실제로 바꾸지 못했는데도 조용히
/// `Ok`를 반환해서는 안 된다.
#[test]
fn set_field_value_by_name_on_paragraphless_cell_returns_error_not_silent_success() {
    let mut para = Paragraph::default();
    para.controls.push(table_with_named_cell("cell-field", "old"));
    let mut core = core_with_section(Section { paragraphs: vec![para], ..Default::default() });

    // 셀의 문단을 모두 비워 "문단이 없는 셀" 상태를 재현한다.
    if let Control::Table(table) = &mut core.document.sections[0].paragraphs[0].controls[0] {
        table.cells[0].paragraphs.clear();
    }

    let result = core.set_field_value_by_name("cell-field", "new");
    assert!(
        result.is_err(),
        "writing to a cell with no paragraphs must fail loudly, not report success \
         while leaving the value unchanged"
    );
}

/// 문서화 성격의 회귀 테스트 (b0284787b853, 조사 결과 false positive):
/// 이 finding은 `set_active_field_by_path`가 빈 경로에 대해
/// `path.last().unwrap()`으로 panic할 수 있다고 지적했다. 하지만
/// `resolve_paragraph_by_path`(cursor_nav.rs)는 함수 맨 앞에서
/// `path.is_empty()`이면 항상 `Err`를 반환하므로, `set_active_field_by_path`는
/// 그 경우 `path.last().unwrap()`에 도달하기 전에 이미 `return false`로
/// 빠져나간다 -- 즉 현재 코드에서는 이 panic 경로에 도달할 방법이 없다.
/// 이 테스트는 그 불변조건을 회귀로 고정한다.
#[test]
fn set_active_field_by_path_with_empty_path_returns_false_without_panicking() {
    let mut para = Paragraph::default();
    para.controls.push(table_with_named_cell("cell-field", "value"));
    let mut core = core_with_section(Section { paragraphs: vec![para], ..Default::default() });

    let changed = core.set_active_field_by_path(0, 0, &[], 0);
    assert!(!changed, "an empty path must be rejected, not treated as a valid host paragraph");
}
