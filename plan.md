# ZaiqoM-MeetingHelper implementation status

## Implemented

- Request-scoped answer lifecycle, cancellation, error recovery and queued answers.
- Query-based project knowledge, atomic indexing, source revision/hash/line metadata and semantic adapters.
- Project preparation, open questions and reviewed decision memory.
- MCP management with typed tools, validation and isolated background jobs.
- Session credential cache to reduce repeated Keychain reads; existing storage identity preserved.
- Eight appearances: IBM, Liquid Glass, Apple, Linear, Notion, Material, GitHub, Terminal; independent light/dark/system mode.
- New generated ZaiqoM identity across UI and desktop icons.
- Frontend, native and dependency upgrades; browser, unit and MCP coverage.

## Release v1.0.3

Build the same tagged revision for macOS ARM64, Windows x64 and Linux x64. Publish installers only after all packaging jobs pass, with SHA-256 checksums. This is the first ZaiqoM release; inherited upstream version tags are unrelated.

## Remaining acceptance

### Active reliability pass — 2026-09-28

- Fixed explicit answer refinements staying hidden behind the queued-answer selector. Toolbar actions and keyboard shortcuts now use the selected answer and its original evidence; automatic arrivals still wait.
- Removed conflicting instructions that added follow-up lists to shortened answers; corrected the MCP Say mode mapping.
- Browser regression checks: seven preparation/answer workflows passed, including selected historical answer refinement, error/retry and automatic queue preservation. Frontend checks passed (25 unit tests, 7 MCP tests, production build).
- Real semantic indexing encountered provider HTTP 429. Configuration alone is not semantic readiness. Added per-batch embedding checkpoints so a later provider failure does not discard completed work; checkpoint regression tests passed (2 tests), and the native app bundle built successfully. The preparation UI shows saved passage counts and offers resume after failure.
- Installed the corrected local v1.0.3 build at `/Applications/ZaiqoM-MeetingHelper.app`; binary UUID matches the produced bundle and local signature verification passed. Backed up the prior app and data. Startup was blocked by an OS Keychain read; process sampling showed async workers waiting on the credential mutex. Subsequent source changes move credential commands off async workers, avoid unrelated translation/embedding credential reads, and keep MCP status responsive while credentials are busy. These subsequent changes passed 81 native unit tests and 4 compatibility tests; the interactive OS Keychain test remains intentionally excluded. A new app bundle is being built and still requires installation.
- Search limitations now carry their exact reason through streaming and answer history: keyword-only selection, missing/partial index, provider failure, or live lookup timeout. Missing indexes avoid unnecessary query embedding calls. The 400px browser screenshot was reviewed with the new message.
- Still required: exercise real AI refinements with preserved citations/language, finish semantic indexing and prove hybrid retrieval, make preparation/progress/errors actionable, and review the entire live-assistance experience. macOS credential access was subsequently moved to an owner-only local file at the user’s explicit request; see the verification below.

- Real 60-minute meetings on macOS and Windows, audio device changes and language switching.
- Full Linux audio/runtime verification.
- Measured source recall and factual answer quality on a user-owned private dataset.
- Native glass compositing across platforms.
- Trusted platform signing/notarization, signed updater and rollback validation.

Private company datasets and local pilot records are retained locally and are not part of the public repository. Build success does not establish meeting acceptance or answer quality.

### macOS no-Keychain storage — 2026-09-28

- User explicitly requested no more Keychain prompts. macOS now uses a local plaintext credential document restricted to the current account (0700 directory, 0600 file), with atomic replacement. No automatic Keychain lookup or migration occurs. Windows/Linux retain their existing adapters.
- Gemini credential transferred from the previously authorized value to the private local store without reading Keychain. Secret contents are excluded from this document and repository.
- New adapter tests cover persistence across manager recreation, multiple providers, deletion, filesystem permissions, corrupt-file preservation and rejection of symlink/public-directory targets. All 81 unit, 4 compatibility and 2 local credential integration tests passed. The app was built, installed in /Applications, locally signed and signature-verified. Two native launches opened normally without a Keychain prompt; the stored Gemini credential remained available and the app MCP real-model connection test passed. The local build remains version 1.0.3 and has not been published as a new release.

### Calendar setup and calmer meeting workspace — 2026-09-28

