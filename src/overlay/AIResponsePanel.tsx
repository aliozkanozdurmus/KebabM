import { readingColor } from "../lib/readingColor";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Loader2, Pin, X } from "lucide-react";
import { useStreamStore } from "../stores/streamStore";
import { useConfigStore } from "../stores/configStore";
import { cancelGeneration, generateAssist } from "../lib/ipc";
import { showToast } from "../stores/toastStore";
import { AnswerMarkdown } from "./AnswerMarkdown";
import { splitFollowUps } from "./followUps";
import { ColorPickerButton } from "../components/ColorPickerButton";
import type { EvidenceRef } from "../lib/types";
import { costScope, estimateAnswerCost } from "../lib/usage";

const button = "rounded-md border border-border px-2.5 py-1.5 text-xs hover:bg-accent focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-50";

export function AIResponsePanel() {
  const stream = useStreamStore();
  const config = useConfigStore();
  // Stable response IDs keep the answer being read in place when another completes.
  const [selected, setSelected] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [source, setSource] = useState<{ ref: EvidenceRef; text: string } | null>(null);
  const [sourceError, setSourceError] = useState("");
  const [loadingSource, setLoadingSource] = useState(false);
  const latest = stream.responseHistory[0];
  useEffect(() => { if (!selected && latest && !stream.isStreaming) setSelected(latest.id); }, [latest, selected, stream.isStreaming]);
  const response = [...stream.responseHistory, ...stream.pinnedResponses].find(r => r.id === selected);
  const content = response?.content ?? stream.currentContent;
  const evidence = response?.evidence ?? stream.currentEvidence;
  const question = response?.question ?? stream.currentQuestion;
  const follow = splitFollowUps(content);
  const [brief, ...detail] = follow.body.split(/\n(?=## (?:Detail|Details|Detay|Ayrıntı))/i);
  const isReadingLive = !response;
  const estimatedCost = response && estimateAnswerCost(response.provider, response.model, response.promptTokens, response.completionTokens);
  const act = (mode: string, text: string) => {
    generateAssist(mode, text, ["Shorten", "FollowUp"].includes(mode) ? evidence : undefined).catch(e => showToast(String(e), "error"));
  };
  const openSource = async (ref: EvidenceRef) => {
    setLoadingSource(true); setSourceError(""); setSource(null);
    try { setSource({ ref, text: await invoke<string>("read_project_evidence", { id: ref.id }) }); }
    catch (e) { setSourceError(String(e)); }
    finally { setLoadingSource(false); }
  };
  return <section className="flex min-h-0 flex-1 flex-col gap-3" aria-label="Meeting answer">
    <div className="flex flex-wrap items-center gap-2 border-b border-border pb-2">
      <span className="mr-auto text-xs font-medium">Your answer</span>
      {stream.isStreaming && <span className="flex items-center gap-1.5 text-xs text-muted-foreground" role="status"><Loader2 className="h-3 w-3 animate-spin" />{stream.phase === "searching" ? "Finding sources" : "Preparing"}</span>}
      {stream.isStreaming && <button className={button} onClick={() => cancelGeneration().catch(e => showToast(String(e), "error"))}>Cancel</button>}
      {latest && latest.id !== selected && <button className={`${button} text-primary`} onClick={() => { setSelected(latest.id); setSource(null); }}>New answer</button>}
      {stream.isStreaming && response && <button className={button} onClick={() => setSelected(null)}>View live</button>}
    </div>
    {stream.error && <div role="alert" className="flex flex-wrap items-center gap-2 rounded-md border border-destructive/40 p-2 text-xs">
      <span className="min-w-0 flex-1 text-destructive">{stream.error}</span>
      <button className={button} disabled={stream.isStreaming} onClick={() => generateAssist(stream.currentMode ?? "Assist", stream.currentQuestion ?? "", stream.currentEvidence).catch(e => showToast(String(e), "error"))}>Retry</button>
    </div>}
    <div className="min-h-0 flex-1 overflow-y-auto space-y-4 pr-1">
      {(response?.searchDegraded ?? stream.searchDegraded) && <p className="text-xs text-muted-foreground">Keyword search · semantic search unavailable or not configured</p>}
      {question && <p className="text-xs leading-relaxed text-muted-foreground">{question}</p>}
      {content ? <>
        <div className="answer-content prose prose-sm max-w-none break-words" style={{ fontSize: `${config.aiResponseFontSize}px`, color: readingColor(config.aiResponseTextColor, "answer"), lineHeight: config.aiResponseLineHeight, paddingInline: config.aiResponseHPad, textAlign: config.aiResponseAlign }}>
          <AnswerMarkdown content={brief} />
        </div>
        {detail.length > 0 && <details><summary className="cursor-pointer text-xs font-medium">Details</summary><div className="prose prose-sm prose-invert mt-3"><AnswerMarkdown content={detail.join("\n")} /></div></details>}
        <div className="flex flex-wrap gap-2">
          <button className={button} disabled={stream.isStreaming} onClick={() => act("Shorten", `Shorten this answer, preserving its source markers:\n${content}`)}>Shorter</button>
          <button className={button} disabled={stream.isStreaming} onClick={() => act("FollowUp", `Suggest a useful follow-up to this question and answer:\n${question ?? ""}\n${content}`)}>Follow-up</button>
          <button className={`${button} flex items-center gap-1`} onClick={async () => { try { await navigator.clipboard.writeText(content); setCopied(true); setTimeout(() => setCopied(false), 1800); } catch { showToast("Clipboard unavailable", "error"); } }}>{copied ? <Check size={12} /> : <Copy size={12} />}Copy</button>
          {response && <button className={`${button} flex items-center gap-1`} aria-pressed={stream.pinnedResponses.some(r => r.id === response.id)} onClick={() => stream.pinnedResponses.some(r => r.id === response.id) ? stream.unpinResponse(response.id) : stream.pinResponse(response.id)}><Pin size={12} />Keep</button>}
        </div>
        <details className="border-y border-border py-2" open={source !== null}>
          <summary className="cursor-pointer text-xs font-medium">Sources · {evidence.length}</summary>
          {!evidence.length && <p className="mt-2 text-xs text-muted-foreground">No indexed project evidence was returned. This answer may rely on conversation or added documents.</p>}
          <ol className="mt-2 space-y-1.5">
            {evidence.map((ref, i) => <li key={ref.id}><button className="w-full rounded-md p-2 text-left text-xs hover:bg-accent focus-visible:outline-2 focus-visible:outline-primary" onClick={() => openSource(ref)}><span className="font-mono break-all">[S{i + 1}] {ref.path}:{ref.startLine}–{ref.endLine}</span><span className="mt-1 block text-muted-foreground">{ref.revision.slice(0, 10)}{ref.dirty ? " · local changes" : ""}</span></button></li>)}
          </ol>
          {loadingSource && <p role="status" className="text-xs">Opening source…</p>}
          {sourceError && <p role="alert" className="text-xs text-destructive">{sourceError}</p>}
          {source && <div className="mt-2 rounded-md border border-border p-3"><div className="flex items-start gap-2"><p className="min-w-0 flex-1 break-all text-xs font-mono">{source.ref.path}</p><button aria-label="Close source" onClick={() => setSource(null)}><X size={14} /></button></div><p className="my-2 text-xs text-muted-foreground">Indexed snapshot · line {source.ref.startLine}</p><pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words text-xs leading-relaxed">{source.text}</pre></div>}
        </details>
        {follow.questions.length > 0 && <details><summary className="text-xs cursor-pointer">Suggested follow-ups</summary><div className="mt-2 space-y-2">{follow.questions.map(q => <button key={q} className={`${button} block text-left`} disabled={stream.isStreaming} onClick={() => act("AskQuestion", q)}>{q}</button>)}</div></details>}
        {response && <p className="text-[11px] text-muted-foreground" title={costScope}>{response.model} · {(response.latency_ms / 1000).toFixed(1)} s{response.totalTokens ? ` · ${response.totalTokens} tokens` : ""}{estimatedCost != null ? ` · ~$${estimatedCost.toFixed(4)} text cost` : " · cost unavailable"}</p>}
      </> : <div className="py-8"><p className="text-base font-medium">Ready for your next question</p><p className="mt-2 max-w-md text-sm leading-relaxed text-muted-foreground">Ask about the selected project, or start a meeting to receive help as questions arrive. Answers and their sources stay here for you to read.</p>{isReadingLive && stream.isStreaming && <p className="mt-4 text-sm text-primary" role="status">Finding context and preparing an answer…</p>}</div>}
    </div>
    {stream.responseHistory.length > 0 && <details className="border-t border-border pt-2"><summary className="text-xs cursor-pointer">Answer history · {stream.responseHistory.length}</summary><div className="max-h-32 overflow-y-auto mt-2 space-y-1">{stream.responseHistory.map(r => <button key={r.id} className={`${button} block w-full text-left truncate ${selected === r.id ? "text-primary" : ""}`} onClick={() => { setSelected(r.id); setSource(null); }}>{r.question || r.content.slice(0, 70)}</button>)}</div></details>}
    <details className="text-xs text-muted-foreground"><summary className="cursor-pointer">Reading preferences</summary><div className="flex flex-wrap gap-3 py-2 items-center">
      <label>Size <input className="w-12 bg-transparent" type="number" min={10} max={32} value={config.aiResponseFontSize} onChange={e => config.setAiResponseFontSize(Math.max(10, Math.min(32, Number(e.target.value))))} /></label>
      <label>Spacing <input className="w-12 bg-transparent" type="number" min={1} max={3} step={0.1} value={config.aiResponseLineHeight} onChange={e => config.setAiResponseLineHeight(Math.max(1, Math.min(3, Number(e.target.value))))} /></label>
      <ColorPickerButton value={config.aiResponseTextColor} onChange={config.setAiResponseTextColor} label="Answer text color" />
      <label>Margin <input className="w-12 bg-transparent" type="number" min={0} max={80} value={config.aiResponseHPad} onChange={e => config.setAiResponseHPad(Math.max(0, Math.min(80, Number(e.target.value))))} /></label>
      <label>Align <select className="bg-background" value={config.aiResponseAlign} onChange={e => config.setAiResponseAlign(e.target.value as "left" | "center" | "right")}><option>left</option><option>center</option><option>right</option></select></label>
    </div></details>
  </section>;
}
