//! 가상 셀 필드 ID 충돌 회귀 테스트 (rhwp-2), 2/2 (200줄 제한 분할):
//! `resolve_virtual_field_id_collisions` 자체의 유일성 보장과, 적대적인
//! 충돌 입력에서도 O(n^2)로 퇴화하지 않는지를 검증한다.

use super::*;

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

/// 조작된 문서가 다수의 가상 셀 필드에 정확히 동일한 제안 ID를 갖게
/// 만든 최악의 경우를 재현한다. 이전 구현은 충돌이 발생할 때마다 항상
/// 자신의 제안값에서부터 다시 1씩 증가시키며 빈 자리를 찾았기 때문에,
/// k번째로 처리되는 충돌 필드마다 이미 채워진 앞쪽 구간 전체를 처음부터
/// 다시 훑어야 해서 총 시간이 O(n^2)이 됐다 (CPU 소모형 DoS로 이어질 수
/// 있음). 이 테스트는 정답(고유성)뿐 아니라, 넉넉한 시간 예산 안에 끝나는지
/// 함께 검증해 알고리즘 복잡도 퇴화를 잡아낸다. n=20,000일 때 O(n^2) 구현은
/// 수 초~수십 초가 걸리는 반면(각 삽입이 이전 구간을 재스캔), O(n) 구현은
/// 수 밀리초 안에 끝난다.
#[test]
fn resolve_virtual_field_id_collisions_stays_fast_under_adversarial_collisions() {
    const N: usize = 20_000;
    let mut fields: Vec<FieldInfo> = (0..N)
        .map(|i| virtual_field(0x8000_0000, &format!("cell-{i}")))
        .collect();

    let start = std::time::Instant::now();
    resolve_virtual_field_id_collisions(&mut fields);
    let elapsed = start.elapsed();

    let ids: Vec<u32> = fields.iter().map(|fi| fi.field.field_id).collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(), ids.len(),
        "every one of the {N} colliding virtual fields must still end up unique"
    );

    assert!(
        elapsed.as_millis() < 1000,
        "resolving {N} fully-colliding virtual field IDs took {elapsed:?}; \
         this should be near-linear (a handful of ms), not the O(n^2) blowup \
         a naive restart-from-scratch probe would produce"
    );
}
