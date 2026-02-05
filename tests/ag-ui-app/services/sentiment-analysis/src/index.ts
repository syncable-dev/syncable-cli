import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { serve } from '@hono/node-server'
import { analyzeSentiment } from './analyzer.js'
import './db/schema.js' // Initialize database

const app = new Hono()

// Enable CORS for all origins (development)
app.use('*', cors())

// Health check
app.get('/health', (c) => {
  return c.json({
    status: 'ok',
    service: 'sentiment-analysis',
    port: 3002
  })
})

// Analyze sentiment endpoint
app.post('/api/analyze', async (c) => {
  try {
    const body = await c.req.json<{ message: string }>()
    const { message } = body

    if (!message?.trim()) {
      return c.json({ error: 'Message is required' }, 400)
    }

    if (message.length > 10000) {
      return c.json({ error: 'Message too long. Maximum 10000 characters.' }, 400)
    }

    console.log(`[Sentiment] Analyzing message: "${message.substring(0, 50)}..."`)

    const result = await analyzeSentiment(message.trim())

    return c.json(result)
  } catch (error) {
    console.error('[Sentiment] Analysis error:', error)
    return c.json({ error: 'Failed to analyze sentiment' }, 500)
  }
})

// Start server
const PORT = parseInt(process.env.PORT || '3002', 10)

console.log(`
========================================
  Sentiment Analysis Service
  Port: ${PORT}
  Endpoints:
    GET  /health
    POST /api/analyze
========================================
`)

serve({
  fetch: app.fetch,
  port: PORT
})
