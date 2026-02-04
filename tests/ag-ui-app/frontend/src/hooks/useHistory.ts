import { useState, useEffect, useCallback } from 'react'
import { api, type Conversation } from '@/lib/api'

export interface UseHistoryReturn {
  // State
  history: Conversation[]
  isOpen: boolean
  isLoading: boolean
  error: string | null

  // Actions
  setIsOpen: (open: boolean) => void
  fetchHistory: () => Promise<void>
  loadConversation: (id: string) => Promise<Conversation | null>
  deleteConversation: (id: string) => Promise<void>
  refreshHistory: () => Promise<void>
}

export function useHistory(): UseHistoryReturn {
  const [history, setHistory] = useState<Conversation[]>([])
  const [isOpen, setIsOpen] = useState(false)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  /**
   * Fetch conversation history from the API
   */
  const fetchHistory = useCallback(async () => {
    setIsLoading(true)
    setError(null)

    try {
      const data = await api.getHistory(50)
      setHistory(data)
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to fetch history'
      setError(message)
      console.error('History fetch error:', err)
    } finally {
      setIsLoading(false)
    }
  }, [])

  /**
   * Load a specific conversation by ID
   */
  const loadConversation = useCallback(async (id: string): Promise<Conversation | null> => {
    try {
      return await api.getConversation(id)
    } catch (err) {
      console.error('Load conversation error:', err)
      return null
    }
  }, [])

  /**
   * Delete a conversation
   */
  const deleteConversation = useCallback(async (id: string): Promise<void> => {
    try {
      await api.deleteConversation(id)
      // Optimistically update the local state
      setHistory(prev => prev.filter(c => c.id !== id))
    } catch (err) {
      console.error('Delete conversation error:', err)
      throw err
    }
  }, [])

  /**
   * Refresh history (alias for fetchHistory)
   */
  const refreshHistory = fetchHistory

  // Fetch history when drawer opens
  useEffect(() => {
    if (isOpen) {
      fetchHistory()
    }
  }, [isOpen, fetchHistory])

  return {
    history,
    isOpen,
    isLoading,
    error,
    setIsOpen,
    fetchHistory,
    loadConversation,
    deleteConversation,
    refreshHistory
  }
}
