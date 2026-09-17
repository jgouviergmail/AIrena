use chrono::DateTime;
use tokio_rusqlite::Connection;

use super::rolling_period;
use super::schema;
use crate::models::template::DiscussionTemplate;
use crate::models::persona_memory::{PersonaMemory, PersonaRecap};
use crate::rag::bm25::{tokenize, Bm25Index};
use crate::models::history::{
    DiscussionDetail, DiscussionSummary, ParticipantInfo, SaveDiscussionRequest,
};
use crate::models::llm::{LlmUsage, PeriodHistoryEntry, PeriodUsage, ProviderKind, ReasoningLevel, ReasoningPace, UsageLedger};
use crate::models::message::{Message, MessageKind, Reaction, SpeakerRole, ThoughtKind};
use crate::models::profile::PredefinedProfile;
use crate::constants;
use crate::models::settings::{AppSettings, TtsMode};

pub async fn get_settings(db: &Connection) -> Result<AppSettings, tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut settings = AppSettings::default();
        for (key, value) in rows {
            match key.as_str() {
                "username" => settings.username = value,
                "language" => settings.language = value,
                "theme" => settings.theme = value,
                "ollama_url" => settings.ollama_url = value,
                "ollama_model" => settings.ollama_model = value,
                "emotion_driven" => settings.emotion_driven = value == "true",
                "tavily_api_key" => settings.tavily_api_key = value,
                "tavily_period_start" => settings.tavily_period_start = value,
                "tavily_usage_count" => settings.tavily_usage_count = value.parse().unwrap_or(0),
                "tavily_usage_history" => settings.tavily_usage_history = value,
                "embedding_model" => settings.embedding_model = value,
                "license_key" => settings.license_key = value,
                "token_budget_priorities" => settings.token_budget_priorities = value,
                "num_ctx" => settings.num_ctx = value.parse().unwrap_or(constants::LLM_DEFAULT_NUM_CTX),
                "llm_provider" => settings.llm_provider = ProviderKind::parse(&value),
                "reasoning_level" => settings.reasoning_level = ReasoningLevel::parse(&value),
                "show_model_reasoning" => settings.show_model_reasoning = value == "true",
                "reasoning_pace" => settings.reasoning_pace = ReasoningPace::parse(&value),
                "tts_enabled" => settings.tts_enabled = value == "true",
                "tts_mode" => settings.tts_mode = TtsMode::parse(&value),
                "tts_volume" => settings.tts_volume = value.parse::<f32>().map(|v| v.clamp(0.0, 1.0)).unwrap_or(constants::AUDIO_DEFAULT_TTS_VOLUME),
                "sound_enabled" => settings.sound_enabled = value == "true",
                "sound_volume" => settings.sound_volume = value.parse::<f32>().map(|v| v.clamp(0.0, 1.0)).unwrap_or(constants::AUDIO_DEFAULT_SOUND_VOLUME),
                "deepseek_api_key" => settings.deepseek_api_key = value,
                "deepseek_model" => {
                    if !value.trim().is_empty() {
                        settings.deepseek_model = value;
                    }
                }
                "deepseek_monthly_budget_usd" => {
                    settings.deepseek_monthly_budget_usd = value.parse().unwrap_or(0.0)
                }
                "deepseek_period_start" => settings.deepseek_period_start = value,
                "deepseek_period_usage_json" => settings.deepseek_period_usage_json = value,
                "deepseek_usage_history" => settings.deepseek_usage_history = value,
                "openai_compat_base_url" => {
                    if !value.trim().is_empty() {
                        settings.openai_compat_base_url = value;
                    }
                }
                "openai_compat_api_key" => settings.openai_compat_api_key = value,
                "openai_compat_model" => settings.openai_compat_model = value,
                "openai_compat_models" => settings.openai_compat_models = value,
                "advanced_tuning_json" => {
                    if !value.trim().is_empty() {
                        settings.advanced_tuning_json = value;
                    }
                }
                "persona_memory_enabled" => settings.persona_memory_enabled = value == "true",
                _ => {}
            }
        }
        Ok(settings)
    })
    .await
}

/// Persist settings coming from the frontend. The DeepSeek period counters are
/// owned by the backend (`record_deepseek_usage`, rollover, reset): a stale
/// payload from the UI must never overwrite them, so they are re-read from the
/// database before the write.
pub async fn save_user_settings(
    db: &Connection,
    incoming: &AppSettings,
) -> Result<(), tokio_rusqlite::Error> {
    let current = get_settings(db).await?;
    let merged = AppSettings {
        deepseek_period_start: current.deepseek_period_start,
        deepseek_period_usage_json: current.deepseek_period_usage_json,
        deepseek_usage_history: current.deepseek_usage_history,
        ..incoming.clone()
    };
    save_settings(db, &merged).await
}

/// Persist every setting, server-owned counters included (internal use).
pub async fn save_settings(
    db: &Connection,
    settings: &AppSettings,
) -> Result<(), tokio_rusqlite::Error> {
    let mut settings = settings.clone();

    // Auto-set period_start if key is present but period is empty
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if !settings.tavily_api_key.is_empty() && settings.tavily_period_start.is_empty() {
        settings.tavily_period_start = today.clone();
    }
    if !settings.deepseek_api_key.is_empty() && settings.deepseek_period_start.is_empty() {
        settings.deepseek_period_start = today;
    }

    db.call(move |conn| {
        let tx = conn.transaction()?;
        let pairs: Vec<(&str, String)> = vec![
            ("username", settings.username.clone()),
            ("language", settings.language.clone()),
            ("theme", settings.theme.clone()),
            ("ollama_url", settings.ollama_url.clone()),
            ("ollama_model", settings.ollama_model.clone()),
            ("emotion_driven", settings.emotion_driven.to_string()),
            ("tavily_api_key", settings.tavily_api_key.clone()),
            ("tavily_period_start", settings.tavily_period_start.clone()),
            ("tavily_usage_count", settings.tavily_usage_count.to_string()),
            ("tavily_usage_history", settings.tavily_usage_history.clone()),
            ("embedding_model", settings.embedding_model.clone()),
            ("license_key", settings.license_key.clone()),
            ("token_budget_priorities", settings.token_budget_priorities.clone()),
            ("num_ctx", settings.num_ctx.to_string()),
            ("llm_provider", settings.llm_provider.as_str().to_string()),
            ("reasoning_level", settings.reasoning_level.as_str().to_string()),
            ("show_model_reasoning", settings.show_model_reasoning.to_string()),
            ("reasoning_pace", settings.reasoning_pace.as_str().to_string()),
            ("tts_enabled", settings.tts_enabled.to_string()),
            ("tts_mode", settings.tts_mode.as_str().to_string()),
            ("tts_volume", settings.tts_volume.to_string()),
            ("sound_enabled", settings.sound_enabled.to_string()),
            ("sound_volume", settings.sound_volume.to_string()),
            ("deepseek_api_key", settings.deepseek_api_key.clone()),
            ("deepseek_model", settings.deepseek_model.clone()),
            ("deepseek_monthly_budget_usd", settings.deepseek_monthly_budget_usd.to_string()),
            ("deepseek_period_start", settings.deepseek_period_start.clone()),
            ("deepseek_period_usage_json", settings.deepseek_period_usage_json.clone()),
            ("deepseek_usage_history", settings.deepseek_usage_history.clone()),
            ("openai_compat_base_url", settings.openai_compat_base_url.clone()),
            ("openai_compat_api_key", settings.openai_compat_api_key.clone()),
            ("openai_compat_model", settings.openai_compat_model.clone()),
            ("openai_compat_models", settings.openai_compat_models.clone()),
            ("advanced_tuning_json", settings.advanced_tuning_json.clone()),
            ("persona_memory_enabled", settings.persona_memory_enabled.to_string()),
        ];
        for (key, value) in &pairs {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                rusqlite::params![key, value],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Column list for predefined_profiles queries
const PROFILE_COLUMNS: &str = "id, name, personality, system_prompt, is_builtin, profile_type, category, initial_emotions";

/// Map a database row to PredefinedProfile
fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<PredefinedProfile> {
    Ok(PredefinedProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        personality: row.get(2)?,
        system_prompt: row.get(3)?,
        is_builtin: row.get::<_, i32>(4)? != 0,
        profile_type: row.get(5)?,
        category: row.get(6)?,
        initial_emotions: row.get(7)?,
    })
}

async fn list_profiles_by_type(
    db: &Connection,
    profile_type: &str,
) -> Result<Vec<PredefinedProfile>, tokio_rusqlite::Error> {
    let profile_type = profile_type.to_string();
    db.call(move |conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {PROFILE_COLUMNS} FROM predefined_profiles WHERE profile_type = ?1 ORDER BY name"
        ))?;
        let profiles = stmt
            .query_map(rusqlite::params![profile_type], row_to_profile)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(profiles)
    })
    .await
}

