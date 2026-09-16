//! End-to-end engine tests on a scripted `MockLlmProvider` (no network, no GPU).
//!
//! These pin the orchestration contract: event sequence, message accounting,
//! usage metering, cancellation and provider failures.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::db::schema;
use crate::engine::orchestrator::DiscussionEngine;
use crate::engine::token_budget;
use crate::llm::mock::MockLlmProvider;
use crate::llm::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse};
use crate::models::discussion::{DiscussionConfig, DiscussionMode, DocumentFormat, DocumentInjectionMode, DocumentUpdateGranularity, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::models::gladiateur::GladIAteurConfig;
use crate::models::iarbitre::IArbitreConfig;
use crate::models::llm::{CallKind, LlmUsage, ReasoningLevel};
use crate::models::settings::LlmParams;

// ── Harness ─────────────────────────────────────────────────────────────

/// Collects every event emitted on the channel as parsed JSON.
fn event_channel() -> (Channel<ArenaEvent>, Arc<Mutex<Vec<serde_json::Value>>>) {
    let sink: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let sink_clone = Arc::clone(&sink);
    let channel = Channel::new(move |body| {
        if let InvokeResponseBody::Json(json) = body {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
                sink_clone.lock().unwrap().push(v);
            }
        }
        Ok(())
    });
    (channel, sink)
}

fn event_types(events: &[serde_json::Value]) -> Vec<String> {
    events.iter().filter_map(|e| e["type"].as_str().map(str::to_string)).collect()
}

fn count(events: &[serde_json::Value], ty: &str) -> usize {
    events.iter().filter(|e| e["type"] == ty).count()
}

fn glad(id: &str, name: &str, n: u32) -> GladIAteurConfig {
    GladIAteurConfig {
        id: id.to_string(),
        name: name.to_string(),
        intervention_number: n,
        system_prompt: format!("<persona>{name}</persona>"),
        llm_params: LlmParams::default(),
        emoji: None,
        initial_emotions: None,
    }
}

fn config(max_turns: u32) -> DiscussionConfig {
    DiscussionConfig {
        topic: "L'IA va-t-elle remplacer les développeurs ?".to_string(),
        discussion_language: "fr".to_string(),
        arbitre: IArbitreConfig {
            id: "arb".to_string(),
            name: "Le Modérateur".to_string(),
            system_prompt: "<persona>moderateur</persona>".to_string(),
            turn_distribution: TurnDistribution::Sequential,
            llm_params: LlmParams::default(),
            web_search_intro: false,
            wiki_search_intro: false,
        },
        gladiateurs: vec![glad("g1", "Le Scientifique", 1), glad("g2", "Le Philosophe", 2)],
        max_turns: Some(max_turns),
        user_name: "Léo".to_string(),
        user_intervention_timeout_secs: 1,
        web_search_pool: 0,
        wiki_search_pool: 0,
        discussion_mode: DiscussionMode::Debate,
        document_format: DocumentFormat::None,
        argument_map_enabled: false,
        document_injection_mode: DocumentInjectionMode::Rag,
        document_update_granularity: DocumentUpdateGranularity::Turn,
    }
}

/// Last `emotionUpdated` payload emitted for a speaker.
fn last_emotions(events: &[serde_json::Value], speaker_id: &str) -> serde_json::Value {
    events
        .iter()
        .rev()
        .find(|e| e["type"] == "emotionUpdated" && e["data"]["speakerId"] == speaker_id)
        .map(|e| e["data"]["emotions"].clone())
        .unwrap_or_else(|| panic!("no emotionUpdated for {speaker_id}"))
}

/// Name after the focus phrase in an intervention prompt, if any.
fn focus_target(user_prompt: &str) -> Option<String> {
    const PHRASE: &str = "Adresse-toi en priorité à ";
    let start = user_prompt.find(PHRASE)? + PHRASE.len();
    let rest = &user_prompt[start..];
    let end = rest.find(" — ")?;
    Some(rest[..end].to_string())
}

fn usage(p: u32, c: u32) -> Option<LlmUsage> {
    Some(LlmUsage { prompt_tokens: p, cached_tokens: 0, completion_tokens: c, reasoning_tokens: 0 })
}

/// Canned, well-formed answers for every call kind.
fn default_script(req: &LlmRequest) -> Result<LlmResponse, LlmError> {
    let content = match req.call_kind {
        CallKind::Introduction => "Bienvenue dans ce débat. Le Scientifique, à vous.".to_string(),
        CallKind::Thought => "Je devrais insister sur les données.".to_string(),
        CallKind::Intervention => format!("Intervention de {} : les données montrent une transformation, pas un remplacement.", req.speaker_id.clone().unwrap_or_default()),
        CallKind::Reaction => "[]".to_string(),
        CallKind::Moderation => r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0}"#.to_string(),
        CallKind::Memory => r#"{"summary":"Les deux camps s'accordent sur une transformation.","positions":{"Le Scientifique":"prudent","Le Philosophe":"critique"}}"#.to_string(),
        CallKind::Emotion => "{}".to_string(),
        CallKind::Synthesis => "## Synthèse\n\nUne transformation plutôt qu'un remplacement.".to_string(),
        _ => "{}".to_string(),
    };
    Ok(LlmResponse { content, reasoning: None, usage: usage(100, 20), truncated: false })
}

