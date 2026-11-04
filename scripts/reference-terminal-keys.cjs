// Read-only protocol oracle from the reference app's installed xterm package.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../../vela/node_modules/@xterm/xterm/src');
const ts = require(path.resolve(__dirname, '../../vela/node_modules/typescript'));
const cache = new Map();
function load(name) {
  if (!name.startsWith('common/') || name.includes('..')) throw new Error(`Unsupported module: ${name}`);
  if (cache.has(name)) return cache.get(name).exports;
  const filename = path.join(root, `${name}.ts`);
  const source = fs.readFileSync(filename, 'utf8');
  const module = { exports: {} }; cache.set(name, module);
  const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText;
  vm.runInNewContext('(function(require,module,exports){' + js + '\n})', {}, { filename })(load, module, module.exports);
  return module.exports;
}
const evaluate = load('common/input/Keyboard').evaluateKeyboardEvent;
const names = {backspace:8,tab:9,enter:13,escape:27,left:37,right:39,up:38,down:40,insert:45,delete:46,home:36,end:35,pageup:33,pagedown:34};
for (let n=1;n<=12;n++) names[`f${n}`]=111+n;
const cases=[];
for (const [key,keyCode] of Object.entries(names)) for (let mask=0;mask<16;mask++) for (const application_cursor of [false,true]) {
  const modifiers = [['shift',1],['alt',2],['ctrl',4],['cmd',8]].filter(([,bit])=>mask&bit).map(([name])=>name);
  const result = evaluate({keyCode,key,code:key,shiftKey:!!(mask&1),altKey:!!(mask&2),ctrlKey:!!(mask&4),metaKey:!!(mask&8)},application_cursor,true,false);
  cases.push({key:[...modifiers,key].join('-'),application_cursor,bytes:[...Buffer.from(result.key??'')]});
}
const source=fs.readFileSync(path.join(root,'common/input/Keyboard.ts'));
process.stdout.write(JSON.stringify({source:'@xterm/xterm/src/common/input/Keyboard.ts',sha256:crypto.createHash('sha256').update(source).digest('hex'),cases})+'\n');
