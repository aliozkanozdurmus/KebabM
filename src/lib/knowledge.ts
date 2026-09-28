import { invoke } from "@tauri-apps/api/core";
import type { EvidenceRef } from "./types";
export interface Coverage { revision: string; scannedAt: string; indexedFiles: number; reusedFiles: number; chunks: number; gitAware: boolean; excluded: { path: string; reason: string }[]; searchMode: string; freshness?: "current" | "stale" | "unavailable"; freshnessCheckedAt?: string }
export interface EmbeddingConfig { provider: "lexical" | "gemini" | "ollama"; model: string; dimensions: number; baseUrl: string }
export interface KnowledgeHit { evidence: EvidenceRef; text: string; score: number }
export interface MemoryItem { id: string; text: string; status: string; evidence: EvidenceRef[]; updatedAt: string }
export interface ProjectMemory { questions: MemoryItem[]; decisions: MemoryItem[] }
export type PreparationKind = "return" | "rehearsal" | "handbook";
export interface Preparation { text: string; evidence: EvidenceRef[]; revision: string; createdAt: string; searchDegraded?: boolean }
export const knowledge = {
  coverage: async (id: string) => JSON.parse(await invoke<string>("project_knowledge_status", { id })) as Coverage,
  config: (id: string, config?: EmbeddingConfig) => invoke<EmbeddingConfig>("project_embedding_config", { id, config }),
  embed: (id: string) => invoke<number>("embed_project", { id }),
  search: async (id: string, question: string) => JSON.parse(await invoke<string>("search_project_knowledge", { id, question })) as KnowledgeHit[],
  source: (id: string) => invoke<string>("read_project_evidence", { id }),
  memory: (id: string) => invoke<ProjectMemory>("project_memory", { id }),
  draftDecisions: (id: string) => invoke<{ drafts: number; passagesReviewed: number; totalPassages: number }>("draft_meeting_decisions", { id }),
  saveQuestion: (id: string, question: string, meetingId?: string) => invoke("save_open_question", { id, question, meetingId }),
  updateMemory: (id: string, action: { kind: "decision" | "question" | "recheck"; itemId?: string; text?: string; evidence?: EvidenceRef[]; status?: string }) => invoke("update_project_memory", { id, ...action }),
  preparation: (id: string, kind: PreparationKind, regenerate = false) => invoke<Preparation | null>("project_preparation", { id, kind, regenerate }),
  documents: (id: string) => invoke<string[]>("project_documents", { id }),
  linkDocument: (id: string, resourceId: string, linked: boolean) => invoke("link_project_document", { id, resourceId, linked }),
};