pub async fn list_profiles(
    db: &Connection,
) -> Result<Vec<PredefinedProfile>, tokio_rusqlite::Error> {
    list_profiles_by_type(db, "gladiateur").await
}

pub async fn list_arbitre_profiles(
    db: &Connection,
) -> Result<Vec<PredefinedProfile>, tokio_rusqlite::Error> {
    list_profiles_by_type(db, "arbitre").await
}

pub async fn get_profile(
    db: &Connection,
    id: &str,
) -> Result<Option<PredefinedProfile>, tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {PROFILE_COLUMNS} FROM predefined_profiles WHERE id = ?1"
        ))?;
        let profile = stmt
            .query_row(rusqlite::params![id], row_to_profile)
            .optional()?;
        Ok(profile)
    })
    .await
}

pub async fn save_profile(
    db: &Connection,
    profile: &PredefinedProfile,
) -> Result<(), tokio_rusqlite::Error> {
    let profile = profile.clone();
    db.call(move |conn| {
        conn.execute(
            "INSERT INTO predefined_profiles (id, name, personality, system_prompt, is_builtin, profile_type, category, initial_emotions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET name = ?2, personality = ?3, system_prompt = ?4, is_builtin = ?5, profile_type = ?6, category = ?7, initial_emotions = ?8",
            rusqlite::params![
                profile.id,
                profile.name,
                profile.personality,
                profile.system_prompt,
                profile.is_builtin as i32,
                profile.profile_type,
                profile.category,
                profile.initial_emotions,
            ],
        )?;
        Ok(())
    })
    .await
}

pub async fn delete_profile(db: &Connection, id: &str) -> Result<(), tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        conn.execute(
            "DELETE FROM predefined_profiles WHERE id = ?1 AND is_builtin = 0",
            rusqlite::params![id],
        )?;
        Ok(())
    })
    .await
}

/// Trait d'extension pour Option sur rusqlite
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// ── Discussion history ──────────────────────────────────────────────

pub async fn save_discussion(
    db: &Connection,
    request: SaveDiscussionRequest,
) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        let tx = conn.transaction()?;

        let participants_json = serde_json::to_string(&request.participants)
            .unwrap_or_else(|_| "[]".to_string());
        let usage_json = serde_json::to_string(&request.usage).unwrap_or_else(|_| "{}".to_string());

        tx.execute(
            "INSERT INTO discussions (id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_content, document_format, argument_map_md, argument_map_md_by_speaker, llm_provider, usage_json, estimated_cost_usd, argument_map_json, report_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                request.id,
                request.topic,
                request.discussion_language,
                request.model_name,
                participants_json,
                request.total_turns,
                request.synthesis,
                request.created_at,
                request.discussion_mode,
                request.document_content,
                request.document_format,
                request.argument_map_md,
                request.argument_map_md_by_speaker,
                request.llm_provider,
                usage_json,
                request.estimated_cost_usd,
                request.argument_map_json,
                request.report_json,
            ],
        )?;
        let inserted = tx.changes() > 0;

        for (i, msg) in request.messages.iter().enumerate() {
            let reactions_json = serde_json::to_string(&msg.reactions)
                .unwrap_or_else(|_| "[]".to_string());
            let role_str = serde_json::to_string(&msg.role)
                .unwrap_or_else(|_| "\"GladIAteur\"".to_string());
            // Remove surrounding quotes from serialized role string
            let role_str = role_str.trim_matches('"');
            let timestamp_str = msg.timestamp.to_rfc3339();

            tx.execute(
                "INSERT INTO discussion_messages (id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp, sort_order, thought_kind, kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO NOTHING",
                rusqlite::params![
                    msg.id,
                    msg.discussion_id,
                    msg.turn_number,
                    msg.speaker_id,
                    msg.speaker_name,
                    role_str,
                    msg.content,
                    msg.inner_thought,
                    reactions_json,
                    msg.is_ban_notification as i32,
                    timestamp_str,
                    i as i32,
                    msg.thought_kind.as_str(),
                    msg.kind.as_str(),
                ],
            )?;
        }

        // Long memory (v1.20): one row per persona recap, cascading with the discussion
        if inserted {
            for (i, r) in request.recaps.iter().enumerate() {
                if r.profile_id.trim().is_empty() || r.recap.is_empty() {
                    continue;
                }
                let recap_json = serde_json::to_string(&r.recap).unwrap_or_else(|_| "{}".to_string());
                tx.execute(
                    "INSERT INTO persona_memories (id, profile_id, discussion_id, topic, created_at, recap_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(id) DO NOTHING",
                    rusqlite::params![format!("{}-{}", request.id, i), r.profile_id, request.id, request.topic, request.created_at, recap_json],
                )?;
            }
        }

        // Full-text index (v1.20): the discussion, its synthesis and every message
        if inserted && schema::fts_available(&tx) {
            let content = request.messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>().join(" ");
            tx.execute(
                "INSERT INTO discussions_fts (discussion_id, topic, synthesis, content) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![request.id, request.topic, request.synthesis, content],
            )?;
        }

        tx.commit()?;
        Ok(())
    })
    .await
}