- Default launcher now opens Today: upcoming meetings and recent notes. Projects and All notes remain separate tabs; service diagnostics are collapsed. Added Notebook, the ninth appearance, with warm paper/olive colors and light/dark variants. Existing appearance choices remain available.
- Added Google Calendar Desktop OAuth setup, PKCE/state-checked loopback authorization, cancellation, token refresh, primary-calendar seven-day agenda, conference links and preparation with the event title/time. Calendar scope is read-only. MCP exposes status, agenda and local disconnect without returning credentials.
- No Desktop OAuth client exists yet (confirmed by user). Setup instructions and fields are delivered; live Google consent, refresh and account data remain unverified until configured. See docs/google-calendar.md. macOS uses the existing no-Keychain local credential adapter.
- Verification: 27 frontend unit tests, 7 MCP tests, 86 Rust unit tests, 4 compatibility tests, 2 local credential integration tests and the full 17 browser tests passed. The 4 Calendar browser tests were rerun after the Notebook fixture correction and passed. Screenshot review covered 400, 700 and 1440 px. Frontend production build passed; existing dynamic-import bundler warnings remain.
- macOS release build and app bundle succeeded. Installed locally in /Applications after backing up the previous app to ZaiqoM-Install-Backups/20260928-074729-calendar; local ad-hoc signature verified. Running installed app MCP confirmed Calendar disconnected (no OAuth client) and accepted Notebook/light preferences. Native screenshot verification was blocked by CUA `cgWindowNotFound`; browser fixture screenshots passed. This remains a local 1.0.3 build, not a newly published release.

### KebabM identity — 2026-09-28

- Renamed the visible product, window/tray/onboarding/settings labels, OAuth callback copy, MCP server identity and package display name to KebabM. GitHub repository renamed to aliozkanozdurmus/KebabM; origin and current documentation links updated.
- Generated a new olive/ivory/orange K logo with ChatGPT image generation and applied it to the shared BrandMark, favicon and native desktop icon formats. Source and prompt are documented in docs/design/logo-prompt.md.
- Preserved com.nexq.app, credential namespaces and local control/event identifiers so existing data, provider settings and MCP clients continue to work.
- 27 frontend unit, 7 MCP and 17 browser tests passed; updated 400/700/1440 px screenshots generated and reviewed. Production frontend build passed. Historical release/pilot records retain their original product names.
- Built and installed /Applications/KebabM.app; the old named app was moved to the timestamped installation backup. Ad-hoc signature verified. Installed app MCP reports name KebabM and version 1.0.3. Native screenshot remains unavailable (`cgWindowNotFound`); browser visual verification passed. No new release was published.

### Live assistance verification in KebabM — 2026-09-28

- Real installed Gemini 3.8 Flash call completed with a Crosswalk CI answer in 10,656 ms. All 8 attached evidence snapshots resolved through MCP. This is one working sample, not a factual-quality or latency acceptance pass.
- Resumed actual Gemini semantic indexing of 17,334 passages; initially no vectors existed. Confirmed durable checkpoints growing beyond 2,976. Job remains live; do not restart merely because a monitoring call expires. Current process output: /tmp/kebabm-semantic-resume.log, exec session 54408.
- Preparation status exceeded its client deadline. Native process sample traced the Git child to a macOS getcwd/open wait, not a proven fsmonitor issue. Added a 15-second child-process bound, concurrent output draining and an explicit failure preserving the index; 4 knowledge regression tests pass. This source change is not installed yet.
- Added MCP refine_answer selecting an exact canonical session answer and carrying its immutable evidence into Shorten/FollowUp. Missing/empty answer IDs and invalid modes are rejected. Selection test and 7 MCP tests pass. Native real-model refinement verification remains pending installation of the new build; keep running semantic work intact first.
- Current verification meeting: d406166a-d82b-4d3b-9dd7-352ebe36ce3a (no capture); initial answer: 186f64ef-4ea3-4fc8-acd4-c6c19d150983. New app package build output: /tmp/kebabm-assist-package.log. No global goal completion claimed.

### Native refinement and transport verification — 2026-09-28 08:07 EDT

