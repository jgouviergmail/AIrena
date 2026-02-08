use tokio_rusqlite::Connection;

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
const PROFILE_COLUMNS: &str = "id, name, personality, system_prompt, is_builtin";

/// Map a database row to PredefinedProfile
fn row_to_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<PredefinedProfile> {
    Ok(PredefinedProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        personality: row.get(2)?,
        system_prompt: row.get(3)?,
        is_builtin: row.get::<_, i32>(4)? != 0,
    })
}

pub async fn list_profiles(
    db: &Connection,
) -> Result<Vec<PredefinedProfile>, tokio_rusqlite::Error> {
    db.call(|conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {PROFILE_COLUMNS} FROM predefined_profiles ORDER BY name"
        ))?;
        let profiles = stmt
            .query_map([], row_to_profile)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(profiles)
    })
    .await
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
            "INSERT INTO predefined_profiles (id, name, personality, system_prompt, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name = ?2, personality = ?3, system_prompt = ?4, is_builtin = ?5",
            rusqlite::params![
                profile.id,
                profile.name,
                profile.personality,
                profile.system_prompt,
                profile.is_builtin as i32,
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
