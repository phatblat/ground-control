use rusqlite::{Connection, params};
use std::path::Path;

use crate::models::SessionSummary;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                session_id    TEXT PRIMARY KEY,
                project_path  TEXT NOT NULL,
                display_name  TEXT NOT NULL,
                custom_title  TEXT,
                ai_title      TEXT,
                agent_name    TEXT,
                version       TEXT,
                git_branch    TEXT,
                input_tokens  INTEGER NOT NULL DEFAULT 0,
                output_tokens INTEGER NOT NULL DEFAULT 0,
                cache_read    INTEGER NOT NULL DEFAULT 0,
                cache_create  INTEGER NOT NULL DEFAULT 0,
                message_count INTEGER NOT NULL DEFAULT 0,
                indexed_at    TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                session_id UNINDEXED,
                custom_title,
                ai_title,
                agent_name,
                project_path,
                content='sessions',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS sessions_ai AFTER INSERT ON sessions BEGIN
                INSERT INTO sessions_fts(rowid, session_id, custom_title, ai_title, agent_name, project_path)
                VALUES (new.rowid, new.session_id, new.custom_title, new.ai_title, new.agent_name, new.project_path);
            END;

            CREATE TRIGGER IF NOT EXISTS sessions_ad AFTER DELETE ON sessions BEGIN
                INSERT INTO sessions_fts(sessions_fts, rowid, session_id, custom_title, ai_title, agent_name, project_path)
                VALUES ('delete', old.rowid, old.session_id, old.custom_title, old.ai_title, old.agent_name, old.project_path);
            END;

            CREATE TRIGGER IF NOT EXISTS sessions_au AFTER UPDATE ON sessions BEGIN
                INSERT INTO sessions_fts(sessions_fts, rowid, session_id, custom_title, ai_title, agent_name, project_path)
                VALUES ('delete', old.rowid, old.session_id, old.custom_title, old.ai_title, old.agent_name, old.project_path);
                INSERT INTO sessions_fts(rowid, session_id, custom_title, ai_title, agent_name, project_path)
                VALUES (new.rowid, new.session_id, new.custom_title, new.ai_title, new.agent_name, new.project_path);
            END;"
        )?;
        Ok(())
    }

    pub fn upsert_session(&self, s: &SessionSummary) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO sessions (session_id, project_path, display_name, custom_title, ai_title, agent_name, version, git_branch, input_tokens, output_tokens, cache_read, cache_create, message_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(session_id) DO UPDATE SET
                custom_title = excluded.custom_title,
                ai_title = excluded.ai_title,
                agent_name = excluded.agent_name,
                version = excluded.version,
                git_branch = excluded.git_branch,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                cache_read = excluded.cache_read,
                cache_create = excluded.cache_create,
                message_count = excluded.message_count,
                indexed_at = datetime('now')",
            params![
                s.session_id.to_string(),
                s.project_path,
                s.display_name,
                s.custom_title,
                s.ai_title,
                s.agent_name,
                s.version,
                s.git_branch,
                s.total_input_tokens as i64,
                s.total_output_tokens as i64,
                s.total_cache_read_tokens as i64,
                s.total_cache_creation_tokens as i64,
                s.message_count as i64,
            ],
        )?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT s.session_id, s.project_path, s.display_name,
                    s.custom_title, s.ai_title, s.agent_name,
                    s.input_tokens, s.output_tokens, s.message_count
             FROM sessions_fts f
             JOIN sessions s ON f.session_id = s.session_id
             WHERE sessions_fts MATCH ?1
             ORDER BY rank
             LIMIT 50"
        )?;

        let results = stmt.query_map(params![query], |row| {
            Ok(SearchResult {
                session_id: row.get(0)?,
                project_path: row.get(1)?,
                display_name: row.get(2)?,
                custom_title: row.get(3)?,
                ai_title: row.get(4)?,
                agent_name: row.get(5)?,
                input_tokens: row.get(6)?,
                output_tokens: row.get(7)?,
                message_count: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn all_sessions(&self) -> Result<Vec<SearchResult>, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, project_path, display_name,
                    custom_title, ai_title, agent_name,
                    input_tokens, output_tokens, message_count
             FROM sessions
             ORDER BY indexed_at DESC
             LIMIT 100"
        )?;

        let results = stmt.query_map([], |row| {
            Ok(SearchResult {
                session_id: row.get(0)?,
                project_path: row.get(1)?,
                display_name: row.get(2)?,
                custom_title: row.get(3)?,
                ai_title: row.get(4)?,
                agent_name: row.get(5)?,
                input_tokens: row.get(6)?,
                output_tokens: row.get(7)?,
                message_count: row.get(8)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn token_summary(&self) -> Result<TokenSummary, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cache_read), 0), COALESCE(SUM(cache_create), 0),
                    COALESCE(SUM(message_count), 0)
             FROM sessions"
        )?;

        stmt.query_row([], |row| {
            Ok(TokenSummary {
                session_count: row.get(0)?,
                total_input: row.get(1)?,
                total_output: row.get(2)?,
                total_cache_read: row.get(3)?,
                total_cache_create: row.get(4)?,
                total_messages: row.get(5)?,
            })
        }).map_err(StoreError::from)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub session_id: String,
    pub project_path: String,
    pub display_name: String,
    pub custom_title: Option<String>,
    pub ai_title: Option<String>,
    pub agent_name: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub message_count: i64,
}

impl SearchResult {
    pub fn title(&self) -> &str {
        self.custom_title
            .as_deref()
            .or(self.ai_title.as_deref())
            .or(self.agent_name.as_deref())
            .unwrap_or("untitled")
    }
}

#[derive(Debug, Clone)]
pub struct TokenSummary {
    pub session_count: i64,
    pub total_input: i64,
    pub total_output: i64,
    pub total_cache_read: i64,
    pub total_cache_create: i64,
    pub total_messages: i64,
}
