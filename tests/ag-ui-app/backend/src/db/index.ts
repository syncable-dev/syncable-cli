import Database from 'better-sqlite3'
import { initDb, type Tone, type Conversation, type Reply, type ConversationWithReplies } from './schema.js'
import { nanoid } from 'nanoid'

// Singleton database instance
let db: Database.Database | null = null

export function getDb(): Database.Database {
  if (!db) {
    db = initDb()
  }
  return db
}

// Database operations
export const dbOps = {
  // Create a new conversation
  createConversation(message: string, tone: Tone, context?: string, intent?: string): string {
    const db = getDb()
    const id = nanoid()
    db.prepare('INSERT INTO conversations (id, original_message, context, intent, tone) VALUES (?, ?, ?, ?, ?)')
      .run(id, message, context || null, intent || null, tone)
    return id
  },

  // Save generated replies
  saveReplies(conversationId: string, replies: string[]): void {
    const db = getDb()
    const stmt = db.prepare(
      'INSERT INTO replies (id, conversation_id, content, reply_index) VALUES (?, ?, ?, ?)'
    )

    const insertMany = db.transaction((items: string[]) => {
      items.forEach((content, index) => {
        stmt.run(nanoid(), conversationId, content, index)
      })
    })

    insertMany(replies)
  },

  // Get all conversations with their replies
  getHistory(limit: number = 50): ConversationWithReplies[] {
    const db = getDb()

    const conversations = db.prepare(`
      SELECT c.*,
             json_group_array(
               json_object(
                 'id', r.id,
                 'content', r.content,
                 'reply_index', r.reply_index,
                 'created_at', r.created_at
               )
             ) as replies_json
      FROM conversations c
      LEFT JOIN replies r ON c.id = r.conversation_id
      GROUP BY c.id
      ORDER BY c.created_at DESC
      LIMIT ?
    `).all(limit) as (Conversation & { replies_json: string })[]

    return conversations.map(conv => ({
      ...conv,
      replies: JSON.parse(conv.replies_json)
        .filter((r: Reply & { id: string | null }) => r.id !== null)
        .sort((a: Reply, b: Reply) => a.reply_index - b.reply_index)
    }))
  },

  // Get a single conversation with replies
  getConversation(id: string): ConversationWithReplies | null {
    const db = getDb()

    const conversation = db.prepare('SELECT * FROM conversations WHERE id = ?').get(id) as Conversation | undefined
    if (!conversation) return null

    const replies = db.prepare(
      'SELECT * FROM replies WHERE conversation_id = ? ORDER BY reply_index'
    ).all(id) as Reply[]

    return { ...conversation, replies }
  },

  // Delete a conversation (cascades to replies)
  deleteConversation(id: string): boolean {
    const db = getDb()
    const result = db.prepare('DELETE FROM conversations WHERE id = ?').run(id)
    return result.changes > 0
  },

  // Log analytics event
  logEvent(eventType: 'generate' | 'copy' | 'select' | 'delete', conversationId?: string, metadata?: object): void {
    const db = getDb()
    db.prepare(
      'INSERT INTO analytics (id, event_type, conversation_id, metadata) VALUES (?, ?, ?, ?)'
    ).run(nanoid(), eventType, conversationId || null, metadata ? JSON.stringify(metadata) : null)
  }
}

// Re-export types
export type { Tone, Conversation, Reply, ConversationWithReplies }
