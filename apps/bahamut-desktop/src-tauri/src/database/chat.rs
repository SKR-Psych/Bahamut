//! Local conversation storage. Conversations, messages, and attachment
//! metadata live in the same SQLite database as settings and the audit log,
//! but chat content is deliberately NOT part of the hash-chained audit table
//! (per docs/security.md, the chain records security-relevant actions, not
//! conversation text). No credentials or tokens are ever stored here.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

pub fn init_chat_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            model TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            model TEXT,
            status TEXT NOT NULL DEFAULT 'complete',
            error TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_chat_messages_conv
            ON chat_messages (conversation_id, id);
        CREATE TABLE IF NOT EXISTS chat_attachments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL,
            kind TEXT NOT NULL,
            label TEXT NOT NULL,
            path TEXT,
            chars INTEGER NOT NULL,
            truncated INTEGER NOT NULL DEFAULT 0,
            flagged INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_chat_attachments_msg
            ON chat_attachments (message_id);",
    )
    .map_err(|e| format!("Failed to create chat tables: {}", e))
}

#[derive(Debug, Serialize)]
pub struct ConversationMeta {
    pub id: i64,
    pub title: String,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: i64,
}

#[derive(Debug, Serialize)]
pub struct StoredAttachment {
    pub kind: String,
    pub label: String,
    pub path: Option<String>,
    pub chars: i64,
    pub truncated: bool,
    pub flagged: bool,
}

#[derive(Debug, Serialize)]
pub struct StoredMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
    pub attachments: Vec<StoredAttachment>,
}

#[derive(Debug, Serialize)]
pub struct ChatStorageInfo {
    pub conversations: i64,
    pub messages: i64,
    pub attachment_records: i64,
    /// What is stored: titles, message text, model names, attachment
    /// metadata (label/path/size — not credentials).
    pub description: String,
}

pub fn create_conversation(
    conn: &Connection,
    title: &str,
    model: Option<&str>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO conversations (title, model) VALUES (?1, ?2)",
        params![title, model],
    )
    .map_err(|e| format!("Failed to create conversation: {}", e))?;
    Ok(conn.last_insert_rowid())
}

pub fn list_conversations(conn: &Connection) -> Result<Vec<ConversationMeta>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.title, c.model, c.created_at, c.updated_at,
                    (SELECT COUNT(*) FROM chat_messages m WHERE m.conversation_id = c.id)
             FROM conversations c ORDER BY c.updated_at DESC, c.id DESC",
        )
        .map_err(|e| format!("Failed to list conversations: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ConversationMeta {
                id: row.get(0)?,
                title: row.get(1)?,
                model: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                message_count: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to list conversations: {}", e))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to list conversations: {}", e))
}

pub fn rename_conversation(conn: &Connection, id: i64, title: &str) -> Result<(), String> {
    let changed = conn
        .execute(
            "UPDATE conversations SET title = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![title, id],
        )
        .map_err(|e| format!("Failed to rename conversation: {}", e))?;
    if changed == 0 {
        return Err("Conversation not found".to_string());
    }
    Ok(())
}

pub fn delete_conversation(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "DELETE FROM chat_attachments WHERE message_id IN
            (SELECT id FROM chat_messages WHERE conversation_id = ?1)",
        params![id],
    )
    .map_err(|e| format!("Failed to delete conversation attachments: {}", e))?;
    conn.execute(
        "DELETE FROM chat_messages WHERE conversation_id = ?1",
        params![id],
    )
    .map_err(|e| format!("Failed to delete conversation messages: {}", e))?;
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])
        .map_err(|e| format!("Failed to delete conversation: {}", e))?;
    Ok(())
}

/// Removes every conversation, message, and attachment record. Returns the
/// number of conversations removed.
pub fn clear_all_history(conn: &Connection) -> Result<i64, String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
        .map_err(|e| format!("Failed to count conversations: {}", e))?;
    conn.execute_batch(
        "DELETE FROM chat_attachments; DELETE FROM chat_messages; DELETE FROM conversations;",
    )
    .map_err(|e| format!("Failed to clear chat history: {}", e))?;
    Ok(count)
}

#[allow(clippy::too_many_arguments)]
pub fn add_message(
    conn: &Connection,
    conversation_id: i64,
    role: &str,
    content: &str,
    model: Option<&str>,
    status: &str,
    error: Option<&str>,
) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO chat_messages (conversation_id, role, content, model, status, error)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![conversation_id, role, content, model, status, error],
    )
    .map_err(|e| format!("Failed to store message: {}", e))?;
    conn.execute(
        "UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![conversation_id],
    )
    .map_err(|e| format!("Failed to touch conversation: {}", e))?;
    Ok(conn.last_insert_rowid())
}

pub fn add_attachment_meta(
    conn: &Connection,
    message_id: i64,
    kind: &str,
    label: &str,
    path: Option<&str>,
    chars: i64,
    truncated: bool,
    flagged: bool,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO chat_attachments (message_id, kind, label, path, chars, truncated, flagged)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![message_id, kind, label, path, chars, truncated, flagged],
    )
    .map_err(|e| format!("Failed to store attachment metadata: {}", e))?;
    Ok(())
}

