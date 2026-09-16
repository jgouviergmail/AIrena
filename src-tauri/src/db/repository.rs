use chrono::DateTime;
use tokio_rusqlite::Connection;

use super::rolling_period;
use crate::models::history::{
    DiscussionDetail, DiscussionSummary, ParticipantInfo, SaveDiscussionRequest,
};
use crate::models::llm::{LlmUsage, PeriodHistoryEntry, PeriodUsage, ProviderKind, ReasoningLevel, UsageLedger};
use crate::models::message::{Message, Reaction, SpeakerRole, ThoughtKind};
use crate::models::profile::PredefinedProfile;
use crate::constants;
use crate::models::settings::AppSettings;

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
            ("deepseek_api_key", settings.deepseek_api_key.clone()),
            ("deepseek_model", settings.deepseek_model.clone()),
            ("deepseek_monthly_budget_usd", settings.deepseek_monthly_budget_usd.to_string()),
            ("deepseek_period_start", settings.deepseek_period_start.clone()),
            ("deepseek_period_usage_json", settings.deepseek_period_usage_json.clone()),
            ("deepseek_usage_history", settings.deepseek_usage_history.clone()),
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
            "INSERT INTO discussions (id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_content, document_format, argument_map_md, argument_map_md_by_speaker, llm_provider, usage_json, estimated_cost_usd, argument_map_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
            ],
        )?;

        for (i, msg) in request.messages.iter().enumerate() {
            let reactions_json = serde_json::to_string(&msg.reactions)
                .unwrap_or_else(|_| "[]".to_string());
            let role_str = serde_json::to_string(&msg.role)
                .unwrap_or_else(|_| "\"GladIAteur\"".to_string());
            // Remove surrounding quotes from serialized role string
            let role_str = role_str.trim_matches('"');
            let timestamp_str = msg.timestamp.to_rfc3339();

            tx.execute(
                "INSERT INTO discussion_messages (id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp, sort_order, thought_kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
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
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    })
    .await
}

pub async fn list_discussions(
    db: &Connection,
) -> Result<Vec<DiscussionSummary>, tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_format, argument_map_md, llm_provider, usage_json, estimated_cost_usd
             FROM discussions ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let participants_json: String = row.get(4)?;
                let synthesis: String = row.get(6)?;
                let argument_map_md: String = row.get::<_, String>(10).unwrap_or_default();
                let participants: Vec<ParticipantInfo> =
                    serde_json::from_str(&participants_json).unwrap_or_default();
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
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
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
                "SELECT id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at, discussion_mode, document_content, document_format, argument_map_md, argument_map_md_by_speaker, llm_provider, usage_json, estimated_cost_usd, argument_map_json
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
                })
            })
            .optional()?;

        let Some(mut detail) = disc else {
            return Ok(None);
        };

        // Fetch messages
        let mut stmt = conn.prepare(
            "SELECT id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp, thought_kind
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
                    is_ban_notification: row.get::<_, i32>(9)? != 0,
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
        Ok(())
    })
    .await
}

pub async fn delete_all_discussions(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute("DELETE FROM discussions", [])?;
        Ok(())
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
            "estimatedCostUsd": 0.0123
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
        assert!(get_discussion(&db, "missing").await.unwrap().is_none());
    }
}