/// Optional harness knobs.
#[derive(Default)]
struct RunOptions {
    cancel: Option<CancellationToken>,
    /// (period_spent_usd, monthly_budget_usd)
    budget: Option<(f64, f64)>,
    /// Global reasoning level (default: Auto)
    reasoning: Option<ReasoningLevel>,
}

async fn run_engine(
    provider: MockLlmProvider,
    cfg: DiscussionConfig,
    cancel: Option<CancellationToken>,
) -> (Vec<serde_json::Value>, crate::models::llm::UsageLedger, Vec<LlmRequest>) {
    let (events, ledger, calls, _db) = run_engine_with(provider, cfg, RunOptions { cancel, ..Default::default() }).await;
    (events, ledger, calls)
}

async fn run_engine_with(
    provider: MockLlmProvider,
    cfg: DiscussionConfig,
    opts: RunOptions,
) -> (Vec<serde_json::Value>, crate::models::llm::UsageLedger, Vec<LlmRequest>, tokio_rusqlite::Connection) {
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();

    let provider = Arc::new(provider);
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let argument_map_enabled = cfg.argument_map_enabled;
    let mut engine = DiscussionEngine::new(
        cfg,
        "disc-test".to_string(),
        provider_dyn,
        None,
        db.clone(),
        None,
        token_budget::default_priorities(),
    );
    engine.set_cancel_token(opts.cancel.unwrap_or_default());
    engine.set_emotion_driven(false);
    engine.set_argument_map_enabled(argument_map_enabled); // mirrors commands/discussion.rs
    engine.set_reasoning_options(opts.reasoning.unwrap_or(ReasoningLevel::Auto), true);
    if let Some((spent, budget)) = opts.budget {
        engine.set_budget_guard(spent, budget);
    }

    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    // Handle to inspect the ledger after the engine is consumed by run().
    let ledger_handle = engine_ledger_handle(&engine);
    engine.run(rx, channel).await;

    let events = sink.lock().unwrap().clone();
    let ledger = ledger_handle();
    (events, ledger, provider.recorded_calls(), db)
}

fn billable_caps() -> LlmCapabilities {
    LlmCapabilities {
        supports_reasoning: true,
        reasoning_levels: true,
        reasoning_displayable: true,
        supports_json_mode: true,
        context_tokens: 32_768,
        chars_per_token_latin: 3.3,
        chars_per_token_cjk: 1.7,
        reports_usage: true,
        billable: true,
    }
}

/// `run()` consumes the engine; snapshot the ledger through a closure captured
/// before the move (the metered provider is shared via Arc inside the engine).
fn engine_ledger_handle(engine: &DiscussionEngine) -> impl Fn() -> crate::models::llm::UsageLedger {
    let snapshot = engine.usage_snapshot_handle();
    move || snapshot()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn full_debate_two_turns_emits_expected_sequence_and_meters_usage() {
    let provider = MockLlmProvider::scripted(default_script);
    let (events, ledger, calls) = run_engine(provider, config(2), None).await;
    let types = event_types(&events);

    assert_eq!(types.first().map(String::as_str), Some("discussionStarted"));
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"));
    assert_eq!(count(&events, "turnStarted"), 2, "{types:?}");
    // 1 introduction + 2 gladiateurs × 2 turns
    assert_eq!(count(&events, "messageComplete"), 5, "{types:?}");
    assert_eq!(count(&events, "synthesisComplete"), 1);
    assert!(count(&events, "messageChunk") > 5, "content must be streamed");
    assert_eq!(count(&events, "error"), 0, "{types:?}");

    // Speaker order follows intervention_number
    let first_turn = events.iter().find(|e| e["type"] == "turnStarted").unwrap();
    assert_eq!(first_turn["data"]["speakerOrder"], serde_json::json!(["g1", "g2"]));

    // Every LLM call reported usage → ledger totals match the number of calls
    assert_eq!(ledger.calls as usize, calls.len());
    assert_eq!(ledger.total.prompt_tokens as usize, 100 * calls.len());
    assert!(ledger.by_speaker.contains_key("g1"));
    assert!(ledger.by_call_kind.contains_key(&CallKind::Synthesis));

    // Utility calls never request reasoning and are JSON-mode
    for c in calls.iter().filter(|c| matches!(c.call_kind, CallKind::Reaction | CallKind::Moderation | CallKind::Memory | CallKind::Emotion)) {
        assert_eq!(c.reasoning, ReasoningLevel::Off);
        assert!(c.json_mode, "{:?} must be JSON mode", c.call_kind);
    }
    // No reactions on turn 1 (nothing to react to), reactions on turn 2 for both speakers
    let seq: Vec<String> = calls.iter().map(|c| format!("{:?}/{}", c.call_kind, c.speaker_id.clone().unwrap_or_default())).collect();
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Reaction).count(), 2, "{seq:?}");
    // Memory + emotion analysis once per turn
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Memory).count(), 2);
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Emotion).count(), 2);
}

