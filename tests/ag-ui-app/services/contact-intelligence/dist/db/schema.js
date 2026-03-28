import Database from 'better-sqlite3';
import path from 'path';
import { fileURLToPath } from 'url';
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const dbPath = path.join(__dirname, '../../data/contacts.db');
export const db = new Database(dbPath);
// Enable WAL mode for better concurrent access
db.pragma('journal_mode = WAL');
// Create tables
db.exec(`
  CREATE TABLE IF NOT EXISTS contacts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT,
    relationship TEXT NOT NULL,
    company TEXT,
    notes TEXT,
    formality TEXT DEFAULT 'adaptive',
    use_emojis INTEGER DEFAULT 0,
    preferred_tone TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE IF NOT EXISTS contact_interactions (
    id TEXT PRIMARY KEY,
    contact_id TEXT,
    conversation_id TEXT,
    direction TEXT,
    summary TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
  );

  CREATE INDEX IF NOT EXISTS idx_contacts_name ON contacts(name);
  CREATE INDEX IF NOT EXISTS idx_contacts_email ON contacts(email);
  CREATE INDEX IF NOT EXISTS idx_interactions_contact ON contact_interactions(contact_id);
`);
console.log('[Contacts DB] Database initialized');