pub fn get_messages(conn: &Connection, conversation_id: i64) -> Result<Vec<StoredMessage>, String> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM conversations WHERE id = ?1",
            params![conversation_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to read conversation: {}", e))?;
    if exists.is_none() {
        return Err("Conversation not found".to_string());
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, role, content, model, status, error, created_at
             FROM chat_messages WHERE conversation_id = ?1 ORDER BY id ASC",
        )
        .map_err(|e| format!("Failed to read messages: {}", e))?;
    let mut messages: Vec<StoredMessage> = stmt
        .query_map(params![conversation_id], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                model: row.get(3)?,
                status: row.get(4)?,
                error: row.get(5)?,
                created_at: row.get(6)?,
                attachments: Vec::new(),
            })
        })
        .map_err(|e| format!("Failed to read messages: {}", e))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("Failed to read messages: {}", e))?;

    let mut att_stmt = conn
        .prepare(
            "SELECT kind, label, path, chars, truncated, flagged
             FROM chat_attachments WHERE message_id = ?1 ORDER BY id ASC",
        )
        .map_err(|e| format!("Failed to read attachments: {}", e))?;
    for message in &mut messages {
        let atts = att_stmt
            .query_map(params![message.id], |row| {
                Ok(StoredAttachment {
                    kind: row.get(0)?,
                    label: row.get(1)?,
                    path: row.get(2)?,
                    chars: row.get(3)?,
                    truncated: row.get::<_, i64>(4)? != 0,
                    flagged: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| format!("Failed to read attachments: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read attachments: {}", e))?;
        message.attachments = atts;
    }
    Ok(messages)
}

pub fn storage_info(conn: &Connection) -> Result<ChatStorageInfo, String> {
    let count = |sql: &str| -> Result<i64, String> {
        conn.query_row(sql, [], |r| r.get(0))
            .map_err(|e| format!("Failed to inspect chat storage: {}", e))
    };
    Ok(ChatStorageInfo {
        conversations: count("SELECT COUNT(*) FROM conversations")?,
        messages: count("SELECT COUNT(*) FROM chat_messages")?,
        attachment_records: count("SELECT COUNT(*) FROM chat_attachments")?,
        description: "Stored locally in bahamut.db: conversation titles, message text, \
                      model names, timestamps, generation status, and attachment metadata \
                      (label, path, size, truncation/flag markers). Attachment file \
                      contents and credentials are never stored."
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_schema;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        init_schema(&c).unwrap();
        c
    }

    #[test]
    fn conversation_roundtrip_with_messages_and_attachments() {
        let c = conn();
        let conv = create_conversation(&c, "About the parser", Some("qwen2.5-coder:7b")).unwrap();
        let msg = add_message(
            &c,
            conv,
            "user",
            "What does parse() do?",
            None,
            "complete",
            None,
        )
        .unwrap();
        add_attachment_meta(&c, msg, "file", "parser.rs", Some("C:\\p\\parser.rs"), 1200, false, false)
            .unwrap();
        add_message(
            &c,
            conv,
            "assistant",
            "It parses…",
            Some("qwen2.5-coder:7b"),
            "complete",
            None,
        )
        .unwrap();

        let list = list_conversations(&c).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].message_count, 2);

        let messages = get_messages(&c, conv).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].attachments.len(), 1);
        assert_eq!(messages[0].attachments[0].label, "parser.rs");
        assert_eq!(messages[1].role, "assistant");
    }

    #[test]
    fn rename_and_delete_conversation() {
        let c = conn();
        let conv = create_conversation(&c, "New chat", None).unwrap();
        rename_conversation(&c, conv, "Better title").unwrap();
        assert_eq!(list_conversations(&c).unwrap()[0].title, "Better title");

        let msg = add_message(&c, conv, "user", "hi", None, "complete", None).unwrap();
        add_attachment_meta(&c, msg, "manual", "note", None, 2, false, false).unwrap();
        delete_conversation(&c, conv).unwrap();
        assert!(list_conversations(&c).unwrap().is_empty());
        let orphans: i64 = c
            .query_row("SELECT COUNT(*) FROM chat_attachments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphans, 0, "attachments cascade-deleted");
        assert!(get_messages(&c, conv).is_err());
    }

    #[test]
    fn clear_all_history_empties_everything() {
        let c = conn();
        for i in 0..3 {
            let conv = create_conversation(&c, &format!("c{}", i), None).unwrap();
            add_message(&c, conv, "user", "x", None, "complete", None).unwrap();
        }
        assert_eq!(clear_all_history(&c).unwrap(), 3);
        let info = storage_info(&c).unwrap();
        assert_eq!(info.conversations, 0);
        assert_eq!(info.messages, 0);
        assert_eq!(info.attachment_records, 0);
    }

    #[test]
    fn cancelled_and_error_statuses_are_recorded() {
        let c = conn();
        let conv = create_conversation(&c, "t", None).unwrap();
        add_message(&c, conv, "assistant", "partial…", Some("m"), "cancelled", None).unwrap();
        add_message(&c, conv, "assistant", "", Some("m"), "error", Some("timeout")).unwrap();
        let msgs = get_messages(&c, conv).unwrap();
        assert_eq!(msgs[0].status, "cancelled");
        assert_eq!(msgs[1].error.as_deref(), Some("timeout"));
    }
}