#[tokio::test]
async fn reasoning_capable_provider_replaces_thought_phase_and_streams_reasoning() {
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        if req.call_kind == CallKind::Intervention && req.reasoning.is_active() {
            r.reasoning = Some("Raisonnement interne du modèle.".to_string());
        }
        Ok(r)
    })
    .with_capabilities(LlmCapabilities {
        supports_reasoning: true,
        reasoning_levels: true,
        reasoning_displayable: true,
        supports_json_mode: true,
        context_tokens: 32_768,
        chars_per_token_latin: 3.3,
        chars_per_token_cjk: 1.7,
        reports_usage: true,
        billable: true,
    });
    let mut cfg = config(2);
    // Force reasoning on for gladiateurs (Auto is probabilistic)
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::High);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    // No separate Thought call when reasoning is active
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Thought).count(), 0);
    let interventions: Vec<_> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert_eq!(interventions.len(), 4);
    assert!(interventions.iter().all(|c| c.reasoning == ReasoningLevel::High));
    // Synthesis uses deep reasoning on level-capable providers; introduction defaults to Low
    assert_eq!(calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap().reasoning, ReasoningLevel::High);
    assert_eq!(calls.iter().find(|c| c.call_kind == CallKind::Introduction).unwrap().reasoning, ReasoningLevel::Low);

    // Reasoning streamed as thought chunks and stored on the message as `reasoning`
    assert!(count(&events, "thoughtChunk") > 0);
    let msg = events
        .iter()
        .filter(|e| e["type"] == "messageComplete")
        .find(|e| e["data"]["message"]["role"] == "GladIAteur")
        .unwrap();
    assert_eq!(msg["data"]["message"]["thoughtKind"], "reasoning");
    assert_eq!(msg["data"]["message"]["innerThought"], "Raisonnement interne du modèle.");
}

/// Case 14 — reasoning hidden by the setting: nothing streamed live, but the
/// reasoning stays available on the message ("show reasoning" toggle).
#[tokio::test]
async fn hidden_reasoning_setting_streams_nothing_but_keeps_inner_thought() {
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        if req.call_kind == CallKind::Intervention {
            r.reasoning = Some("Raisonnement interne du modèle.".to_string());
        }
        Ok(r)
    })
    .with_capabilities(billable_caps());
    let mut cfg = config(1);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::High);
    }
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider = Arc::new(provider);
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut engine = DiscussionEngine::new(cfg, "disc-hidden".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    engine.set_reasoning_options(ReasoningLevel::Auto, false); // ← hidden
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();

    assert_eq!(count(&events, "thoughtChunk"), 0, "hidden: no live stream");
    assert_eq!(count(&events, "thoughtComplete"), 0);
    let msg = events.iter().filter(|e| e["type"] == "messageComplete").find(|e| e["data"]["message"]["role"] == "GladIAteur").unwrap();
    assert_eq!(msg["data"]["message"]["thoughtKind"], "reasoning");
    assert_eq!(msg["data"]["message"]["innerThought"], "Raisonnement interne du modèle.", "kept on demand");
}

#[tokio::test]
async fn ollama_like_provider_keeps_thought_phase_and_hides_reasoning() {
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        if req.call_kind == CallKind::Intervention {
            r.reasoning = Some("We need to respond as the persona...".to_string());
        }
        Ok(r)
    })
    .with_capabilities(LlmCapabilities {
        supports_reasoning: true,
        reasoning_levels: false,
        reasoning_displayable: false,
        supports_json_mode: true,
        context_tokens: 8192,
        chars_per_token_latin: 3.8,
        chars_per_token_cjk: 1.5,
        reports_usage: true,
        billable: false,
    });
    let mut cfg = config(1);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::Off);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    // Off → classic thought + intervention path
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Thought).count(), 2);
    let msg = events
        .iter()
        .filter(|e| e["type"] == "messageComplete")
        .find(|e| e["data"]["message"]["role"] == "GladIAteur")
        .unwrap();
    assert_eq!(msg["data"]["message"]["thoughtKind"], "persona");
    assert_eq!(msg["data"]["message"]["innerThought"], "Je devrais insister sur les données.");
    // Synthesis never requests reasoning on providers without levels
    assert_eq!(calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap().reasoning, ReasoningLevel::Off);
}

#[tokio::test]
async fn reasoning_failure_falls_back_then_disables_after_threshold() {
    // Reasoning interventions always come back empty → fallback each time,
    // and after REASONING_MAX_FAILURES no more reasoning requests are issued.
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        if req.call_kind == CallKind::Intervention && req.reasoning.is_active() {
            r.content = String::new();
        }
        Ok(r)
    })
    .with_capabilities(LlmCapabilities {
        supports_reasoning: true,
        reasoning_levels: true,
        reasoning_displayable: true,
        supports_json_mode: true,
        context_tokens: 32_768,
        chars_per_token_latin: 3.3,
        chars_per_token_cjk: 1.7,
        reports_usage: true,
        billable: true,
    });
    let mut cfg = config(2);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::Low);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    let reasoning_attempts = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.reasoning.is_active()).count();
    assert_eq!(reasoning_attempts as u32, crate::constants::REASONING_MAX_FAILURES);
    // Every intervention still produced a message thanks to the fallback
    assert_eq!(count(&events, "messageComplete"), 5);
    assert_eq!(count(&events, "error"), 0);
}

#[tokio::test]
async fn cancellation_during_run_ends_cleanly_with_discussion_ended() {
    let cancel = CancellationToken::new();
    let cancel_after_intro = cancel.clone();
    let provider = MockLlmProvider::scripted(move |req| {
        let r = default_script(req)?;
        if req.call_kind == CallKind::Introduction {
            cancel_after_intro.cancel();
        }
        Ok(r)
    });
    let (events, _ledger, _calls) = run_engine(provider, config(5), Some(cancel)).await;
    let types = event_types(&events);
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"), "{types:?}");
    assert!(count(&events, "turnStarted") <= 1, "{types:?}");
}

