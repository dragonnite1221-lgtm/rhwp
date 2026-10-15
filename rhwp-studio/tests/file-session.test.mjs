import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';

// Execute complete production modules with only import adapters replaced.
// No browser or WASM execution is claimed. The native fake deliberately reuses
// its object on createBlankDocument, unlike a document-reference-only mock.
function moduleBody(path) {
  return stripTypeScriptTypes(readFileSync(new URL(path, import.meta.url), 'utf8'))
    .replace(/^import[\s\S]*?;\s*/gm, '')
    .replace(/\bexport (?=(?:async )?function|const|class)/g, '');
}
const helperNames = ['pickOpenFileHandle', 'readFileFromHandle', 'saveDocumentToFileSystem'];
const helpers = new Function(moduleBody('../src/command/file-system-access.ts') +
  '; return {' + helperNames.join(',') + '};')();
class NativeDocument {
  constructor(bytes) { this.bytes = bytes; }
  static createEmpty() { return new NativeDocument(new Uint8Array([0])); }
  createBlankDocument() { this.bytes = new Uint8Array([0]); return '{"pageCount":1}'; }
  free() {}
  convertToEditable() {}
  setFileName() {}
  getDocumentInfo() { return '{"pageCount":1}'; }
  getExternalImageBasenames() { return '[]'; }
  exportHwp() { return this.bytes; }
}
const WasmBridge = new Function('HwpDocument',
  moduleBody('../src/core/wasm-bridge.ts') + '; return WasmBridge;')(NativeDocument);