/// Columns of a `DiscussionSummary` row (shared by the list and the search).
const SUMMARY_COLUMNS: &str = "id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_format, argument_map_md, llm_provider, usage_json, estimated_cost_usd, tags, favorite";
/// How many columns `SUMMARY_COLUMNS` selects (extra columns of a query start after them).
const SUMMARY_COLUMN_COUNT: usize = 16;
/// Position of `synthesis` in `SUMMARY_COLUMNS`.
const SUMMARY_SYNTHESIS_INDEX: usize = 6;

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<DiscussionSummary> {
    let participants_json: String = row.get(4)?;
    let synthesis: String = row.get(6)?;
    let argument_map_md: String = row.get::<_, String>(10).unwrap_or_default();
    let participants: Vec<ParticipantInfo> = serde_json::from_str(&participants_json).unwrap_or_default();
    let usage: UsageLedger = serde_json::from_str(&row.get::<_, String>(12).unwrap_or_default()).unwrap_or_default();
    Ok(DiscussionSummary {
        id: row.get(0)?,
        topic: row.get(1)?,
        discussion_language: row.get(2)?,
        model_name: row.get(3)?,
        participants,
        total_turns: row.get(5)?,
        has_synthesis: !synthesis.is_empty(),
        created_at: row.get(7)?,
        discussion_mode: row.get::<_, String>(8).unwrap_or_else(|_| "debate".to_string()),
        document_format: row.get::<_, String>(9).unwrap_or_else(|_| "none".to_string()),
        has_argument_map: !argument_map_md.is_empty(),
        llm_provider: row.get::<_, String>(11).unwrap_or_else(|_| "ollama".to_string()),
        total_tokens: usage.total.total_tokens(),
        estimated_cost_usd: row.get::<_, f64>(13).unwrap_or(0.0),
        tags: parse_tags(&row.get::<_, String>(14).unwrap_or_default()),
        favorite: row.get::<_, i64>(15).unwrap_or(0) != 0,
    })
}

/// Tags are stored as a JSON array of strings; anything else reads as no tags.
fn parse_tags(json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(json).unwrap_or_default()
}

pub async fn list_discussions(
    db: &Connection,
) -> Result<Vec<DiscussionSummary>, tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare(&format!("SELECT {SUMMARY_COLUMNS} FROM discussions ORDER BY created_at DESC"))?;
        let rows = stmt.query_map([], row_to_summary)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
}

/// FTS5 query from free text: every term becomes a quoted prefix match, all
/// terms required (`"auto"* "prod"*`). Empty when the text has no term.
pub fn fts_query(text: &str) -> String {
    text.split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Full-text search over topics, syntheses and messages (S41). Uses the FTS5
/// index when available, a `LIKE` scan over the same fields otherwise; an
/// empty query lists everything.
pub async fn search_discussions(db: &Connection, query: &str) -> Result<Vec<DiscussionSummary>, tokio_rusqlite::Error> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return list_discussions(db).await;
    }
    db.call(move |conn| {
        let use_fts = schema::fts_available(conn);
        let rows = if use_fts {
            let mut stmt = conn.prepare(&format!(
                "SELECT {SUMMARY_COLUMNS} FROM discussions
                 WHERE id IN (SELECT discussion_id FROM discussions_fts WHERE discussions_fts MATCH ?1)
                 ORDER BY created_at DESC"
            ))?;
            let matched = stmt.query_map(rusqlite::params![fts_query(&query)], row_to_summary);
            match matched {
                Ok(rows) => rows.collect::<Result<Vec<_>, _>>()?,
                // A query FTS5 rejects (odd punctuation) → same LIKE scan as without FTS
                Err(e) => {
                    tracing::warn!(error = %e, "FTS query rejected — LIKE fallback");
                    like_search(conn, &query)?
                }
            }
        } else {
            like_search(conn, &query)?
        };
        Ok(rows)
    })
    .await
}

/// Case-insensitive substring scan of every term over topic, synthesis and
/// messages — one query for all the rows (the messages joined per discussion),
/// the matching done in Rust so that accents and case follow Unicode rules.
fn like_search(conn: &rusqlite::Connection, query: &str) -> rusqlite::Result<Vec<DiscussionSummary>> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut stmt = conn.prepare(&format!(
        "SELECT {SUMMARY_COLUMNS}, COALESCE((SELECT group_concat(m.content, ' ') FROM discussion_messages m WHERE m.discussion_id = d.id), '')
         FROM discussions d ORDER BY created_at DESC"
    ))?;
    let rows = stmt.query_map([], |row| {
        let summary = row_to_summary(row)?;
        let synthesis: String = row.get(SUMMARY_SYNTHESIS_INDEX)?;
        let messages: String = row.get(SUMMARY_COLUMN_COUNT)?;
        let haystack = format!("{} {} {}", summary.topic, synthesis, messages).to_lowercase();
        Ok((summary, haystack))
    })?;
    let mut kept = Vec::new();
    for row in rows {
        let (summary, haystack) = row?;
        if terms.iter().all(|t| haystack.contains(t.as_str())) {
            kept.push(summary);
        }
    }
    Ok(kept)
}

/// Tags as stored: trimmed, lower-cased, bounded in length, deduplicated, at
/// most `HISTORY_TAGS_MAX` of them (the UI applies the same rules; the server
/// never trusts it).
pub fn normalise_tags(tags: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in tags {
        let tag: String = raw.trim().trim_start_matches('#').to_lowercase().chars().take(constants::HISTORY_TAG_MAX_CHARS).collect();
        if tag.is_empty() || out.contains(&tag) {
            continue;
        }
        out.push(tag);
        if out.len() >= constants::HISTORY_TAGS_MAX {
            break;
        }
    }
    out
}

/// Replace the tags of a discussion (S42). Unknown id → no row updated (Ok).
pub async fn set_discussion_tags(db: &Connection, id: &str, tags: Vec<String>) -> Result<(), tokio_rusqlite::Error> {
    let id = id.to_string();
    let tags = normalise_tags(tags);
    db.call(move |conn| {
        let json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());
        conn.execute("UPDATE discussions SET tags = ?2 WHERE id = ?1", rusqlite::params![id, json])?;
        Ok(())
    })
    .await
}

pub async fn set_discussion_favorite(db: &Connection, id: &str, favorite: bool) -> Result<(), tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        conn.execute("UPDATE discussions SET favorite = ?2 WHERE id = ?1", rusqlite::params![id, favorite as i32])?;
        Ok(())
    })
    .await
}

