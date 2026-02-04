import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { serve } from '@hono/node-server'
import { dbOps, type ContactInput, type Relationship } from './db/index.js'
import { matchMessageToContact, getContactSuggestions } from './matcher.js'
import './db/schema.js' // Initialize database

const app = new Hono()

// Enable CORS for all origins (development)
app.use('*', cors())

// Health check
app.get('/health', (c) => {
  return c.json({
    status: 'ok',
    service: 'contact-intelligence',
    port: 3003
  })
})

// Validate relationship
function isValidRelationship(rel: string): rel is Relationship {
  return ['colleague', 'manager', 'client', 'vendor', 'friend', 'family', 'other'].includes(rel)
}

// Create a new contact
app.post('/api/contacts', async (c) => {
  try {
    const body = await c.req.json<ContactInput>()

    if (!body.name?.trim()) {
      return c.json({ error: 'Name is required' }, 400)
    }

    if (!body.relationship || !isValidRelationship(body.relationship)) {
      return c.json({
        error: 'Invalid relationship. Must be one of: colleague, manager, client, vendor, friend, family, other'
      }, 400)
    }

    const contact = dbOps.createContact({
      name: body.name.trim(),
      email: body.email?.trim(),
      relationship: body.relationship,
      company: body.company?.trim(),
      notes: body.notes?.trim(),
      preferences: body.preferences
    })

    console.log(`[Contacts] Created contact: ${contact.name} (${contact.id})`)

    return c.json({ id: contact.id, created: true, contact }, 201)
  } catch (error) {
    console.error('[Contacts] Create error:', error)
    return c.json({ error: 'Failed to create contact' }, 500)
  }
})

// Get all contacts
app.get('/api/contacts', (c) => {
  try {
    const contacts = dbOps.getAllContacts()
    return c.json({ contacts, count: contacts.length })
  } catch (error) {
    console.error('[Contacts] List error:', error)
    return c.json({ error: 'Failed to fetch contacts' }, 500)
  }
})

// Get a single contact
app.get('/api/contacts/:id', (c) => {
  try {
    const { id } = c.req.param()
    const contact = dbOps.getContact(id)

    if (!contact) {
      return c.json({ error: 'Contact not found' }, 404)
    }

    const suggestions = getContactSuggestions(contact)
    const interactions = dbOps.getInteractions(id, 5)

    return c.json({ contact, suggestions, recentInteractions: interactions })
  } catch (error) {
    console.error('[Contacts] Get error:', error)
    return c.json({ error: 'Failed to fetch contact' }, 500)
  }
})

// Update a contact
app.put('/api/contacts/:id', async (c) => {
  try {
    const { id } = c.req.param()
    const body = await c.req.json<Partial<ContactInput>>()

    if (body.relationship && !isValidRelationship(body.relationship)) {
      return c.json({
        error: 'Invalid relationship. Must be one of: colleague, manager, client, vendor, friend, family, other'
      }, 400)
    }

    const contact = dbOps.updateContact(id, body)

    if (!contact) {
      return c.json({ error: 'Contact not found' }, 404)
    }

    console.log(`[Contacts] Updated contact: ${contact.name} (${contact.id})`)

    return c.json({ contact, updated: true })
  } catch (error) {
    console.error('[Contacts] Update error:', error)
    return c.json({ error: 'Failed to update contact' }, 500)
  }
})

// Delete a contact
app.delete('/api/contacts/:id', (c) => {
  try {
    const { id } = c.req.param()
    const deleted = dbOps.deleteContact(id)

    if (!deleted) {
      return c.json({ error: 'Contact not found' }, 404)
    }

    console.log(`[Contacts] Deleted contact: ${id}`)

    return c.json({ deleted: true })
  } catch (error) {
    console.error('[Contacts] Delete error:', error)
    return c.json({ error: 'Failed to delete contact' }, 500)
  }
})

// Match a message to a contact
app.post('/api/contacts/match', async (c) => {
  try {
    const body = await c.req.json<{ message: string }>()
    const { message } = body

    if (!message?.trim()) {
      return c.json({ error: 'Message is required' }, 400)
    }

    console.log(`[Contacts] Matching message: "${message.substring(0, 50)}..."`)

    const result = matchMessageToContact(message.trim())

    return c.json(result)
  } catch (error) {
    console.error('[Contacts] Match error:', error)
    return c.json({ error: 'Failed to match contact' }, 500)
  }
})

// Record an interaction
app.post('/api/contacts/:id/interactions', async (c) => {
  try {
    const { id } = c.req.param()
    const body = await c.req.json<{
      conversationId: string
      direction: 'inbound' | 'outbound'
      summary: string
    }>()

    const contact = dbOps.getContact(id)
    if (!contact) {
      return c.json({ error: 'Contact not found' }, 404)
    }

    dbOps.recordInteraction(id, body.conversationId, body.direction, body.summary)

    return c.json({ recorded: true })
  } catch (error) {
    console.error('[Contacts] Interaction error:', error)
    return c.json({ error: 'Failed to record interaction' }, 500)
  }
})

// Start server
const PORT = parseInt(process.env.PORT || '3003', 10)

console.log(`
========================================
  Contact Intelligence Service
  Port: ${PORT}
  Endpoints:
    GET    /health
    POST   /api/contacts
    GET    /api/contacts
    GET    /api/contacts/:id
    PUT    /api/contacts/:id
    DELETE /api/contacts/:id
    POST   /api/contacts/match
    POST   /api/contacts/:id/interactions
========================================
`)

serve({
  fetch: app.fetch,
  port: PORT
})
