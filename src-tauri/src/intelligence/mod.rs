pub mod action_config;
pub mod context_builder;
pub mod prompt_templates;
pub mod question_detector;
pub mod session;
pub mod transcript_buffer;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::llm::request::{RequestIdentity, ResponseSink};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use action_config::AllActionConfigs;
use context_builder::ContextBuilder;
use question_detector::{DetectedQuestion, QuestionDetector};
use transcript_buffer::TranscriptBuffer;

use crate::llm::provider::GenerationParams;

/// Event payload for question detection events emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionDetectedPayload {
    pub question: DetectedQuestion,
}

/// Orchestrates: read transcript -> detect question -> load context -> build prompt -> LLM stream
pub struct IntelligenceEngine {
    pub transcript_buffer: TranscriptBuffer,
    pub session: session::Session,
    question_detector: QuestionDetector,
    auto_trigger: AtomicBool,
    is_generating: Arc<AtomicBool>,
    cancel_requested: CancellationToken,
    last_detected_question: Option<DetectedQuestion>,
    seen_segments: std::collections::VecDeque<(u64, String, String)>,
    action_configs: AllActionConfigs,
}

impl IntelligenceEngine {
    pub fn new() -> Self {
        Self {
            transcript_buffer: TranscriptBuffer::new(),
            session: session::Session::default(),
            question_detector: QuestionDetector::new(),
            auto_trigger: AtomicBool::new(true),
            is_generating: Arc::new(AtomicBool::new(false)),
            cancel_requested: CancellationToken::new(),
            last_detected_question: None,
            seen_segments: std::collections::VecDeque::new(),
            action_configs: AllActionConfigs::default(),
        }
    }

    /// Push a transcript segment into the buffer and run question detection.
    /// Returns detected questions (if any) so the caller can emit events.
    pub fn push_transcript(
        &mut self,
        text: String,
        speaker: String,
        timestamp_ms: u64,
        is_final: bool,
    ) -> Vec<DetectedQuestion> {
        self.push_transcript_persisted(text, speaker, timestamp_ms, is_final, || Ok(()))
            .unwrap_or_default()
    }

    pub fn push_transcript_persisted<F: FnOnce() -> Result<(), String>>(
        &mut self,
        text: String,
        speaker: String,
        timestamp_ms: u64,
        is_final: bool,
        persist: F,
    ) -> Result<Vec<DetectedQuestion>, String> {
        let party = match speaker.to_lowercase().as_str() {
            "user" | "you" | "me" => "me",
            "room" => "room",
            _ => "them",
        };
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if is_final {
            if self.seen_segments.iter().any(|(ts, src, body)| {
                src == party && body == &normalized && timestamp_ms.abs_diff(*ts) < 15_000
            }) {
                return Ok(Vec::new());
            }
            persist()?;
            self.session.sequence += 1;
            if party != "me" {
                self.session.question_sequence += 1;
            }
            self.seen_segments
                .push_back((timestamp_ms, party.into(), normalized.clone()));
            while self.seen_segments.len() > 500 {
                self.seen_segments.pop_front();
            }
        }
        self.transcript_buffer.push_segment(
            normalized.clone(),
            party.into(),
            timestamp_ms,
            is_final,
        );

        // Only detect questions on final segments
        if !is_final || party == "me" {
            return Ok(Vec::new());
        }

        let questions = self
            .question_detector
            .detect_questions(&normalized, timestamp_ms, party);

        // Store the most recent high-confidence question
        if let Some(q) = questions.iter().max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            self.last_detected_question = Some(q.clone());
        }

