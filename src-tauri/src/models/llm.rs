//! Provider-agnostic LLM data types shared by the engine, the providers,
//! the persistence layer and the frontend (all `Serialize`/`Deserialize`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Which backend serves the discussion. Stored in settings as its lowercase name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Ollama,
    DeepSeek,
    /// Any OpenAI-compatible server (LM Studio, vLLM, llama.cpp, OpenRouter…), v1.20
    #[serde(rename = "openaiCompat")]
    OpenAiCompat,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::DeepSeek => "deepseek",
            Self::OpenAiCompat => "openaiCompat",
        }
    }

    /// Parse a stored value; unknown values fall back to Ollama (backward compatible).
    pub fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "deepseek" => Self::DeepSeek,
            "openaicompat" => Self::OpenAiCompat,
            _ => Self::Ollama,
        }
    }
}

/// Reasoning ("thinking") intensity requested for an LLM call.
///
/// `Auto` is a user-facing setting resolved by the engine before the call
/// (providers never receive `Auto`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningLevel {
    #[default]
    Off,
    Low,
    High,
    Max,
    Auto,
}

impl ReasoningLevel {
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Off | Self::Auto)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::High => "high",
            Self::Max => "max",
            Self::Auto => "auto",
        }
    }

    /// Parse a stored value; unknown values fall back to `Auto`.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "off" | "none" => Self::Off,
            "low" => Self::Low,
            "high" => Self::High,
            "max" => Self::Max,
            _ => Self::Auto,
        }
    }
}

/// How much wall-clock time the reasoning may take (v1.17). `Fast` caps the
/// `Auto` heuristic at `Low` and scales down the providers' reasoning allowances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningPace {
    #[default]
    Normal,
    Fast,
}

impl ReasoningPace {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Fast => "fast",
        }
    }

    /// Parse a stored value; unknown values fall back to `Normal`.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "fast" => Self::Fast,
            _ => Self::Normal,
        }
    }

    /// Level the `Auto` heuristic may reach under this pace (explicit levels are never capped).
    pub fn cap_auto(&self, level: ReasoningLevel) -> ReasoningLevel {
        match (self, level) {
            (Self::Fast, ReasoningLevel::High | ReasoningLevel::Max) => ReasoningLevel::Low,
            _ => level,
        }
    }
}

/// Purpose of an LLM call. Drives the reasoning policy, the provider-specific
/// token budget and the usage ledger breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallKind {
    Introduction,
    /// Legacy (≤ v1.16) free-text inner thought — kept for persisted ledgers; the engine now issues `Intention`
    Thought,
    Intervention,
    Reaction,
    Moderation,
    Memory,
    Emotion,
    Vote,
    SearchDecision,
    RagSelect,
    DocumentUpdate,
    ArgumentMap,
    Synthesis,
    Socratic,
    RespondOrPass,
    /// Structured pre-speech contract (target, goal, angle…) — replaces `Thought` (v1.17)
    Intention,
    /// Secret objective generated for a participant at the start
    Agenda,
    /// Cast suggestion for a topic (outside a discussion)
    Casting,
    /// End-of-discussion memory of a participant (long-term persona memory)
    Recap,
    /// Fused end-of-turn analysis (summary + positions + emotions) on sequential providers
    TurnAnalyst,
    /// Verdict / agreement vote at the end of trial-like modes
    Verdict,
    /// The moderator voices an act or scene announcement in its own words (v1.20.3)
    Announcement,
    /// The moderator writes the room's question to a participant (v1.20.3)
    AudienceQuestion,
    /// The crisis cell's dispatches, generated once at the start (v1.19)
    CrisisDispatches,
}

impl CallKind {
    /// Calls whose output is spoken content by a persona (streamed to the feed).
    /// Ollama applies the thinking `num_predict` multiplier to these only.
    pub fn is_discussion_content(&self) -> bool {
        matches!(self, Self::Introduction | Self::Thought | Self::Intervention)
    }
}

