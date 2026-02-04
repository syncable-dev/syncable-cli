import Database, { type Database as DatabaseType } from 'better-sqlite3'
import path from 'path'
import fs from 'fs'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

// Use absolute path in production, relative in development
const dbPath = process.env.DB_PATH || 
  (process.env.NODE_ENV === 'production' 
    ? '/app/data/style.db'
    : path.join(__dirname, '../../data/style.db'))

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
  CREATE TABLE IF NOT EXISTS style_samples (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    type TEXT NOT NULL,
    word_count INTEGER,
    sentence_count INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS style_patterns (
    id TEXT PRIMARY KEY,
    pattern_type TEXT NOT NULL,
    value TEXT NOT NULL,
    frequency INTEGER DEFAULT 1,
    last_used DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS style_profile (
    id TEXT PRIMARY KEY DEFAULT 'default',
    avg_sentence_length REAL DEFAULT 0,
    avg_word_length REAL DEFAULT 0,
    vocabulary_level TEXT DEFAULT 'mixed',
    emoji_usage REAL DEFAULT 0,
    exclamation_frequency REAL DEFAULT 0,
    question_frequency REAL DEFAULT 0,
    total_samples INTEGER DEFAULT 0,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE INDEX IF NOT EXISTS idx_patterns_type ON style_patterns(pattern_type);
  CREATE INDEX IF NOT EXISTS idx_samples_type ON style_samples(type);

  -- Initialize default profile if not exists
  INSERT OR IGNORE INTO style_profile (id) VALUES ('default');
`)

console.log('[Style DB] Database initialized')
