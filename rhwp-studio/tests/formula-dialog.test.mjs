// Runs the real FormulaDialog handler with a small bridge model, not WASM/browser integration.
// Node >= 22.13; no dependency installation required.
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { runInNewContext } from 'node:vm';
import assert from 'node:assert/strict';
import { test } from 'node:test';
const source = process.env.FORMULA_DIALOG_SOURCE || new URL('../src/ui/formula-dialog.ts', import.meta.url);
const ts = readFileSync(source, 'utf8');
const js = stripTypeScriptTypes(ts).replace(/^import .*;$/gm, '').replace('export class FormulaDialog', 'class FormulaDialog');
const FormulaDialog = runInNewContext(js + '\nFormulaDialog;', { ModalDialog: class {}, makeOption() {}, setTimeout });

function fixture({ value = 3, index = 0, comma = true, merged = false, invalid = false } = {}) {
  const cells = merged
    ? [{row:0,col:0,text:'11',extra:'keep'}, {row:1,col:0,text:'21',extra:'keep'}, {row:1,col:1,text:'22',extra:'keep'}]
    : [{row:0,col:0,text:'old😀',extra:'keep'}, {row:0,col:1,text:'12',extra:'keep'}, {row:1,col:0,text:'21',extra:'keep'}, {row:1,col:1,text:'22',extra:'keep'}];
  const calls = [], events = [];
  const bridge = {
    getTableProperties() { return { borderFillId: 0 }; },
    getTableDimensions() { return { rowCount: 2, colCount: 2, cellCount: cells.length }; },
    getCellInfo(_s,_p,_c,i) { return invalid ? {row:0,col:2} : cells[i]; },
    evaluateTableFormula(_s,_p,_c,row,col,formula,write) {
      calls.push(['evaluate', row, col, write]);
      // Candidate native target-grid contract; this bridge remains synthetic.
      const target = cells.find(c => c.row === row && c.col === col);
      if (!target) throw new Error('invalid target');
      if (write) target.text = String(value);
      return JSON.stringify({ok:true,result:value,formula});
    },
    insertTextInCell() { throw new Error('unexpected formatted insertion'); },
  };
  const dialog = new FormulaDialog(bridge, {emit: event => events.push(event)}, {sec:0,ppi:0,ci:0,cellIndex:index});
  dialog.formulaInput = {value:'=1+2',classList:{add(){},remove(){}},focus(){}};
  dialog.formatSelect = {value:'default'}; dialog.commaCheck = {checked:comma}; dialog.errorMsg = {style:{},textContent:''};
  return {dialog,cells,calls,events};
}
test('B1 uses actual coordinates and leaves A2 intact (commas disabled)',()=>{
  const f=fixture({index:1,value:7,comma:false}); assert.equal(f.dialog.onConfirm(),true);
  assert.equal(f.cells[1].text,'7'); assert.equal(f.cells[2].text,'21');
  assert.deepEqual(f.calls[0],['evaluate',0,1,false]);
});
test('merged table B2 storage index is not row-major grid index',()=>{
  const f=fixture({index:2,merged:true,value:7,comma:false}); assert.equal(f.dialog.onConfirm(),true);
  assert.equal(f.cells[2].text,'7'); assert.equal(f.cells[1].text,'21');
  assert.deepEqual(f.calls[0],['evaluate',1,1,false]);
});
test('invalid target fails before evaluation or editing',()=>{
  const f=fixture({invalid:true,comma:false}); assert.equal(f.dialog.onConfirm(),false);
  assert.equal(f.calls.length,0); assert.equal(f.cells[0].text,'old😀'); assert.equal(f.events.length,0);
});
