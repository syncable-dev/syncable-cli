import { Hono } from 'hono'
import { chat, toServerSentEventsResponse } from '@tanstack/ai'
import { dbOps, type Tone } from '../db/index.js'
import { getOpenAIAdapter, buildSmartReplyPrompt } from '../lib/openai.js'
import { buildEnhancedContext, learnFromReply } from '../lib/services.js'

const repliesRouter = new Hono()

// Validate tone
function isValidTone(tone: string): tone is Tone {
  return ['professional', 'friendly', 'apologetic', 'assertive', 'neutral'].includes(tone)
}

interface GenerateRequest {
  message: string
  tone: string
  context?: string
  intent?: string
}

/**
 * POST /api/replies/generate
 * Generate smart replies using TanStack AI with streaming
 */
repliesRouter.post('/generate', async (c) => {
  try {
    const body = await c.req.json<GenerateRequest>()
    const { message, tone, context, intent } = body

    // Validate input
    if (!message?.trim()) {
      return c.json({ error: 'Message is required' }, 400)
    }

    if (!tone || !isValidTone(tone)) {
      return c.json({
        error: 'Invalid tone. Must be one of: professional, friendly, apologetic, assertive, neutral'
      }, 400)
    }

    if (message.length > 5000) {
      return c.json({ error: 'Message too long. Maximum 5000 characters.' }, 400)
    }

    // Create conversation record with context and intent
    const conversationId = dbOps.createConversation(
      message.trim(),
      tone,
      context?.trim() || undefined,
      intent?.trim() || undefined
    )
    dbOps.logEvent('generate', conversationId, {
      tone,
      messageLength: message.length,
      hasContext: !!context,
      hasIntent: !!intent
    })

    // Get OpenAI adapter from TanStack AI
    const adapter = getOpenAIAdapter()

    // Build system prompt
    const systemPrompt = buildSmartReplyPrompt(tone)

    // Query microservices for enhanced context (non-blocking, with timeout)
    const enhancedContext = await buildEnhancedContext(message.trim())

    // Build user message with context, intent, and service insights
    const userMessage = buildUserMessage(message.trim(), context?.trim(), intent?.trim(), enhancedContext)

    // Create streaming chat with TanStack AI
    const stream = chat({
      adapter,
      systemPrompts: [systemPrompt],
      messages: [
        {
          role: 'user',
          content: userMessage
        }
      ],
      temperature: 0.8,
      maxTokens: 2000,
    })

    // Convert to Server-Sent Events response using TanStack AI utility
    const response = toServerSentEventsResponse(stream)

    // Add custom header with conversation ID
    const headers = new Headers(response.headers)
    headers.set('X-Conversation-Id', conversationId)
    headers.set('Access-Control-Expose-Headers', 'X-Conversation-Id')

    return new Response(response.body, {
      status: response.status,
      headers
    })

  } catch (error) {
    console.error('Generate endpoint error:', error)
    return c.json({ error: 'Internal server error' }, 500)
  }
})

/**
 * Build the user message with context, intent, and service insights
 */
function buildUserMessage(message: string, context?: string, intent?: string, enhancedContext?: string): string {
  let parts: string[] = []

  // Add AI-analyzed context from microservices
  if (enhancedContext) {
    parts.push(`AI-ANALYZED INSIGHTS:\n${enhancedContext}`)
  }

  if (context) {
    parts.push(`CONVERSATION CONTEXT (from user):\n${context}`)
  }

  parts.push(`MESSAGE RECEIVED:\n"${message}"`)

  if (intent) {
    parts.push(`WHAT I WANT TO COMMUNICATE:\n${intent}`)
  }

  parts.push('Generate 3 reply options based on the above information. Consider the sentiment, relationship context, and user\'s writing style if provided:')

  return parts.join('\n\n')
}

/**
 * POST /api/replies/save
 * Save generated replies to database (called after streaming completes)
 */
repliesRouter.post('/save', async (c) => {
  try {
    const { conversationId, replies } = await c.req.json<{
      conversationId: string
      replies: string[]
    }>()

    if (!conversationId || !replies || !Array.isArray(replies)) {
      return c.json({ error: 'Invalid request body' }, 400)
    }

    // Save replies to database
    dbOps.saveReplies(conversationId, replies.slice(0, 3))

    return c.json({ success: true })
  } catch (error) {
    console.error('Save replies error:', error)
    return c.json({ error: 'Failed to save replies' }, 500)
  }
})

/**
 * POST /api/replies/learn
 * Learn from a selected/copied reply to improve style matching
 */
repliesRouter.post('/learn', async (c) => {
  try {
    const { content, type } = await c.req.json<{
      content: string
      type: 'selected_reply' | 'custom_edit' | 'sent_message'
    }>()

    if (!content?.trim()) {
      return c.json({ error: 'Content is required' }, 400)
    }

    const validTypes = ['selected_reply', 'custom_edit', 'sent_message']
    if (!type || !validTypes.includes(type)) {
      return c.json({ error: 'Invalid type' }, 400)
    }

    // Send to writing style service (non-blocking)
    const success = await learnFromReply(content.trim(), type)

    return c.json({ success, learned: success })
  } catch (error) {
    console.error('Learn endpoint error:', error)
    return c.json({ error: 'Failed to learn from reply' }, 500)
  }
})

export { repliesRouter }
