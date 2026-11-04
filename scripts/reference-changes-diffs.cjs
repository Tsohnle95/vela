// Read-only reference execution. No production profiles or application startup.
const fs = require('node:fs');
const path = require('node:path');
const { createRequire } = require('node:module');
const source = process.argv[2] || path.resolve(__dirname, '../../vela');
const fromSource = createRequire(path.join(source, 'package.json'));
const { diffLines } = fromSource('diff');
const version = fromSource('diff/package.json').version;
const pairs = [ ['', ''], ['', 'new\n'], ['a\nb\n', 'a\nc\n'], ['a\nb\n',''], ['a\n','a'], ['a','a\n'], ['a\r\nb\r\n','a\nb\n'], ['界\n🌍\nlast','界\nreplacement\nlast'], ['\n\n','\n'], ['a\nb\na\n','b\na\nb\n'] ];
for (let n = 0; n < 48; n++) {
  const a = Array.from({length: n % 12}, (_, i) => `${(i * 7 + n) % 4}\n`).join('');
  const b = Array.from({length: (n * 5) % 13}, (_, i) => `${(i * 3 + n) % 4}\n`).join('');
  pairs.push([a,b]);
}
const cases = pairs.map(([before,after]) => {
  const chunks = diffLines(before,after);
  return { before, after, additions: chunks.filter(p=>p.added).reduce((sum,p)=>sum+(p.count||0),0), deletions: chunks.filter(p=>p.removed).reduce((sum,p)=>sum+(p.count||0),0) };
});
process.stdout.write(JSON.stringify({reference: 'changes-view-model.ts changeDiffStats', dependency: `diff@${version}`, cases}, null, 2)+'\n');
