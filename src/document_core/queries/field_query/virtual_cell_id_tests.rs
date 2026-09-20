//! 가상 셀 필드 ID 충돌 회귀 테스트 (rhwp-2).
//!
//! 서로 다른 문단에 표가 하나씩 있고 각각 controls[0]에 위치하면, 이전
//! 구현은 두 표의 cells[0]에 대해 모두 field_id=0을 만들어 이름으로 조회한
//! 두 번째 셀의 fieldId를 다시 ID로 조회하면 첫 번째 셀의 값이 반환됐다.

use super::*;
use crate::model::document::Section;
use crate::model::table::{Cell, Table};

fn table_with_named_cell(field_name: &str, value: &str) -> Control {
    let mut cell = Cell::new_empty(0, 0, 1000, 1000, 0);
    cell.field_name = Some(field_name.to_string());
    cell.paragraphs[0].text = value.to_string();
    Control::Table(Box::new(Table {
        cells: vec![cell],
        ..Default::default()
    }))
}

#[test]
fn virtual_cell_fields_in_different_paragraphs_get_distinct_ids() {
    let mut core = DocumentCore::new_empty();
    core.document.sections.push(Section::default());

    // 두 문단, 각각 표 하나(controls[0]), 표마다 cells[0]에 서로 다른
    // field_name/값 -- 이전 구현에서는 (ci=0)<<16 | (cell_i=0) = 0으로 충돌.
    let mut para_a = Paragraph::default();
    para_a.controls.push(table_with_named_cell("first", "value-a"));
    core.document.sections[0].paragraphs.push(para_a);

    let mut para_b = Paragraph::default();
    para_b.controls.push(table_with_named_cell("second", "value-b"));
    core.document.sections[0].paragraphs.push(para_b);

    let fields = core.collect_all_fields();
    assert_eq!(fields.len(), 2);
    assert_ne!(
        fields[0].field.field_id, fields[1].field.field_id,
        "two unrelated virtual cell fields must not share an ID"
    );

    // 두 번째 셀을 이름으로 조회해 fieldId를 얻은 뒤, 그 ID로 다시 조회하면
    // 반드시 같은 셀(second/value-b)의 값이 나와야 한다.
    let by_name = core.get_field_value_by_name("second").unwrap();
    assert!(by_name.contains("value-b"));

    let second_id = fields.iter().find(|fi| fi.field.field_name() == Some("second"))
        .unwrap().field.field_id;
    let by_id = core.get_field_value_by_id(second_id).unwrap();
    assert!(
        by_id.contains("value-b"),
        "ID-based lookup for the field found by name must return that same field's value, got: {by_id}"
    );
}

#[test]
fn get_field_value_by_id_errors_on_a_genuine_duplicate_instead_of_guessing() {
    let fields = vec![
        FieldInfo {
            field: Field {
                field_type: FieldType::ClickHere,
                command: String::new(),
                properties: 0,
                extra_properties: 0,
                field_id: 42,
                ctrl_id: 0,
                ctrl_data_name: Some("a".to_string()),
                memo_index: 0,
            },
            location: FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] },
            value: "value-a".to_string(),
            field_range_index: 0,
            is_virtual_cell_field: true,
        },
        FieldInfo {
            field: Field {
                field_type: FieldType::ClickHere,
                command: String::new(),
                properties: 0,
                extra_properties: 0,
                field_id: 42,
                ctrl_id: 0,
                ctrl_data_name: Some("b".to_string()),
                memo_index: 0,
            },
            location: FieldLocation { section_index: 1, para_index: 0, nested_path: vec![] },
            value: "value-b".to_string(),
            field_range_index: 0,
            is_virtual_cell_field: true,
        },
    ];

    let result = find_field_by_id(&fields, 42);
    assert!(
        result.is_err(),
        "a genuine ID collision must be reported as an error, not silently resolved to the first match"
    );
}

fn virtual_field(id: u32, name: &str) -> FieldInfo {
    FieldInfo {
        field: Field {
            field_type: FieldType::ClickHere,
            command: String::new(),
            properties: 0,
            extra_properties: 0,
            field_id: id,
            ctrl_id: 0,
            ctrl_data_name: Some(name.to_string()),
            memo_index: 0,
        },
        location: FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] },
        value: String::new(),
        field_range_index: 0,
        is_virtual_cell_field: true,
    }
}

/// ctrl_id: 0으로 구성한다 -- HWP3/HWPX 파서가 Field::default()로 만드는
/// 실제 필드(메일머지, 색인 표시, HWPX FIELD_BEGIN 등)도 ctrl_id를 채우지
/// 않아 그대로 0이므로, is_virtual_cell_field가 (ctrl_id가 아니라) 진짜
/// 판별 기준이어야 함을 이 값 자체로 검증한다.
fn real_field(id: u32) -> FieldInfo {
    FieldInfo {
        field: Field {
            field_type: FieldType::ClickHere,
            command: String::new(),
            properties: 0,
            extra_properties: 0,
            field_id: id,
            ctrl_id: 0,
            ctrl_data_name: None,
            memo_index: 0,
        },
        location: FieldLocation { section_index: 0, para_index: 0, nested_path: vec![] },
        value: String::new(),
        field_range_index: 0,
        is_virtual_cell_field: false,
    }
}

/// 해시가 우연히 같은 제안 ID를 만들었다고 가정한 경우(직접 구성해 재현) --
/// 재배정 후에는 모든 필드의 ID가 서로 다르고, 실제 필드
/// (is_virtual_cell_field == false, ctrl_id == 0인 경우 포함)의 ID는
/// 그대로 유지되어야 한다.
#[test]
fn resolve_virtual_field_id_collisions_guarantees_document_wide_uniqueness() {
    // The colliding virtual field is listed BEFORE the real field it
    // collides with deliberately: a discriminator that (incorrectly)
    // treats ctrl_id == 0 as "virtual" would put the real field in the
    // same reassignment pool too, and since the virtual entry claims the
    // shared ID first by iteration order, the real field's OWN id would
    // then appear "already used" and get reassigned right along with it
    // -- exactly the bug this ordering is designed to catch.
    let mut fields = vec![
        virtual_field(0x8000_0005, "collides-with-real"),
        real_field(0x8000_0005),
        virtual_field(0x9000_0000, "collides-with-next"),
        virtual_field(0x9000_0000, "collides-with-prev"),
    ];

    resolve_virtual_field_id_collisions(&mut fields);

    assert_eq!(fields[1].field.field_id, 0x8000_0005, "real field IDs must never change");

    let ids: Vec<u32> = fields.iter().map(|fi| fi.field.field_id).collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(), ids.len(),
        "every field must end up with a distinct ID after resolution, got: {ids:?}"
    );
}
