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
use crate::engine::scene_events::SceneEventKind;
use crate::engine::token_budget;
use crate::llm::mock::MockLlmProvider;
use crate::llm::{LlmCapabilities, LlmError, LlmProvider, LlmRequest, LlmResponse};
use crate::models::discussion::{DiscussionConfig, DiscussionFeatures, DiscussionMode, DocumentFormat, DocumentInjectionMode, DocumentUpdateGranularity, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::events::ArenaEvent;
use crate::models::gladiateur::GladIAteurConfig;
use crate::models::iarbitre::IArbitreConfig;
use crate::models::llm::{CallKind, LlmUsage, ReasoningLevel, ReasoningPace};
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

/// Spoken messages (interventions, introduction, moderator comments) — not the
/// templated lines of the staging (act announcements, scene events, stage directions).
fn count_spoken(events: &[serde_json::Value]) -> usize {
    events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "normal").count()
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
        mode_role: None,
        model: None,
        source_profile_id: None,
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
            model: None,
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
        // The historical tests pin the v1.16 behaviour; liveliness tests opt in explicitly.
        features: DiscussionFeatures::legacy(),
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

/// The other gladiateur of the two-speaker fixtures (by display name).
fn other_name(req: &LlmRequest) -> &'static str {
    if req.speaker_id.as_deref() == Some("g1") { "Le Philosophe" } else { "Le Scientifique" }
}

/// A well-formed intention answer aimed at the other speaker.
fn intention_json(req: &LlmRequest, question: Option<&str>, concession: Option<&str>, answers: Option<usize>) -> String {
    let opt = |v: Option<&str>| v.map(|s| format!("\"{s}\"")).unwrap_or_else(|| "null".to_string());
    let answers = answers.map(|n| n.to_string()).unwrap_or_else(|| "null".to_string());
    format!(
        r#"{{"target":"{}","goal":"convaincre","angle":"les données d'abord","concession":{},"question":{},"answers":{answers},"thought":"Je devrais insister sur les données."}}"#,
        other_name(req), opt(concession), opt(question)
    )
}

/// A well-formed secret agenda naming the speaker (French keys, like a French run).
fn agenda_json(req: &LlmRequest) -> String {
    let id = req.speaker_id.clone().unwrap_or_default();
    format!(r#"{{"objectif":"faire admettre le coût ({id})","ligne_rouge":"jamais le remplacement total","victoire":"{} cite mes chiffres"}}"#, other_name(req))
}

/// Canned, well-formed answers for every call kind.
fn default_script(req: &LlmRequest) -> Result<LlmResponse, LlmError> {
    let content = match req.call_kind {
        CallKind::Introduction => "Bienvenue dans ce débat. Le Scientifique, à vous.".to_string(),
        CallKind::Intention => intention_json(req, None, None, None),
        CallKind::Agenda => agenda_json(req),
        CallKind::Intervention => format!("Intervention de {} : les données montrent une transformation, pas un remplacement.", req.speaker_id.clone().unwrap_or_default()),
        // One shape serves the trial verdict and the negotiation decision
        CallKind::Verdict => r#"{"verdict":"accusation","accepts":true,"reason":"les preuves l'emportent"}"#.to_string(),
        CallKind::CrisisDispatches => r#"{"dispatches":["Dépêche un : la panne s'étend.","Dépêche deux : un témoin parle.","Dépêche trois : l'échéance tombe."]}"#.to_string(),
        // The mock moderator "voices" an announcement by repeating its brief (the templates stay observable)
        CallKind::Announcement => req.user.lines().nth(1).unwrap_or_default().to_string(),
        CallKind::AudienceQuestion => r#"{"question":"Sur quelles données fondez-vous ce chiffre de transformation ?"}"#.to_string(),
        CallKind::Recap => r#"{"positions":["prudent sur l'automatisation"],"best_lines":["les données montrent"],"allies":[],"rivals":["Le Philosophe"],"lesson":"mesurer avant de juger"}"#.to_string(),
        CallKind::Reaction => "[]".to_string(),
        CallKind::Moderation => r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0}"#.to_string(),
        CallKind::Memory => r#"{"summary":"Les deux camps s'accordent sur une transformation.","positions":{"Le Scientifique":"prudent","Le Philosophe":"critique"}}"#.to_string(),
        CallKind::Emotion => "{}".to_string(),
        CallKind::TurnAnalyst => fused_analyst_json(
            r#"{"summary":"Les deux camps s'accordent sur une transformation.","positions":{"Le Scientifique":"prudent","Le Philosophe":"critique"}}"#,
            "{}",
        ),
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
    /// Reasoning pace (default: Normal)
    pace: Option<ReasoningPace>,
    /// Emotion-driven behaviour (default: false — the harness keeps prompts deterministic)
    emotion_driven: bool,
    /// Knowledge base handed to the engine (documents imported in the wizard)
    rag: Option<crate::rag::RagStore>,
    /// Long memory of the personas (v1.20): recall past recaps, write new ones
    persona_memory: bool,
    /// A database prepared by the test (past discussions, memories); fresh otherwise
    db: Option<tokio_rusqlite::Connection>,
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
    let db = match opts.db {
        Some(db) => db,
        None => {
            let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
            schema::initialize(&db).await.unwrap();
            db
        }
    };

    let provider = Arc::new(provider);
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let argument_map_enabled = cfg.argument_map_enabled;
    let mut engine = DiscussionEngine::new(
        cfg,
        "disc-test".to_string(),
        provider_dyn,
        None,
        db.clone(),
        opts.rag,
        token_budget::default_priorities(),
    );
    engine.set_cancel_token(opts.cancel.unwrap_or_default());
    engine.set_emotion_driven(opts.emotion_driven);
    engine.disable_random_staging(); // deterministic: scene events and coalitions only when forced
    engine.set_argument_map_enabled(argument_map_enabled); // mirrors commands/discussion.rs
    engine.set_reasoning_options(opts.reasoning.unwrap_or(ReasoningLevel::Auto), true, opts.pace.unwrap_or_default());
    engine.set_persona_memory(opts.persona_memory);
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
        max_parallel_calls: 4,
    }
}

/// Mock capabilities of a sequential provider (Ollama-like): the fused turn analyst path.
fn sequential_caps() -> LlmCapabilities {
    LlmCapabilities { max_parallel_calls: 1, ..MockLlmProvider::scripted(default_script).capabilities().clone() }
}

/// The fused end-of-turn answer: the memory fields plus an `emotions` object.
fn fused_analyst_json(memory_json: &str, emotion_json: &str) -> String {
    let mut memory: serde_json::Value = serde_json::from_str(memory_json).unwrap();
    let emotions: serde_json::Value = serde_json::from_str(emotion_json).unwrap();
    memory["emotions"] = emotions;
    memory.to_string()
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
    // 1 introduction + 2 gladiateurs × 2 turns (act announcements are staging lines, not speech)
    assert_eq!(count_spoken(&events), 5, "{types:?}");
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
        max_parallel_calls: 1,
    });
    let mut cfg = config(2);
    // Force reasoning on for gladiateurs (Auto is probabilistic)
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::High);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    // The intention contract (JSON, no reasoning) still precedes every reasoning intervention
    let intentions: Vec<_> = calls.iter().filter(|c| c.call_kind == CallKind::Intention).collect();
    assert_eq!(intentions.len(), 4);
    assert!(intentions.iter().all(|c| c.json_mode && c.reasoning == ReasoningLevel::Off));
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
    engine.set_reasoning_options(ReasoningLevel::Auto, false, ReasoningPace::Normal); // ← hidden
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();

    assert_eq!(count(&events, "thoughtChunk"), 0, "hidden: no live stream");
    // Only the persona thoughts of the intention contracts (one per speaker), never the model reasoning
    let thoughts: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "thoughtComplete").collect();
    assert_eq!(thoughts.len(), 2);
    assert!(thoughts.iter().all(|e| e["data"]["thought"] == "Je devrais insister sur les données."));
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
        max_parallel_calls: 1,
    });
    let mut cfg = config(1);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::Off);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    // Off → intention + plain intervention path, the intention's thought stored as the persona thought
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Intention).count(), 2);
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
        max_parallel_calls: 1,
    });
    let mut cfg = config(2);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::Low);
    }
    let (events, _ledger, calls) = run_engine(provider, cfg, None).await;

    let reasoning_attempts = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.reasoning.is_active()).count();
    assert_eq!(reasoning_attempts as u32, crate::constants::REASONING_MAX_FAILURES);
    // Every intervention still produced a message thanks to the fallback
    assert_eq!(count_spoken(&events), 5);
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
    // Long memory on: the recaps would be more billed calls after the cap — never written
    let opts = RunOptions { budget: Some((0.60, 1.00)), persona_memory: true, ..Default::default() };
    let mut cfg = config(10);
    cfg.gladiateurs[0].source_profile_id = Some("scientist".into());
    let (events, _ledger, calls, _db) = run_engine_with(provider, cfg, opts).await;
    let types = event_types(&events);
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Recap), "no recap once the budget is exhausted");

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
    assert_eq!(count_spoken(&events), 2, "{types:?}");
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
    // One stagnation penalty net of the homeostasis (v1.20.4): below 50, above 50 − penalty
    let eng = emo["engagement"].as_u64().unwrap();
    let cur = emo["curiosite"].as_u64().unwrap();
    assert!(eng < 50 && eng >= 50 - u64::from(crate::constants::EMOTION_STAGNATION_ENG), "{emo}");
    assert!(cur < 50 && cur >= 50 - u64::from(crate::constants::EMOTION_STAGNATION_CURIOSITE), "{emo}");
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
    // Turns 2 and 3 without reactions → stagnating from turn 4 (two penalties, net of the homeostasis)
    let (events, _, _) = run_engine(provider, config(5), None).await;
    let emo = last_emotions(&events, "g1");
    let eng = emo["engagement"].as_u64().unwrap();
    assert!(eng < 50 - u64::from(crate::constants::EMOTION_STAGNATION_ENG), "two stagnating turns bite deeper than one: {emo}");
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

// ── Intention, open loops, trajectory (Lot 5) ───────────────────────────

/// Intention script: g1 questions g2; g2 answers loop 1 and concedes a point.
fn intention_script(req: &LlmRequest) -> Result<LlmResponse, LlmError> {
    if req.call_kind == CallKind::Intention {
        let content = match req.speaker_id.as_deref() {
            Some("g1") => intention_json(req, Some("Et le coût social ?"), None, None),
            _ => intention_json(req, None, Some("le rythme compte"), Some(1)),
        };
        return Ok(LlmResponse { content, reasoning: None, usage: usage(60, 30), truncated: false });
    }
    default_script(req)
}

/// S8 — a valid intention: the contract is in the intervention prompt (target named),
/// the thought is the persona's, `IntentionGenerated` is emitted.
#[tokio::test]
async fn intention_contract_reaches_the_intervention_prompt() {
    let provider = MockLlmProvider::scripted(intention_script);
    let (events, _, calls) = run_engine(provider, config(1), None).await;
    assert_eq!(count(&events, "error"), 0);
    let generated: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "intentionGenerated").collect();
    assert_eq!(generated.len(), 2);
    assert_eq!(generated[0]["data"]["speakerId"], "g1");
    // Nobody had spoken before g1 (the act announcement is staging): the invented target is dropped
    assert!(generated[0]["data"]["target"].is_null(), "{}", generated[0]);
    assert_eq!(generated[0]["data"]["goal"], "convince");
    assert_eq!(generated[0]["data"]["question"], "Et le coût social ?");
    assert_eq!(generated[1]["data"]["target"], "Le Scientifique");
    assert!(generated[1]["data"]["question"].is_null());
    // ThoughtComplete carries the intention's thought, stored on the message
    let thoughts: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "thoughtComplete").collect();
    assert_eq!(thoughts.len(), 2);
    assert_eq!(thoughts[0]["data"]["thought"], "Je devrais insister sur les données.");
    let msg = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["speakerId"] == "g1").unwrap();
    assert_eq!(msg["data"]["message"]["innerThought"], "Je devrais insister sur les données.");
    assert_eq!(msg["data"]["message"]["thoughtKind"], "persona");
    // The intervention prompt carries the contract and the private reflection
    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert!(interventions[0].user.contains("[Ton intention]\nCible : le sujet — Objectif : convaincre — Angle : les données d'abord"), "{}", interventions[0].user);
    assert!(interventions[0].user.contains("Question à poser : Et le coût social ?"), "{}", interventions[0].user);
    assert!(interventions[0].user.contains("[Ta réflexion privée]\nJe devrais insister sur les données."), "{}", interventions[0].user);
    assert!(interventions[1].user.contains("[Ton intention]\nCible : Le Scientifique — Objectif : convaincre"), "{}", interventions[1].user);
    // The intention prompt is a JSON call; it lists whom the speaker may address once
    // someone has spoken (the first speaker of the discussion has nobody to address)
    let intentions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intention).collect();
    assert!(intentions[0].json_mode);
    assert!(!intentions[0].user.contains("[Participants que tu peux viser]"), "{}", intentions[0].user);
    assert!(intentions[0].user.contains("Tu es le premier à prendre la parole"), "{}", intentions[0].user);
    assert!(intentions[1].user.contains("[Participants que tu peux viser] Le Scientifique."), "{}", intentions[1].user);
    assert!(intentions[1].user.contains("Avant de parler, décide de ton intention"), "{}", intentions[1].user);
    assert!(intentions[0].user.contains("\"target\""));
}

