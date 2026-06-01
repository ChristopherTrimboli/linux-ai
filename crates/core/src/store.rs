//! SQLite-backed persistence for conversations, messages, and tool calls.
//!
//! Stored at `~/.local/share/linux-ai/linux-ai.db`. Access is wrapped in a
//! `Mutex` so the store is cheap to clone (via `Arc`) and share across the
//! async runtime; queries are short and synchronous.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::message::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub conversation_id: String,
    pub idx: i64,
    pub message: Message,
    pub created_at: String,
}

#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

impl Store {
    pub fn data_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("dev", "linux-ai", "linux-ai")
            .ok_or_else(|| Error::Config("could not determine data directory".into()))?;
        Ok(dirs.data_dir().join("linux-ai.db"))
    }

    pub fn open_default() -> Result<Store> {
        let path = Self::data_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::open(&path)
    }

    pub fn open(path: &std::path::Path) -> Result<Store> {
        let conn = Connection::open(path)?;
        let store = Store {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    /// In-memory store, mainly for tests.
    pub fn open_in_memory() -> Result<Store> {
        let conn = Connection::open_in_memory()?;
        let store = Store {
            conn: Arc::new(Mutex::new(conn)),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS conversations (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS messages (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
                idx             INTEGER NOT NULL,
                role            TEXT NOT NULL,
                content_json    TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id, idx);
            CREATE TABLE IF NOT EXISTS tool_calls (
                id              TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                message_id      TEXT,
                name            TEXT NOT NULL,
                input_json      TEXT NOT NULL,
                output          TEXT,
                is_error        INTEGER NOT NULL DEFAULT 0,
                created_at      TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn now() -> String {
        chrono::Utc::now().to_rfc3339()
    }

    pub fn create_conversation(&self, title: &str) -> Result<Conversation> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            rusqlite::params![id, title, now],
        )?;
        Ok(Conversation {
            id,
            title: title.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET title = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, title, Self::now()],
        )?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages WHERE conversation_id = ?1", [id])?;
        conn.execute("DELETE FROM conversations WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn append_message(&self, conversation_id: &str, message: &Message) -> Result<StoredMessage> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Self::now();
        let content_json = serde_json::to_string(&message.content)?;
        let role = serde_json::to_value(message.role)?
            .as_str()
            .unwrap_or("user")
            .to_string();

        let conn = self.conn.lock().unwrap();
        let idx: i64 = conn.query_row(
            "SELECT COALESCE(MAX(idx), -1) + 1 FROM messages WHERE conversation_id = ?1",
            [conversation_id],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO messages (id, conversation_id, idx, role, content_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, conversation_id, idx, role, content_json, now],
        )?;
        conn.execute(
            "UPDATE conversations SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![conversation_id, now],
        )?;
        Ok(StoredMessage {
            id,
            conversation_id: conversation_id.to_string(),
            idx,
            message: message.clone(),
            created_at: now,
        })
    }

    pub fn load_messages(&self, conversation_id: &str) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT role, content_json FROM messages WHERE conversation_id = ?1 ORDER BY idx ASC",
        )?;
        let rows = stmt
            .query_map([conversation_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut out = Vec::with_capacity(rows.len());
        for (role, content_json) in rows {
            let role = serde_json::from_value(serde_json::Value::String(role))?;
            let content = serde_json::from_str(&content_json)?;
            out.push(Message { role, content });
        }
        Ok(out)
    }

    pub fn record_tool_call(
        &self,
        conversation_id: &str,
        name: &str,
        input: &serde_json::Value,
        output: &str,
        is_error: bool,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tool_calls (id, conversation_id, message_id, name, input_json, output, is_error, created_at)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                conversation_id,
                name,
                serde_json::to_string(input)?,
                output,
                is_error as i64,
                Self::now(),
            ],
        )?;
        Ok(())
    }
}
