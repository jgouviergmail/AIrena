use serde::Serialize;

use super::agenda::AgendaReveal;
use super::outcome::{ModeOutcome, VotePhase};
use super::persona_memory::PersonaRecapRecord;
use super::argument_map::ArgumentMap;
use super::diagnostics::{DiscussionDiagnostics, TurnTimings};
use crate::engine::dramaturgy::ActKey;
use crate::engine::scene_events::SceneEvent;
use super::relationship::RelationshipEdge;
use super::source::{WebSourceInfo, WikiSourceInfo};
use super::emotion::{EmotionSnapshot, EmotionalProfile, RoomMood};
use super::llm::{LlmUsage, ProviderKind};
use super::memory::ParticipantPosition;
use super::message::{Message, Reaction};

/// Who plays what this turn (v1.19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignment {
    pub speaker_id: String,
    pub role: String,
    /// Display label in the discussion language
    pub label: String,
}

/// Events sent from the backend to the frontend via Channel<ArenaEvent>
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "data")]
pub enum ArenaEvent {
    /// Discussion started successfully
    #[serde(rename_all = "camelCase")]
    DiscussionStarted { discussion_id: String },
    /// Streaming message token
    #[serde(rename_all = "camelCase")]
    MessageChunk { speaker_id: String, chunk: String },
    /// Complete message (after streaming ends)
    MessageComplete { message: Message },
    /// Reaction emitted by a participant
    #[serde(rename_all = "camelCase")]
    ReactionEmitted {
        message_id: String,
        reaction: Reaction,
    },
    /// Streaming inner thought token
    #[serde(rename_all = "camelCase")]
    ThoughtChunk { speaker_id: String, chunk: String },
    /// Complete inner thought
    #[serde(rename_all = "camelCase")]
    ThoughtComplete { speaker_id: String, thought: String },
    /// New turn started
    #[serde(rename_all = "camelCase")]
    TurnStarted {
        turn_number: u32,
        speaker_order: Vec<String>,
    },
    /// Turn skipped (all banned)
    #[serde(rename_all = "camelCase")]
    TurnSkipped {
        reason: String,
        next_available_turn: u32,
    },
    /// Turn order is being determined (democratic/authoritarian modes)
    #[serde(rename_all = "camelCase")]
    DeterminingOrder { turn_number: u32 },
    /// Active speaker changed
    #[serde(rename_all = "camelCase")]
    SpeakerActive { speaker_id: String },
    /// Emotions updated (rule-based, instant)
    #[serde(rename_all = "camelCase")]
    EmotionUpdated {
        speaker_id: String,
        emotions: EmotionalProfile,
        mood_summary: Option<String>,
    },
    /// Ban issued by the IArbitre
    #[serde(rename_all = "camelCase")]
    BanIssued {
        banned_id: String,
        banned_name: String,
        reason: String,
        duration: u32,
    },
    /// Ban lifted (participant returns)
    #[serde(rename_all = "camelCase")]
    BanLifted {
        speaker_id: String,
        speaker_name: String,
    },
    /// It's the user's turn
    UserTurnReady,
    /// User intervention timed out
    UserTurnTimeout,
    /// Step mode (v1.20.1): the engine waits for the audience's cue before this speaker talks
    #[serde(rename_all = "camelCase")]
    AwaitingCue { speaker_id: String, speaker_name: String },
    /// Pause confirmed
    PauseConfirmed,
    /// Resume confirmed
    ResumeConfirmed,
    /// Streaming synthesis token
    SynthesisChunk { chunk: String },
    /// Final synthesis complete
    SynthesisComplete { summary: String },
    /// Web search performed by a speaker (batched, one event per speaker per turn)
    #[serde(rename_all = "camelCase")]
    WebSearchPerformed {
        speaker_id: String,
        speaker_name: String,
        queries: Vec<String>,
        results_count: u32,
        pool_used: u32,
        /// Title, link and excerpt of every result injected (v1.17 — sources panel)
        results: Vec<WebSourceInfo>,
    },
    /// Full emotion history for a participant (emitted end of each turn)
    #[serde(rename_all = "camelCase")]
    EmotionHistoryUpdate {
        speaker_id: String,
        history: Vec<EmotionSnapshot>,
    },
    /// A critical emotional threshold was crossed
    #[serde(rename_all = "camelCase")]
    EmotionalThresholdCrossed {
        speaker_id: String,
        axis: String,
        direction: String,
        value: u8,
    },
    /// Wikipedia search performed by a speaker (batched, one event per speaker per turn)
    #[serde(rename_all = "camelCase")]
    WikiSearchPerformed {
        speaker_id: String,
        speaker_name: String,
        queries: Vec<String>,
        results_count: u32,
        pool_used: u32,
        /// URLs of Wikipedia articles found (for clickable links in the feed)
        article_urls: Vec<String>,
        /// Title, link and excerpt of every article injected (v1.17 — sources panel)
        articles: Vec<WikiSourceInfo>,
    },
    /// Dynamic behavioral directive generated for a speaker (for UI visualization)
    #[serde(rename_all = "camelCase")]
    DirectiveGenerated {
        speaker_id: String,
        speaker_name: String,
        speech_act: String,
        emotion_behavior: Option<String>,
        relationship_summary: String,
        /// Participant the speaker was asked to address in priority (None = the topic)
        focus_speaker: Option<String>,
        /// Reasoning level resolved for this intervention ("off" | "low" | "high" | …)
        reasoning_level: String,
    },
    /// Pre-speech contract of a speaker (v1.17): whom they address and why
    #[serde(rename_all = "camelCase")]
    IntentionGenerated {
        speaker_id: String,
        speaker_name: String,
        /// Participant addressed (display name); None = the topic
        target: Option<String>,
        /// `IntentionGoal` as its wire name ("convince" | "nuance" | …)
        goal: String,
        angle: String,
        concession: Option<String>,
        question: Option<String>,
    },
    /// Positions of every participant after the end-of-turn memory update (v1.17)
    #[serde(rename_all = "camelCase")]
    PositionsUpdated { positions: Vec<ParticipantPosition> },
    /// UserDriven: a participant chose not to respond this turn
    #[serde(rename_all = "camelCase")]
    SpeakerPassed { speaker_id: String, speaker_name: String },
    /// Cumulative reaction graph between participants (after each reaction round)
    #[serde(rename_all = "camelCase")]
    RelationshipsUpdated { edges: Vec<RelationshipEdge> },
    /// A relationship changed class after a reaction (v1.17): `from` / `to` are
    /// "ally" | "rival" | "tense" | "none"
    #[serde(rename_all = "camelCase")]
    RelationshipShift { a: String, b: String, from: String, to: String },
    /// Temperature of the room after an emotion round (v1.17)
    #[serde(rename_all = "camelCase")]
    RoomMoodUpdated { avg: EmotionalProfile, label: RoomMood },
    /// Wall-clock phases of the turn that just ended (v1.17)
    #[serde(rename_all = "camelCase")]
    TurnTimings { timings: TurnTimings },
    /// A new act of the mode's script begins (v1.18)
    #[serde(rename_all = "camelCase")]
    ActStarted { turn: u32, act: ActKey, title: String },
    /// The moderator broke the routine of the turn (v1.18)
    #[serde(rename_all = "camelCase")]
    SceneEventTriggered { turn: u32, event: SceneEvent, participants: Vec<String> },
    /// Two allies relay each other this turn (v1.18)
    #[serde(rename_all = "camelCase")]
    CoalitionFormed { turn: u32, a: String, b: String, a_name: String, b_name: String },
    /// The secret agendas, unveiled after the synthesis (v1.19)
    #[serde(rename_all = "camelCase")]
    AgendaRevealed { agendas: Vec<AgendaReveal> },
    /// Roles (trial, Oxford) or hats (six hats) of the participants, dealt for this turn (v1.19)
    #[serde(rename_all = "camelCase")]
    RolesAssigned { turn: u32, roles: Vec<RoleAssignment> },
    /// The audience is asked to vote on the motion (Oxford, v1.19)
    #[serde(rename_all = "camelCase")]
    AudienceVoteRequested { phase: VotePhase, timeout_secs: u64 },
    /// The audience's vote was taken into account (v1.19)
    #[serde(rename_all = "camelCase")]
    AudienceVoteRecorded { phase: VotePhase, choice: String },
    /// Mode-specific result (verdict, agreement, audience swing), before the synthesis (v1.19)
    #[serde(rename_all = "camelCase")]
    OutcomeReady { outcome: ModeOutcome },
    /// What a persona keeps of the discussion (long memory, v1.20) — persisted with the discussion
    #[serde(rename_all = "camelCase")]
    PersonaRecapReady { recap: PersonaRecapRecord },
    /// Diagnostics of the whole discussion, just before `DiscussionEnded` (v1.17)
    #[serde(rename_all = "camelCase")]
    DiagnosticsReady { diagnostics: DiscussionDiagnostics },
    /// Document updated by a speaker (co-construction)
    #[serde(rename_all = "camelCase")]
    DocumentUpdated {
        speaker_id: String,
        speaker_name: String,
        content: String,
        format: String,
    },
    /// RAG knowledge base context injected for a speaker
    #[serde(rename_all = "camelCase")]
    RagContextInjected {
        speaker_id: String,
        speaker_name: String,
        chunks: Vec<crate::rag::RagChunkInfo>,
        /// Whether this result was served from the per-speaker cache.
        #[serde(default)]
        cached: bool,
    },
    /// Argument map updated after turn analysis
    #[serde(rename_all = "camelCase")]
    ArgumentMapUpdated {
        markdown: String,
        markdown_by_speaker: String,
        theses_count: u32,
        arguments_count: u32,
        /// Structured map (persisted by the frontend as `argument_map_json`)
        map: ArgumentMap,
        /// Ids of the nodes added by this extraction
        new_node_ids: Vec<String>,
        /// Arguments/theses discarded because a cap was reached
        dropped_count: u32,
        /// How deep the map goes and how many objections wait for an answer (v1.20.1)
        depth: crate::models::argument_map::DepthStats,
    },
    /// Token usage snapshot (after each speaker, end of turn, synthesis)
    #[serde(rename_all = "camelCase")]
    LlmUsageUpdated {
        provider: ProviderKind,
        model: String,
        total: LlmUsage,
        calls: u32,
        /// Estimated spend for this discussion (None: free provider / unknown price list)
        estimated_cost_usd: Option<f64>,
        /// Spend accumulated over the current monthly period before this discussion
        period_spent_usd: f64,
        /// Monthly cap (0 = unlimited)
        budget_usd: f64,
        /// Peak tariff currently in force
        peak: bool,
    },
    /// Monthly budget threshold reached ("warning" at 80%, "exceeded" at 100%)
    #[serde(rename_all = "camelCase")]
    BudgetAlert {
        level: String,
        spent_usd: f64,
        budget_usd: f64,
    },
    /// Discussion ended
    DiscussionEnded,
    /// Non-fatal error (displayed in feed)
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::{Reaction, ReactionType};

    #[test]
    fn test_reaction_emitted_serialization() {
        let event = ArenaEvent::ReactionEmitted {
            message_id: "msg-123".to_string(),
            reaction: Reaction {
                from_speaker_id: "glad-456".to_string(),
                from_speaker_name: "Le Scientifique".to_string(),
                reaction_type: ReactionType::Like,
                target_message_id: "msg-123".to_string(),
                justification: Some("Argument solide et bien documenté".to_string()),
                quote: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        println!("ReactionEmitted JSON: {json}");

        // Verify the exact field names the frontend expects
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "reactionEmitted", "variant name should be camelCase");
        let data = &value["data"];
        // THIS is the critical check: is it "messageId" or "message_id"?
        assert!(
            data.get("messageId").is_some(),
            "Expected 'messageId' (camelCase) but got keys: {:?}",
            data.as_object().unwrap().keys().collect::<Vec<_>>()
        );
        assert_eq!(data["messageId"], "msg-123");

        let reaction = &data["reaction"];
        assert_eq!(reaction["fromSpeakerId"], "glad-456");
        assert_eq!(reaction["reactionType"], "like");
    }
}
