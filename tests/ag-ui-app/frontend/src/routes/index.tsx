import { createFileRoute } from '@tanstack/react-router'
import { useState, useCallback } from 'react'
import { Sparkles, Clock, RefreshCw } from 'lucide-react'

import {
  BackgroundEffects,
  MessageInputCard,
  ToneSelector,
  GenerateButton,
  ReplyOptionsSection,
  HistoryDrawer,
} from '@/components/smart-reply'

import { useSmartReply } from '@/hooks/useSmartReply'
import { useHistory } from '@/hooks/useHistory'

import type { Tone, Conversation } from '@/lib/api'

export const Route = createFileRoute('/')({
  component: SmartReplyApp,
})

function SmartReplyApp() {
  const [message, setMessage] = useState('')
  const [context, setContext] = useState('')
  const [intent, setIntent] = useState('')
  const [selectedTone, setSelectedTone] = useState<Tone>('professional')
  const [copiedId, setCopiedId] = useState<string | null>(null)

  const {
    isGenerating,
    streamedContent,
    replies,
    error,
    generate,
    stop,
    reset,
    setReplies,
  } = useSmartReply()

  const {
    history,
    isOpen: isHistoryOpen,
    isLoading: isHistoryLoading,
    setIsOpen: setIsHistoryOpen,
    deleteConversation,
    refreshHistory,
  } = useHistory()

  const handleGenerate = useCallback(async () => {
    if (!message.trim() || isGenerating) return
    await generate(message, selectedTone, context, intent)
    refreshHistory()
  }, [message, selectedTone, context, intent, isGenerating, generate, refreshHistory])

  const handleCopy = useCallback(async (replyId: string, content: string) => {
    try {
      await navigator.clipboard.writeText(content)
      setCopiedId(replyId)
      setTimeout(() => setCopiedId(null), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }, [])

  const handleSelect = useCallback(async (replyId: string, content: string) => {
    await handleCopy(replyId, content)
  }, [handleCopy])

  const handleLoadConversation = useCallback((conversation: Conversation) => {
    setMessage(conversation.original_message)
    setContext(conversation.context || '')
    setIntent(conversation.intent || '')
    setSelectedTone(conversation.tone)
    setReplies(conversation.replies)
    setIsHistoryOpen(false)
  }, [setReplies, setIsHistoryOpen])

  const handleDeleteConversation = useCallback(async (id: string) => {
    try {
      await deleteConversation(id)
    } catch (err) {
      console.error('Failed to delete:', err)
    }
  }, [deleteConversation])

  const handleReset = useCallback(() => {
    setMessage('')
    setContext('')
    setIntent('')
    reset()
  }, [reset])

  return (
    <main className="min-h-screen bg-slate-950 relative overflow-hidden">
      <BackgroundEffects />

      <div className="relative z-10 max-w-2xl mx-auto px-4 sm:px-6 py-12 sm:py-16">
        {/* Header */}
        <header className="flex flex-col items-center text-center mb-10 animate-fade-in-up">
          <div className="flex items-center gap-3 mb-4">
            <div className="p-3 rounded-2xl bg-gradient-to-br from-cyan-500/20 to-violet-600/20 border border-cyan-500/30 shadow-[0_0_30px_rgba(34,211,238,0.15)]">
              <Sparkles className="w-8 h-8 text-cyan-400" />
            </div>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight bg-gradient-to-r from-cyan-400 via-blue-400 to-violet-400 bg-clip-text text-transparent">
              Smart Reply
            </h1>
          </div>

          <p className="text-slate-400 max-w-md text-base sm:text-lg leading-relaxed">
            Provide context and intent to get AI-powered reply suggestions tailored to your needs.
          </p>

          <div className="flex items-center gap-3 mt-6">
            <button
              onClick={() => setIsHistoryOpen(true)}
              className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-white/5 border border-white/10
                text-slate-400 hover:text-slate-100 hover:bg-white/10 hover:border-white/20
                transition-all duration-200 text-sm font-medium cursor-pointer
                focus:outline-none focus:ring-2 focus:ring-cyan-500/50"
              aria-label="Open history"
            >
              <Clock className="w-4 h-4" />
              <span>History</span>
            </button>

            {(message || context || intent || replies.length > 0) && (
              <button
                onClick={handleReset}
                className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-white/5 border border-white/10
                  text-slate-400 hover:text-slate-100 hover:bg-white/10 hover:border-white/20
                  transition-all duration-200 text-sm font-medium cursor-pointer
                  focus:outline-none focus:ring-2 focus:ring-cyan-500/50"
                aria-label="Reset form"
              >
                <RefreshCw className="w-4 h-4" />
                <span>Reset</span>
              </button>
            )}
          </div>
        </header>

        {/* Main Content */}
        <div className="space-y-8">
          <MessageInputCard
            message={message}
            context={context}
            intent={intent}
            onMessageChange={setMessage}
            onContextChange={setContext}
            onIntentChange={setIntent}
            disabled={isGenerating}
          />

          <ToneSelector
            selected={selectedTone}
            onChange={setSelectedTone}
            disabled={isGenerating}
          />

          <div className="flex justify-center pt-4">
            <GenerateButton
              onClick={handleGenerate}
              onStop={stop}
              isLoading={isGenerating}
              disabled={!message.trim()}
            />
          </div>

          {error && (
            <div className="bg-rose-500/10 border border-rose-500/20 rounded-2xl p-5 animate-fade-in-up">
              <p className="text-rose-400 text-center text-sm font-medium">
                {error}
              </p>
            </div>
          )}

          <ReplyOptionsSection
            replies={replies}
            isLoading={isGenerating}
            tone={selectedTone}
            copiedId={copiedId}
            onCopy={handleCopy}
            onSelect={handleSelect}
            streamedContent={streamedContent}
          />
        </div>

        {/* Footer */}
        <footer className="mt-16 text-center">
          <p className="text-xs text-slate-600">
            Powered by AI
          </p>
        </footer>
      </div>

      <HistoryDrawer
        isOpen={isHistoryOpen}
        onClose={() => setIsHistoryOpen(false)}
        history={history}
        isLoading={isHistoryLoading}
        onSelectConversation={handleLoadConversation}
        onDeleteConversation={handleDeleteConversation}
      />
    </main>
  )
}
