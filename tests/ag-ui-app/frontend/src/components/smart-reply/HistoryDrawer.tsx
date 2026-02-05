import { X, Clock, Loader2 } from 'lucide-react'
import { HistoryItem } from './HistoryItem'
import type { Conversation } from '@/lib/api'

interface HistoryDrawerProps {
  isOpen: boolean
  onClose: () => void
  history: Conversation[]
  isLoading: boolean
  onSelectConversation: (conversation: Conversation) => void
  onDeleteConversation: (id: string) => void
}

export function HistoryDrawer({
  isOpen,
  onClose,
  history,
  isLoading,
  onSelectConversation,
  onDeleteConversation,
}: HistoryDrawerProps) {
  if (!isOpen) return null

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/70 backdrop-blur-sm z-40 animate-backdrop-fade-in cursor-pointer"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Drawer */}
      <aside
        className="fixed top-0 right-0 h-full w-full sm:w-[400px] bg-slate-900/95 backdrop-blur-xl
          border-l border-white/10 shadow-2xl z-50 flex flex-col animate-slide-in-right"
        role="dialog"
        aria-modal="true"
        aria-labelledby="history-title"
      >
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-white/10">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-xl bg-cyan-500/10 border border-cyan-500/20">
              <Clock className="w-5 h-5 text-cyan-400" />
            </div>
            <h2 id="history-title" className="text-lg font-semibold text-slate-100">
              History
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-2.5 rounded-xl hover:bg-white/10 text-slate-400 hover:text-slate-100
              transition-all duration-200 cursor-pointer
              focus:outline-none focus:ring-2 focus:ring-cyan-500/50"
            aria-label="Close history"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto custom-scrollbar p-4">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center h-48 text-slate-400">
              <Loader2 className="w-8 h-8 animate-spin mb-3 text-cyan-400" />
              <span className="text-sm">Loading history...</span>
            </div>
          ) : history.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-48 text-center">
              <div className="w-16 h-16 rounded-2xl bg-white/5 border border-white/10 flex items-center justify-center mb-4">
                <Clock className="w-8 h-8 text-slate-600" />
              </div>
              <p className="text-slate-300 font-medium">No history yet</p>
              <p className="text-slate-500 text-sm mt-1">
                Your generated replies will appear here
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {history.map((conversation, index) => (
                <HistoryItem
                  key={conversation.id}
                  conversation={conversation}
                  onClick={() => onSelectConversation(conversation)}
                  onDelete={() => onDeleteConversation(conversation.id)}
                  animationDelay={index * 50}
                />
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        {history.length > 0 && (
          <div className="p-4 border-t border-white/10">
            <p className="text-xs text-slate-500 text-center">
              {history.length} conversation{history.length !== 1 ? 's' : ''} saved
            </p>
          </div>
        )}
      </aside>
    </>
  )
}
