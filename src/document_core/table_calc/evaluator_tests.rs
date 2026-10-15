use super::*;

fn make_ctx() -> TableContext {
    TableContext { row_count: 5, col_count: 5, current_row: 4, current_col: 0 }
}

fn sample_cell(col: usize, row: usize) -> Option<f64> {
    // 5x5 표: 값 = (row+1)*10 + (col+1)
    Some((row as f64 + 1.0) * 10.0 + (col as f64 + 1.0))
}

#[test]
fn test_literal() {
    let ctx = make_ctx();
    let r = evaluate_formula("=42", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 42.0);
}

#[test]
fn test_cell_ref() {
    let ctx = make_ctx();
    // A1 = (0,0) = 11.0
    let r = evaluate_formula("=A1", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 11.0);
}

#[test]
fn test_arithmetic() {
    let ctx = make_ctx();
    // A1(11) + B2(22) * 2 = 11 + 44 = 55
    let r = evaluate_formula("=A1+B2*2", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 55.0);
}

#[test]
fn test_sum_range() {
    let ctx = make_ctx();
    // SUM(A1:A3) = 11 + 21 + 31 = 63
    let r = evaluate_formula("=SUM(A1:A3)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 63.0);
}

#[test]
fn test_sum_direction() {
    // 현재 셀 (0, 4), above = (0,0)~(0,3) = 11+21+31+41 = 104
    let ctx = make_ctx();
    let r = evaluate_formula("=SUM(above)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 104.0);
}

#[test]
fn test_avg() {
    let ctx = make_ctx();
    // AVG(A1:A3) = (11+21+31)/3 = 21
    let r = evaluate_formula("=AVG(A1:A3)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 21.0);
}

#[test]
fn test_product() {
    let ctx = make_ctx();
    // PRODUCT(B1,C3) = 12 * 33 = 396
    let r = evaluate_formula("=PRODUCT(B1,C3)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 396.0);
}

#[test]
fn test_nested_function() {
    let ctx = make_ctx();
    // SUM(A1:A3, AVG(B1,B2)) = 63 + (12+22)/2 = 63 + 17 = 80
    let r = evaluate_formula("=SUM(A1:A3,AVG(B1,B2))", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 80.0);
}

#[test]
fn test_min_max() {
    let ctx = make_ctx();
    let r = evaluate_formula("=MIN(A1:C1)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 11.0);
    let r = evaluate_formula("=MAX(A1:C1)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 13.0);
}

#[test]
fn test_abs_sqrt() {
    let ctx = make_ctx();
    let r = evaluate_formula("=ABS(-25)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 25.0);
    let r = evaluate_formula("=SQRT(16)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 4.0);
}

#[test]
fn test_if_function() {
    let ctx = make_ctx();
    let r = evaluate_formula("=IF(1,10,20)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 10.0);
    let r = evaluate_formula("=IF(0,10,20)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 20.0);
}

#[test]
fn test_mod_function() {
    let ctx = make_ctx();
    let r = evaluate_formula("=MOD(10,3)", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 1.0);
}

#[test]
fn test_complex_formula() {
    let ctx = make_ctx();
    // a1+(b3-3)*2+sum(a1:b5,avg(c3,e5-3))
    // a1=11, b3=32, sum(a1:b5)=11+12+21+22+31+32+41+42+51+52=315
    // avg(c3, e5-3) = avg(33, 55-3) = avg(33, 52) = 42.5
    // = 11 + (32-3)*2 + (315 + 42.5) = 11 + 58 + 357.5 = 426.5
    let r = evaluate_formula("=a1+(b3-3)*2+sum(a1:b5,avg(c3,e5-3))", &ctx, &sample_cell).unwrap();
    assert_eq!(r, 426.5);
}

#[test]
fn test_div_by_zero() {
    let ctx = make_ctx();
    let r = evaluate_formula("=1/0", &ctx, &sample_cell);
    assert!(r.is_err());
}

#[test]
fn rejects_out_of_bounds_before_lookup() {
    let ctx = TableContext { row_count: 2, col_count: 2, current_row: 0, current_col: 0 };
    let lookup = |_: usize, _: usize| -> Option<f64> { panic!("invalid reference reached lookup") };
    for formula in ["=C1", "=A3", "=SUM(A1:C1)", "=SUM(C1:A1)", "=SUM(A1:B3)"] {
        assert!(evaluate_formula(formula, &ctx, &lookup).is_err(), "{formula}");
    }
}

#[test]
fn validates_current_cell_and_preserves_valid_wildcards() {
    let valid = TableContext { row_count: 2, col_count: 2, current_row: 1, current_col: 1 };
    assert_eq!(evaluate_formula("=SUM(?1:?2)", &valid, &sample_cell).unwrap(), 34.0);
    assert_eq!(evaluate_formula("=SUM(A?:B?)", &valid, &sample_cell).unwrap(), 43.0);
    assert_eq!(evaluate_formula("=SUM(B2:A1)", &valid, &sample_cell).unwrap(), 66.0);
    for (row, col) in [(2, 0), (0, 2), (usize::MAX, 0)] {
        let invalid = TableContext { current_row: row, current_col: col, ..valid };
        assert!(evaluate_formula("=1", &invalid, &sample_cell).is_err());
        assert!(evaluate_formula("=SUM(left)", &invalid, &sample_cell).is_err());
    }
}