/// S9 — the model answers in prose: the text is the thought, no intention, no error.
#[tokio::test]
async fn prose_intention_falls_back_to_a_plain_thought() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Intention {
            return Ok(LlmResponse { content: "Je vais insister sur le terrain.".to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls) = run_engine(provider, config(1), None).await;
    assert_eq!(count(&events, "error"), 0);
    assert_eq!(count(&events, "intentionGenerated"), 0);
    assert_eq!(count(&events, "thoughtComplete"), 2);
    let msg = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "GladIAteur").unwrap();
    assert_eq!(msg["data"]["message"]["innerThought"], "Je vais insister sur le terrain.");
    let first = calls.iter().find(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(first.user.contains("[Ta réflexion privée]\nJe vais insister sur le terrain."));
    assert!(!first.user.contains("[Ton intention]"));

    // Broken JSON: neither a thought nor an intention, and still no error
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Intention {
            return Ok(LlmResponse { content: "{\"target\": \"Le".to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, config(1), None).await;
    assert_eq!(count(&events, "error"), 0);
    assert_eq!(count(&events, "thoughtComplete"), 0);
    let msg = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "GladIAteur").unwrap();
    assert!(msg["data"]["message"]["innerThought"].is_null());
}

/// S10 — open loops: g1's question waits in g2's prompts (intention then intervention),
/// g2 closes it by declaring `answers: 1`, g2's concession becomes a commitment that
/// expires after TTL own interventions; the memory analyst's open questions also land.
#[tokio::test]
async fn open_loops_follow_questions_answers_and_ttl() {
    let provider = MockLlmProvider::scripted(intention_script);
    let (events, _, calls) = run_engine(provider, config(4), None).await;
    assert_eq!(count(&events, "error"), 0);
    let g2_intentions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g2")).collect();
    let g2_interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g2")).collect();
    assert_eq!(g2_intentions.len(), 4);
    // Turn 1: g1 spoke first with nobody to address (its question has no addressee) → no loop yet
    assert!(!g2_intentions[0].user.contains("[Fils ouverts"), "{}", g2_intentions[0].user);
    assert!(!g2_interventions[0].user.contains("[Fils ouverts"), "{}", g2_interventions[0].user);
    // g2 conceded on turn 1 → turn 2 shows the commitment, and g1's question asked on turn 2
    // (now aimed at g2) is loop 1 of g2, in both prompts
    let turn2 = &g2_intentions[1].user;
    assert!(turn2.contains("Ton engagement (tour 1) : « le rythme compte »"), "{turn2}");
    assert!(turn2.contains("Question de Le Scientifique (tour 2) : « Et le coût social ? »"), "{turn2}");
    assert!(g2_interventions[1].user.contains("[Fils ouverts — on attend une réponse de toi]\n1."), "{}", g2_interventions[1].user);
    // The commitment is deduplicated (same text each turn) and expires after TTL own interventions
    let commitments = |u: &str| u.matches("Ton engagement").count();
    assert_eq!(commitments(turn2), 1);
    // g1 never receives a question: the default memory script asks none and g2 asks none
    let g1_intentions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g1")).collect();
    assert!(g1_intentions.iter().all(|c| !c.user.contains("[Fils ouverts")), "g1 owes nothing");
    // Memory analyst questions open loops too (script: one question to Le Philosophe each turn)
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Memory {
            return Ok(LlmResponse { content: r#"{"summary":"s","positions":{},"open_questions":[{"to":"Philosophe","from":"Modérateur","question":"Et demain ?"}]}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls) = run_engine(provider, config(2), None).await;
    assert_eq!(count(&events, "error"), 0);
    let g2_turn2 = calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g2")).nth(1).unwrap();
    assert!(g2_turn2.user.contains("Question de Le Modérateur (tour 1) : « Et demain ? »"), "{}", g2_turn2.user);
}

/// S10 bis — a question asked to a banned speaker waits for their return.
#[tokio::test]
async fn open_loops_survive_a_ban() {
    // g1 questions g2 every turn; the moderator bans g2 for 1 turn after turn 1
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Moderation && req.user.contains("Le Philosophe") && req.user.contains("Intervention de g2") && !req.user.contains("[Ton intention]") {
            return Ok(LlmResponse { content: r#"{"action":"ban","comment":"Trop long.","ban_reason":"hors sujet","ban_duration":1}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        intention_script(req)
    });
    let (events, _, calls) = run_engine(provider, config(3), None).await;
    assert!(count(&events, "banIssued") >= 1, "{:?}", event_types(&events));
    let g2_intentions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g2")).collect();
    // g2 spoke on turn 1 (then banned) and again once back: the question asked while banned is still there
    assert!(g2_intentions.len() >= 2, "{:?}", event_types(&events));
    let back = g2_intentions.last().unwrap();
    assert!(back.user.contains("Question de Le Scientifique"), "{}", back.user);
}

/// S11 — positions: a mixed memory answer (string + object) yields one coherent map,
/// the trajectory is kept turn after turn, shown in the prompts and in the synthesis.
#[tokio::test]
async fn positions_keep_their_trajectory() {
    let counter = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&counter);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Memory {
            let n = c.fetch_add(1, Ordering::SeqCst);
            let content = if n == 0 {
                r#"{"summary":"s1","positions":{"Le Scientifique":"prudent","Le Philosophe":{"stance":"critique","shift":null,"would_change_if":"des preuves"}}}"#
            } else {
                r#"{"summary":"s2","positions":{"Le Scientifique":{"stance":"ouvert","shift":"s'est ouvert"},"Le Philosophe":"critique"}}"#
            };
            return Ok(LlmResponse { content: content.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    // A roomier context: the positions block must show the whole trajectory line
    let mut cfg = config(3);
    for g in &mut cfg.gladiateurs {
        g.llm_params.num_ctx = 16_384;
    }
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    let updates: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "positionsUpdated").collect();
    assert_eq!(updates.len(), 3);
    let first = &updates[0]["data"]["positions"];
    assert_eq!(first[0]["participantName"], "Le Philosophe");
    assert_eq!(first[0]["wouldChangeIf"], "des preuves");
    assert_eq!(first[1]["stance"], "prudent");
    assert_eq!(first[1]["initialStance"], "prudent");
    let last = &updates[2]["data"]["positions"];
    assert_eq!(last[1]["stance"], "ouvert");
    assert_eq!(last[1]["initialStance"], "prudent");
    assert_eq!(last[1]["shift"], "s'est ouvert");
    // Turn 3 intervention prompts show the evolution; the synthesis gets the block
    let turn3 = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).nth(4).unwrap();
    assert!(turn3.user.contains("- Le Scientifique : ouvert (a évolué : s'est ouvert)"), "{}", turn3.user);
    let synthesis = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synthesis.user.contains("[Évolution des positions]\n- Le Scientifique : prudent → ouvert (s'est ouvert)"), "{}", synthesis.user);
    // The memory prompt of turn 2 hands the trajectory back to the analyst
    let memory2 = calls.iter().filter(|c| c.call_kind == CallKind::Memory).nth(1).unwrap();
    assert!(memory2.user.contains(r#""would_change_if":"des preuves""#), "{}", memory2.user);
}

/// S11 bis — a name the analyst invented never enters the positions map; fuzzy
/// names are normalised to the participant's display name; fiction asks no target.
#[tokio::test]
async fn positions_keep_known_participants_only_and_fiction_has_no_targets() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Memory {
            return Ok(LlmResponse { content: r#"{"summary":"s","positions":{"Scientifique":"prudent","Le Fantôme":"absent","Léo":"curieux"}}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, config(1), None).await;
    let update = events.iter().find(|e| e["type"] == "positionsUpdated").unwrap();
    let names: Vec<&str> = update["data"]["positions"].as_array().unwrap().iter().map(|p| p["participantName"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Le Scientifique", "Léo"], "{update}");

    let provider = MockLlmProvider::scripted(default_script);
    let mut cfg = config(1);
    cfg.discussion_mode = DiscussionMode::CollaborativeFiction;
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    let intention = calls.iter().find(|c| c.call_kind == CallKind::Intention).unwrap();
    assert!(!intention.user.contains("[Participants que tu peux viser]"), "{}", intention.user);
    // The script still names a co-author → unknown in fiction → the topic
    let generated = events.iter().find(|e| e["type"] == "intentionGenerated").unwrap();
    assert!(generated["data"]["target"].is_null(), "{generated}");
}

/// S12 — fast pace: `Auto` is capped at `Low`, explicit levels are untouched, every
/// content request carries the pace (the provider scales its allowance from it).
#[tokio::test]
async fn fast_pace_caps_auto_and_travels_with_requests() {
    let caps = |reasoning_levels: bool| LlmCapabilities {
        supports_reasoning: true,
        reasoning_levels,
        reasoning_displayable: true,
        context_tokens: 8192,
        chars_per_token_latin: 3.8,
        chars_per_token_cjk: 1.5,
        reports_usage: true,
        billable: true,
        max_parallel_calls: 1,
        supports_json_mode: true,
    };
    // Explicit High + Fast: level kept, pace on every content request
    let provider = MockLlmProvider::scripted(default_script).with_capabilities(caps(true));
    let mut cfg = config(1);
    for g in &mut cfg.gladiateurs {
        g.llm_params.reasoning_level = Some(ReasoningLevel::High);
    }
    let (events, _, calls, _) = run_engine_with(provider, cfg, RunOptions { pace: Some(ReasoningPace::Fast), ..Default::default() }).await;
    assert_eq!(count(&events, "error"), 0);
    for kind in [CallKind::Introduction, CallKind::Intervention, CallKind::Synthesis] {
        let c = calls.iter().find(|c| c.call_kind == kind).unwrap();
        assert_eq!(c.pace, ReasoningPace::Fast, "{kind:?}");
    }
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intervention).all(|c| c.reasoning == ReasoningLevel::High));
    // Auto + Fast + a strong trigger (near the end): the heuristic would say High → capped at Low
    let provider = MockLlmProvider::scripted(default_script).with_capabilities(caps(true));
    let (_, _, calls, _) = run_engine_with(provider, config(2), RunOptions { reasoning: Some(ReasoningLevel::Auto), pace: Some(ReasoningPace::Fast), ..Default::default() }).await;
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intervention).all(|c| c.reasoning != ReasoningLevel::High), "Auto never reaches High under Fast");
    // Pure cap logic (deterministic, independent of the random gate)
    assert_eq!(ReasoningPace::Fast.cap_auto(ReasoningLevel::High), ReasoningLevel::Low);
    assert_eq!(ReasoningPace::Fast.cap_auto(ReasoningLevel::Off), ReasoningLevel::Off);
    assert_eq!(ReasoningPace::Normal.cap_auto(ReasoningLevel::High), ReasoningLevel::High);
}

// ── Dramaturgie (Lot 8) ─────────────────────────────────────────────────

/// Debate config with three gladiateurs and every staging feature on.
fn staged_config(max_turns: u32) -> DiscussionConfig {
    let mut cfg = immediate(config(max_turns));
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    cfg
}

/// A discussion that never stagnates: reactions every round, a summary whose words change every turn.
fn lively_script(counter: Arc<AtomicU32>) -> impl Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync + 'static {
    const SUMMARIES: [&str; 5] = [
        "Ouverture prudente sur les données du terrain.",
        "Bascule vers la formation continue et ses coûts cachés.",
        "Contre-interrogatoire serré autour des métriques de productivité.",
        "Concessions mutuelles : transformation plutôt que remplacement.",
        "Plaidoiries finales, positions clarifiées de part et d'autre.",
    ];
    let react = reactive_script("like");
    move |req: &LlmRequest| {
        if req.call_kind == CallKind::Memory {
            let n = counter.fetch_add(1, Ordering::SeqCst) as usize;
            let content = format!(r#"{{"summary":"{}","positions":{{"Le Scientifique":"prudent","Le Philosophe":"critique"}}}}"#, SUMMARIES[n % SUMMARIES.len()]);
            return Ok(LlmResponse { content, reasoning: None, usage: usage(60, 20), truncated: false });
        }
        react(req)
    }
}

/// S13 — five turns walk the debate script; the announcements reach the speakers as moderator lines.
#[tokio::test]
async fn acts_follow_the_debate_script_and_reach_the_prompts() {
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(lively_script(Arc::new(AtomicU32::new(0)))), immediate(config(5)), None).await;
    assert_eq!(count(&events, "error"), 0);
    let acts: Vec<&str> = events.iter().filter(|e| e["type"] == "actStarted").map(|e| e["data"]["act"].as_str().unwrap()).collect();
    assert_eq!(acts, vec!["opening", "confrontation", "crossExamination", "concessions", "closingStatements"]);
    let titles: Vec<&str> = events.iter().filter(|e| e["type"] == "actStarted").map(|e| e["data"]["title"].as_str().unwrap()).collect();
    assert_eq!(titles[0], "Ouverture");
    // One announcement per act, as a moderator line of kind actAnnouncement
    let announcements: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "actAnnouncement").collect();
    assert_eq!(announcements.len(), 5);
    assert_eq!(announcements[0]["data"]["message"]["role"], "IArbitre");
    assert!(announcements[0]["data"]["message"]["content"].as_str().unwrap().starts_with("Ouverture :"));
    // The speakers see the announcement in the moderator block and the act in their staging block
    let first = calls.iter().find(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(first.user.contains("DIRECTIVE DU MODÉRATEUR") && first.user.contains("Ouverture : chacun pose sa position"), "{}", first.user);
    assert!(first.user.contains("[Mise en scène de ce tour]\nActe — ouverture"), "{}", first.user);
    let last = calls.iter().rfind(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(last.user.contains("Acte — plaidoirie"), "{}", last.user);
    assert!(!last.user.contains("C'est le DERNIER tour"), "the closing act replaces the generic reminder");
    // The moderator gets the act's hint
    let moderation = calls.iter().find(|c| c.call_kind == CallKind::Moderation).unwrap();
    assert!(moderation.user.contains("Acte en cours : ouverture"), "{}", moderation.user);
    // The announcement lines are persisted as messages of the right kind (never as normal interventions)
    assert!(announcements.iter().all(|a| a["data"]["message"]["kind"] == "actAnnouncement"));
}

/// Step mode (v1.20.1): with the mode on, the engine announces `AwaitingCue` right
/// before each speaker and waits for the audience's `NextSpeaker`; cues sent ahead
/// free the speakers without a wait; leaving the mode releases a pending wait.
#[tokio::test]
async fn step_mode_waits_for_the_audience_cue_before_each_speaker() {
    async fn run_with(pre: Vec<EngineCommand>, react: impl Fn(usize) -> Option<EngineCommand> + Send + 'static) -> Vec<serde_json::Value> {
        let (tx, rx) = mpsc::channel::<EngineCommand>(8);
        for cmd in pre {
            tx.try_send(cmd).unwrap();
        }
        let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        let provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider::scripted(default_script));
        let mut engine = DiscussionEngine::new(config(1), "disc-step".to_string(), provider, None, db, None, token_budget::default_priorities());
        engine.disable_random_staging();
        let (channel, sink) = event_channel();
        // The audience reacts to each wait announcement (polled: the sink is a plain vector)
        let watched = Arc::clone(&sink);
        tokio::spawn(async move {
            let mut answered = 0;
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                let waits = watched.lock().unwrap().iter().filter(|e| e["type"] == "awaitingCue").count();
                while answered < waits {
                    answered += 1;
                    if let Some(cmd) = react(answered) {
                        if tx.send(cmd).await.is_err() {
                            return;
                        }
                    }
                }
            }
        });
        engine.run(rx, channel).await;
        let events = sink.lock().unwrap().clone();
        events
    }
    // 1. Every wait is answered by a cue: two waits (two speakers), each right before the speaker starts
    let events = run_with(vec![EngineCommand::SetStepMode { enabled: true }], |_| Some(EngineCommand::NextSpeaker)).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "awaitingCue"), 2, "{types:?}");
    for (i, t) in types.iter().enumerate() {
        if t == "awaitingCue" {
            assert_eq!(types[i + 1], "speakerActive", "{types:?}");
            assert_eq!(events[i]["data"]["speakerId"], events[i + 1]["data"]["speakerId"]);
        }
    }
    assert_eq!(events.iter().find(|e| e["type"] == "awaitingCue").unwrap()["data"]["speakerName"], "Le Scientifique");
    assert_eq!(count(&events, "discussionEnded"), 1);
    // 2. Cues sent ahead: no wait at all
    let events = run_with(vec![EngineCommand::SetStepMode { enabled: true }, EngineCommand::NextSpeaker, EngineCommand::NextSpeaker], |_| None).await;
    assert_eq!(count(&events, "awaitingCue"), 0);
    assert_eq!(count(&events, "discussionEnded"), 1);
    // 3. Leaving the mode on the first wait releases it, and no further wait happens
    let events = run_with(vec![EngineCommand::SetStepMode { enabled: true }], |_| Some(EngineCommand::SetStepMode { enabled: false })).await;
    assert_eq!(count(&events, "awaitingCue"), 1);
    assert_eq!(count(&events, "discussionEnded"), 1);
    // 4. Mode off (default): nothing changes
    let events = run_with(vec![EngineCommand::NextSpeaker], |_| None).await;
    assert_eq!(count(&events, "awaitingCue"), 0);
}

/// v1.20.1 — depth: a counter-argument extracted against g1's thesis opens an
/// objection loop on g1 (its next intention and intervention prompts), the
/// moderator is briefed on the owed objection, and g2 owes nothing.
#[tokio::test]
async fn objections_from_the_argument_map_open_loops_and_brief_the_moderator() {
    let extracted = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&extracted);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::ArgumentMap && c.fetch_add(1, Ordering::SeqCst) == 0 {
            let content = r#"{"extractions":[
                {"speaker":"Le Scientifique","new_theses":["L'IA remplacera les développeurs"],"arguments":[]},
                {"speaker":"Le Philosophe","new_theses":[],"arguments":[{"text":"Les outils déplacent le travail, ils ne le suppriment pas","type":"counter","for_thesis":null,"against_thesis":"L'IA remplacera les développeurs","targets_argument":null}]}
            ]}"#;
            return Ok(LlmResponse { content: content.to_string(), reasoning: None, usage: usage(80, 40), truncated: false });
        }
        default_script(req)
    });
    let mut cfg = config(3);
    cfg.argument_map_enabled = true;
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    assert!(count(&events, "argumentMapUpdated") >= 1, "{:?}", event_types(&events));
    let g1_last_intention = calls.iter().rfind(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g1")).unwrap();
    assert!(g1_last_intention.user.contains("Objection de Le Philosophe (tour") && g1_last_intention.user.contains("« Les outils déplacent le travail, ils ne le suppriment pas » — réponds sur le fond ou concède"), "{}", g1_last_intention.user);
    let g1_last = calls.iter().rfind(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).unwrap();
    assert!(g1_last.user.contains("[Fils ouverts — on attend une réponse de toi]") && g1_last.user.contains("Objection de Le Philosophe"), "{}", g1_last.user);
    // The moderator of g1's last intervention is briefed on the owed objection
    let moderation = calls.iter().rfind(|c| c.call_kind == CallKind::Moderation && c.user.starts_with("Tu viens d'entendre l'intervention suivante de Le Scientifique")).unwrap();
    assert!(moderation.user.contains("Profondeur : une objection reste sans réponse — « Les outils déplacent le travail, ils ne le suppriment pas (Le Philosophe) »"), "{}", moderation.user);
    // g2 owes nothing
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g2")).all(|c| !c.user.contains("Objection de")));
    // The next extraction is told about the open objection, so an answer nests under it
    let next_extraction = calls.iter().filter(|c| c.call_kind == CallKind::ArgumentMap).nth(1).expect("a second extraction");
    assert!(next_extraction.user.contains("Objections encore sans réponse") && next_extraction.user.contains("- « Les outils déplacent le travail, ils ne le suppriment pas » (objection de Le Philosophe à Le Scientifique)"), "{}", next_extraction.user);
    // v1.20.3 — every speaker of turn 2 sees the state of the debate: the thesis, its owner, the open objection
    assert!(g1_last.user.contains("[État du débat]\n- L'IA remplacera les développeurs (Le Scientifique) : 1 argument(s), 1 objection(s) ouverte(s)\nObjections sans réponse :\n- « Les outils déplacent le travail, ils ne le suppriment pas » (Le Philosophe à Le Scientifique)\nSache où en est le débat"), "{}", g1_last.user);
    let first_turn = calls.iter().find(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(!first_turn.user.contains("[État du débat]"), "nothing mapped yet on turn 1");
}

/// v1.20.3 — who is who: each speaker's system prompt portrays the others (role,
/// creed, register read from the kernel; the name alone for a prose persona), the
/// moderator's in-character calls carry its whole cast.
#[tokio::test]
async fn speakers_and_the_moderator_know_who_the_others_are() {
    let mut cfg = config(1);
    cfg.gladiateurs[0].system_prompt = "<system_kernel>\n<identity>\nLe Scientifique — Chercheur en sciences expérimentales\n\"Sans données reproductibles, vous n'avez qu'une anecdote.\"\nDocteur en sciences.\n</identity>\n<voice>\nRegistre: SOUTENU, TECHNIQUE\n</voice>\n</system_kernel>".to_string();
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    let g2 = calls.iter().find(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g2")).unwrap();
    assert!(g2.system.contains("[Les autres participants — qui ils sont]\n- Le Scientifique : Chercheur en sciences expérimentales ; « Sans données reproductibles, vous n'avez qu'une anecdote. » ; registre soutenu, technique\nParle-leur comme aux personnes qu'ils sont"), "{}", g2.system);
    assert!(!g2.system.contains("- Le Philosophe"), "never a portrait of oneself: {}", g2.system);
    let g1 = calls.iter().find(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).unwrap();
    assert!(g1.system.contains("[Les autres participants — qui ils sont]\n- Le Philosophe\n"), "a prose persona is portrayed by its name: {}", g1.system);
    // The intention call shares the same system prompt (memories, cast, role)
    let g1_intention = calls.iter().find(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g1")).unwrap();
    assert!(g1_intention.system.contains("[Les autres participants — qui ils sont]"));
    // The moderator knows its cast in its in-character calls (introduction, moderation, synthesis, announcements)
    for kind in [CallKind::Introduction, CallKind::Moderation, CallKind::Synthesis, CallKind::Announcement] {
        let c = calls.iter().find(|c| c.call_kind == kind).unwrap_or_else(|| panic!("{kind:?}"));
        assert!(c.system.starts_with("<persona>moderateur</persona>\n\n[Ton plateau — qui ils sont]\n- Le Scientifique : Chercheur") && c.system.contains("- Le Philosophe\nAdresse-toi à chacun pour ce qu'il est"), "{kind:?}: {}", c.system);
    }
    // Utility calls of the moderator (memory, emotions) keep their own system prompts
    let memory = calls.iter().find(|c| c.call_kind == CallKind::Memory).unwrap();
    assert!(!memory.system.contains("[Ton plateau"));
}

/// v1.20.2 — the audience member who speaks is a participant: the next speaker
/// gets them as forced focus and the "answer them first" directive, the intention
/// prompt lists them in priority, the moderator is told when the answer ignores
/// them, their claims reach the argument map under the `user` id, and the
/// moderator's own openings are quoted back with the comment rate.
#[tokio::test]
async fn audience_message_is_answered_first_and_reaches_every_representation() {
    let (tx, rx) = mpsc::channel::<EngineCommand>(8);
    tx.try_send(EngineCommand::UserWantsToIntervene).unwrap();
    let provider = Arc::new(MockLlmProvider::scripted(|req| {
        let content = match req.call_kind {
            // The moderator comments every time, always the same way
            CallKind::Moderation => r#"{"action":"comment","comment":"MAIS ATTENDEZ ! Coup de théâtre, chers téléspectateurs !","ban_reason":"","ban_duration":0}"#.to_string(),
            CallKind::ArgumentMap if req.user.contains("Léo") => r#"{"extractions":[{"speaker":"Léo","new_theses":["L'IA c'est bien"],"arguments":[]}]}"#.to_string(),
            _ => return default_script(req),
        };
        Ok(LlmResponse { content, reasoning: None, usage: usage(60, 10), truncated: false })
    }));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let mut cfg = immediate(config(2));
    cfg.argument_map_enabled = true;
    let mut engine = DiscussionEngine::new(cfg, "disc-audience".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.disable_random_staging();
    engine.set_argument_map_enabled(true);
    engine.set_emotion_driven(true); // the dynamic directive carries the audience reminder
    let (channel, sink) = event_channel();
    // The audience answers the user-turn window with a message
    let watched = Arc::clone(&sink);
    tokio::spawn(async move {
        let mut sent = false;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let ready = watched.lock().unwrap().iter().any(|e| e["type"] == "userTurnReady");
            if ready && !sent {
                sent = true;
                if tx.send(EngineCommand::SubmitUserMessage { content: "salut moi c'est Léo. Je pense que l'IA c'est bien".to_string() }).await.is_err() {
                    return;
                }
            }
        }
    });
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    let calls = provider.recorded_calls();
    assert_eq!(count(&events, "error"), 0, "{:?}", event_types(&events));
    // The pending request is served at the start of turn 1: the first speaker (g1) answers Léo first
    let user_msg = events.iter().position(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "user").expect("user message");
    let directive = events.iter().skip(user_msg).find(|e| e["type"] == "directiveGenerated").expect("a directive after the user's message");
    assert_eq!(directive["data"]["speakerId"], "g1");
    assert_eq!(directive["data"]["focusSpeaker"], "Léo", "{directive}");
    let g1_calls: Vec<&LlmRequest> = calls.iter().filter(|c| c.speaker_id.as_deref() == Some("g1")).collect();
    let intention = g1_calls.iter().find(|c| c.call_kind == CallKind::Intention).unwrap();
    assert!(intention.user.contains("[Participants que tu peux viser] Le Philosophe, Léo. Priorité : adresse-toi à Léo."), "{}", intention.user);
    let intervention = g1_calls.iter().find(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(intervention.user.contains("Léo, dans le public, vient d'intervenir : « salut moi c'est Léo. Je pense que l'IA c'est bien ». Il fait partie du débat désormais"), "{}", intervention.user);
    assert!(!intervention.user.contains("Ne t'adresse PAS à Léo"), "{}", intervention.user);
    // g1's canned answer never names Léo: the moderator is told, once (the debt is settled after that speaker)
    let moderations: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Moderation).collect();
    assert!(moderations[0].user.contains("intervention suivante de Le Scientifique") && moderations[0].user.contains("Léo, dans le public, a pris la parole juste avant") && moderations[0].user.contains("Cette intervention l'ignore"), "{}", moderations[0].user);
    assert!(!moderations[1].user.contains("a pris la parole juste avant"), "{}", moderations[1].user);
    // Later speakers: the audience member is a participant, no longer an observer; g2 may draw them as focus
    let g2_first = calls.iter().find(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g2")).unwrap();
    assert!(g2_first.user.contains("Léo, dans le public, a pris part au débat"), "{}", g2_first.user);
    assert!(!g2_first.user.contains("Ne t'adresse PAS"), "{}", g2_first.user);
    // The argument map knows the audience member under the `user` id
    let map = events.iter().rfind(|e| e["type"] == "argumentMapUpdated").expect("a map");
    let thesis = map["data"]["map"]["theses"].as_array().unwrap().iter().find(|t| t["speakerName"] == "Léo").expect("Léo's thesis");
    assert_eq!(thesis["speakerId"], "user");
    // The moderator's own openings are quoted back; past the rate, it is told to hold back
    let last_moderation = moderations.last().unwrap();
    assert!(last_moderation.user.contains("« MAIS ATTENDEZ ! », « MAIS ATTENDEZ ! »") && last_moderation.user.contains("n'ouvre jamais deux commentaires de la même façon"), "{}", last_moderation.user);
    assert!(last_moderation.user.contains("Tu es déjà intervenu sur 3 des 3 dernières interventions"), "{}", last_moderation.user);
    // The introduction and the (voiced) opening-act announcement are the first openings remembered
    assert!(moderations[0].user.contains("commençaient par « Bienvenue dans ce débat. », « Ouverture :") && !moderations[0].user.contains("Tu es déjà intervenu"), "{}", moderations[0].user);
}

/// S14 — no turn limit, soft stop during turn 3: the remaining speaker of the turn
/// delivers a closing statement (the act switches at once), the end of turn still
/// runs, then the synthesis.
#[tokio::test]
async fn soft_stop_without_limit_brings_the_closing_act() {
    let mut cfg = immediate(config(3));
    cfg.max_turns = None;
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let (tx, rx) = mpsc::channel::<EngineCommand>(8);
    let counter = Arc::new(AtomicU32::new(0));
    let provider = Arc::new(MockLlmProvider::scripted(move |req| {
        // The stop is requested while g1 speaks on turn 3 (its 3rd intervention)
        if req.call_kind == CallKind::Intervention && req.speaker_id.as_deref() == Some("g1") && counter.fetch_add(1, Ordering::SeqCst) == 2 {
            let _ = tx.try_send(EngineCommand::Stop);
        }
        lively_script(Arc::new(AtomicU32::new(0)))(req)
    }));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut engine = DiscussionEngine::new(cfg, "disc-stop".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    let (channel, sink) = event_channel();
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    let calls = provider.recorded_calls();
    let acts: Vec<(u64, &str)> = events.iter().filter(|e| e["type"] == "actStarted").map(|e| (e["data"]["turn"].as_u64().unwrap(), e["data"]["act"].as_str().unwrap())).collect();
    assert_eq!(acts.first().copied(), Some((1, "opening")));
    assert_eq!(acts.last().copied(), Some((3, "closingStatements")), "{acts:?}");
    assert_eq!(count(&events, "turnStarted"), 3);
    // g2, speaking after the stop, got the closing instruction; the end of turn ran; one end
    let last = calls.iter().rfind(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert_eq!(last.speaker_id.as_deref(), Some("g2"));
    assert!(last.user.contains("Acte — plaidoirie"), "{}", last.user);
    assert_eq!(count(&events, "turnTimings"), 3);
    assert_eq!(count(&events, "discussionEnded"), 1);
}

/// S15 — stagnation pulls the concessions act forward.
#[tokio::test]
async fn stagnation_brings_concessions_early() {
    // The analyst flags stagnation from the first turn on
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Emotion {
            return Ok(LlmResponse { content: r#"{"stagnating":true}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, config(8), None).await;
    let acts: Vec<&str> = events.iter().filter(|e| e["type"] == "actStarted").map(|e| e["data"]["act"].as_str().unwrap()).collect();
    assert_eq!(acts, vec!["opening", "concessions", "closingStatements"], "confrontation skipped");
}

/// S16 — a duel with four active speakers: only the two duellists speak, the end of turn still runs.
#[tokio::test]
async fn duel_shortens_the_turn_to_two_speakers() {
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let mut cfg = staged_config(3);
    cfg.gladiateurs.push(glad("g4", "L'Économiste", 4));
    let provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider::scripted(default_script));
    let mut engine = DiscussionEngine::new(cfg, "disc-duel".to_string(), provider, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    engine.force_scene_event(SceneEventKind::Duel);
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    assert_eq!(count(&events, "error"), 0);
    let turns: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "turnStarted").collect();
    assert_eq!(turns[0]["data"]["speakerOrder"].as_array().unwrap().len(), 2, "the forced duel lands on turn 1: {:?}", turns[0]);
    assert_eq!(turns[1]["data"]["speakerOrder"].as_array().unwrap().len(), 4);
    let scene = events.iter().find(|e| e["type"] == "sceneEventTriggered").unwrap();
    assert_eq!(scene["data"]["event"]["kind"], "duel");
    assert_eq!(scene["data"]["participants"].as_array().unwrap().len(), 2);
    let line = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "sceneEvent").unwrap();
    assert!(line["data"]["message"]["content"].as_str().unwrap().starts_with("Duel :"));
    // End of turn still ran for turn 1 (memory positions emitted three times)
    assert_eq!(count(&events, "positionsUpdated"), 3);
    assert_eq!(count(&events, "turnTimings"), 3);
}

/// S17 — a surprise fact without any knowledge source becomes a format constraint.
#[tokio::test]
async fn surprise_fact_without_search_becomes_a_constraint() {
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider::scripted(default_script));
    let mut engine = DiscussionEngine::new(staged_config(2), "disc-fact".to_string(), provider, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    engine.force_scene_event(SceneEventKind::SurpriseFact);
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    assert_eq!(count(&events, "error"), 0);
    let scene = events.iter().find(|e| e["type"] == "sceneEventTriggered").unwrap();
    assert_eq!(scene["data"]["event"]["kind"], "formatConstraint", "{scene}");
    assert!(scene["data"]["event"]["constraint"].is_string());
    assert_eq!(count(&events, "webSearchPerformed") + count(&events, "wikiSearchPerformed"), 0);
}

/// Hot seat and the room's question: the target speaks last / gets the specific line; the others address them.
#[tokio::test]
async fn hot_seat_reorders_and_instructs() {
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider = Arc::new(MockLlmProvider::scripted(default_script));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut engine = DiscussionEngine::new(staged_config(1), "disc-hot".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    engine.force_scene_event(SceneEventKind::HotSeat);
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    let calls = provider.recorded_calls();
    let scene = events.iter().find(|e| e["type"] == "sceneEventTriggered").unwrap();
    let target = scene["data"]["event"]["target"].as_str().unwrap().to_string();
    let order = events.iter().find(|e| e["type"] == "turnStarted").unwrap()["data"]["speakerOrder"].clone();
    let last_id = order.as_array().unwrap().last().unwrap().as_str().unwrap();
    let last_name = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["speakerId"] == last_id).unwrap()["data"]["message"]["speakerName"].as_str().unwrap();
    assert_eq!(last_name, target, "the target speaks last");
    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert!(interventions.last().unwrap().user.contains("tu es sur la sellette"), "{}", interventions.last().unwrap().user);
    assert!(interventions[0].user.contains(&format!("adresse-toi directement à {target}")), "{}", interventions[0].user);
    assert!(interventions.iter().all(|c| c.user.contains("[Mise en scène de ce tour]")));
}

/// v1.20.3 — the room's question is written by the moderator from the exchanges
/// (grounded prompt), voiced in its own words, and reaches the target's staging
/// block; a moderator that cannot write one produces no event at all; the voiced
/// announcement falls back to the template when the call fails.
#[tokio::test]
async fn audience_question_is_written_from_the_debate_and_announcements_are_voiced() {
    async fn run(script: impl Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync + 'static) -> (Vec<serde_json::Value>, Vec<LlmRequest>) {
        let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        let provider = Arc::new(MockLlmProvider::scripted(script));
        let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
        let mut engine = DiscussionEngine::new(staged_config(1), "disc-room".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
        engine.set_emotion_driven(false);
        engine.force_scene_event(SceneEventKind::AudienceQuestion);
        let (channel, sink) = event_channel();
        let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
        engine.run(rx, channel).await;
        let events = sink.lock().unwrap().clone();
        (events, provider.recorded_calls())
    }
    // 1. The moderator voices every announcement in its own words; the room's question is grounded and reaches the target
    let (events, calls) = run(|req| match req.call_kind {
        CallKind::Announcement => Ok(LlmResponse { content: "\"Mes amis, la salle s'impatiente : à vous de jouer.\"".to_string(), reasoning: None, usage: usage(30, 10), truncated: false }),
        _ => default_script(req),
    }).await;
    assert_eq!(count(&events, "error"), 0, "{:?}", event_types(&events));
    let scene = events.iter().find(|e| e["type"] == "sceneEventTriggered").expect("the forced event");
    assert_eq!(scene["data"]["event"]["kind"], "audienceQuestion");
    assert_eq!(scene["data"]["event"]["question"], "Sur quelles données fondez-vous ce chiffre de transformation ?");
    let target = scene["data"]["event"]["target"].as_str().unwrap().to_string();
    // The question call is grounded: topic, summary line, the target's name and what it owes
    let q = calls.iter().find(|c| c.call_kind == CallKind::AudienceQuestion).unwrap();
    assert!(q.json_mode && q.user.contains("Sujet : L'IA va-t-elle remplacer les développeurs ?") && q.user.contains(&format!("Position de {target} jusqu'ici")) && q.user.contains("Jamais générique"), "{}", q.user);
    assert!(q.system.contains("la voix du public"), "{}", q.system);
    // Every announcement line (act, scene) is the moderator's voiced text, quotation marks stripped
    let lines: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "messageComplete" && (e["data"]["message"]["kind"] == "sceneEvent" || e["data"]["message"]["kind"] == "actAnnouncement")).collect();
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|l| l["data"]["message"]["content"] == "Mes amis, la salle s'impatiente : à vous de jouer."), "{lines:?}");
    // The announcement call is briefed with the template and the moderator's recent openings
    let a = calls.iter().filter(|c| c.call_kind == CallKind::Announcement).nth(1).unwrap();
    assert!(a.user.contains(&format!("Une question de la salle pour {target} : « Sur quelles données")) && a.user.contains("commençaient par « Bienvenue dans ce débat. », « Mes amis, la salle s'impatiente : à vous de jouer. »"), "{}", a.user);
    assert!(a.call_kind == CallKind::Announcement && !a.json_mode && a.params.num_predict == crate::constants::ANNOUNCEMENT_NUM_PREDICT);
    // The target's staging block carries the actual question; the others get nothing about it
    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    let target_id = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["speakerName"] == target).unwrap()["data"]["message"]["speakerId"].as_str().unwrap().to_string();
    let target_call = interventions.iter().find(|c| c.speaker_id.as_deref() == Some(&target_id)).unwrap();
    assert!(target_call.user.contains("la salle te demande — « Sur quelles données fondez-vous ce chiffre de transformation ? »"), "{}", target_call.user);
    assert!(interventions.iter().filter(|c| c.speaker_id.as_deref() != Some(&target_id)).all(|c| !c.user.contains("la salle te demande")));

    // 2. No usable question → no event; a failing announcement call → the templated line
    let (events, calls) = run(|req| match req.call_kind {
        CallKind::AudienceQuestion => Ok(LlmResponse { content: "je ne sais pas".to_string(), reasoning: None, usage: usage(30, 10), truncated: false }),
        CallKind::Announcement => Err(LlmError::Overloaded),
        _ => default_script(req),
    }).await;
    assert_eq!(count(&events, "sceneEventTriggered"), 0, "{:?}", event_types(&events));
    assert!(calls.iter().any(|c| c.call_kind == CallKind::AudienceQuestion));
    let act = events.iter().find(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "actAnnouncement").unwrap();
    let templated = act["data"]["message"]["content"].as_str().unwrap();
    assert!(templated.starts_with("Ouverture :") || templated.starts_with("Plaidoiries :"), "{act}");
    let diag = events.iter().find(|e| e["type"] == "diagnosticsReady").unwrap();
    assert_eq!(diag["data"]["diagnostics"]["jsonParseFailures"]["audienceQuestion"], 1);
}

/// S20 — an ally pair relays: the leader gets the Relay act, the follower speaks right after; never with two actives.
#[tokio::test]
async fn coalitions_relay_between_allies() {
    // g1 and g2 approve each other on every round → allies from turn 2 on
    let likes = |req: &LlmRequest| -> Result<LlmResponse, LlmError> {
        if req.call_kind == CallKind::Reaction {
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            let kind = if matches!(req.speaker_id.as_deref(), Some("g1") | Some("g2")) && (name == "Le Scientifique" || name == "Le Philosophe") { "like" } else { "none" };
            return Ok(LlmResponse { content: format!(r#"[{{"speaker":"{name}","reaction":"{kind}","justification":"j","quote":""}}]"#), reasoning: None, usage: usage(60, 15), truncated: false });
        }
        default_script(req)
    };
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider = Arc::new(MockLlmProvider::scripted(likes));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut cfg = staged_config(4);
    cfg.features.scene_events = false;
    let mut engine = DiscussionEngine::new(cfg, "disc-coalition".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(true);
    engine.force_coalitions();
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    let calls = provider.recorded_calls();
    assert_eq!(count(&events, "error"), 0);
    let coalitions: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "coalitionFormed").collect();
    assert!(!coalitions.is_empty(), "{:?}", event_types(&events));
    let c = coalitions[0];
    let (a, b) = (c["data"]["a"].as_str().unwrap(), c["data"]["b"].as_str().unwrap());
    let turn = c["data"]["turn"].as_u64().unwrap();
    let order = events.iter().find(|e| e["type"] == "turnStarted" && e["data"]["turnNumber"] == turn).unwrap()["data"]["speakerOrder"].clone();
    let ids: Vec<&str> = order.as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    let pos_a = ids.iter().position(|i| *i == a).unwrap();
    assert_eq!(ids.get(pos_a + 1).copied(), Some(b), "the follower speaks right after the leader: {ids:?}");
    let directive = events.iter().find(|e| e["type"] == "directiveGenerated" && e["data"]["speakerId"] == a && e["data"]["speechAct"] == "Relay");
    assert!(directive.is_some(), "leader gets the Relay act");
    let follower_prompt = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some(b)).any(|c| c.user.contains("vient de te passer le relais"));
    assert!(follower_prompt);

    // Two actives: never
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider: Arc<dyn LlmProvider> = Arc::new(MockLlmProvider::scripted(likes));
    let mut engine = DiscussionEngine::new(immediate(config(4)), "disc-two".to_string(), provider, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(true);
    engine.force_coalitions();
    let (channel, sink) = event_channel();
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    assert_eq!(count(&events, "coalitionFormed"), 0);
}

// ── Pipeline et mesures (Lot 7) ─────────────────────────────────────────

/// S35 — end of turn on a parallel provider: the four calls all start before the
/// first result is applied; the final state equals the sequential one (golden).
#[tokio::test]
async fn end_of_turn_calls_run_together_and_apply_in_order() {
    // Deterministic answers (no call counter): the parallel and sequential runs must land on the same state
    fn golden_script(req: &LlmRequest) -> Result<LlmResponse, LlmError> {
        let content = match req.call_kind {
            CallKind::Reaction => {
                let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
                let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
                format!(r#"[{{"speaker":"{name}","reaction":"like","justification":"net","quote":""}}]"#)
            }
            CallKind::Memory => realistic_memory_json(1),
            CallKind::Emotion => realistic_emotion_json(1),
            CallKind::TurnAnalyst => fused_analyst_json(&realistic_memory_json(1), &realistic_emotion_json(1)),
            CallKind::ArgumentMap => r#"{"extractions":[{"speaker":"Le Scientifique","new_theses":["L'automatisation transforme le métier"],"arguments":[{"text":"Chiffre à l'appui","type":"evidence","for_thesis":"L'automatisation transforme le métier"}]}]}"#.to_string(),
            CallKind::DocumentUpdate => "# Plan\n\n- Point consolidé\n".to_string(),
            _ => return default_script(req),
        };
        Ok(LlmResponse { content, reasoning: None, usage: usage(100, 20), truncated: false })
    }
    let realistic = || {
        let mut cfg = config(3);
        cfg.argument_map_enabled = true;
        cfg.document_format = DocumentFormat::Md;
        cfg.document_update_granularity = DocumentUpdateGranularity::Turn;
        cfg
    };
    let parallel = MockLlmProvider::scripted(golden_script).with_capabilities(billable_caps()).with_model_name("deepseek-flash");
    let (events, _, calls, _) = run_engine_with(parallel, realistic(), RunOptions { reasoning: Some(ReasoningLevel::Off), ..Default::default() }).await;
    assert_eq!(count(&events, "error"), 0, "{:?}", event_types(&events));
    // The four end-of-turn requests of turn 1 are consecutive in the mock's ledger:
    // document, emotion, memory, argument map — none waited for another's answer
    let kinds: Vec<CallKind> = calls.iter().map(|c| c.call_kind).collect();
    let first_doc = kinds.iter().position(|k| *k == CallKind::DocumentUpdate).unwrap();
    assert_eq!(&kinds[first_doc..first_doc + 4], &[CallKind::DocumentUpdate, CallKind::Emotion, CallKind::Memory, CallKind::ArgumentMap], "{kinds:?}");
    // Every phase applied: document, emotions, positions, map, timings
    assert_eq!(count(&events, "documentUpdated"), 3);
    assert_eq!(count(&events, "positionsUpdated"), 3);
    assert!(count(&events, "argumentMapUpdated") >= 1);
    let timings: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "turnTimings").collect();
    assert_eq!(timings.len(), 3);
    let names: Vec<&str> = timings[0]["data"]["timings"]["phases"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
    for expected in ["speakers", "document", "emotion", "memory", "argumentMap", "endOfTurn"] {
        assert!(names.contains(&expected), "{names:?} lacks {expected}");
    }
    // Golden: the sequential provider (fused analyst) ends in the same state
    let sequential = MockLlmProvider::scripted(golden_script).with_capabilities(sequential_caps());
    let (seq_events, _, seq_calls, _) = run_engine_with(sequential, realistic(), RunOptions { reasoning: Some(ReasoningLevel::Off), ..Default::default() }).await;
    assert_eq!(count(&seq_events, "error"), 0);
    assert!(seq_calls.iter().all(|c| !matches!(c.call_kind, CallKind::Memory | CallKind::Emotion)), "fused on a sequential provider");
    assert_eq!(seq_calls.iter().filter(|c| c.call_kind == CallKind::TurnAnalyst).count(), 3);
    let (p1, p2) = (last_emotions(&events, "g1"), last_emotions(&seq_events, "g1"));
    assert_eq!(p1, p2, "same final emotions");
    let last_positions = |ev: &[serde_json::Value]| ev.iter().rev().find(|e| e["type"] == "positionsUpdated").unwrap()["data"].clone();
    assert_eq!(last_positions(&events), last_positions(&seq_events), "same final positions");
    let doc = |ev: &[serde_json::Value]| ev.iter().rev().find(|e| e["type"] == "documentUpdated").unwrap()["data"]["content"].clone();
    assert_eq!(doc(&events), doc(&seq_events));
}

/// S36 — sequential provider: one `TurnAnalyst` call replaces memory + emotion;
/// a partial answer (no emotions) still updates the memory and leaves emotions alone.
#[tokio::test]
async fn fused_turn_analyst_tolerates_partial_answers() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::TurnAnalyst {
            return Ok(LlmResponse { content: r#"{"summary":"Résumé fusionné","positions":{"Le Scientifique":"prudent"}}"#.to_string(), reasoning: None, usage: usage(60, 20), truncated: false });
        }
        default_script(req)
    })
    .with_capabilities(sequential_caps());
    let (events, _, calls) = run_engine(provider, config(2), None).await;
    assert_eq!(count(&events, "error"), 0);
    let analyst: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::TurnAnalyst).collect();
    assert_eq!(analyst.len(), 2);
    assert!(analyst[0].json_mode);
    assert!(analyst[0].user.contains("=== PARTIE 1 — MÉMOIRE ===") && analyst[0].user.contains("=== PARTIE 2 — ÉMOTIONS ===") && analyst[0].user.contains("FUSION"), "{}", analyst[0].user);
    assert_eq!(calls.iter().filter(|c| matches!(c.call_kind, CallKind::Memory | CallKind::Emotion)).count(), 0);
    // Memory applied
    let positions = events.iter().rev().find(|e| e["type"] == "positionsUpdated").unwrap();
    assert_eq!(positions["data"]["positions"][0]["stance"], "prudent");
    let synthesis = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synthesis.user.contains("Résumé fusionné"));
    // No emotion deltas: the rule-based values stand, nothing counted as a failure
    let diagnostics = events.iter().find(|e| e["type"] == "diagnosticsReady").unwrap();
    assert!(diagnostics["data"]["diagnostics"]["jsonParseFailures"].as_object().unwrap().is_empty(), "{diagnostics}");
}

/// S37 — cancellation during the parallel end of turn: one `DiscussionEnded`, usage recorded.
#[tokio::test]
async fn cancellation_during_end_of_turn_ends_once_with_usage() {
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Memory {
            trigger.cancel();
            return Err(LlmError::Cancelled);
        }
        default_script(req)
    });
    let db_check = {
        let (events, ledger, calls, db) = run_engine_with(provider, config(3), RunOptions { cancel: Some(cancel), ..Default::default() }).await;
        assert_eq!(count(&events, "discussionEnded"), 1, "{:?}", event_types(&events));
        assert_eq!(count(&events, "diagnosticsReady"), 1);
        assert!(count(&events, "turnStarted") < 3, "stopped before the last turn");
        assert!(ledger.calls > 0 && ledger.calls as usize <= calls.len() + 1);
        db
    };
    drop(db_check);
}

/// S38 — timings and diagnostics are emitted; broken JSON answers are counted by call kind.
#[tokio::test]
async fn diagnostics_count_parse_failures_refusals_and_intentions() {
    // g1's interventions: turn 1 has no target (nobody spoke yet: not counted), turn 2
    // names the target, turn 3 does not → compliance 0.5
    let g1_spoken = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&g1_spoken);
    let provider = MockLlmProvider::scripted(move |req| {
        let content = match req.call_kind {
            CallKind::Memory => "pas du json".to_string(),
            CallKind::Moderation => "{broken".to_string(),
            CallKind::Intention if req.speaker_id.as_deref() == Some("g2") => "{\"target\": ".to_string(),
            CallKind::Intervention if req.speaker_id.as_deref() == Some("g2") => "Je ne peux pas répondre à cette demande.".to_string(),
            CallKind::Intervention => match counter.fetch_add(1, Ordering::SeqCst) {
                1 => "Le Philosophe, les données montrent une transformation.".to_string(),
                _ => "Les données montrent une transformation.".to_string(),
            },
            _ => return default_script(req),
        };
        Ok(LlmResponse { content, reasoning: None, usage: usage(60, 10), truncated: false })
    });
    let (events, _, calls) = run_engine(provider, config(3), None).await;
    let d = &events.iter().find(|e| e["type"] == "diagnosticsReady").unwrap()["data"]["diagnostics"];
    assert_eq!(d["jsonParseFailures"]["memory"], 3, "{d}");
    assert_eq!(d["jsonParseFailures"]["moderation"].as_u64().unwrap(), 3, "one per g1 intervention (g2 never speaks: refusal)");
    assert_eq!(d["jsonParseFailures"]["intention"], 3);
    assert!(d["refusals"].as_u64().unwrap() >= 3, "{d}");
    assert!(d["retries"].as_u64().unwrap() >= 3, "{d}");
    let g1: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).collect();
    assert_eq!(g1.len(), 3);
    assert_eq!(d["intentionCompliance"], 0.5, "{d}");
    // Diagnostics come right before the end
    let types = event_types(&events);
    assert_eq!(&types[types.len() - 2..], &["diagnosticsReady".to_string(), "discussionEnded".to_string()]);
    assert_eq!(count(&events, "turnTimings"), 3);
}

// ── Émotions incarnées (Lot 6) ──────────────────────────────────────────

/// Two-speaker config with OCEAN matrices in the system prompts (g1 neurotic, g2 stable).
fn ocean_config(max_turns: u32, g1_n: u8, g2_n: u8) -> DiscussionConfig {
    let mut cfg = config(max_turns);
    cfg.gladiateurs[0].system_prompt = format!("<persona>Le Scientifique</persona> O=5 C=5 E=5 A=5 N={g1_n}");
    cfg.gladiateurs[1].system_prompt = format!("<persona>Le Philosophe</persona> O=5 C=5 E=5 A=5 N={g2_n}");
    cfg
}

/// S22 — the same dislikes hurt a neurotic persona more than a stable one; neutral
/// matrices reproduce the v1.16 values exactly (golden against the plain config).
#[tokio::test]
async fn ocean_gains_shape_the_felt_reactions() {
    let dislikes = |req: &LlmRequest| -> Result<LlmResponse, LlmError> {
        if req.call_kind == CallKind::Reaction {
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            return Ok(LlmResponse { content: format!(r#"[{{"speaker":"{name}","reaction":"dislike","justification":"non","quote":""}}]"#), reasoning: None, usage: usage(60, 15), truncated: false });
        }
        default_script(req)
    };
    let (events, _, _) = run_engine(MockLlmProvider::scripted(dislikes), immediate(ocean_config(2, 9, 3)), None).await;
    assert_eq!(count(&events, "error"), 0);
    let nervous = last_emotions(&events, "g1")["frustration"].as_i64().unwrap();
    let stable = last_emotions(&events, "g2")["frustration"].as_i64().unwrap();
    assert!(nervous > stable, "N=9 {nervous} vs N=3 {stable}");
    // Golden: a neutral matrix and no matrix at all end up identical
    let (plain, _, _) = run_engine(MockLlmProvider::scripted(dislikes), immediate(config(2)), None).await;
    let (neutral, _, _) = run_engine(MockLlmProvider::scripted(dislikes), immediate(ocean_config(2, 5, 5)), None).await;
    for id in ["g1", "g2"] {
        assert_eq!(last_emotions(&plain, id), last_emotions(&neutral, id), "{id}");
    }
}

/// S27 — emotion-driven sampling: the intervention request carries the modulated
/// parameters, JSON calls keep the original ones, and `emotion_driven = false` changes nothing.
#[tokio::test]
async fn emotion_driven_sampling_reaches_the_intervention_request_only() {
    let mut cfg = config(1);
    cfg.gladiateurs[0].initial_emotions = Some(serde_json::to_string(&crate::models::emotion::EmotionalProfile { enthousiasme: 90, engagement: 20, ..Default::default() }).unwrap());
    let base = cfg.gladiateurs[0].llm_params.clone();
    let (events, _, calls, _) = run_engine_with(MockLlmProvider::scripted(default_script), cfg.clone(), RunOptions { emotion_driven: true, ..Default::default() }).await;
    assert_eq!(count(&events, "error"), 0);
    let g1 = |kind: CallKind| calls.iter().find(|c| c.call_kind == kind && c.speaker_id.as_deref() == Some("g1")).unwrap();
    let intervention = g1(CallKind::Intervention);
    assert!(intervention.params.temperature > base.temperature, "{} vs {}", intervention.params.temperature, base.temperature);
    assert!(intervention.params.num_predict < base.num_predict);
    let intention = g1(CallKind::Intention);
    assert_eq!(intention.params.num_predict, base.num_predict, "JSON calls are never modulated");
    assert_eq!(intention.params.temperature, crate::constants::TEMP_JSON_OUTPUT);
    // Off: original parameters
    let (_, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let plain = calls.iter().find(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).unwrap();
    assert_eq!((plain.params.temperature, plain.params.num_predict), (base.temperature, base.num_predict));
}

/// v1.20.5 — "insightful" is a scarce token: a reactor that answers 💡 to every
/// intervention gets one 💡 per credit window, the others are recorded as likes,
/// and its reaction prompts say the credit is spent meanwhile; a declined
/// reaction ("reacts": false) is a silence.
#[tokio::test]
async fn insightful_credit_bounds_the_strong_points_and_declined_reactions_are_silences() {
    let g2_calls = Arc::new(AtomicU32::new(0));
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Reaction {
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            // The philosopher declines every other time; everyone else finds everything insightful
            let declines = req.speaker_id.as_deref() == Some("g2") && g2_calls.fetch_add(1, Ordering::SeqCst).is_multiple_of(2);
            let content = if declines {
                format!(r#"[{{"speaker":"{name}","reacts":false,"reaction":"none","justification":"","quote":""}}]"#)
            } else {
                format!(r#"[{{"speaker":"{name}","reacts":true,"reaction":"insightful","justification":"fort","quote":""}}]"#)
            };
            return Ok(LlmResponse { content, reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls) = run_engine(provider, immediate(config(6)), None).await;
    assert_eq!(count(&events, "error"), 0);
    let given = |from: &str, kind: &str| events.iter().filter(|e| e["type"] == "reactionEmitted" && e["data"]["reaction"]["fromSpeakerId"] == from && e["data"]["reaction"]["reactionType"] == kind).count();
    // g1 reacts to every one of g2's six interventions: one 💡, then likes while the 💡 stays in the window
    assert_eq!(given("g1", "insightful") + given("g1", "like"), 6, "{:?}", event_types(&events));
    assert_eq!(given("g1", "insightful"), 1, "one per window of {}", crate::constants::INSIGHTFUL_CREDIT_WINDOW);
    assert_eq!(given("g1", "like"), 5);
    // The prompts told g1 so once the credit was spent
    let spent: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Reaction && c.speaker_id.as_deref() == Some("g1") && c.user.contains("crédit \"insightful\" est épuisé")).collect();
    assert_eq!(spent.len(), 5, "every call after the first");
    // g2 declined every other time: three silences, none of them counted
    assert_eq!(given("g2", "insightful") + given("g2", "like"), 3);
}

/// S28 — a threshold crossing yields one stage direction (feed only): never in the
/// prompts, never in the engine history, at most one per speaker and turn.
#[tokio::test]
async fn stage_directions_stay_out_of_the_prompts_and_are_capped() {
    // The analyst pushes g1 to the extremes every turn (two crossings at once)
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Emotion {
            return Ok(LlmResponse { content: r#"{"Le Scientifique":{"frustration":40,"engagement":-40}}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let mut cfg = config(2);
    cfg.gladiateurs[0].system_prompt = "<persona>Le Scientifique</persona><dynamics>Sous pression: Devient sarcastique et coupe court. Multiplie les piques.</dynamics>".to_string();
    // Near both thresholds: the (capped, elastic — v1.20.4) analyst deltas cross frustration ≥ 85 and engagement ≤ 15 at once
    cfg.gladiateurs[0].initial_emotions = Some(serde_json::to_string(&crate::models::emotion::EmotionalProfile { frustration: 83, engagement: 17, ..Default::default() }).unwrap());
    let (events, _, calls, _) = run_engine_with(provider, cfg.clone(), RunOptions { emotion_driven: true, ..Default::default() }).await;
    assert_eq!(count(&events, "error"), 0);
    let directions: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "stageDirection").collect();
    let for_g1: Vec<&&serde_json::Value> = directions.iter().filter(|d| d["data"]["message"]["speakerId"] == "g1").collect();
    assert!(!for_g1.is_empty(), "{:?}", event_types(&events));
    let crossings = events.iter().filter(|e| e["type"] == "emotionalThresholdCrossed" && e["data"]["speakerId"] == "g1").count();
    assert!(crossings >= 2, "two axes cross at the end of turn 1: {crossings}");
    assert!(for_g1.len() < crossings, "one line per speaker and turn at most, whatever the number of crossings");
    let line = for_g1[0]["data"]["message"]["content"].as_str().unwrap();
    assert_eq!(line, "Le Scientifique — devient sarcastique et coupe court.", "the persona's own <dynamics> under pressure");
    assert_eq!(for_g1[0]["data"]["message"]["role"], "GladIAteur");
    // Never in any prompt
    for c in &calls {
        assert!(!c.user.contains("se cale dans son siège") && !c.user.contains("devient sarcastique"), "{}", c.user);
    }
    // Without emotion-driven behaviour: thresholds still emitted for the UI, no stage direction
    let (events, _, _, _) = run_engine_with(MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Emotion {
            return Ok(LlmResponse { content: r#"{"Le Scientifique":{"frustration":40,"engagement":-40}}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    }), cfg, RunOptions::default()).await;
    assert!(count(&events, "emotionalThresholdCrossed") > 0, "{:?}", event_types(&events));
    assert_eq!(events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "stageDirection").count(), 0);
}

/// S29 — a rivalry fades after quiet turns; a positive reaction from a rival is a
/// reconciliation (shift event, stage direction, one-off directive line).
#[tokio::test]
async fn rivalries_decay_and_a_kind_word_reconciles() {
    // Turn 1: mutual dislikes (2 each way with 3 speakers reacting… keep 2 speakers: 1 each way per round)
    let counter = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&counter);
    let provider = MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Reaction {
            let n = c.fetch_add(1, Ordering::SeqCst);
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            // Rounds 0..6 (turns 1-3): dislikes both ways → rival by turn 3 (0.85² + 0.85 + 1 ≥ 2); then silence
            let kind = if n < 6 { "dislike" } else { "none" };
            return Ok(LlmResponse { content: format!(r#"[{{"speaker":"{name}","reaction":"{kind}","justification":"j","quote":""}}]"#), reasoning: None, usage: usage(60, 15), truncated: false });
        }
        default_script(req)
    });
    let (events, _, _, _) = run_engine_with(provider, immediate(config(7)), RunOptions { emotion_driven: true, ..Default::default() }).await;
    assert_eq!(count(&events, "error"), 0);
    let updates: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "relationshipsUpdated").collect();
    let kinds: Vec<Option<&str>> = updates.iter().map(|u| u["data"]["edges"][0]["kind"].as_str()).collect();
    assert!(kinds.contains(&Some("rival")), "{kinds:?}");
    assert_eq!(kinds.last().copied().flatten(), None, "faded after the quiet turns: {kinds:?}");
    let shifts: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "relationshipShift").collect();
    assert!(shifts.iter().any(|s| s["data"]["from"] == "none" && s["data"]["to"] == "rival"), "{shifts:?}");
    // Edges carry a score and a trend
    let last = updates.last().unwrap()["data"]["edges"][0].clone();
    assert!(last["score"].is_number() && last["trend"].is_string(), "{last}");
    assert!(updates.iter().any(|u| u["data"]["edges"][0]["trend"] == "warming"), "decay warms a rivalry");
    // Reconciliation: rivals for three turns, then g2 approves g1 on turn 4
    let counter = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&counter);
    let (events, _, calls, _) = run_engine_with(MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::Reaction {
            let n = c.fetch_add(1, Ordering::SeqCst);
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            // round 6 = turn 4, g2 reacting to g1's intervention
            let kind = if n == 6 { "like" } else { "dislike" };
            return Ok(LlmResponse { content: format!(r#"[{{"speaker":"{name}","reaction":"{kind}","justification":"j","quote":""}}]"#), reasoning: None, usage: usage(60, 15), truncated: false });
        }
        default_script(req)
    }), immediate(config(5)), RunOptions { emotion_driven: true, ..Default::default() }).await;
    let shifts: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "relationshipShift").collect();
    assert!(shifts.iter().any(|s| s["data"]["from"] == "rival"), "{:?}", shifts);
    let reconciliation = events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "stageDirection").any(|e| e["data"]["message"]["content"].as_str().unwrap().contains("tend la main"));
    assert!(reconciliation, "{:?}", event_types(&events));
    let invited = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).any(|c| c.user.contains("vient de te tendre la main"));
    assert!(invited);
}

/// S30 / S31 — the room's temperature reaches the moderator; a lone active speaker has none.
#[tokio::test]
async fn room_mood_reaches_the_moderator_and_needs_two_speakers() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Emotion {
            return Ok(LlmResponse { content: r#"{"Le Scientifique":{"frustration":40},"Le Philosophe":{"frustration":40}}"#.to_string(), reasoning: None, usage: usage(60, 10), truncated: false });
        }
        default_script(req)
    });
    let mut cfg = config(2);
    for g in &mut cfg.gladiateurs {
        g.initial_emotions = Some(serde_json::to_string(&crate::models::emotion::EmotionalProfile { frustration: 60, ..Default::default() }).unwrap());
    }
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    let moods: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "roomMoodUpdated").collect();
    assert!(!moods.is_empty());
    assert_eq!(moods[0]["data"]["label"], "serene");
    assert_eq!(moods.last().unwrap()["data"]["label"], "tense", "{:?}", moods.last());
    assert!(moods.last().unwrap()["data"]["avg"]["frustration"].as_u64().unwrap() > 65);
    // Turn 2 moderation prompts carry the hint (turn 1: serene, no hint needed but still one line)
    let moderations: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Moderation).collect();
    assert!(moderations[0].user.contains("Ambiance de la salle : sereine"), "{}", moderations[0].user);
    assert!(moderations.last().unwrap().user.contains("Ambiance de la salle : tendue"), "{}", moderations.last().unwrap().user);

    let mut cfg = config(1);
    cfg.gladiateurs.truncate(1);
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    assert_eq!(count(&events, "roomMoodUpdated"), 0);
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Moderation).all(|c| !c.user.contains("Ambiance de la salle")));
}

// ── Reactions (Lot 4) ─────────────────────────────────────────────────

/// Reaction script: every reactor approves the target, with a quote copied from the message.
fn reactive_script(kind: &'static str) -> impl Fn(&LlmRequest) -> Result<LlmResponse, LlmError> + Send + Sync + 'static {
    move |req: &LlmRequest| {
        if req.call_kind == CallKind::Reaction {
            // The prompt lists the targets as "- Name : \"content\"" — react to the first one
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            return Ok(LlmResponse {
                content: format!(r#"[{{"speaker":"{name}","reaction":"{kind}","justification":"argument net","quote":"les données montrent"}}]"#),
                reasoning: None, usage: usage(60, 15), truncated: false,
            });
        }
        default_script(req)
    }
}

fn immediate(mut cfg: DiscussionConfig) -> DiscussionConfig {
    cfg.features = DiscussionFeatures::default();
    cfg
}

/// S1 — immediate timing: each intervention is followed by one reaction per other
/// speaker, applied before the next speaker starts; reactions are typed and quoted.
#[tokio::test]
async fn immediate_reactions_follow_every_intervention() {
    let provider = MockLlmProvider::scripted(reactive_script("insightful"));
    let mut cfg = immediate(config(2));
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    // 6 interventions × 2 reactors
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Reaction).count(), 12, "{types:?}");
    assert_eq!(count(&events, "reactionEmitted"), 12);
    assert_eq!(events.iter().filter(|e| e["type"] == "reactionEmitted" && e["data"]["reaction"]["reactionType"] == "insightful").count(), 3, "one 💡 per reactor (v1.20.5 credit)");
    // Every reaction arrives before the following speakerActive
    let mut last_speaker = None;
    for e in &events {
        match e["type"].as_str().unwrap() {
            "messageComplete" if e["data"]["message"]["role"] == "GladIAteur" => last_speaker = Some(e["data"]["message"]["speakerId"].as_str().unwrap().to_string()),
            "reactionEmitted" => {
                assert_ne!(e["data"]["reaction"]["fromSpeakerId"].as_str(), last_speaker.as_deref(), "nobody reacts to themselves");
                // v1.20.5 — 💡 is a scarce token: one per reactor within the credit window, likes afterwards
                assert!(matches!(e["data"]["reaction"]["reactionType"].as_str(), Some("insightful" | "like")), "{e}");
                assert_eq!(e["data"]["reaction"]["quote"], "les données montrent", "exact excerpt kept");
            }
            _ => {}
        }
    }
    // The prompt of the reaction round targets the last intervention only
    let round = calls.iter().find(|c| c.call_kind == CallKind::Reaction).unwrap();
    assert!(round.user.contains("vient de se terminer"), "{}", round.user);
    // The next speaker sees the reactions under the message in the current-turn block
    let second = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).nth(1).unwrap();
    assert!(second.user.contains("[réactions: 💡"), "{}", second.user);
    // Relationship graph refreshed after each round
    assert!(count(&events, "relationshipsUpdated") >= 6);
}

/// S2 — deferred timing reproduces the v1.16 call sequence exactly.
#[tokio::test]
async fn deferred_timing_keeps_the_v116_sequence() {
    let provider = MockLlmProvider::scripted(reactive_script("like"));
    let (events, _, calls) = run_engine(provider, config(2), None).await;
    // The voiced act announcements (v1.20.3) are additive utility calls: left out of the golden sequence
    let kinds: Vec<CallKind> = calls.iter().map(|c| c.call_kind).filter(|k| *k != CallKind::Announcement).collect();
    let expected = vec![
        CallKind::Introduction,
        // turn 1: intention + intervention + moderation ×2, then memory + emotion
        CallKind::Intention, CallKind::Intervention, CallKind::Moderation,
        CallKind::Intention, CallKind::Intervention, CallKind::Moderation,
        CallKind::Emotion, CallKind::Memory,
        // turn 2: reaction before each intervention
        CallKind::Reaction, CallKind::Intention, CallKind::Intervention, CallKind::Moderation,
        CallKind::Reaction, CallKind::Intention, CallKind::Intervention, CallKind::Moderation,
        CallKind::Emotion, CallKind::Memory,
        CallKind::Synthesis,
    ];
    assert_eq!(kinds, expected);
    assert_eq!(count(&events, "reactionEmitted"), 2);
    let prompt = calls.iter().find(|c| c.call_kind == CallKind::Reaction).unwrap();
    assert!(prompt.user.contains("tour précédent"));
}

/// S3 — a lone active speaker triggers no round.
#[tokio::test]
async fn no_round_with_a_single_active_speaker() {
    let provider = MockLlmProvider::scripted(reactive_script("like"));
    let mut cfg = immediate(config(1));
    cfg.gladiateurs.truncate(1);
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Reaction).count(), 0);
    assert_eq!(count(&events, "reactionEmitted"), 0);
    assert_eq!(count(&events, "error"), 0);
}