function deferred() {
  let resolve, reject;
  const promise = new Promise((a, b) => { resolve = a; reject = b; });
  return { promise, resolve, reject };
}
function handle(name, phase) {
  const reached = deferred(), gate = deferred(), writes = [];
  return {
    name, writes, reached, gate,
    getFile: async () => new File([new Uint8Array([2])], name),
    async createWritable() {
      return {
        async write(blob) { writes.push([...new Uint8Array(await blob.arrayBuffer())]); if (phase === 'write') { reached.resolve(); await gate.promise; } },
        async close() { if (phase === 'close') { reached.resolve(); await gate.promise; } },
      };
    },
  };
}
function setup() {
  const wasm = new WasmBridge();
  wasm.loadDocument(new Uint8Array([1]), 'A.hwp');
  const state = { clicks: 0, alerts: [], downloads: [], events: [], dialog: async () => 'saved.hwp' };
  const win = {};
  const doc = {
    getElementById: () => ({ click() { state.clicks++; } }),
    createElement: () => ({ click() { state.downloads.push(this.download); } }),
  };
  const commands = new Function(...helperNames, 'PageSetupDialog', 'AboutDialog', 'showConfirm', 'showSaveAs',
    'window', 'document', 'alert', 'URL', 'setTimeout',
    moduleBody('../src/command/commands/file.ts') + '; return fileCommands;')(
    ...helperNames.map(n => helpers[n]), class {}, class {}, async () => true,
    (...args) => state.dialog(...args), win, doc, msg => state.alerts.push(msg),
    { createObjectURL: () => 'blob:synthetic', revokeObjectURL() {} }, fn => fn());
  const services = { wasm, getContext: () => ({ hasDocument: true }), eventBus: { emit(name, payload) {
    state.events.push([name, payload]);
    if (name === 'create-new-document') wasm.createNewDocument();
    if (name === 'open-document-bytes') { wasm.loadDocument(payload.bytes, payload.fileName); wasm.currentFileHandle = payload.fileHandle; }
  } } };
  return { wasm, state, win, run: id => commands.find(c => c.id === id).execute(services) };
}
for (const command of ['file:save', 'file:save-as']) {
  for (const phase of ['write', 'close']) {
    for (const replacement of ['new', 'open', 'drop', 'release']) {
      test(command + ': delayed ' + phase + ' cannot attach A to ' + replacement, async () => {
        const x = setup(), a = handle('A.hwp', phase), b = handle('B.hwp');
        x.wasm.currentFileHandle = a;
        x.win.showSaveFilePicker = async () => a;
        const pending = x.run(command);
        await a.reached.promise;
        const oldNative = x.wasm.doc;
        if (replacement === 'new') { await x.run('file:new-doc'); assert.equal(x.wasm.doc, oldNative); }
        if (replacement === 'open') { x.win.showOpenFilePicker = async () => [b]; await x.run('file:open'); }
        // Drop uses main.ts's loadBytes -> loadDocument seam; no DOM drop event is simulated.
        if (replacement === 'drop') x.wasm.loadDocument(new Uint8Array([2]), 'B.hwp');
        if (replacement === 'release') x.wasm.releaseDocument();
        const expectedName = x.wasm.fileName, expectedHandle = x.wasm.currentFileHandle;
        a.gate.resolve(); await pending;
        assert.equal(x.wasm.fileName, expectedName);
        assert.equal(x.wasm.currentFileHandle, expectedHandle);
        assert.deepEqual(a.writes, [[1]]);
        if (replacement !== 'release') {
          x.win.showSaveFilePicker = async () => b;
          await x.run('file:save');
          assert.equal(a.writes.length, 1, 'next save must not overwrite A');
          assert.equal(b.writes.length, 1);
        }
      });
    }
  }
  test(command + ': failed old write cannot start fallback for replacement', async () => {
    const x = setup(), a = handle('A.hwp', 'write');
    x.wasm.currentFileHandle = a; x.win.showSaveFilePicker = async () => a;
    let dialogs = 0; x.state.dialog = async () => { dialogs++; return 'wrong.hwp'; };
    const pending = x.run(command); await a.reached.promise;
    x.wasm.createNewDocument();
    a.gate.reject(new Error('disk failure')); await pending;
    assert.equal(dialogs, 0); assert.deepEqual(x.state.downloads, []);
    assert.equal(x.wasm.fileName, '새 문서.hwp'); assert.equal(x.wasm.currentFileHandle, null);
  });
  test(command + ': same-session success publishes selected handle', async () => {
    const x = setup(), a = handle('chosen.hwp', 'close');
    x.win.showSaveFilePicker = async () => a;
    const pending = x.run(command); await a.reached.promise; a.gate.resolve(); await pending;
    assert.equal(x.wasm.currentFileHandle, a); assert.equal(x.wasm.fileName, 'chosen.hwp');
  });
  test(command + ': late fallback filename cannot rename replacement', async () => {
    const x = setup(), gate = deferred(), reached = deferred();
    x.wasm.createNewDocument();
    x.state.dialog = () => { reached.resolve(); return gate.promise; };
    const pending = x.run(command); await reached.promise;
    x.wasm.loadDocument(new Uint8Array([2]), 'B.hwp');
    gate.resolve('A-saved.hwp'); await pending;
    assert.equal(x.wasm.fileName, 'B.hwp'); assert.equal(x.wasm.currentFileHandle, null);
    assert.deepEqual(x.state.downloads, []);
  });
  test(command + ': same-session fallback downloads and updates filename', async () => {
    const x = setup(); x.wasm.createNewDocument(); await x.run(command);
    assert.equal(x.wasm.fileName, 'saved.hwp'); assert.deepEqual(x.state.downloads, ['saved.hwp']);
  });
  for (const error of ['AbortError', 'NotAllowedError']) test(command + ': preserves ' + error + ' cancellation', async () => {
    const x = setup(); x.win.showSaveFilePicker = async () => { throw new DOMException('cancel', error); };
    await x.run(command); assert.deepEqual(x.state.downloads, []); assert.deepEqual(x.state.alerts, []);
  });
}
for (const mode of ['abort', 'empty', 'unsupported', 'success', 'error']) test('open: ' + mode, async () => {
  const x = setup(), b = handle('B.hwp');
  if (mode !== 'unsupported') x.win.showOpenFilePicker = async () => {
    if (mode === 'abort') throw new DOMException('cancel', 'AbortError');
    if (mode === 'error') throw new Error('read failure');
    return mode === 'empty' ? [] : [b];
  };
  await x.run('file:open');
  assert.equal(x.state.clicks, mode === 'unsupported' ? 1 : 0);
  assert.equal(x.state.alerts.length, mode === 'error' ? 1 : 0);
  assert.equal(x.state.events.length, mode === 'success' ? 1 : 0);
  if (mode === 'success') { assert.equal(x.wasm.currentFileHandle, b); assert.equal(x.wasm.fileName, 'B.hwp'); }
});