pub async fn get_discussion(
    db: &Connection,
    id: &str,
) -> Result<Option<DiscussionDetail>, tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        // Fetch discussion metadata
        let disc = conn
            .prepare(
                "SELECT id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_content, document_format, argument_map_md, argument_map_md_by_speaker, llm_provider, usage_json, estimated_cost_usd, argument_map_json, report_json, tags, favorite
                 FROM discussions WHERE id = ?1",
            )?
            .query_row(rusqlite::params![id], |row| {
                let participants_json: String = row.get(4)?;
                let participants: Vec<ParticipantInfo> =
                    serde_json::from_str(&participants_json).unwrap_or_default();
                let usage: UsageLedger = serde_json::from_str(&row.get::<_, String>(14).unwrap_or_default()).unwrap_or_default();
                Ok(DiscussionDetail {
                    id: row.get(0)?,
                    topic: row.get(1)?,
                    discussion_language: row.get(2)?,
                    model_name: row.get(3)?,
                    participants,
                    total_turns: row.get(5)?,
                    synthesis: row.get(6)?,
                    created_at: row.get(7)?,
                    messages: Vec::new(), // filled below
                    discussion_mode: row.get::<_, String>(8).unwrap_or_else(|_| "debate".to_string()),
                    document_content: row.get::<_, String>(9).unwrap_or_else(|_| String::new()),
                    document_format: row.get::<_, String>(10).unwrap_or_else(|_| "none".to_string()),
                    argument_map_md: row.get::<_, String>(11).unwrap_or_default(),
                    argument_map_md_by_speaker: row.get::<_, String>(12).unwrap_or_default(),
                    argument_map_json: row.get::<_, String>(16).unwrap_or_default(),
                    llm_provider: row.get::<_, String>(13).unwrap_or_else(|_| "ollama".to_string()),
                    usage,
                    estimated_cost_usd: row.get::<_, f64>(15).unwrap_or(0.0),
                    report_json: row.get::<_, String>(17).unwrap_or_default(),
                    tags: parse_tags(&row.get::<_, String>(18).unwrap_or_default()),
                    favorite: row.get::<_, i64>(19).unwrap_or(0) != 0,
                })
            })
            .optional()?;

        let Some(mut detail) = disc else {
            return Ok(None);
        };

        // Fetch messages
        let mut stmt = conn.prepare(
            "SELECT id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp, thought_kind, kind
             FROM discussion_messages WHERE discussion_id = ?1 ORDER BY sort_order",
        )?;
        let messages = stmt
            .query_map(rusqlite::params![id], |row| {
                let role_str: String = row.get(5)?;
                let role: SpeakerRole =
                    serde_json::from_str(&format!("\"{}\"", role_str))
                        .unwrap_or(SpeakerRole::Gladiateur);
                let reactions_json: String = row.get(8)?;
                let reactions: Vec<Reaction> =
                    serde_json::from_str(&reactions_json).unwrap_or_default();
                let timestamp_str: String = row.get(10)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now());

                let is_ban_notification = row.get::<_, i32>(9)? != 0;
                // Rows written before v1.17 have no kind: derive it from the ban flag
                let kind = match MessageKind::parse(&row.get::<_, String>(12).unwrap_or_default()) {
                    MessageKind::Normal if is_ban_notification => MessageKind::BanNotification,
                    k => k,
                };
                Ok(Message {
                    id: row.get(0)?,
                    discussion_id: row.get(1)?,
                    turn_number: row.get(2)?,
                    speaker_id: row.get(3)?,
                    speaker_name: row.get(4)?,
                    role,
                    content: row.get(6)?,
                    inner_thought: row.get(7)?,
                    thought_kind: ThoughtKind::parse(&row.get::<_, String>(11).unwrap_or_default()),
                    reactions,
                    is_ban_notification,
                    kind,
                    timestamp,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        detail.messages = messages;
        Ok(Some(detail))
    })
    .await
}

pub async fn delete_discussion(
    db: &Connection,
    id: &str,
) -> Result<(), tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        conn.execute("DELETE FROM discussions WHERE id = ?1", rusqlite::params![id])?;
        if schema::fts_available(conn) {
            conn.execute("DELETE FROM discussions_fts WHERE discussion_id = ?1", rusqlite::params![id])?;
        }
        Ok(())
    })
    .await
}

pub async fn delete_all_discussions(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute("DELETE FROM discussions", [])?;
        if schema::fts_available(conn) {
            conn.execute("DELETE FROM discussions_fts", [])?;
        }
        Ok(())
    })
    .await
}

// ── Long memory of the personas (v1.20) ─────────────────────────────

fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<PersonaMemory> {
    Ok(PersonaMemory {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        discussion_id: row.get(2)?,
        topic: row.get(3)?,
        created_at: row.get(4)?,
        recap: serde_json::from_str::<PersonaRecap>(&row.get::<_, String>(5)?).unwrap_or_default(),
    })
}

/// Every memory of a profile, newest first.
pub async fn list_persona_memories(db: &Connection, profile_id: &str) -> Result<Vec<PersonaMemory>, tokio_rusqlite::Error> {
    let profile_id = profile_id.to_string();
    db.call(move |conn| {
        let mut stmt = conn.prepare("SELECT id, profile_id, discussion_id, topic, created_at, recap_json FROM persona_memories WHERE profile_id = ?1 ORDER BY created_at DESC")?;
        let rows = stmt.query_map(rusqlite::params![profile_id], row_to_memory)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
}

/// The `limit` memories of a profile closest to `topic` (BM25 over the topic
/// and the recap text; the most recent ones when nothing matches).
pub async fn recall_persona_memories(db: &Connection, profile_id: &str, topic: &str, limit: usize) -> Result<Vec<PersonaMemory>, tokio_rusqlite::Error> {
    let all = list_persona_memories(db, profile_id).await?;
    Ok(rank_memories(all, topic, limit))
}

/// Pure ranking: BM25 of the topic against "topic + recap" (the closest first, even
/// when everything fits); ties and no-match fall back to recency.
pub fn rank_memories(memories: Vec<PersonaMemory>, topic: &str, limit: usize) -> Vec<PersonaMemory> {
    if memories.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut index = Bm25Index::new();
    for (i, m) in memories.iter().enumerate() {
        index.add_document(i, &tokenize(&format!("{} {}", m.topic, m.recap.as_text())));
    }
    // Short tokens (articles, prepositions) would match everything: the ranking ignores them
    let query: Vec<String> = tokenize(topic).into_iter().filter(|t| t.chars().count() >= constants::PERSONA_MEMORY_MIN_TOKEN_CHARS).collect();
    let scored = index.search(&query, memories.len());
    let mut order: Vec<usize> = scored.iter().filter(|(_, s)| *s > 0.0).map(|(i, _)| *i).collect();
    // Recency completes the list when the topic matches fewer memories than asked
    for i in 0..memories.len() {
        if order.len() >= limit {
            break;
        }
        if !order.contains(&i) {
            order.push(i);
        }
    }
    order.truncate(limit);
    let mut picked: Vec<Option<PersonaMemory>> = memories.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| picked[i].take()).collect()
}

pub async fn count_persona_memories(db: &Connection) -> Result<u32, tokio_rusqlite::Error> {
    db.call(|conn| {
        let n: u32 = conn.query_row("SELECT count(*) FROM persona_memories", [], |row| row.get(0))?;
        Ok(n)
    })
    .await
}

/// "Forget everything": every persona memory goes away (the discussions stay).
pub async fn delete_all_persona_memories(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute("DELETE FROM persona_memories", [])?;
        Ok(())
    })
    .await
}

// ── Discussion templates (v1.20) ────────────────────────────────────

pub async fn list_templates(db: &Connection) -> Result<Vec<DiscussionTemplate>, tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, config_json, builtin, created_at FROM discussion_templates ORDER BY builtin DESC, name")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DiscussionTemplate {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    config_json: row.get(2)?,
                    builtin: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
    .await
}

/// Insert or replace a user template (a builtin id is never overwritten by the user).
pub async fn save_template(db: &Connection, template: DiscussionTemplate) -> Result<(), tokio_rusqlite::Error> {
    db.call(move |conn| {
        conn.execute(
            "INSERT INTO discussion_templates (id, name, config_json, builtin, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name = ?2, config_json = ?3 WHERE builtin = 0",
            rusqlite::params![template.id, template.name, template.config_json, template.builtin as i32, template.created_at],
        )?;
        Ok(())
    })
    .await
}

/// Delete a user template; builtin ones stay (returns whether a row went away).
pub async fn delete_template(db: &Connection, id: &str) -> Result<bool, tokio_rusqlite::Error> {
    let id = id.to_string();
    db.call(move |conn| {
        let n = conn.execute("DELETE FROM discussion_templates WHERE id = ?1 AND builtin = 0", rusqlite::params![id])?;
        Ok(n > 0)
    })
    .await
}