/// S4 — typed reactions: their emotional effects and the relationship classes.
#[tokio::test]
async fn typed_reactions_move_emotions_and_relationships() {
    // g2 finds g1's points strong and funny; g1 questions g2 (neutral for the graph)
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Reaction {
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            let kind = if req.speaker_id.as_deref() == Some("g2") { "laugh" } else { "question" };
            return Ok(LlmResponse {
                content: format!(r#"[{{"speaker":"{name}","reaction":"{kind}","justification":"","quote":""}}]"#),
                reasoning: None, usage: usage(60, 15), truncated: false,
            });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, immediate(config(2)), None).await;
    assert_eq!(count(&events, "error"), 0);
    let g1 = last_emotions(&events, "g1");
    let g2 = last_emotions(&events, "g2");
    // g1 received 2 laughs (+4 enth each, decayed by 1 per own intervention) and gave 2 questions
    assert!(g1["enthousiasme"].as_i64().unwrap() > 50, "{g1}");
    // g2 received 2 questions → curiosity up; laughing lifts the laugher too
    assert!(g2["curiosite"].as_i64().unwrap() > 50, "{g2}");
    assert!(g2["enthousiasme"].as_i64().unwrap() > 50, "{g2}");
    // Neutral colours never build a relationship
    let last = events.iter().rev().find(|e| e["type"] == "relationshipsUpdated").unwrap();
    assert!(last["data"]["edges"].as_array().unwrap().iter().all(|e| e["kind"].is_null()), "{last}");
}

/// S5 — a paraphrased quote is dropped, an exact one kept.
#[tokio::test]
async fn reaction_quote_is_validated_against_the_message() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Reaction {
            let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
            let name = line.trim_start_matches("- ").split(" : ").next().unwrap_or_default();
            let quote = if req.speaker_id.as_deref() == Some("g2") { "les données montrent une transformation" } else { "les robots remplacent tout le monde" };
            return Ok(LlmResponse {
                content: format!(r#"[{{"speaker":"{name}","reaction":"like","justification":"j","quote":"{quote}"}}]"#),
                reasoning: None, usage: usage(60, 15), truncated: false,
            });
        }
        default_script(req)
    });
    let (events, _, _) = run_engine(provider, immediate(config(1)), None).await;
    let reactions: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "reactionEmitted").collect();
    assert_eq!(reactions.len(), 2);
    let from_g2 = reactions.iter().find(|e| e["data"]["reaction"]["fromSpeakerId"] == "g2").unwrap();
    assert_eq!(from_g2["data"]["reaction"]["quote"], "les données montrent une transformation");
    let from_g1 = reactions.iter().find(|e| e["data"]["reaction"]["fromSpeakerId"] == "g1").unwrap();
    assert!(from_g1["data"]["reaction"].get("quote").is_none(), "paraphrase dropped: {from_g1}");
}