#[tokio::test]
async fn usage_events_carry_totals_and_cost_for_billable_provider() {
    // 1M prompt tokens per call on deepseek-flash → 0.30 USD (peak) or 0.15 (off-peak) per call
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        r.usage = Some(LlmUsage { prompt_tokens: 1_000_000, cached_tokens: 0, completion_tokens: 0, reasoning_tokens: 0 });
        Ok(r)
    })
    .with_capabilities(billable_caps())
    .with_model_name("deepseek-flash");
    let (events, ledger, calls, db) = run_engine_with(provider, config(1), RunOptions::default()).await;

    let usage_events: Vec<_> = events.iter().filter(|e| e["type"] == "llmUsageUpdated").collect();
    assert!(usage_events.len() >= 3, "after intro, after each speaker, end of turn, after synthesis");
    let last = usage_events.last().unwrap();
    assert_eq!(last["data"]["provider"], "ollama"); // mock reports Ollama kind
    assert_eq!(last["data"]["model"], "deepseek-flash");
    assert_eq!(last["data"]["calls"].as_u64().unwrap() as usize, calls.len());
    assert_eq!(last["data"]["total"]["promptTokens"].as_u64().unwrap() as usize, 1_000_000 * calls.len());
    let cost = last["data"]["estimatedCostUsd"].as_f64().unwrap();
    let per_call = cost / calls.len() as f64;
    assert!((per_call - 0.30).abs() < 1e-6 || (per_call - 0.15).abs() < 1e-6, "{per_call}");
    assert!(ledger.estimated_cost_usd.is_some());
    // Snapshots are monotonic
    let costs: Vec<f64> = usage_events.iter().map(|e| e["data"]["estimatedCostUsd"].as_f64().unwrap_or(0.0)).collect();
    assert!(costs.windows(2).all(|w| w[0] <= w[1]), "{costs:?}");

    // Period recorded in settings at the end of the discussion
    let period = crate::db::repository::get_deepseek_period_usage(&db).await.unwrap();
    assert_eq!(period.discussions, 1);
    assert_eq!(period.usage.prompt_tokens as usize, 1_000_000 * calls.len());
    assert!((period.cost_usd - cost).abs() < 1e-9);
}

#[tokio::test]
async fn free_provider_emits_usage_without_cost_and_records_nothing() {
    let provider = MockLlmProvider::scripted(default_script);
    let (events, _ledger, _calls, db) = run_engine_with(provider, config(1), RunOptions::default()).await;
    let last = events.iter().rev().find(|e| e["type"] == "llmUsageUpdated").unwrap();
    assert!(last["data"]["estimatedCostUsd"].is_null());
    assert!(last["data"]["total"]["promptTokens"].as_u64().unwrap() > 0);
    let period = crate::db::repository::get_deepseek_period_usage(&db).await.unwrap();
    assert_eq!(period.discussions, 0);
}

#[tokio::test]
async fn budget_warning_then_soft_stop_when_exhausted() {
    // Each call costs ≈0.15–0.30 USD; budget 1.00 with 0.60 already spent:
    // warning at 0.80 (after 1–2 calls), exceeded at 1.00 → soft stop after the current speaker.
    let provider = MockLlmProvider::scripted(|req| {
        let mut r = default_script(req)?;
        r.usage = Some(LlmUsage { prompt_tokens: 1_000_000, cached_tokens: 0, completion_tokens: 0, reasoning_tokens: 0 });
        Ok(r)
    })
    .with_capabilities(billable_caps())
    .with_model_name("deepseek-flash");
    let opts = RunOptions { budget: Some((0.60, 1.00)), ..Default::default() };
    let (events, _ledger, _calls, _db) = run_engine_with(provider, config(10), opts).await;
    let types = event_types(&events);

    let alerts: Vec<&str> = events.iter().filter(|e| e["type"] == "budgetAlert").map(|e| e["data"]["level"].as_str().unwrap()).collect();
    assert!(alerts.contains(&"exceeded"), "{alerts:?} {types:?}");
    assert_eq!(alerts.iter().filter(|l| **l == "exceeded").count(), 1, "exceeded fires once");
    assert!(alerts.iter().filter(|l| **l == "warning").count() <= 1, "warning fires at most once");
    // Stopped long before 10 turns, still synthesized and ended cleanly
    assert!(count(&events, "turnStarted") <= 2, "{types:?}");
    assert_eq!(count(&events, "synthesisComplete"), 1);
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"));
}

#[tokio::test]
async fn fatal_provider_error_stops_discussion_without_synthesis() {
    // Balance runs out on the first gladiateur intervention.
    let provider = MockLlmProvider::scripted(|req| match req.call_kind {
        CallKind::Intervention => Err(LlmError::InsufficientBalance),
        _ => default_script(req),
    })
    .with_capabilities(billable_caps());
    let (events, _ledger, calls, _db) = run_engine_with(provider, config(5), RunOptions::default()).await;
    let types = event_types(&events);
    assert!(count(&events, "error") >= 1, "{types:?}");
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"));
    // No synthesis call after a fatal error, single turn attempted
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Synthesis).count(), 0);
    assert_eq!(count(&events, "turnStarted"), 1, "{types:?}");
    assert_eq!(count(&events, "synthesisComplete"), 1, "empty synthesis still closes the UI state");
}

