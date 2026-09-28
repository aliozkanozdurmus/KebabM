// Sub-PRD 6: Question detection
// Layer 1: Regex (sentences ending ?, interrogative words)
// Layer 2: Interview-specific contextual patterns

use serde::{Deserialize, Serialize};

/// A detected question from the transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedQuestion {
    pub text: String,
    pub confidence: f64,
    pub timestamp_ms: u64,
    pub source: String,
}

/// Detects questions in transcript text using two layers:
/// - Layer 1: Regex patterns (question marks, interrogative words)
/// - Layer 2: Interview-specific contextual patterns
pub struct QuestionDetector;

/// Interrogative words that commonly start questions.
const INTERROGATIVE_STARTERS: &[&str] = &[
    "what", "why", "how", "when", "where", "who", "which",
    "can", "could", "would", "should",
    "do", "does", "is", "are", "will", "have", "has",
    "tell", "explain", "nasıl", "neden", "niçin", "hangi", "kim", "nerede", "ne", "comment", "pourquoi", "quel", "quelle",
];

/// Interview-specific patterns that indicate a question or prompt.
const INTERVIEW_PATTERNS: &[&str] = &[
    "walk me through",
    "describe a time",
    "tell me about",
    "what would you do",
    "how do you handle",
    "give me an example",
    "explain how",
    "what's your experience with",
    "what is your experience with",
    "can you walk me through",
    "can you describe",
    "can you explain",
    "can you tell me",
    "talk about a time",
    "share an example",
    "how would you",
    "how have you",
    "what approach would you",
    "what's your approach to",
    "what is your approach to",
];

impl QuestionDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect questions in the given text.
    /// Returns a list of detected questions with confidence scores.
    pub fn detect_questions(&self, text: &str, timestamp_ms: u64, source: &str) -> Vec<DetectedQuestion> {
        let mut questions: Vec<DetectedQuestion> = Vec::new();

        // Split text into sentences
        let sentences = split_sentences(text);

        for sentence in &sentences {
            let trimmed = sentence.trim();
            if trimmed.is_empty() || trimmed.len() < 5 {
                continue;
            }

            let mut confidence: f64 = 0.0;

            // Layer 1: Check for question mark
            if trimmed.ends_with('?') {
                confidence = 0.95;
            }

            // Layer 1: Check for interrogative word starters
            if confidence < 0.5 {
                let lower = trimmed.to_lowercase();
                let first_word = lower.split_whitespace().next().unwrap_or("");
                if INTERROGATIVE_STARTERS.contains(&first_word) {
                    // Interrogative word at start without question mark
                    confidence = confidence.max(0.6);
                }
            }

            // Layer 2: Check for interview-specific patterns
            let lower = trimmed.to_lowercase();
            for pattern in INTERVIEW_PATTERNS {
                if lower.contains(pattern) {
                    // Interview patterns are strong signals
                    confidence = confidence.max(0.85);
                    break;
                }
            }

            // STT frequently omits punctuation. Promote direct question syntax,
            // but leave declarative clauses ("how we deploy") below the fallback threshold.
            if direct_english_question(&lower) {
                confidence = confidence.max(0.8);
            }

            // Word boundaries avoid matching syllables inside unrelated Turkish words.
            if lower.split_whitespace().any(|w| ["nasıl", "neden", "nereden", "nerede", "hangi", "mı", "mi", "mu", "mü"].contains(&w.trim_matches(|c: char| !c.is_alphabetic()))) {
                confidence = confidence.max(0.75);
            }
            // Only include if confidence is above threshold
            if confidence >= 0.5 {
                questions.push(DetectedQuestion {
                    text: trimmed.to_string(),
                    confidence,
                    timestamp_ms,
                    source: source.to_string(),
                });
            }
        }

        questions
    }
}

fn direct_english_question(text: &str) -> bool {
    let words: Vec<_> = text.split_whitespace().collect();
    if words.len() < 3 { return false; }
    let auxiliaries = ["am", "is", "are", "was", "were", "do", "does", "did", "can", "could", "would", "should", "will", "has", "have", "had"];
    let wh = ["what", "why", "how", "when", "where", "who", "which"];
    (wh.contains(&words[0]) && auxiliaries.contains(&words[1]))
        || (["which", "what"].contains(&words[0])
            && !["i", "you", "we", "they", "he", "she", "it"].contains(&words[1])
            && auxiliaries.contains(&words[2]))
        || (auxiliaries.contains(&words[0])
            && ["i", "you", "we", "they", "he", "she", "it", "this", "that", "these", "those", "there"].contains(&words[1]))
        || (words[0] == "what" && ["about", "happens"].contains(&words[1]))
}

/// Split text into sentences using common sentence terminators.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if ch == '.' || ch == '?' || ch == '!' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }

    // Don't forget the last sentence fragment (may not end with punctuation)
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_recognizes_direct_english_questions_without_stt_punctuation() {
        let detector = QuestionDetector::new();
        for text in [
            "How does this pipeline work", "Where is the purchase order stored",
            "Why did the extraction fail", "When will the release be ready",
            "Which checks are required", "What happens after human review",
            "Can we retry a failed document", "Do we have a rollback plan",
            "Is this feature enabled in production", "What about failed invoices",
        ] {
            assert!(detector.detect_questions(text, 1, "them").iter().any(|q| q.confidence >= 0.75), "Missed: {text}");
        }
    }

    #[test]
    fn fallback_does_not_promote_english_declarative_clauses() {
        let detector = QuestionDetector::new();
        for text in [
            "How we deploy depends on the environment", "What we need is a rollback plan",
            "What we are reviewing is the deployment", "Which we are fixing this week",
            "Where the data goes is documented", "Which checks run depends on the branch",
            "When we finish we will review it", "The pipeline validates every change",
            "Do not deploy before approval", "Can be retried after the review",
        ] {
            assert!(detector.detect_questions(text, 1, "them").iter().all(|q| q.confidence < 0.75), "False trigger: {text}");
        }
    }
}
