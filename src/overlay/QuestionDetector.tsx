import { useEffect, useState, useRef, useCallback } from "react";
import { HelpCircle, Sparkles, Check, Clock, X } from "lucide-react";
import { onQuestionDetected } from "../lib/events";
import { generateAssist } from "../lib/ipc";
import { useTranscriptStore } from "../stores/transcriptStore";
import { useStreamStore } from "../stores/streamStore";
import { useMeetingStore } from "../stores/meetingStore";
import type { DetectedQuestion, TranscriptSegment } from "../lib/types";

function looksLikeQuestion(text: string): boolean {
  const trimmed = text.trim();
  if (trimmed.length < 8) return false;
  if (trimmed.includes("?")) return true;
  const lower = trimmed.toLowerCase();
  const cues = [
    "what ", "how ", "why ", "when ", "where ", "who ", "which ",
    "can you", "could you", "would you", "do you", "are you",
    "is there", "have you", "tell me", "explain", "walk me",
    "ne ", "nasıl", "neden", "niçin", "kim ", "hangi", "nerede", "ne zaman",
    "comment ", "pourquoi", "est-ce", "quel ", "quelle ", "pouvez-vous",
  ];
  if (cues.some((cue) => lower.includes(cue))) return true;
  return /\b(mi|mı|mu|mü)\b/.test(lower);
}

function fromOtherParty(segment: TranscriptSegment): boolean {
  const speaker = (segment.speaker || "").toLowerCase();
  const id = (segment.speaker_id || "").toLowerCase();
  if (speaker === "you" || speaker === "user" || id === "you") return false;
  return speaker === "them" || speaker === "interviewer" || id === "them";
}

interface TrackedQuestion extends DetectedQuestion {
  assisted: boolean;
}

