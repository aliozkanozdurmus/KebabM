// Development-only entry: exercises the production components through Tauri's official mock IPC.
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import React from 'react';
import { createRoot } from 'react-dom/client';
import '../../src/index.css';
import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/ibm-plex-mono/400.css';
const project = { id: 'crosswalk', name: 'Crosswalk', root_path: '/selected/waste-harmonics', is_active: true, status: 'ready', created_at: '2026-09-26T10:00:00Z' };
const evidence = { id: 'source-one', projectId: project.id, path: '.github/workflows/ci.yml', startLine: 1, endLine: 12, revision: 'a123456789', contentHash: 'hash', sourceType: 'code', dirty: false };
const coverage = { revision: 'a123456789', scannedAt: '2026-09-26T10:00:00Z', indexedFiles: 132, reusedFiles: 120, chunks: 740, gitAware: true, searchMode: 'lexical', freshness: 'stale', excluded: [{ path: '.env', reason: 'secret file' }] };
const semanticFixture = new URLSearchParams(location.search).has('semantic');
let embedded = 32;
const questions: unknown[] = [];
const assistRequests: Record<string, any>[] = [];
const sttTests: string[] = [];
let releaseStt: (() => void) | undefined;
let holdStt = false;
let holdEvidence = false;
const pendingEvidence: { resolve: (text: string) => void; reject: (error: string) => void }[] = [];
const saved = JSON.parse(localStorage.getItem("fixture-config") ?? "{}");
let calendarConnected = new URLSearchParams(location.search).has('calendar');
let fail = false;
mockWindows('main', 'overlay');
mockIPC(async (cmd, input) => {
  const args = input as Record<string, any> ?? {};
  switch (cmd) {
    case 'plugin:store|load': return 1;
    case 'plugin:store|get': return [saved[args.key], args.key in saved];
    case 'plugin:store|set':
      saved[args.key] = args.value;
      localStorage.setItem('fixture-config', JSON.stringify(saved));
      await emit('store://change', { resourceId: 1, key: args.key, value: args.value, exists: true }); return;

    case 'calendar_status': return { connected: calendarConnected, scope: 'Read-only calendar events', clientId: calendarConnected ? 'fixture.apps.googleusercontent.com' : undefined };
    case 'calendar_connect': if (fail) throw 'Google Calendar permission was not granted.'; calendarConnected = true; return;
    case 'calendar_disconnect': calendarConnected = false; return;
    case 'calendar_events': if (fail) throw 'Calendar could not refresh. Check your connection and retry.'; return { syncedAt: new Date().toISOString(), truncated: false, events: [
      { id: 'event-1', title: 'Crosswalk delivery review', start: new Date().toISOString(), end: new Date(Date.now()+3600000).toISOString(), allDay: false, attendeeCount: 4, joinUrl: 'https://meet.google.com/example' },
      { id: 'event-2', title: 'Planning & open questions', start: new Date(Date.now()+7200000).toISOString(), end: new Date(Date.now()+10800000).toISOString(), allDay: false, attendeeCount: 2 },
    ] };
    case 'test_stt_connection': sttTests.push(args.provider); if (holdStt) await new Promise<void>(resolve => { releaseStt = resolve; }); if (args.provider === 'deepgram') throw 'No Deepgram API key configured'; return true;
    case 'list_audio_devices': return JSON.stringify({ inputs: [], outputs: [] });
    case 'list_local_stt_engines': return '[]';
    case 'get_audio_sessions': return '[]';
    case 'list_projects': return JSON.stringify([project]);
    case 'list_context_resources': case 'list_meetings': return '[]';
    case 'get_token_budget': return JSON.stringify({ total: 0, limit: 100000, resources: [] });
    case 'get_rag_status': return JSON.stringify({ total_chunks: 0, indexed_files: 0, is_ready: false });
    case 'get_platform_capabilities': return { os: 'macos', mic_capture: true, system_audio: true, credential_store: true };
    case 'get_assist_session': return { sessionId: 'meeting-one', questions: [], answers: [], detectorStatus: 'semantic' };
    case 'project_knowledge_status': return JSON.stringify({ ...coverage, ...(semanticFixture ? { semantic: { provider: 'gemini', model: 'gemini-embedding-2', embedded, total: 740, ready: false } } : {}) });
    case 'project_embedding_config': return { provider: semanticFixture ? 'gemini' : 'lexical', model: 'gemini-embedding-2', dimensions: 1536, baseUrl: '' };
    case 'embed_project': {
      embedded += 16;
      await emit('project-embedding-progress', { projectId: project.id, completed: 16, total: 708 });
      throw 'Embedding provider returned HTTP 429. 16 passages saved in this run. Resume indexing to continue from the last saved batch.';
    }
    case 'project_memory': return { questions, decisions: [] };
    case 'project_documents': case 'get_meeting_translations': return [];
    case 'project_preparation': return args.regenerate ? { text: 'Review the pipeline before the meeting. [S1]', evidence: [evidence], revision: coverage.revision, createdAt: coverage.scannedAt } : null;
    case 'search_project_knowledge': return JSON.stringify([{ evidence, text: 'name: CI\non: [push, pull_request]\njobs:\n  check:\n    steps: npm run build', score: 1 }]);
    case 'read_project_evidence': if (holdEvidence) return new Promise<string>((resolve, reject) => pendingEvidence.push({ resolve, reject })); return 'name: CI\non: [push, pull_request]\njobs:\n  check:\n    steps: npm run build';
    case 'save_open_question': questions.push({ id: 'q1', text: args.question, status: 'open', evidence: [], updatedAt: coverage.scannedAt }); return;
    case 'scan_project': if (fail) throw 'Index update failed; previous snapshot is available.'; return;
    case 'generate_assist': {
      assistRequests.push(args);
      const identity = { requestId: args.requestId, sessionId: args.sessionId };
      await emit('llm_stream_start', { ...identity, mode: args.mode, model: 'claude-sonnet-5', provider: 'Anthropic', question: args.customQuestion, evidence: [evidence] });
      if (fail) { await emit('llm_stream_error', { ...identity, message: 'Rate limit reached. Retry shortly.' }); throw 'Rate limit reached. Retry shortly.'; }
      await emit('llm_stream_token', { ...identity, token: args.mode === 'FollowUp' ? '1. Which checks must pass before delivery?\n2. Who reviews a failed check?' : 'The pipeline validates the change before delivery. [S1]' });
      await emit('llm_stream_end', { ...identity, latency_ms: 800, total_tokens: 100 }); return;
    }
    default: return null;
  }
}, { shouldMockEvents: true });
const { useStreamStore } = await import('../../src/stores/streamStore');
const { useConfigStore } = await import('../../src/stores/configStore');
const { useMeetingStore } = await import('../../src/stores/meetingStore');
const { useStreamBuffer } = await import('../../src/hooks/useStreamBuffer');
const { useTheme } = await import('../../src/hooks/useTheme');
const { SettingsOverlay } = await import('../../src/settings/SettingsOverlay');
const { LauncherView } = await import('../../src/launcher/LauncherView');
const { OverlayView } = await import('../../src/overlay/OverlayView');
const { ReadinessChecks } = await import('../../src/launcher/ReadinessChecks');
useConfigStore.setState({ startOnLogin: false, llmProvider: 'anthropic', llmModel: 'claude-sonnet-5', theme: new URLSearchParams(location.search).has('calendar') ? 'light' : 'dark', appearance: new URLSearchParams(location.search).has('calendar') ? 'notebook' : 'ibm' });
await useConfigStore.getState().loadConfig();
if (new URLSearchParams(location.search).has('calendar')) useConfigStore.setState({ theme: 'light', appearance: 'notebook' });
const readiness = new URLSearchParams(location.search).has('readiness');
if (readiness) useConfigStore.setState({ sttProvider: 'windows_native', meetingAudioConfig: {
  you: { role: 'You', device_id: 'default', is_input_device: true, stt_provider: 'groq_whisper' },
  them: { role: 'Them', device_id: 'default', is_input_device: false, stt_provider: 'deepgram' }, recording_enabled: false,
} });
const settings = new URLSearchParams(location.search).has('settings');
const overlay = new URLSearchParams(location.search).has('overlay');
if (overlay) {
  useMeetingStore.setState({ activeMeeting: { id: 'meeting-one', title: 'Crosswalk delivery review', project_id: project.id, start_time: coverage.scannedAt } as any, isRecording: false });
  useStreamStore.setState({ responseHistory: [{ id: 'answer-one', content: 'The workflow checks each change before delivery. Review its configured steps before describing a deployment as complete. [S1]', question: 'How does this pipeline work?', evidence: [evidence], mode: 'Assist', model: 'claude-sonnet-5', provider: 'Anthropic', timestamp: Date.now(), latency_ms: 1500, pinned: false, searchDegraded: true, searchReason: "Semantic index is not built yet. Build it in project preparation; keyword search is active." }] });
}
(window as any).fixture = { config: useConfigStore, externalAppearance: (value: string) => emit('store://change', {resourceId: 1, key: 'appearance', value, exists: true}), fail: (value: boolean) => { fail = value; }, nextAnswer: () => useStreamStore.setState(s => ({ responseHistory: [{ ...s.responseHistory[0], id: 'answer-two', content: 'A new answer waits until you choose it.', timestamp: Date.now() + 1 }, ...s.responseHistory] })) };
(window as any).fixture.holdEvidence = () => { holdEvidence = true; };
(window as any).fixture.releaseEvidence = (index: number, error = false) => { const pending = pendingEvidence[index]; if (error) pending.reject('Old source failed'); else pending.resolve(`Source response ${index}`); };
(window as any).fixture.assistRequests = assistRequests;
(window as any).fixture.sttTests = sttTests;
(window as any).fixture.holdStt = () => { holdStt = true; };
(window as any).fixture.releaseStt = () => { holdStt = false; releaseStt?.(); };
function TestApp() { useTheme(); useStreamBuffer(); return <div className="h-screen flex flex-col bg-background text-foreground">{readiness ? <ReadinessChecks /> : settings ? <SettingsOverlay isModal={new URLSearchParams(location.search).has("modal")} /> : overlay ? <OverlayView /> : <LauncherView />}</div>; }
createRoot(document.getElementById('root')!).render(<TestApp />);