        Ok(questions)
    }

    /// Main entry point: generate AI assistance in the given mode.
    /// Orchestrates the full pipeline: transcript -> question -> context -> prompt -> LLM stream.
    ///
    /// This is called from the Tauri command layer. The LLMRouter and ContextManager
    /// are passed in because they live in AppState under separate locks.
    pub async fn generate_assist(
        system_prompt: &str,
        mode: &str,
        custom_question: Option<&str>,
        transcript_text: String,
        last_question: Option<DetectedQuestion>,
        context_text: String,
        include_context: bool,
        include_transcript: bool,
        include_question: bool,
        include_rag: bool,
        include_instructions: bool,
        llm_provider: Arc<dyn crate::llm::provider::LLMProvider>,
        model: String,
        provider_name: String,
        params: GenerationParams,
        // New metadata fields for StreamStartEvent
        temperature: f64,
        rag_query: Option<String>,
        rag_chunks: Vec<crate::llm::provider::RagChunkInfo>,
        rag_chunks_filtered: usize,
        rag_total_candidates: usize,
        transcript_window_seconds: u64,
        transcript_segments_count: usize,
        transcript_segments_total: usize,
        app_handle: tauri::AppHandle,
        cancel_flag: CancellationToken,
        identity: RequestIdentity,
        evidence: Vec<crate::projects::knowledge::EvidenceRef>,
        search_degraded: bool,
        search_reason: Option<String>,
    ) -> Result<(String, crate::llm::provider::CompletionStats), String> {
        let sink = ResponseSink::new(Some(app_handle), identity, cancel_flag.clone());

        // Build the prompt using configurable flags.
        // The system prompt IS the only instruction (per-action editable + composed instructions).
        // The user message contains only togglable data sections (transcript, RAG, questions).
        let builder = ContextBuilder::new();
        let messages = builder.build_prompt_with_config(
            system_prompt,
            &transcript_text,
            last_question.as_ref(),
            &context_text,
            custom_question,
            include_context,
            include_transcript,
            include_question,
        );

        // Check for cancellation before starting
        if cancel_flag.is_cancelled() {
            return Err("Answer cancelled.".into());
        }

        sink.phase("generating", None);
        // Emit stream start with source metadata; prompt contents remain private
        let _ = sink.emit(
            "llm_stream_start",
            crate::llm::provider::StreamStartPayload {
                mode: mode.to_string(),
                model: model.clone(),
                provider: provider_name.clone(),
                evidence,
                search_degraded,
                search_reason,
                question: custom_question
                    .map(str::to_owned)
                    .or_else(|| last_question.as_ref().map(|q| q.text.clone())),
                system_prompt: String::new(),
                user_prompt: String::new(),
                include_transcript,
                include_rag,
                include_instructions,
                include_question,
                temperature,
                rag_query,
                rag_chunks: rag_chunks
                    .into_iter()
                    .map(|mut c| {
                        c.text.clear();
                        c
                    })
                    .collect(),
                rag_chunks_filtered,
                rag_total_candidates,
                transcript_window_seconds,
                transcript_segments_count,
                transcript_segments_total,
            },
        );

        crate::llm::request::complete_unfinished(
            llm_provider.as_ref(),
            messages,
            &model,
            params,
            sink.clone(),
        )
        .await
        .map(|stats| (sink.text(), stats))
        .map_err(|e| crate::llm::request::user_error(&e))
    }

    /// Reserve the live generation slot until every exit path has unwound.
    pub fn begin_generation(&mut self) -> Result<GenerationLease, String> {
        if self.is_generating.swap(true, Ordering::SeqCst) {
            return Err("Generation already in progress".into());
        }
        self.cancel_requested = CancellationToken::new();
        Ok(GenerationLease {
            generating: self.is_generating.clone(),
        })
    }

    /// Cancel the current generation.
    pub fn cancel(&self) {
        self.cancel_requested.cancel();
    }

    /// Get the cancel flag for passing to async tasks.
    pub fn cancel_flag(&self) -> CancellationToken {
        self.cancel_requested.clone()
    }

    /// Check if currently generating.
    pub fn is_generating(&self) -> bool {
        self.is_generating.load(Ordering::SeqCst)
    }

    /// Set the generating state.
    pub fn set_generating(&self, generating: bool) {
        self.is_generating.store(generating, Ordering::SeqCst);
    }

    /// Toggle auto-detection of questions.
    pub fn set_auto_trigger(&self, enabled: bool) {
        self.auto_trigger.store(enabled, Ordering::SeqCst);
    }

    /// Check if auto-trigger is enabled.
    pub fn auto_trigger_enabled(&self) -> bool {
        self.auto_trigger.load(Ordering::SeqCst)
    }

    /// Set the transcript buffer context window size.
    pub fn set_context_window(&mut self, seconds: u64) {
        self.transcript_buffer.set_window_seconds(seconds);
    }

    /// Get the most recently detected question.
    pub fn last_detected_question(&self) -> Option<&DetectedQuestion> {
        self.last_detected_question.as_ref()
    }

    /// Get recent transcript text within the configured window.
    pub fn get_recent_transcript(&self) -> String {
        let window = self.transcript_buffer.window_seconds();
        self.transcript_buffer.get_recent_text(window)
    }

    /// Get all transcript text (for Recap mode with window=0).
    pub fn get_all_transcript(&self) -> String {
        self.transcript_buffer.get_all_text()
    }

    /// Set action configs (called from IPC when frontend syncs).
    pub fn set_action_configs(&mut self, configs: AllActionConfigs) {
        self.action_configs = configs;
    }

    /// Get action configs.
    pub fn get_action_configs(&self) -> &AllActionConfigs {
        &self.action_configs
    }

    /// Get a specific action config by mode key.
    pub fn get_action_config(&self, mode: &str) -> Option<&action_config::ActionConfig> {
        self.action_configs.actions.get(mode)
    }

    /// Reset per-meeting state so the next meeting starts clean.
    /// Clears the transcript buffer and last detected question.
    pub fn clear_session(&mut self) {
        self.session = session::Session::default();
        self.transcript_buffer.clear();
        self.last_detected_question = None;
        self.seen_segments.clear();
        self.cancel_requested.cancel();
        // The old lease must not release the next meeting's generation slot.
        self.cancel_requested = CancellationToken::new();
        self.is_generating = Arc::new(AtomicBool::new(false));
        log::info!("[intelligence] session cleared for new meeting");
    }
}