- Installed the built KebabM app with `refine_answer` and bounded Git discovery. Previous app backed up under `~/Library/Application Support/ZaiqoM-Install-Backups/20260928-080204-assist/`; ad-hoc signature verified. This is a local install, not a published release.
- Real Gemini 3.8 Flash: initial pipeline answer completed in 5,065 ms; Shorten in 3,445 ms (two sentences); FollowUp in 6,955 ms (three questions). Both refinements selected the original answer and preserved all eight evidence objects exactly. All eight evidence IDs resolved through native MCP. Evidence: `/tmp/kebabm-native-refinements.json`. Single samples, not latency/quality acceptance statistics.
- Native cancellation/recovery passed: cancelled request added no answer; immediate subsequent request succeeded. Answer count increased from 3 to 4 only for recovery. Evidence: `/tmp/kebabm-native-cancel-recovery.json`. Verification meeting `9e7b2e2a-0f08-498b-86ed-f1630c6e9b90` ended; audio was not captured.
- Previous embedding client session 54408 terminated with a desktop connection error at 3,856 saved vectors. No active network operation remained. Resumed from checkpoints with session **16806**, log `/tmp/kebabm-semantic-http-resume.log`; now 6,880 / 17,334 vectors and confirmed running. Do not restart the app or launch duplicate embedding while this handle is live.
- MCP loopback transport now uses `node:http` with the explicit operation deadline, avoiding fetch's separate response-header deadline for long jobs. Tests cover delayed response, explicit timeout, socket cancellation, private configuration validation and secret suppression. Seven MCP tests passed. Long live indexing still needs terminal success verification.
- Preparation status now returns instead of hanging, but reports freshness unavailable: the installed app's Git process waits in macOS directory access. Source now propagates the actual freshness error to the UI instead of hiding its reason. Need successful folder access/freshness verification.
- Follow-up wording tightened to short, single-focus questions; freshness error changes built successfully in `/tmp/kebabm-followup-package.log`. This latest bundle is **not installed**, to avoid interrupting active embedding.
- 27 frontend tests and production build passed. Native UI automation still reports `cgWindowNotFound`; requested user check whether Mac/app is visible or blocked by folder permission.
- Browser test startup reproduced a macOS FSEvents `open` wait. Added explicit optional polling settings in Vite (only when CHOKIDAR_USEPOLLING=1). Rerun passed all 11 workflow scenarios at 400/700/1440 px, including shortening selected historical answers and error/retry. Log `/tmp/kebabm-assist-workflows-explicit-polling.log`. Original and initial polling attempts interrupted due to confirmed watcher hang; browser fixtures do not prove native button clicks.
- Goal remains active: finish semantic index and prove hybrid retrieval; install and verify concise follow-ups after indexing; resolve source folder access and native UI visibility; complete broader live audio/meeting and reliability acceptance. Do not claim every feature is working from these bounded tests.

### Speech readiness and preserved provider selection — 2026-09-28

- Readiness now checks each meeting party's actual provider/device instead of the unrelated legacy global STT preference. It distinguishes browser support, downloaded local models and cloud connectivity from successful live recognition. The native service no longer claims it can verify frontend-only Web Speech. Twelve browser workflow tests passed (`/tmp/kebabm-readiness-workflows-final.log`).
- Actual selected Them provider was Deepgram with no stored Deepgram key; the only configured cloud key was Gemini. Downloaded multilingual Whisper small through MCP. Download completion alone was not treated as recognition proof.
- Local Whisper startup now waits for model/state initialization and propagates corrupt-model, initialization and timeout failures. The corrupt-model regression test passed. Missing per-party Whisper models now return an actionable startup error rather than silently returning no provider.
- Real local-model smoke test recognized the complete synthetic English pipeline question, then emitted the same text as a final transcript after silence. Test passed in 3.60 seconds, including model initialization (`/tmp/kebabm-whisper-real-final.log`). This proves the local provider path with a fixture, not microphone/Zoom capture, Turkish accuracy or sustained meeting performance. Repeatable opt-in test accepts local model/PCM/expected-phrase environment variables.
- Removed legacy startup migration that silently replaced selected Whisper providers with Web Speech/Deepgram. Regression test verifies both party selections and independent transcription/answer languages survive startup. All 28 frontend unit tests passed (`/tmp/kebabm-stt-selection-tests.log`).
- New package build: `/tmp/kebabm-stt-selection-package.log`. Installation and provider selection are pending terminal completion of embedding session 16806; do not interrupt its live network operation. Last verified count exceeded 15,680 / 17,334. Native audio capture is off.

### Completed semantic index and installed speech fixes — 2026-09-28 08:23 EDT