/// S6 — audience reactions: capped per message, felt by the target, refused when disabled.
#[tokio::test]
async fn audience_reactions_are_capped_and_felt() {
    use crate::models::message::ReactionType;
    let provider = MockLlmProvider::scripted(default_script);
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider = Arc::new(provider);
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut cfg = immediate(config(1));
    cfg.gladiateurs.truncate(1); // no peer rounds: only the audience reacts
    cfg.user_intervention_timeout_secs = 5;
    let mut engine = DiscussionEngine::new(cfg, "disc-audience".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(false);
    let (channel, sink) = event_channel();
    let (tx, rx) = mpsc::channel::<EngineCommand>(32);
    // The user asks to intervene: the engine waits for them after the first speaker,
    // and audience reactions are served while it waits (same select loop).
    tx.send(EngineCommand::UserWantsToIntervene).await.unwrap();
    let sink_watch = Arc::clone(&sink);
    let sender = tx.clone();
    let feeder = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            let waiting = sink_watch.lock().unwrap().iter().any(|e| e["type"] == "userTurnReady");
            // React to the introduction (the audience may react to the moderator too)
            let first = sink_watch.lock().unwrap().iter()
                .find(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "IArbitre")
                .map(|e| e["data"]["message"]["id"].as_str().unwrap().to_string());
            if let (true, Some(id)) = (waiting, first) {
                for kind in [ReactionType::Like, ReactionType::Laugh, ReactionType::Insightful, ReactionType::Dislike] {
                    let _ = sender.send(EngineCommand::AudienceReaction { message_id: id.clone(), reaction_type: kind }).await;
                }
                let _ = sender.send(EngineCommand::AudienceReaction { message_id: "unknown".into(), reaction_type: ReactionType::Like }).await;
                let _ = sender.send(EngineCommand::SkipUserTurn).await;
                break;
            }
        }
    });
    engine.run(rx, channel).await;
    feeder.abort();
    let events = sink.lock().unwrap().clone();
    let audience: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "reactionEmitted" && e["data"]["reaction"]["fromSpeakerId"] == "user").collect();
    assert_eq!(audience.len(), 3, "cap of 3 per message: {:?}", event_types(&events));
    assert!(audience.iter().all(|e| e["data"]["reaction"]["fromSpeakerName"] == "Léo"));
    assert_eq!(count(&events, "userTurnTimeout"), 0, "the user skipped explicitly");
    // The moderator felt it: 2 approvals (+10 conf, +5 support), 1 laugh, 1 dislike is over the cap
    let arb = last_emotions(&events, "arb");
    assert!(arb["confiance"].as_i64().unwrap() >= 60, "{arb}");
    assert!(arb["enthousiasme"].as_i64().unwrap() > 50, "{arb}");
    assert_eq!(count(&events, "error"), 0);
}

