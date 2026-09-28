# Turkish assistance spot checks — 2026-09-28

Installed KebabM 1.0.3, Gemini 3.8 Flash, Crosswalk saved index revision 9519450d7ad8afa8611d4ae1197b55b999e01355. Calls used the native MCP assistance path; no audio capture was started. The answer language was temporarily set to Turkish and restored to English; the test meeting ended successfully.

| Scenario | Action | Latency (ms) | Evidence references | Original evidence preserved |
|---|---|---:|---:|---|
| CI pipeline | assist | 5171 | 8 | N/A |
| CI pipeline | short | 2425 | 8 | Exact match |
| CI pipeline | followup | 5600 | 8 | Exact match |
| Unavailable live metrics | assist | 5667 | 8 | N/A |
| CI pipeline | short | 3221 | 8 | Exact match |
| CI pipeline | followup | 4284 | 8 | Exact match |

## Review

- All six outputs were in Turkish. Pipeline shortening retained the central build/deployment distinction in two sentences. Follow-up returned three single-sentence questions.
- Asking for current production active-user count and the exact last-hour error percentage produced an explicit statement that the available sources do not contain those current metrics. The shortened answer preserved that limitation.
- The live-metrics answer additionally mentioned historical account counts and diagnostic query windows. Inspection of the cited handoff and diagnostic pack supports those historical details, but the extra counts can distract from the unknown current state. Do not interpret them as current telemetry or online-session counts.
- All 16 distinct evidence snapshots resolved through the installed app. References on refinements matched their selected source answer exactly. Both initial answers reported searchDegraded=false.
- Raw local evidence: /tmp/kebabm-turkish-grounding.json and /tmp/kebabm-turkish-grounding-sources.json. Source contents are intentionally not copied into this report.

## Limits

These are two scenarios and six requests, not the 50-scenario acceptance corpus, a latency percentile result, or a factual-accuracy percentage. Source freshness remains unavailable. Native UI clicks, real audio and sustained meeting behavior are separate unresolved acceptance requirements.

## Live-metrics policy follow-up

The project evidence policy now explicitly distinguishes historical account counts, online users, configured diagnostic queries and actual current measurements. Historical comparisons require a request and a stated date/environment. The installed build was retested with the same live-metrics question:

- Assist: 3,749 ms; explicitly unavailable current metrics; no historical user counts inserted.
- Shorten: 2,820 ms; retained the current-data limitation and exact evidence list.
- FollowUp: 5,570 ms; three questions about access/measurement; exact evidence list retained.
- Inspected cited `lib/ops/status-probes.ts`: it does define account/session count queries, so the proposed session-measurement route has repository support. No query was executed against a live company database. Unexpired sessions are not proof of currently online people. APM/log inspection remains a proposed measurement step, not evidence that a particular APM is configured.

Evidence: `/tmp/kebabm-live-metrics-policy.json`. This is a single before/after case, not proof that every unsupported question is handled correctly.
