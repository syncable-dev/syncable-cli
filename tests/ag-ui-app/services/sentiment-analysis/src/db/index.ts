import { db } from './schema.js'
import { nanoid } from 'nanoid'
import crypto from 'crypto'

export interface SentimentResult {
  sentiment: 'positive' | 'negative' | 'neutral' | 'mixed'
  confidence: number
  emotions: Array<{ emotion: string; score: number }>
  urgency: 'low' | 'medium' | 'high' | 'critical'
  keyPoints: string[]
  suggestedApproach: string
}

export interface CachedResult {
  id: string
  message_hash: string
  result: string
  created_at: string
}

function hashMessage(message: string): string {
  return crypto.createHash('sha256').update(message.toLowerCase().trim()).digest('hex')
}

export const dbOps = {
  // Check cache for existing analysis
  getCached(message: string): SentimentResult | null {
    const hash = hashMessage(message)
    const cached = db.prepare<string, CachedResult>(
      'SELECT result FROM sentiment_cache WHERE message_hash = ?'
    ).get(hash)

    if (cached) {
      return JSON.parse(cached.result)
    }
    return null
  },

  // Save analysis to cache
  saveCache(message: string, result: SentimentResult): void {
    const hash = hashMessage(message)
    const id = nanoid()

    db.prepare(`
      INSERT OR REPLACE INTO sentiment_cache (id, message_hash, result)
      VALUES (?, ?, ?)
    `).run(id, hash, JSON.stringify(result))
  },

  // Get emotion patterns for fallback detection
  getEmotionPatterns(): Array<{ pattern: string; emotion: string; weight: number }> {
    return db.prepare<[], { pattern: string; emotion: string; weight: number }>(
      'SELECT pattern, emotion, weight FROM emotion_patterns'
    ).all()
  },

  // Clear old cache entries (older than 24 hours)
  cleanCache(): number {
    const result = db.prepare(`
      DELETE FROM sentiment_cache
      WHERE created_at < datetime('now', '-24 hours')
    `).run()
    return result.changes
  }
}