/// S7 — turn 1 is reactive without breaking the opening-round instruction.
#[tokio::test]
async fn turn_one_reactions_keep_the_opening_round_rule() {
    let provider = MockLlmProvider::scripted(reactive_script("like"));
    let (events, _, calls) = run_engine(provider, immediate(config(1)), None).await;
    assert_eq!(count(&events, "reactionEmitted"), 2);
    let second = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).nth(1).unwrap();
    assert!(second.user.contains("TOUR D'OUVERTURE") || second.user.contains("Présente ta position initiale"), "{}", second.user);
    assert!(second.user.contains("[réactions: 👍"), "{}", second.user);
}

/// A lexical-only knowledge base (no embedding service reachable): the chunks
/// still reach the speakers, are exposed as sources and listed for the synthesis.
#[tokio::test]
async fn rag_sources_are_exposed_and_listed_in_the_synthesis() {
    use crate::rag::chunker::chunk_text;
    use crate::rag::parser::{ParsedDocument, RagFileFormat};
    use crate::rag::{EmbeddingClient, RagStore};

    let mut store = RagStore::new(EmbeddingClient::new("http://127.0.0.1:9", "none"));
    let doc = ParsedDocument {
        file_name: "contrat.txt".to_string(),
        format: RagFileFormat::Txt,
        text: "La clause de non concurrence dure douze mois. L'automatisation des tâches est encadrée par l'article quatre. Le télétravail reste possible trois jours par semaine.".to_string(),
        page_count: 1,
    };
    let chunks = chunk_text(&doc.text, 0, 60, 10);
    store.add_document_text_only(&doc, "doc-1", chunks);

    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::RagSelect {
            return Ok(LlmResponse { content: r#"{"selected": [0, 1]}"#.to_string(), reasoning: None, usage: usage(50, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls, _) = run_engine_with(provider, config(2), RunOptions { rag: Some(store), ..Default::default() }).await;

    assert_eq!(count(&events, "error"), 0, "{:?}", event_types(&events));
    let injected: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "ragContextInjected").collect();
    assert!(injected.len() >= 3, "introduction + speakers: {}", injected.len());
    assert!(injected[0]["data"]["chunks"].as_array().unwrap().iter().all(|c| c["fileName"] == "contrat.txt"));
    // Cache hits are flagged (same speaker within the TTL)
    assert!(injected.iter().any(|e| e["data"]["cached"] == true));

    // The synthesis prompt lists the document chunks as sources, with the pseudo-url the frontend uses
    let synth = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synth.user.contains("[Sources utilisées pendant la discussion]"), "{}", synth.user);
    // Document chunks listed with the frontend pseudo-url, deduplicated by link (retrieval order is not pinned)
    assert!(synth.user.contains("- [document] contrat.txt #"), "{}", synth.user);
    for n in 1..=6 {
        assert!(synth.user.matches(&format!("contrat.txt#{n} (")).count() <= 1, "{}", synth.user);
    }
    assert!(synth.user.contains("## Sources"));
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
            CallKind::Intention => format!(
                r#"{{"target":"{other}","goal":"{}","angle":"angle {n}","concession":{},"question":{},"answers":{},"thought":"Réflexion privée n°{n}."}}"#,
                ["convaincre", "nuancer", "contester", "questionner", "concéder", "relancer"][n as usize % 6],
                if n.is_multiple_of(4) { format!("\"je concède le point {n}\"") } else { "null".to_string() },
                if n.is_multiple_of(3) { format!("\"et le point {n} ?\"") } else { "null".to_string() },
                if n.is_multiple_of(5) { "1".to_string() } else { "null".to_string() },
            ),
            CallKind::Intervention => format!(
                "Intervention {n} de {speaker} : l'automatisation transforme le métier, chiffre {n} à l'appui. {other}, qu'en dites-vous ?"
            ),
            CallKind::Reaction => {
                // React to the first listed target (immediate rounds list one message, deferred the previous turn)
                let line = req.user.lines().find(|l| l.starts_with("- ")).unwrap_or_default();
                let target = line.trim_start_matches("- ").split(" : ").next().filter(|t| !t.is_empty()).unwrap_or(other);
                let kinds = ["like", "insightful", "dislike", "question", "laugh", "like", "offTopic"];
                format!(
                    r#"[{{"speaker":"{target}","reaction":"{}","justification":"argument {n}","quote":"chiffre"}}]"#,
                    kinds[n as usize % kinds.len()]
                )
            }
            CallKind::Moderation => if n.is_multiple_of(7) {
                r#"{"action":"comment","comment":"Restez sur le sujet.","ban_reason":"","ban_duration":0}"#.to_string()
            } else {
                r#"{"action":"none","comment":"","ban_reason":"","ban_duration":0}"#.to_string()
            },
            CallKind::Memory => realistic_memory_json(n),
            CallKind::Emotion => realistic_emotion_json(n),
            CallKind::TurnAnalyst => fused_analyst_json(&realistic_memory_json(n), &realistic_emotion_json(n)),
            CallKind::ArgumentMap => format!(
                r#"{{"extractions":[{{"speaker":"{}","new_theses":["L'automatisation transforme le métier {n}"],"arguments":[{{"text":"Chiffre {n} à l'appui","type":"evidence","for_thesis":"L'automatisation transforme le métier {n}"}}]}}]}}"#,
                if n.is_multiple_of(2) { "Le Scientifique" } else { "Le Philosophe" }
            ),
            CallKind::DocumentUpdate => format!("# Plan\n\n- Point consolidé {n}\n"),
            CallKind::Socratic => format!("Question socratique {n} ?"),
            CallKind::RespondOrPass => format!(r#"{{"respond": {}}}"#, speaker != "g2"),
            CallKind::Synthesis => "## Synthèse\n\nTransformation plutôt que remplacement.\n\n## Agendas\n- **Le Scientifique** : objectif atteint — ses chiffres ont été repris.\n- **Le Philosophe** : objectif non atteint.\n- **La Juriste** : partiellement atteint.".to_string(),
            CallKind::Agenda => agenda_json(req),
            CallKind::Verdict => r#"{"verdict":"défense","accepts":false,"reason":"le doute profite à l'accusé"}"#.to_string(),
            CallKind::CrisisDispatches => r#"{"dispatches":["Dépêche un.","Dépêche deux."]}"#.to_string(),
            CallKind::Announcement => req.user.lines().nth(1).unwrap_or_default().to_string(),
            CallKind::AudienceQuestion => r#"{"question":"Sur quelles données fondez-vous ce chiffre ?"}"#.to_string(),
            CallKind::Vote | CallKind::SearchDecision | CallKind::RagSelect | CallKind::Thought
            | CallKind::Casting | CallKind::Recap => "{}".to_string(),
        };
        Ok(LlmResponse { content, reasoning: Some(format!("raisonnement {n}")), usage: usage(400, 60), truncated: false })
    }
}

