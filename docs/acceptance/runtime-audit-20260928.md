# Installed runtime acceptance audit — 2026-09-28

This is a verification checkpoint, not full meeting acceptance.

| Requirement | Current evidence | Conclusion |
| --- | --- | --- |
| Native regression suite | 92 passed, 0 failed, 1 ignored; `/tmp/kebabm-full-native-audit.log` | Passing automated suite. Ignored test requires real Whisper model/audio fixture. |
| Frontend regression, MCP, production frontend build | 28 frontend tests, 7 MCP tests passed; build succeeded; `/tmp/kebabm-full-frontend-audit.log` | Passing. Build has existing ineffective dynamic-import warnings. |
| Browser interaction/layout suite | 23 passed; `/tmp/kebabm-full-browser-audit.log` | Includes refinements, evidence selection, readiness, calendar, appearances and responsive layouts using mocked Tauri. Not native window proof. |
| Saved semantic index | Installed API reports 17,334/17,334 vectors ready; 4,639 indexed files; Gemini embedding-2, 1536 dimensions | Semantic index is configured and complete for the saved snapshot. |
| Repo snapshot freshness | revision 9519450d7ad8afa8611d4ae1197b55b999e01355; current installed freshness=unavailable | Cannot claim current source contents match the saved snapshot. |
| Native repo inspection | Child Git `ls-files -z --cached --others --exclude-standard` timed out; sampled in `setup_git_directory_gently → strbuf_getcwd → __private_getcwd → open$NOCANCEL`. Same /usr/bin/git command from terminal returned 4,889 paths in 0.04 s. | App-context filesystem access is the bounded finding. A particular macOS permission root cause is not proven. |
| Native window | App reports launcher visible, not minimized; CUA returns cgWindowNotFound and Finder New Window also fails | Native visual/accessibility acceptance remains blocked; manual Mac visibility response is pending. |
| Real speech and meeting use | No completed native microphone/system-audio → live transcript → automatic answer → UI interaction run | Not accepted. Synthetic/model/HTTP tests do not replace this. |
| Broad answer quality / latency | Earlier limited real-provider samples are documented separately | No 50-case quality, p95 latency, or 60-minute meeting acceptance claim. |

Filesystem diagnosis: `/tmp/kebabm-git-child-sample.txt`; status evidence: `/tmp/kebabm-current-knowledge-audit.json`. No repository contents, OS permissions, or company services were changed in this audit. A saved unfinished meeting remains preserved; active_session_id is null and capture is false.