#[tokio::test]
async fn provider_failure_on_intervention_emits_error_and_continues() {
    let provider = MockLlmProvider::scripted(|req| match req.call_kind {
        CallKind::Intervention if req.speaker_id.as_deref() == Some("g2") => Err(LlmError::Overloaded),
        _ => default_script(req),
    });
    let (events, _ledger, _calls) = run_engine(provider, config(1), None).await;
    let types = event_types(&events);
    assert!(count(&events, "error") >= 1, "{types:?}");
    // g1 spoke, g2 failed, synthesis and end still happen
    assert_eq!(count(&events, "messageComplete"), 2, "{types:?}");
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"));
}

// ── Lot 7: engine consolidation (audit §5.2 cases 21–27) ─────────────────

/// Case 21 — the rotating focus must not converge on one participant.
/// The draw is random (thread RNG): the share bound is deliberately loose
/// (expected ≈ 25 % per name over 12 turns; the old behaviour was 100 % on the
/// first speaker of each turn).
#[tokio::test]
async fn focus_rotates_and_nobody_is_targeted_more_than_half_the_time() {
    let provider = MockLlmProvider::scripted(default_script);
    let mut cfg = config(12);
    cfg.gladiateurs = vec![
        glad("g1", "Le Scientifique", 1),
        glad("g2", "Le Philosophe", 2),
        glad("g3", "La Juriste", 3),
        glad("g4", "Le Poète", 4),
    ];
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);

    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert_eq!(interventions.len(), 48);
    // Turn 1: nobody to address yet
    for c in &interventions[..4] {
        assert!(focus_target(&c.user).is_none(), "no focus on turn 1");
        assert!(!c.user.contains("ne réponds à personne en particulier"));
    }
    // Turns ≥ 2: every prompt carries a focus (speaker or topic), never oneself
    let mut per_target: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut speaker_draws = 0usize;
    for c in &interventions[4..] {
        let has_topic = c.user.contains("ne réponds à personne en particulier");
        match focus_target(&c.user) {
            Some(name) => {
                assert!(!has_topic);
                let own_name = match c.speaker_id.as_deref() {
                    Some("g1") => "Le Scientifique", Some("g2") => "Le Philosophe", Some("g3") => "La Juriste", _ => "Le Poète",
                };
                assert_ne!(name, own_name, "a speaker never focuses on themselves");
                *per_target.entry(name).or_default() += 1;
                speaker_draws += 1;
            }
            None => assert!(has_topic, "turn ≥ 2 prompts must carry a focus: {}", &c.user[c.user.len().saturating_sub(600)..]),
        }
    }
    assert!(speaker_draws >= 20, "speaker_draws={speaker_draws}");
    assert!(per_target.len() >= 3, "attention must spread across participants: {per_target:?}");
    for (name, n) in &per_target {
        let share = *n as f64 / speaker_draws as f64;
        assert!(share <= 0.50, "{name} targeted {n}/{speaker_draws} times ({:.0}%)", share * 100.0);
    }
}

/// Case 22 — identical summaries two turns in a row = stagnation → engagement/curiosity drop.
#[tokio::test]
async fn stagnating_summaries_lower_engagement() {
    // default_script returns the same summary every turn → detected after turn 2, applied on turn 3
    let provider = MockLlmProvider::scripted(default_script);
    let (events, _, _) = run_engine(provider, config(3), None).await;
    let emo = last_emotions(&events, "g1");
    assert_eq!(emo["engagement"], 45, "{emo}");
    assert_eq!(emo["curiosite"], 45, "{emo}");
}

/// Case 22 (control) — a living discussion (new summary each turn) keeps engagement intact.
#[tokio::test]
async fn living_discussion_keeps_engagement() {
    let turn = Arc::new(AtomicU32::new(0));
    let t = Arc::clone(&turn);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Memory {
            let n = t.fetch_add(1, Ordering::SeqCst);
            let summaries = [
                "Les intervenants posent le cadre économique du remplacement.",
                "Le débat bascule vers la question éthique et juridique des responsabilités.",
                "Nouvelle piste : la formation continue comme réponse politique majeure.",
            ];
            let summary = summaries[(n as usize) % summaries.len()];
            return Ok(LlmResponse {
                content: format!(r#"{{"summary":"{summary}","positions":{{}}}}"#),
                reasoning: None, usage: usage(100, 20), truncated: false,
            });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, config(3), None).await;
    let emo = last_emotions(&events, "g1");
    assert_eq!(emo["engagement"], 50, "{emo}");
    assert_eq!(emo["curiosite"], 50, "{emo}");
}

/// Case 22 (drought) — no reaction at all for two turns is a stagnation signal too.
#[tokio::test]
async fn reaction_drought_is_a_stagnation_signal() {
    let turn = Arc::new(AtomicU32::new(0));
    let t = Arc::clone(&turn);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Memory {
            let n = t.fetch_add(1, Ordering::SeqCst);
            // Genuinely different wording each turn (word-set Jaccard ≈ 0)
            let summaries = [
                "Ouverture prudente centrée sur les gains de productivité.",
                "Bascule vers la responsabilité juridique et les biais algorithmiques.",
                "Nouvelle piste : formation continue financée par les entreprises.",
                "Confrontation autour du salaire minimum des développeurs juniors.",
                "Conclusion partielle : hybridation humain-machine plutôt que substitution.",
            ];
            let summary = summaries[(n as usize) % summaries.len()];
            return Ok(LlmResponse {
                content: format!(r#"{{"summary":"{summary}","positions":{{}}}}"#),
                reasoning: None, usage: usage(100, 20), truncated: false,
            });
        }
        default_script(req)
    });
    // Turns 2 and 3 without reactions → stagnating from turn 4 (2 turns × −5)
    let (events, _, _) = run_engine(provider, config(5), None).await;
    let emo = last_emotions(&events, "g1");
    assert_eq!(emo["engagement"], 40, "{emo}");
}