fn realistic_memory_json(n: u32) -> String {
    format!(
        r#"{{"summary":"Résumé du tour {n} : angle inédit {n} sur la productivité et la formation.","positions":{{"Le Scientifique":{{"stance":"prudent {n}","shift":"plus ouvert","would_change_if":"des preuves"}},"Le Philosophe":"critique {n}"}},"open_questions":[{{"to":"Le Philosophe","from":"Le Scientifique","question":"Et la formation {n} ?"}}]}}"#
    )
}

fn realistic_emotion_json(n: u32) -> String {
    format!(
        r#"{{"Le Scientifique":{{"engagement":3,"frustration":-2}},"Le Philosophe":{{"curiosite":25}},"Le Modérateur":{{"engagement":4,"confiance":3}},"stagnating":{}}}"#,
        n.is_multiple_of(5)
    )
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
/// reasoning-capable billable provider, every liveliness feature on.
async fn run_full_feature_simulation() -> (Vec<serde_json::Value>, crate::models::llm::UsageLedger, Vec<LlmRequest>, tokio_rusqlite::Connection) {
    let provider = MockLlmProvider::scripted(realistic_script(Arc::new(AtomicU32::new(0))))
        .with_capabilities(billable_caps())
        .with_model_name("deepseek-flash");
    let mut cfg = config(6);
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    cfg.argument_map_enabled = true;
    cfg.document_format = DocumentFormat::Md;
    cfg.features = DiscussionFeatures::default();
    run_engine_with(
        provider,
        cfg,
        RunOptions { budget: Some((0.0, 5.0)), reasoning: Some(ReasoningLevel::High), emotion_driven: true, ..Default::default() },
    )
    .await
}

/// Index of the first event of a type.
fn position_of(types: &[String], ty: &str) -> usize {
    types.iter().position(|t| t == ty).unwrap_or_else(|| panic!("no {ty} in {types:?}"))
}

/// S18 — trial: the roles reach the system prompts (dealt by default, or as
/// configured), the jurors return one `Verdict` call each, the outcome is
/// emitted before the synthesis and the synthesis prompt carries it; without
/// juror the moderator rules alone; a split jury is settled by the moderator.
#[tokio::test]
async fn trial_roles_reach_the_prompts_and_the_jury_returns_the_verdict() {
    let mut cfg = staged_config(2);
    cfg.discussion_mode = DiscussionMode::Trial;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    // Roles dealt from the casting order, announced once
    let roles = events.iter().find(|e| e["type"] == "rolesAssigned").expect("rolesAssigned");
    assert_eq!(roles["data"]["turn"], 1);
    let dealt: Vec<(String, String)> = roles["data"]["roles"].as_array().unwrap().iter().map(|r| (r["speakerId"].as_str().unwrap().to_string(), r["role"].as_str().unwrap().to_string())).collect();
    assert_eq!(dealt, vec![("g1".into(), "prosecutor".into()), ("g2".into(), "defense".into()), ("g3".into(), "juror".into())]);
    assert_eq!(count(&events, "rolesAssigned"), 1);
    assert_eq!(roles["data"]["roles"][0]["label"], "Accusation");
    // The role block sits in the system prompt of the intentions and the interventions
    for c in calls.iter().filter(|c| matches!(c.call_kind, CallKind::Intervention | CallKind::Intention)) {
        let expected = match c.speaker_id.as_deref() { Some("g1") => "Accusation : tu portes l'accusation", Some("g2") => "Défense : tu défends", _ => "Juré : tu sièges au jury" };
        assert!(c.system.contains("[Ton rôle — tiens-le du début à la fin]") && c.system.contains(expected), "{:?}: {}", c.speaker_id, c.system);
        assert!(c.system.contains("FORMAT DE DISCUSSION : PROCÈS") || c.call_kind == CallKind::Intention);
    }
    // One verdict call, by the juror, JSON, with its role
    let verdicts: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Verdict).collect();
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].speaker_id.as_deref(), Some("g3"));
    assert!(verdicts[0].json_mode && verdicts[0].user.contains("En tant que juré") && verdicts[0].system.contains("Juré"));
    // The outcome precedes the synthesis, which is told the verdict
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap();
    assert_eq!(outcome["data"]["outcome"], serde_json::json!({ "kind": "verdict", "votes": [{ "voterId": "g3", "voterName": "La Juriste", "choice": "prosecution", "reason": "les preuves l'emportent" }], "winner": "prosecution", "byArbitre": false }));
    assert!(position_of(&types, "outcomeReady") < position_of(&types, "synthesisChunk"), "{types:?}");
    let synth = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synth.user.contains("[Verdict]") && synth.user.contains("- La Juriste : l'accusation — les preuves l'emportent") && synth.user.contains("Verdict rendu : l'accusation."), "{}", synth.user);
    assert!(synth.user.contains("compte rendu d'audience"));

    // No juror (two gladiateurs): the moderator rules alone
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::Trial;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let verdicts: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Verdict).collect();
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].speaker_id.as_deref(), Some("arb"));
    assert!(verdicts[0].user.contains("Aucun jury n'a siégé"));
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap();
    assert_eq!(outcome["data"]["outcome"]["byArbitre"], true);
    assert_eq!(outcome["data"]["outcome"]["winner"], "prosecution");
    assert_eq!(outcome["data"]["outcome"]["votes"][0]["voterId"], "arb");

    // Configured roles, split jury: the moderator breaks the tie (one more call)
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::Trial;
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    cfg.gladiateurs.push(glad("g4", "L'Ingénieure", 4));
    cfg.gladiateurs[0].mode_role = Some("defense".into());
    cfg.gladiateurs[1].mode_role = Some("prosecutor".into());
    cfg.gladiateurs[2].mode_role = Some("juror".into());
    cfg.gladiateurs[3].mode_role = Some("juror".into());
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Verdict {
            let content = match req.speaker_id.as_deref() {
                Some("g3") => r#"{"verdict":"accusation","reason":"a"}"#,
                Some("g4") => r#"{"verdict":"défense","reason":"b"}"#,
                _ => r#"{"verdict":"défense","reason":"le doute"}"#,
            };
            return Ok(LlmResponse { content: content.to_string(), reasoning: None, usage: usage(50, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    let roles = events.iter().find(|e| e["type"] == "rolesAssigned").unwrap();
    assert_eq!(roles["data"]["roles"][0]["role"], "defense");
    assert_eq!(roles["data"]["roles"][1]["role"], "prosecutor");
    let voters: Vec<&str> = calls.iter().filter(|c| c.call_kind == CallKind::Verdict).map(|c| c.speaker_id.as_deref().unwrap()).collect();
    assert_eq!(voters, vec!["g3", "g4", "arb"]);
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap()["data"]["outcome"].clone();
    assert_eq!(outcome["winner"], "defense");
    assert_eq!(outcome["byArbitre"], false);
    assert_eq!(outcome["votes"].as_array().unwrap().len(), 3);

    // A jury that returns nothing usable (prose, no side): the moderator rules alone
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::Trial;
    cfg.gladiateurs.push(glad("g3", "La Juriste", 3));
    cfg.gladiateurs[2].mode_role = Some("juror".into());
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Verdict && req.speaker_id.as_deref() == Some("g3") {
            return Ok(LlmResponse { content: "Je ne saurais dire.".to_string(), reasoning: None, usage: usage(50, 10), truncated: false });
        }
        default_script(req)
    });
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    let voters: Vec<&str> = calls.iter().filter(|c| c.call_kind == CallKind::Verdict).map(|c| c.speaker_id.as_deref().unwrap()).collect();
    assert_eq!(voters, vec!["g3", "arb"], "the silent jury hands the ruling to the moderator");
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap()["data"]["outcome"].clone();
    assert_eq!(outcome["winner"], "prosecution");
    assert_eq!(outcome["byArbitre"], true);
    assert_eq!(outcome["votes"].as_array().unwrap().len(), 1);
    let diag = events.iter().find(|e| e["type"] == "diagnosticsReady").unwrap();
    assert_eq!(diag["data"]["diagnostics"]["jsonParseFailures"]["verdict"], 1);
}

/// S19 — Oxford debate: the camps are dealt, the audience is asked to vote
/// before turn 1 and after the last turn, the votes make the outcome (winner
/// by displacement), an unknown choice is ignored, no vote → no winner.
#[tokio::test]
async fn oxford_debate_takes_the_audience_votes_before_and_after() {
    let provider = MockLlmProvider::scripted(default_script);
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let provider = Arc::new(provider);
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::OxfordDebate;
    cfg.user_intervention_timeout_secs = 5;
    let mut engine = DiscussionEngine::new(cfg, "disc-oxford".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.disable_random_staging();
    let (channel, sink) = event_channel();
    let (tx, rx) = mpsc::channel::<EngineCommand>(32);
    let sink_watch = Arc::clone(&sink);
    let feeder = tokio::spawn(async move {
        let mut answered = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            let requests = sink_watch.lock().unwrap().iter().filter(|e| e["type"] == "audienceVoteRequested").count();
            if requests > answered {
                answered = requests;
                if requests == 1 {
                    let _ = tx.send(EngineCommand::AudienceVote { choice: "maybe".into() }).await;
                    let _ = tx.send(EngineCommand::AudienceVote { choice: "For".into() }).await;
                } else {
                    let _ = tx.send(EngineCommand::AudienceVote { choice: "against".into() }).await;
                    break;
                }
            }
        }
    });
    engine.run(rx, channel).await;
    feeder.abort();
    let events = sink.lock().unwrap().clone();
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    // Camps dealt alternately, in the prompts
    let roles = events.iter().find(|e| e["type"] == "rolesAssigned").unwrap();
    assert_eq!(roles["data"]["roles"][0]["role"], "for");
    assert_eq!(roles["data"]["roles"][1]["role"], "against");
    let calls = provider.recorded_calls();
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g2")).all(|c| c.system.contains("Camp Contre : tu défends le camp CONTRE la motion")));
    // Two vote windows: before turn 1 (after the introduction) and after the last turn (before the synthesis)
    let requests: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "audienceVoteRequested").collect();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["data"]["phase"], "before");
    assert_eq!(requests[1]["data"]["phase"], "after");
    assert!(position_of(&types, "audienceVoteRequested") < position_of(&types, "turnStarted"));
    let recorded: Vec<(String, String)> = events.iter().filter(|e| e["type"] == "audienceVoteRecorded").map(|e| (e["data"]["phase"].as_str().unwrap().into(), e["data"]["choice"].as_str().unwrap().into())).collect();
    assert_eq!(recorded, vec![("before".to_string(), "for".to_string()), ("after".to_string(), "against".to_string())]);
    assert_eq!(count(&events, "userTurnTimeout"), 0);
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap();
    assert_eq!(outcome["data"]["outcome"], serde_json::json!({ "kind": "audienceSwing", "before": "for", "after": "against", "winner": "against" }));
    let synth = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synth.user.contains("[Vote du public]") && synth.user.contains("Vainqueur par déplacement : contre la motion."), "{}", synth.user);
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Verdict), "no verdict call in an Oxford debate");

    // Nobody votes: both windows time out (or are declined), no winner
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let (tx, rx) = mpsc::channel::<EngineCommand>(8);
    let declined = Arc::new(AtomicU32::new(0));
    let counter = Arc::clone(&declined);
    let provider = Arc::new(MockLlmProvider::scripted(move |req| {
        // Decline the first window from inside the run (the introduction is the last call before it)
        if req.call_kind == CallKind::Introduction && counter.fetch_add(1, Ordering::SeqCst) == 0 {
            let _ = tx.try_send(EngineCommand::SkipUserTurn);
        }
        default_script(req)
    }));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::OxfordDebate;
    cfg.user_intervention_timeout_secs = 1;
    let mut engine = DiscussionEngine::new(cfg, "disc-oxford-2".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.disable_random_staging();
    let (channel, sink) = event_channel();
    let started = std::time::Instant::now();
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    assert!(started.elapsed() < std::time::Duration::from_millis(1_800), "the declined window did not wait for its timeout");
    assert_eq!(count(&events, "audienceVoteRequested"), 2);
    assert_eq!(count(&events, "audienceVoteRecorded"), 0);
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap();
    assert_eq!(outcome["data"]["outcome"], serde_json::json!({ "kind": "audienceSwing", "before": null, "after": null, "winner": null }));
}

