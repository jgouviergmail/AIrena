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

CREATE TABLE IF NOT EXISTS persona_memories (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    discussion_id TEXT NOT NULL,
    topic TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT '',
    recap_json TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (discussion_id) REFERENCES discussions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pm_profile_id ON persona_memories(profile_id);

CREATE TABLE IF NOT EXISTS discussion_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    config_json TEXT NOT NULL DEFAULT '{}',
    builtin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT ''
);
"#;

/// Full-text index of the discussions (v1.20). FTS5 ships with the bundled
/// SQLite (spike H-P5); should it be missing, the search falls back to `LIKE`.
const FTS_SCHEMA: &str = "CREATE VIRTUAL TABLE IF NOT EXISTS discussions_fts USING fts5(discussion_id UNINDEXED, topic, synthesis, content);";

/// Whether the full-text index can be used on this connection.
pub fn fts_available(conn: &rusqlite::Connection) -> bool {
    conn.prepare("SELECT count(*) FROM discussions_fts").is_ok()
}

/// Index every discussion the index does not know yet (idempotent).
pub fn backfill_fts(conn: &rusqlite::Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO discussions_fts (discussion_id, topic, synthesis, content)
         SELECT d.id, d.topic, d.synthesis, COALESCE((SELECT group_concat(m.content, ' ') FROM discussion_messages m WHERE m.discussion_id = d.id), '')
         FROM discussions d
         WHERE d.id NOT IN (SELECT discussion_id FROM discussions_fts)",
        [],
    )
}

/// Idempotent column migrations, in the order they shipped: `(table, column, type + default)`.
const COLUMN_MIGRATIONS: &[(&str, &str, &str)] = &[
    ("predefined_profiles", "profile_type", "TEXT NOT NULL DEFAULT 'gladiateur'"),
    ("predefined_profiles", "category", "TEXT NOT NULL DEFAULT 'autres'"),
    ("predefined_profiles", "initial_emotions", "TEXT"),
    ("discussions", "discussion_mode", "TEXT NOT NULL DEFAULT 'debate'"),
    ("discussions", "document_content", "TEXT NOT NULL DEFAULT ''"),
    ("discussions", "document_format", "TEXT NOT NULL DEFAULT 'none'"),
    ("discussions", "argument_map_md", "TEXT NOT NULL DEFAULT ''"),
    ("discussions", "argument_map_md_by_speaker", "TEXT NOT NULL DEFAULT ''"),
    // v1.16: provider, usage, cost, structured argument map
    ("discussions", "llm_provider", "TEXT NOT NULL DEFAULT 'ollama'"),
    ("discussions", "usage_json", "TEXT NOT NULL DEFAULT '{}'"),
    ("discussions", "estimated_cost_usd", "REAL NOT NULL DEFAULT 0"),
    ("discussions", "argument_map_json", "TEXT NOT NULL DEFAULT ''"),
    // v1.17: analytical report (sources, timeline, emotion history, positions, agendas, scores…)
    ("discussions", "report_json", "TEXT NOT NULL DEFAULT ''"),
    // v1.20: tags and favourite flag
    ("discussions", "tags", "TEXT NOT NULL DEFAULT '[]'"),
    ("discussions", "favorite", "INTEGER NOT NULL DEFAULT 0"),
    ("discussion_messages", "thought_kind", "TEXT NOT NULL DEFAULT 'persona'"),
    // v1.17: message kind (normal / banNotification / stageDirection / …)
    ("discussion_messages", "kind", "TEXT NOT NULL DEFAULT 'normal'"),
];

/// Whether `table` already has `column` (`PRAGMA table_info`).
fn has_column(conn: &rusqlite::Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let has = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col| col.as_deref() == Ok(column));
    Ok(has)
}

pub async fn initialize(db: &Connection) -> Result<(), tokio_rusqlite::Error> {
    db.call(|conn| {
        conn.execute_batch(SCHEMA)?;
        for (table, column, definition) in COLUMN_MIGRATIONS {
            if !has_column(conn, table, column)? {
                conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition};"))?;
            }
        }
        // Full-text index (v1.20): created when FTS5 is compiled in, backfilled from existing rows
        match conn.execute_batch(FTS_SCHEMA) {
            Ok(()) => match backfill_fts(conn) {
                Ok(n) if n > 0 => tracing::info!(indexed = n, "Discussion full-text index backfilled"),
                Ok(_) => {}
                Err(e) => tracing::warn!(error = %e, "Full-text index backfill failed — search falls back to LIKE"),
            },
            Err(e) => tracing::warn!(error = %e, "FTS5 unavailable — history search falls back to LIKE"),
        }
        Ok(())
    })
    .await
}
