import Database, { type Database as DatabaseType } from 'better-sqlite3'
import path from 'path'
import fs from 'fs'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Use absolute path in production, relative in development
const dbPath = process.env.DB_PATH || 
  (process.env.NODE_ENV === 'production' 
    ? '/app/data/sentiment.db'
    : path.join(__dirname, '../../data/sentiment.db'))

// Ensure directory exists
const dbDir = path.dirname(dbPath)
if (!fs.existsSync(dbDir)) {
  fs.mkdirSync(dbDir, { recursive: true })
}

export const db: DatabaseType = new Database(dbPath)

// Enable WAL mode for better concurrent access
db.pragma('journal_mode = WAL')

// Create tables
db.exec(`
  CREATE TABLE IF NOT EXISTS sentiment_cache (
    id TEXT PRIMARY KEY,
    message_hash TEXT UNIQUE,
    result TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS emotion_patterns (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    emotion TEXT NOT NULL,
    weight REAL DEFAULT 1.0
  );

  CREATE INDEX IF NOT EXISTS idx_cache_hash ON sentiment_cache(message_hash);
  CREATE INDEX IF NOT EXISTS idx_patterns_emotion ON emotion_patterns(emotion);
`)

// Seed some basic emotion patterns for fallback detection
const seedPatterns = [
  { pattern: 'frustrated', emotion: 'frustrated', weight: 0.9 },
  { pattern: 'annoyed', emotion: 'frustrated', weight: 0.8 },
  { pattern: 'angry', emotion: 'angry', weight: 0.9 },
  { pattern: 'furious', emotion: 'angry', weight: 1.0 },
  { pattern: 'happy', emotion: 'happy', weight: 0.9 },
  { pattern: 'excited', emotion: 'happy', weight: 0.8 },
  { pattern: 'confused', emotion: 'confused', weight: 0.9 },
  { pattern: "don't understand", emotion: 'confused', weight: 0.8 },
  { pattern: 'anxious', emotion: 'anxious', weight: 0.9 },
  { pattern: 'worried', emotion: 'anxious', weight: 0.8 },
  { pattern: 'urgent', emotion: 'urgent', weight: 0.9 },
  { pattern: 'asap', emotion: 'urgent', weight: 0.8 },
  { pattern: 'immediately', emotion: 'urgent', weight: 0.9 },
  { pattern: 'thank', emotion: 'grateful', weight: 0.8 },
  { pattern: 'appreciate', emotion: 'grateful', weight: 0.9 },
  { pattern: 'grateful', emotion: 'grateful', weight: 1.0 },
]

const insertPattern = db.prepare(`
  INSERT OR IGNORE INTO emotion_patterns (id, pattern, emotion, weight)
  VALUES (?, ?, ?, ?)
`)

for (const p of seedPatterns) {
  insertPattern.run(`seed-${p.pattern}`, p.pattern, p.emotion, p.weight)
}

console.log('[Sentiment DB] Database initialized')
