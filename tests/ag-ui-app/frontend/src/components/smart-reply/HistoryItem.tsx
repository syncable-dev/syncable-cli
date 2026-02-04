import { MessageSquare, Trash2, ChevronRight } from 'lucide-react'
import type { Conversation, Tone } from '@/lib/api'

interface HistoryItemProps {
  conversation: Conversation
  onClick: () => void
  onDelete: () => void
  animationDelay?: number
}

// Tone badge colors
const TONE_COLORS: Record<Tone, string> = {
  professional: 'bg-blue-500/20 text-blue-300',
  friendly: 'bg-emerald-500/20 text-emerald-300',
  apologetic: 'bg-amber-500/20 text-amber-300',
  assertive: 'bg-rose-500/20 text-rose-300',
  neutral: 'bg-slate-500/20 text-slate-300',
}

// Format relative time
function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMins / 60)
  const diffDays = Math.floor(diffHours / 24)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 7) return `${diffDays}d ago`

  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}

// Truncate text
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text
  return text.slice(0, maxLength).trim() + '...'
}

/**
 * HistoryItem Component
 * Individual history entry with preview
 */
export function HistoryItem({
  conversation,
  onClick,
  onDelete,
  animationDelay = 0,
}: HistoryItemProps) {
  const replyCount = conversation.replies?.length || 0

  return (
    <div
      className="group relative bg-white/5 hover:bg-white/10 border border-white/10 hover:border-white/20
        rounded-xl p-3 cursor-pointer transition-all duration-200 animate-fade-in-up"
      style={{ animationDelay: `${animationDelay}ms`, animationFillMode: 'both' }}
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClick()
        }
      }}
    >
      {/* Header row */}
      <div className="flex items-center justify-between mb-2">
        {/* Tone badge */}
        <span
          className={`px-2 py-0.5 rounded text-xs font-medium ${TONE_COLORS[conversation.tone]}`}
        >
          {conversation.tone}
        </span>

        {/* Timestamp and delete */}
        <div className="flex items-center gap-2">
          <span className="text-xs text-slate-500">
            {formatRelativeTime(conversation.created_at)}
          </span>

          {/* Delete button - shown on hover */}
          <button
            onClick={(e) => {
              e.stopPropagation()
              onDelete()
            }}
            className="p-1.5 rounded-lg opacity-0 group-hover:opacity-100 hover:bg-rose-500/20
              text-slate-500 hover:text-rose-400 transition-all duration-200 cursor-pointer
              focus:outline-none focus:opacity-100 focus:ring-2 focus:ring-rose-500/50"
            aria-label="Delete conversation"
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Message preview */}
      <p className="text-sm text-slate-300 leading-relaxed mb-2">
        {truncate(conversation.original_message, 80)}
      </p>

      {/* Footer row */}
      <div className="flex items-center justify-between">
        {/* Reply count */}
        <div className="flex items-center gap-1.5 text-xs text-slate-500">
          <MessageSquare className="w-3.5 h-3.5" />
          <span>{replyCount} repl{replyCount !== 1 ? 'ies' : 'y'}</span>
        </div>

        {/* Load indicator */}
        <div className="flex items-center gap-1 text-xs text-cyan-400 opacity-0 group-hover:opacity-100 transition-opacity">
          <span>Load</span>
          <ChevronRight className="w-3.5 h-3.5" />
        </div>
      </div>
    </div>
  )
}
