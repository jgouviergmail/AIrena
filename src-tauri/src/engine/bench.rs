//! Real-model prompt bench (ignored: needs a running Ollama or a DeepSeek key).
//!
//! ```text
//! AIRENA_BENCH_PROVIDER=ollama  OLLAMA_MODEL=<model> [OLLAMA_URL=…] cargo test --lib bench_prompts -- --ignored --nocapture
//! AIRENA_BENCH_PROVIDER=deepseek DEEPSEEK_API_KEY=… [DEEPSEEK_MODEL=deepseek-flash] cargo test --lib bench_prompts -- --ignored --nocapture
//! ```
//!
//! Three scenarios run with the seeded personas (`AIRENA_BENCH_SCENARIOS=debat,ideation`
//! restricts the run, `AIRENA_BENCH_TURNS=2` shortens every scenario,
//! `AIRENA_BENCH_RUNS=3` plays every scenario three times and reports the mean
//! (per-run rows below it), `AIRENA_BENCH_TAG=x` suffixes the report names,
//! `AIRENA_BENCH_REPLAY=<report prefix> cargo test --lib bench_replay -- --ignored --nocapture`
//! recomputes the metrics of an earlier run from its saved events (no model call —
//! for when a metric's definition changes after a long run),
//! `AIRENA_BENCH_TRACE=info` prints the engine's tracing to stderr — the raw
//! answers of failed structured calls included); the metrics of `bench_metrics` are written to
//! `target/bench/<date>-<provider>.{md,json}`, the raw events of each scenario to
//! `target/bench/<date>-<provider>-<scenario>.events.json` (post-mortem of the
//! metrics: which intention missed its target, which answer leaked Markdown…), and
//! compared with `node tools/bench-compare.mjs <before.json> <after.json>`.
//! Keep the reference report of a release under `Docs/Technique/bench/`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::sync::mpsc;

use super::bench_metrics::{self, BenchMetrics};
use super::orchestrator::DiscussionEngine;
use super::token_budget;
use crate::db::{repository, schema, seed};
use crate::llm::factory;
use crate::models::discussion::{DiscussionConfig, DiscussionFeatures, DiscussionMode, DocumentFormat, DocumentInjectionMode, DocumentUpdateGranularity, TurnDistribution};
use crate::models::engine_command::EngineCommand;
use crate::models::gladiateur::GladIAteurConfig;
use crate::models::iarbitre::IArbitreConfig;
use crate::models::llm::{ProviderKind, ReasoningLevel};
use crate::models::profile::PredefinedProfile;
use crate::models::settings::{AppSettings, LlmParams};

const BENCH_NUM_CTX: u32 = 16_384;

struct Scenario {
    name: &'static str,
    topic: &'static str,
    mode: DiscussionMode,
    profiles: &'static [&'static str],
    turns: u32,
    argument_map: bool,
}

const SCENARIOS: &[Scenario] = &[
    Scenario { name: "debat", topic: "L'intelligence artificielle va-t-elle remplacer les développeurs d'ici dix ans ?", mode: DiscussionMode::Debate, profiles: &["scientist", "philosopher", "devils-advocate"], turns: 4, argument_map: true },
    Scenario { name: "ideation", topic: "Comment rendre une ville moyenne attractive pour les jeunes actifs ?", mode: DiscussionMode::Ideation, profiles: &["creative", "pragmatic", "optimist"], turns: 3, argument_map: false },
    Scenario { name: "socratique", topic: "Peut-on être libre sans être responsable ?", mode: DiscussionMode::Socratic, profiles: &["philosopher", "critic"], turns: 3, argument_map: false },
];

fn settings_from_env() -> AppSettings {
    let provider = std::env::var("AIRENA_BENCH_PROVIDER").unwrap_or_else(|_| "ollama".to_string());
    AppSettings {
        username: "Bench".to_string(),
        ollama_url: std::env::var("OLLAMA_URL").unwrap_or_else(|_| crate::constants::DEFAULT_OLLAMA_URL.to_string()),
        ollama_model: std::env::var("OLLAMA_MODEL").unwrap_or_default(),
        llm_provider: ProviderKind::parse(&provider),
        deepseek_api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
        deepseek_model: std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| crate::constants::DEEPSEEK_DEFAULT_MODEL.to_string()),
        num_ctx: BENCH_NUM_CTX,
        reasoning_level: ReasoningLevel::Auto,
        emotion_driven: true,
        ..Default::default()
    }
}

fn pick<'a>(profiles: &'a [PredefinedProfile], id: &str) -> &'a PredefinedProfile {
    profiles.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("seed profile {id} missing"))
}