export function QuestionDetector() {
  const [questions, setQuestions] = useState<TrackedQuestion[]>([]);
  const processedIdsRef = useRef<Set<string>>(new Set());
  const answeredRef = useRef<Set<string>>(new Set());
  const pendingQuestionRef = useRef<string | null>(null);
  const answerTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastAnswerAtRef = useRef(0);
  const lastFlowCharsRef = useRef(0);
  const segments = useTranscriptStore((s) => s.segments);
  const isRecording = useMeetingStore((s) => s.isRecording);
  const isStreaming = useStreamStore((s) => s.isStreaming);

  const addQuestion = useCallback((q: DetectedQuestion) => {
    setQuestions((prev) => {
      if (prev.length > 0 && prev[0].text === q.text) return prev;
      return [{ ...q, assisted: false }, ...prev].slice(0, 10);
    });
  }, []);

  useEffect(() => {
    const p = onQuestionDetected((event) => {
      if (event.source === "Them" || event.source === "Interviewer") {
        addQuestion(event);
      }
    });
    return () => { p.then((u) => u()); };
  }, [addQuestion]);

  const answerQuestion = useCallback((text: string) => {
    const key = text.trim().toLowerCase().slice(0, 180);
    if (!key || answeredRef.current.has(key)) return;
    if (useStreamStore.getState().isStreaming) {
      pendingQuestionRef.current = text;
      return;
    }
    answeredRef.current.add(key);
    lastAnswerAtRef.current = Date.now();
    addQuestion({ text, confidence: 0.9, timestamp_ms: Date.now(), source: "Them" });
    setQuestions((prev) => prev.map((q) => q.text === text ? { ...q, assisted: true } : q));
    generateAssist("Assist", text).catch(() => {});
  }, [addQuestion]);

  useEffect(() => {
    if (!isRecording) return;
    for (const seg of segments) {
      if (!seg.is_final || processedIdsRef.current.has(seg.id)) continue;
      processedIdsRef.current.add(seg.id);
      if (!fromOtherParty(seg) || !looksLikeQuestion(seg.text)) continue;
      addQuestion({ text: seg.text, confidence: 0.8, timestamp_ms: seg.timestamp_ms, source: seg.speaker || "Them" });
      pendingQuestionRef.current = seg.text;
      if (answerTimerRef.current) clearTimeout(answerTimerRef.current);
      answerTimerRef.current = setTimeout(() => {
        const next = pendingQuestionRef.current;
        pendingQuestionRef.current = null;
        if (next) answerQuestion(next);
      }, 900);
    }
  }, [segments, isRecording, addQuestion, answerQuestion]);

  useEffect(() => {
    if (isStreaming || !pendingQuestionRef.current) return;
    const next = pendingQuestionRef.current;
    pendingQuestionRef.current = null;
    answerQuestion(next);
  }, [isStreaming, answerQuestion]);

  useEffect(() => {
    if (!isRecording) return;
    const timer = setInterval(() => {
      if (useStreamStore.getState().isStreaming) return;
      if (Date.now() - lastAnswerAtRef.current < 18000) return;
      const spoken = useTranscriptStore.getState().segments
        .filter((seg) => seg.is_final)
        .map((seg) => seg.text)
        .join(" ");
      if (spoken.length - lastFlowCharsRef.current < 180) return;
      lastFlowCharsRef.current = spoken.length;
      lastAnswerAtRef.current = Date.now();
      generateAssist(
        "FollowUp",
        "From the latest part of the meeting, give follow-up questions and example questions I can ask next."
      ).catch(() => {});
    }, 40000);
    return () => clearInterval(timer);
  }, [isRecording]);

  const handleAssist = useCallback((index: number) => {
    const questionText = questions[index]?.text;
    setQuestions((prev) =>
      prev.map((q, i) => i === index ? { ...q, assisted: true } : q)
    );
    generateAssist("Assist", questionText).catch(() => {});
  }, [questions]);

  const handleDismiss = useCallback((index: number, e: React.MouseEvent) => {
    e.stopPropagation();
    setQuestions((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const latest = questions.length > 0 ? questions[0] : null;
  const previousQuestions = questions.slice(1, 4);

  return (
    <div className="flex flex-col gap-2.5" role="region" aria-label="Detected questions">
      {/* Latest question — prominent card */}
      <div
        className={`group flex items-start gap-3 rounded-lg transition-all duration-200 ${
          latest ? "cursor-pointer hover:bg-info/10 question-card-enter" : ""
        }`}
        onClick={() => latest && handleAssist(0)}
        onKeyDown={(e) => { if (latest && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); handleAssist(0); } }}
        role={latest ? "button" : undefined}
        tabIndex={latest ? 0 : undefined}
        aria-label={latest ? `Question: ${latest.text}. ${latest.assisted ? "Answered" : "Click to assist"}` : undefined}
      >
        <div className="relative mt-0.5 shrink-0" aria-hidden="true">
          <HelpCircle className={`h-5 w-5 transition-colors ${latest ? "text-info" : "text-muted-foreground/50"}`} />
          {latest && !latest.assisted && (
            <span className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-info animate-pulse" />
          )}
          {latest?.assisted && (
            <span className="absolute -top-0.5 -right-0.5 flex h-3 w-3 items-center justify-center rounded-full bg-success">
              <Check className="h-2 w-2 text-white" />
            </span>
          )}
        </div>

        <div className="flex-1 min-w-0">
          {latest ? (
            <p className="text-sm leading-relaxed font-medium text-foreground/90">
              &ldquo;{latest.text}&rdquo;
            </p>
          ) : (
            <p className="text-xs text-muted-foreground/50">
              Listening. A question from the other side is answered on its own. Follow-ups appear as the meeting moves.
            </p>
          )}
        </div>

        {latest && (
          <div className="flex shrink-0 items-center gap-1">
            <button
              onClick={(e) => { e.stopPropagation(); handleAssist(0); }}
              aria-label={latest.assisted ? "Already answered" : "Get AI assistance for this question"}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-all duration-150 cursor-pointer ${
                latest.assisted
                  ? "bg-success/10 border border-success/20 text-success"
                  : "bg-info/10 border border-info/20 text-info hover:bg-info/20"
              }`}
            >
              {latest.assisted ? (
                <><Check className="h-3.5 w-3.5" aria-hidden="true" />Answered</>
              ) : (
                <><Sparkles className="h-3.5 w-3.5" aria-hidden="true" />Assist</>
              )}
            </button>
            <button
              onClick={(e) => handleDismiss(0, e)}
              aria-label="Dismiss question"
              className="rounded-lg p-1.5 text-muted-foreground/30 opacity-0 transition-all duration-150 hover:bg-destructive/10 hover:text-destructive group-hover:opacity-100 cursor-pointer"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
      </div>

      {/* Previous questions — compact list */}
      {previousQuestions.length > 0 && (
        <div className="flex flex-col gap-1">
          {previousQuestions.map((q, idx) => {
            const realIdx = idx + 1;
            return (
              <div
                key={`q-${realIdx}-${q.timestamp_ms}`}
                className={`group/q flex items-center gap-2 rounded-lg px-2.5 py-1.5 text-left transition-all duration-150 question-card-enter ${
                  q.assisted
                    ? "bg-success/5 border border-success/10"
                    : "bg-card/20 hover:bg-card/40 hover:border-border/20 cursor-pointer"
                }`}
                onClick={() => !q.assisted && handleAssist(realIdx)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); handleAssist(realIdx); } }}
                title={q.text}
              >
                <div className="shrink-0">
                  {q.assisted ? (
                    <div className="flex h-4 w-4 items-center justify-center rounded-full bg-success/20">
                      <Check className="h-2.5 w-2.5 text-success" />
                    </div>
                  ) : (
                    <div className="flex h-4 w-4 items-center justify-center rounded-full bg-muted/30">
                      <Clock className="h-2.5 w-2.5 text-muted-foreground/60" />
                    </div>
                  )}
                </div>

                <span className={`flex-1 truncate text-xs leading-snug transition-colors ${
                  q.assisted
                    ? "text-success/70 font-medium"
                    : "text-muted-foreground/60 group-hover/q:text-foreground/80"
                }`}>
                  {q.text}
                </span>

                {!q.assisted && (
                  <Sparkles className="h-3 w-3 shrink-0 text-info/0 group-hover/q:text-info/60 transition-colors" />
                )}
                <button
                  onClick={(e) => handleDismiss(realIdx, e)}
                  aria-label="Dismiss question"
                  className="rounded p-0.5 text-muted-foreground/0 opacity-0 transition-all duration-150 hover:bg-destructive/10 hover:text-destructive group-hover/q:opacity-100 group-hover/q:text-muted-foreground/30 cursor-pointer"
                >
                  <X className="h-3 w-3" />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
