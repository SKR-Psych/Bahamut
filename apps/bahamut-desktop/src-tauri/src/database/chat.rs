use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub attachment_metadata: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationDetail {
    pub conversation: Conversation,
    pub messages: Vec<StoredMessage>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDataSummary {
    pub conversations: i64,
    pub messages: i64,
    pub approximate_bytes: i64,
    pub persistence_enabled: bool,
}

pub fn init_chat_schema(conn: &Connection) -> Result<(), String> {
    conn.execute("CREATE TABLE IF NOT EXISTS conversations (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, model TEXT, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)", []).map_err(|e| format!("Failed to create conversations table: {e}"))?;
    conn.execute("CREATE TABLE IF NOT EXISTS conversation_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, role TEXT NOT NULL, content TEXT NOT NULL, model TEXT, attachment_metadata TEXT, status TEXT NOT NULL, error TEXT, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)", []).map_err(|e| format!("Failed to create messages table: {e}"))?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_conversation_messages_conversation ON conversation_messages(conversation_id, id)", []).map_err(|e| format!("Failed to index messages: {e}"))?;
    Ok(())
}

pub fn create_conversation(
    conn: &Connection,
    title: &str,
    model: Option<&str>,
) -> Result<Conversation, String> {
    conn.execute(
        "INSERT INTO conversations(title, model) VALUES (?1, ?2)",
        params![
            if title.trim().is_empty() {
                "Untitled chat"
            } else {
                title.trim()
            },
            model
        ],
    )
    .map_err(|e| format!("Failed to create conversation: {e}"))?;
    get_conversation(conn, conn.last_insert_rowid())
}

pub fn list_conversations(conn: &Connection) -> Result<Vec<Conversation>, String> {
    let mut stmt = conn.prepare("SELECT id,title,model,created_at,updated_at FROM conversations ORDER BY updated_at DESC, id DESC").map_err(|e| format!("Failed to list conversations: {e}"))?;
    let conversations = stmt
        .query_map([], map_conversation)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read conversations: {e}"))?;
    Ok(conversations)
}

pub fn get_conversation(conn: &Connection, id: i64) -> Result<Conversation, String> {
    conn.query_row(
        "SELECT id,title,model,created_at,updated_at FROM conversations WHERE id=?1",
        params![id],
        map_conversation,
    )
    .optional()
    .map_err(|e| format!("Failed to read conversation: {e}"))?
    .ok_or_else(|| "Conversation not found".to_string())
}

pub fn read_conversation(conn: &Connection, id: i64) -> Result<ConversationDetail, String> {
    let conversation = get_conversation(conn, id)?;
    let mut stmt = conn.prepare("SELECT id,conversation_id,role,content,model,attachment_metadata,status,error,created_at FROM conversation_messages WHERE conversation_id=?1 ORDER BY id ASC").map_err(|e| format!("Failed to read messages: {e}"))?;
    let messages = stmt
        .query_map(params![id], map_message)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map messages: {e}"))?;
    Ok(ConversationDetail {
        conversation,
        messages,
    })
}

pub fn rename_conversation(
    conn: &Connection,
    id: i64,
    title: &str,
) -> Result<Conversation, String> {
    conn.execute(
        "UPDATE conversations SET title=?1, updated_at=CURRENT_TIMESTAMP WHERE id=?2",
        params![
            if title.trim().is_empty() {
                "Untitled chat"
            } else {
                title.trim()
            },
            id
        ],
    )
    .map_err(|e| format!("Failed to rename conversation: {e}"))?;
    get_conversation(conn, id)
}

pub fn delete_conversation(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM conversations WHERE id=?1", params![id])
        .map_err(|e| format!("Failed to delete conversation: {e}"))?;
    Ok(())
}
pub fn clear_history(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM conversation_messages", [])
        .map_err(|e| format!("Failed to clear messages: {e}"))?;
    conn.execute("DELETE FROM conversations", [])
        .map_err(|e| format!("Failed to clear conversations: {e}"))?;
    Ok(())
}

pub struct NewMessage<'a> {
    pub conversation_id: i64,
    pub role: &'a str,
    pub content: &'a str,
    pub model: Option<&'a str>,
    pub attachment_metadata: Option<&'a str>,
    pub status: &'a str,
    pub error: Option<&'a str>,
}

pub fn insert_message(conn: &Connection, message: NewMessage<'_>) -> Result<StoredMessage, String> {
    if !matches!(message.role, "system" | "user" | "assistant" | "tool") {
        return Err("Invalid message role".into());
    }
    conn.execute(
        "INSERT INTO conversation_messages(conversation_id, role, content, model, attachment_metadata, status, error) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            message.conversation_id,
            message.role,
            message.content,
            message.model,
            message.attachment_metadata,
            message.status,
            message.error
        ],
    )
    .map_err(|e| format!("Failed to store message: {e}"))?;
    conn.execute(
        "UPDATE conversations SET updated_at=CURRENT_TIMESTAMP, model=COALESCE(?1, model) WHERE id=?2",
        params![message.model, message.conversation_id],
    )
    .ok();
    let id = conn.last_insert_rowid();
    conn.query_row("SELECT id,conversation_id,role,content,model,attachment_metadata,status,error,created_at FROM conversation_messages WHERE id=?1", params![id], map_message).map_err(|e| format!("Failed to read stored message: {e}"))
}

pub fn inspect_stored_data(
    conn: &Connection,
    persistence_enabled: bool,
) -> Result<StoredDataSummary, String> {
    let conversations: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
        .unwrap_or(0);
    let messages: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversation_messages", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    let approximate_bytes: i64 = conn.query_row("SELECT COALESCE(SUM(LENGTH(content) + COALESCE(LENGTH(attachment_metadata),0)),0) FROM conversation_messages", [], |r| r.get(0)).unwrap_or(0);
    Ok(StoredDataSummary {
        conversations,
        messages,
        approximate_bytes,
        persistence_enabled,
    })
}

fn map_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        model: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}
fn map_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredMessage> {
    Ok(StoredMessage {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        model: row.get(4)?,
        attachment_metadata: row.get(5)?,
        status: row.get(6)?,
        error: row.get(7)?,
        created_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conversation_crud_and_clear() {
        let c = Connection::open_in_memory().unwrap();
        init_chat_schema(&c).unwrap();
        let conv = create_conversation(&c, "A", Some("m")).unwrap();
        insert_message(
            &c,
            NewMessage {
                conversation_id: conv.id,
                role: "user",
                content: "hello",
                model: Some("m"),
                attachment_metadata: None,
                status: "complete",
                error: None,
            },
        )
        .unwrap();
        assert_eq!(read_conversation(&c, conv.id).unwrap().messages.len(), 1);
        rename_conversation(&c, conv.id, "B").unwrap();
        assert_eq!(list_conversations(&c).unwrap()[0].title, "B");
        clear_history(&c).unwrap();
        assert_eq!(list_conversations(&c).unwrap().len(), 0);
    }
}
