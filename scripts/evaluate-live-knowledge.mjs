// Read-only evaluation against the installed application's actual retrieval service.
// Keep company fixtures and reports outside tracked files.
import { readFile, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { callApp } from '../mcp/control-client.mjs';

const [projectId, repo, fixture, output] = process.argv.slice(2);
if (!output) throw new Error('Usage: node scripts/evaluate-live-knowledge.mjs PROJECT_ID REPO FIXTURE.json REPORT.json');
const dataset = JSON.parse(await readFile(fixture, 'utf8'));
const git = (...args) => execFileSync('git', ['-C', repo, ...args], { encoding: 'utf8', timeout: 15_000 }).trim();
if (git('rev-parse', 'HEAD') !== dataset.revision || git('status', '--porcelain')) {
  throw new Error('Evaluation requires a clean worktree at the fixture revision.');
}
// Validate every anchor before issuing any cloud query.
for (const test of dataset.cases) {
  for (const source of test.sources) {
    const path = resolve(repo, source.path);
    if (!path.startsWith(resolve(repo) + '/')) throw new Error('Fixture path escapes repository');
    const lines = (await readFile(path, 'utf8')).split('\n');
    if (source.anchor && !lines[source.startLine - 1]?.toLowerCase().includes(source.anchor.toLowerCase())) {
      throw new Error(`Fixture anchor drift: ${test.id}`);
    }
  }
}
const report = { revision: dataset.revision, startedAt: new Date().toISOString(), mode: 'installed_retrieval', complete: false, answerQuality: 'not_run', results: [] };
for (const test of dataset.cases) {
  const result = await callApp('search_knowledge', { id: projectId, question: test.question, includeDiagnostics: true });
  if (!Array.isArray(result.hits) || typeof result.degraded !== 'boolean') throw new Error('Install a build supporting retrieval diagnostics before evaluation.');
  const sources = result.hits.slice(0, 8).map(hit => hit.evidence);
  if (sources.some(source => source.revision !== dataset.revision || source.dirty)) throw new Error('Retrieved sources differ from the pinned clean revision.');
  const missing = test.sources.filter(expected => !sources.some(source =>
    source.path === expected.path && source.startLine <= expected.startLine && source.endLine >= expected.startLine));
  report.results.push({ id: test.id, kind: test.kind, question: test.question, found: test.kind === 'answerable' ? missing.length === 0 : null, missing, sources, degraded: result.degraded, reason: result.reason, retrievalMs: result.retrievalMs });
  await writeFile(output, JSON.stringify(report, null, 2) + '\n');
  console.log(`${test.id}: ${test.kind === 'answerable' ? (missing.length ? 'MISS' : 'PASS') : 'answer review required'}; degraded=${result.degraded}; ${result.retrievalMs} ms`);
}
const answerable = report.results.filter(test => test.kind === 'answerable');
report.answerable = answerable.length;
report.evidenceFoundAt8 = answerable.filter(test => test.found).length;
report.recallAt8 = answerable.length ? report.evidenceFoundAt8 / answerable.length : null;
report.degradedQueries = report.results.filter(test => test.degraded).length;
report.complete = true;
report.finishedAt = new Date().toISOString();
await writeFile(output, JSON.stringify(report, null, 2) + '\n');
console.log(`Evidence recall@8: ${report.evidenceFoundAt8}/${report.answerable}; degraded queries: ${report.degradedQueries}. Answer quality and live audio were not evaluated.`);
if (report.recallAt8 === null || report.recallAt8 < 0.9 || report.degradedQueries) process.exitCode = 1;
