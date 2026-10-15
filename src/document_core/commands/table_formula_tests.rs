use crate::document_core::DocumentCore;
use crate::model::control::Control;
use crate::model::paragraph::Paragraph;
use crate::model::document::Section;
use crate::model::table::{Cell, Table};

fn core_with_table(merged: bool) -> DocumentCore {
    let mut core = DocumentCore::new_empty();
    core.batch_mode = true;
    let mut table = Table { row_count: 2, col_count: 2, ..Default::default() };
    for (row, col, text) in [(0, 0, "11"), (0, 1, "12"), (1, 0, "21"), (1, 1, "22")] {
        if merged && row == 0 && col == 1 { continue; }
        let mut cell = Cell::new_empty(col, row, 7200, 3600, 0);
        if merged && row == 0 { cell.col_span = 2; }
        cell.paragraphs[0].insert_text_at(0, text);
        table.cells.push(cell);
    }
    table.rebuild_grid();
    let mut parent = Paragraph::new_empty();
    parent.controls.push(Control::Table(table));
    core.document.sections.push(Section { paragraphs: vec![parent], ..Default::default() });
    core
}

fn table(core: &DocumentCore) -> &Table {
    match &core.document.sections[0].paragraphs[0].controls[0] {
        Control::Table(table) => table,
        _ => panic!("missing table"),
    }
}

#[test]
fn formula_rejects_invalid_target_and_reference_without_mutation() {
    let mut core = core_with_table(false);
    for (row, col, formula) in [(0, 2, "=7"), (2, 0, "=7"), (0, 0, "=C1"), (0, 0, "=SUM(A1:C1)")] {
        assert!(core.evaluate_table_formula(0, 0, 0, row, col, formula, true).is_err());
        assert_eq!(table(&core).cells.iter().map(|c| c.paragraphs[0].text.as_str()).collect::<Vec<_>>(),
                   vec!["11", "12", "21", "22"]);
    }
}

#[test]
fn formula_targets_merged_anchor_without_flattening_storage() {
    let mut core = core_with_table(true);
    core.evaluate_table_formula(0, 0, 0, 1, 0, "=7", true).unwrap();
    assert_eq!(table(&core).cells[1].paragraphs[0].text, "7");
    assert_eq!(table(&core).cells[2].paragraphs[0].text, "22");
    core.evaluate_table_formula(0, 0, 0, 1, 1, "=8", true).unwrap();
    assert_eq!(table(&core).cells[2].paragraphs[0].text, "8");
    assert_eq!(table(&core).cells[0].paragraphs[0].text, "11");
}