fn build_config(sc: &Scenario, gladiateurs: &[PredefinedProfile], arbitre: &PredefinedProfile) -> DiscussionConfig {
    let params = LlmParams { num_ctx: BENCH_NUM_CTX, ..LlmParams::default() };
    DiscussionConfig {
        topic: sc.topic.to_string(),
        discussion_language: "fr".to_string(),
        arbitre: IArbitreConfig {
            id: arbitre.id.clone(),
            name: arbitre.name.clone(),
            system_prompt: arbitre.system_prompt.clone(),
            turn_distribution: TurnDistribution::Sequential,
            llm_params: params.clone(),
            web_search_intro: false,
            wiki_search_intro: false,
            model: None,
        },
        gladiateurs: gladiateurs
            .iter()
            .enumerate()
            .map(|(i, p)| GladIAteurConfig {
                mode_role: None,
                model: None,
                source_profile_id: None,
                id: p.id.clone(),
                name: p.name.clone(),
                intervention_number: i as u32 + 1,
                system_prompt: p.system_prompt.clone(),
                llm_params: params.clone(),
                emoji: None,
                initial_emotions: p.initial_emotions.clone(),
            })
            .collect(),
        max_turns: Some(std::env::var("AIRENA_BENCH_TURNS").ok().and_then(|v| v.parse().ok()).unwrap_or(sc.turns)),
        user_name: "Léo".to_string(),
        user_intervention_timeout_secs: 1,
        web_search_pool: 0,
        wiki_search_pool: 0,
        discussion_mode: sc.mode.clone(),
        document_format: DocumentFormat::None,
        argument_map_enabled: sc.argument_map,
        document_injection_mode: DocumentInjectionMode::Rag,
        document_update_granularity: DocumentUpdateGranularity::Turn,
        features: DiscussionFeatures::default(),
    }
}

/// Runs one scenario and returns (events, reception times in ms since start).
async fn run_scenario(settings: &AppSettings, cfg: DiscussionConfig) -> (Vec<serde_json::Value>, Vec<u64>) {
    let provider = factory::build_provider(settings).await.expect("provider must validate");
    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    let argument_map = cfg.argument_map_enabled;
    let mut engine = DiscussionEngine::new(cfg, "bench".to_string(), provider, None, db, None, token_budget::default_priorities());
    engine.set_emotion_driven(settings.emotion_driven);
    engine.set_argument_map_enabled(argument_map);
    engine.set_reasoning_options(settings.reasoning_level, false, settings.reasoning_pace);

    let start = Instant::now();
    let sink: Arc<Mutex<Vec<(serde_json::Value, u64)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink_clone = Arc::clone(&sink);
    let channel: Channel<crate::models::events::ArenaEvent> = Channel::new(move |body| {
        if let InvokeResponseBody::Json(json) = body {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
                sink_clone.lock().unwrap().push((v, start.elapsed().as_millis() as u64));
            }
        }
        Ok(())
    });
    let (_tx, rx) = mpsc::channel::<EngineCommand>(8);
    engine.run(rx, channel).await;
    let collected = sink.lock().unwrap().clone();
    collected.into_iter().unzip()
}

/// Report (Markdown + JSON) of per-scenario runs, means first.
fn write_report(base: &std::path::Path, date: &str, provider_label: &str, runs: usize, elapsed_secs: f32, rows: &[(String, Vec<BenchMetrics>)]) {
    let mut md = format!("# Banc de prompts — {date} — {provider_label}\n\nDurée totale : {elapsed_secs:.0} s — {runs} passe(s) par scénario\n\n{}\n", bench_metrics::MARKDOWN_HEADER);
    for (name, list) in rows {
        md.push_str(&bench_metrics::to_markdown_row(name, &BenchMetrics::mean(list)));
        md.push('\n');
        if runs > 1 {
            for (i, m) in list.iter().enumerate() {
                md.push_str(&bench_metrics::to_markdown_row(&format!("{name} #{}", i + 1), m));
                md.push('\n');
            }
        }
    }
    let json = serde_json::json!({
        "date": date,
        "provider": provider_label,
        "runs": runs,
        "scenarios": rows.iter().map(|(n, list)| serde_json::json!({"name": n, "metrics": BenchMetrics::mean(list), "runs": list})).collect::<Vec<_>>(),
    });
    std::fs::write(format!("{}.md", base.display()), &md).unwrap();
    std::fs::write(format!("{}.json", base.display()), serde_json::to_string_pretty(&json).unwrap()).unwrap();
    println!("{md}\nwritten to {}", base.display());
}