/// Case 23 — without events, emotions decay toward the persona's OWN baseline, not 50.
/// (Both personas share the profile so that group contagion is a no-op.)
#[tokio::test]
async fn emotions_stay_on_persona_baseline_without_events() {
    let provider = MockLlmProvider::scripted(default_script);
    let mut cfg = config(2);
    let hot_headed = r#"{"engagement":60,"accord":50,"confiance":50,"frustration":40,"curiosite":50,"enthousiasme":80}"#;
    cfg.gladiateurs[0].initial_emotions = Some(hot_headed.to_string());
    cfg.gladiateurs[1].initial_emotions = Some(hot_headed.to_string());
    let (events, _, _) = run_engine(provider, cfg, None).await;
    for id in ["g1", "g2"] {
        let emo = last_emotions(&events, id);
        assert_eq!(emo["frustration"], 40, "{id}: {emo}");
        assert_eq!(emo["enthousiasme"], 80, "{id}: {emo}");
        assert_eq!(emo["engagement"], 60, "{id}: {emo}");
    }
}

/// Case 24 — a ban hits the banned participant's emotions immediately.
#[tokio::test]
async fn ban_applies_immediate_emotional_penalty() {
    let banned = Arc::new(AtomicBool::new(false));
    let b = Arc::clone(&banned);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Moderation
            && req.user.contains("Le Scientifique")
            && !b.swap(true, Ordering::SeqCst)
        {
            return Ok(LlmResponse {
                content: r#"{"action":"ban","comment":"Attaque personnelle.","ban_reason":"attaque personnelle","ban_duration":1}"#.to_string(),
                reasoning: None, usage: usage(100, 20), truncated: false,
            });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, config(2), None).await;
    assert_eq!(count(&events, "banIssued"), 1);
    let g1 = last_emotions(&events, "g1");
    assert_eq!(g1["frustration"], 25, "{g1}"); // 10 + EMOTION_BAN_FRUST
    assert_eq!(g1["engagement"], 40, "{g1}"); // 50 − EMOTION_BAN_ENG
    // The banned speaker sits out turn 2
    let second_turn = events.iter().filter(|e| e["type"] == "turnStarted").nth(1).unwrap();
    assert_eq!(second_turn["data"]["speakerOrder"], serde_json::json!(["g2"]));
}

/// Case 25 — fiction: when the user does not write the opening, the first co-author does.
#[tokio::test]
async fn fiction_first_coauthor_writes_the_opening_when_user_skips() {
    let provider = MockLlmProvider::scripted(default_script);
    let mut cfg = config(1);
    cfg.discussion_mode = DiscussionMode::CollaborativeFiction;
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "userTurnTimeout"), 1);
    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert_eq!(interventions.len(), 2);
    assert!(interventions[0].user.contains("Personne n'a encore écrit"), "{}", interventions[0].user);
    assert!(!interventions[0].user.contains("l'auteur précédent"));
    assert!(interventions[1].user.contains("CONTINUE L'HISTOIRE EXACTEMENT"), "{}", interventions[1].user);
    assert!(interventions[1].user.contains("l'auteur précédent s'est arrêté"));
    assert!(!interventions[1].user.contains("Personne n'a encore écrit"));
    // Debate-only wording never leaks into fiction prompts
    for c in &interventions {
        assert!(!c.user.contains("TA PROPRE position"), "{}", c.user);
    }
}

/// Case 26 — UserDriven: participants who pass are announced, only responders speak.
#[tokio::test]
async fn user_driven_emits_speaker_passed_for_each_pass() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::RespondOrPass {
            let respond = req.speaker_id.as_deref() == Some("g3");
            return Ok(LlmResponse {
                content: format!(r#"{{"respond": {respond}}}"#),
                reasoning: None, usage: usage(100, 20), truncated: false,
            });
        }
        default_script(req)
    });
    let mut cfg = config(1);
    cfg.discussion_mode = DiscussionMode::UserDriven;
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    let (events, _, _) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "speakerPassed"), 2, "{:?}", event_types(&events));
    let passed: Vec<&str> = events.iter()
        .filter(|e| e["type"] == "speakerPassed")
        .map(|e| e["data"]["speakerId"].as_str().unwrap())
        .collect();
    assert_eq!(passed, vec!["g1", "g2"]);
    let turn = events.iter().find(|e| e["type"] == "turnStarted").unwrap();
    assert_eq!(turn["data"]["speakerOrder"], serde_json::json!(["g3"]));
    assert_eq!(count(&events, "messageComplete"), 2); // intro + g3
}

