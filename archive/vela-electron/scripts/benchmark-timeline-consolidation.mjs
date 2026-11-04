import fs from 'node:fs';
import { createRequire } from 'node:module';
import { performance } from 'node:perf_hooks';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
const require = createRequire(`${root}/package.json`);
const ts = require('typescript');
const esbuild = require('esbuild');
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--ref')) {
  throw new Error('Usage: node scripts/benchmark-timeline-consolidation.mjs [--ref <git-commit>]');
}
const supportedNode = fs.readFileSync(`${root}/.node-version`, 'utf8').trim();
if (process.versions.node !== supportedNode) throw new Error(`Use supported Node ${supportedNode}`);
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }).trim();
const ref = args[1] ? git('rev-parse', '--verify', `${args[1]}^{commit}`) : null;
const modelPath = 'src/renderer/src/components/timeline-model.ts';
const facadePath = 'src/renderer/src/components/OpenCodeTimeline.tsx';
const sourcePath = ref && !git('ls-tree', '-r', '--name-only', ref, modelPath) ? facadePath : modelPath;
const source = ref ? git('show', `${ref}:${sourcePath}`) : fs.readFileSync(`${root}/${sourcePath}`, 'utf8');
const names = new Set(['parseInput', 'toolKey', 'parseSubagentTag', 'parseLegacyTaskText', 'matchChildSession',
  'subagentChildID', 'taskChildID', 'dispatchSignature', 'taskDispatchSignature', 'refDispatchSignature',
  'mergeSubagentTool', 'consolidateSubagentTools', 'taskDescriptor']);
const ast = ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true);
const declarations = ast.statements.filter(statement => statement.name && names.has(statement.name.text));
const code = declarations.map(statement => statement.getText(ast).replace(/^export /, "")).join('\n') + '\nexport { consolidateSubagentTools };';
const bundle = esbuild.transformSync(code, { loader: 'ts', format: 'esm' }).code;
const { consolidateSubagentTools } = await import(`data:text/javascript;base64,${Buffer.from(bundle).toString('base64')}`);
const assistant = (id, parts) => ({ kind: 'assistant', id, messageID: id, completed: true, parts });
const tool = (id, title, input, metadata) => ({ kind: 'tool', id, tool: {
  id, title, detail: 'Task detail', status: 'running', input: JSON.stringify(input), metadata
} });
const build = mode => {
  const sessions = mode === 'known-graph' ? Array.from({ length: 150 }, (_, index) => ({
    id: `child-${index}`, parentID: 'parent', directory: '/repo', agent: 'build', title: `Inspect ${index}`, updatedAt: index
  })) : [];
  const transcript = [];
  for (let index = 0; index < 150; index++) {
    const input = { agent: 'build', description: `Inspect ${index}` };
    transcript.push({ kind: 'user', id: `user-${index}`, text: `Prompt ${index}` });
    for (let step = 0; step < 3; step++) {
      const parts = mode === 'no-delegation'
        ? [tool(`read-${index}-${step}`, 'read', { filePath: '/repo/file.ts' })]
        : [tool(`task-${index}-${step}`, 'task', input, mode === 'explicit-id' ? { sessionID: `child-${index}` } : undefined)];
      parts.push({ kind: 'text', id: `text-${index}-${step}`, text: 'Response text', complete: true });
      transcript.push(assistant(`assistant-${index}-${step}`, parts));
    }
    if (mode !== 'no-delegation') transcript.push({ kind: 'synthetic', id: `settled-${index}`,
      text: `<subagent id="${mode === 'missing-graph' ? '' : `child-${index}`}" agent="build" description="Inspect ${index}" state="completed"></subagent>` });
  }
  return { transcript, sessions };
};
const workloads = ['no-delegation', 'known-graph', 'missing-graph', 'explicit-id'];
const results = {};
for (const mode of workloads) {
  const { transcript, sessions } = build(mode);
  const snapshot = JSON.stringify(transcript);
  const expected = consolidateSubagentTools(transcript, sessions, 'parent');
  const expectedSnapshot = JSON.stringify(expected);
  const sample = () => {
    const start = performance.now();
    let result;
    for (let index = 0; index < 100; index++) result = consolidateSubagentTools(transcript, sessions, 'parent');
    const elapsedMs = performance.now() - start;
    if (JSON.stringify(result) !== expectedSnapshot || JSON.stringify(transcript) !== snapshot) throw new Error('Output or input changed');
    return { elapsedMs, retainedAssistantRows: result.filter((row, index) => row.kind === 'assistant' && row === transcript[index]).length };
  };
  for (let index = 0; index < 3; index++) sample();
  const samples = Array.from({ length: 9 }, sample);
  const sorted = samples.map(sample => sample.elapsedMs).sort((a, b) => a - b);
  results[mode] = { transcriptItems: transcript.length, assistantItems: 450, iterations: 100,
    outputHash: createHash('sha256').update(expectedSnapshot).digest('hex'),
    fixtureHash: createHash('sha256').update(snapshot).digest('hex'),
    minMs: sorted[0], medianMs: sorted[4], maxMs: sorted[8], samples };
}
console.log(JSON.stringify({ node: process.version, source: ref ?? 'working tree', head: git('rev-parse', 'HEAD'), sourcePath, sourceHash: createHash('sha256').update(source).digest('hex'),
  scope: 'Pure delegation consolidation only; no Store, React, IPC, browser layout or paint', warmups: 3, samples: 9, results }, null, 2));