/// Recompute the metrics of a saved run from its `<prefix>-<scenario>[-runN].events.json`
/// files (v1.20.5): the cast is read from the messages, the timestamps are gone
/// (no speaker gap). Writes `<prefix>-replay.{md,json}`.
#[tokio::test]
#[ignore]
async fn bench_replay() {
    let prefix = std::env::var("AIRENA_BENCH_REPLAY").expect("AIRENA_BENCH_REPLAY=<report prefix>");
    let prefix_path = std::path::Path::new(&prefix);
    let dir = prefix_path.parent().filter(|p| !p.as_os_str().is_empty()).map(std::path::Path::to_path_buf).unwrap_or_else(|| std::path::PathBuf::from("."));
    let stem = prefix_path.file_name().unwrap().to_string_lossy().to_string();
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with(&format!("{stem}-")) && name.ends_with(".events.json")
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no events file under {}", dir.display());
    let mut rows: Vec<(String, Vec<BenchMetrics>)> = Vec::new();
    let mut runs = 1usize;
    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy().to_string();
        let middle = name.trim_start_matches(&format!("{stem}-")).trim_end_matches(".events.json").to_string();
        let (scenario, run) = match middle.rsplit_once("-run") {
            Some((sc, n)) => (sc.to_string(), n.parse::<usize>().unwrap_or(1)),
            None => (middle.clone(), 1),
        };
        runs = runs.max(run);
        let events: Vec<serde_json::Value> = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let names: HashMap<String, String> = events
            .iter()
            .filter(|e| e["type"] == "messageComplete" && e["data"]["message"]["role"] == "GladIAteur")
            .filter_map(|e| Some((e["data"]["message"]["speakerId"].as_str()?.to_string(), e["data"]["message"]["speakerName"].as_str()?.to_string())))
            .collect();
        let metrics = bench_metrics::from_events(&events, &names, None);
        println!("[replay] {scenario} run {run}: {} interventions", metrics.interventions);
        match rows.iter_mut().find(|(n, _)| *n == scenario) {
            Some((_, list)) => list.push(metrics),
            None => rows.push((scenario, vec![metrics])),
        }
    }
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    write_report(&dir.join(format!("{stem}-replay")), &date, &format!("{stem} (replay)"), runs, 0.0, &rows);
}

#[tokio::test]
#[ignore]
async fn bench_prompts() {
    if let Ok(filter) = std::env::var("AIRENA_BENCH_TRACE") {
        let _ = tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).try_init();
    }
    let settings = settings_from_env();
    let provider_label = match settings.llm_provider {
        ProviderKind::Ollama => format!("ollama-{}", settings.ollama_model.replace([':', '/', '.'], "_")),
        ProviderKind::DeepSeek => format!("deepseek-{}", settings.deepseek_model.replace('.', "_")),
        ProviderKind::OpenAiCompat => format!("openai-compat-{}", settings.openai_compat_model.replace([':', '/', '.'], "_")),
    };

    let db = tokio_rusqlite::Connection::open_in_memory().await.unwrap();
    schema::initialize(&db).await.unwrap();
    seed::seed_profiles(&db).await.unwrap();
    let profiles = repository::list_profiles(&db).await.unwrap();
    let arbitres = repository::list_arbitre_profiles(&db).await.unwrap();
    let arbitre = pick(&arbitres, "arb-impartial");

    let only: Vec<String> = std::env::var("AIRENA_BENCH_SCENARIOS")
        .map(|v| v.split(',').map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("bench");
    std::fs::create_dir_all(&dir).unwrap();
    let tag = std::env::var("AIRENA_BENCH_TAG").ok().filter(|t| !t.is_empty()).map(|t| format!("-{t}")).unwrap_or_default();
    let runs: usize = std::env::var("AIRENA_BENCH_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1, 10);
    let base = dir.join(format!("{date}-{provider_label}{tag}"));

    // Per scenario: the metrics of every run (v1.20.5)
    let mut rows: Vec<(String, Vec<BenchMetrics>)> = Vec::new();
    let started = Instant::now();
    for run in 1..=runs {
        for sc in SCENARIOS.iter().filter(|sc| only.is_empty() || only.iter().any(|o| o == sc.name)) {
            let glads: Vec<PredefinedProfile> = sc.profiles.iter().map(|id| pick(&profiles, id).clone()).collect();
            let cfg = build_config(sc, &glads, arbitre);
            let names: HashMap<String, String> = cfg.gladiateurs.iter().map(|g| (g.id.clone(), g.name.clone())).collect();
            let t0 = Instant::now();
            let (events, ts) = run_scenario(&settings, cfg).await;
            let metrics = bench_metrics::from_events(&events, &names, Some(&ts));
            println!("[bench] {} (run {run}/{runs}) — {} interventions in {:.0}s", sc.name, metrics.interventions, t0.elapsed().as_secs_f32());
            // Raw events for the post-mortem (streaming chunks left out: they would dwarf the file)
            let kept: Vec<&serde_json::Value> = events.iter().filter(|e| !matches!(e["type"].as_str(), Some("messageChunk" | "thoughtChunk" | "synthesisChunk"))).collect();
            let run_suffix = if runs > 1 { format!("-run{run}") } else { String::new() };
            std::fs::write(format!("{}-{}{run_suffix}.events.json", base.display(), sc.name), serde_json::to_string_pretty(&kept).unwrap()).unwrap();
            match rows.iter_mut().find(|(n, _)| n == sc.name) {
                Some((_, list)) => list.push(metrics),
                None => rows.push((sc.name.to_string(), vec![metrics])),
            }
        }
    }

    write_report(&base, &date, &format!("{provider_label}{tag}"), runs, started.elapsed().as_secs_f32(), &rows);
}