/// Negotiation: every party decides at the end (one `Verdict` call each); the
/// agreement holds only when all sign; an unusable answer counts as a refusal.
#[tokio::test]
async fn negotiation_ends_with_every_party_deciding() {
    let provider = MockLlmProvider::scripted(|req| {
        if req.call_kind == CallKind::Verdict {
            let content = match req.speaker_id.as_deref() {
                Some("g1") => r#"{"accepts": true, "reason": "j'obtiens la formation"}"#,
                Some("g2") => r#"{"accepte": "non, pas sans garantie"}"#,
                _ => "pas du json",
            };
            return Ok(LlmResponse { content: content.to_string(), reasoning: None, usage: usage(50, 10), truncated: false });
        }
        default_script(req)
    });
    let mut cfg = staged_config(1);
    cfg.discussion_mode = DiscussionMode::Negotiation;
    let (events, _, calls) = run_engine(provider, cfg, None).await;
    assert_eq!(count(&events, "error"), 0);
    let deciders: Vec<&str> = calls.iter().filter(|c| c.call_kind == CallKind::Verdict).map(|c| c.speaker_id.as_deref().unwrap()).collect();
    assert_eq!(deciders, vec!["g1", "g2", "g3"]);
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Verdict).all(|c| c.user.contains("signes-tu l'accord")));
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap()["data"]["outcome"].clone();
    assert_eq!(outcome["kind"], "agreement");
    assert_eq!(outcome["reached"], false);
    let accepts: Vec<bool> = outcome["parties"].as_array().unwrap().iter().map(|p| p["accepts"].as_bool().unwrap()).collect();
    assert_eq!(accepts, vec![true, false, false]);
    let synth = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synth.user.contains("[Accord]") && synth.user.contains("Pas d'accord") && synth.user.contains("- Le Scientifique : signe — j'obtiens la formation"), "{}", synth.user);
    // Every party gets a secret agenda in a negotiation
    assert_eq!(calls.iter().filter(|c| c.call_kind == CallKind::Agenda).count(), 3);

    // All sign — and the agendas stay mandatory even with the feature off
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::Negotiation;
    cfg.features.hidden_agenda = false;
    let (events, _, _) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let outcome = events.iter().find(|e| e["type"] == "outcomeReady").unwrap()["data"]["outcome"].clone();
    assert_eq!(outcome["reached"], true);
    assert_eq!(count(&events, "agendaRevealed"), 1);
}

/// Six hats: the hats are dealt every turn and rotate so that nobody keeps
/// theirs; each intervention is prompted under the hat of the turn.
#[tokio::test]
async fn six_hats_rotate_every_turn_and_reach_the_prompts() {
    let mut cfg = immediate(config(3));
    cfg.discussion_mode = DiscussionMode::SixHats;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    let deals: Vec<Vec<String>> = events.iter().filter(|e| e["type"] == "rolesAssigned").map(|e| e["data"]["roles"].as_array().unwrap().iter().map(|r| r["role"].as_str().unwrap().to_string()).collect()).collect();
    assert_eq!(deals, vec![vec!["white", "red"], vec!["red", "black"], vec!["black", "yellow"]]);
    // Each deal is part of the staging: it lands right before its TurnStarted
    for (i, t) in types.iter().enumerate() {
        if t == "rolesAssigned" {
            assert_eq!(types[i + 1], "turnStarted", "{types:?}");
        }
    }
    // Turn 2, g1 wears the red hat
    let turn2_g1 = calls.iter().filter(|c| c.call_kind == CallKind::Intervention && c.speaker_id.as_deref() == Some("g1")).nth(1).unwrap();
    assert!(turn2_g1.system.contains("[Ton chapeau ce tour — pense avec lui et rien d'autre]\nChapeau rouge — émotions : émotions et intuitions"), "{}", turn2_g1.system);
    assert!(turn2_g1.system.contains("FORMAT DE DISCUSSION : SIX CHAPEAUX"));
    // No agenda, no scene event in a six-hats session
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Agenda && c.call_kind != CallKind::Verdict));
    assert_eq!(count(&events, "outcomeReady"), 0);
}

