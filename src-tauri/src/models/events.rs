use serde::Serialize;

use super::argument_map::ArgumentMap;
use super::relationship::RelationshipEdge;
use super::emotion::{EmotionSnapshot, EmotionalProfile};
use super::llm::{LlmUsage, ProviderKind};
use super::message::{Message, Reaction};

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
    /// UserDriven: a participant chose not to respond this turn
    #[serde(rename_all = "camelCase")]
    SpeakerPassed { speaker_id: String, speaker_name: String },
    /// Cumulative reaction graph between participants (after each reaction round)
    #[serde(rename_all = "camelCase")]
    RelationshipsUpdated { edges: Vec<RelationshipEdge> },
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
