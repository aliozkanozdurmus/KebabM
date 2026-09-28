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
const questions: unknown[] = [];
const saved = JSON.parse(localStorage.getItem("fixture-config") ?? "{}");
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

    case 'list_audio_devices': return JSON.stringify({ inputs: [], outputs: [] });
    case 'list_local_stt_engines': return '[]';
    case 'get_audio_sessions': return '[]';
    case 'list_projects': return JSON.stringify([project]);
    case 'list_context_resources': case 'list_meetings': return '[]';
    case 'get_token_budget': return JSON.stringify({ total: 0, limit: 100000, resources: [] });
    case 'get_rag_status': return JSON.stringify({ total_chunks: 0, indexed_files: 0, is_ready: false });
    case 'get_platform_capabilities': return { os: 'macos', mic_capture: true, system_audio: true, credential_store: true };
    case 'get_assist_session': return { sessionId: 'meeting-one', questions: [], answers: [], detectorStatus: 'semantic' };
    case 'project_knowledge_status': return JSON.stringify(coverage);
    case 'project_embedding_config': return { provider: 'lexical', model: 'gemini-embedding-2', dimensions: 1536, baseUrl: '' };
    case 'project_memory': return { questions, decisions: [] };
    case 'project_documents': case 'get_meeting_translations': return [];
    case 'project_preparation': return args.regenerate ? { text: 'Review the pipeline before the meeting. [S1]', evidence: [evidence], revision: coverage.revision, createdAt: coverage.scannedAt } : null;
    case 'search_project_knowledge': return JSON.stringify([{ evidence, text: 'name: CI\non: [push, pull_request]\njobs:\n  check:\n    steps: npm run build', score: 1 }]);
    case 'read_project_evidence': return 'name: CI\non: [push, pull_request]\njobs:\n  check:\n    steps: npm run build';
    case 'save_open_question': questions.push({ id: 'q1', text: args.question, status: 'open', evidence: [], updatedAt: coverage.scannedAt }); return;
    case 'scan_project': if (fail) throw 'Index update failed; previous snapshot is available.'; return;
    case 'generate_assist': {
      const identity = { requestId: args.requestId, sessionId: args.sessionId };
      await emit('llm_stream_start', { ...identity, mode: args.mode, model: 'claude-sonnet-5', provider: 'Anthropic', question: args.customQuestion, evidence: [evidence] });
      if (fail) { await emit('llm_stream_error', { ...identity, message: 'Rate limit reached. Retry shortly.' }); throw 'Rate limit reached. Retry shortly.'; }
      await emit('llm_stream_token', { ...identity, token: 'The pipeline validates the change before delivery. [S1]' });
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
useConfigStore.setState({ startOnLogin: false, llmProvider: 'anthropic', llmModel: 'claude-sonnet-5', theme: 'dark' });
await useConfigStore.getState().loadConfig();
const settings = new URLSearchParams(location.search).has('settings');
const overlay = new URLSearchParams(location.search).has('overlay');
if (overlay) {
  useMeetingStore.setState({ activeMeeting: { id: 'meeting-one', title: 'Crosswalk delivery review', project_id: project.id, start_time: coverage.scannedAt } as any, isRecording: false });
  useStreamStore.setState({ responseHistory: [{ id: 'answer-one', content: 'The workflow checks each change before delivery. Review its configured steps before describing a deployment as complete. [S1]', question: 'How does this pipeline work?', evidence: [evidence], mode: 'Assist', model: 'claude-sonnet-5', provider: 'Anthropic', timestamp: Date.now(), latency_ms: 1500, pinned: false, searchDegraded: true }] });
}
(window as any).fixture = { config: useConfigStore, externalAppearance: (value: string) => emit('store://change', {resourceId: 1, key: 'appearance', value, exists: true}), fail: (value: boolean) => { fail = value; }, nextAnswer: () => useStreamStore.setState(s => ({ responseHistory: [{ ...s.responseHistory[0], id: 'answer-two', content: 'A new answer waits until you choose it.', timestamp: Date.now() + 1 }, ...s.responseHistory] })) };
function TestApp() { useTheme(); useStreamBuffer(); return <div className="h-screen flex flex-col bg-background text-foreground">{settings ? <SettingsOverlay isModal={new URLSearchParams(location.search).has("modal")} /> : overlay ? <OverlayView /> : <LauncherView />}</div>; }
createRoot(document.getElementById('root')!).render(<TestApp />);
