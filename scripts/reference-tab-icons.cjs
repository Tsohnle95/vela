// Build-time snapshot helper: execute only the reference's pure SVG component.
// React/TypeScript remain in the reference checkout, outside the native runtime.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const reference = path.resolve(process.argv[2]);
const local = createRequire(path.join(reference, 'package.json'));
const ts = local('typescript');
const React = local('react');
const { renderToStaticMarkup } = local('react-dom/server');
const source = fs.readFileSync(path.join(reference, 'src/renderer/src/components/FileIcons.tsx'), 'utf8');
const compiled = ts.transpileModule(source, {
  compilerOptions: { jsx: ts.JsxEmit.ReactJSX, module: ts.ModuleKind.CommonJS },
}).outputText;
const exported = {};
new Function('exports', 'require', compiled)(exported, (id) => id === './file-glyphs' ? {} : local(id));
const examples = { generic: 'file.unknown', code: 'file.ts', json: 'file.json', markdown: 'file.md',
  shell: 'file.sh', text: 'file.txt', config: 'file.yml', media: 'file.png', binary: 'file.bin' };
process.stdout.write(JSON.stringify(Object.fromEntries(Object.entries(examples).map(([kind, name]) => [kind,
  renderToStaticMarkup(React.createElement(exported.FileIcon, { name, isDir: false }))]))));
