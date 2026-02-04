import { db } from './schema.js'
import { nanoid } from 'nanoid'

export type SampleType = 'selected_reply' | 'custom_edit' | 'sent_message'
export type PatternType = 'greeting' | 'signoff' | 'phrase' | 'word'
export type VocabularyLevel = 'professional' | 'casual' | 'mixed'

export interface StyleSample {
  id: string
  content: string
  type: SampleType
  word_count: number
  sentence_count: number
  created_at: string
}

export interface StylePattern {
  id: string
  pattern_type: PatternType
  value: string
  frequency: number
  last_used: string
}

export interface StyleProfile {
  id: string
  avg_sentence_length: number
  avg_word_length: number
  vocabulary_level: VocabularyLevel
  emoji_usage: number
  exclamation_frequency: number
  question_frequency: number
  total_samples: number
  updated_at: string
}

export const dbOps = {
  // Save a writing sample
  saveSample(content: string, type: SampleType): StyleSample {
    const id = nanoid()
    const words = content.split(/\s+/).filter(w => w.length > 0)
    const sentences = content.split(/[.!?]+/).filter(s => s.trim().length > 0)

    db.prepare(`
      INSERT INTO style_samples (id, content, type, word_count, sentence_count)
      VALUES (?, ?, ?, ?, ?)
    `).run(id, content, type, words.length, sentences.length)

    return {
      id,
      content,
      type,
      word_count: words.length,
      sentence_count: sentences.length,
      created_at: new Date().toISOString()
    }
  },

  // Update or increment a pattern
  updatePattern(type: PatternType, value: string): void {
    const existing = db.prepare<[string, string], StylePattern>(
      'SELECT * FROM style_patterns WHERE pattern_type = ? AND value = ?'
    ).get(type, value)

    if (existing) {
      db.prepare(`
        UPDATE style_patterns
        SET frequency = frequency + 1, last_used = CURRENT_TIMESTAMP
        WHERE id = ?
      `).run(existing.id)
    } else {
      const id = nanoid()
      db.prepare(`
        INSERT INTO style_patterns (id, pattern_type, value, frequency)
        VALUES (?, ?, ?, 1)
      `).run(id, type, value)
    }
  },

  // Get patterns by type
  getPatterns(type: PatternType, limit: number = 10): StylePattern[] {
    return db.prepare<[string, number], StylePattern>(
      'SELECT * FROM style_patterns WHERE pattern_type = ? ORDER BY frequency DESC LIMIT ?'
    ).all(type, limit)
  },

  // Get all patterns
  getAllPatterns(): StylePattern[] {
    return db.prepare<[], StylePattern>(
      'SELECT * FROM style_patterns ORDER BY pattern_type, frequency DESC'
    ).all()
  },

  // Get the style profile
  getProfile(): StyleProfile {
    return db.prepare<[string], StyleProfile>(
      'SELECT * FROM style_profile WHERE id = ?'
    ).get('default')!
  },

  // Update the style profile
  updateProfile(updates: Partial<Omit<StyleProfile, 'id' | 'updated_at'>>): void {
    const sets: string[] = []
    const values: (string | number)[] = []

    for (const [key, value] of Object.entries(updates)) {
      if (value !== undefined) {
        sets.push(`${key} = ?`)
        values.push(value)
      }
    }

    if (sets.length > 0) {
      sets.push('updated_at = CURRENT_TIMESTAMP')
      values.push('default')
      db.prepare(`UPDATE style_profile SET ${sets.join(', ')} WHERE id = ?`).run(...values)
    }
  },

  // Get sample count
  getSampleCount(): number {
    const result = db.prepare<[], { count: number }>(
      'SELECT COUNT(*) as count FROM style_samples'
    ).get()
    return result?.count || 0
  },

  // Get recent samples
  getRecentSamples(limit: number = 20): StyleSample[] {
    return db.prepare<number, StyleSample>(
      'SELECT * FROM style_samples ORDER BY created_at DESC LIMIT ?'
    ).all(limit)
  },

  // Clear all data
  clearAll(): void {
    db.exec(`
      DELETE FROM style_samples;
      DELETE FROM style_patterns;
      UPDATE style_profile SET
        avg_sentence_length = 0,
        avg_word_length = 0,
        vocabulary_level = 'mixed',
        emoji_usage = 0,
        exclamation_frequency = 0,
        question_frequency = 0,
        total_samples = 0,
        updated_at = CURRENT_TIMESTAMP
      WHERE id = 'default';
    `)
  }
}
