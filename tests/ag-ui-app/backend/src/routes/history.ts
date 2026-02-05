import { Hono } from 'hono'
import { dbOps } from '../db/index.js'

const historyRouter = new Hono()

// GET /api/history - Get all conversations with replies
historyRouter.get('/', (c) => {
  try {
    const limitParam = c.req.query('limit')
    const limit = limitParam ? Math.min(Math.max(parseInt(limitParam, 10), 1), 100) : 50

    const conversations = dbOps.getHistory(limit)

    return c.json({
      success: true,
      data: conversations,
      count: conversations.length
    })
  } catch (error) {
    console.error('Get history error:', error)
    return c.json({ error: 'Failed to fetch history' }, 500)
  }
})

// GET /api/history/:id - Get single conversation with replies
historyRouter.get('/:id', (c) => {
  try {
    const { id } = c.req.param()

    const conversation = dbOps.getConversation(id)

    if (!conversation) {
      return c.json({ error: 'Conversation not found' }, 404)
    }

    return c.json({
      success: true,
      data: conversation
    })
  } catch (error) {
    console.error('Get conversation error:', error)
    return c.json({ error: 'Failed to fetch conversation' }, 500)
  }
})

// DELETE /api/history/:id - Delete a conversation
historyRouter.delete('/:id', (c) => {
  try {
    const { id } = c.req.param()

    const deleted = dbOps.deleteConversation(id)

    if (!deleted) {
      return c.json({ error: 'Conversation not found' }, 404)
    }

    dbOps.logEvent('delete', id)

    return c.json({
      success: true,
      message: 'Conversation deleted successfully'
    })
  } catch (error) {
    console.error('Delete conversation error:', error)
    return c.json({ error: 'Failed to delete conversation' }, 500)
  }
})

export { historyRouter }
