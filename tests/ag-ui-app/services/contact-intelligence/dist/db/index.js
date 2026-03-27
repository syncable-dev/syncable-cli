import { db } from './schema.js';
import { nanoid } from 'nanoid';
function rowToContact(row) {
    return {
        ...row,
        relationship: row.relationship,
        formality: row.formality,
        use_emojis: row.use_emojis === 1
    };
}
export const dbOps = {
    // Create a new contact
    createContact(input) {
        const id = nanoid();
        const now = new Date().toISOString();
        db.prepare(`
      INSERT INTO contacts (id, name, email, relationship, company, notes, formality, use_emojis, preferred_tone, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(id, input.name, input.email || null, input.relationship, input.company || null, input.notes || null, input.preferences?.formality || 'adaptive', input.preferences?.useEmojis ? 1 : 0, input.preferences?.preferredTone || null, now, now);
        return this.getContact(id);
    },
    // Get all contacts
    getAllContacts() {
        const rows = db.prepare('SELECT * FROM contacts ORDER BY name').all();
        return rows.map(rowToContact);
    },
    // Get a single contact
    getContact(id) {
        const row = db.prepare('SELECT * FROM contacts WHERE id = ?').get(id);
        return row ? rowToContact(row) : null;
    },
    // Update a contact
    updateContact(id, input) {
        const existing = this.getContact(id);
        if (!existing)
            return null;
        const updates = [];
        const values = [];
        if (input.name !== undefined) {
            updates.push('name = ?');
            values.push(input.name);
        }
        if (input.email !== undefined) {
            updates.push('email = ?');
            values.push(input.email || null);
        }
        if (input.relationship !== undefined) {
            updates.push('relationship = ?');
            values.push(input.relationship);
        }
        if (input.company !== undefined) {
            updates.push('company = ?');
            values.push(input.company || null);
        }
        if (input.notes !== undefined) {
            updates.push('notes = ?');
            values.push(input.notes || null);
        }
        if (input.preferences?.formality !== undefined) {
            updates.push('formality = ?');
            values.push(input.preferences.formality);
        }
        if (input.preferences?.useEmojis !== undefined) {
            updates.push('use_emojis = ?');
            values.push(input.preferences.useEmojis ? 1 : 0);
        }
        if (input.preferences?.preferredTone !== undefined) {
            updates.push('preferred_tone = ?');
            values.push(input.preferences.preferredTone || null);
        }
        if (updates.length === 0)
            return existing;
        updates.push('updated_at = ?');
        values.push(new Date().toISOString());
        values.push(id);
        db.prepare(`UPDATE contacts SET ${updates.join(', ')} WHERE id = ?`).run(...values);
        return this.getContact(id);
    },
    // Delete a contact
    deleteContact(id) {
        const result = db.prepare('DELETE FROM contacts WHERE id = ?').run(id);
        return result.changes > 0;
    },
    // Search contacts by name (fuzzy)
    searchContacts(query) {
        const searchPattern = `%${query}%`;
        const rows = db.prepare(`SELECT * FROM contacts WHERE name LIKE ? OR email LIKE ? ORDER BY name`).all(searchPattern, searchPattern);
        return rows.map(rowToContact);
    },
    // Record an interaction
    recordInteraction(contactId, conversationId, direction, summary) {
        const id = nanoid();
        db.prepare(`
      INSERT INTO contact_interactions (id, contact_id, conversation_id, direction, summary)
      VALUES (?, ?, ?, ?, ?)
    `).run(id, contactId, conversationId, direction, summary);
    },
    // Get interactions for a contact
    getInteractions(contactId, limit = 10) {
        return db.prepare(`SELECT * FROM contact_interactions WHERE contact_id = ? ORDER BY created_at DESC LIMIT ?`).all(contactId, limit);
    }
};