- Embedding session 16806 exited successfully: 13,478 additional passages embedded, 17,334 total durable vectors. Native knowledge status confirms Gemini embedding-2 ready=true with embedded=total=17,334. No duplicate job was started.
- Built and installed the speech/selection/readiness/follow-up changes into `/Applications/KebabM.app`. Existing bundle backed up in the timestamped `ZaiqoM-Install-Backups` directory. Initial signature validation failed; re-signed the local bundle ad hoc and deep/strict verification passed. No release published.
- Configured both meeting parties to downloaded Whisper small using MCP, preserving devices, independent language settings and recording=false. MCP returned no warnings. Provider-selection startup regression is covered by the frontend test.
- Actual installed Gemini answer completed in 5,616 ms with `searchDegraded=false`, eight resolvable sources. Shorten completed in 4,381 ms; FollowUp in 5,212 ms, returning three concise single-sentence questions. Both preserve the original evidence objects exactly. Evidence: `/tmp/kebabm-complete-semantic-answer.json`, `/tmp/kebabm-complete-semantic-evidence.json`, `/tmp/kebabm-semantic-short.json`, `/tmp/kebabm-semantic-followup.json`. Verification session f8d2c9b2-af97-4f42-ab47-99cebb0d29a3 ended cleanly.
- Source freshness is still unavailable: native folder inspection reaches its bounded timeout. The installed UI now receives the actionable macOS folder-access error. Semantic readiness proves the saved snapshot is searchable, not that the snapshot matches current source files.
- Native capture smoke test failed before playback: start_capture timed out while opening the microphone. Process sampling shows CoreAudio `AudioDeviceCreateIOProcID` / `HALC_ProxyObject::SetPropertyData` waiting in mach_msg, called synchronously from start_capture_per_party while holding the audio manager lock. Stop capture consequently could not acquire that lock. Terminated the test app process and reopened the installed bundle; no audio-file recording was enabled. Evidence: `/tmp/kebabm-native-audio-sample.txt`, `/tmp/kebabm-native-local-audio-smoke.json`. Next work must address bounded/cancellable device startup and OS device availability; do not label this a passed real-audio/Zoom test.

### Bounded native input startup and recovery — 2026-09-28 08:30 EDT

- Microphone/shared-input streams now remain on their owning worker thread. Device opening has a 15-second deadline; timed-out workers cannot publish a late stream or deliver audio, and another opener cannot accumulate behind an unresponsive driver. Stream shutdown signals its worker instead of destroying CPAL streams under the shared audio lock. System audio shutdown also has a two-second join bound.
- Preserved detailed speech-provider startup errors instead of replacing model failures with generic credential advice. Thirteen native audio tests passed, including delayed-open cleanup and normal owner-thread shutdown. `git diff --check` passed.
- Built and installed the updated app; backup `~/Library/Application Support/ZaiqoM-Install-Backups/20260928-082931-bounded-input/KebabM.app`, ad-hoc signature verified. Build: `/tmp/kebabm-bounded-input-package.log`.
- Reproduced the actual CoreAudio failure on the installed bundle: explicit microphone timeout returned after 15,359 ms. Subsequent stop_capture succeeded in 2 ms, audio_status reported capturing=false in 1 ms, and end_meeting succeeded in 2 ms, without restarting the application. Evidence: `/tmp/kebabm-bounded-input-native.json`.
- This fixes indefinite input opening and recovery, not the external device failure itself. A synchronous caller can still wait up to the bounded device deadline; immediate cancellation during opening is not proven. Native UI remains unavailable to automation (`cgWindowNotFound`). Real microphone/system speech and a sustained Zoom session remain unverified.

### Readiness result isolation and AI recovery evidence — 2026-09-28

- Inspected `/tmp/kebabm-ai-after-audio-failure.json`: the installed app answered after the bounded microphone failure in 6,405 ms, with eight evidence references and searchDegraded=false. This demonstrates AI recovery after that device failure; it does not prove functioning microphone capture.
- Reproduced a readiness race in two browser scenarios: changing the selected party provider during a pending connection test allowed the old success or error to appear under the new configuration. Both regressions failed before the fix.
- Readiness now versions the tested settings and discards stale results/errors. A synchronous in-flight guard prevents duplicate checks; audio startup also checks whether its configuration is still current before continuing the signal test. Local model preference changes invalidate readiness results too.
- All 14 browser workflow tests passed, including both regressions and existing refinement/retry workflows (`/tmp/kebabm-readiness-race-after.log`). Production frontend build passed (`/tmp/kebabm-readiness-race-build.log`); diff whitespace check passed. This readiness change is in the source workspace, not yet installed or published.
- Remaining acceptance gaps include native button interaction, OS microphone/system capture, source freshness and sustained real meeting verification. No overall completion claim.

