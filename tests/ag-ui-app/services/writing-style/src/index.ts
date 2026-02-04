import { Hono } from 'hono'
import { cors } from 'hono/cors'
import { serve } from '@hono/node-server'
import { dbOps, type SampleType } from './db/index.js'
import { learnFromSample, enhanceReply } from './learner.js'
import './db/schema.js' // Initialize database

const app = new Hono()

// Enable CORS for all origins (development)
app.use('*', cors())

// Health check
app.get('/health', (c) => {
  return c.json({
    status: 'ok',
    service: 'writing-style',
    port: 3004
  })
})

// Validate sample type
function isValidSampleType(type: string): type is SampleType {
  return ['selected_reply', 'custom_edit', 'sent_message'].includes(type)
}

// Learn from a writing sample
app.post('/api/learn', async (c) => {
  
  try {
    const body = await c.req.json<{ content: string; type: string }>()
    const { content, type } = body

    if (!content?.trim()) {
      return c.json({ error: 'Content is required' }, 400)
    }

    if (!type || !isValidSampleType(type)) {
      return c.json({
        error: 'Invalid type. Must be one of: selected_reply, custom_edit, sent_message'
      }, 400)
    }

    console.log(`[Style] Learning from ${type}: "${content.substring(0, 50)}..."`)

    const result = learnFromSample(content.trim(), type)

    return c.json({
      learned: result.learned,
      patterns_updated: result.patternsUpdated,
      extracted: result.extracted
    })
  } catch (error) {
    console.error('[Style] Learn error:', error)
    return c.json({ error: 'Failed to learn from sample' }, 500)
  }
})

// Get the user's style profile
app.get('/api/profile', (c) => {
  try {
    const profile = dbOps.getProfile()
    const greetings = dbOps.getPatterns('greeting', 5)
    const signoffs = dbOps.getPatterns('signoff', 5)
    const phrases = dbOps.getPatterns('phrase', 10)

    return c.json({
      totalSamples: profile.total_samples,
      averageSentenceLength: profile.avg_sentence_length,
      commonGreetings: greetings.map(p => p.value),
      commonSignoffs: signoffs.map(p => p.value),
      vocabularyLevel: profile.vocabulary_level,
      usesEmojis: profile.emoji_usage > 0.01,
      emojiFrequency: profile.emoji_usage,
      commonPhrases: phrases.map(p => p.value),
      punctuationStyle: {
        exclamationFrequency: profile.exclamation_frequency,
        questionFrequency: profile.question_frequency
      },
      lastUpdated: profile.updated_at
    })
  } catch (error) {
    console.error('[Style] Profile error:', error)
    return c.json({ error: 'Failed to get profile' }, 500)
  }
})

// Enhance a reply based on learned style
app.post('/api/enhance', async (c) => {
  try {
    const body = await c.req.json<{ reply: string }>()
    const { reply } = body

    if (!reply?.trim()) {
      return c.json({ error: 'Reply is required' }, 400)
    }

    console.log(`[Style] Enhancing reply: "${reply.substring(0, 50)}..."`)

    const result = enhanceReply(reply.trim())

    return c.json(result)
  } catch (error) {
    console.error('[Style] Enhance error:', error)
    return c.json({ error: 'Failed to enhance reply' }, 500)
  }
})

// Get all patterns (for debugging/admin)
app.get('/api/patterns', (c) => {
  try {
    const patterns = dbOps.getAllPatterns()
    return c.json({
      patterns,
      count: patterns.length
    })
  } catch (error) {
    console.error('[Style] Patterns error:', error)
    return c.json({ error: 'Failed to get patterns' }, 500)
  }
})

// Clear all learned data
app.delete('/api/profile', (c) => {
  try {
    dbOps.clearAll()
    console.log('[Style] Profile cleared')
    return c.json({ cleared: true })
  } catch (error) {
    console.error('[Style] Clear error:', error)
    return c.json({ error: 'Failed to clear profile' }, 500)
  }
})

// Start server
const PORT = parseInt(process.env.PORT || '3004', 10)

console.log(`
========================================
  Writing Style Service
  Port: ${PORT}
  Endpoints:
    GET    /health
    POST   /api/learn
    GET    /api/profile
    POST   /api/enhance
    GET    /api/patterns
    DELETE /api/profile
========================================
`)

serve({
  fetch: app.fetch,
  port: PORT
})
