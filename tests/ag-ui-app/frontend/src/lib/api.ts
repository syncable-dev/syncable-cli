// API Configuration

import { createServerFn } from "@tanstack/react-start"

const getApiBase = createServerFn({ method: 'GET' }).handler(() => {
  const base = process.env.API_BASE || 'http://localhost:3001'
  // Ensure /api suffix
  return base.endsWith('/api') ? base : `${base}/api`
})

// Types
export type Tone = 'professional' | 'friendly' | 'apologetic' | 'assertive' | 'neutral'

export interface Reply {
  id: string
  content: string
  reply_index: number
}

export interface Conversation {
  id: string
  original_message: string
  context?: string
  intent?: string
  tone: Tone
  created_at: string
  replies: Reply[]
}

export interface GenerateResponse {
  type: 'chunk' | 'complete' | 'error'
  content?: string
  conversationId: string
  replies?: Reply[]
  error?: string
}

export interface HistoryResponse {
  success: boolean
  data: Conversation[]
  count: number
}

export interface ConversationResponse {
  success: boolean
  data: Conversation
}

// API Client
export const api = {
  /**
   * Generate smart replies with SSE streaming
   * Returns a fetch Response that can be streamed
   */
  generateReplies: async (message: string, tone: Tone, context?: string, intent?: string, signal?: AbortSignal): Promise<Response> => {
    return fetch(`${await getApiBase()}/replies/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, tone, context, intent }),
      signal
    })
  },

  /**
   * Get conversation history
   */
  getHistory: async (limit: number = 50): Promise<Conversation[]> => {
    const res = await fetch(`${await getApiBase()}/history?limit=${limit}`)
    if (!res.ok) {
      throw new Error('Failed to fetch history')
    }
    const data: HistoryResponse = await res.json()
    return data.data
  },

  /**
   * Get a single conversation by ID
   */
  getConversation: async (id: string): Promise<Conversation> => {
    const res = await fetch(`${await getApiBase()}/history/${id}`)
    if (!res.ok) {
      throw new Error('Conversation not found')
    }
    const data: ConversationResponse = await res.json()
    return data.data
  },

  /**
   * Delete a conversation
   */
  deleteConversation: async (id: string): Promise<void> => {
    const res = await fetch(`${await getApiBase()}/history/${id}`, { method: 'DELETE' })
    if (!res.ok) {
      throw new Error('Failed to delete conversation')
    }
  },

  /**
   * Save generated replies to database
   */
  saveReplies: async (conversationId: string, replies: string[]): Promise<void> => {
    const res = await fetch(`${await getApiBase()}/replies/save`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ conversationId, replies })
    })
    if (!res.ok) {
      throw new Error('Failed to save replies')
    }
  },

  /**
   * Health check
   */
  healthCheck: async (): Promise<boolean> => {
    try {
      const res = await fetch(`${await getApiBase()}.replace('/api', '')}/health`)
      return res.ok
    } catch {
      return false
    }
  }
}

// Utility function to parse SSE events from a stream
export async function* parseSSEStream(response: Response): AsyncGenerator<GenerateResponse> {
  const reader = response.body?.getReader()
  if (!reader) {
    throw new Error('No response body')
  }

  const decoder = new TextDecoder()
  let buffer = ''

  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done) break

      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      buffer = lines.pop() || '' // Keep incomplete line in buffer

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6).trim()
          if (data === '[DONE]') {
            return
          }
          try {
            const parsed: GenerateResponse = JSON.parse(data)
            yield parsed
          } catch {
            // Skip malformed JSON
          }
        }
      }
    }
  } finally {
    reader.releaseLock()
  }
}
