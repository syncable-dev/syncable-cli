import { db } from './schema.js';
import { nanoid } from 'nanoid';
export const dbOps = {
    // Save a writing sample
    saveSample(content, type) {
        const id = nanoid();
        const words = content.split(/\s+/).filter(w => w.length > 0);
        const sentences = content.split(/[.!?]+/).filter(s => s.trim().length > 0);
        db.prepare(`
      INSERT INTO style_samples (id, content, type, word_count, sentence_count)
      VALUES (?, ?, ?, ?, ?)
    `).run(id, content, type, words.length, sentences.length);
        return {
            id,
            content,
            type,
            word_count: words.length,
            sentence_count: sentences.length,
            created_at: new Date().toISOString()
        };
    },
    // Update or increment a pattern
    updatePattern(type, value) {
        const existing = db.prepare('SELECT * FROM style_patterns WHERE pattern_type = ? AND value = ?').get(type, value);
        if (existing) {
            db.prepare(`
        UPDATE style_patterns
        SET frequency = frequency + 1, last_used = CURRENT_TIMESTAMP
        WHERE id = ?
      `).run(existing.id);
        }
        else {
            const id = nanoid();
            db.prepare(`
        INSERT INTO style_patterns (id, pattern_type, value, frequency)
        VALUES (?, ?, ?, 1)
      `).run(id, type, value);
        }
    },
    // Get patterns by type
    getPatterns(type, limit = 10) {
        return db.prepare('SELECT * FROM style_patterns WHERE pattern_type = ? ORDER BY frequency DESC LIMIT ?').all(type, limit);
    },
    // Get all patterns
    getAllPatterns() {
        return db.prepare('SELECT * FROM style_patterns ORDER BY pattern_type, frequency DESC').all();
    },
    // Get the style profile
    getProfile() {
        return db.prepare('SELECT * FROM style_profile WHERE id = ?').get('default');
    },
    // Update the style profile
    updateProfile(updates) {
        const sets = [];
        const values = [];
        for (const [key, value] of Object.entries(updates)) {
            if (value !== undefined) {
                sets.push(`${key} = ?`);
                values.push(value);
            }
        }
        if (sets.length > 0) {
            sets.push('updated_at = CURRENT_TIMESTAMP');
            values.push('default');
            db.prepare(`UPDATE style_profile SET ${sets.join(', ')} WHERE id = ?`).run(...values);
        }
    },
    // Get sample count
    getSampleCount() {
        const result = db.prepare('SELECT COUNT(*) as count FROM style_samples').get();
        return result?.count || 0;
    },
    // Get recent samples
    getRecentSamples(limit = 20) {
        return db.prepare('SELECT * FROM style_samples ORDER BY created_at DESC LIMIT ?').all(limit);
    },
    // Clear all data
    clearAll() {
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
    `);
    }
};
