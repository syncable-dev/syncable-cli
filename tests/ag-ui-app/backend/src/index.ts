import { serve } from '@hono/node-server'
import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { logger } from 'hono/logger'
import { repliesRouter } from './routes/replies.js'
import { historyRouter } from './routes/history.js'
import { checkServicesHealth } from './lib/services.js'
import 'dotenv/config'

// Create Hono app
const app = new Hono()

// Middleware
app.use('*', logger())
app.use('*', cors({
  origin: (origin) => {
    // Allow requests with no origin (e.g., mobile apps, curl)
    if (!origin) return origin
    // Allow localhost for development
    if (origin.startsWith('http://localhost:') || origin.startsWith('http://127.0.0.1:')) {
      return origin
    }
    // Allow all syncable.dev subdomains (production)
    if (origin.endsWith('.syncable.dev')) {
      return origin
    }
    return null
  },
  allowMethods: ['GET', 'POST', 'DELETE', 'OPTIONS'],
  allowHeaders: ['Content-Type', 'Authorization'],
  exposeHeaders: ['Content-Length'],
  maxAge: 86400,
  credentials: true
}))

// Health check endpoint
app.get('/health', (c) => {
  return c.json({
    status: 'ok',
    timestamp: new Date().toISOString(),
    service: 'smart-reply-backend'
  })
})

// Services health check (includes microservices)
app.get('/health/services', async (c) => {
  const services = await checkServicesHealth()
  const allHealthy = services.sentiment && services.contacts && services.style

  return c.json({
    status: allHealthy ? 'ok' : 'degraded',
    timestamp: new Date().toISOString(),
    services: {
      'sentiment-analysis': services.sentiment ? 'up' : 'down',
      'contact-intelligence': services.contacts ? 'up' : 'down',
      'writing-style': services.style ? 'up' : 'down'
    }
  })
})

// API routes
app.route('/api/replies', repliesRouter)
app.route('/api/history', historyRouter)

// 404 handler
app.notFound((c) => {
  return c.json({ error: 'Not found' }, 404)
})

// Error handler
app.onError((err, c) => {
  console.error('Unhandled error:', err)
  return c.json({ error: 'Internal server error' }, 500)
})

// Start server
const port = parseInt(process.env.PORT || '3001', 10)

serve({ fetch: app.fetch, port }, (info) => {
  console.log(`
  ╔═══════════════════════════════════════════════════════╗
  ║                                                       ║
  ║   Smart Reply Backend                                 ║
  ║   Running at http://localhost:${info.port}                 ║
  ║                                                       ║
  ║   Endpoints:                                          ║
  ║   • GET  /health              - Health check          ║
  ║   • GET  /health/services     - Services status       ║
  ║   • POST /api/replies/generate - Generate replies     ║
  ║   • POST /api/replies/save    - Save replies          ║
  ║   • POST /api/replies/learn   - Learn from reply      ║
  ║   • GET  /api/history         - List conversations    ║
  ║   • GET  /api/history/:id     - Get conversation      ║
  ║   • DELETE /api/history/:id   - Delete conversation   ║
  ║                                                       ║
  ║   Microservices:                                      ║
  ║   • Sentiment Analysis  - localhost:3002              ║
  ║   • Contact Intelligence - localhost:3003             ║
  ║   • Writing Style       - localhost:3004              ║
  ║                                                       ║
  ╚═══════════════════════════════════════════════════════╝
  `)
})