/// Case 27 — co-construction: one document regeneration per turn by default,
/// integrating every contribution; per-intervention granularity still available.
#[tokio::test]
async fn document_is_regenerated_once_per_turn_by_default() {
    let script = |req: &LlmRequest| {
        if req.call_kind == CallKind::DocumentUpdate {
            return Ok(LlmResponse {
                content: "# Plan\n\n- point consolidé".to_string(),
                reasoning: None, usage: usage(100, 20), truncated: false,
            });
        }
        default_script(req)
    };
    let mut cfg = config(2);
    cfg.discussion_mode = DiscussionMode::CoConstruction;
    cfg.document_format = DocumentFormat::Md;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(script), cfg.clone(), None).await;
    assert_eq!(count(&events, "documentUpdated"), 2, "{:?}", event_types(&events));
    let doc_calls: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::DocumentUpdate).collect();
    assert_eq!(doc_calls.len(), 2);
    for c in &doc_calls {
        assert!(c.user.contains("--- Le Scientifique ---") && c.user.contains("--- Le Philosophe ---"), "{}", c.user);
    }
    let updated = events.iter().find(|e| e["type"] == "documentUpdated").unwrap();
    assert_eq!(updated["data"]["speakerId"], "arb");
    assert_eq!(updated["data"]["format"], "md");

    cfg.document_update_granularity = DocumentUpdateGranularity::Intervention;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(script), cfg, None).await;
    assert_eq!(count(&events, "documentUpdated"), 4);
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::DocumentUpdate).count(), 4);
}

/// Socratic mode: IArbitre opens each turn ≥ 2 with a question and never repeats itself.
#[tokio::test]
async fn socratic_questions_are_remembered_across_turns() {
    let n = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&n);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Socratic {
            let i = c.fetch_add(1, Ordering::SeqCst) + 1;
            return Ok(LlmResponse { content: format!("Question socratique numéro {i} ?"), reasoning: None, usage: usage(100, 20), truncated: false });
        }
        default_script(req)
    });
    let mut cfg = config(3);
    cfg.discussion_mode = DiscussionMode::Socratic;
    let (_, _, calls) = run_engine(provider, cfg, None).await;
    let socratic: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Socratic).collect();
    assert_eq!(socratic.len(), 2, "turns 2 and 3");
    assert!(!socratic[0].user.contains("DÉJÀ posées"));
    assert!(socratic[1].user.contains("Question socratique numéro 1 ?"), "{}", socratic[1].user);
}

// ── Realistic simulations (every feature on, every mode) ─────────────────

/// A "model" that behaves like a real one: reactions with justifications,
/// evolving summaries, emotion deltas with a stagnation flag, argument
/// extractions, document updates, respond/pass decisions and an occasional
/// moderation comment. Speaker-aware so reactions never target oneself.
fn realistic_script(counter: Arc<AtomicU32>) -> impl Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync + 'static {
    move |req: &LlmRequest| {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        let speaker = req.speaker_id.clone().unwrap_or_default();
        let other = if speaker == "g1" { "Le Philosophe" } else { "Le Scientifique" };
        let content = match req.call_kind {
            CallKind::Introduction => "Bienvenue. Le sujet est ouvert : commençons.".to_string(),
            CallKind::Thought => format!("Réflexion privée n°{n}."),
            CallKind::Intervention => format!(
                "Intervention {n} de {speaker} : l'automatisation transforme le métier, chiffre {n} à l'appui. {other}, qu'en dites-vous ?"
            ),
            CallKind::Reaction => format!(
                r#"[{{"speaker":"{other}","reaction":"{}","justification":"argument {n}"}}]"#,
                if n.is_multiple_of(3) { "dislike" } else { "like" }
            ),
            CallKind::Moderation => if n.is_multiple_of(7) {
                r#"{"action":"comment","comment":"Restez sur le sujet.","ban_reason":"","ban_duration":0}"#.to_string()
            } else {
                r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0}"#.to_string()
            },
            CallKind::Memory => format!(
                r#"{{"summary":"Résumé du tour {n} : angle inédit {n} sur la productivité et la formation.","positions":{{"Le Scientifique":"prudent {n}","Le Philosophe":"critique {n}"}}}}"#
            ),
            CallKind::Emotion => format!(
                r#"{{"Le Scientifique":{{"engagement":3,"frustration":-2}},"Le Philosophe":{{"curiosite":25}},"Le Modérateur":{{}},"stagnating":{}}}"#,
                n.is_multiple_of(5)
            ),
            CallKind::ArgumentMap => format!(
                r#"{{"extractions":[{{"speaker":"{}","new_theses":["L'automatisation transforme le métier {n}"],"arguments":[{{"text":"Chiffre {n} à l'appui","type":"evidence","for_thesis":"L'automatisation transforme le métier {n}"}}]}}]}}"#,
                if n.is_multiple_of(2) { "Le Scientifique" } else { "Le Philosophe" }
            ),
            CallKind::DocumentUpdate => format!("# Plan\n\n- Point consolidé {n}\n"),
            CallKind::Socratic => format!("Question socratique {n} ?"),
            CallKind::RespondOrPass => format!(r#"{{"respond": {}}}"#, speaker != "g2"),
            CallKind::Synthesis => "## Synthèse\n\nTransformation plutôt que remplacement.".to_string(),
            CallKind::Vote | CallKind::SearchDecision | CallKind::RagSelect => "{}".to_string(),
        };
        Ok(LlmResponse { content, reasoning: Some(format!("raisonnement {n}")), usage: usage(400, 60), truncated: false })
    }
}

