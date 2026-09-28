# Installed retrieval quality — 2026-09-28

This checkpoint evaluates source retrieval, not answer correctness, automatic
question detection or real Zoom audio. Company fixtures and result passages
remain in ignored local files.

## Reproduction

```sh
node scripts/evaluate-live-knowledge.mjs PROJECT_ID REPO tests/fixtures/crosswalk-golden.json artifacts/crosswalk-retrieval-diagnostics.json
```

The runner requires a clean repository at the fixture revision, validates source
anchors before making embedding requests, and checks the expected file and
anchored line among the first eight returned passages. It records all 50 cases;
only the 40 answerable cases contribute to recall. The other ten still need
separate answer review. An interrupted report has `complete: false`.

MCP now supports optional `includeDiagnostics: true` on `search_knowledge`,
returning hits, degradation/reason and retrieval duration. The default array
contract remains unchanged. Installed checks verified the legacy array and
rejection of a non-boolean diagnostic flag. Seven MCP tests and the native
release build passed; the installed app signature was verified.

## Baseline

The first 50-query installed run, before diagnostics, retrieved the expected
passages for 29/40 answerable questions (72.5%). This fails the 90% acceptance
target. Because the legacy endpoint discarded retrieval degradation, that run
alone cannot establish semantic-service success for each request.

Misses cluster around product scope, human approval, handoff boundaries and
operational caveats. Do not fix this by weakening fixture expectations or
claiming matching filenames alone are sufficient.

## Next controlled experiment

The current embedding query always requests `task: code retrieval`, including
product and business questions. Google's official embedding documentation
separates code retrieval and question-answering query instructions while using
the same document format. Compare the question-answering task on the complete
pinned set before adopting it; the observed misses do not yet prove this is the
cause. Also inspect fused ranks and duplicate/source diversity. Preserve code
question performance and do not add expected filenames to production queries.

Reference: https://ai.google.dev/gemini-api/docs/embeddings#task-types

The full user goal remains open, including retrieval quality, grounded answer
review, response latency and native meeting/audio acceptance.

## Diagnostics rerun

All 50 requests completed with `degraded: false`. Expected passage recall remained
29/40 (72.5%); the same 11 answerable cases missed. This reproduces a ranking/
retrieval quality gap even when semantic retrieval succeeds. Retrieval-only
latency was median 3731.5 ms and p95 4248 ms; these are not end-to-end answer latencies.

Private evidence: `artifacts/crosswalk-retrieval-diagnostics.json`; execution log:
`/tmp/kebabm-retrieval-diagnostics.log`. Exit code 1 is the expected failed quality
gate, not a crashed evaluation. No answer review was performed in this run.

## Controlled query-task experiment

Changed only the Gemini query prefix from code retrieval to question answering;
the indexed document vectors, fixture revision and ranking code were unchanged.
All 50 queries completed without degraded search. Recall fell from 29/40 to
28/40 (70%). Cases cw-20 and cw-30 regressed; cw-38 improved. This does not support
adopting the query-task change. One example also ranked a mailbox stylesheet
first for a pipeline question; query-task selection alone does not ensure
useful source ordering.

Reverted the source change and restored the verified pre-experiment application.
Installed executable SHA-256 equals the pre-experiment backup. The experimental
release build under target/release remains an experiment and must not be shipped;
rebuild from current source before packaging another release.

Evidence: private `artifacts/crosswalk-retrieval-qa-task.json`,
`/tmp/kebabm-retrieval-qa-task.log`, `/tmp/kebabm-qa-retrieval-package.log`.
Two embedding integrity tests passed. No quality threshold was relaxed.
Next investigation: per-channel ranks, explanatory-source coverage and rank
fusion. The lower-quality task prefix is not the current installed behavior.

## Lexical candidate audit

Added `src-tauri/examples/inspect_retrieval.rs` to inspect the existing index with
a read-only SQLite connection. It saves the first 40 lexical candidates and each
expected source's position to a private report. This does not modify the app DB
or send source content to a provider.

The saved index's lexical recall@8 is 34/40. Among the 11 installed hybrid misses,
seven expected passages are already at lexical positions 1–6; three are absent
from the first 40 and one is at position 28. Thus candidate generation and fusion
both need investigation; the seven fusion losses cannot be explained solely by
missing documents. Evidence: `artifacts/crosswalk-lexical-candidates.json` and
`/tmp/kebabm-lexical-candidates.log`.

## Fusion rank sensitivity experiment

Changed only the RRF smoothing constant from 60 to 10 to give the strongest
single-channel ranks more influence. All 50 requests completed without degraded
search. Recall became 30/40: cw-09, cw-17 and cw-38 improved, but cw-20 and cw-25
regressed. The small net improvement does not justify losing these previously
working technical answers. Rejected the candidate and restored source and the
signature/hash-verified previous installed app. Four knowledge integrity tests
passed. This experiment was reverted before the next package build.

Evidence: `artifacts/crosswalk-retrieval-rrf10.json`,
`/tmp/kebabm-retrieval-rrf10.log`, `/tmp/kebabm-rrf-rank-package.log`,
`/tmp/kebabm-rrf-knowledge-tests.log`. Investigate candidate coverage and
source diversity rather than further tuning this constant against the same set.

## Lexical evidence reservation candidate

Current source reserves up to two of eight evidence slots for the strongest
lexical matches, then fills remaining slots from the existing RRF results with
content deduplication. Offline replay of saved candidates reaches 32/40, with
three gains and no losses against the 29/40 baseline. This is not a live quality
result. Five knowledge tests pass, including the new evidence retention and
deduplication regression. The current macOS app package builds successfully.
The full installed-app evaluation and Windows runtime acceptance remain pending.