/// Token usage reported by a provider for one call (all zero when unknown).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsage {
    /// Prompt tokens, including cached ones.
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Subset of `prompt_tokens` served from the provider's prefix cache.
    #[serde(default)]
    pub cached_tokens: u32,
    /// Generated tokens, including reasoning tokens.
    #[serde(default)]
    pub completion_tokens: u32,
    /// Subset of `completion_tokens` spent on reasoning.
    #[serde(default)]
    pub reasoning_tokens: u32,
}

impl LlmUsage {
    pub fn add(&mut self, other: &LlmUsage) {
        self.prompt_tokens = self.prompt_tokens.saturating_add(other.prompt_tokens);
        self.cached_tokens = self.cached_tokens.saturating_add(other.cached_tokens);
        self.completion_tokens = self.completion_tokens.saturating_add(other.completion_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
    }

    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens.saturating_add(self.completion_tokens)
    }

    pub fn is_empty(&self) -> bool {
        self.prompt_tokens == 0 && self.completion_tokens == 0
    }
}

/// Accumulated usage for a discussion, broken down by call kind and speaker.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLedger {
    #[serde(default)]
    pub total: LlmUsage,
    /// Number of LLM calls that reported usage.
    #[serde(default)]
    pub calls: u32,
    #[serde(default)]
    pub by_call_kind: HashMap<CallKind, LlmUsage>,
    #[serde(default)]
    pub by_speaker: HashMap<String, LlmUsage>,
    /// Usage per model when several models serve the discussion (v1.20)
    #[serde(default)]
    pub by_model: HashMap<String, LlmUsage>,
    /// Estimated spend in USD (None: free provider or unknown price list).
    #[serde(default)]
    pub estimated_cost_usd: Option<f64>,
}

impl UsageLedger {
    /// Record one call served by `model` (attributed per model as well, v1.20).
    /// `cost_usd` is the estimated price of this call when known.
    pub fn record(&mut self, call_kind: CallKind, speaker_id: Option<&str>, model: Option<&str>, usage: &LlmUsage, cost_usd: Option<f64>) {
        if usage.is_empty() {
            return;
        }
        self.total.add(usage);
        self.calls = self.calls.saturating_add(1);
        self.by_call_kind.entry(call_kind).or_default().add(usage);
        if let Some(id) = speaker_id {
            self.by_speaker.entry(id.to_string()).or_default().add(usage);
        }
        if let Some(m) = model {
            self.by_model.entry(m.to_string()).or_default().add(usage);
        }
        if let Some(c) = cost_usd {
            self.estimated_cost_usd = Some(self.estimated_cost_usd.unwrap_or(0.0) + c);
        }
    }
}

/// Usage accumulated over a rolling monthly period (persisted in settings as JSON).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeriodUsage {
    #[serde(default)]
    pub usage: LlmUsage,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub discussions: u32,
}

impl PeriodUsage {
    pub fn add_discussion(&mut self, usage: &LlmUsage, cost_usd: f64) {
        self.usage.add(usage);
        self.cost_usd += cost_usd;
        self.discussions = self.discussions.saturating_add(1);
    }
}