// ── Tavily usage tracking ───────────────────────────────────────────

pub async fn get_tavily_usage(db: &Connection) -> Result<u32, tokio_rusqlite::Error> {
    db.call(|conn| {
        let count: u32 = conn
            .query_row(
                "SELECT COALESCE(CAST(value AS INTEGER), 0) FROM settings WHERE key = 'tavily_usage_count'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count)
    })
    .await
}

pub async fn increment_tavily_usage(db: &Connection) -> Result<u32, tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute(
            "UPDATE settings SET value = CAST(value AS INTEGER) + 1 WHERE key = 'tavily_usage_count'",
            [],
        )?;
        let count: u32 = conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'tavily_usage_count'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    })
    .await
}

pub async fn check_and_reset_tavily_period(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    let settings = get_settings(db).await?;

    if settings.tavily_api_key.is_empty() || settings.tavily_period_start.is_empty() {
        return Ok(());
    }

    let Some(period_start) = rolling_period::parse_period_start(&settings.tavily_period_start) else {
        return Ok(());
    };
    let today = chrono::Local::now().date_naive();
    let Some(rollover) = rolling_period::rollover(period_start, today) else {
        return Ok(()); // still within current period
    };

    // Archive the expired period
    let mut history: Vec<serde_json::Value> =
        serde_json::from_str(&settings.tavily_usage_history).unwrap_or_default();
    history.push(serde_json::json!({
        "periodStart": settings.tavily_period_start,
        "periodEnd": rollover.expired_end.format("%Y-%m-%d").to_string(),
        "usageCount": settings.tavily_usage_count,
    }));

    let new_settings = AppSettings {
        tavily_period_start: rollover.new_start.format("%Y-%m-%d").to_string(),
        tavily_usage_count: 0,
        tavily_usage_history: serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string()),
        ..settings
    };

    save_settings(db, &new_settings).await
}

// ── DeepSeek spend tracking (rolling monthly period) ────────────────

/// Roll the DeepSeek period over if it expired; archives the expired period.
/// No-op without an API key or a period start.
pub async fn check_and_reset_deepseek_period(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    let settings = get_settings(db).await?;
    if settings.deepseek_api_key.is_empty() {
        return Ok(());
    }
    let Some(period_start) = rolling_period::parse_period_start(&settings.deepseek_period_start) else {
        return Ok(());
    };
    let today = chrono::Local::now().date_naive();
    let Some(rollover) = rolling_period::rollover(period_start, today) else {
        return Ok(());
    };

    let expired: PeriodUsage = serde_json::from_str(&settings.deepseek_period_usage_json).unwrap_or_default();
    let mut history: Vec<PeriodHistoryEntry> =
        serde_json::from_str(&settings.deepseek_usage_history).unwrap_or_default();
    history.push(PeriodHistoryEntry {
        period_start: settings.deepseek_period_start.clone(),
        period_end: rollover.expired_end.format("%Y-%m-%d").to_string(),
        usage: expired,
    });

    let new_settings = AppSettings {
        deepseek_period_start: rollover.new_start.format("%Y-%m-%d").to_string(),
        deepseek_period_usage_json: "{}".to_string(),
        deepseek_usage_history: serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string()),
        ..settings
    };
    save_settings(db, &new_settings).await
}

/// Current DeepSeek period usage (zero when unset or unparsable).
pub async fn get_deepseek_period_usage(db: &Connection) -> Result<PeriodUsage, tokio_rusqlite::Error> {
    let settings = get_settings(db).await?;
    Ok(serde_json::from_str(&settings.deepseek_period_usage_json).unwrap_or_default())
}

/// Add one finished discussion's usage and estimated cost to the current period.
pub async fn record_deepseek_usage(
    db: &Connection,
    usage: &LlmUsage,
    cost_usd: f64,
) -> Result<PeriodUsage, tokio_rusqlite::Error> {
    let settings = get_settings(db).await?;
    let mut period: PeriodUsage = serde_json::from_str(&settings.deepseek_period_usage_json).unwrap_or_default();
    period.add_discussion(usage, cost_usd);
    let new_settings = AppSettings {
        deepseek_period_usage_json: serde_json::to_string(&period).unwrap_or_else(|_| "{}".to_string()),
        ..settings
    };
    save_settings(db, &new_settings).await?;
    Ok(period)
}

/// Manually reset the current DeepSeek period counters (user action).
pub async fn reset_deepseek_period_usage(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    let settings = get_settings(db).await?;
    let new_settings = AppSettings {
        deepseek_period_usage_json: "{}".to_string(),
        ..settings
    };
    save_settings(db, &new_settings).await
}

// ── License tracking ────────────────────────────────────────────────

/// Read license tracking data from settings KV store.
/// Returns (key_hash, discussions_count, last_check_timestamp).
pub async fn get_license_tracking(
    db: &Connection,
) -> Result<(String, u32, i64), tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare(
            "SELECT key, value FROM settings WHERE key IN ('license_key_hash', 'license_discussions_count', 'last_check_timestamp')",
        )?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut key_hash = String::new();
        let mut disc_count: u32 = 0;
        let mut last_check: i64 = 0;

        for (key, value) in rows {
            match key.as_str() {
                "license_key_hash" => key_hash = value,
                "license_discussions_count" => disc_count = value.parse().unwrap_or(0),
                "last_check_timestamp" => last_check = value.parse().unwrap_or(0),
                _ => {}
            }
        }

        Ok((key_hash, disc_count, last_check))
    })
    .await
}