fn assert_emotions_in_range(events: &[serde_json::Value]) {
    for e in events.iter().filter(|e| e["type"] == "emotionUpdated") {
        for (axis, v) in e["data"]["emotions"].as_object().unwrap() {
            let v = v.as_i64().unwrap();
            assert!((0..=100).contains(&v), "axis {axis} out of range: {v}");
        }
    }
}

/// Six turns, three gladiateurs, argument map + co-construction document,
/// reasoning-capable billable provider: the full pipeline must run without a
/// single error event and keep every invariant.
#[tokio::test]
async fn realistic_full_feature_debate_runs_clean() {
    let provider = MockLlmProvider::scripted(realistic_script(Arc::new(AtomicU32::new(0))))
        .with_capabilities(billable_caps())
        .with_model_name("deepseek-flash");
    let mut cfg = config(6);
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    cfg.argument_map_enabled = true;
    cfg.document_format = DocumentFormat::Md;
    let (events, ledger, calls, db) = run_engine_with(
        provider,
        cfg,
        RunOptions { budget: Some((0.0, 5.0)), reasoning: Some(ReasoningLevel::High), ..Default::default() },
    )
    .await;
    let types = event_types(&events);

    assert_eq!(count(&events, "error"), 0, "{types:?}");
    assert_eq!(count(&events, "turnStarted"), 6);
    assert!(count(&events, "messageComplete") > 6 * 3, "intro + every intervention (+ moderation comments)");
    assert_eq!(count(&events, "documentUpdated"), 6, "one document integration per turn");
    assert!(count(&events, "argumentMapUpdated") >= 4, "map extracted on turns ≥ 2");
    assert!(count(&events, "relationshipsUpdated") >= 5, "reaction graph refreshed after reaction rounds");
    assert!(count(&events, "reactionEmitted") >= 10);
    assert!(count(&events, "thoughtChunk") > 0, "reasoning streamed live");
    assert_eq!(count(&events, "synthesisComplete"), 1);
    assert_eq!(types.last().map(String::as_str), Some("discussionEnded"));
    assert_emotions_in_range(&events);

    // With a fixed High level every intervention reasons natively → reasoning stored as the thought
    let interventions: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "GladIAteur")
        .collect();
    assert!(interventions.len() >= 18);
    assert!(interventions.iter().all(|m| m["data"]["message"]["thoughtKind"] == "reasoning"));

    // Usage: every call metered, cost computed, period persisted with one discussion
    assert_eq!(ledger.calls as usize, calls.len());
    assert!(ledger.estimated_cost_usd.unwrap_or(0.0) > 0.0);
    let period = crate::db::repository::get_deepseek_period_usage(&db).await.unwrap();
    assert_eq!(period.discussions, 1);
    assert_eq!(period.usage.prompt_tokens, ledger.total.prompt_tokens);

    // The last map event is consistent with its structured payload
    let last_map = events.iter().rev().find(|e| e["type"] == "argumentMapUpdated").unwrap();
    let theses = last_map["data"]["map"]["theses"].as_array().unwrap().len();
    assert_eq!(last_map["data"]["thesesCount"].as_u64().unwrap() as usize, theses);
    assert!(last_map["data"]["markdown"].as_str().unwrap().contains("✨"), "new nodes are flagged");
}

/// Every discussion mode must complete a short run without error events —
/// the mode templates and their engine branches are all exercised.
#[tokio::test]
async fn every_mode_completes_without_errors() {
    let modes = [
        DiscussionMode::Debate,
        DiscussionMode::Ideation,
        DiscussionMode::CoConstruction,
        DiscussionMode::UserDriven,
        DiscussionMode::Socratic,
        DiscussionMode::Tutorial,
        DiscussionMode::CritiqueReview,
        DiscussionMode::CollaborativeFiction,
    ];
    for mode in modes {
        let provider = MockLlmProvider::scripted(realistic_script(Arc::new(AtomicU32::new(0))));
        let mut cfg = config(2);
        cfg.discussion_mode = mode.clone();
        if mode == DiscussionMode::CoConstruction {
            cfg.document_format = DocumentFormat::Md;
        }
        let (events, _, _) = run_engine(provider, cfg, None).await;
        let types = event_types(&events);
        assert_eq!(count(&events, "error"), 0, "{mode:?}: {types:?}");
        assert_eq!(types.last().map(String::as_str), Some("discussionEnded"), "{mode:?}");
        assert!(count(&events, "turnStarted") >= 1, "{mode:?}: {types:?}");
        assert_eq!(count(&events, "synthesisComplete"), 1, "{mode:?}");
        assert_emotions_in_range(&events);
        match mode {
            DiscussionMode::UserDriven => assert!(count(&events, "speakerPassed") >= 1, "g2 passes each turn"),
            DiscussionMode::Socratic => assert!(events.iter().any(|e| {
                e["type"] == "messageComplete"
                    && e["data"]["message"]["content"].as_str().unwrap_or("").starts_with("Question socratique")
            })),
            DiscussionMode::CoConstruction => assert_eq!(count(&events, "documentUpdated"), 2),
            DiscussionMode::CollaborativeFiction => assert_eq!(count(&events, "userTurnTimeout"), 1),
            _ => {}
        }
    }
}
