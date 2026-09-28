import { create } from "zustand";
import type { AIResponse, IntelligenceMode, StreamSource, StreamStartEvent, EvidenceRef } from "../lib/types";
import { stripThinkTags } from "../lib/utils";

interface StreamState {
  // Current stream
  isStreaming: boolean;
  phase: "searching" | "generating" | null;
  currentContent: string;
  _rawContent: string; // unfiltered content (includes <think> tags)
  currentMode: IntelligenceMode | null;
  currentModel: string;
  currentProvider: string;
  currentSources: StreamSource[];
  currentEvidence: EvidenceRef[];
  searchDegraded: boolean;
  searchReason?: string;
  currentQuestion?: string;
  requestId?: string;
  sessionId?: string;
  error: string | null;
  latencyMs: number | null;

  // Session history and pinned responses
  responseHistory: AIResponse[];
  pinnedResponses: AIResponse[];
  selectedResponseId: string | null;
  selectResponse: (id: string | null) => void;

  // Actions
  setStreaming: (streaming: boolean) => void;
  appendToken: (token: string) => void;
  startStream: (mode: IntelligenceMode, model: string, provider: string, event?: StreamStartEvent) => void;
  setSources: (sources: StreamSource[]) => void;
  endStream: (latencyMs: number, totalTokens?: number, promptTokens?: number, completionTokens?: number) => void;
  setError: (error: string | null) => void;
  clearCurrent: () => void;
  pinResponse: (id: string) => void;
  unpinResponse: (id: string) => void;
}

export const useStreamStore = create<StreamState>((set, get) => ({
  isStreaming: false,
  phase: null,
  currentContent: "",
  _rawContent: "",
  currentMode: null,
  currentModel: "",
  currentProvider: "",
  currentSources: [],
  currentEvidence: [],
  searchDegraded: false,
  error: null,
  latencyMs: null,
  responseHistory: [],
  pinnedResponses: [],
  selectedResponseId: null,
  selectResponse: (selectedResponseId) => set({ selectedResponseId }),

  setStreaming: (streaming) => set({ isStreaming: streaming }),

  appendToken: (token) =>
    set((state) => {
      const raw = state._rawContent + token;
      return {
        _rawContent: raw,
        currentContent: stripThinkTags(raw),
      };
    }),

  startStream: (mode, model, provider, event) =>
    set({
      isStreaming: true,
      phase: "generating",
      currentContent: "",
      _rawContent: "",
      currentMode: mode,
      currentModel: model,
      currentProvider: provider,
      currentSources: [],
      currentEvidence: event?.evidence ?? [],
      searchDegraded: event?.search_degraded ?? false,
      searchReason: event?.search_reason,
      currentQuestion: event?.question,
      requestId: event?.requestId,
      sessionId: event?.sessionId,
      error: null,
      latencyMs: null,
    }),

  setSources: (sources) => set({ currentSources: sources }),

  endStream: (latencyMs, totalTokens, promptTokens, completionTokens) => {
    const state = get();
    // Strip <think> tags from the final stored content
    const content = stripThinkTags(state._rawContent);
    if (!content.trim() || !state.isStreaming) { set({ isStreaming: false, phase: null }); return; }
    const response: AIResponse = {
      id: state.requestId ?? crypto.randomUUID(),
      question: state.currentQuestion,
      evidence: state.currentEvidence,
      searchDegraded: state.searchDegraded,
      searchReason: state.searchReason,
      sessionId: state.sessionId,
      totalTokens,
      promptTokens,
      completionTokens,
      content,
      mode: state.currentMode!,
      timestamp: Date.now(),
      pinned: false,
      model: state.currentModel,
      provider: state.currentProvider,
      latency_ms: latencyMs,
      sources: state.currentSources.length > 0 ? state.currentSources : undefined,
    };

    set((s) => ({
      isStreaming: false,
      phase: null,
      currentContent: content,
      latencyMs,
      // Rust can publish its durable session snapshot before the terminal stream event.
      responseHistory: [response, ...s.responseHistory.filter(item => item.id !== response.id)].slice(0, 100),
    }));
  },

  setError: (error) => set({ error, isStreaming: false, phase: null }),
  clearCurrent: () =>
    set({ selectedResponseId: null, isStreaming: false, phase: null, currentEvidence: [], currentQuestion: undefined, requestId: undefined, sessionId: undefined, searchDegraded: false, searchReason: undefined, currentContent: "", _rawContent: "", currentMode: null, currentModel: "", currentProvider: "", currentSources: [], error: null, latencyMs: null }),

  pinResponse: (id) => {
    const state = get();
    const response = state.responseHistory.find((r) => r.id === id);
    if (response && !state.pinnedResponses.find((r) => r.id === id)) {
      set({
        pinnedResponses: [
          ...state.pinnedResponses,
          { ...response, pinned: true },
        ],
      });
    }
  },

  unpinResponse: (id) =>
    set((state) => ({
      pinnedResponses: state.pinnedResponses.filter((r) => r.id !== id),
    })),
}));