/// Increment discussion counter. Handles key change implicitly:
/// if key_hash differs from stored hash → set count = 1 (new key),
/// if key_hash matches → increment count.
pub async fn increment_license_discussions(
    db: &Connection,
    key_hash: &str,
) -> Result<(), tokio_rusqlite::Error> {
    let key_hash = key_hash.to_string();
    db.call(move |conn| {
        // Read current stored hash + count
        let (stored_hash, count) = {
            let mut stmt = conn.prepare(
                "SELECT key, value FROM settings WHERE key IN ('license_key_hash', 'license_discussions_count')",
            )?;
            let rows: Vec<(String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;

            let mut hash = String::new();
            let mut cnt: u32 = 0;
            for (k, v) in rows {
                match k.as_str() {
                    "license_key_hash" => hash = v,
                    "license_discussions_count" => cnt = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
            (hash, cnt)
        };

        let new_count = if stored_hash == key_hash { count + 1 } else { 1 };
        let now = chrono::Utc::now().timestamp();

        let tx = conn.transaction()?;
        for (k, v) in [
            ("license_key_hash", key_hash.as_str()),
            ("license_discussions_count", &new_count.to_string()),
            ("last_check_timestamp", &now.to_string()),
        ] {
            tx.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                rusqlite::params![k, v],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
    .await
}

/// Update last check timestamp (called on every license validation).
pub async fn update_last_check_timestamp(
    db: &Connection,
) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        let now = chrono::Utc::now().timestamp().to_string();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('last_check_timestamp', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1",
            rusqlite::params![now],
        )?;
        Ok(())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use crate::models::argument_map::{ArgumentMap, ThesisNode};
    use crate::models::llm::LlmUsage;
    use crate::models::settings::AppSettings;

    /// A stale settings payload from the UI must not erase spend recorded by the
    /// engine in the meantime.
    #[tokio::test]
    async fn user_settings_save_keeps_server_owned_period_counters() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        let mut initial = AppSettings { username: "Léo".into(), deepseek_api_key: "sk-test".into(), ..Default::default() };
        save_user_settings(&db, &initial).await.unwrap();
        // Key present → the period start is initialised by the backend
        let after_key = get_settings(&db).await.unwrap();
        assert!(!after_key.deepseek_period_start.is_empty());

        // The engine records a discussion
        let usage = LlmUsage { prompt_tokens: 1000, cached_tokens: 100, completion_tokens: 200, reasoning_tokens: 50 };
        let period = record_deepseek_usage(&db, &usage, 0.42).await.unwrap();
        assert_eq!(period.discussions, 1);

        // The UI saves with the payload it hydrated BEFORE the discussion
        initial.theme = "light".into();
        save_user_settings(&db, &initial).await.unwrap();
        let after = get_settings(&db).await.unwrap();
        assert_eq!(after.theme, "light", "user fields are written");
        let kept = get_deepseek_period_usage(&db).await.unwrap();
        assert_eq!(kept.discussions, 1, "server-owned spend must survive a stale UI save");
        assert!((kept.cost_usd - 0.42).abs() < 1e-9);
        assert_eq!(after.deepseek_period_start, after_key.deepseek_period_start);

        // Explicit reset is still a backend operation
        reset_deepseek_period_usage(&db).await.unwrap();
        assert_eq!(get_deepseek_period_usage(&db).await.unwrap().discussions, 0);
    }

    /// Save → list → get round trip, including the v1.16 columns
    /// (provider, usage, cost, structured argument map).
    #[tokio::test]
    async fn discussion_round_trip_keeps_v116_columns() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();

        let map = ArgumentMap {
            theses: vec![ThesisNode {
                id: "t-0".into(),
                label: "Thèse".into(),
                speaker_id: "g1".into(),
                speaker_name: "Alice".into(),
                arguments: vec![],
            }],
        };
        let request: SaveDiscussionRequest = serde_json::from_value(serde_json::json!({
            "id": "disc-1",
            "topic": "Sujet",
            "discussionLanguage": "fr",
            "modelName": "deepseek · deepseek-flash",
            "participants": [{"id": "g1", "name": "Alice", "role": "GladIAteur", "emoji": "🦉"}],
            "totalTurns": 2,
            "synthesis": "Synthèse",
            "createdAt": "2026-09-16T10:00:00Z",
            "messages": [],
            "argumentMapMd": "# Sujet",
            "argumentMapJson": serde_json::to_string(&map).unwrap(),
            "llmProvider": "deepseek",
            "usage": {"total": {"promptTokens": 10, "cachedTokens": 0, "completionTokens": 5, "reasoningTokens": 0}, "calls": 1},
            "estimatedCostUsd": 0.0123,
            "reportJson": "{\"version\":1}",
            "messages": [
                {"id":"m1","discussionId":"disc-1","turnNumber":1,"speakerId":"g1","speakerName":"Alice","role":"GladIAteur","content":"c","innerThought":null,"reactions":[],"isBanNotification":false,"timestamp":"2026-09-16T10:00:00Z"},
                {"id":"m2","discussionId":"disc-1","turnNumber":1,"speakerId":"arb","speakerName":"Arb","role":"IArbitre","content":"ban","innerThought":null,"reactions":[],"isBanNotification":true,"timestamp":"2026-09-16T10:00:01Z"},
                {"id":"m3","discussionId":"disc-1","turnNumber":1,"speakerId":"g1","speakerName":"Alice","role":"GladIAteur","content":"repose ses notes","innerThought":null,"reactions":[],"isBanNotification":false,"kind":"stageDirection","timestamp":"2026-09-16T10:00:02Z"}
            ]
        }))
        .unwrap();
        save_discussion(&db, request.clone()).await.unwrap();
        // Idempotent (ON CONFLICT DO NOTHING)
        save_discussion(&db, request).await.unwrap();

        let list = list_discussions(&db).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].has_argument_map);
        assert_eq!(list[0].llm_provider, "deepseek");

        let detail = get_discussion(&db, "disc-1").await.unwrap().expect("saved discussion");
        assert_eq!(detail.llm_provider, "deepseek");
        assert_eq!(detail.usage.total.prompt_tokens, 10);
        assert!((detail.estimated_cost_usd - 0.0123).abs() < 1e-9);
        let restored: ArgumentMap = serde_json::from_str(&detail.argument_map_json).unwrap();
        assert_eq!(restored.theses[0].label, "Thèse");
        assert_eq!(detail.report_json, "{\"version\":1}");
        // Message kinds: explicit kind kept, v1.16 ban flag derived into a kind, normal by default
        let kinds: Vec<MessageKind> = detail.messages.iter().map(|m| m.kind).collect();
        assert_eq!(kinds, vec![MessageKind::Normal, MessageKind::BanNotification, MessageKind::StageDirection]);
        assert!(detail.messages[1].is_ban_notification);
        assert!(get_discussion(&db, "missing").await.unwrap().is_none());
    }

    fn request(id: &str, topic: &str, synthesis: &str, message: &str) -> SaveDiscussionRequest {
        serde_json::from_value(serde_json::json!({
            "id": id, "topic": topic, "discussionLanguage": "fr", "modelName": "m", "participants": [], "totalTurns": 1,
            "synthesis": synthesis, "createdAt": format!("2026-09-16T10:00:0{}Z", id.chars().last().filter(char::is_ascii_digit).unwrap_or('0')), "llmProvider": "ollama", "usage": {}, "estimatedCostUsd": 0.0,
            "messages": [{"id": format!("{id}-m1"), "discussionId": id, "turnNumber": 1, "speakerId": "g1", "speakerName": "Alice", "role": "GladIAteur", "content": message, "innerThought": null, "reactions": [], "isBanNotification": false, "timestamp": "2026-09-16T10:00:00Z"}]
        }))
        .unwrap()
    }

    /// S41 — the full-text search finds a discussion by its synthesis or a message,
    /// every term is required, the query is sanitised, and the `LIKE` scan gives
    /// the same answers (the index is the fast path, not the source of truth).
    #[tokio::test]
    async fn full_text_search_finds_by_synthesis_and_messages_with_like_fallback() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        save_discussion(&db, request("d1", "Le télétravail", "L'automatisation des tâches a dominé la synthèse.", "Bonjour")).await.unwrap();
        save_discussion(&db, request("d2", "Cuisine", "Recettes de saison", "Le message parle de productivité et d'automatisation")).await.unwrap();
        save_discussion(&db, request("d3", "Jardinage", "Semis", "Rien à voir")).await.unwrap();
        let fts_on = db.call(|conn| Ok(schema::fts_available(conn))).await.unwrap();
        assert!(fts_on, "the bundled SQLite carries FTS5 (spike H-P5)");

        let ids = |rows: Vec<DiscussionSummary>| rows.into_iter().map(|d| d.id).collect::<Vec<_>>();
        assert_eq!(ids(search_discussions(&db, "automatisation").await.unwrap()), vec!["d2", "d1"]);
        assert_eq!(ids(search_discussions(&db, "automatisation productivité").await.unwrap()), vec!["d2"]);
        assert_eq!(ids(search_discussions(&db, "télétravail").await.unwrap()), vec!["d1"]);
        assert!(search_discussions(&db, "inexistant").await.unwrap().is_empty());
        assert_eq!(search_discussions(&db, "   ").await.unwrap().len(), 3, "empty query lists everything");
        // Odd punctuation never breaks the query
        search_discussions(&db, "auto\"mat*( -x").await.expect("odd punctuation never errors");
        assert_eq!(fts_query("auto \"quoted\""), "\"auto\"* \"\"\"quoted\"\"\"*");
        // The LIKE scan agrees with the index
        let like = db.call(|conn| Ok(like_search(conn, "automatisation productivité").unwrap())).await.unwrap();
        assert_eq!(ids(like), vec!["d2"]);
        // Deleting a discussion removes it from the index
        delete_discussion(&db, "d1").await.unwrap();
        assert_eq!(ids(search_discussions(&db, "automatisation").await.unwrap()), vec!["d2"]);
        let indexed: i64 = db.call(|conn| Ok(conn.query_row("SELECT count(*) FROM discussions_fts", [], |r| r.get(0)).unwrap())).await.unwrap();
        assert_eq!(indexed, 2);
        delete_all_discussions(&db).await.unwrap();
        let indexed: i64 = db.call(|conn| Ok(conn.query_row("SELECT count(*) FROM discussions_fts", [], |r| r.get(0)).unwrap())).await.unwrap();
        assert_eq!(indexed, 0);
    }

    /// A database written by v1.16 (no report, kind, tags, favourite, memories,
    /// templates or index) migrates in place: the old rows list, search and read
    /// back with the derived message kind, and the new tables exist.
    #[tokio::test]
    async fn v116_database_migrates_in_place() {
        let db = Connection::open_in_memory().await.unwrap();
        db.call(|conn| {
            conn.execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE predefined_profiles (id TEXT PRIMARY KEY, name TEXT NOT NULL, personality TEXT NOT NULL, system_prompt TEXT NOT NULL, is_builtin INTEGER NOT NULL DEFAULT 1, profile_type TEXT NOT NULL DEFAULT 'gladiateur', category TEXT NOT NULL DEFAULT 'autres', initial_emotions TEXT);
                 CREATE TABLE discussions (id TEXT PRIMARY KEY, topic TEXT NOT NULL, discussion_language TEXT NOT NULL DEFAULT 'fr', model_name TEXT NOT NULL DEFAULT '', participants_json TEXT NOT NULL DEFAULT '[]', total_turns INTEGER NOT NULL DEFAULT 0, synthesis TEXT NOT NULL DEFAULT '', created_at TEXT NOT NULL DEFAULT '', discussion_mode TEXT NOT NULL DEFAULT 'debate', document_content TEXT NOT NULL DEFAULT '', document_format TEXT NOT NULL DEFAULT 'none', argument_map_md TEXT NOT NULL DEFAULT '', argument_map_md_by_speaker TEXT NOT NULL DEFAULT '', llm_provider TEXT NOT NULL DEFAULT 'ollama', usage_json TEXT NOT NULL DEFAULT '{}', estimated_cost_usd REAL NOT NULL DEFAULT 0, argument_map_json TEXT NOT NULL DEFAULT '');
                 CREATE TABLE discussion_messages (id TEXT PRIMARY KEY, discussion_id TEXT NOT NULL, turn_number INTEGER NOT NULL DEFAULT 0, speaker_id TEXT NOT NULL, speaker_name TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL DEFAULT '', inner_thought TEXT, reactions_json TEXT NOT NULL DEFAULT '[]', is_ban_notification INTEGER NOT NULL DEFAULT 0, timestamp TEXT NOT NULL DEFAULT '', sort_order INTEGER NOT NULL DEFAULT 0, thought_kind TEXT NOT NULL DEFAULT 'persona', FOREIGN KEY (discussion_id) REFERENCES discussions(id) ON DELETE CASCADE);
                 INSERT INTO discussions (id, topic, synthesis, created_at, total_turns) VALUES ('old', 'Un vieux sujet', 'Une synthèse sur la robotique', '2026-01-01T00:00:00Z', 1);
                 INSERT INTO discussion_messages (id, discussion_id, turn_number, speaker_id, speaker_name, role, content, is_ban_notification, sort_order) VALUES ('m1', 'old', 1, 'g1', 'Alice', 'GladIAteur', 'Bonjour', 0, 0), ('m2', 'old', 1, 'arb', 'Arb', 'IArbitre', 'Exclusion', 1, 1);",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        schema::initialize(&db).await.unwrap();
        schema::initialize(&db).await.unwrap(); // idempotent

        let list = list_discussions(&db).await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].tags.is_empty() && !list[0].favorite);
        let detail = get_discussion(&db, "old").await.unwrap().unwrap();
        assert_eq!(detail.report_json, "");
        assert_eq!(detail.messages.iter().map(|m| m.kind).collect::<Vec<_>>(), vec![MessageKind::Normal, MessageKind::BanNotification]);
        // The old discussion is searchable (index backfilled at startup)
        assert_eq!(search_discussions(&db, "robotique").await.unwrap().len(), 1);
        assert_eq!(search_discussions(&db, "bonjour").await.unwrap().len(), 1);
        // New tables are usable; tags and favourite write on the old row
        assert_eq!(count_persona_memories(&db).await.unwrap(), 0);
        assert!(list_templates(&db).await.unwrap().is_empty());
        set_discussion_tags(&db, "old", vec!["archive".into()]).await.unwrap();
        set_discussion_favorite(&db, "old", true).await.unwrap();
        let list = list_discussions(&db).await.unwrap();
        assert_eq!(list[0].tags, vec!["archive"]);
        assert!(list[0].favorite);
    }

    /// The index is backfilled for discussions saved before it existed.
    #[tokio::test]
    async fn fts_backfill_indexes_pre_existing_discussions() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        save_discussion(&db, request("old", "Ancien sujet", "Synthèse ancienne", "contenu")).await.unwrap();
        // Simulate a discussion written before the index existed
        db.call(|conn| { conn.execute("DELETE FROM discussions_fts", [])?; Ok(()) }).await.unwrap();
        assert!(search_discussions(&db, "ancienne").await.unwrap().is_empty());
        let n = db.call(|conn| Ok(schema::backfill_fts(conn).unwrap())).await.unwrap();
        assert_eq!(n, 1);
        assert_eq!(search_discussions(&db, "ancienne").await.unwrap().len(), 1);
        assert_eq!(db.call(|conn| Ok(schema::backfill_fts(conn).unwrap())).await.unwrap(), 0, "idempotent");
    }

    /// S42 — tags and favourite are updated in place and read back; deleting the
    /// discussion takes them away (they live on the row).
    #[tokio::test]
    async fn tags_and_favourite_round_trip() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        save_discussion(&db, request("d1", "Sujet", "s", "m")).await.unwrap();
        set_discussion_tags(&db, "d1", vec![" IA ".into(), String::new(), "travail".into(), "ia".into()]).await.unwrap();
        set_discussion_favorite(&db, "d1", true).await.unwrap();
        let list = list_discussions(&db).await.unwrap();
        assert_eq!(list[0].tags, vec!["ia", "travail"], "trimmed, lower-cased, deduplicated");
        // Bounded: too many tags are dropped, a long tag is cut on a char boundary
        let many: Vec<String> = (0..constants::HISTORY_TAGS_MAX + 5).map(|i| format!("t{i}")).collect();
        set_discussion_tags(&db, "d1", many).await.unwrap();
        assert_eq!(list_discussions(&db).await.unwrap()[0].tags.len(), constants::HISTORY_TAGS_MAX);
        set_discussion_tags(&db, "d1", vec!["é".repeat(constants::HISTORY_TAG_MAX_CHARS + 10)]).await.unwrap();
        assert_eq!(list_discussions(&db).await.unwrap()[0].tags[0].chars().count(), constants::HISTORY_TAG_MAX_CHARS);
        set_discussion_tags(&db, "d1", vec!["ia".into(), "travail".into()]).await.unwrap();
        assert!(list[0].favorite);
        let detail = get_discussion(&db, "d1").await.unwrap().unwrap();
        assert_eq!(detail.tags, vec!["ia", "travail"]);
        assert!(detail.favorite);
        set_discussion_favorite(&db, "d1", false).await.unwrap();
        assert!(!get_discussion(&db, "d1").await.unwrap().unwrap().favorite);
        // Unknown id: no error, nothing written
        set_discussion_tags(&db, "ghost", vec!["x".into()]).await.unwrap();
        delete_discussion(&db, "d1").await.unwrap();
        assert!(list_discussions(&db).await.unwrap().is_empty());
    }

    /// S26 — memories: two past discussions of a profile are recalled ranked by topic
    /// (BM25), recency completes the list, deleting the discussion deletes them (cascade),
    /// "forget everything" empties the table; only recaps with a profile and content are kept.
    #[tokio::test]
    async fn persona_memories_are_saved_recalled_by_topic_and_forgotten() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        let with_recap = |id: &str, topic: &str, positions: &[&str], lesson: &str| -> SaveDiscussionRequest {
            let mut req = request(id, topic, "s", "m");
            req.recaps = vec![
                serde_json::from_value(serde_json::json!({ "speakerId": "g1", "speakerName": "Le Scientifique", "profileId": "scientist", "recap": { "positions": positions, "bestLines": ["ligne"], "allies": [], "rivals": ["Le Philosophe"], "lesson": lesson } })).unwrap(),
                // No profile → not stored; empty recap → not stored
                serde_json::from_value(serde_json::json!({ "speakerId": "g2", "speakerName": "Perso", "profileId": "", "recap": { "lesson": "x" } })).unwrap(),
                serde_json::from_value(serde_json::json!({ "speakerId": "g3", "speakerName": "Vide", "profileId": "philosopher", "recap": {} })).unwrap(),
            ];
            req
        };
        save_discussion(&db, with_recap("d1", "Le télétravail et la productivité", &["le télétravail isole"], "mesurer avant de juger")).await.unwrap();
        save_discussion(&db, with_recap("d2", "La cuisine de saison", &["les légumes d'abord"], "goûter")).await.unwrap();
        save_discussion(&db, with_recap("d3", "L'automatisation des usines", &["automatiser sans licencier"], "former")).await.unwrap();
        assert_eq!(count_persona_memories(&db).await.unwrap(), 3);
        assert!(list_persona_memories(&db, "philosopher").await.unwrap().is_empty());

        // Topic ranking: the productivity discussion first, then recency completes
        let recalled = recall_persona_memories(&db, "scientist", "La productivité en télétravail", 2).await.unwrap();
        assert_eq!(recalled.iter().map(|m| m.discussion_id.as_str()).collect::<Vec<_>>(), vec!["d1", "d3"]);
        assert_eq!(recalled[0].recap.positions, vec!["le télétravail isole"]);
        assert_eq!(recalled[0].recap.rivals, vec!["Le Philosophe"]);
        // No match at all → the most recent ones
        let recent = recall_persona_memories(&db, "scientist", "zzz", 2).await.unwrap();
        assert_eq!(recent.iter().map(|m| m.discussion_id.as_str()).collect::<Vec<_>>(), vec!["d3", "d2"]);
        // Fewer memories than asked → all of them, newest first
        assert_eq!(recall_persona_memories(&db, "scientist", "x", 10).await.unwrap().len(), 3);

        // Cascade with the discussion, then forget everything
        delete_discussion(&db, "d1").await.unwrap();
        assert_eq!(count_persona_memories(&db).await.unwrap(), 2);
        delete_all_persona_memories(&db).await.unwrap();
        assert_eq!(count_persona_memories(&db).await.unwrap(), 0);
        assert_eq!(list_discussions(&db).await.unwrap().len(), 2, "discussions stay");
    }

    /// S40 — templates: builtin ones are seeded and protected, user ones are saved, replaced and deleted.
    #[tokio::test]
    async fn templates_are_seeded_saved_and_protected() {
        let db = Connection::open_in_memory().await.unwrap();
        schema::initialize(&db).await.unwrap();
        crate::db::seed::seed_templates(&db).await.unwrap();
        crate::db::seed::seed_templates(&db).await.unwrap(); // idempotent
        let builtin = list_templates(&db).await.unwrap();
        assert_eq!(builtin.len(), 4);
        assert!(builtin.iter().all(|t| t.builtin && t.validate().is_ok()));
        // Every seeded profile id exists in the catalogue
        let profiles: Vec<String> = crate::db::seed::builtin_profiles().into_iter().chain(crate::db::seed::builtin_arbitre_profiles()).map(|p| p.id).collect();
        for t in &builtin {
            let cfg: serde_json::Value = serde_json::from_str(&t.config_json).unwrap();
            assert!(profiles.contains(&cfg["arbitreProfileId"].as_str().unwrap().to_string()), "{}", t.id);
            for g in cfg["gladiateurs"].as_array().unwrap() {
                assert!(profiles.contains(&g["profileId"].as_str().unwrap().to_string()), "{}: {}", t.id, g);
            }
        }
        // A user template: saved, replaced on the same id, deleted
        let mine = DiscussionTemplate { id: "tpl-mine".into(), name: "Le mien".into(), config_json: r#"{"topic":"x"}"#.into(), builtin: false, created_at: "2026-09-16T10:00:00Z".into() };
        save_template(&db, mine.clone()).await.unwrap();
        save_template(&db, DiscussionTemplate { name: "Renommé".into(), ..mine.clone() }).await.unwrap();
        let all = list_templates(&db).await.unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all.iter().find(|t| t.id == "tpl-mine").unwrap().name, "Renommé");
        assert!(all.iter().position(|t| t.builtin).unwrap() < all.iter().position(|t| !t.builtin).unwrap(), "builtin first");
        // A builtin id cannot be overwritten nor deleted by the user
        save_template(&db, DiscussionTemplate { id: "tpl-trial-of-an-idea".into(), name: "Hack".into(), ..mine.clone() }).await.unwrap();
        assert_ne!(list_templates(&db).await.unwrap().iter().find(|t| t.id == "tpl-trial-of-an-idea").unwrap().name, "Hack");
        assert!(!delete_template(&db, "tpl-trial-of-an-idea").await.unwrap());
        assert!(delete_template(&db, "tpl-mine").await.unwrap());
        assert_eq!(list_templates(&db).await.unwrap().len(), 4);
    }
}
