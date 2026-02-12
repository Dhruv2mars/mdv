#!/usr/bin/env node
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const runs = Number(process.env.MDV_PERF_RUNS || 30);
const budgetMs = Number(process.env.MDV_PERF_BUDGET_MS || 250);
const warmupRuns = Number(process.env.MDV_PERF_WARMUP_RUNS || 2);
const fixtureBytes = Number(process.env.MDV_PERF_FIXTURE_BYTES || 200 * 1024);
const bin = process.env.MDV_PERF_BIN || resolve('target/release/mdv-cli');
const summaryPath = process.env.MDV_PERF_SUMMARY || '/tmp/mdv-perf-summary.txt';

function p95(values) {
  const sorted = [...values].sort((a, b) => a - b);
  const idx = Math.max(0, Math.ceil(sorted.length * 0.95) - 1);
  return sorted[idx];
}

function buildFixture() {
  const row = '# title\n- item one\n- item two\n`inline`\n\n';
  let text = '';
  while (Buffer.byteLength(text, 'utf8') < fixtureBytes) text += row;
  return text;
}

function measureRun() {
  const started = process.hrtime.bigint();
  const res = spawnSync(bin, [fixture], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: process.env
  });
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1_000_000;
  if ((res.status ?? 1) !== 0) {
    const err = res.stderr?.toString() || '';
    throw new Error(`perf run failed status=${res.status} stderr=${err}`);
  }
  return elapsedMs;
}

const tempRoot = mkdtempSync(join(tmpdir(), 'mdv-perf-'));
const fixture = join(tempRoot, 'perf.md');
writeFileSync(fixture, buildFixture(), 'utf8');

for (let i = 0; i < warmupRuns; i += 1) {
  measureRun();
}

const samples = [];
for (let i = 0; i < runs; i += 1) {
  samples.push(measureRun());
}

const result = {
  runs,
  warmup_runs: warmupRuns,
  fixture_bytes: fixtureBytes,
  p95_ms: p95(samples),
  min_ms: Math.min(...samples),
  max_ms: Math.max(...samples),
  avg_ms: samples.reduce((a, b) => a + b, 0) / samples.length,
  budget_ms: budgetMs
};

const summary = [
  `fixture_bytes=${result.fixture_bytes}`,
  `runs=${result.runs}`,
  `warmup_runs=${result.warmup_runs}`,
  `p95_ms=${result.p95_ms.toFixed(2)}`,
  `min_ms=${result.min_ms.toFixed(2)}`,
  `max_ms=${result.max_ms.toFixed(2)}`,
  `avg_ms=${result.avg_ms.toFixed(2)}`,
  `budget_ms=${result.budget_ms.toFixed(2)}`
].join('\n');

writeFileSync(summaryPath, `${summary}\n`, 'utf8');
rmSync(tempRoot, { recursive: true, force: true });

if (result.p95_ms > budgetMs) {
  console.error(summary);
  process.exit(1);
}

console.log(summary);