### Installed readiness isolation — 2026-09-28 08:36 EDT

- Built the current app bundle successfully (native release compilation 52.64 seconds). Installed `/Applications/KebabM.app` after confirming capture=false and no active session. Previous bundle preserved at `~/Library/Application Support/ZaiqoM-Install-Backups/20260928-083653-readiness/KebabM.app`. Local ad-hoc signing and deep/strict signature verification passed.
- Restarted app MCP reports KebabM 1.0.3, credentials ready, Gemini 3.8 Flash retained, both party Whisper small selections retained, recording disabled and capture inactive.
- Native window inspection again returned `cgWindowNotFound`. Therefore real native refinement button clicks remain unverified despite passing browser workflows and native MCP refinement calls. A past interrupted test meeting remains in history with no end time; no current in-memory session is active.
- Package evidence: `/tmp/kebabm-readiness-package.log`. No push or new public release performed.
- Installed app real-model connection check returned `ok: true` for Gemini 3.8 Flash (`/tmp/kebabm-readiness-installed-ai.json`). This is connection validation, not a new answer-quality acceptance run.

### Turkish real-model assistance spot checks — 2026-09-28

- Six installed-native requests covered two Turkish questions, each followed by Shorten and FollowUp: pipeline behavior and deliberately unavailable live production metrics. All completed, both initial answers used non-degraded retrieval, and all four refinements preserved their selected evidence exactly.
- The live-metrics answer and its shortened version explicitly stated that current metrics are unavailable. It also included historical account counts, verified against the cited handoff; these are not current telemetry. All 16 unique evidence snapshots resolved.
- Saved the bounded review and exact latencies in `docs/acceptance/turkish-assistance-20260928.md`. No broad quality score or percentile inferred from this small sample. Restored the original English reply preference and ended the test meeting.

### Native interaction blocker revalidation — 2026-09-28

- CUA inventory lists installed KebabM as running. Resolving by bundle identifier is ambiguous because backups share the identifier; resolving the exact installed path still returns cgWindowNotFound.
- Finder Desktop accessibility and screenshot initially worked. A subsequent Finder Go to Folder keyboard interaction also returned cgWindowNotFound. This is broader than a proven KebabM-only UI failure, but does not establish an OS root cause.
- Process sample of installed PID 63360 shows the main thread in the normal AppKit event loop, rather than blocked in audio initialization. Evidence `/tmp/kebabm-window-sample.txt`. The native backend remains responsive from prior/current MCP checks.
- Asked the user whether the installed app is visibly open on an unlocked Mac, with the exact observed window and CoreAudio failures. Native UI/audio acceptance remains pending that environment clarification. No claim that the UI is functional merely from process liveness.

### Current metrics versus historical sources — 2026-09-28 08:43 EDT

- Tightened the shared project evidence policy after observing historical account counts in an answer requesting current telemetry. Old counts, account status and configured query text must not substitute for current live measurements; explicit historical comparisons must preserve date/environment limitations, including refinements.
- Built and installed the change; local signature verified. Previous bundle: `~/Library/Application Support/ZaiqoM-Install-Backups/20260928-084355-grounding-policy/KebabM.app`. Build log `/tmp/kebabm-grounding-policy-build.log`.
- Repeated the same Turkish live-metrics question through installed MCP: Assist 3,749 ms, Shorten 2,820 ms, FollowUp 5,570 ms. No historical counts inserted, unavailability preserved, refinement evidence matched exactly. Verified the cited session-query source, without executing live company queries. Details in `docs/acceptance/turkish-assistance-20260928.md`.
- Test meeting ended and English reply preference restored. Native Mac interaction/audio blocker remains unresolved; no release or overall acceptance claim.

### Source viewer request ordering — 2026-09-28
- Reproduced three real component failures with deferred Tauri responses: old source success/error appearing after selecting another answer, and an older read replacing a newer completed read.
- The answer panel now scopes source reads to the displayed answer and a monotonically increasing request identity. Selection changes and unmount invalidate pending reads; stale completions cannot change content, errors, or loading state.
- Browser validation: all 17 workflow tests passed, including Shorter/Follow-up, selected historical answer evidence, readiness invalidation, and the three new regressions. Reviewed the 400 px source-viewer screenshot. Native microphone/Zoom validation remains separate and incomplete.
- Evidence: `/tmp/kebabm-source-race-before.log`, `/tmp/kebabm-source-race-after.log`.

