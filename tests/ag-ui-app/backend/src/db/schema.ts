import Database from 'better-sqlite3'
import path from 'path'
import fs from 'fs'
import { fileURLToPath } from 'url'

// Types
export type Tone = 'professional' | 'friendly' | 'apologetic' | 'assertive' | 'neutral'

export interface Conversation {
  id: string
  original_message: string
  context: string | null
  intent: string | null
  tone: Tone
  created_at: string
}

export interface Reply {
  id: string
  conversation_id: string
  content: string
  reply_index: number
  created_at: string
}

export interface ConversationWithReplies extends Conversation {
  replies: Reply[]
}

// Initialize database
export function initDb(): Database.Database {
  // Use DB_PATH env var, or /app/data in production, or local ./data in development
  const dbPath = process.env.DB_PATH || 
    (process.env.NODE_ENV === 'production' 
      ? '/app/data/smart-reply.db'
      : path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..', 'data', 'smart-reply.db'))

  // Ensure directory exists
  const dbDir = path.dirname(dbPath)
  if (!fs.existsSync(dbDir)) {
    fs.mkdirSync(dbDir, { recursive: true })
  }

  const db = new Database(dbPath)

  // Enable WAL mode for better concurrent performance
  db.pragma('journal_mode = WAL')
  db.pragma('foreign_keys = ON')

  // Create tables
  db.exec(`
    CREATE TABLE IF NOT EXISTS conversations (
      id TEXT PRIMARY KEY,
      original_message TEXT NOT NULL,
      context TEXT,
      intent TEXT,
      tone TEXT NOT NULL CHECK (tone IN ('professional', 'friendly', 'apologetic', 'assertive', 'neutral')),
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP
    );

    -- Add context and intent columns if they don't exist (for existing databases)
    -- SQLite doesn't support IF NOT EXISTS for ALTER TABLE, so we handle this in code


    CREATE TABLE IF NOT EXISTS replies (
      id TEXT PRIMARY KEY,
      conversation_id TEXT NOT NULL,
      content TEXT NOT NULL,
      reply_index INTEGER NOT NULL CHECK (reply_index BETWEEN 0 AND 2),
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS analytics (
      id TEXT PRIMARY KEY,
      event_type TEXT NOT NULL CHECK (event_type IN ('generate', 'copy', 'select', 'delete')),
      conversation_id TEXT,
      metadata TEXT,
      created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
    );

    CREATE INDEX IF NOT EXISTS idx_replies_conversation ON replies(conversation_id);
    CREATE INDEX IF NOT EXISTS idx_conversations_created ON conversations(created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_analytics_event ON analytics(event_type, created_at);
  `)

  return db
}
