import { db } from './schema.js'
import { nanoid } from 'nanoid'

export type Relationship = 'colleague' | 'manager' | 'client' | 'vendor' | 'friend' | 'family' | 'other'
export type Formality = 'formal' | 'casual' | 'adaptive'

export interface Contact {
  id: string
  name: string
  email: string | null
  relationship: Relationship
  company: string | null
  notes: string | null
  formality: Formality
  use_emojis: boolean
  preferred_tone: string | null
  created_at: string
  updated_at: string
}

export interface ContactInput {
  name: string
  email?: string
  relationship: Relationship
  company?: string
  notes?: string
  preferences?: {
    formality?: Formality
    useEmojis?: boolean
    preferredTone?: string
  }
}

export interface ContactInteraction {
  id: string
  contact_id: string
  conversation_id: string
  direction: 'inbound' | 'outbound'
  summary: string
  created_at: string
}

interface DBContact {
  id: string
  name: string
  email: string | null
  relationship: string
  company: string | null
  notes: string | null
  formality: string
  use_emojis: number
  preferred_tone: string | null
  created_at: string
  updated_at: string
}

function rowToContact(row: DBContact): Contact {
  return {
    ...row,
    relationship: row.relationship as Relationship,
    formality: row.formality as Formality,
    use_emojis: row.use_emojis === 1
  }
}

export const dbOps = {
  // Create a new contact
  createContact(input: ContactInput): Contact {
    const id = nanoid()
    const now = new Date().toISOString()

    db.prepare(`
      INSERT INTO contacts (id, name, email, relationship, company, notes, formality, use_emojis, preferred_tone, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      id,
      input.name,
      input.email || null,
      input.relationship,
      input.company || null,
      input.notes || null,
      input.preferences?.formality || 'adaptive',
      input.preferences?.useEmojis ? 1 : 0,
      input.preferences?.preferredTone || null,
      now,
      now
    )

    return this.getContact(id)!
  },

  // Get all contacts
  getAllContacts(): Contact[] {
    const rows = db.prepare<[], DBContact>('SELECT * FROM contacts ORDER BY name').all()
    return rows.map(rowToContact)
  },

  // Get a single contact
  getContact(id: string): Contact | null {
    const row = db.prepare<string, DBContact>('SELECT * FROM contacts WHERE id = ?').get(id)
    return row ? rowToContact(row) : null
  },

  // Update a contact
  updateContact(id: string, input: Partial<ContactInput>): Contact | null {
    const existing = this.getContact(id)
    if (!existing) return null

    const updates: string[] = []
    const values: (string | number | null)[] = []

    if (input.name !== undefined) {
      updates.push('name = ?')
      values.push(input.name)
    }
    if (input.email !== undefined) {
      updates.push('email = ?')
      values.push(input.email || null)
    }
    if (input.relationship !== undefined) {
      updates.push('relationship = ?')
      values.push(input.relationship)
    }
    if (input.company !== undefined) {
      updates.push('company = ?')
      values.push(input.company || null)
    }
    if (input.notes !== undefined) {
      updates.push('notes = ?')
      values.push(input.notes || null)
    }
    if (input.preferences?.formality !== undefined) {
      updates.push('formality = ?')
      values.push(input.preferences.formality)
    }
    if (input.preferences?.useEmojis !== undefined) {
      updates.push('use_emojis = ?')
      values.push(input.preferences.useEmojis ? 1 : 0)
    }
    if (input.preferences?.preferredTone !== undefined) {
      updates.push('preferred_tone = ?')
      values.push(input.preferences.preferredTone || null)
    }

    if (updates.length === 0) return existing

    updates.push('updated_at = ?')
    values.push(new Date().toISOString())
    values.push(id)

    db.prepare(`UPDATE contacts SET ${updates.join(', ')} WHERE id = ?`).run(...values)

    return this.getContact(id)
  },

  // Delete a contact
  deleteContact(id: string): boolean {
    const result = db.prepare('DELETE FROM contacts WHERE id = ?').run(id)
    return result.changes > 0
  },

  // Search contacts by name (fuzzy)
  searchContacts(query: string): Contact[] {
    const searchPattern = `%${query}%`
    const rows = db.prepare<[string, string], DBContact>(
      `SELECT * FROM contacts WHERE name LIKE ? OR email LIKE ? ORDER BY name`
    ).all(searchPattern, searchPattern)
    return rows.map(rowToContact)
  },

  // Record an interaction
  recordInteraction(contactId: string, conversationId: string, direction: 'inbound' | 'outbound', summary: string): void {
    const id = nanoid()
    db.prepare(`
      INSERT INTO contact_interactions (id, contact_id, conversation_id, direction, summary)
      VALUES (?, ?, ?, ?, ?)
    `).run(id, contactId, conversationId, direction, summary)
  },

  // Get interactions for a contact
  getInteractions(contactId: string, limit: number = 10): ContactInteraction[] {
    return db.prepare<[string, number], ContactInteraction>(
      `SELECT * FROM contact_interactions WHERE contact_id = ? ORDER BY created_at DESC LIMIT ?`
    ).all(contactId, limit)
  }
}
