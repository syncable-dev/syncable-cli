import { useState, useCallback } from 'react'
import type { Tone, Reply } from '@/lib/api'
import { api } from '@/lib/api'

export interface UseSmartReplyReturn {
  isGenerating: boolean
  streamedContent: string
  replies: Reply[]
  conversationId: string | null
  error: string | null
  generate: (message: string, tone: Tone, context?: string, intent?: string) => Promise<void>
  stop: () => void
  reset: () => void
  setReplies: (replies: Reply[]) => void
}

export function useSmartReply(): UseSmartReplyReturn {
  const [isGenerating, setIsGenerating] = useState(false)
  const [streamedContent, setStreamedContent] = useState('')
  const [replies, setReplies] = useState<Reply[]>([])
  const [conversationId, setConversationId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [abortController, setAbortController] = useState<AbortController | null>(null)

  const generate = useCallback(async (message: string, tone: Tone, context?: string, intent?: string) => {
    // Reset state
    setIsGenerating(true)
    setReplies([])
    setConversationId(null)
    setStreamedContent('')
    setError(null)

    // Create abort controller for cancellation
    const controller = new AbortController()
    setAbortController(controller)

    try {
      // Use the centralized API client to get the correct base URL
      const response = await api.generateReplies(message, tone, context, intent, controller.signal)

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}))
        throw new Error(errorData.error || `HTTP ${response.status}`)
      }

      // Get conversation ID from header
      const convId = response.headers.get('X-Conversation-Id')
      if (convId) {
        setConversationId(convId)
      }

      // Process the SSE stream
      const reader = response.body?.getReader()
      if (!reader) {
        throw new Error('No response body')
      }

      const decoder = new TextDecoder()
      let accumulated = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        const text = decoder.decode(value, { stream: true })
        const lines = text.split('\n')

        for (const line of lines) {
          if (line.startsWith('data: ')) {
            const data = line.slice(6).trim()
            if (data === '[DONE]') continue

            try {
              const chunk = JSON.parse(data)

              // Handle different chunk types from TanStack AI
              if (chunk.type === 'text' && chunk.content) {
                accumulated += chunk.content
                setStreamedContent(accumulated)
              } else if (chunk.type === 'content' && chunk.content) {
                accumulated = chunk.content
                setStreamedContent(accumulated)
              } else if (chunk.type === 'error') {
                throw new Error(chunk.error?.message || 'Stream error')
              }
            } catch (parseErr) {
              // Skip malformed chunks but log them
              if (data && !data.startsWith('{')) {
                // Might be raw text content
                accumulated += data
                setStreamedContent(accumulated)
              }
            }
          }
        }
      }

      // Parse replies from accumulated content
      const parsedReplies = parseReplies(accumulated)
      setReplies(parsedReplies)

      // Save replies to backend
      if (convId && parsedReplies.length > 0) {
        try {
          await api.saveReplies(convId, parsedReplies.map(r => r.content))
        } catch (e) {
          console.error('Failed to save replies:', e)
        }
      }

    } catch (err) {
      if (err instanceof Error && err.name === 'AbortError') {
        // User cancelled - not an error
        return
      }
      const errorMessage = err instanceof Error ? err.message : 'Failed to generate replies'
      setError(errorMessage)
      console.error('Generation error:', err)
    } finally {
      setIsGenerating(false)
      setAbortController(null)
    }
  }, [])

  const stop = useCallback(() => {
    if (abortController) {
      abortController.abort()
      setIsGenerating(false)
    }
  }, [abortController])

  const reset = useCallback(() => {
    stop()
    setStreamedContent('')
    setReplies([])
    setConversationId(null)
    setError(null)
  }, [stop])

  return {
    isGenerating,
    streamedContent,
    replies,
    conversationId,
    error,
    generate,
    stop,
    reset,
    setReplies
  }
}

/**
 * Parse JSON array of replies from AI response
 */
function parseReplies(content: string): Reply[] {
  try {
    // Find JSON array in content
    const jsonMatch = content.match(/\[[\s\S]*\]/)
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[0])
      if (Array.isArray(parsed)) {
        return parsed.slice(0, 3).map((text: string, index: number) => ({
          id: `reply-${Date.now()}-${index}`,
          content: String(text),
          reply_index: index
        }))
      }
    }
  } catch (e) {
    console.error('Failed to parse replies:', e, content)
  }
  return []
}
