use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatData {
   pub id: String,
   pub title: String,
   pub created_at: i64,
   pub last_message_at: i64,
   pub agent_id: Option<String>,
   pub acp_session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageData {
   pub id: String,
   pub chat_id: String,
   pub role: String,
   pub content: String,
   pub timestamp: i64,
   pub is_streaming: bool,
   pub is_tool_use: bool,
   pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallData {
   pub message_id: String,
   pub name: String,
   pub input: Option<String>,
   pub output: Option<String>,
   pub error: Option<String>,
   pub timestamp: i64,
   pub is_complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatWithMessages {
   pub chat: ChatData,
   pub messages: Vec<MessageData>,
   pub tool_calls: Vec<ToolCallData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatStats {
   pub total_chats: i64,
   pub total_messages: i64,
   pub total_tool_calls: i64,
}

#[derive(Clone)]
pub struct ChatHistoryRepository {
   db_path: PathBuf,
}

impl ChatHistoryRepository {
   pub fn new(db_path: PathBuf) -> Self {
      Self { db_path }
   }

   pub fn initialize(&self) -> Result<(), String> {
      let conn = self.open_connection()?;

      conn
         .execute(
            "CREATE TABLE IF NOT EXISTS chats (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               created_at INTEGER NOT NULL,
               last_message_at INTEGER NOT NULL,
               agent_id TEXT DEFAULT 'custom',
               acp_session_id TEXT
            )",
            [],
         )
         .map_err(|e| format!("Failed to create chats table: {}", e))?;

      let _ = conn.execute(
         "ALTER TABLE chats ADD COLUMN agent_id TEXT DEFAULT 'custom'",
         [],
      );
      let _ = conn.execute("ALTER TABLE chats ADD COLUMN acp_session_id TEXT", []);

      conn
         .execute(
            "CREATE TABLE IF NOT EXISTS messages (
               id TEXT PRIMARY KEY,
               chat_id TEXT NOT NULL,
               role TEXT NOT NULL,
               content TEXT NOT NULL,
               timestamp INTEGER NOT NULL,
               is_streaming BOOLEAN DEFAULT 0,
               is_tool_use BOOLEAN DEFAULT 0,
               tool_name TEXT,
               FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
            )",
            [],
         )
         .map_err(|e| format!("Failed to create messages table: {}", e))?;

      conn
         .execute(
            "CREATE TABLE IF NOT EXISTS tool_calls (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               message_id TEXT NOT NULL,
               name TEXT NOT NULL,
               input TEXT,
               output TEXT,
               error TEXT,
               timestamp INTEGER NOT NULL,
               is_complete BOOLEAN DEFAULT 0,
               FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
            )",
            [],
         )
         .map_err(|e| format!("Failed to create tool_calls table: {}", e))?;

      conn
         .execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id)",
            [],
         )
         .map_err(|e| format!("Failed to create messages index: {}", e))?;
      conn
         .execute(
            "CREATE INDEX IF NOT EXISTS idx_chats_last_message ON chats(last_message_at DESC)",
            [],
         )
         .map_err(|e| format!("Failed to create chats index: {}", e))?;
      conn
         .execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_calls_message_id ON tool_calls(message_id)",
            [],
         )
         .map_err(|e| format!("Failed to create tool_calls index: {}", e))?;

      Ok(())
   }

   pub fn save_chat(
      &self,
      chat: ChatData,
      messages: Vec<MessageData>,
      tool_calls: Vec<ToolCallData>,
   ) -> Result<(), String> {
      let mut conn = self.open_connection()?;
      let tx = conn
         .transaction()
         .map_err(|e| format!("Failed to begin transaction: {}", e))?;

      tx.execute(
         "INSERT OR REPLACE INTO chats (id, title, created_at, last_message_at, agent_id, \
          acp_session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
         params![
            chat.id,
            chat.title,
            chat.created_at,
            chat.last_message_at,
            chat.agent_id.unwrap_or_else(|| "custom".to_string()),
            chat.acp_session_id
         ],
      )
      .map_err(|e| format!("Failed to save chat: {}", e))?;

      tx.execute("DELETE FROM messages WHERE chat_id = ?1", params![chat.id])
         .map_err(|e| format!("Failed to delete old messages: {}", e))?;

      for message in messages {
         tx.execute(
            "INSERT INTO messages (id, chat_id, role, content, timestamp, is_streaming, \
             is_tool_use, tool_name)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
               message.id,
               message.chat_id,
               message.role,
               message.content,
               message.timestamp,
               message.is_streaming,
               message.is_tool_use,
               message.tool_name
            ],
         )
         .map_err(|e| format!("Failed to save message: {}", e))?;
      }

      for tool_call in tool_calls {
         tx.execute(
            "INSERT INTO tool_calls (message_id, name, input, output, error, timestamp, \
             is_complete)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
               tool_call.message_id,
               tool_call.name,
               tool_call.input,
               tool_call.output,
               tool_call.error,
               tool_call.timestamp,
               tool_call.is_complete
            ],
         )
         .map_err(|e| format!("Failed to save tool call: {}", e))?;
      }

      tx.commit()
         .map_err(|e| format!("Failed to commit transaction: {}", e))
   }

   pub fn load_all_chats(&self) -> Result<Vec<ChatData>, String> {
      let conn = self.open_connection()?;
      let mut stmt = conn
         .prepare(
            "SELECT id, title, created_at, last_message_at, agent_id, acp_session_id
             FROM chats ORDER BY last_message_at DESC",
         )
         .map_err(|e| format!("Failed to prepare query: {}", e))?;

      stmt
         .query_map([], map_chat_row)
         .map_err(|e| format!("Failed to query chats: {}", e))?
         .collect::<SqliteResult<Vec<_>>>()
         .map_err(|e| format!("Failed to collect chats: {}", e))
   }

   pub fn load_chat(&self, chat_id: &str) -> Result<ChatWithMessages, String> {
      let conn = self.open_connection()?;

      let chat = conn
         .query_row(
            "SELECT id, title, created_at, last_message_at, agent_id, acp_session_id
             FROM chats WHERE id = ?1",
            [chat_id],
            map_chat_row,
         )
         .map_err(|e| format!("Failed to load chat: {}", e))?;

      let messages = {
         let mut stmt = conn
            .prepare(
               "SELECT id, chat_id, role, content, timestamp, is_streaming, is_tool_use, tool_name
                FROM messages WHERE chat_id = ?1 ORDER BY timestamp ASC",
            )
            .map_err(|e| format!("Failed to prepare messages query: {}", e))?;
         stmt
            .query_map([chat_id], map_message_row)
            .map_err(|e| format!("Failed to query messages: {}", e))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| format!("Failed to collect messages: {}", e))?
      };

      let tool_calls = {
         let mut stmt = conn
            .prepare(
               "SELECT tc.message_id, tc.name, tc.input, tc.output, tc.error, tc.timestamp, \
                tc.is_complete
                FROM tool_calls tc
                JOIN messages m ON m.id = tc.message_id
                WHERE m.chat_id = ?1
                ORDER BY tc.timestamp ASC",
            )
            .map_err(|e| format!("Failed to prepare tool call query: {}", e))?;
         stmt
            .query_map([chat_id], map_tool_call_row)
            .map_err(|e| format!("Failed to query tool calls: {}", e))?
            .collect::<SqliteResult<Vec<_>>>()
            .map_err(|e| format!("Failed to collect tool calls: {}", e))?
      };

      Ok(ChatWithMessages {
         chat,
         messages,
         tool_calls,
      })
   }

   pub fn delete_chat(&self, chat_id: &str) -> Result<(), String> {
      let conn = self.open_connection()?;
      conn
         .execute("DELETE FROM chats WHERE id = ?1", [chat_id])
         .map(|_| ())
         .map_err(|e| format!("Failed to delete chat: {}", e))
   }

   pub fn search_chats(&self, query: &str) -> Result<Vec<ChatData>, String> {
      let conn = self.open_connection()?;
      let pattern = format!("%{}%", query);
      let mut stmt = conn
         .prepare(
            "SELECT DISTINCT c.id, c.title, c.created_at, c.last_message_at, c.agent_id, \
             c.acp_session_id
             FROM chats c
             LEFT JOIN messages m ON m.chat_id = c.id
             WHERE c.title LIKE ?1 OR m.content LIKE ?1
             ORDER BY c.last_message_at DESC",
         )
         .map_err(|e| format!("Failed to prepare search query: {}", e))?;

      stmt
         .query_map([pattern], map_chat_row)
         .map_err(|e| format!("Failed to search chats: {}", e))?
         .collect::<SqliteResult<Vec<_>>>()
         .map_err(|e| format!("Failed to collect search results: {}", e))
   }

   pub fn get_stats(&self) -> Result<ChatStats, String> {
      let conn = self.open_connection()?;
      Ok(ChatStats {
         total_chats: conn
            .query_row("SELECT COUNT(*) FROM chats", [], |row| row.get(0))
            .unwrap_or(0),
         total_messages: conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap_or(0),
         total_tool_calls: conn
            .query_row("SELECT COUNT(*) FROM tool_calls", [], |row| row.get(0))
            .unwrap_or(0),
      })
   }

   fn open_connection(&self) -> Result<Connection, String> {
      if let Some(parent) = self.db_path.parent() {
         std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create chat data directory: {}", e))?;
      }
      let conn = Connection::open(&self.db_path)
         .map_err(|e| format!("Failed to open chat database: {}", e))?;
      conn
         .execute("PRAGMA foreign_keys = ON", [])
         .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
      Ok(conn)
   }
}

fn map_chat_row(row: &rusqlite::Row<'_>) -> SqliteResult<ChatData> {
   Ok(ChatData {
      id: row.get(0)?,
      title: row.get(1)?,
      created_at: row.get(2)?,
      last_message_at: row.get(3)?,
      agent_id: row.get(4)?,
      acp_session_id: row.get(5)?,
   })
}

fn map_message_row(row: &rusqlite::Row<'_>) -> SqliteResult<MessageData> {
   Ok(MessageData {
      id: row.get(0)?,
      chat_id: row.get(1)?,
      role: row.get(2)?,
      content: row.get(3)?,
      timestamp: row.get(4)?,
      is_streaming: row.get(5)?,
      is_tool_use: row.get(6)?,
      tool_name: row.get(7)?,
   })
}

fn map_tool_call_row(row: &rusqlite::Row<'_>) -> SqliteResult<ToolCallData> {
   Ok(ToolCallData {
      message_id: row.get(0)?,
      name: row.get(1)?,
      input: row.get(2)?,
      output: row.get(3)?,
      error: row.get(4)?,
      timestamp: row.get(5)?,
      is_complete: row.get(6)?,
   })
}
