import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { pushTranscript } from "../lib/ipc";
import { useTranscriptStore } from "../stores/transcriptStore";
import { useStreamStore } from "../stores/streamStore";
import { useMeetingStore } from "../stores/meetingStore";
import { useConfigStore } from "../stores/configStore";
import { showToast } from "../stores/toastStore";
import type { AIResponse } from "../lib/types";
interface SessionQuestion { id: string; text: string; state: string; error?: string; requestId?: string }
interface AssistSession { sessionId?: string; questions: SessionQuestion[]; answers: AIResponse[]; detectorStatus: string }
export function QuestionDetector() {
  const [snapshot, setSnapshot] = useState<AssistSession>({ questions: [], answers: [], detectorStatus: "" });
  const [requesting, setRequesting] = useState<string>();
  const seen = useRef(new Set<string>());
  const segments = useTranscriptStore(s => s.segments);
  const isRecording = useMeetingStore(s => s.isRecording);
  const session = useMeetingStore(s => s.activeMeeting?.id);
  const isStreaming = useStreamStore(s => s.isStreaming);
  const auto = useConfigStore(s => s.autoTrigger);
  const setAuto = useConfigStore(s => s.setAutoTrigger);
  useEffect(() => { seen.current.clear(); }, [session]);
  useEffect(() => {
    let active = true;
    const update = (s: AssistSession) => { if (!active || s.sessionId !== useMeetingStore.getState().activeMeeting?.id) return; setSnapshot(s);
      if (s.answers?.length) useStreamStore.setState(old => ({ responseHistory: [...s.answers, ...old.responseHistory.filter(a => !s.answers.some(b => a.id === b.id))].sort((a, b) => b.timestamp - a.timestamp).slice(0, 100) }));
    };
    const pending = listen<AssistSession>("assist_session", e => update(e.payload));
    invoke<AssistSession>("get_assist_session").then(update).catch(() => {});
    return () => { active = false; pending.then(stop => stop()); };
  }, [session]);
  useEffect(() => {
    if (!isRecording || !session) return;
    for (const segment of segments) {
      if (segment.sessionId || !segment.is_final || seen.current.has(segment.id)) continue;
      seen.current.add(segment.id);
      pushTranscript(segment.text, segment.source ?? (["User", "You", "me"].includes(segment.speaker) ? "me" : "them"), segment.timestamp_ms, true, session, segment.id)
        .catch(() => seen.current.delete(segment.id));
    }
  }, [segments, isRecording, session]);
  const pending = snapshot.questions.filter(q => q.state !== "answered");
  const answer = async (id: string) => { setRequesting(id); try { await invoke("answer_session_question", { id }); } catch (e) { showToast(String(e), "error"); } finally { setRequesting(undefined); } };
  return <section className="space-y-2" aria-label="Question queue">
    <div className="flex flex-wrap items-center justify-between gap-2 text-xs"><span className="font-medium">{pending.length ? `${pending.length} question${pending.length === 1 ? "" : "s"} waiting` : isRecording ? "Listening for questions" : "Manual help available"}</span><label className="flex items-center gap-2 cursor-pointer"><input type="checkbox" checked={auto} onChange={e => setAuto(e.target.checked)} />Automatic help</label></div>
    {snapshot.detectorStatus === "rules_fallback" && <p className="text-xs text-muted-foreground">Question detection is using speech patterns; semantic detection was unavailable.</p>}
    {!pending.length && <p className="text-xs text-muted-foreground">Ask below at any time. New answers wait while you read.</p>}
    {pending.slice(0, 3).map(q => <div key={q.id} className="flex items-start gap-2 border-l-2 border-primary/40 pl-2"><div className="min-w-0 flex-1"><p className="text-sm leading-relaxed">{q.text}</p>{q.error && <p className="mt-1 text-xs text-destructive" role="alert">{q.error}</p>}</div><button disabled={isStreaming || !!requesting || q.state === "preparing"} className="border border-border px-2 py-1 text-xs disabled:opacity-50" onClick={() => answer(q.id)}>{q.state === "error" ? "Retry" : q.state === "preparing" ? "Preparing…" : "Answer"}</button></div>)}
    {pending.length > 3 && <details><summary className="text-xs cursor-pointer">{pending.length - 3} more queued</summary>{pending.slice(3).map(q => <p key={q.id} className="mt-2 text-xs">{q.text}</p>)}</details>}
  </section>;
}
