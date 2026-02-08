use chrono::DateTime;
use tokio_rusqlite::Connection;

use crate::models::history::{
    DiscussionDetail, DiscussionSummary, ParticipantInfo, SaveDiscussionRequest,
};
use crate::models::message::{Message, Reaction, SpeakerRole};
use crate::models::profile::PredefinedProfile;
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
                _ => {}
            }
        }
        Ok(settings)
    })
    .await
}

pub async fn save_settings(
    db: &Connection,
    settings: &AppSettings,
) -> Result<(), tokio_rusqlite::Error> {
    let settings = settings.clone();
    db.call(move |conn| {
        let tx = conn.transaction()?;
        let pairs: Vec<(&str, String)> = vec![
            ("username", settings.username.clone()),
            ("language", settings.language.clone()),
            ("theme", settings.theme.clone()),
            ("ollama_url", settings.ollama_url.clone()),
            ("ollama_model", settings.ollama_model.clone()),
            ("emotion_driven", settings.emotion_driven.to_string()),
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
const PROFILE_COLUMNS: &str = "id, name, personality, system_prompt, is_builtin, profile_type, category";

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
            "INSERT INTO predefined_profiles (id, name, personality, system_prompt, is_builtin, profile_type, category)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET name = ?2, personality = ?3, system_prompt = ?4, is_builtin = ?5, profile_type = ?6, category = ?7",
            rusqlite::params![
                profile.id,
                profile.name,
                profile.personality,
                profile.system_prompt,
                profile.is_builtin as i32,
                profile.profile_type,
                profile.category,
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

        tx.execute(
            "INSERT INTO discussions (id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
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
                "INSERT INTO discussion_messages (id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
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
            "SELECT id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at
             FROM discussions ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let participants_json: String = row.get(4)?;
                let synthesis: String = row.get(6)?;
                let participants: Vec<ParticipantInfo> =
                    serde_json::from_str(&participants_json).unwrap_or_default();
                Ok(DiscussionSummary {
                    id: row.get(0)?,
                    topic: row.get(1)?,
                    discussion_language: row.get(2)?,
                    model_name: row.get(3)?,
                    participants,
                    total_turns: row.get(5)?,
                    has_synthesis: !synthesis.is_empty(),
                    created_at: row.get(7)?,
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
                "SELECT id, topic, discussion_language, model_name, participants_json, total_turns, synthesis, created_at
                 FROM discussions WHERE id = ?1",
            )?
            .query_row(rusqlite::params![id], |row| {
                let participants_json: String = row.get(4)?;
                let participants: Vec<ParticipantInfo> =
                    serde_json::from_str(&participants_json).unwrap_or_default();
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
                })
            })
            .optional()?;

        let Some(mut detail) = disc else {
            return Ok(None);
        };

        // Fetch messages
        let mut stmt = conn.prepare(
            "SELECT id, discussion_id, turn_number, speaker_id, speaker_name, role, content, inner_thought, reactions_json, is_ban_notification, timestamp
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