/// Dropping a request (including a failed preparation or cancelled future) releases the slot.
pub struct GenerationLease {
    generating: Arc<AtomicBool>,
}
impl Drop for GenerationLease {
    fn drop(&mut self) {
        self.generating.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn failed_preparation_releases_the_slot_and_old_session_cannot_release_new_request() {
        let mut e = IntelligenceEngine::new();
        let old = e.begin_generation().unwrap();
        assert!(e.begin_generation().is_err());
        drop(old);
        assert!(!e.is_generating());
        let old = e.begin_generation().unwrap();
        e.clear_session();
        let new = e.begin_generation().unwrap();
        drop(old);
        assert!(e.is_generating());
        drop(new);
        assert!(!e.is_generating());
    }
    #[test]
    fn failed_transcript_write_does_not_advance_or_deduplicate_retry() {
        let mut e = IntelligenceEngine::new();
        assert!(e
            .push_transcript_persisted(
                "How does it work?".into(),
                "them".into(),
                1000,
                true,
                || Err("disk full".into())
            )
            .is_err());
        assert_eq!(e.session.sequence, 0);
        assert!(e.transcript_buffer.get_recent_text(60).is_empty());
        e.push_transcript_persisted(
            "How does it work?".into(),
            "them".into(),
            1000,
            true,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(e.session.sequence, 1);
        e.push_transcript_persisted(
            "How does it work?".into(),
            "them".into(),
            1001,
            true,
            || panic!("duplicate must not write"),
        )
        .unwrap();
        assert_eq!(e.session.sequence, 1);
    }
    #[test]
    fn both_party_transcripts_deduplicate_and_me_does_not_cancel_question_detection() {
        let mut e = IntelligenceEngine::new();
        e.push_transcript("How does".into(), "Them".into(), 1000, true);
        e.push_transcript("the pipeline work?".into(), "them".into(), 2000, true);
        assert_eq!(
            e.transcript_buffer.recent_party_text("them", 6000),
            "How does the pipeline work?"
        );
        e.push_transcript(
            "the pipeline work?".into(),
            "Interviewer".into(),
            2001,
            true,
        );
        assert_eq!(e.session.sequence, 2);
        e.push_transcript("Let me check".into(), "User".into(), 3000, true);
        assert_eq!(e.session.sequence, 3);
        assert_eq!(e.session.question_sequence, 2);
    }
}
