use tokio_rusqlite::Connection;

const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS predefined_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    personality TEXT NOT NULL,
    system_prompt TEXT NOT NULL,
    is_builtin INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS discussions (
    id TEXT PRIMARY KEY,
    topic TEXT NOT NULL,
    discussion_language TEXT NOT NULL DEFAULT 'fr',
    model_name TEXT NOT NULL DEFAULT '',
    participants_json TEXT NOT NULL DEFAULT '[]',
    total_turns INTEGER NOT NULL DEFAULT 0,
    synthesis TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS discussion_messages (
    id TEXT PRIMARY KEY,
    discussion_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL DEFAULT 0,
    speaker_id TEXT NOT NULL,
    speaker_name TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    inner_thought TEXT,
    reactions_json TEXT NOT NULL DEFAULT '[]',
    is_ban_notification INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (discussion_id) REFERENCES discussions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_dm_discussion_id
    ON discussion_messages(discussion_id, sort_order);
"#;

pub async fn initialize(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute_batch(SCHEMA)?;
        // Migration: add profile_type column (idempotent)
        let has_column: bool = conn
            .prepare("PRAGMA table_info(predefined_profiles)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .any(|col| col.as_deref() == Ok("profile_type"));
        if !has_column {
            conn.execute_batch(
                "ALTER TABLE predefined_profiles ADD COLUMN profile_type TEXT NOT NULL DEFAULT 'gladiateur';"
            )?;
        }
        // Migration: add category column (idempotent)
        let has_category: bool = conn
            .prepare("PRAGMA table_info(predefined_profiles)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .any(|col| col.as_deref() == Ok("category"));
        if !has_category {
            conn.execute_batch(
                "ALTER TABLE predefined_profiles ADD COLUMN category TEXT NOT NULL DEFAULT 'autres';"
            )?;
        }
        Ok(())
    })
    .await
}