### Punctuation-free English question fallback — 2026-09-28
- Confirmed native window access still fails with `cgWindowNotFound`; no successful microphone/Zoom acceptance is claimed.
- Found an actual fallback gap: direct English questions without STT punctuation scored 0.6 and were discarded by the session's 0.75 fallback threshold when semantic classification failed/timed out.
- Added conservative direct-question syntax recognition while keeping declarative clauses below the fallback threshold. Regression samples cover 10 direct questions and 10 declarative counterexamples, including "what we are reviewing".
- The positive test failed before the fix. All 7 intelligence tests passed after it (`/tmp/kebabm-detector-before.log`, `/tmp/kebabm-detector-after.log`). These curated examples are not a precision/recall benchmark or a live speech test.

### Native window versus automation state — 2026-09-28
- Added read-only `status.windows` flags and `active_session_id` to the authenticated control API. Documented that legacy `open_meeting` is a persisted unfinished row, not proof of an active meeting or capture.
- Built and installed the app with verified ad-hoc signature; 7 MCP tests passed. The new installed status returned launcher visible=true, minimized=false, focused=false; overlay visible=false; active_session_id=null; audio capture=false. A stale unfinished saved meeting still exists and was preserved.
- Evidence: `/tmp/kebabm-runtime-window-state.json`, `/tmp/kebabm-runtime-status-mcp-tests.log`, `/tmp/kebabm-runtime-status-package.log`.
- Finder's New Window action also returned `cgWindowNotFound`. The app's launcher is not hidden/minimized according to its native API, but native screen/accessibility interaction is still unavailable. This narrows the blocker without asserting a specific macOS root cause. User-visible Mac verification remains pending.

### Full current-state verification — 2026-09-28
- Recorded the requirement-by-requirement checkpoint in `docs/acceptance/runtime-audit-20260928.md`.
- Current checks: 92 Rust passed / 1 ignored; 28 frontend passed; 7 MCP passed; 23 browser passed; production frontend build succeeded.
- Installed semantic status: 17,334/17,334 ready, saved snapshot; freshness unavailable. Sampled the actual app-spawned Git child blocked in getcwd/open, while the identical terminal command completes in 0.04 s. No speculative permission change applied.
- Native meeting acceptance and broad quality/latency benchmarks remain unproven. Goal is not complete.

### Installed retrieval benchmark — 2026-09-28
- Added optional MCP search diagnostics without changing default hit-array clients, plus a reproducible private-fixture live evaluator.
- Built, installed and signature-verified the diagnostics build. Seven MCP tests passed; installed legacy response and invalid flag checks passed.
- Pinned clean Crosswalk revision: 50 requests completed, all non-degraded. Expected source passage recall is **29/40 (72.5%)**, below 90%. This is an open quality failure, not completion. Details: `docs/acceptance/retrieval-quality-20260928.md`.
- Next: compare question-answering query embeddings with the current universal code-retrieval task, inspect fusion/source diversity, rerun the entire set without weakening expectations. Answer-quality and native meeting gates remain open.

### Query-task comparison — 2026-09-28
- Completed the full 50-query controlled experiment with `question answering` instead of `code retrieval`; no degraded requests, but source recall fell from 29/40 to 28/40 (cw-20/cw-30 lost, cw-38 gained).
- Rejected the change, reverted source, restored the signature-verified pre-experiment application and verified executable hash parity. Rebuild current source before future packaging; target/release still contains the experimental build.
- Evidence and next investigation are in `docs/acceptance/retrieval-quality-20260928.md`. Ranking/source selection remains an open quality failure; task-prefix substitution is not a fix.

### Fusion diagnosis — 2026-09-28
- Added a read-only existing-index lexical candidate audit example. Lexical recall is 34/40; seven of the eleven hybrid misses had the correct passage at lexical rank 1–6. Three were absent from the first 40; one was at rank 28.
- Full 50-case RRF smoothing experiment: 30/40, no degraded queries; three gains but two technical regressions. Rejected and restored source plus verified previous installed app. Four knowledge integrity tests passed.
- Next: examine source diversity and candidate coverage, not further constant tuning on the same fixtures. Current source and installed behavior retain baseline; target/release must be rebuilt before shipping.