/// Crisis cell: the dispatches are written once at the start (as many as
/// turns), one lands every turn as a scene event the speakers must answer;
/// an unusable answer leaves the cell without dispatches.
#[tokio::test]
async fn crisis_cell_delivers_one_dispatch_per_turn() {
    let mut cfg = immediate(config(3));
    cfg.discussion_mode = DiscussionMode::CrisisCell;
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    let dispatch_calls: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::CrisisDispatches).collect();
    assert_eq!(dispatch_calls.len(), 1);
    assert!(dispatch_calls[0].json_mode && dispatch_calls[0].user.contains("Rédige 3 dépêches") && dispatch_calls[0].speaker_id.as_deref() == Some("arb"));
    assert!(position_of(&types, "turnStarted") > calls.iter().position(|c| c.call_kind == CallKind::CrisisDispatches).unwrap(), "written before the first turn");
    let scene: Vec<&serde_json::Value> = events.iter().filter(|e| e["type"] == "sceneEventTriggered").collect();
    assert_eq!(scene.len(), 3);
    for (i, e) in scene.iter().enumerate() {
        assert_eq!(e["data"]["event"]["kind"], "dispatch");
        assert_eq!(e["data"]["event"]["index"], i as u64 + 1);
        assert_eq!(e["data"]["event"]["total"], 3);
        assert_eq!(e["data"]["turn"], i as u64 + 1);
    }
    assert_eq!(scene[1]["data"]["event"]["text"], "Dépêche deux : un témoin parle.");
    // The moderator announces each dispatch; the speakers are told to answer it
    assert!(events.iter().any(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "sceneEvent" && e["data"]["message"]["content"].as_str().unwrap().starts_with("Dépêche 2/3 : Dépêche deux")));
    let turn3 = calls.iter().rfind(|c| c.call_kind == CallKind::Intervention).unwrap();
    assert!(turn3.user.contains("une nouvelle dépêche vient de tomber — « Dépêche trois : l'échéance tombe. »"), "{}", turn3.user);
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Agenda && c.call_kind != CallKind::Verdict));

    // Unusable answer: no dispatches, the cell still runs; without a turn limit the
    // desk writes the default number of dispatches and a soft stop ends the run
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let (tx, rx) = mpsc::channel::<EngineCommand>(8);
    let provider = Arc::new(MockLlmProvider::scripted(move |req| {
        if req.call_kind == CallKind::CrisisDispatches {
            return Ok(LlmResponse { content: "{}".to_string(), reasoning: None, usage: usage(50, 10), truncated: false });
        }
        if req.call_kind == CallKind::Memory {
            let _ = tx.try_send(EngineCommand::Stop);
        }
        default_script(req)
    }));
    let provider_dyn: Arc<dyn LlmProvider> = provider.clone();
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::CrisisCell;
    cfg.max_turns = None;
    let mut engine = DiscussionEngine::new(cfg, "disc-crisis".to_string(), provider_dyn, None, db, None, token_budget::default_priorities());
    engine.disable_random_staging();
    let (channel, sink) = event_channel();
    engine.run(rx, channel).await;
    let events = sink.lock().unwrap().clone();
    let calls = provider.recorded_calls();
    assert_eq!(count(&events, "error"), 0);
    let desk = calls.iter().find(|c| c.call_kind == CallKind::CrisisDispatches).unwrap();
    assert!(desk.user.contains(&format!("Rédige {} dépêches", crate::constants::CRISIS_DISPATCH_DEFAULT_COUNT)), "{}", desk.user);
    assert_eq!(count(&events, "sceneEventTriggered"), 0);
    let diagnostics = events.iter().find(|e| e["type"] == "diagnosticsReady").unwrap();
    assert_eq!(diagnostics["data"]["diagnostics"]["jsonParseFailures"]["crisisDispatches"], 1);
    assert_eq!(count(&events, "synthesisComplete"), 1);
}

/// S26 — long memory: the past recaps of a profile are recalled by topic into
/// the speaker's system prompt; at the end one `Recap` call per speaker built
/// from a profile is emitted as `PersonaRecapReady` (persisted by the frontend
/// with the discussion); a custom persona gets nothing; the setting off means
/// no recall and no recap.
#[tokio::test]
async fn persona_memories_are_recalled_and_recaps_written() {
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let past = |id: &str, topic: &str, lesson: &str| -> crate::models::history::SaveDiscussionRequest {
        serde_json::from_value(serde_json::json!({
            "id": id, "topic": topic, "discussionLanguage": "fr", "modelName": "m", "participants": [], "totalTurns": 1, "synthesis": "s",
            "createdAt": format!("2026-09-1{}T10:00:00Z", id.len()), "messages": [], "llmProvider": "ollama", "usage": {}, "estimatedCostUsd": 0.0,
            "recaps": [{ "speakerId": "x", "speakerName": "Le Scientifique", "profileId": "scientist", "recap": { "positions": ["le télétravail isole"], "lesson": lesson, "allies": ["La Juriste"] } }]
        })).unwrap()
    };
    crate::db::repository::save_discussion(&db, past("p1", "Le télétravail et la productivité", "mesurer avant de juger")).await.unwrap();
    crate::db::repository::save_discussion(&db, past("p22", "La cuisine de saison", "goûter")).await.unwrap();

    let mut cfg = config(1);
    cfg.topic = "La productivité en télétravail".to_string();
    cfg.gladiateurs[0].source_profile_id = Some("scientist".to_string());
    let (events, _, calls, _) = run_engine_with(MockLlmProvider::scripted(default_script), cfg.clone(), RunOptions { persona_memory: true, db: Some(db.clone()), ..Default::default() }).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    // Recall: both memories, the closest to the topic first, in the persona's system prompt (intention and intervention)
    for kind in [CallKind::Intention, CallKind::Intervention] {
        let g1 = calls.iter().find(|c| c.call_kind == kind && c.speaker_id.as_deref() == Some("g1")).unwrap();
        let block = g1.system.find("[Souvenirs de discussions passées]").expect("memories block");
        assert!(block > g1.system.find("</persona>").unwrap(), "after the persona");
        let a = g1.system.find("« Le télétravail et la productivité »").unwrap();
        let b = g1.system.find("« La cuisine de saison »").unwrap();
        assert!(a < b, "closest topic first: {}", g1.system);
        assert!(g1.system.contains("tu défendais : le télétravail isole") && g1.system.contains("leçon : mesurer avant de juger") && g1.system.contains("alliés : La Juriste"));
        let g2 = calls.iter().find(|c| c.call_kind == kind && c.speaker_id.as_deref() == Some("g2")).unwrap();
        assert!(!g2.system.contains("Souvenirs"), "custom persona: nothing to recall");
    }
    // Recap: one call for g1 only, JSON, with its own words, after the synthesis
    let recaps: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Recap).collect();
    assert_eq!(recaps.len(), 1);
    assert_eq!(recaps[0].speaker_id.as_deref(), Some("g1"));
    assert!(recaps[0].json_mode && recaps[0].params.num_predict == crate::constants::RECAP_NUM_PREDICT);
    assert!(recaps[0].user.contains("[Fin de la discussion — mémoire de Le Scientifique]") && recaps[0].user.contains("- « Intervention de g1"), "{}", recaps[0].user);
    assert!(calls.iter().position(|c| c.call_kind == CallKind::Recap).unwrap() > calls.iter().position(|c| c.call_kind == CallKind::Synthesis).unwrap());
    let recap = events.iter().find(|e| e["type"] == "personaRecapReady").expect("personaRecapReady");
    assert_eq!(recap["data"]["recap"]["profileId"], "scientist");
    assert_eq!(recap["data"]["recap"]["speakerId"], "g1");
    assert_eq!(recap["data"]["recap"]["recap"]["rivals"], serde_json::json!(["Le Philosophe"]));
    assert_eq!(recap["data"]["recap"]["recap"]["lesson"], "mesurer avant de juger");
    let end = types.iter().position(|t| t == "discussionEnded").unwrap();
    assert!(types.iter().position(|t| t == "personaRecapReady").unwrap() < end);
    assert!(types.iter().position(|t| t == "personaRecapReady").unwrap() > types.iter().position(|t| t == "synthesisComplete").unwrap());
    // The engine persists nothing itself: the frontend saves the recap with the discussion
    assert_eq!(crate::db::repository::count_persona_memories(&db).await.unwrap(), 2);

    // Setting off: no recall, no recap
    let (events, _, calls, _) = run_engine_with(MockLlmProvider::scripted(default_script), cfg, RunOptions { persona_memory: false, db: Some(db), ..Default::default() }).await;
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Recap && !c.system.contains("Souvenirs")));
    assert_eq!(count(&events, "personaRecapReady"), 0);
}

/// S21 — hidden agendas: one `Agenda` call per gladiateur right after the
/// introduction; the block sits in the SYSTEM prompt of every intervention (and
/// the objective in the intention prompt); the synthesis receives the agendas;
/// `AgendaRevealed` follows `SynthesisComplete` with the verdicts read from the
/// synthesis; a failed call leaves that speaker without agenda; the legacy
/// profile and incompatible modes issue no call.
#[tokio::test]
async fn hidden_agendas_are_generated_injected_judged_and_revealed() {
    let provider = MockLlmProvider::scripted(|req| {
        let content = match req.call_kind {
            CallKind::Agenda if req.speaker_id.as_deref() == Some("g2") => return Err(LlmError::Connection("boom".to_string())),
            CallKind::Synthesis => "## Synthèse\n\nBla.\n\n## Agendas\n- **Le Scientifique** : objectif atteint — les chiffres ont été repris.".to_string(),
            _ => return default_script(req),
        };
        Ok(LlmResponse { content, reasoning: None, usage: usage(100, 20), truncated: false })
    });
    let (events, _, calls) = run_engine(provider, immediate(config(2)), None).await;
    let types = event_types(&events);
    assert_eq!(count(&events, "error"), 0, "{types:?}");
    // One agenda call per gladiateur, right after the introduction, JSON mode
    let kinds: Vec<CallKind> = calls.iter().map(|c| c.call_kind).collect();
    let intro = kinds.iter().position(|k| *k == CallKind::Introduction).unwrap();
    assert_eq!(&kinds[intro + 1..intro + 3], &[CallKind::Agenda, CallKind::Agenda], "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == CallKind::Agenda).count(), 2);
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Agenda).all(|c| c.json_mode && c.params.num_predict == crate::constants::AGENDA_NUM_PREDICT));
    // g1's interventions carry the block in the SYSTEM prompt; g2 (failed call) has none; never in the user message
    let interventions: Vec<&LlmRequest> = calls.iter().filter(|c| c.call_kind == CallKind::Intervention).collect();
    assert_eq!(interventions.len(), 4);
    for c in &interventions {
        let has = c.system.contains("[Ton agenda secret — ne le révèle jamais explicitement]\nObjectif : faire admettre le coût (g1)");
        assert_eq!(has, c.speaker_id.as_deref() == Some("g1"), "{:?}", c.speaker_id);
        assert!(!c.user.contains("agenda secret"));
    }
    assert!(calls.iter().any(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g1") && c.user.contains("[Ton agenda secret] faire admettre le coût (g1)")));
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intention && c.speaker_id.as_deref() == Some("g2")).all(|c| !c.user.contains("agenda secret")));
    // The synthesis receives the agendas and the instruction to judge them
    let synth = calls.iter().find(|c| c.call_kind == CallKind::Synthesis).unwrap();
    assert!(synth.user.contains("[Agendas secrets") && synth.user.contains("- Le Scientifique : objectif « faire admettre le coût (g1) »"), "{}", synth.user);
    assert!(!synth.user.contains("- Le Philosophe : objectif") && synth.user.contains("## Agendas"));
    // Revealed after the synthesis, before the end, with the verdict
    let synth_pos = types.iter().position(|t| t == "synthesisComplete").unwrap();
    let reveal_pos = types.iter().position(|t| t == "agendaRevealed").unwrap();
    assert!(synth_pos < reveal_pos && reveal_pos < types.len() - 1, "{types:?}");
    let reveal = events.iter().find(|e| e["type"] == "agendaRevealed").unwrap();
    assert_eq!(
        reveal["data"]["agendas"],
        serde_json::json!([{ "speakerId": "g1", "speakerName": "Le Scientifique", "objective": "faire admettre le coût (g1)", "redLine": "jamais le remplacement total", "victory": "Le Philosophe cite mes chiffres", "achieved": true }])
    );
    // The token budget reserved the block
    assert!(events.iter().any(|e| e["type"] == "diagnosticsReady"));

    // Legacy profile (v1.16): no call, no reveal
    let (events, _, calls) = run_engine(MockLlmProvider::scripted(default_script), config(1), None).await;
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Agenda));
    assert_eq!(count(&events, "agendaRevealed"), 0);
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intervention).all(|c| !c.system.contains("agenda")));
    // Incompatible mode (Tutorial): ignored even with the feature on
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::Tutorial;
    let (_, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    assert!(calls.iter().all(|c| c.call_kind != CallKind::Agenda));
    // Relay story: the author's wording
    let mut cfg = immediate(config(1));
    cfg.discussion_mode = DiscussionMode::CollaborativeFiction;
    let (_, _, calls) = run_engine(MockLlmProvider::scripted(default_script), cfg, None).await;
    assert!(calls.iter().any(|c| c.call_kind == CallKind::Agenda && c.user.contains("agenda d'auteur")));
    assert!(calls.iter().filter(|c| c.call_kind == CallKind::Intervention).all(|c| c.system.contains("[Ton agenda d'auteur secret")));
}

/// Writes the full-feature event stream to `fixtures/events-full.json` — the
/// frontend replays it through its store (`src/stores/arena/fixture.test.ts`).
/// Run with: `cargo test --lib export_event_fixture -- --ignored`
#[tokio::test]
#[ignore]
async fn export_event_fixture() {
    let (events, _, _, _) = run_full_feature_simulation().await;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures").join("events-full.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&events).unwrap()).unwrap();
    println!("wrote {} events to {}", events.len(), path.display());
}

/// The full pipeline must run without a single error event and keep every invariant.
#[tokio::test]
async fn realistic_full_feature_debate_runs_clean() {
    let (events, ledger, calls, db) = run_full_feature_simulation().await;
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
    // Secret agendas: three generated, unveiled after the synthesis with the moderator's verdicts
    let reveal = events.iter().find(|e| e["type"] == "agendaRevealed").expect("agendaRevealed");
    let verdicts: Vec<serde_json::Value> = reveal["data"]["agendas"].as_array().unwrap().iter().map(|a| a["achieved"].clone()).collect();
    assert_eq!(verdicts, vec![serde_json::json!(true), serde_json::json!(false), serde_json::Value::Null]);

    // With a fixed High level every intervention reasons natively → reasoning stored as the thought
    let interventions: Vec<&serde_json::Value> = events
        .iter()
        .filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "GladIAteur" && e["data"]["message"]["kind"] == "normal")
        .collect();
    assert!(interventions.len() >= 18);
    assert!(interventions.iter().all(|m| m["data"]["message"]["thoughtKind"] == "reasoning"));
    // Emotion-driven theatre: stage directions (feed only) and the room mood
    assert!(events.iter().any(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "stageDirection"));
    assert!(count(&events, "roomMoodUpdated") >= 6);
    // v1.20.5 — balance under a realistic run (rotating reaction colours, a flattering
    // analyst): no axis saturates, the theatre fires but never at every intervention,
    // and the moderator drifts no further than anyone else
    let peak = events
        .iter()
        .filter(|e| e["type"] == "emotionUpdated")
        .flat_map(|e| e["data"]["emotions"].as_object().unwrap().values().map(|v| v.as_u64().unwrap()))
        .max()
        .unwrap();
    assert!(peak <= 92, "emotional peak {peak}");
    let turns = count(&events, "turnStarted");
    for sid in ["g1", "g2", "g3", "arb"] {
        let lines = events.iter().filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["kind"] == "stageDirection" && e["data"]["message"]["speakerId"] == sid).count();
        assert!(lines <= turns / 2 + 1, "{sid}: {lines} stage directions over {turns} turns — the theatre must stay rare");
    }
    assert!(count(&events, "emotionalThresholdCrossed") >= 1, "movements from the baseline reach the UI");
    let moderator_last = events.iter().rev().find(|e| e["type"] == "emotionUpdated" && e["data"]["speakerId"] == "arb").unwrap();
    let arb_engagement = moderator_last["data"]["emotions"]["engagement"].as_u64().unwrap();
    assert!((42..=75).contains(&arb_engagement), "a flattered moderator in a stagnating debate stays bounded: {arb_engagement}");

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
    for mode in DiscussionMode::ALL {
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
            DiscussionMode::Trial | DiscussionMode::Negotiation => assert_eq!(count(&events, "outcomeReady"), 1, "{mode:?}: {types:?}"),
            DiscussionMode::OxfordDebate => {
                assert_eq!(count(&events, "audienceVoteRequested"), 2, "{types:?}");
                assert_eq!(count(&events, "outcomeReady"), 1);
            }
            DiscussionMode::SixHats => assert_eq!(count(&events, "rolesAssigned"), 2, "one deal per turn"),
            DiscussionMode::CrisisCell => assert_eq!(count(&events, "sceneEventTriggered"), 2, "one dispatch per turn: {types:?}"),
            _ => {}
        }
    }
}