/// Archived period (history list in settings).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeriodHistoryEntry {
    pub period_start: String,
    pub period_end: String,
    #[serde(flatten)]
    pub usage: PeriodUsage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_roundtrip() {
        assert_eq!(ProviderKind::parse("deepseek"), ProviderKind::DeepSeek);
        assert_eq!(ProviderKind::parse("DeepSeek"), ProviderKind::DeepSeek);
        assert_eq!(ProviderKind::parse("ollama"), ProviderKind::Ollama);
        assert_eq!(ProviderKind::parse(""), ProviderKind::Ollama);
        assert_eq!(ProviderKind::parse("unknown"), ProviderKind::Ollama);
        assert_eq!(serde_json::to_string(&ProviderKind::DeepSeek).unwrap(), "\"deepseek\"");
        assert_eq!(ProviderKind::DeepSeek.as_str(), "deepseek");
    }

    #[test]
    fn reasoning_level_roundtrip_and_activity() {
        assert_eq!(ReasoningLevel::parse("none"), ReasoningLevel::Off);
        assert_eq!(ReasoningLevel::parse("MAX"), ReasoningLevel::Max);
        assert_eq!(ReasoningLevel::parse("garbage"), ReasoningLevel::Auto);
        assert!(!ReasoningLevel::Off.is_active());
        assert!(!ReasoningLevel::Auto.is_active());
        assert!(ReasoningLevel::Low.is_active());
        assert_eq!(serde_json::to_string(&ReasoningLevel::High).unwrap(), "\"high\"");
        let parsed: ReasoningLevel = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(parsed, ReasoningLevel::Auto);
    }

    #[test]
    fn call_kind_discussion_content() {
        assert!(CallKind::Intervention.is_discussion_content());
        assert!(CallKind::Introduction.is_discussion_content());
        assert!(CallKind::Thought.is_discussion_content());
        assert!(!CallKind::Synthesis.is_discussion_content());
        assert!(!CallKind::Reaction.is_discussion_content());
        assert_eq!(serde_json::to_string(&CallKind::SearchDecision).unwrap(), "\"searchDecision\"");
    }

    #[test]
    fn usage_add_and_ledger_breakdown() {
        let mut ledger = UsageLedger::default();
        let u1 = LlmUsage { prompt_tokens: 100, cached_tokens: 40, completion_tokens: 20, reasoning_tokens: 5 };
        let u2 = LlmUsage { prompt_tokens: 50, cached_tokens: 0, completion_tokens: 10, reasoning_tokens: 0 };
        ledger.record(CallKind::Intervention, Some("g1"), Some("m1"), &u1, Some(0.01));
        ledger.record(CallKind::Reaction, Some("g1"), Some("m2"), &u2, Some(0.005));
        assert_eq!(ledger.by_model.len(), 2);
        ledger.record(CallKind::Memory, None, None, &u2, None);
        ledger.record(CallKind::Memory, None, None, &LlmUsage::default(), Some(9.0)); // ignored

        assert_eq!(ledger.calls, 3);
        assert_eq!(ledger.total.prompt_tokens, 200);
        assert_eq!(ledger.total.completion_tokens, 40);
        assert_eq!(ledger.total.cached_tokens, 40);
        assert_eq!(ledger.total.total_tokens(), 240);
        assert_eq!(ledger.by_speaker["g1"].prompt_tokens, 150);
        assert_eq!(ledger.by_call_kind[&CallKind::Memory].prompt_tokens, 50);
        assert!(!ledger.by_speaker.contains_key("arbitre"));
        assert!((ledger.estimated_cost_usd.unwrap() - 0.015).abs() < 1e-12);

        let mut free = UsageLedger::default();
        free.record(CallKind::Memory, None, None, &u2, None);
        assert!(free.estimated_cost_usd.is_none());

        // Serializes with enum keys as strings (frontend-friendly)
        let json = serde_json::to_string(&ledger).unwrap();
        assert!(json.contains("\"intervention\""));
        assert!(json.contains("\"byCallKind\""));
    }

    #[test]
    fn period_usage_and_history_roundtrip() {
        let mut p = PeriodUsage::default();
        p.add_discussion(&LlmUsage { prompt_tokens: 10, cached_tokens: 0, completion_tokens: 5, reasoning_tokens: 0 }, 0.25);
        p.add_discussion(&LlmUsage { prompt_tokens: 10, cached_tokens: 0, completion_tokens: 5, reasoning_tokens: 0 }, 0.25);
        assert_eq!(p.discussions, 2);
        assert_eq!(p.usage.total_tokens(), 30);
        assert!((p.cost_usd - 0.5).abs() < 1e-12);

        let entry = PeriodHistoryEntry { period_start: "2026-08-10".into(), period_end: "2026-09-10".into(), usage: p.clone() };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"costUsd\":0.5"), "{json}");
        let back: PeriodHistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back, entry);
        // Legacy/partial JSON still parses
        let partial: PeriodUsage = serde_json::from_str("{}").unwrap();
        assert_eq!(partial, PeriodUsage::default());
    }

    #[test]
    fn usage_add_saturates() {
        let mut a = LlmUsage { prompt_tokens: u32::MAX, ..Default::default() };
        a.add(&LlmUsage { prompt_tokens: 10, ..Default::default() });
        assert_eq!(a.prompt_tokens, u32::MAX);
    }
}
